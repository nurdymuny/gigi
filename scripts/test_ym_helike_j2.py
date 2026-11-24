"""Gates for the spin-2 covariant-sector solver (scripts/ym_helike_j2.py) and its use with quantum_ordering."""
import glob
import unittest
from pathlib import Path

import numpy as np
import scipy.linalg as la

import ym_helike_j0 as j0
import ym_helike_j2 as j2
from test_ym_helike_j0 import CONTROLS, evaluate, gens, num_grad, num_lap


def dev(X):
    return X - np.trace(X) / 3 * np.eye(3)


def A(q, k):
    G = q.T @ q
    return dev(G) if k == 1 else dev(G @ G)


def tensor_eval(tp, q):
    out = np.zeros((3, 3))
    for (m, k), c in tp.items():
        out += float(c) * evaluate({m: 1}, q) * A(q, k)
    return out


def controls():
    for f in sorted(glob.glob(str(CONTROLS / "*.npz"))):
        name = Path(f).name
        yield name, int(name[1:3]), float(name.split("-")[2]), float(name.split("alpha-")[1][:-4]), np.load(f)


def _solve(tc, fn, *args, **kw):
    """A solver refusal (symmetry or conditioning) is a failed assertion, not an error."""
    try:
        return fn(*args, **kw)
    except ArithmeticError as exc:
        tc.fail(f"solver refused: {exc}")


class Tables(unittest.TestCase):
    def setUp(self):
        self.q = np.random.default_rng(21).normal(size=(3, 3))

    def test_contractions(self):
        for (k, l), p in j2.CONTRACT.items():
            want = np.sum(A(self.q, k) * A(self.q, l))
            self.assertAlmostEqual(evaluate(p, self.q), want, delta=1e-9 * max(1, abs(want)), msg=(k, l))

    def test_directional_derivatives(self):
        for (x, k), tp in j2.DIR.items():
            gx = num_grad(lambda z: gens(z)[x], self.q)
            lhs = np.array([[num_grad(lambda z: A(z, k)[a, b], self.q) @ gx for b in range(3)] for a in range(3)])
            self.assertLess(np.abs(lhs - tensor_eval(tp, self.q)).max(), 1e-5 * max(1, np.abs(lhs).max()), (x, k))

    def test_laplacians(self):
        for k in (1, 2):
            lhs = np.array([[num_lap(lambda z: A(z, k)[a, b], self.q) for b in range(3)] for a in range(3)])
            self.assertLess(np.abs(lhs - tensor_eval(j2.LAP_A[k], self.q)).max(), 1e-4 * max(1, np.abs(lhs).max()))
        for m, k in [((1, 1, 1), 2), ((0, 1, 0), 1), ((2, 0, 1), 2)]:
            lhs = np.array([[num_lap(lambda z: evaluate({m: 1}, z) * A(z, k)[a, b], self.q) for b in range(3)]
                            for a in range(3)])
            got = tensor_eval(j2.tensor_laplacian(m, k), self.q)
            self.assertLess(np.abs(lhs - got).max(), 1e-6 * max(1, np.abs(lhs).max()), (m, k))


