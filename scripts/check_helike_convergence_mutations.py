"""Disable evidence safeguards in memory; require assertion failures, never incidental errors."""
import contextlib
import inspect
import io
import json
from pathlib import Path
import sys
import unittest
from unittest.mock import patch

import run_helike_cone_convergence as run
import run_helike_flux_background as flux
from test_helike_cone_convergence import ConvergenceEvidence


def changed(module, function, old, new):
    source = inspect.getsource(getattr(module, function))
    if source.count(old) != 1:
        raise AssertionError(f'mutant target must occur once: {function}: {old}')
    namespace = {}
    exec(compile(source.replace(old, new), f'<mutant:{function}>', 'exec'), module.__dict__, namespace)
    return namespace[function]


def mutations():
    yield 'extra-control-coverage', [(run, 'check_rows', changed(run, 'check_rows',
        'set(rows) != set(G2S)', 'not set(G2S).issubset(rows)'))], 'test_extra_control_or_rung_refuses'
    yield 'extra-rung-coverage', [(run, 'check_rows', changed(run, 'check_rows',
        'set(by) != set(LADDER)', 'not set(LADDER).issubset(by)'))], 'test_extra_control_or_rung_refuses'
    yield 'finite-number-guards', [(run, 'finite_tree', lambda value: None),
                                  (run, 'number', lambda value: value)], 'test_invalid_measurements_refuse'
    yield 'gap-threshold', [(run, 'GAP_TOL', 1.)], 'test_gap_threshold_boundary'
    yield 'weight-threshold', [(run, 'WEIGHT_MIN', .9)], 'test_weight_threshold_boundary'
    yield 'overlap-error-gate', [(run, 'check_rung', changed(run, 'check_rung',
        'r["reference_weight_error"] > 1e-6', 'False'))], 'test_diagnostic_refusals'
    yield 'contract-check', [(run, 'read_valid_receipt', changed(run, 'read_valid_receipt',
        'type(rec[key]) is not type(expected) or rec[key] != expected', 'False'))], 'test_changed_parameters_reject'
    yield 'provenance-check', [(run, 'read_valid_receipt', changed(run, 'read_valid_receipt',
        'type(rec["source_dirty"]) is not bool or any(rec[k] != v for k, v in provenance().items())',
        'False'))], 'test_forged_provenance_rejects'
    yield 'snapshot-hash', [(run, 'read_valid_receipt', changed(run, 'read_valid_receipt',
        'sha(path.parent/"sources"/source) != digest or sha(ROOT/source) != digest',
        'False'))], 'test_snapshot_change_rejects'
    yield 'receipt-hash', [(run, 'read_valid_receipt', changed(run, 'read_valid_receipt',
        'integrity != {"sha256": sha(path)}', 'False'))], 'test_integrity_change_rejects'
    no_recompute = changed(run, 'read_valid_receipt',
        'rec["decision"] != decision or json.dumps(rec["changes"], sort_keys=True) != json.dumps(expected_changes, sort_keys=True)',
        'False')
    yield 'change-recomputation', [(run, 'read_valid_receipt', no_recompute)], 'test_resealed_forged_changes_reject'
    yield 'decision-recomputation', [(run, 'read_valid_receipt', no_recompute)], 'test_resealed_forged_decision_reject'
    # Remove only the first write; the interrupt must expose the stale PASS instead of raising a test error.
    yield 'stale-pass-invalidation', [(run, 'verify_receipt', changed(run, 'verify_receipt',
        '    write(report_path, report)\n    try:', '    try:'))], 'test_interrupted_verification_invalidates_pass_first'
    yield 'convergence-release-validation', [(run, 'read_valid_receipt',
        lambda path: dict(decision='CONVERGED', source_dirty=False))], 'test_forged_statuses_never_release'
    yield 'certificate-release-hold', [(flux, 'unmet_release_conditions', changed(flux, 'unmet_release_conditions',
        '    unmet.append("certificate")', '    pass'))], 'test_forged_statuses_never_release'


def main():
    if len(sys.argv) != 2:
        raise SystemExit('usage: check_helike_convergence_mutations.py OUTPUT_JSON')
    results = []
    for name, replacements, test in mutations():
        stream = io.StringIO()
        with contextlib.ExitStack() as stack:
            for module, attr, value in replacements:
                stack.enter_context(patch.object(module, attr, value))
            with contextlib.redirect_stdout(stream), contextlib.redirect_stderr(stream):
                result = unittest.TextTestRunner(stream=stream, verbosity=2).run(
                    unittest.TestSuite([ConvergenceEvidence(test)]))
        caught = len(result.failures) > 0 and not result.errors
        results.append(dict(mutation=name, test=test, caught_by_assertion=caught,
                            failures=len(result.failures), errors=len(result.errors), log=stream.getvalue()))
        print(name, 'CAUGHT' if caught else 'GATE FAILURE', flush=True)
    report = dict(scope='in-memory verifier mutants; no physical solve',
                  all_caught=all(r['caught_by_assertion'] for r in results), mutants=results)
    flux.write(Path(sys.argv[1]), report)
    return 0 if report['all_caught'] else 1


if __name__ == '__main__':
    sys.exit(main())
