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


class FluxBackground(unittest.TestCase):
    """The sourced-condensate variant: verified facts used in the flux-background design."""

    def test_vibrations_are_a_quarter_of_the_edge_sector_zero_hessian(self):
        from test_ym_two_grain_potential import hessian
        g = .6
        for (x, y, t) in [(-2., 1.5, 4.), (.5, 1., 4.), (1.1, 2.2, 4.)]:
            qb = np.diag([x, y, y]) / (2 * g)
            Hq = np.sort(np.linalg.eigvalsh(hessian(lambda z: bm.V(z.reshape(3, 3), g, t), qb.ravel(), h=1e-4)))
            He = np.sort(np.linalg.eigvalsh(hc.operators(x, y, t, 0)[1]))
            np.testing.assert_allclose(Hq, He / 4, atol=2e-4)

    def test_window_global_minimum_and_orbit_zero_modes(self):
        from scipy.optimize import minimize
        from test_ym_two_grain_potential import hessian
        g, t, s = .6, 4., 1.1
        x, y, qb = bm.mc_background(s, t, g)
        a, b = bm.sources(qb, g, t)
        self.assertTrue(bm.bounded(g, t, a, b))
        f = lambda z: bm.V_sourced(z.reshape(3, 3), g, t, a, b)
        ev = np.linalg.eigvalsh(hessian(f, qb.ravel(), h=1e-4))
        self.assertEqual(int(np.sum(np.abs(ev) < 1e-5)), 3)
        self.assertGreater(ev[np.abs(ev) >= 1e-5].min(), 0)
        rng = np.random.default_rng(1)
        best = min(minimize(f, rng.normal(size=9) * rng.uniform(.3, 3)).fun for _ in range(20))
        self.assertGreaterEqual(best, f(qb.ravel()) - 1e-6)
        self.assertLess(f(qb.ravel()), 0)

    def test_boundedness_limit(self):
        """Numerically s_max = 1 + 1/(t + 1) (derivation owed)."""
        g = .6
        for t in (3., 4., 6.):
            smax = 1 + 1 / (t + 1)
            for s, want in ((smax - 1e-3, True), (smax + 1e-3, False)):
                _, _, qb = bm.mc_background(s, t, g)
                self.assertEqual(bm.bounded(g, t, *bm.sources(qb, g, t)), want, (t, s))

    def test_helike_becomes_sector_zero_floor_inside_the_window(self):
        for t, (below, above) in {4.: (1.10, 1.13), 6.: (1.06, 1.08)}.items():
            for s, floor in ((below, False), (above, True)):
                x, y = 2 * s / np.sqrt(t), 2 * s
                hg, _ = hc.gauge_orthogonal(hc.operators(x, y, t, 0)[1], hc.gauge_map(x, y, t, 0))
                lam = hc.twist_closed_form(x, y, t, 0)
                self.assertEqual(bool(hg[0] >= lam - 1e-8), floor, (t, s))
            self.assertLess(above, 1 + 1 / (t + 1))


class ClosedForms(unittest.TestCase):
    def test_sources_depth_and_window(self):
        g = .6
        for s, t in ((1.1, 4.), (1.05, 6.), (1.15, 3.), (.95, 2.)):
            _, _, qb = bm.mc_background(s, t, g)
            a, b = bm.sources(qb, g, t)
            np.testing.assert_allclose((a, b), bm.mc_sources(s, t), atol=1e-6)
            self.assertAlmostEqual(bm.V_sourced(qb, g, t, a, b), bm.mc_depth(s, t, g), places=6)
        for t in (2., 4., 9.):
            self.assertAlmostEqual(bm.mc_s_max(t), 1 + 1 / (t + 1))

    def test_helike_frequency_and_flat_limit(self):
        g = .6
        for s, t in ((1.13, 4.), (1.06, 6.), (1.2, 3.)):
            self.assertAlmostEqual(bm.mc_vibrations(s, t, g)[2][0], bm.mc_helike_freq2(s, t), places=4)
        self.assertAlmostEqual(bm.mc_helike_freq2(1., 4.), 1.)          # = (2/sqrt t)^2: Step 1's protected gap

    def test_sourced_crossing_inside_window(self):
        """Helike (|Q| = 2) overtakes the Q = 0 competitor at s*_src(t) < s_max."""
        for t, (lo, hi) in {4.: (1.09, 1.11), 6.: (1.05, 1.06)}.items():
            v_lo, v_hi = bm.mc_vibrations(lo, t, .6), bm.mc_vibrations(hi, t, .6)
            self.assertGreater(v_lo[2][0], v_lo[0][0])
            self.assertLess(v_hi[2][0], v_hi[0][0])
            self.assertLess(hi, bm.mc_s_max(t))
            self.assertEqual(sum(len(v) for v in v_hi.values()), 6)      # gauge-orthogonal vibrations only
