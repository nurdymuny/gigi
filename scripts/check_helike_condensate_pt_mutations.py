"""Each deliberate error in the condensate perturbation theory must be caught by a test assertion."""
import json
import os
import sys
import unittest
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import helike_condensate_pt as cp  # noqa: E402
import test_helike_condensate_pt as t  # noqa: E402

ORIG_T3, ORIG_V, ORIG_C = cp.CondensatePT.apply_T3, cp.CondensatePT.apply_V, cp.C_A.copy()


def no_kinetic_cubic(self, st):
    return {}


def quartic_sign(self, st, degree):
    out = ORIG_V(self, st, degree)
    return {k: -v for k, v in out.items()} if degree == 4 else out


def run_suite():
    suite = unittest.defaultTestLoader.loadTestsFromModule(t)
    with open(os.devnull, "w") as sink:
        return unittest.TextTestRunner(verbosity=0, stream=sink).run(suite)


def main():
    out, ok = {}, True
    base = run_suite()
    ok = not base.failures and not base.errors
    cases = {
        "kinetic_cubic_dropped": lambda: setattr(cp.CondensatePT, "apply_T3", no_kinetic_cubic),
        "quartic_sign": lambda: setattr(cp.CondensatePT, "apply_V", quartic_sign),
        "kinetic_metric_entry": lambda: cp.C_A.__setitem__((0, 0, 0), 5.),
    }
    for name, apply in cases.items():
        apply()
        try:
            r = run_suite()
        finally:
            cp.CondensatePT.apply_T3, cp.CondensatePT.apply_V = ORIG_T3, ORIG_V
            cp.C_A[...] = ORIG_C
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
