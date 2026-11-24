"""Gates for the spin-0 invariant-sector solver (scripts/ym_helike_j0.py)."""
import glob
import itertools
import unittest
from fractions import Fraction
from pathlib import Path

import numpy as np

import ym_helike_j0 as j

ROOT = Path(__file__).resolve().parent.parent
CONTROLS = ROOT / "research/quantum-order-controls-20261123/controls-20261124T062338Z-1b0b1ad1"


def gens(q):
    Q = q @ q.T
    return np.array([np.trace(Q), np.trace(Q @ Q), np.linalg.det(q)])


def evaluate(p, q):
    t, pp, d = gens(q)
    return sum(float(c) * t ** a * pp ** b * d ** e for (a, b, e), c in p.items())


def num_grad(f, q, h=1e-6):
    g = np.zeros(9)
    for k in range(9):
        e = np.zeros(9)
        e[k] = h
        g[k] = (f((q.ravel() + e).reshape(3, 3)) - f((q.ravel() - e).reshape(3, 3))) / (2 * h)
    return g


def num_lap(f, q, h=1e-4):
    s = 0.
    for k in range(9):
        e = np.zeros(9)
        e[k] = h
        s += (f((q.ravel() + e).reshape(3, 3)) - 2 * f(q) + f((q.ravel() - e).reshape(3, 3))) / h ** 2
    return s


class GeneratorIdentities(unittest.TestCase):
    def test_gradient_inner_products(self):
        rng = np.random.default_rng(11)
        for _ in range(3):
            q = rng.normal(size=(3, 3))
            G = [num_grad(lambda x, i=i: gens(x)[i], q) for i in range(3)]
            for (x, y), p in j.GRAD_DOT.items():
                want = evaluate(p, q)
                self.assertAlmostEqual(G[x] @ G[y], want, delta=1e-5 * max(1, abs(want)), msg=(x, y))

    def test_laplacians(self):
        rng = np.random.default_rng(12)
        q = rng.normal(size=(3, 3)) * .7
        for x in range(3):
            want = evaluate(j.LAPLACIAN[x], q) if j.LAPLACIAN[x] else 0.
            self.assertAlmostEqual(num_lap(lambda z: gens(z)[x], q), want, delta=1e-4 * max(1, abs(want)))
        for m in [(1, 1, 1), (2, 0, 1), (0, 2, 0), (1, 0, 2)]:
            want = evaluate(j.laplacian(m), q)
            self.assertAlmostEqual(num_lap(lambda z: evaluate({m: 1}, z), q), want, delta=1e-4 * max(1, abs(want)))

    def test_superpotential(self):
        """V = 1/2 |grad W|^2 with W = T/2 - g det q."""
        rng = np.random.default_rng(13)
        for _ in range(5):
            q, g = rng.normal(size=(3, 3)), rng.uniform(.2, 3)
            t, p, d = gens(q)
            V = t / 2 - 3 * g * d + g * g / 4 * (t * t - p)
            gradW = q - g * d * np.linalg.inv(q).T
            self.assertAlmostEqual(V, .5 * np.sum(gradW ** 2), delta=1e-9 * max(1, abs(V)))


