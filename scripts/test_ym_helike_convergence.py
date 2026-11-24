"""Small CPU regression gates for Amendment 2; never launch the survey."""

import copy
import tempfile
import unittest
from pathlib import Path
from unittest.mock import Mock, patch

import numpy as np

import ym_helike_quantum as q
import run_ym_helike_convergence as runner


class WidthTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.b = q.build_basis(4)
        cls.ops = q.build_operators(cls.b)

    def test_alpha_one_preserves_matrix_and_exterior(self):
        h = self.ops['H0'] - 3*self.ops['D'] + self.ops['V4']
        np.testing.assert_array_equal(q.hamiltonian(self.ops, 1., 1.), h)
        e, v, _ = q.solve_generalized(h, self.b['S'], 'cpu')
        old = q.exterior_diagnostics(self.b, self.ops, e, v, 1.)
        new = q.exterior_diagnostics_scaled(self.b, self.ops, e, v, 1., 1.)
        np.testing.assert_array_equal(new['full_residual_norms'], old['full_residual_norms'])
        self.assertEqual(new['probe_discarded_fractions'], old['probe_discarded_fractions'])

    def test_physical_probe_normalization(self):
        alpha = 2.
        scaled = q.scaled_probes(self.ops, alpha)
        e, v, _ = q.solve_generalized(q.hamiltonian(self.ops, 1., alpha), self.b['S'], 'cpu')
        for name, raw, physical, factor in (
            ('H2', self.ops['tensor'], scaled['tensor'], 4.),
            ('H0', [self.ops['scalar']], [scaled['scalar']], 16.),
        ):
            with self.subTest(channel=name):
                a = q.probe_report(e, v, raw)
                b = q.probe_report(e, v, physical)
                self.assertAlmostEqual(a['excited_norm_squared']/b['excited_norm_squared'], factor, places=11)
                self.assertAlmostEqual(a['lowest_weight'], b['lowest_weight'], places=12)
                np.testing.assert_allclose(np.array(a['levels'][0]['kernel_real'])/factor,
                                           b['levels'][0]['kernel_real'], atol=1e-12)

    def test_exterior_scaled_trace_matches_independent_oscillator_action(self):
        alpha = 2.
        # Free oscillator at N=2: compare each selected column's scaled action.
        b = q.build_basis(2)
        ops = q.build_operators(b)
        e, v, _ = q.solve_generalized(q.hamiltonian(ops, 0., alpha), b['S'], 'cpu')
        result = q.exterior_diagnostics_scaled(b, ops, e, v, 0., alpha)
        codes = q.oscillator_codes(6)
        indices = result['state_indices']
        fock = b['C'] @ v[:, indices]
        coefficient = (1/alpha-alpha)/2
        action = q.expanded_action(b, v[:, indices], q.terms(q.operator_polynomials()['trace']), codes)*coefficient
        pos = np.searchsorted(codes, b['codes'])
        action[pos] += (alpha*(b['states'].sum(axis=1)+4.5)[:, None]-e[indices])*fock
        np.testing.assert_allclose(result['full_residual_norms'], np.linalg.norm(action, axis=0), atol=1e-12)

    def test_invalid_width_refused(self):
        for alpha in (0., -1., float('nan'), float('inf')):
            with self.subTest(alpha=alpha), self.assertRaises(ValueError):
                q.hamiltonian(self.ops, 1., alpha)


