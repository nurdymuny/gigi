"""Remove individual mechanisms in memory; require the focused gates to go red."""

import inspect
import io
import json
import unittest
from unittest.mock import patch

import ym_helike_quantum as q
import test_ym_helike_quantum as tests


def run():
    cases = [
        ("drop_cubic", "operator_polynomials", '"D": determinant_polynomial()', '"D": {}',
         "BasisTests.test_independent_gauss_kernel_and_operator"),
        ("ignore_metric", "solve_generalized", 'metric = torch.as_tensor(s, device="cuda")',
         'metric = torch.eye(len(s), dtype=a.dtype, device="cuda")',
         "SolverTests.test_generalized_nonidentity_metric_cuda"),
        ("retain_vacuum", "probe_report", 'amplitudes[0] = 0', 'pass',
         "ProbeTests.test_constant_probe_does_not_normalize_vacuum"),
        ("single_vector_instead_of_level", "energy_groups", 'groups.append(np.arange(start, i))',
         'groups.extend(np.array([k]) for k in range(start, i))',
         "ProbeTests.test_degenerate_level_uses_whole_projector"),
        ("discard_exterior", "expanded_action", 'return out',
         'out[decode(output_codes).sum(axis=1) > basis["cutoff"]] = 0\n    return out',
         "ProbeTests.test_exterior_action_keeps_excitation_above_cutoff"),
        ("accept_drifting_energies", "convergence", 'return False', 'return True',
         "ReceiptTests.test_stable_gap_cannot_hide_drifting_energies"),
        ("accept_stale_receipt", "validate_receipt", 'receipt.get("run_id") == run_id', 'True',
         "ReceiptTests.test_stale_and_missing_gates_cannot_pass"),
        ("allow_missing_gates", "validate_receipt", 'set(required) == set(required_gates)', 'True',
         "ReceiptTests.test_stale_and_missing_gates_cannot_pass"),
    ]
    results = []
    for name, function, old, new, test in cases:
        source = inspect.getsource(getattr(q, function))
        if old not in source:
            raise RuntimeError(f"mutation target drifted: {name}")
        namespace = dict(q.__dict__)
        exec(compile(source.replace(old, new), f"<mutation:{name}>", "exec"), namespace)
        stream = io.StringIO()
        with patch.object(q, function, namespace[function]):
            result = unittest.TextTestRunner(stream=stream).run(unittest.defaultTestLoader.loadTestsFromName(test, tests))
        results.append({"mutation": name, "test": test, "rejected": not result.wasSuccessful(),
                        "test_output": stream.getvalue()})
    return results


if __name__ == "__main__":
    results = run()
    print(json.dumps(results, indent=2))
    raise SystemExit(0 if all(r["rejected"] for r in results) else 1)
