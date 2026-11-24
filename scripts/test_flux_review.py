"""Adversarial controls for the reviewed local flux calculation; no registered grid."""
import copy
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

import numpy as np

import helike_berger_model as bm
import helike_condensate_fock as lf
import helike_condensate_pt as cp
import run_helike_flux_background as run
import ym_two_grain_molien as mo


def result(h=1., c=1.2):
    return dict(E0=0., helike=dict(gap=h, weight=.99), competitor=dict(gap=c, weight=.99))


class FakeSolver:
    basis = [0]
    asym = 0.

    def __init__(self, *args, **kwargs):
        self.Mq, self.P = kwargs['Mq'], kwargs['P']

    def helike_and_competitor(self):
        h = {(6, 6): 1., (6, 8): 1.5, (8, 8): 1.501}.get((self.P, self.Mq), 1.)
        return result(h, h+.2)


class FluxReview(unittest.TestCase):
    def test_molien_rejects_aliasing_even_when_answers_are_integer(self):
        with self.assertRaises(ValueError):
            mo.hilbert_bigraded(2, M=1)

    def test_switch_rejects_reversed_grid(self):
        with self.assertRaises(ValueError):
            run.switch_point([2., 1.], [-1., 1.])

    def test_switch_rejects_nonfinite_or_incomplete_evidence(self):
        for ss, dd in (([1., 2.], [float('nan'), 1.]), ([1., 2.], [-1.]), ([], [])):
            with self.subTest(ss=ss, dd=dd), self.assertRaises(ValueError):
                run.switch_point(ss, dd)

    def test_multiple_crossings_are_not_silently_selected(self):
        with self.assertRaises(ValueError):
            run.switch_point([1., 2., 3., 4.], [-1., 1., -1., 1.])

    def test_full_grid_is_held_before_any_solver(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)/'run'
            with patch.object(sys, 'argv', ['runner', str(out)]), patch.object(run.lf, 'LocalFock') as solver:
                try:
                    self.assertEqual(run.main(), 2)
                except SystemExit as exc:
                    self.assertEqual(exc.code, 2)
                except Exception as exc:
                    self.fail(f'expected a recorded domain hold, not {type(exc).__name__}: {exc}')
            self.assertFalse(solver.called)
            rec = json.loads((out/'receipt.json').read_text())
            self.assertEqual(rec['status'], 'HOLD_DOMAIN')
            self.assertEqual(rec['decision'], 'NOT_EVALUATED')

    def test_passing_receipts_cannot_release_the_localfock_grid(self):
        with tempfile.TemporaryDirectory() as tmp:
            conv, cert = Path(tmp)/'conv.json', Path(tmp)/'cert.json'
            conv.write_text(json.dumps({'decision': 'CONVERGED'}))
            cert.write_text(json.dumps({'status': 'PASS_CERTIFIED'}))
            self.assertEqual(run.unmet_release_conditions(conv, cert), ['convergence', 'certificate', 'method_amendment'])
            self.assertEqual(run.unmet_release_conditions(), ['convergence', 'certificate', 'method_amendment'])
            conv.write_text(json.dumps({'decision': 'NOT_CONVERGED'}))
            self.assertIn('convergence', run.unmet_release_conditions(conv, cert))
            out = Path(tmp)/'run'
            argv = ['runner', str(out), '--convergence', str(Path(tmp)/'missing.json'), '--certificate', str(cert)]
            with patch.object(sys, 'argv', argv), patch.object(run.lf, 'LocalFock') as solver:
                try:
                    self.assertEqual(run.main(), 2)
                except SystemExit as exc:
                    self.assertEqual(exc.code, 2)
            self.assertFalse(solver.called)
            rec = json.loads((out/'receipt.json').read_text())
            self.assertEqual(rec['status'], 'HOLD_DOMAIN')
            self.assertEqual(set(rec['hold_conditions']), {'convergence', 'certificate', 'method_amendment'})

    def test_partial_rung_is_retained_on_failure(self):
        design = copy.deepcopy(run.SMOKE)
        design['ladder'] = [[6, 6], [6, 8]]
        calls = 0
        def fail_second(*args, **kwargs):
            nonlocal calls
            calls += 1
            if calls == 2:
                raise ArithmeticError('planted interruption')
            return FakeSolver(*args, **kwargs)
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)/'run'
            with patch.object(sys, 'argv', ['runner', str(out), '--smoke']), patch.object(run, 'SMOKE', design), \
                    patch.object(run.lf, 'LocalFock', side_effect=fail_second), self.assertRaises(ArithmeticError):
                run.main()
            rec = json.loads((out/'receipt.json').read_text())
            self.assertEqual(rec['status'], 'FAILED')
            self.assertEqual(len(rec['cases'][0]['points']), 1)
            self.assertEqual(len(rec['cases'][0]['points'][0]['ladder']), 1)

    def test_basis_refinement_cannot_hide_behind_stable_taylor_step(self):
        design = copy.deepcopy(run.SMOKE)
        design.update(ladder=[[6, 6], [6, 8], [8, 8]], convergence_tol=.003)
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)/'run'
            with patch.object(sys, 'argv', ['runner', str(out), '--smoke']), patch.object(run, 'SMOKE', design), \
                    patch.object(run.lf, 'LocalFock', FakeSolver):
                try: run.main()
                except SystemExit as exc: self.assertEqual(exc.code, 0)
            rec = json.loads((out/'receipt.json').read_text())
            self.assertFalse(rec['cases'][0]['points'][0]['converged'])

    def test_continued_kinetic_matrix_has_negative_region(self):
        G0 = np.array([2., 3., 4., 0., 0., 0.])
        M = np.linalg.cholesky(cp.A_at(G0))
        inv = np.linalg.inv(M)
        z = -2*inv @ G0
        G = G0 + M @ z
        np.testing.assert_allclose(inv @ cp.A_at(G) @ inv.T, -np.eye(6), atol=1e-14)
        self.assertLess(np.linalg.eigvalsh(cp.mat_from_vec(G)).max(), 0.)

    def local_stub(self, triple=False):
        L = object.__new__(lf.LocalFock)
        L.s, L.t = 1.15, 3.
        wH = np.sqrt(bm.mc_helike_freq2(L.s, L.t))
        L.w = np.array([.4, .7, .7, wH, wH, wH if triple else 3.])
        L.basis = [(0,)*6] + [tuple(int(i==j) for i in range(6)) for j in range(6)]
        L.spectrum = lambda nlev: (np.arange(7, dtype=float), np.eye(7))
        return L

    def test_selection_uses_available_eigenpairs(self):
        try:
            r = self.local_stub().helike_and_competitor(nlev=12)
        except (IndexError, ValueError) as exc:
            self.fail(f'small valid spectrum crashed: {exc}')
        self.assertEqual(r['helike']['gap'], 4.)
        self.assertEqual(r['competitor']['gap'], 1.)

    def test_accidental_harmonic_degeneracy_refuses_labels(self):
        with self.assertRaises(ArithmeticError):
            self.local_stub(triple=True).helike_and_competitor(nlev=7)

    def test_materially_asymmetric_spectrum_refuses(self):
        L = object.__new__(lf.LocalFock)
        L.H = np.array([[1., 3.], [0., 2.]])
        with self.assertRaises(ArithmeticError): L.spectrum()

    def fake_run(self, out):
        with patch.object(sys, 'argv', ['runner', str(out), '--smoke']), \
                patch.object(run.lf, 'LocalFock', FakeSolver), self.assertRaises(SystemExit) as ex:
            run.main()
        self.assertEqual(ex.exception.code, 0)

    def verify(self, out, code):
        with patch.object(sys, 'argv', ['runner', str(out), '--verify']), self.assertRaises(SystemExit) as ex:
            run.main()
        self.assertEqual(ex.exception.code, code)

    def test_smoke_cannot_emit_scientific_verdict(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)/'run'
            self.fake_run(out)
            rec = json.loads((out/'receipt.json').read_text())
            self.assertEqual(rec['decision'], 'SOFTWARE_SMOKE_ONLY')
            self.assertFalse(rec['certified'])

    def test_valid_receipt_replays_without_solver(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)/'run'
            self.fake_run(out)
            with patch.object(run.lf, 'LocalFock', side_effect=AssertionError('replay must not solve')):
                self.verify(out, 0)
            report = json.loads((out/'verification.json').read_text())
            self.assertEqual(report['status'], 'PASS')
            self.assertEqual(report['scope'], 'receipt consistency only, not matrix replay or physical certification')

    def test_changed_receipt_clears_stale_pass(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)/'run'
            self.fake_run(out)
            (out/'verification.json').write_text('{"status":"PASS"}')
            rec = json.loads((out/'receipt.json').read_text())
            rec['cases'][0]['points'][0]['ladder'][0]['helike']['gap'] += .1
            (out/'receipt.json').write_text(json.dumps(rec))
            self.verify(out, 1)
            self.assertEqual(json.loads((out/'verification.json').read_text())['status'], 'FAIL')

    def test_empty_primary_is_not_support(self):
        design = copy.deepcopy(run.SMOKE)
        design['primary']['g2'] = []
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)/'run'
            with patch.object(sys, 'argv', ['runner', str(out), '--smoke']), patch.object(run, 'SMOKE', design), \
                    patch.object(run.lf, 'LocalFock') as solver, self.assertRaises(ValueError):
                run.main()
            self.assertFalse(solver.called)

    def test_exact_zero_requires_a_unique_interior_bracket(self):
        self.assertEqual(run.switch_point([1., 2., 3.], [-1., 0., 1.]), 2.)
        for d in ([0., 1., 2.], [-1., 0., 0.], [1., 0., 1.]):
            with self.subTest(d=d), self.assertRaises(ValueError):
                run.switch_point([1., 2., 3.], d)

    def test_nonfinite_matrix_refuses(self):
        L = object.__new__(lf.LocalFock)
        L.H = np.array([[np.nan, 0.], [0., 2.]])
        with self.assertRaises(ArithmeticError): L.spectrum()

    def test_each_refinement_must_pass(self):
        D = copy.deepcopy(run.DESIGN)
        rows = []
        for (P, Mq), h in zip(D['ladder'], (1., 1.001, 1.002)):
            rows.append(dict(result(h, h+.2), P=P, Mq=Mq, dimension=7, asymmetry=0., seconds=0.))
        self.assertTrue(run.assess_point({'ladder': rows}, D)['converged'])
        rows[-1]['helike']['gap'] += .1
        self.assertFalse(run.assess_point({'ladder': rows}, D)['converged'])

    def test_smallest_nonaliasing_molien_grid_agrees(self):
        np.testing.assert_array_equal(mo.hilbert_bigraded(3, M=8), mo.hilbert_bigraded(3))

    def test_resealed_incomplete_evidence_cannot_pass(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)/'run'
            self.fake_run(out)
            rec = json.loads((out/'receipt.json').read_text())
            rec['cases'] = []
            run.save_receipt(out/'receipt.json', rec)
            self.verify(out, 1)

    def test_resealed_forged_convergence_cannot_pass(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)/'run'
            self.fake_run(out)
            rec = json.loads((out/'receipt.json').read_text())
            rec['cases'][0]['points'][0]['converged'] = True
            run.save_receipt(out/'receipt.json', rec)
            self.verify(out, 1)

    def test_changed_snapshot_cannot_pass(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)/'run'
            self.fake_run(out)
            p = out/'sources'/run.SOURCES[0]
            p.write_text(p.read_text() + '\n# planted corruption\n')
            self.verify(out, 1)

    def test_line_endings_do_not_break_receipt_replay(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)/'run'
            self.fake_run(out)
            files = [out/'receipt.json', out/'receipt-integrity.json']
            files += [out/'sources'/s for s in run.SOURCES]
            for p in files:
                p.write_bytes(p.read_bytes().replace(b'\r\n', b'\n').replace(b'\n', b'\r\n'))
            self.verify(out, 0)

    def test_nonfinite_rung_fails_without_losing_previous_rung(self):
        design = copy.deepcopy(run.SMOKE)
        design['ladder'] = [[6, 6], [6, 8]]
        class BadSecond(FakeSolver):
            def helike_and_competitor(self):
                return result(float('nan') if self.Mq == 8 else 1.)
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)/'run'
            with patch.object(sys, 'argv', ['runner', str(out), '--smoke']), patch.object(run, 'SMOKE', design), \
                    patch.object(run.lf, 'LocalFock', BadSecond), self.assertRaises(ValueError):
                run.main()
            rec = json.loads((out/'receipt.json').read_text())
            self.assertEqual(rec['status'], 'FAILED')
            self.assertEqual(len(rec['cases'][0]['points'][0]['ladder']), 1)


if __name__ == '__main__':
    unittest.main(verbosity=2)