class SpinTests(unittest.TestCase):
    def mixed_fixture(self):
        e = np.array([0., 1., 1., 1., 1.])
        j2 = np.diag([0., 0., 2., 2., 2.])
        o = np.zeros((5, 5)); o[0, 1] = o[1, 0] = 1.
        return e, np.eye(5), j2, o

    def test_mixed_level_keeps_scalar_signal(self):
        e, v, j2, o = self.mixed_fixture()
        r = q.probe_report_with_spin(e, v, [o], j2, 'H0')
        self.assertEqual(r['status'], 'DETECTED')
        self.assertEqual(r['lowest_weight'], 1.)
        self.assertFalse(r['levels'][0]['selection_rule_zero'])
        self.assertEqual([c['J'] for c in r['levels'][0]['spin_components']], [0, 1])

    def test_rotated_mixed_level_and_repeated_irreps(self):
        e, v, j2, o = self.mixed_fixture()
        v[1:, 1:] = np.linalg.qr(np.random.default_rng(5).normal(size=(4, 4)))[0]
        r = q.probe_report_with_spin(e, v, [o], j2, 'H0')
        self.assertEqual(r['status'], 'DETECTED')
        self.assertAlmostEqual(r['lowest_weight'], 1.)
        spins = q.level_spins(np.array([0.]+[1.]*6), np.eye(7), np.diag([0.]+[2.]*6))
        self.assertEqual(spins[1]['components'][0]['copies'], 2)

    def test_forbidden_complete_level_is_symmetry_zero(self):
        e = np.array([0.]+[1.]*3+[2.])
        j2 = np.diag([0.]+[2.]*3+[0.])
        o = np.zeros((5, 5)); o[0, 4] = o[4, 0] = 1.
        r = q.probe_report_with_spin(e, np.eye(5), [o], j2, 'H0')
        self.assertEqual(r['status'], 'SELECTION_RULE_ZERO')
        self.assertEqual(r['lowest_weight'], 0.)
        self.assertEqual(r['accessible_gap'], 2.)

    def test_incomplete_multiplet_refused(self):
        with self.assertRaisesRegex(ValueError, 'multiplet'):
            q.level_spins(np.array([0., 1., 1.]), np.eye(3), np.diag([0., 2., 2.]))

    def test_nonintegral_casimir_refused(self):
        with self.assertRaisesRegex(ValueError, 'Casimir'):
            q.level_spins(np.array([0., 1.]), np.eye(2), np.diag([0., .4]))

    def test_casimir_leakage_refused(self):
        with self.assertRaisesRegex(ValueError, 'leakage'):
            q.level_spins(np.array([0., 1.]), np.eye(2), np.array([[0., .1], [.1, 0.]]))

    def test_non_scalar_vacuum_refused(self):
        with self.assertRaises(ValueError):
            q.probe_report_with_spin(np.array([0., 1.]), np.eye(2), [np.eye(2)], np.diag([2., 0.]), 'H0')

    def test_wrong_rank_probe_refused(self):
        e, v, j2, _ = self.mixed_fixture()
        # Both spins inhabit one level; level exclusion alone cannot catch this.
        o = np.zeros((5, 5)); o[0, 2] = o[2, 0] = 1.
        with self.assertRaisesRegex(ValueError, 'probe'):
            q.probe_report_with_spin(e, v, [o], j2, 'H0')

    def test_constant_probe_is_not_indexed_as_an_excited_level(self):
        r = q.probe_report_with_spin(np.array([0., 1.]), np.eye(2), [np.eye(2)], np.zeros((2, 2)), 'H0')
        self.assertEqual(r['status'], 'ZERO_EXCITED_NORM')


class EnclosureTests(unittest.TestCase):
    def test_ground_guard(self):
        r = q.residual_enclosures(np.array([0., 1.]), [2.], [[0]])[0]
        self.assertNotIn('kato_temple_conditional', r)

    def test_eligible_interval_is_uncertified(self):
        r = q.residual_enclosures(np.array([0., 1., 3.]), [.1], [[1]])[0]
        np.testing.assert_allclose(r['weinstein'], [.9, 1.1])
        np.testing.assert_allclose(r['kato_temple_conditional'], [.995, 1.01])
        self.assertIs(r['certified'], False)
        self.assertEqual(r['status'], 'NUMERICAL_ESTIMATE')

    def test_group_center_includes_energy_spread(self):
        e = np.array([0., 1., 1.+1e-9, 3.])
        r = q.residual_enclosures(e, [0.], [[1, 2]])[0]
        self.assertLessEqual(r['weinstein'][0], e[1])
        self.assertGreaterEqual(r['weinstein'][1], e[2])

    def test_nonfinite_or_negative_residual_refused(self):
        for eps in (-1., float('nan'), float('inf')):
            with self.subTest(eps=eps), self.assertRaises(ValueError):
                q.residual_enclosures(np.array([0., 1.]), [eps], [[0]])


