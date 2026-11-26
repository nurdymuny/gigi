"""Tests for M8 layer 3 (physical overlaps between points; A3.1). Uses two cheap real exports (Mq = 6).
usage: python scripts/test_helike_m8_overlap.py [--mutants]"""
import math
import os
import sys
import tempfile
import unittest

import numpy as np

import helike_m8_overlap as ov
import helike_m8_tracking as tr

WORK = os.environ.get("M8_OVERLAP_WORK", os.path.join(tempfile.gettempdir(), "m8_overlap_test"))
_fam = None


def fam():
    global _fam
    if _fam is None:
        _fam = ov.PhysicalFamily(3.0, 0.004, 6, "helike", WORK)
    return _fam


class Overlap(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.A, cls.B = fam().export(1.15), fam().export(1.16)
        cls.c = list(range(len(cls.A["levels"])))

    def test_self_overlap_is_identity(self):
        O = ov.overlap_matrix(self.A, self.c, self.A, self.c, 10)
        self.assertLess(np.max(np.abs(O - np.eye(len(self.c)))), 1e-10)

    def test_change_of_variables_is_symmetric(self):
        """<A|B> on B's grid equals <B|A> on A's grid: the Jacobian and Gaussian factors are right."""
        Oab = ov.overlap_matrix(self.A, self.c, self.B, self.c, 10)
        Oba = ov.overlap_matrix(self.B, self.c, self.A, self.c, 10)
        self.assertLess(np.max(np.abs(Oab - Oba.T)), 1e-6)
        self.assertGreater(np.max(np.abs(Oab)), 0.5)             # not trivially zero

    def test_vacuum_overlap_against_closed_form(self):
        """The two charts' harmonic vacua (basis state 000000) overlap by an exact Gaussian integral:
        jac * N_A N_B (2 pi)^3 / sqrt(det Q) * exp(b^T Q^{-1} b / 2 - k),  z_A = T z_B + c,
        Q = T^T W_A T + W_B,  b = T^T W_A c,  k = c^T W_A c / 2."""
        A, B = dict(self.A), dict(self.B)
        i0 = A["basis"].index((0,) * 6)
        for X in (A, B):
            e = np.zeros((len(X["basis"]), 1)); e[i0] = 1.0
            X["coeff"] = e
        got = ov.overlap_matrix(A, [0], B, [0], 12)[0, 0]
        T = np.linalg.solve(A["S"], B["S"]); c = np.linalg.solve(A["S"], B["u0"] - A["u0"])
        WA, WB = np.diag(A["w"]), np.diag(B["w"])
        Q = T.T @ WA @ T + WB; b = T.T @ WA @ c; k = 0.5 * c @ WA @ c
        N = lambda w: np.prod((w / math.pi) ** 0.25)
        jac = math.sqrt(abs(np.linalg.det(B["S"]) / np.linalg.det(A["S"])))
        want = jac * N(A["w"]) * N(B["w"]) * (2 * math.pi) ** 3 / math.sqrt(np.linalg.det(Q)) * math.exp(0.5 * b @ np.linalg.solve(Q, b) - k)
        self.assertAlmostEqual(got, want, places=10)
        self.assertLess(want, 0.9999)                              # the charts genuinely differ

    def test_rust_port_matches_the_python_reference(self):
        """helike cone-overlap reproduces overlap_matrix (both directions) to 1e-12."""
        A = fam().export(1.15); B = fam().export(1.16)
        R = ov.rust_overlap(A["path"], B["path"], 10, WORK)
        Pm = ov.overlap_matrix(A, self.c, B, self.c, 10)
        self.assertLess(np.max(np.abs(R - Pm)), 1e-12)
        R2 = ov.rust_overlap(B["path"], A["path"], 10, WORK)
        self.assertLess(np.max(np.abs(R2 - R.T)), 1e-6)

    def test_quadrature_seam(self):
        O, seam = ov.resolved_overlap(self.A, self.c, self.B, self.c)
        self.assertLessEqual(seam, ov.SEAM)
        self.assertTrue(np.all(O.sum(axis=1) <= 1 + 1e-8))

    def test_tracking_step_on_real_states(self):
        """A short real step: matching resolves through the midpoint, Helike's pair is followed, no crossing."""
        rep = tr.run_case(fam(), [1.15, 1.16], lambda S, c: 0.9)
        st = rep["steps"][0]
        self.assertEqual(st["status"], "RESOLVED")
        self.assertEqual(st["crossing"]["kind"], "NONE")
        S1 = fam().sample(1.16)
        self.assertEqual(S1.clusters[rep["points"][-1]["carrier"]], (1, 2))
        self.assertEqual(rep["verdict"], "PERSISTS")


MUTANTS = {}


def mutant(name):
    def deco(f):
        MUTANTS[name] = f
        return f
    return deco


@mutant("the constant Jacobian sqrt|det S_B / det S_A| dropped")
def _m1():
    orig = ov.overlap_matrix
    def om(XA, cA, XB, cB, order):
        return orig(XA, cA, XB, cB, order) / math.sqrt(abs(np.linalg.det(XB["S"]) / np.linalg.det(XA["S"])))
    ov.overlap_matrix = om
    return lambda: setattr(ov, "overlap_matrix", orig)


@mutant("the chart change ignored (states compared coefficient by coefficient)")
def _m2():
    orig = ov.overlap_matrix
    def om(XA, cA, XB, cB, order):
        return XA["coeff"][:, cA].T @ XB["coeff"][:, cB]
    ov.overlap_matrix = om
    return lambda: setattr(ov, "overlap_matrix", orig)


def run():
    return unittest.TextTestRunner(verbosity=0).run(unittest.defaultTestLoader.loadTestsFromTestCase(Overlap))


if __name__ == "__main__":
    if "--mutants" in sys.argv:
        caught = 0
        for name, apply in MUTANTS.items():
            undo = apply()
            try:
                ok = not run().wasSuccessful()
            finally:
                undo()
            caught += ok
            print(("CAUGHT  " if ok else "MISSED  ") + name, flush=True)
        print(f"{caught}/{len(MUTANTS)} mutants caught")
        sys.exit(0 if caught == len(MUTANTS) else 1)
    sys.exit(0 if run().wasSuccessful() else 1)
