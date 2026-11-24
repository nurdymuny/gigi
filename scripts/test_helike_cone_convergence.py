"""Planted receipt controls, never an Mq=8 calculation."""
import copy
import json
import math
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import run_helike_cone_convergence as run
import run_helike_flux_background as flux


def rung(m=6, delta=0.0):
    return dict(E0=1., energy_shift=-100.,
                scope='positive-cone interior trial values; no boundary or sector convergence certificate',
                boundary_status='UNRESOLVED_PHYSICAL_REALIZATION',
                overlap_orders=[10, 12], reference_weight_error=1e-8,
                dimension=math.comb(m+6, 6), asymmetry=0., seconds=0.,
                helike=dict(gap=1.+delta, weight=.99, log_basis_weight=.99, level_index=1),
                competitor=dict(gap=1.2+delta, weight=.98, log_basis_weight=.98, level_index=3))


def rows(delta=.001):
    return {g: {m: rung(m, delta if m == 8 else 0.) for m in (6, 8)} for g in (.004, .002)}


def fixture(out, delta=.001):
    out.mkdir(parents=True, exist_ok=True)
    data = rows(delta)
    rec = dict(schema='cone-convergence-v1', status='COMPLETE', decision='CONVERGED',
               scope=flux.REPLAY_SCOPE, hash_policy=flux.HASH_POLICY,
               s=1.15, t=3., g2=[.004, .002], ladder=[6, 8], gap_tol=.003, weight_min=.95,
               revision=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=run.ROOT, text=True).strip(),
               source_dirty=bool(subprocess.check_output(
                   ['git', 'status', '--porcelain', '--', *run.SOURCES], cwd=run.ROOT, text=True).strip()),
               python=sys.version, platform='planted software fixture',
               started_utc='2026-11-24T00:00:00Z', finished_utc='2026-11-24T00:00:01Z',
               sources_sha256={s: flux.sha(run.ROOT/s) for s in run.SOURCES},
               rows={str(g): {str(m): r for m, r in by.items()} for g, by in data.items()},
               changes={str(g): {ch: abs(by[8][ch]['gap']-by[6][ch]['gap'])
                                 for ch in ('helike', 'competitor')} for g, by in data.items()})
    if delta > .003:
        rec['decision'] = 'NOT_CONVERGED'
    for source in run.SOURCES:
        dest = out/'sources'/source
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(run.ROOT/source, dest)
    flux.save_receipt(out/'receipt.json', rec)
    return rec