class Spectrum(unittest.TestCase):
    def test_free_oscillator_is_exact(self):
        s = _solve(self, j2.solve, 12, 0.0, 1.0, dps=40)
        degs = sorted(j2.item_degree(it) for it in s["basis"])
        np.testing.assert_allclose(s["energies"], 4.5 + np.array(degs), atol=1e-11)
        self.assertLess(max(s["residuals"]), 1e-10)

    def test_all_levels_above_superpotential_floor(self):
        for g2, a in ((.5, 1.35), (1., 1.7), (2., 2.3), (4., 2.7)):
            self.assertGreater(_solve(self, j2.solve, 10, g2, a, dps=40)["energies"][0], 4.5)

    def test_ritz_values_do_not_increase_with_cutoff(self):
        prev = None
        for N in (8, 10, 12, 14):
            E = _solve(self, j2.solve, N, 2., 2.3, dps=40)["energies"][:3]
            if prev is not None:
                for a, b in zip(E, prev):
                    self.assertLessEqual(a, b + 1e-12)
            prev = E

    def test_precision_is_sufficient(self):
        a, b = _solve(self, j2.solve, 14, 1., 1.7, dps=40), _solve(self, j2.solve, 14, 1., 1.7, dps=90)
        np.testing.assert_allclose(a["energies"][:4], b["energies"][:4], rtol=0, atol=1e-12)
        np.testing.assert_allclose(a["residuals"][:4], b["residuals"][:4], rtol=1e-9, atol=1e-14)

    def test_branch_carries_measured_tensor_residual(self):
        s = _solve(self, j2.solve, 12, 2., 2.3, dps=40)
        b = j2.branch(s)
        self.assertEqual((b["low"], b["residual"], b["next_low"], b["next_residual"]),
                         (s["energies"][0], s["residuals"][0], s["energies"][1], s["residuals"][1]))
        self.assertGreater(b["residual"], 0.)

    def test_operator_is_symmetric_and_refusal_is_explicit(self):
        try:
            _solve(self, j2.solve, 10, 2., 2.3, dps=40)
        except ArithmeticError as exc:
            self.fail(f"H refused: {exc}")


class NineVariableControls(unittest.TestCase):
    def test_module_is_complete(self):
        """Module dimension equals the number of spin-2 multiplets in the 9-variable basis."""
        seen = False
        for name, N, g2, alpha, z in controls():
            C = z["vectors"].T @ z["J2"] @ z["vectors"]
            count = int(np.sum(np.abs(la.eigvalsh((C + C.T) / 2) - 6) < 1e-6))
            self.assertEqual(count % 5, 0, name)
            self.assertEqual(len(j2.basis(N)), count // 5, name)
            seen = True
        if not seen:
            self.skipTest("saved control arrays unavailable")

    def test_matches_gpu_tensor_levels(self):
        seen = False
        for name, N, g2, alpha, z in controls():
            try:
                s = _solve(self, j2.solve, N, g2, alpha, dps=50)
            except ArithmeticError as exc:
                self.fail(f"{name}: solver refused: {exc}")
            for k, key in enumerate(("tensor", "tensor_next")):
                E = la.eigvalsh(z[f"B_{key}"])
                r = la.eigvalsh(z[f"R_{key}"])[-1] ** .5
                self.assertAlmostEqual(s["energies"][k], E.mean(), delta=1e-11, msg=(name, key))
                self.assertLess(E.max() - E.min(), 1e-10, (name, key))
                self.assertAlmostEqual(s["residuals"][k], r, delta=1e-10, msg=(name, key))
            seen = True
        if not seen:
            self.skipTest("saved control arrays unavailable")

    def test_reproduces_pilot_conditional_orders(self):
        """Feeding both sector solvers into quantum_ordering reproduces the six-control pilot's verdicts."""
        import quantum_ordering as qo
        want = {(10, 1.): "SCALAR_LOWER", (12, 1.): "SCALAR_LOWER", (14, 1.): "SCALAR_LOWER",
                (10, 2.): ("ORDER_UNRESOLVED", "BOUNDS_OVERLAP"),
                (12, 2.): ("ORDER_UNRESOLVED", "SEPARATION_SENSITIVE"), (14, 2.): "TENSOR_LOWER"}
        seen = False
        for name, N, g2, alpha, z in controls():
            s0, s2 = _solve(self, j0.solve, N, g2, alpha, dps=50), _solve(self, j2.solve, N, g2, alpha, dps=50)
            o = qo.ordering(j0.branch(s0), j2.branch(s2))
            w = want[(N, g2)]
            if isinstance(w, tuple):
                self.assertEqual((o["conditional_order"], o["reason"]), w, name)
            else:
                self.assertEqual(o["conditional_order"], w, name)
            self.assertFalse(o["separation_verified"])
            self.assertEqual(o["unconditional_order"], "ORDER_UNRESOLVED")
            seen = True
        if not seen:
            self.skipTest("saved control arrays unavailable")


if __name__ == "__main__":
    unittest.main()
