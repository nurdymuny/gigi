"""Each deliberate error in the dome solver must be caught by a test assertion (not by a crash).

usage: python scripts/check_ym_helike_dome_mutations.py [OUT.json]
"""
import json
import os
import sys
import unittest
from fractions import Fraction as F
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import mpmath as mp  # noqa: E402
import ym_helike_dome as dm  # noqa: E402
import ym_helike_j0 as j0  # noqa: E402
import test_ym_helike_dome as t  # noqa: E402


def set_key(d, key, value):
    def apply():
        old = d[key]
        d[key] = value
        return lambda: d.__setitem__(key, old)
    return apply


def patch(obj, name, value):
    def apply():
        old = getattr(obj, name)
        setattr(obj, name, value)
        return lambda: setattr(obj, name, old)
    return apply


def wrong_t_coefficient(f, a, g):
    h = ORIG_H(f, a, g)
    for m, c in f.items():           # (1 - a^2)/2 replaced by (1 - a)/2
        k = dm.mono_mul(m, dm.TM)
        h[k] = h.get(k, 0) + c * (a * a - a) / 2
    return h


def wrong_gamma(self, mono, gamma):
    return gamma ** (-mp.mpf(dm.degree(mono) + 8) / 2) * dm._mpf(dm.moment(mono))


def wrong_factorial(K, kappa):
    out = {}
    fact = mp.mpf(1)
    for k, ck in enumerate(dm.haar_series(K)):
        fact *= max(k, 1)            # k! instead of (k+1)!
        for m, v in ck.items():
            out[m] = out.get(m, 0) + kappa ** k / fact * dm._mpf(v)
    return out


ORIG_H = dm.hamiltonian_poly
MUTANTS = {
    "haar_e3_minus6": set_key(dm.HAAR_U, 3, {dm.DM: F(-6)}),
    "haar_e4_sign": set_key(dm.HAAR_U, 4, {dm.PM: F(2), (2, 0, 0): F(1)}),
    "hamiltonian_T_coefficient": patch(dm, "hamiltonian_poly", wrong_t_coefficient),
    "mixed_width_exponent": patch(dm.Moments, "value", wrong_gamma),
    "dome_factorial": patch(dm, "dome_polynomial", wrong_factorial),
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
