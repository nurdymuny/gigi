"""Design-stage gates for the Berger-deformed matrix model (no spectra)."""
import unittest

import numpy as np

import helike_berger_model as bm
import helike_curl_spectrum as hc
from test_ym_helike_second_vacuum import V as V_round


def energy_edge(M, t):
    _, c = hc.frame(t)
    E = 0.
    for i, j in [(0, 1), (1, 2), (0, 2)]:
        F = np.cross(M[i], M[j]) - sum(c[i, j, k] * M[k] for k in range(3))
        E += .5 * F @ F
    return E


class Model(unittest.TestCase):
    def test_superpotential_matches_edge_paper_curvature(self):
        rng = np.random.default_rng(101)
        for t in (.6, 1., 2.5, 7.):
            c = 2 * bm.omega(t)
            for _ in range(3):
                M = rng.normal(size=(3, 3))
                self.assertAlmostEqual(energy_edge(M, t), .5 * np.sum((c[:, None] * M - bm.cof(M)) ** 2), places=9)

    def test_round_limit_is_the_existing_model(self):
        rng = np.random.default_rng(102)
        for g in (.5, 1.3):
            q = rng.normal(size=(3, 3))
            self.assertAlmostEqual(bm.V(q, g, 1.), V_round(q, g), places=10)

    def test_three_term_decomposition(self):
        rng = np.random.default_rng(103)
        for t in (.5, 4.):
            q = rng.normal(size=(3, 3))
            self.assertAlmostEqual(bm.V(q, .8, t), sum(bm.V_parts(q, .8, t)), places=10)

    def test_floor_and_round_value(self):
        """(3/2)(sqrt t + 2/sqrt t): 4.5 at t = 1 and t = 4, smallest at t = 2 (3 sqrt 2)."""
        self.assertAlmostEqual(bm.floor(1.), 4.5)
        self.assertAlmostEqual(bm.floor(4.), 4.5)
        self.assertAlmostEqual(bm.floor(2.), 3 * np.sqrt(2))
        self.assertLess(bm.floor(2.), min(bm.floor(1.5), bm.floor(2.5)))

    def test_channels_carry_their_long_axis_charge(self):
        rng = np.random.default_rng(104)
        q = rng.normal(size=(3, 3))
        phi = .37
        R = np.array([[1, 0, 0], [0, np.cos(phi), -np.sin(phi)], [0, np.sin(phi), np.cos(phi)]])
        a, b = bm.probe_channels(q), bm.probe_channels(R @ q)
        for m in (0, 1, 2):
            self.assertAlmostEqual(abs(b[m]), abs(a[m]), places=10)
            if m:
                self.assertAlmostEqual(np.angle(b[m] / a[m]) % (2 * np.pi), (m * phi) % (2 * np.pi), places=9)


class Budget(unittest.TestCase):
    def test_degree_two_content(self):
        self.assertEqual(bm.sector_counts(4, 0)[2], 2)      # G_00 and G_11 + G_22
        self.assertEqual(bm.sector_counts(4, 2)[2], 1)      # Helike channel
        self.assertEqual(bm.sector_counts(4, 1)[2], 1)

    def test_round_marginal_contains_the_one_grain_count(self):
        """SO(2) sectors refine the round J sectors: m = 0 at cutoff N must be at least the round J=0 count."""
        import ym_helike_j0 as j0
        H0 = bm.sector_counts(14, 0)
        self.assertGreaterEqual(int(H0[:15].sum()), len(j0.basis(14)))


if __name__ == "__main__":
    unittest.main()
