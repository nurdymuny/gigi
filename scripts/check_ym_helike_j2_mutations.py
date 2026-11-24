"""Each deliberate error in the spin-2 solver must be caught by a test assertion (not by a crash).

usage: python scripts/check_ym_helike_j2_mutations.py [OUT.json]
"""
import json
import os
import sys
import unittest
from fractions import Fraction as F
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import ym_helike_j0 as j0  # noqa: E402
import ym_helike_j2 as j2  # noqa: E402
import test_ym_helike_j2 as t  # noqa: E402


def set_key(d, key, value, mirror=None):
    def apply():
        old = d[key]
        d[key] = value
        if mirror:
            d[mirror] = value
        def undo():
            d[key] = old
            if mirror:
                d[mirror] = old
        return undo
    return apply


def patch_attr(obj, name, value):
    def apply():
        old = getattr(obj, name)
        setattr(obj, name, value)
        return lambda: setattr(obj, name, old)
    return apply


def wrong_branch(orig):
    def b(sol, index=0):
        out = orig(sol, index)
        out["residual"] = 0.0          # pretend the tensor is exact
        return out
    return b


MUTANTS = {
    "lapA2_21": set_key(j2.LAP_A, 2, {(j2.ZERO, 1): F(21)}),
    "dirP2_drop_e2": set_key(j2.DIR, (j2.P, 2), {((1, 0, 0), 2): F(16)}),
    "dirD2_dropped": set_key(j2.DIR, (j2.D, 2), {}),
    "dirP1_7": set_key(j2.DIR, (j2.P, 1), {(j2.ZERO, 2): F(7)}),
    "contract22_d2_3": set_key(j2.CONTRACT, (2, 2), {(4, 0, 0): F(-1, 2), (2, 1, 0): F(1), (0, 2, 0): F(1, 6),
                                                     (1, 0, 2): F(3)}),
    "contract12_wrong": set_key(j2.CONTRACT, (1, 2), {(3, 0, 0): F(-1, 2), (1, 1, 0): F(3, 2), (0, 0, 2): F(3)},
                                mirror=(2, 1)),
    "tensor_residual_zeroed": patch_attr(j2, "branch", wrong_branch(j2.branch)),
}


def run_suite():
    suite = unittest.defaultTestLoader.loadTestsFromModule(t)
    with open(os.devnull, "w") as sink:
        return unittest.TextTestRunner(verbosity=0, stream=sink).run(suite)


def main():
    out = {}
    base = run_suite()
    out["baseline"] = {"failures": len(base.failures), "errors": len(base.errors), "skipped": len(base.skipped)}
    ok = not base.failures and not base.errors
    for name, apply in MUTANTS.items():
        j0.moment.cache_clear()
        undo = apply()
        try:
            r = run_suite()
        finally:
            undo()
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
