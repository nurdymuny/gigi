"""Each deliberate error in the Berger Step 1 / sector code must be caught by a test assertion (not by a crash).

usage: python scripts/check_helike_berger_mutations.py [OUT.json]
"""
import json
import os
import sys
import unittest
from fractions import Fraction as F
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import helike_berger_pert as bp  # noqa: E402
import helike_berger_sector as bs  # noqa: E402
import test_helike_berger_sector as t  # noqa: E402

FAST = ["test_round_scalar_coefficient", "test_berger_splitting_closed_form",
        "test_protected_channel_is_a_formal_eigenfunction", "test_dimensions_match_molien",
        "test_round_limit_reproduces_verified_solvers"]
ORIG_GDV, ORIG_LAP, ORIG_V3 = bs.grad_dot_var, bs.lap_var, bp.apply_V3


def gdv_wrong(i, j):
    if bs.ND in (i, j) and i != j:
        k, l = bs.GKEYS[j if i == bs.ND else i]
        return {bs.unit(bs.ND): F(1)} if k == l else {}
    return ORIG_GDV(i, j)


def lap_wrong(i):
    out = ORIG_LAP(i)
    return {k: F(5) for k in out} if out else out


def v3_half(state, g, om):
    return {k: .5 * v for k, v in ORIG_V3(state, g, om).items()}


def missing_det_relation(a, b):
    out = {}
    for ka, va in a.items():
        for kb, vb in b.items():
            e = tuple(x + y for x, y in zip(ka, kb))
            if e[bs.ND] >= 2:
                continue                                    # drops d^2 = det G
            out = bs.padd(out, {e: va * vb})
    return out


MUTANTS = {
    "grad_G_dot_grad_d": (bs, "grad_dot_var", gdv_wrong),
    "laplacian_G": (bs, "lap_var", lap_wrong),
    "pert_cubic_halved": (bp, "apply_V3", v3_half),
    "d_squared_relation_dropped": (bs, "pmul", missing_det_relation),
}


def run_suite():
    loader = unittest.TestLoader()
    suite = unittest.TestSuite()
    for cls in (t.Perturbation, t.SectorSolver):
        for name in loader.getTestCaseNames(cls):
            if name in FAST:
                suite.addTest(cls(name))
    with open(os.devnull, "w") as sink:
        return unittest.TextTestRunner(verbosity=0, stream=sink).run(suite)


def main():
    out = {}
    base = run_suite()
    out["baseline"] = {"failures": len(base.failures), "errors": len(base.errors)}
    ok = not base.failures and not base.errors
    for name, (mod, attr, fn) in MUTANTS.items():
        old = getattr(mod, attr)
        setattr(mod, attr, fn)
        bs.iso_moment.cache_clear()
        try:
            r = run_suite()
        finally:
            setattr(mod, attr, old)
            bs.iso_moment.cache_clear()
        caught = len(r.failures) > 0 and len(r.errors) == 0
        out[name] = {"failures": len(r.failures), "errors": len(r.errors), "caught_by_assertion": caught}
        ok &= caught
        print(name, out[name], flush=True)
    out["all_caught"] = ok
    if len(sys.argv) > 1:
        Path(sys.argv[1]).write_text(json.dumps(out, indent=1))
    print("ALL MUTANTS CAUGHT" if ok else "MUTATION CHECK FAILED")
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
