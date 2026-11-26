"""Tests and mutants for the R1.8 pieces: the symmetry-reduced C3 bound and the ray-aware fold check
(docs/research/helike-sc-rigorous-base-R1-registration-2026-11-25.md, R1.8). Arb at 128 bits; a few minutes.

usage: python scripts/test_helike_sc_r1_fold.py            (tests)
       python scripts/test_helike_sc_r1_fold.py --mutants  (each mutant must be caught by a test)
"""
import math
import sys
import unittest

import numpy as np
from scipy.optimize import minimize

import helike_sc_r1 as r1

M = {}


def model(g2):
    if g2 not in M:
        M[g2] = r1.Model(g2)
    return M[g2]


def float_poly(p):
    E = np.array([e for e in p.t]); C = np.array([float(c.mid()) for c in p.t.values()])
    return lambda z: float(C @ np.prod(np.asarray(z)[None, :] ** E, axis=1))


def sampled_max_P3(g2, starts=60):
    f = float_poly(model(g2).P.part(3)); rng = np.random.default_rng(7); best = 0.0
    for _ in range(starts):
        r = minimize(lambda y: -abs(f(y / np.linalg.norm(y))), rng.normal(size=6), method="Nelder-Mead",
                     options={"maxiter": 4000, "xatol": 1e-10, "fatol": 1e-12})
        best = max(best, -r.fun)
    return best


