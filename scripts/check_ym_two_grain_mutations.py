"""Each deliberate error in the two-grain potential must be caught by a test assertion (not by a crash).

usage: python scripts/check_ym_two_grain_mutations.py [OUT.json]
"""
import json
import os
import sys
import unittest
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import ym_two_grain_potential as tg  # noqa: E402
import test_ym_two_grain_potential as t  # noqa: E402

EPS = tg.EPS


def curvature_frame_sign(M, N, D):           # X_m D with the wrong sign
    B = M + N @ D
    XD = np.einsum("jl,mlk->mjk", D, EPS)
    XB = np.einsum("ai,mik->amk", N, XD)
    return XB - XB.transpose(0, 2, 1) - np.einsum("jkl,al->ajk", EPS, B) + np.einsum("abc,bj,ck->ajk", EPS, B, B)


def curvature_bracket_sign(M, N, D):         # [X_j, X_k] = -eps X_l
    B = M + N @ D
    XD = -np.einsum("jl,mlk->mjk", D, EPS)
    XB = np.einsum("ai,mik->amk", N, XD)
    return XB - XB.transpose(0, 2, 1) + np.einsum("jkl,al->ajk", EPS, B) + np.einsum("abc,bj,ck->ajk", EPS, B, B)


def curvature_transposed_frame(M, N, D):     # right frame read as D^T
    return ORIG(M, N, D.T)


def projection_without_rotation(M, N, Ds=None):
    Ds = tg.rotations() if Ds is None else Ds
    cM, cN = np.zeros((3, 3)), np.zeros((3, 3))
    for D in Ds:
        starF = .5 * np.einsum("ljk,ajk->al", EPS, tg.curvature(M, N, D))
        cM += starF
        cN += starF                          # forgot to rotate into the right frame
    return cM / len(Ds), cN / len(Ds)


def plane_cross_3(u, v):                     # wrong cross coefficient in the plane formula
    return 3 * np.pi ** 2 * (u ** 2 * (2 - u) ** 2 + v ** 2 * (2 - v) ** 2) + 3 * np.pi ** 2 * u ** 2 * v ** 2


def chern_simons_sum(q, p, g):               # oriented difference replaced by the auxiliary sum
    WL = .5 * np.sum(q * q) - g * np.linalg.det(q)
    WR = .5 * np.sum(p * p) + g * np.linalg.det(p)
    return 2 * g * g * (WL + WR)


ORIG = tg.curvature
ORIG_BIDEGREE = tg.bidegree_parts


def bidegree_ignores_degree_guard(M, N, Ds=None, maxdeg=4):
    return ORIG_BIDEGREE(M, N, Ds, maxdeg=4)


MUTANTS = {
    "frame_derivative_sign": ("curvature", curvature_frame_sign),
    "bracket_sign": ("curvature", curvature_bracket_sign),
    "right_frame_transposed": ("curvature", curvature_transposed_frame),
    "projection_unrotated": ("projected_gradient", projection_without_rotation),
    "degree_guard_ignored": ("bidegree_parts", bidegree_ignores_degree_guard),
    "plane_cross_coefficient": ("plane_potential", plane_cross_3),
    "chern_simons_as_sum": ("chern_simons", chern_simons_sum),
}


def run_suite():
    suite = unittest.defaultTestLoader.loadTestsFromModule(t)
    with open(os.devnull, "w") as sink:
        return unittest.TextTestRunner(verbosity=0, stream=sink).run(suite)


def main():
    out = {}
    base = run_suite()
    out["baseline"] = {"failures": len(base.failures), "errors": len(base.errors)}
    ok = not base.failures and not base.errors
    for name, (attr, fn) in MUTANTS.items():
        old = getattr(tg, attr)
        setattr(tg, attr, fn)
        try:
            r = run_suite()
        finally:
            setattr(tg, attr, old)
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