class ConvergenceEvidence(unittest.TestCase):
    def invoke(self, out, *args):
        with patch.object(sys, 'argv', ['runner', str(out), *args]):
            try:
                return run.main()
            except SystemExit as exc:
                return exc.code
            except Exception as exc:
                self.fail(f'expected recorded refusal, not {type(exc).__name__}: {exc}')

    def test_empty_controls_refuse(self):
        self.assertEqual(run.decide({})[0], 'UNRESOLVED')

    def test_missing_coupling_refuses(self):
        self.assertEqual(run.decide({.004: rows()[.004]})[0], 'UNRESOLVED')

    def test_extra_control_or_rung_refuses(self):
        for data in ({**rows(), .01: rows()[.004]}, rows()):
            if .01 not in data:
                data[.004][10] = rung(10)
            with self.subTest(data=data.keys()):
                self.assertEqual(run.decide(data)[0], 'UNRESOLVED')

    def test_invalid_measurements_refuse(self):
        for ch, key, value in [('helike', 'gap', float('nan')), ('helike', 'gap', float('inf')),
                               ('competitor', 'gap', -1.), ('competitor', 'weight', float('nan')),
                               ('helike', 'weight', 1.1), ('helike', 'gap', True)]:
            data = rows()
            data[.004][6][ch][key] = value
            with self.subTest(ch=ch, key=key, value=value):
                self.assertEqual(run.decide(data)[0], 'UNRESOLVED')

    def test_diagnostic_refusals(self):
        for key, value in [('dimension', True), ('dimension', 12), ('seconds', -1.),
                           ('E0', float('nan')), ('asymmetry', -1.),
                           ('reference_weight_error', math.nextafter(1e-6, math.inf)),
                           ('overlap_orders', [8, 10]), ('boundary_status', 'CERTIFIED')]:
            data = rows()
            data[.002][8][key] = value
            with self.subTest(key=key):
                self.assertEqual(run.decide(data)[0], 'UNRESOLVED')

    def test_gap_threshold_boundary(self):
        data = rows(0.)
        for by in data.values():
            by[6]['helike']['gap'] = 0.
            by[8]['helike']['gap'] = .003
        self.assertEqual(run.decide(data)[0], 'CONVERGED')
        data[.002][8]['helike']['gap'] = math.nextafter(.003, math.inf)
        self.assertEqual(run.decide(data)[0], 'NOT_CONVERGED')

    def test_weight_threshold_boundary(self):
        data = rows()
        data[.002][8]['competitor']['weight'] = .95
        self.assertEqual(run.decide(data)[0], 'CONVERGED')
        data[.002][8]['competitor']['weight'] = math.nextafter(.95, -math.inf)
        self.assertEqual(run.decide(data)[0], 'UNRESOLVED')

    def test_valid_converged_receipt(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            fixture(out)
            with patch.object(run.pc, 'ConeRitz', side_effect=AssertionError('verifier must not solve')):
                self.assertEqual(self.invoke(out, '--verify'), 0)
            report = json.loads((out/'verification.json').read_text())
            self.assertEqual(report['status'], 'PASS')
            self.assertEqual(report['scope'], flux.REPLAY_SCOPE)
            self.assertEqual(report['receipt_sha256'], flux.sha(out/'receipt.json'))

    def test_valid_not_converged_receipt(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            fixture(out, .004)
            self.assertEqual(self.invoke(out, '--verify'), 0)
            self.assertEqual(json.loads((out/'verification.json').read_text())['decision'], 'NOT_CONVERGED')

    def reject_edit(self, edit):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            rec = fixture(out)
            edit(rec)
            flux.save_receipt(out/'receipt.json', rec)
            flux.write(out/'verification.json', {'status': 'PASS'})
            self.assertEqual(self.invoke(out, '--verify'), 1)
            self.assertEqual(json.loads((out/'verification.json').read_text())['status'], 'FAIL')

    def test_resealed_empty_and_partial_reject(self):
        for edit in (lambda r: r.update(rows={}), lambda r: r['rows'].pop('0.002'),
                     lambda r: r['rows']['0.004'].pop('8'), lambda r: r.update(status='RUNNING')):
            with self.subTest(edit=edit):
                self.reject_edit(edit)

    def test_resealed_forged_changes_reject(self):
        self.reject_edit(lambda r: r['changes']['0.004'].update(helike=0.))

    def test_resealed_forged_decision_reject(self):
        self.reject_edit(lambda r: r.update(decision='NOT_CONVERGED'))

    def test_changed_parameters_reject(self):
        for key, value in [('s', 1.16), ('t', 4.), ('g2', [.004]), ('ladder', [4, 6]),
                           ('gap_tol', .004), ('weight_min', .9), ('schema', 'future'),
                           ('scope', 'physical certification')]:
            with self.subTest(key=key):
                self.reject_edit(lambda r: r.update({key: value}))

    def test_forged_provenance_rejects(self):
        self.reject_edit(lambda r: r.update(revision='0'*40))
        self.reject_edit(lambda r: r.update(source_dirty=not r['source_dirty']))
        self.reject_edit(lambda r: r.update(sources_sha256={}))

    def test_snapshot_change_rejects(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            fixture(out)
            (out/'sources'/run.SOURCES[0]).write_text('changed snapshot')
            self.assertEqual(self.invoke(out, '--verify'), 1)

    def test_integrity_change_rejects(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            rec = fixture(out)
            rec['platform'] = 'unsealed modification'
            flux.write(out/'receipt.json', rec)
            self.assertEqual(self.invoke(out, '--verify'), 1)

    def test_invalid_json_clears_stale_pass(self):
        for data in ('{', '[]', '{"decision":"CONVERGED","rows":{}}',
                     '{"rows":{},"rows":{}}', '{"gap":NaN}', '{"gap":1e999}'):
            with self.subTest(data=data), tempfile.TemporaryDirectory() as tmp:
                out = Path(tmp)
                fixture(out)
                (out/'receipt.json').write_text(data)
                flux.write(out/'receipt-integrity.json', {'sha256': flux.sha(out/'receipt.json')})
                flux.write(out/'verification.json', {'status': 'PASS'})
                self.assertEqual(self.invoke(out, '--verify'), 1)
                self.assertEqual(json.loads((out/'verification.json').read_text())['status'], 'FAIL')

    def test_interrupted_verification_invalidates_pass_first(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            fixture(out)
            flux.write(out/'verification.json', {'status': 'PASS'})
            with patch.object(run, 'sha', side_effect=KeyboardInterrupt), self.assertRaises(KeyboardInterrupt):
                self.invoke(out, '--verify')
            self.assertEqual(json.loads((out/'verification.json').read_text())['status'], 'RUNNING')

    def test_forged_statuses_never_release(self):
        with tempfile.TemporaryDirectory() as tmp:
            conv, cert = Path(tmp)/'receipt.json', Path(tmp)/'certificate.json'
            conv.write_text('{"decision":"CONVERGED","rows":{}}')
            cert.write_text('{"status":"PASS_CERTIFIED"}')
            self.assertEqual(flux.unmet_release_conditions(conv, cert),
                             ['convergence', 'certificate', 'method_amendment'])

    def test_initial_failure_retains_receipt(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)/'run'
            def fail(*args, **kwargs):
                self.assertEqual(json.loads((out/'receipt.json').read_text())['status'], 'RUNNING')
                raise MemoryError('planted before first solve')
            with patch.object(run.pc, 'ConeRitz', side_effect=fail):
                self.assertEqual(self.invoke(out), 2)
            rec = json.loads((out/'receipt.json').read_text())
            self.assertEqual(rec['status'], 'FAILED')
            self.assertEqual(rec['decision'], 'UNRESOLVED')
            self.assertEqual(rec['rows'], {})

    def test_later_interrupt_retains_completed_rung(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)/'run'
            class Fake:
                basis = range(924)
                asym = 0.
                def helike_and_competitor(self):
                    return rung()
            with patch.object(run.pc, 'ConeRitz', side_effect=[Fake(), KeyboardInterrupt()]):
                self.assertEqual(self.invoke(out), 2)
            rec = json.loads((out/'receipt.json').read_text())
            self.assertEqual(rec['status'], 'FAILED')
            self.assertEqual(set(rec['rows']['0.004']), {'6'})

    def test_nonempty_output_is_unchanged(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            fixture(out)
            before = (out/'receipt.json').read_bytes()
            with patch.object(run.pc, 'ConeRitz') as solver:
                self.assertNotEqual(self.invoke(out), 0)
                solver.assert_not_called()
            self.assertEqual(before, (out/'receipt.json').read_bytes())

    def test_complete_planted_run_checkpoints_and_verifies(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)/'run'
            class Fake:
                asym = 0.
                def __init__(self, s, t, g, Mq, max_Mq):
                    self.m = Mq
                    self.basis = range(math.comb(Mq+6, 6))
                def helike_and_competitor(self):
                    return rung(self.m, .001 if self.m == 8 else 0.)
            with patch.object(run.pc, 'ConeRitz', Fake):
                self.assertEqual(self.invoke(out), 0)
            rec = json.loads((out/'receipt.json').read_text())
            self.assertEqual(rec['decision'], 'CONVERGED')
            self.assertNotIn('active_case', rec)
            self.assertEqual(self.invoke(out, '--verify'), 0)

    def test_nonfinite_output_is_logged_without_nan_json(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)/'run'
            class Fake:
                basis = range(924)
                asym = 0.
                def helike_and_competitor(self):
                    r = rung()
                    r['helike']['gap'] = float('nan')
                    return r
            with patch.object(run.pc, 'ConeRitz', return_value=Fake()):
                self.assertEqual(self.invoke(out), 2)
            raw = (out/'receipt.json').read_text()
            self.assertNotIn('NaN', raw)
            rec = json.loads(raw)
            self.assertEqual(rec['status'], 'FAILED')
            self.assertEqual(rec['rows'], {})
            self.assertIn('nonfinite', rec['error'])

    def test_consistency_does_not_release_dirty_or_negative_evidence(self):
        for decision, dirty, expected in [('CONVERGED', False, ['certificate', 'method_amendment']),
                                         ('CONVERGED', True, ['convergence', 'certificate', 'method_amendment']),
                                         ('NOT_CONVERGED', False, ['convergence', 'certificate', 'method_amendment'])]:
            with self.subTest(decision=decision, dirty=dirty), patch.object(
                    run, 'read_valid_receipt', return_value=dict(decision=decision, source_dirty=dirty)):
                self.assertEqual(flux.unmet_release_conditions('receipt.json', 'certificate.json'), expected)

    def test_line_ending_changes_are_portable(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            fixture(out)
            for path in [out/'receipt.json', out/'receipt-integrity.json', *[out/'sources'/s for s in run.SOURCES]]:
                data = path.read_bytes().replace(b'\r\n', b'\n')
                path.write_bytes(data.replace(b'\n', b'\r\n'))
            self.assertEqual(self.invoke(out, '--verify'), 0)

    def test_unrepresentable_measurement_cannot_escape_release_hold(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            rec = fixture(out)
            rec['rows']['0.004']['6']['helike']['gap'] = 10**400
            flux.save_receipt(out/'receipt.json', rec)
            try:
                unmet = flux.unmet_release_conditions(out/'receipt.json')
            except Exception as exc:
                self.fail(f'invalid evidence must retain the hold, not raise {type(exc).__name__}')
            self.assertEqual(unmet, ['convergence', 'certificate', 'method_amendment'])


if __name__ == '__main__':
    unittest.main()