def sampled_min_D(g2, a, b, n=400_000, seed=3):
    """min of D = P - W0 over random points of the product tube (floating; walls oversampled)."""
    Mo = model(g2); f = float_poly(Mo.P); rng = np.random.default_rng(seed)
    ws = min(float(w.mid()) for w in Mo.w2[:3]) - a; wt = min(float(w.mid()) for w in Mo.w2[3:]) - b
    best = math.inf
    E = np.array([e for e in Mo.P.t]); C = np.array([float(c.mid()) for c in Mo.P.t.values()])
    for _ in range(n // 20000):
        u = rng.normal(size=(20000, 3)); u *= 5 * rng.uniform(0.6, 1, (20000, 1)) / np.linalg.norm(u, axis=1, keepdims=True)
        v = rng.normal(size=(20000, 3)); v *= 2 * rng.uniform(0, 1, (20000, 1)) ** (1 / 3) / np.linalg.norm(v, axis=1, keepdims=True)
        Z = np.hstack([u, v])
        P = np.prod(Z[:, None, :] ** E[None], axis=2) @ C
        D = P - 0.5 * ws * np.sum(u * u, 1) - 0.5 * wt * np.sum(v * v, 1)
        best = min(best, D.min())
    return best


class C3Sym(unittest.TestCase):
    def test_bound_dominates_the_full_sphere_maximum(self):
        for g2 in (0.004, 0.002):
            ub, best, nb = r1.c3_bound_sym(model(g2).P.part(3), max_boxes=60_000)
            self.assertGreaterEqual(nb, 0, "did not converge")
            self.assertGreaterEqual(float(ub.lower()), sampled_max_P3(g2) - 1e-9, g2)
            self.assertLessEqual(float(ub.upper()), sampled_max_P3(g2) + 2e-3, g2)

    def test_the_reduced_faces_are_the_registered_seven(self):
        faces = r1.c3_symmetric_faces()
        self.assertEqual(len(faces), 7)
        for lo, hi in faces:
            self.assertEqual((lo[1], hi[1]), (0.0, 0.0))
            pinned = [k for k in range(6) if k != 1 and lo[k] == hi[k]]
            self.assertEqual(len(pinned), 1)
            for k in (0, 3, 4):
                self.assertGreaterEqual(lo[k], 0.0)


class Fold(unittest.TestCase):
    def test_registered_folds_are_proved(self):
        for g2, (a, b) in ((0.002, ("0.36", "0.17")), (0.004, ("0.45", "0.25"))):
            verdict, worst, nb, _ = r1.fold_check(model(g2), a, b, max_boxes=60_000)
            self.assertEqual(verdict, "PROVED", (g2, a, b))

    def test_a_fold_that_needs_the_quartic_is_proved(self):
        # (0.41, 0.21) at g2 = 0.004: sampled D < 0 without P4 (the quartic unfolds the walls), proved with it
        Mo = model(0.004); full = Mo.P
        class NoP4:
            def part(self, d):
                return r1.Poly(6, {}) if d == 4 else full.part(d)
            @property
            def t(self):
                return {e: c for e, c in full.t.items() if sum(e) != 4}
        Mo.P = NoP4()
        try:
            self.assertLess(sampled_min_D(0.004, 0.41, 0.21), 0.0, "test premise: the quartic is needed")
        finally:
            Mo.P = full
        verdict, _, _, _ = r1.fold_check(Mo, "0.41", "0.21", max_boxes=150_000)
        self.assertEqual(verdict, "PROVED")

    def test_an_infeasible_fold_is_not_proved(self):
        # sampled D < 0 somewhere in the tube: the check must not prove it
        for g2, (a, b) in ((0.004, (0.30, 0.10)), (0.002, (0.20, 0.06))):
            self.assertLess(sampled_min_D(g2, a, b), 0.0, "test premise: the fold is infeasible")
            verdict, _, _, _ = r1.fold_check(model(g2), str(a), str(b), max_boxes=20_000)
            self.assertNotEqual(verdict, "PROVED", (g2, a, b))


MUTANTS = {}


def mutant(name):
    def deco(fn):
        MUTANTS[name] = fn
        return fn
    return deco


@mutant("c3: drop the z3 = +1 face")
def _m1():
    orig = r1.c3_symmetric_faces
    r1.c3_symmetric_faces = lambda: [f for i, f in enumerate(orig()) if i != 3]
    return lambda: setattr(r1, "c3_symmetric_faces", orig)


@mutant("c3: drop the z5 = -1 face")
def _m2():
    orig = r1.c3_symmetric_faces
    r1.c3_symmetric_faces = lambda: [f for i, f in enumerate(orig()) if i != 6]
    return lambda: setattr(r1, "c3_symmetric_faces", orig)


@mutant("fold: drop P4 (the quartic that unfolds the walls)")
def _m3():
    class NoP4:
        def __init__(self, P):
            self._P = P
        def part(self, d):
            return r1.Poly(6, {}) if d == 4 else self._P.part(d)
    origf = r1.fold_check
    def fc(Mo, a, b, **kw):
        saved = Mo.P; Mo.P = NoP4(saved)
        try:
            return origf(Mo, a, b, **kw)
        finally:
            Mo.P = saved
    r1.fold_check = fc
    return lambda: setattr(r1, "fold_check", origf)


@mutant("fold: exit parameter S halved (rays stop short of the wall)")
def _m4():
    origf = r1.fold_check
    def fc(Mo, a, b, Lu=5, Lv=2, **kw):
        return origf(Mo, a, b, Lu=Lu / 2, Lv=Lv / 2, **kw)
    r1.fold_check = fc
    return lambda: setattr(r1, "fold_check", origf)


@mutant("fold: base reductions applied with the wrong sign (W0 above the quadratic)")
def _m5():
    origf = r1.fold_check
    def fc(Mo, a, b, **kw):
        return origf(Mo, str(-float(a)), str(-float(b)), **kw)
    r1.fold_check = fc
    return lambda: setattr(r1, "fold_check", origf)


def run_tests():
    return unittest.TextTestRunner(verbosity=0).run(unittest.defaultTestLoader.loadTestsFromModule(sys.modules[__name__]))


if __name__ == "__main__":
    if "--mutants" in sys.argv:
        caught = 0
        for name, apply in MUTANTS.items():
            undo = apply()
            try:
                res = run_tests()
                ok = not res.wasSuccessful()
            finally:
                undo()
            caught += ok
            print(f"{'CAUGHT' if ok else 'MISSED'}  {name}", flush=True)
        print(f"{caught}/{len(MUTANTS)} mutants caught")
        sys.exit(0 if caught == len(MUTANTS) else 1)
    sys.exit(0 if run_tests().wasSuccessful() else 1)
