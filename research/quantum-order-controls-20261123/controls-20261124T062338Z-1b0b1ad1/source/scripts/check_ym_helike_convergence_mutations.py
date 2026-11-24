"""Remove amendment mechanisms in memory; require the corresponding gate to fail."""
import inspect
import io
import json
import types
import unittest
from unittest.mock import patch

import ym_helike_quantum as q
import run_ym_helike_convergence as runner
import test_ym_helike_convergence as tests


def run():
    cases = [
        ('omit_eigenpair_ordering', q, 'solve_generalized', 'order = np.argsort(energies, kind="stable")',
         'order = np.arange(len(energies))', 'WidthTests.test_solver_orders_eigenpairs_without_changing_values'),
        ('unscaled_H2', q, 'scaled_probes', '[p/alpha for p in ops["tensor"]]', 'ops["tensor"]',
         'WidthTests.test_physical_probe_normalization'),
        ('unscaled_H0', q, 'scaled_probes', 'ops["scalar"]/alpha**2', 'ops["scalar"]',
         'WidthTests.test_physical_probe_normalization'),
        ('drop_exterior_trace', q, 'exterior_diagnostics_scaled', 'if alpha != 1.0:', 'if False:',
         'WidthTests.test_exterior_scaled_trace_matches_independent_oscillator_action'),
        ('average_mixed_spin', q, 'level_spins', 'values = la.eigvalsh(block)',
         'values = np.full(len(indices), np.trace(block).real/len(indices))',
         'SpinTests.test_mixed_level_keeps_scalar_signal'),
        ('allow_incomplete_multiplet', q, 'level_spins', 'if count % (2*J+1):', 'if False:',
         'SpinTests.test_incomplete_multiplet_refused'),
        ('ignore_probe_rank', q, 'probe_report_with_spin',
         'if response_norm > 1e-12 and error/response_norm > 1e-6:', 'if False:',
         'SpinTests.test_wrong_rank_probe_refused'),
        ('drop_ground_guard', q, 'residual_enclosures', 'radius < b - E', 'True',
         'EnclosureTests.test_ground_guard'),
        ('claim_certification', q, 'residual_enclosures', '"certified": False', '"certified": True',
         'EnclosureTests.test_eligible_interval_is_uncertified'),
        ('omit_group_spread', q, 'residual_enclosures', 'radius = float(eps + spread)', 'radius = float(eps)',
         'EnclosureTests.test_group_center_includes_energy_spread'),
        ('bypass_acceptance', runner, 'gated_convergence', 'return False', 'return True',
         'RunnerTests.test_missing_or_failed_gate_cannot_converge'),
        ('bypass_budget', runner, 'prepare_basis', 'enforce_budget(cutoff, device)', 'pass',
         'RunnerTests.test_budget_refusal_precedes_construction'),
        ('stale_convergence_on_failure', runner, 'mark_failed', "track['converged'] = False", 'pass',
         'RunnerTests.test_resource_stop_clears_acceptance'),
        ('omit_probe_replay', runner, 'check_saved_case',
         'compare_report(result[channel], q.probe_report_with_spin(e, v, probes, j2, channel), channel)', 'pass',
         'RunnerTests.test_case_arrays_replay_and_corruption'),
        ('skip_source_drift_check', runner, 'worker', "if source_hashes() != receipt['source_hashes']:", 'if False:',
         'RunnerTests.test_worker_source_drift_stops_before_basis'),
        ('accept_partial_history', runner, 'verify_run', "[c['cutoff'] for c in hist] != cutoffs", 'False',
         'RunnerTests.test_manifest_rejects_missing_gate_partial_run_and_forged_choice'),
    ]
    results = []
    for name, module, function, old, new, test in cases:
        source = inspect.getsource(getattr(module, function))
        if old not in source:
            raise RuntimeError(f'mutation target drifted: {name}')
        namespace = dict(module.__dict__)
        exec(compile(source.replace(old, new), f'<mutation:{name}>', 'exec'), namespace)
        compiled = namespace[function]
        # Rebind the mutant to the module's real globals, so patches a test applies to the module (source hashes,
        # budgets, basis construction) reach the mutant exactly as they reach the pristine function. With a copied
        # namespace the mutant bypassed those patches and failed on unrelated errors.
        mutant = types.FunctionType(compiled.__code__, module.__dict__, compiled.__name__,
                                    compiled.__defaults__, compiled.__closure__)
        mutant.__kwdefaults__ = compiled.__kwdefaults__
        stream = io.StringIO()
        with patch.object(module, function, mutant):
            result = unittest.TextTestRunner(stream=stream).run(unittest.defaultTestLoader.loadTestsFromName(test, tests))
        results.append({'mutation': name, 'test': test, 'rejected': not result.wasSuccessful(),
                        'test_output': stream.getvalue()})
    return results


if __name__ == '__main__':
    results = run()
    print(json.dumps(results, indent=2))
    raise SystemExit(0 if results and all(r['rejected'] for r in results) else 1)
