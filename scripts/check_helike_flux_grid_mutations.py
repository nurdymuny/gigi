"""Mutants of the amended grid runner; each must be caught by an assertion (amendment section 3.5).
usage: python scripts/check_helike_flux_grid_mutations.py OUT.json"""
import contextlib, copy, inspect, io, json, sys, textwrap, unittest
from pathlib import Path
from unittest.mock import patch
import run_helike_flux_grid as grid
from test_helike_flux_grid import Design, Point, Case, Release


def run(case, test):
    s = io.StringIO()
    with contextlib.redirect_stdout(s), contextlib.redirect_stderr(s):
        r = unittest.TextTestRunner(stream=s, verbosity=2).run(unittest.TestSuite([case(test)]))
    return bool(r.failures) and not r.errors, s.getvalue()


def main():
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
        ("gate_always_passes", "domain_gate", 'return "OPEN"', 'return "PASS"', Release, "test_domain_gate_is_open_until_implemented"),
    ]:
        src = textwrap.dedent(inspect.getsource(getattr(grid, func)))
        assert src.count(old) == 1, name
        ns = dict(vars(grid)); exec(src.replace(old, new), ns)
        with patch.object(grid, func, ns[func]):
            caught, log = run(case, test)
        rows.append(dict(name=name, caught_by_assertion=caught, log=log))
    for r in rows:
        print(r["name"], "CAUGHT" if r["caught_by_assertion"] else "NOT CAUGHT")
    rep = dict(all_caught=all(r["caught_by_assertion"] for r in rows), mutants=rows)
    Path(sys.argv[1]).write_text(json.dumps(rep, indent=1))
    return 0 if rep["all_caught"] else 1


if __name__ == "__main__":
    sys.exit(main())
