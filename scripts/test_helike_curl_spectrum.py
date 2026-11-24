"""Gates for the Berger curl-spectrum check (scripts/helike_curl_spectrum.py)."""
import unittest

import numpy as np

import helike_curl_spectrum as hc

POINTS = [(-2., 1.5, 4., 3), (-1., .7, 6., 2), (*hc.berger_xy(4.), 4., 5), (*hc.berger_xy(7.), 7., 4)]


class Operators(unittest.TestCase):
    def test_curl_is_hermitian_and_squares_to_the_derivative_term(self):
        for x, y, t, j in POINTS:
            curl, H, Phi, Dall, _ = hc.operators(x, y, t, j)
            np.testing.assert_allclose(curl, curl.conj().T, atol=1e-12)
            np.testing.assert_allclose(Dall.conj().T @ Dall, curl.conj().T @ curl, atol=1e-12)
            np.testing.assert_allclose(H, curl @ curl + Phi, atol=1e-11)

    def test_berger_point_is_the_levi_civita_field(self):
        import ricci_su2_spin_sector as rs
        for t in (2.5, 4., 9.):
            _, _, m, _ = rs.background([t, 1, 1])
            x, y = hc.berger_xy(t)
            np.testing.assert_allclose(np.array(m), np.array(hc.family_field(x, y)), atol=1e-12)


class Twist(unittest.TestCase):
    def test_helicity_minus_flux(self):
        """The twist is an exact curl eigenvector, mu = -(x + (j+2)/sqrt t), and lambda = mu^2 - B."""
        for x, y, t, j in POINTS:
            curl, H, Phi, _, _ = hc.operators(x, y, t, j)
            lam = hc.twist_closed_form(x, y, t, j)
            V = hc.twist_space(H, lam)
            self.assertEqual(V.shape[1], 2)
            mu, leak = hc.curl_on(V, curl)
            self.assertLess(leak, 1e-10)
            np.testing.assert_allclose(mu, -(x + (j + 2) / np.sqrt(t)), atol=1e-10)
            np.testing.assert_allclose(lam - mu ** 2, -hc.fibre_field(x, y, t), atol=1e-10)
            np.testing.assert_allclose(np.real(np.diag(V.conj().T @ Phi @ V)), -hc.fibre_field(x, y, t), atol=1e-10)


class Windows(unittest.TestCase):
    def test_reproduces_the_published_lower_edge(self):
        edge = hc.bisect_edge("H_every_sector", 2.6, 2.75, 12, iters=30)
        self.assertIsNotNone(edge, "no floor edge between t = 2.6 and 2.75")
        self.assertAlmostEqual(edge, 2.6980, delta=2e-4)

    def test_twist_is_not_the_curl_floor_inside_the_window(self):
        for t in (3., 6., 10.):
            r = hc.per_sector_status(t, 10)
            self.assertTrue(r["H_every_sector"], t)
            self.assertFalse(r["curl_every_sector"], t)
        self.assertTrue(hc.per_sector_status(4., 10)["curl_every_sector"])

    def test_twist_minimises_the_flux_term_on_the_berger_path(self):
        for t in (2.75, 6., 10.):
            x, y = hc.berger_xy(t)
            low = min(np.linalg.eigvalsh(hc.operators(x, y, t, j)[2])[0] for j in range(9))
            self.assertAlmostEqual(low, -hc.fibre_field(x, y, t), places=9)

    def test_flux_bound_fails_for_positive_x(self):
        x, y, t = .5, 1., 4.
        low = min(np.linalg.eigvalsh(hc.operators(x, y, t, j)[2])[0] for j in range(7))
        self.assertLess(low, -hc.fibre_field(x, y, t) - .1)


if __name__ == "__main__":
    unittest.main()
