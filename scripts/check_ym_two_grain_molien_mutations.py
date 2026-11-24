"""Each deliberate error in the Molien/basis code must be caught by a test assertion (not by a crash).

usage: python scripts/check_ym_two_grain_molien_mutations.py [OUT.json]
"""
import json
import os
import sys
import unittest
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import ym_two_grain_molien as mo  # noqa: E402
import test_ym_two_grain_molien as t  # noqa: E402

ORIG_GEN = mo.generators_mod


def wrong_weight(M):                       # Weyl density with the wrong sign
    th = 2 * np.pi * np.arange(M) / M
    return th, (1 + np.cos(th)) / M


def wrong_character(j, th):                # drops the m = 0 weight
    return sum(np.cos(m * th) for m in range(-j, j + 1) if m != 0) if j else np.ones_like(th)


def drop_det(q, p):
    g = ORIG_GEN(q, p)
    g.pop("d_q")
    return g


def wrong_mixed(q, p):                     # X21 computed as tr(A B^2) (duplicates X12)
    g = ORIG_GEN(q, p)
    g["X21"] = ((4, 2), g["X12"][1])
    return g


MUTANTS = {
    "weyl_weight_sign": ("weyl_grid", wrong_weight),
    "character_missing_zero_weight": ("character", wrong_character),
    "generator_det_dropped": ("generators_mod", drop_det),
    "generator_X21_wrong": ("generators_mod", wrong_mixed),
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
        old = getattr(mo, attr)
        setattr(mo, attr, fn)
        try:
            r = run_suite()
        finally:
            setattr(mo, attr, old)
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
