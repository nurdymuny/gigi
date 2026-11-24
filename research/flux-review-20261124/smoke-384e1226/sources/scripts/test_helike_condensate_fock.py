"""Gates for the local Fock solver of the sourced condensate."""
import math
import unittest

import numpy as np

import helike_berger_model as bm
import helike_condensate_fock as lf
import run_helike_flux_background as run


class LocalFockSolver(unittest.TestCase):
    def test_harmonic_limit_is_exact(self):
        """Potential cut at order 2, no curvature term, no kinetic cubic: exactly the six oscillators."""
        L = lf.LocalFock(1.15, 3., math.sqrt(.05), P=2, Mq=2, include_U=False, include_kinetic_cubic=False)
        e = np.linalg.eigvalsh(L.H)
        gaps = e - e[0]
        for w in L.w:                                   # every single-quantum frequency appears exactly
            self.assertLess(np.abs(gaps - w).min(), 1e-6)
        for a in range(6):                              # and every two-quantum sum (Mq = 2)
            for b in range(a, 6):
                self.assertLess(np.abs(gaps - L.w[a] - L.w[b]).min(), 1e-6)
        self.assertLess(L.asym, 1e-10)

    def test_small_g_reproduces_perturbation_theory(self):
        """(gap - omega)/g^2 extrapolated linearly in g^2 to 0 equals the exact O(g^2) coefficients."""
        s, t = 1.15, 3.
        wH = math.sqrt(bm.mc_helike_freq2(s, t))
        vals = {}
        for g2 in (.004, .002):
            L = lf.LocalFock(s, t, math.sqrt(g2), P=4, Mq=4)
            r = L.helike_and_competitor()
            comp_w = min(L.w[n] for n in range(6) if sum(abs(L.w[n] - L.w[m]) < 1e-6 for m in range(6)) == 1)
            vals[g2] = ((r["helike"]["gap"] - wH) / g2, (r["competitor"]["gap"] - comp_w) / g2)
            self.assertGreater(min(r["helike"]["weight"], r["competitor"]["weight"]), .95)
        for k, want in ((0, -.5300), (1, -2.7379)):
            c0 = 2 * vals[.002][k] - vals[.004][k]
            self.assertAlmostEqual(c0, want, delta=.03)

    def test_switch_point_interpolation(self):
        self.assertAlmostEqual(run.switch_point([1., 2.], [-1., 1.]), 1.5)
        self.assertIsNone(run.switch_point([1., 2., 3.], [1., 2., 3.]))


if __name__ == "__main__":
    unittest.main()
