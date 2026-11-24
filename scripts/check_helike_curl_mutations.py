"""Each deliberate error in the curl-spectrum check must be caught by a test assertion (not by a crash).

usage: python scripts/check_helike_curl_mutations.py [OUT.json]
"""
import json
import os
import sys
import unittest
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import helike_curl_spectrum as hc  # noqa: E402
import test_helike_curl_spectrum as t  # noqa: E402

ORIG_OPS = hc.operators


def curl_wrong_orientation(x, y, tt, j):          # (curl b)_1 built from D_02 instead of D_20
    curl, H, Phi, Dall, f = ORIG_OPS(x, y, tt, j)
    d = j + 1
    sl = slice(3 * d, 6 * d)
    curl = curl.copy()
    curl[sl, :] = -curl[sl, :]
    return curl, H, Phi, Dall, f


def flux_sign_flipped(x, y, tt, j):
    curl, H, Phi, Dall, f = ORIG_OPS(x, y, tt, j)
    return curl, Dall.conj().T @ Dall - Phi, -Phi, Dall, f


MUTANTS = {
    "curl_orientation": ("operators", curl_wrong_orientation),
    "flux_sign": ("operators", flux_sign_flipped),
    "twist_formula_shift": ("twist_closed_form",
                            lambda x, y, tt, j: (x + (j + 1) / np.sqrt(tt)) ** 2 + 2 * np.sqrt(tt) * x - y * y),
    "fibre_field_sign": ("fibre_field", lambda x, y, tt: y * y + 2 * np.sqrt(tt) * x),
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
        old = getattr(hc, attr)
        setattr(hc, attr, fn)
        try:
            r = run_suite()
        finally:
            setattr(hc, attr, old)
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