class RunnerTests(unittest.TestCase):
    def history(self):
        c = {'E0':0., 'E1':1., 'absolute_gap':1.,
             'H0':{'accessible_gap':1., 'lowest_weight':1., 'spin_status':'VALIDATED'},
             'H2':{'accessible_gap':1., 'lowest_weight':1., 'spin_status':'VALIDATED'},
             'exterior':{'residual_gate':True, 'tail_gate':True},
             'spin_gate':True, 'evidence_gate':True, 'solver_agreement_gate':True}
        return [copy.deepcopy(c) for _ in range(3)]

    def test_missing_or_failed_gate_cannot_converge(self):
        gates = {name:True for name in runner.REQUIRED_GATES}
        self.assertTrue(runner.gated_convergence(self.history(), gates))
        for name in gates:
            bad = dict(gates); bad.pop(name)
            self.assertFalse(runner.gated_convergence(self.history(), bad))
            bad[name] = False
            self.assertFalse(runner.gated_convergence(self.history(), bad))
        for name in ('spin_gate', 'evidence_gate', 'solver_agreement_gate'):
            history = self.history(); history[-1][name] = False
            self.assertFalse(runner.gated_convergence(history, gates))
        history = self.history(); history[-1]['H0']['spin_status'] = 'VACUUM_NOT_SPIN_0'
        self.assertFalse(runner.gated_convergence(history, gates))

    def test_json_refuses_nan(self):
        with tempfile.TemporaryDirectory() as tmp, self.assertRaises(ValueError):
            runner.atomic(Path(tmp)/'receipt.json', {'value':float('nan')})

    def test_budget_refusal_precedes_construction(self):
        with patch.object(runner, 'HOST_BUDGET_BYTES', 1), patch.object(q, 'build_basis') as build:
            with self.assertRaises(MemoryError):
                runner.prepare_basis(4, 'cpu')
            build.assert_not_called()

    def test_resource_stop_clears_acceptance(self):
        receipt = {'status':'COMPLETE', 'results':{'one':{'converged':True}},
                   'scientific_status':{'one':'numerically_converged_model_estimate'}}
        runner.mark_failed(receipt, 'RESOURCE_STOP', 'test limit')
        self.assertEqual(receipt['status'], 'RESOURCE_STOP')
        self.assertFalse(receipt['results']['one']['converged'])
        self.assertEqual(receipt['scientific_status'], {})

    def test_case_arrays_replay_and_corruption(self):
        b = q.build_basis(4); ops = q.build_operators(b)
        c, arrays = runner.case(b, ops, 1., 2., 'cpu')
        with tempfile.TemporaryDirectory() as tmp:
            folder = Path(tmp)
            runner.save_case(folder, c, arrays)
            self.assertTrue(runner.check_saved_case(folder, c, replay_exterior=True))
            with np.load(folder/c['arrays'], allow_pickle=False) as stored:
                self.assertTrue({'H', 'S', 'J2', 'energies', 'vectors', 'tensor', 'scalar', 'alpha', 'g2'} <= set(stored.files))
            c['H2']['excited_norm_squared'] *= 2
            with self.assertRaises((ValueError, AssertionError)):
                runner.check_saved_case(folder, c)
            (folder/c['arrays']).write_bytes(b'corrupt')
            with self.assertRaisesRegex(ValueError, 'integrity'):
                runner.check_saved_case(folder, c)

    def test_worker_source_drift_stops_before_basis(self):
        with tempfile.TemporaryDirectory() as tmp:
            folder = Path(tmp)
            runner.atomic(folder/'receipt.json', {'status':'RUNNING', 'device':'cpu', 'source_hashes':{}, 'results':{}})
            with patch.object(runner, 'prepare_basis') as build, self.assertRaisesRegex(RuntimeError, 'sources changed'):
                runner.worker(folder)
            build.assert_not_called()
            self.assertEqual(runner.read_json(folder/'receipt.json')['status'], 'FAILED')

    def test_incomplete_receipt_refused(self):
        for status in ('RUNNING', 'FAILED', 'RESOURCE_STOP'):
            with self.subTest(status=status), self.assertRaisesRegex(ValueError, 'incomplete'):
                runner.verify_run(Path('convergence-test'), {'run_id':'test', 'status':status})

    def test_supervisor_stops_memory_overrun_and_clears_success(self):
        with tempfile.TemporaryDirectory() as tmp:
            folder = Path(tmp)
            runner.atomic(folder/'receipt.json', {'status':'COMPLETE', 'results':{'one':{'converged':True}}})
            process = Mock(pid=123); process.poll.return_value = None
            with patch('psutil.Process') as proc, patch.object(runner, 'HOST_BUDGET_BYTES', 1):
                proc.return_value.memory_info.return_value.rss = 2
                self.assertEqual(runner.supervise(process, folder), 2)
            process.kill.assert_called_once()
            saved = runner.read_json(folder/'receipt.json')
            self.assertEqual(saved['status'], 'RESOURCE_STOP')
            self.assertFalse(saved['results']['one']['converged'])

    def test_supervisor_stops_timeout(self):
        with tempfile.TemporaryDirectory() as tmp:
            folder = Path(tmp)
            runner.atomic(folder/'receipt.json', {'status':'RUNNING', 'results':{}})
            process = Mock(pid=123); process.poll.return_value = None
            with patch('psutil.Process') as proc, patch.object(runner, 'TIME_BUDGET_SECONDS', -1):
                proc.return_value.memory_info.return_value.rss = 1
                self.assertEqual(runner.supervise(process, folder), 2)
            self.assertEqual(runner.read_json(folder/'receipt.json')['status'], 'RESOURCE_STOP')

    def test_worker_exit_without_completion_is_failed(self):
        with tempfile.TemporaryDirectory() as tmp:
            folder = Path(tmp)
            runner.atomic(folder/'receipt.json', {'status':'RUNNING', 'results':{}})
            process = Mock(pid=123); process.poll.return_value = 0; process.wait.return_value = 0
            self.assertEqual(runner.supervise(process, folder), 2)
            self.assertEqual(runner.read_json(folder/'receipt.json')['status'], 'FAILED')

    def manifest_fixture(self):
        # Metadata-only fixture: no claimed simulation or planted survey output.
        table = {str(a):(a-1)**2 for a in runner.ALPHAS}
        receipt = {'status':'COMPLETE', 'run_id':'fixture', 'source_hashes':{'fixture':'hash'},
                   'certified_spectral_enclosure':False, 'HP09_utility':'NOT_RUN',
                   'gates':{k:True for k in runner.REQUIRED_GATES}, 'cutoffs':[8, 10, 12],
                   'alphas':list(runner.ALPHAS), 'n_select':10, 'alpha_selection':{},
                   'results':{}, 'scientific_status':{}}
        for g2 in runner.COUPLINGS:
            receipt['alpha_selection'][str(g2)] = {'alpha_star':1., 'E0_by_alpha':table}
            hist = self.history()
            for c, cutoff in zip(hist, receipt['cutoffs']):
                c.update(cutoff=cutoff, g2=g2, alpha=1., arrays='fixture.npz', arrays_sha256='hash')
            key = f'g2={g2:g},alpha=1'
            receipt['results'][key] = {'history':hist, 'converged':True}
            receipt['scientific_status'][key] = 'numerically_converged_model_estimate'
        return receipt

    def test_manifest_rejects_missing_gate_partial_run_and_forged_choice(self):
        receipt = self.manifest_fixture()
        folder = Path('convergence-fixture')
        with patch.object(runner, 'source_hashes', return_value=receipt['source_hashes']), patch.object(runner, 'digest', return_value='hash'):
            self.assertEqual(runner.verify_run(folder, receipt, replay=False)['tracks'], 3)
            bad = copy.deepcopy(receipt); bad['gates'].pop('HP12_spin')
            with self.assertRaisesRegex(ValueError, 'gate'):
                runner.verify_run(folder, bad, replay=False)
            bad = copy.deepcopy(receipt); bad['results']['g2=1,alpha=1']['history'].pop()
            with self.assertRaisesRegex(ValueError, 'partial'):
                runner.verify_run(folder, bad, replay=False)
            bad = copy.deepcopy(receipt); bad['alpha_selection']['1.0']['alpha_star'] = 2.
            with self.assertRaisesRegex(ValueError, 'frozen'):
                runner.verify_run(folder, bad, replay=False)
            bad = copy.deepcopy(receipt); bad['results']['g2=1,alpha=1']['converged'] = False
            with self.assertRaisesRegex(ValueError, 'forged convergence'):
                runner.verify_run(folder, bad, replay=False)
            bad = copy.deepcopy(receipt); bad['certified_spectral_enclosure'] = True
            with self.assertRaisesRegex(ValueError, 'certification'):
                runner.verify_run(folder, bad, replay=False)

    def test_source_snapshot_tampering_refused(self):
        receipt = self.manifest_fixture()
        with patch.object(runner, 'source_hashes', return_value=receipt['source_hashes']), patch.object(runner, 'digest', return_value='tampered'):
            with self.assertRaisesRegex(ValueError, 'snapshot integrity'):
                runner.verify_run(Path('convergence-fixture'), receipt, replay=False)


if __name__ == '__main__':
    unittest.main(verbosity=2)
