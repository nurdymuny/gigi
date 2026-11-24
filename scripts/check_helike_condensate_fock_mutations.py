"""Each deliberate error in the local Fock solver must be caught by a test assertion.

(Dropping the curvature term U is not a mutant here: at O(g^2) it is constant and cancels in gaps, by design.)
"""
import json
import os
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import helike_condensate_fock as lf  # noqa: E402
import test_helike_condensate_fock as t  # noqa: E402

ORIG_INIT = lf.LocalFock.__init__


def init_no_kinetic_cubic(self, s, tt, g, P=6, Mq=6, include_U=True, include_kinetic_cubic=True):
    ORIG_INIT(self, s, tt, g, P, Mq, include_U, False)


def init_flipped_kinetic(self, s, tt, g, P=6, Mq=6, include_U=True, include_kinetic_cubic=True):
    orig_build = lf.LocalFock._build
    lf.LocalFock._build = lambda me: None
    try:
        ORIG_INIT(self, s, tt, g, P, Mq, include_U, include_kinetic_cubic)
    finally:
        lf.LocalFock._build = orig_build
    self.Bl = -self.Bl
    self._build()


def init_low_taylor(self, s, tt, g, P=6, Mq=6, include_U=True, include_kinetic_cubic=True):
    ORIG_INIT(self, s, tt, g, min(P, 2), Mq, include_U, include_kinetic_cubic)


MUTANTS = {"kinetic_cubic_dropped": init_no_kinetic_cubic, "kinetic_cubic_flipped": init_flipped_kinetic,
           "taylor_truncated_at_2": init_low_taylor}


def run_suite():
    suite = unittest.TestSuite([t.LocalFockSolver("test_small_g_reproduces_perturbation_theory")])
    with open(os.devnull, "w") as sink:
        return unittest.TextTestRunner(verbosity=0, stream=sink).run(suite)


def main():
    out, ok = {}, True
    for name, fn in MUTANTS.items():
        lf.LocalFock.__init__ = fn
        try:
            r = run_suite()
        finally:
            lf.LocalFock.__init__ = ORIG_INIT
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
