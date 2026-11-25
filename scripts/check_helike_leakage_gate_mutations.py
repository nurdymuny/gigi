"""Remove the relative leakage gate's mechanisms deliberately. Only assertion failures catch mutants.

usage: python scripts/check_helike_leakage_gate_mutations.py OUT.json
The seven mutants required by section 5 of the amendment, plus the all-states requirement.
"""
import contextlib
import inspect
import io
import json
from pathlib import Path
import sys
import textwrap
import unittest
from unittest.mock import patch

import helike_leakage_gate as lg
import run_helike_leakage_gate as run
from test_helike_leakage_gate import Gate, Runner


def main():
    mutations = [
        ('r_upper_for_r_lower', run, 'r_lower_value', "return rust_state['r_lower']", "return rust_state['r_upper']",
         Runner, 'test_r_lower_is_the_lower_endpoint'),
        ('float_as_r_lower', lg, 'relative_gate', 'if isinstance(r_lower, RigorousLower) else None',
         'if isinstance(r_lower, RigorousLower) else (Fraction(r_lower) if r_lower is not None else None)',
         Gate, 'test_open_without_a_rigorous_lower_bound'),
        ('no_q_division', lg, 'residual_bound', 'return (exact(r_upper) + exact(leakage_upper)) / q',
         'return exact(r_upper) + exact(leakage_upper)', Gate, 'test_residual_bound_and_temple_enlargement'),
        ('drop_l_squared', lg, 'temple_enlargement', 'return (2 * r * l + l * l) / (q * q)', 'return (2 * r * l) / (q * q)',
         Gate, 'test_residual_bound_and_temple_enlargement'),
        ('absolute_promoted_to_gate', lg, 'classify_state', "'state_passed': tail_ok and norm_ok and gate == 'PASS',",
         "'state_passed': tail_ok and norm_ok and gate == 'PASS' and absolute,", Gate, 'test_absolute_diagnostic_is_reported_not_gating'),
        ('relabel_historical', run, 'historical_gate', "return arb_receipt['cutoff_tail_gate_passed']", 'return True',
         Runner, 'test_historical_receipt_is_not_relabelled'),
        ('any_state_suffices', lg, 'overall', "if all(s['state_passed'] for s in states):", "if any(s['state_passed'] for s in states):",
         Gate, 'test_overall_requires_every_state'),
    ]
    rows = []
    for name, owner, func, old, new, case, test in mutations:
        source = textwrap.dedent(inspect.getsource(getattr(owner, func)))
        if source.count(old) != 1:
            raise ValueError('nonunique mutation ' + name)
        namespace = dict(vars(owner))
        exec(source.replace(old, new), namespace)
        stream = io.StringIO()
        with patch.object(owner, func, namespace[func]), contextlib.redirect_stdout(stream), contextlib.redirect_stderr(stream):
            result = unittest.TextTestRunner(stream=stream, verbosity=2).run(unittest.TestSuite([case(test)]))
        caught = bool(result.failures) and not result.errors
        rows.append(dict(name=name, caught_by_assertion=caught, failures=len(result.failures), errors=len(result.errors), log=stream.getvalue()))
        print(name, 'CAUGHT' if caught else 'GATE FAILURE', flush=True)
    # changing kappa is a module constant, not a function body: patch the constant
    stream = io.StringIO()
    with patch.object(lg, 'KAPPA', lg.Fraction(1, 100)), contextlib.redirect_stdout(stream), contextlib.redirect_stderr(stream):
        result = unittest.TextTestRunner(stream=stream, verbosity=2).run(unittest.TestSuite([Gate('test_kappa_and_boundary')]))
    caught = bool(result.failures) and not result.errors
    rows.append(dict(name='kappa_changed', caught_by_assertion=caught, failures=len(result.failures), errors=len(result.errors), log=stream.getvalue()))
    print('kappa_changed', 'CAUGHT' if caught else 'GATE FAILURE', flush=True)
    report = dict(all_caught=all(r['caught_by_assertion'] for r in rows), mutants=rows)
    Path(sys.argv[1]).write_text(json.dumps(report, indent=1))
    return 0 if report['all_caught'] else 1


if __name__ == '__main__':
    sys.exit(main())
