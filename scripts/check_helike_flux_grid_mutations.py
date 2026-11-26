"""Mutants of the amended grid runner; each must be caught by an assertion (amendment section 3.5).
usage: python scripts/check_helike_flux_grid_mutations.py OUT.json"""
import contextlib, copy, inspect, io, json, sys, textwrap, unittest
from pathlib import Path
from unittest.mock import patch
import run_helike_flux_grid as grid
from test_helike_flux_grid import Design, Point, Case, Release, Dressed


def run(case, test):
    s = io.StringIO()
    with contextlib.redirect_stdout(s), contextlib.redirect_stderr(s):
        r = unittest.TextTestRunner(stream=s, verbosity=2).run(unittest.TestSuite([case(test)]))
    return bool(r.failures) and not r.errors, s.getvalue()


def never_solve(*args):
    raise AssertionError("a mutant reached the real solver")


def main():
    # safety net: no mutation run may ever compute a grid point
    grid.solve = never_solve
    import helike_domain_gate
    helike_domain_gate.domain_gate_point = never_solve
    rows = []
    for key, value in (("convergence_tol", 0.004), ("weight_min", 0.85), ("s_tolerance", 0.03)):
        d = copy.deepcopy(grid.DESIGN); d[key] = value
        with patch.object(grid, "DESIGN", d):
            caught, log = run(Design, "test_frozen_design_is_the_registration_with_the_amended_ladder")
        rows.append(dict(name="relax_" + key, caught_by_assertion=caught, log=log))
    for name, func, old, new, case, test in [
        ("drop_domain_gate", "assess_point", 'and point.get("domain_gate") == "PASS"', "", Point, "test_domain_gate_must_pass"),
        ("any_point_suffices", "assess_case", "if not all(a[\"converged\"] and a[\"identified\"] for a in pts):",
         "if not any(a[\"converged\"] and a[\"identified\"] for a in pts):", Case, "test_unresolved_if_any_point_fails"),
        ("gate_always_passes", "domain_gate", 'return r["verdict"], r', 'return "PASS", r', Release, "test_domain_gate_passes_the_stage_verdict_through"),
        ("refusal_becomes_pass", "domain_gate", 'return "OPEN", {"error"', 'return "PASS", {"error"', Release, "test_domain_gate_passes_the_stage_verdict_through"),
        ("no_explicit_flag", "main", "if not a.registered_grid:", "if False:", Release, "test_the_grid_needs_an_explicit_flag"),
        # amendment 5.2: identification by the bare weight instead of w / Z
        ("bare_weight_identification", "assess_point", 'last[n]["dressed_weight"] for n', 'last[n]["weight"] for n',
         Dressed, "test_dressed_weight_identifies_when_the_bare_weight_is_below_threshold"),
    ]:
        src = textwrap.dedent(inspect.getsource(getattr(grid, func)))
        assert src.count(old) == 1, name
        original = getattr(grid, func)
        exec(src.replace(old, new), grid.__dict__)          # live namespace: the tests' stubs apply to the mutant
        try:
            caught, log = run(case, test)
        finally:
            setattr(grid, func, original)
        rows.append(dict(name=name, caught_by_assertion=caught, log=log))
    import helike_domain_gate as dg
    with patch.object(dg, "GATED", ("helike_plus", "helike_minus")):
        from test_helike_flux_grid import DomainStage
        caught, log = run(DomainStage, "test_verdict_aggregation")
    rows.append(dict(name="competitor_not_gated", caught_by_assertion=caught, log=log))
    # amendment 5.7: the cut-energy gate
    import helike_leakage_gate as lg
    from fractions import Fraction
    from test_helike_flux_grid import CutGate
    for name, func, old, new, test in [
        ("relative_gate_decisive", "classify_state_cut", "gate = cut_gate(e_cut)", "gate = relative_gate(leakage_upper, r_lower)",
         "test_relative_ratio_never_decides"),
        ("tail_term_dropped", "cut_energy", "e_cut = l + 2 * t * r", "e_cut = l", "test_tail_term_counts"),
        ("open_read_as_pass_state", "cut_gate", "return 'OPEN'", "return 'PASS'", "test_unresolved_enclosure_is_open_never_pass"),
        ("open_read_as_pass_point", "overall_cut", "    return 'OPEN'", "    return 'PASS'", "test_unresolved_enclosure_is_open_never_pass"),
    ]:
        src = textwrap.dedent(inspect.getsource(getattr(lg, func)))
        assert src.count(old) == 1, name
        original = getattr(lg, func)
        exec(src.replace(old, new), lg.__dict__)
        try:
            caught, log = run(CutGate, test)
        finally:
            setattr(lg, func, original)
        rows.append(dict(name=name, caught_by_assertion=caught, log=log))
    with patch.object(lg, "CUT_ENERGY_BUDGET", Fraction(3, 10**3)):
        caught, log = run(CutGate, "test_threshold_is_a_tenth_of_the_gap_tolerance")
    rows.append(dict(name="threshold_full_tolerance", caught_by_assertion=caught, log=log))
    for r in rows:
        print(r["name"], "CAUGHT" if r["caught_by_assertion"] else "NOT CAUGHT")
    rep = dict(all_caught=all(r["caught_by_assertion"] for r in rows), mutants=rows)
    Path(sys.argv[1]).write_text(json.dumps(rep, indent=1))
    return 0 if rep["all_caught"] else 1


if __name__ == "__main__":
    sys.exit(main())