class Moments(unittest.TestCase):
    def test_against_nine_variable_expansion(self):
        try:
            import sympy as sp
        except ImportError:
            self.skipTest("sympy unavailable")
        xs = sp.symbols("x0:9")
        Qm = sp.Matrix(3, 3, xs)
        Ts, Ps, Ds = sum(v ** 2 for v in xs), ((Qm * Qm.T) ** 2).trace(), Qm.det()

        def gm(k):
            return 0 if k % 2 else (sp.Rational(sp.factorial2(k - 1), 2 ** (k // 2)) if k else 1)

        for m in [(1, 0, 0), (0, 1, 0), (0, 0, 2), (2, 1, 0), (1, 0, 2), (0, 2, 0), (1, 1, 2), (0, 0, 1), (1, 1, 1)]:
            poly = sp.Poly(sp.expand(Ts ** m[0] * Ps ** m[1] * Ds ** m[2]), *xs)
            want = sum(c * sp.prod([gm(k) for k in mon]) for mon, c in poly.terms())
            self.assertEqual(j.moment(m), Fraction(int(sp.numer(want)), int(sp.denom(want))), m)


class Spectrum(unittest.TestCase):
    def test_free_oscillator_is_exact(self):
        """g = 0, alpha = 1: H is the 9D oscillator, closed on each degree; levels 9/2 + degree, zero residual."""
        s = j.solve(12, 0.0, 1.0, dps=40)
        E = np.array(s["energies"])
        degs = sorted(j.degree(m) for m in s["basis"])
        np.testing.assert_allclose(E, 4.5 + np.array(degs), atol=1e-11)
        self.assertLess(max(s["residuals"]), 1e-10)

    def test_operator_is_symmetric_and_refusal_is_explicit(self):
        try:
            j.solve(10, 1., 1.7, dps=40)
        except ArithmeticError as exc:
            self.fail(f"H refused: {exc}")

    def test_ground_energy_above_superpotential_floor(self):
        for g2, a in ((.5, 1.35), (1., 1.7), (2., 2.3), (4., 2.7)):
            E0 = j.solve(10, g2, a, dps=40)["energies"][0]
            self.assertGreater(E0, 4.5, (g2, E0))

    def test_ritz_values_do_not_increase_with_cutoff(self):
        prev = None
        for N in (8, 10, 12, 14):
            E = j.solve(N, 1., 1.7, dps=40)["energies"][:3]
            if prev is not None:
                for a, b in zip(E, prev):
                    self.assertLessEqual(a, b + 1e-12)
            prev = E

    def test_precision_is_sufficient(self):
        a, b = j.solve(14, 2., 2.3, dps=40), j.solve(14, 2., 2.3, dps=90)
        np.testing.assert_allclose(a["energies"][:4], b["energies"][:4], rtol=0, atol=1e-12)
        np.testing.assert_allclose(a["residuals"][:4], b["residuals"][:4], rtol=1e-9, atol=1e-14)

    def test_matches_nine_variable_gpu_controls(self):
        files = sorted(glob.glob(str(CONTROLS / "*.npz")))
        if not files:
            self.skipTest("saved control arrays unavailable")
        for f in files:
            name = Path(f).name
            N, g2 = int(name[1:3]), float(name.split("-")[2])
            alpha = float(name.split("alpha-")[1][:-4])
            z = np.load(f)
            try:
                s = j.solve(N, g2, alpha, dps=50)
            except ArithmeticError as exc:   # symmetry or conditioning refusal is a failed match, not a pass
                self.fail(f"{name}: solver refused: {exc}")
            for k, key in enumerate(("vacuum", "scalar", "scalar_next")):
                self.assertAlmostEqual(s["energies"][k], z[f"B_{key}"][0, 0], delta=1e-11, msg=(name, key))
                self.assertAlmostEqual(s["residuals"][k], z[f"R_{key}"][0, 0] ** .5, delta=1e-10, msg=(name, key))


class OrderingInputs(unittest.TestCase):
    def test_temple_quantities_and_labels(self):
        sol = {"energies": [5., 7., 9., 10.], "residuals": [.01, .1, .2, .3]}
        o = j.ordering_inputs(sol)
        self.assertAlmostEqual(o["beta"], 8.8)
        self.assertAlmostEqual(o["tau_beta"], .01 / 1.8)
        self.assertAlmostEqual(o["tau_beta_half"], .01 / .9)
        self.assertFalse(o["certified"])
        self.assertFalse(o["separation_verified"])

    def test_unusable_separation_is_refused(self):
        o = j.ordering_inputs({"energies": [5., 7., 7.3, 9.], "residuals": [.01, .2, .2, .3]})
        self.assertIsNone(o["temple_lower_beta"])


if __name__ == "__main__":
    unittest.main()
