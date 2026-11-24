"""Each deliberate error in the condensate solver must be caught by a test assertion (not by a crash)."""
import json
import os
import sys
import unittest
from pathlib import Path

import mpmath as mp

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import helike_berger_condensate as bc  # noqa: E402
import helike_berger_sector as bs  # noqa: E402
import test_helike_berger_condensate as t  # noqa: E402

ORIG_VALUE, ORIG_DOME, ORIG_H = bc.Moments.value, bc.dome_poly, bc.h_apply


def value_no_normalisation(self, e, gam):            # drops the gamma^{-3/2} per row
    sc = mp.mpf(1)
    for k in range(3):
        sc *= gam[k] ** (-mp.mpf(bs.row_degree(e, k)) / 2)
    return sc * bs._mpf(bs.iso_moment(e))


def dome_wrong_scale(K, lam):                       # forgets to square lambda in tr(X X^T)
    return ORIG_DOME(K, [mp.sqrt(abs(x)) for x in lam])


def h_without_source(f, beta, g, tt, a, b):
    return ORIG_H(f, beta, g, tt, 0 * a, 0 * b)


MUTANTS = {"moment_normalisation": ("Moments.value", value_no_normalisation),
           "dome_scale": ("dome_poly", dome_wrong_scale),
           "sources_dropped": ("h_apply", h_without_source)}


def setattr_path(path, fn):
    if path == "Moments.value":
        old = bc.Moments.value
        bc.Moments.value = fn
        return lambda: setattr(bc.Moments, "value", old)
    old = getattr(bc, path)
    setattr(bc, path, fn)
    return lambda: setattr(bc, path, old)


def run_suite():
    suite = unittest.defaultTestLoader.loadTestsFromModule(t)
    with open(os.devnull, "w") as sink:
        return unittest.TextTestRunner(verbosity=0, stream=sink).run(suite)


def main():
    out, ok = {}, True
    base = run_suite()
    ok = not base.failures and not base.errors
    for name, (path, fn) in MUTANTS.items():
        undo = setattr_path(path, fn)
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
