"""Gates for the second-envelope (dome) J=0 solver, scripts/ym_helike_dome.py."""
import math
import unittest
from fractions import Fraction as F

import mpmath as mp
import numpy as np

import ym_helike_dome as dm
import ym_helike_j0 as j0
from test_ym_helike_j0 import evaluate, gens, num_lap


def _solve(tc, *a, **k):
    try:
        return dm.solve(*a, **k)
    except (ArithmeticError, ValueError, ZeroDivisionError) as exc:   # a refusal is a failed assertion
        tc.fail(f"solver refused: {exc}")


def signed_singular(X):
    U, s, Vt = np.linalg.svd(X)
    s = s.copy()
    s[2] *= np.sign(np.linalg.det(U) * np.linalg.det(Vt))
    return s


class HaarSeries(unittest.TestCase):
    def test_low_order_coefficients(self):
        c = dm.haar_series(4)
        self.assertEqual(c[0], {dm.ONE: F(1)})
        self.assertEqual(c[1], {})
        self.assertEqual({m: v / 6 for m, v in c[2].items()}, {dm.TM: F(1, 6)})
        self.assertEqual({m: v / 24 for m, v in c[3].items()}, {dm.DM: F(1, 6)})

    def test_lambda_polynomial_identity(self):
        rng = np.random.default_rng(31)
        for _ in range(5):
            X = rng.normal(size=(3, 3))
            s = signed_singular(X)
            lam = np.array([s.sum(), s[0] - s[1] - s[2], -s[0] + s[1] - s[2], -s[0] - s[1] + s[2]])
            t, p, d = gens(X)
            for tt in (0.13, -0.41, 0.77):
                want = np.prod(1 - lam * tt)
                got = 1 + sum(evaluate(poly, X) * tt ** k for k, poly in dm.HAAR_U.items())
                self.assertAlmostEqual(got, want, delta=1e-10 * max(1, abs(want)))

    def test_moments_against_haar_monte_carlo(self):
        try:
            from scipy.spatial.transform import Rotation
        except ImportError:
            self.skipTest("scipy unavailable")
        X = np.random.default_rng(3).normal(size=(3, 3)) * .8
        R = Rotation.random(400000, random_state=7).as_matrix()
        x = np.einsum("nij,ij->n", R, X)
        c = dm.haar_series(6)
        for k in range(2, 7):
            exact = evaluate(c[k], X) * math.factorial(k) / math.factorial(k + 1)
            se = (x ** k).std() / math.sqrt(len(x))
            self.assertLess(abs((x ** k).mean() - exact), 5 * se + 1e-12, k)

    def test_dome_function_against_haar_average(self):
        try:
            from scipy.spatial.transform import Rotation
        except ImportError:
            self.skipTest("scipy unavailable")
        X = np.random.default_rng(5).normal(size=(3, 3)) * .5
        kappa = 1.4
        R = Rotation.random(400000, random_state=8).as_matrix()
        e = np.exp(kappa * np.einsum("nij,ij->n", R, X))
        with mp.workdps(30):
            poly = dm.dome_polynomial(30, mp.mpf(kappa))
        got = sum(float(v) * evaluate({m: 1}, X) for m, v in poly.items())
        self.assertLess(abs(e.mean() - got), 5 * e.std() / math.sqrt(len(e)))

    def test_truncation_converges(self):
        X = np.random.default_rng(4).normal(size=(3, 3)) * .6
        with mp.workdps(40):
            a, b = dm.dome_polynomial(30, mp.mpf(2)), dm.dome_polynomial(40, mp.mpf(2))
        va = sum(float(v) * evaluate({m: 1}, X) for m, v in a.items())
        vb = sum(float(v) * evaluate({m: 1}, X) for m, v in b.items())
        self.assertLess(abs(va - vb), 1e-9 * abs(vb))


