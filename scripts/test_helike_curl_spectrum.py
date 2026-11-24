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


def phi_spectrum(B, k):
    r = np.sqrt(B * B + 8 * k * k)
    return np.sort([-B, -B, B, k, -k, k, -k, (B + r) / 2, (B - r) / 2])


class FluxTheorem(unittest.TestCase):
    """Theorem 1: Phi = phi (x) I_{V_j}, spec phi = {-B (x2), B, +-kappa (x2), (B +- sqrt(B^2 + 8 kappa^2))/2}."""

    def test_spectrum_formula_and_sector_independence(self):
        rng = np.random.default_rng(91)
        for _ in range(6):
            x, y, t = rng.uniform(-3, 2), rng.uniform(.2, 3), rng.uniform(.5, 12)
            B, k = hc.fibre_field(x, y, t), y * (x - 2 / np.sqrt(t))
            np.testing.assert_allclose(np.linalg.eigvalsh(hc.operators(x, y, t, 0)[2]), phi_spectrum(B, k), atol=1e-12)
            ev3 = np.linalg.eigvalsh(hc.operators(x, y, t, 3)[2])
            np.testing.assert_allclose(ev3, np.repeat(phi_spectrum(B, k), 4), atol=1e-11)

    def test_flux_bound_holds_exactly_from_t_equals_two_on_berger(self):
        for t, holds in ((1.9, False), (2.0, True), (2.1, True), (8., True)):
            x, y = hc.berger_xy(t)
            B, k = hc.fibre_field(x, y, t), y * (x - 2 / np.sqrt(t))
            self.assertEqual(bool(phi_spectrum(B, k).min() >= -B - 1e-12), holds, t)

    def test_cauchy_riemann_leak_equals_gauge_component(self):
        for (x, y, t, j) in [(-2., 1.5, 4., 3), (.5, 1., 4., 3), (*hc.berger_xy(9.), 9., 5), (1.2, .4, 1.5, 4)]:
            C, H, Phi, _, _ = hc.operators(x, y, t, j)
            B = hc.fibre_field(x, y, t)
            w, V = np.linalg.eigh(Phi)
            P = V[:, np.abs(w + B) < 1e-9]
            self.assertEqual(P.shape[1], 2 * (j + 1))
            _, U = np.linalg.eigh(P.conj().T @ C @ P)
            psis = P @ U
            leak = np.linalg.norm((np.eye(len(C)) - P @ P.conj().T) @ C @ psis, axis=0) ** 2
            gs = np.linalg.norm(hc.gauge_map(x, y, t, j).conj().T @ psis, axis=0) ** 2
            np.testing.assert_allclose(leak, gs, atol=1e-10)
            l = j / 2
            want = np.sort(np.repeat(2 * (l * (l + 1) - np.arange(-l, l + 1) * (np.arange(-l, l + 1) + 1)), 2))
            np.testing.assert_allclose(np.sort(leak), want, atol=1e-9)


def flux_aligned_state(j, m, branch, x, y, t):
    """psi = f_m (x) (theta1 +- i theta2) (x) (tau1 +- i tau2) in the builder's (frame, Lie, V_j) index.
    f_m: J_0 f = m f, with J_a = (i/2) rho_a and J_0 the long-axis generator."""
    import ricci_su2_spin_sector as rs
    rho = rs.spin_rep(j)
    J = [0.5j * r for r in rho]
    w, V = np.linalg.eigh(J[0])
    f = V[:, np.argmin(np.abs(w - m))]
    d = j + 1
    s = 1 if branch == "+" else -1
    u = np.array([0, 1, 1j * s])
    psi = np.zeros(9 * d, complex)
    for i, coef in ((1, 1.), (2, 1j * s)):
        psi[i * 3 * d:(i + 1) * 3 * d] = coef * np.kron(u, f)
    Jpm = J[1] + 1j * s * J[2]                       # J_+ for "+", J_- for "-"
    return psi, f, u, Jpm


class CauchyRiemann(unittest.TestCase):
    """curl psi = mu psi + chi, chi only in the frame-0 slot, chi = -+ i G* psi = 2 (+-) (J_+- f) (x) u; Phi psi = -B psi."""

    def test_vector_identities_both_branches(self):
        for (x, y, t, j) in [(-2., 1.5, 4., 4), (.7, .3, 1.5, 3), (*hc.berger_xy(6.), 6., 5)]:
            C, H, Phi, _, _ = hc.operators(x, y, t, j)
            G = hc.gauge_map(x, y, t, j)
            B, d, l = hc.fibre_field(x, y, t), j + 1, j / 2
            for branch in ("+", "-"):
                for m in np.arange(-l, l + 1):
                    psi, f, u, Jpm = flux_aligned_state(j, m, branch, x, y, t)
                    mm = m if branch == "+" else -m
                    mu = -(x + (2 * mm + 2) / np.sqrt(t))
                    chi = C @ psi - mu * psi
                    np.testing.assert_allclose(chi[3 * d:], 0, atol=1e-11)          # only the frame-0 slot
                    sgn = 1 if branch == "+" else -1
                    np.testing.assert_allclose(chi[:3 * d], 2 * sgn * np.kron(u, Jpm @ f), atol=1e-11)
                    np.testing.assert_allclose(chi[:3 * d], -1j * sgn * (G.conj().T @ psi), atol=1e-11)
                    np.testing.assert_allclose(Phi @ psi, -B * psi, atol=1e-11)
                    ratio = np.linalg.norm(chi) ** 2 / np.linalg.norm(psi) ** 2
                    self.assertAlmostEqual(ratio, 2 * (l * (l + 1) - mm * (mm + 1)), places=9)
