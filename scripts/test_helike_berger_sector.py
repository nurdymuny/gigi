"""Gates for the Berger probe's Step 1 (perturbation theory) and the colour SO(3) x SO(2) sector solver."""
import math
import unittest

import numpy as np

import helike_berger_model as bm
import helike_berger_pert as bp
import helike_berger_sector as bs


def _solve(tc, *a, **k):
    """A solver refusal (asymmetric operator, failed Cholesky) is a failed assertion, not an error."""
    try:
        return bs.solve(*a, **k)
    except (ArithmeticError, ValueError, ZeroDivisionError) as exc:
        tc.fail(f"Berger sector solver refused: {exc}")


class Perturbation(unittest.TestCase):
    def test_round_scalar_coefficient(self):
        st = {}
        for k in range(3):
            for a in range(3):
                st = bp.add(st, bp.create(bp.create({bp.VAC: 1.}, k, a), k, a))
        vq, vc = bp.shifts({bp.VAC: 1.}, 1., 1.)
        q4, q3 = bp.shifts(bp.normalise(st), 1., 1.)
        self.assertAlmostEqual(q4 + q3 - vq - vc, -12., places=9)
        self.assertAlmostEqual(vq + vc, 0., places=10)         # E0 = 9/2 to all orders (formal e^{-W})

    def test_berger_splitting_closed_form(self):
        for t in (1.5, 2., 3., 6.):
            r = bp.splitting(1., t)
            self.assertAlmostEqual(r[2]["total"], 0., places=10)                 # Helike's channel protected
            self.assertAlmostEqual(r["split"]["total"], 4 + 4 / t, places=9)
            self.assertAlmostEqual(r["split"]["quartic"], -t, places=9)

    def test_protected_channel_is_a_formal_eigenfunction(self):
        """H (f e^{-W}) = (floor + 2/sqrt t) f e^{-W} pointwise, f = sum_a (q_1a + i q_2a)^2."""
        rng = np.random.default_rng(5)
        t, g = 2.5, .7
        f = lambda q: np.sum((q[1] + 1j * q[2]) ** 2)
        psi = lambda q: f(q) * np.exp(-bm.W(q, g, t))
        h = 1e-3
        for _ in range(3):
            q = rng.normal(size=(3, 3)) * .4
            lap = sum((psi(q + e) - 2 * psi(q) + psi(q - e)) / h ** 2
                      for e in (np.eye(9)[k].reshape(3, 3) * h for k in range(9)))
            Hpsi = -.5 * lap + bm.V(q, g, t) * psi(q)
            want = (bm.floor(t) + 2 / math.sqrt(t)) * psi(q)
            self.assertLess(abs(Hpsi - want), 2e-4 * max(1, abs(want)))


class SectorSolver(unittest.TestCase):
    def test_dimensions_match_molien(self):
        for N in (6, 8):
            for m in (1, 2):
                self.assertEqual(len(bs.sector_basis(N, m, 1)), int(bm.sector_counts(N, m)[:N + 1].sum()))
                self.assertEqual(len(bs.sector_basis(N, m, -1)), int(bm.sector_counts(N, m)[:N + 1].sum()))
            self.assertEqual(len(bs.sector_basis(N, 0, 1)) + len(bs.sector_basis(N, 0, -1)),
                             int(bm.sector_counts(N, 0)[:N + 1].sum()))

    def test_round_limit_reproduces_verified_solvers(self):
        import ym_helike_j0 as j0
        import ym_helike_j2 as j2
        N, g2 = 6, .5
        r0, r2 = j0.solve(N, g2, 1., dps=40), j2.solve(N, g2, 1., dps=40)
        s0 = _solve(self, N, g2, 1., 0, 1, dps=40, nlev=None)
        s2 = _solve(self, N, g2, 1., 2, 1, dps=40, nlev=None)
        for ref, sol in ((r0, s0), (r2, s2)):
            E, R = np.array(sol["energies"]), np.array(sol["residuals"])
            for k in range(2):
                i = int(np.argmin(abs(E - ref["energies"][k])))
                self.assertAlmostEqual(E[i], ref["energies"][k], places=11)
                self.assertAlmostEqual(R[i], ref["residuals"][k], places=9)

    def test_step1_slopes(self):
        t, g2s, out = 2., (5e-4, 1e-3), []
        for g2 in g2s:
            v = _solve(self, 8, g2, t, 0, 1, dps=40, nlev=3)
            h = _solve(self, 8, g2, t, 2, 1, dps=40, nlev=2)
            E0 = v["energies"][0]
            out.append((h["energies"][0] - E0, v["energies"][1] - E0))
        dg = g2s[1] - g2s[0]
        self.assertLess(abs((out[1][0] - out[0][0]) / dg), .01)
        self.assertAlmostEqual((out[1][1] - out[0][1]) / dg, -(4 + 4 / t), delta=.15)


if __name__ == "__main__":
    unittest.main()
