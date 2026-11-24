"""Each deliberate error in the spin-0 solver must be caught by a test assertion (not by a crash).

usage: python scripts/check_ym_helike_j0_mutations.py [OUT.json]
"""
import json
import sys
import unittest
from fractions import Fraction as F
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import ym_helike_j0 as j  # noqa: E402
import test_ym_helike_j0 as t  # noqa: E402


def set_dict(d, key, value):
    def apply():
        old = d[key]
        d[key] = value
        return lambda: d.__setitem__(key, old)
    return apply


def patch_attr(obj, name, value):
    def apply():
        old = getattr(obj, name)
        setattr(obj, name, value)
        return lambda: setattr(obj, name, old)
    return apply


def wrong_parts(orig):
    def parts(m):
        A, B, C, Dq = orig(m)
        return A, B, {}, Dq          # drop the cubic (-3 g det q) term
    return parts


def wrong_temple(orig):
    def oi(sol):
        o = orig(sol)
        o["separation_verified"] = True
        return o
    return oi


MUTANTS = {
    "laplacian_P_27T": set_dict(j.LAPLACIAN, j.P, {(1, 0, 0): F(27)}),
    "grad_dd_sign": set_dict(j.GRAD_DOT, (j.D, j.D), {(2, 0, 0): F(-1, 2), (0, 1, 0): F(1, 2)}),
    "grad_PD_3Td": set_dict(j.GRAD_DOT, (j.P, j.D), {(1, 0, 1): F(3)}),
    "grad_PP_drop_d2": set_dict(j.GRAD_DOT, (j.P, j.P), {(3, 0, 0): F(-8), (1, 1, 0): F(24)}),
    "drop_cubic_term": patch_attr(j, "operator_parts", wrong_parts(j.operator_parts)),
    "separation_promoted": patch_attr(j, "ordering_inputs", wrong_temple(j.ordering_inputs)),
}
TESTS = ["test_gradient_inner_products", "test_laplacians", "test_matches_nine_variable_gpu_controls",
         "test_against_nine_variable_expansion", "test_temple_quantities_and_labels"]


def run_suite():
    loader = unittest.TestLoader()
    suite = unittest.TestSuite()
    for cls in (t.GeneratorIdentities, t.Moments, t.Spectrum, t.OrderingInputs):
        for name in loader.getTestCaseNames(cls):
            if name in TESTS:
                suite.addTest(cls(name))
    return unittest.TextTestRunner(verbosity=0, stream=open("/dev/null", "w") if sys.platform != "win32"
                                   else open("NUL", "w")).run(suite)


def main():
    out = {}
    base = run_suite()
    out["baseline"] = {"failures": len(base.failures), "errors": len(base.errors), "skipped": len(base.skipped)}
    ok = not base.failures and not base.errors
    for name, apply in MUTANTS.items():
        j.moment.cache_clear()
        undo = apply()
        if name in ("grad_dd_sign", "grad_PD_3Td", "grad_PP_drop_d2"):
            key = {"grad_dd_sign": (j.D, j.D), "grad_PD_3Td": (j.P, j.D), "grad_PP_drop_d2": (j.P, j.P)}[name]
            j.GRAD_DOT[key[::-1]] = j.GRAD_DOT[key]
        try:
            r = run_suite()
        finally:
            undo()
            for key in [(j.D, j.D), (j.P, j.D), (j.P, j.P)]:
                j.GRAD_DOT[key[::-1]] = j.GRAD_DOT[key]
            j.moment.cache_clear()
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