class Operators(unittest.TestCase):
    def test_mixed_width_moments(self):
        with mp.workdps(40):
            Mo = dm.Moments()
            for gamma in (mp.mpf("0.85"), mp.mpf("2.3")):
                for k in range(5):
                    ratio = Mo.value((k, 0, 0), gamma) / Mo.value(dm.ONE, gamma)
                    want = mp.gamma(mp.mpf(9) / 2 + k) / mp.gamma(mp.mpf(9) / 2) / gamma ** k
                    self.assertLess(abs(ratio - want), mp.mpf(10) ** -30 * want)

    def test_cross_width_overlap(self):
        """Normalised overlap of two centred Gaussians of widths a, b is (2 sqrt(ab)/(a+b))^(9/2)."""
        with mp.workdps(40):
            Mo = dm.Moments()
            a, b = mp.mpf("1.7"), mp.mpf("0.6")
            one = {dm.ONE: mp.mpf(1)}
            ov = Mo.inner(one, a, one, b) / mp.sqrt(Mo.inner(one, a, one, a) * Mo.inner(one, b, one, b))
            want = (2 * mp.sqrt(a * b) / (a + b)) ** (mp.mpf(9) / 2)
            self.assertLess(abs(ov - want), mp.mpf(10) ** -30)
            tf = {dm.TM: mp.mpf(1)}
            ov2 = Mo.inner(tf, a, one, b) / mp.sqrt(Mo.inner(tf, a, tf, a) * Mo.inner(one, b, one, b))
            # <T>_g = 9/(2g); <T^2>_g = 99/(4g^2); gamma = (a+b)/2
            gm = (a + b) / 2
            want2 = (mp.mpf(9) / (2 * gm) * gm ** (-mp.mpf(9) / 2)) / mp.sqrt(
                mp.mpf(99) / (4 * a ** 2) * a ** (-mp.mpf(9) / 2) * b ** (-mp.mpf(9) / 2))
            self.assertLess(abs(ov2 - want2), mp.mpf(10) ** -30)

    def test_hamiltonian_action_by_finite_differences(self):
        rng = np.random.default_rng(41)
        q = rng.normal(size=(3, 3)) * .6
        g, a = 1.3, 1.7
        f = {(1, 0, 1): mp.mpf(1), (0, 1, 0): mp.mpf("-0.4")}
        with mp.workdps(30):
            h = dm.hamiltonian_poly(f, mp.mpf(a), mp.mpf(g))
        psi = lambda z: sum(float(c) * evaluate({m: 1}, z) for m, c in f.items()) * math.exp(-a * gens(z)[0] / 2)
        t, p, d = gens(q)
        V = t / 2 - 3 * g * d + g * g / 4 * (t * t - p)
        want = -.5 * num_lap(psi, q, h=1e-3) + V * psi(q)
        got = sum(float(c) * evaluate({m: 1}, q) for m, c in h.items()) * math.exp(-a * t / 2)
        self.assertAlmostEqual(got, want, delta=1e-4 * max(1, abs(want)))


class Gates(unittest.TestCase):
    def test_D00_reproduces_scalar_sector_solver(self):
        for g2, a in ((1., 1.7), (2., 2.3)):
            s, r = _solve(self, 10, g2, a, dps=60), j0.solve(10, g2, a, dps=60)
            np.testing.assert_allclose(s["energies"], r["energies"][:6], atol=1e-12, rtol=0)
            np.testing.assert_allclose(s["residuals"], r["residuals"][:6], atol=1e-12, rtol=0)

    def test_D01_dome_never_raises_ritz_values_and_D05_fractions(self):
        for g2, a in ((1., 1.7), (2., 2.3)):
            base = _solve(self, 8, g2, a, dps=60)
            aug = _solve(self, 8, g2, a, n_d=4, beta=1.5, K=24, dps=80)
            self.assertEqual(aug["n_dome"], 5)
            for x, y in zip(aug["energies"][:4], base["energies"][:4]):
                self.assertLessEqual(x, y + 1e-12)
            for fr in aug["outside_origin_fraction"]:
                self.assertGreaterEqual(fr, -1e-12)
                self.assertLessEqual(fr, 1 + 1e-12)

    def test_dome_count(self):
        self.assertEqual([len(dm.dome_monomials(n)) for n in (0, 3, 4)], [1, 3, 5])


if __name__ == "__main__":
    unittest.main()
