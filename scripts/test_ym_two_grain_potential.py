"""Phase 1 gates for the two-grain potential (scripts/ym_two_grain_potential.py)."""
import unittest

import numpy as np

import ym_two_grain_potential as tg


def V18(q, p, g):
    """Closed form found in Phase 1, in model units (H = -1/2 Lap + V18)."""
    Qq, Qp = q @ q.T, p @ p.T
    return (tg.one_grain(g * q) / g ** 2 + tg.one_grain(g * p, -1.) / g ** 2
            + g * g / 3 * (np.trace(Qq) * np.trace(Qp) - np.trace(Qq @ Qp)))


def grad_W(M, s):
    """Gradient of T/2 - s det M."""
    return M - s * np.linalg.det(M) * np.linalg.inv(M).T


def hessian(f, x0, h=1e-4):
    n = len(x0)
    H = np.zeros((n, n))
    for a in range(n):
        for b in range(n):
            ea, eb = np.zeros(n), np.zeros(n)
            ea[a], eb[b] = h, h
            H[a, b] = (f(x0 + ea + eb) - f(x0 + ea - eb) - f(x0 - ea + eb) + f(x0 - ea - eb)) / (4 * h * h)
    return (H + H.T) / 2


def random_rotation(rng):
    Q, R = np.linalg.qr(rng.normal(size=(3, 3)))
    Q = Q @ np.diag(np.sign(np.diag(R)))
    return Q if np.linalg.det(Q) > 0 else -Q


class ExactAverage(unittest.TestCase):
    def test_group_and_moments(self):
        G, R = tg.binary_icosahedral(), tg.rotations()
        self.assertEqual(len(G), 120)
        self.assertEqual(len({tuple(np.round(r, 8).ravel()) for r in R}), 60)
        np.testing.assert_allclose(R.mean(0), 0, atol=1e-14)
        DD = np.einsum("nij,nkl->ijkl", R, R) / len(R)
        np.testing.assert_allclose(DD, np.einsum("ik,jl->ijkl", np.eye(3), np.eye(3)) / 3, atol=1e-14)
        self.assertAlmostEqual(np.mean(R[:, 0, 0] ** 4), 1 / 5, places=13)      # D_11 uniform on [-1, 1]
        self.assertAlmostEqual(np.mean(R[:, 0, 0] * R[:, 1, 1] * R[:, 2, 2]), 1 / 6, places=13)


class Potential(unittest.TestCase):
    def setUp(self):
        self.rng = np.random.default_rng(61)

    def test_rejects_unidentifiable_polynomial_degree(self):
        M, N = np.eye(3), 2 * np.eye(3)
        with self.assertRaises(ValueError):
            tg.bidegree_parts(M, N, maxdeg=5)

    def test_frame_derivative_against_group_motion(self):
        u = self.rng.normal(size=4); u /= np.linalg.norm(u)
        x = tg.quat_to_su2(*u)
        D = tg.adjoint_inverse(x)
        h = 1e-5
        for m in range(3):
            ep = np.cos(h/2)*np.eye(2) + 2*np.sin(h/2)*tg.TAU[m]
            em = np.cos(h/2)*np.eye(2) - 2*np.sin(h/2)*tg.TAU[m]
            measured = (tg.adjoint_inverse(x @ ep) - tg.adjoint_inverse(x @ em))/(2*h)
            np.testing.assert_allclose(measured, -D @ tg.EPS[m], atol=1e-9, rtol=0)

    def test_independent_colour_and_spatial_rotations(self):
        M, N = self.rng.normal(size=(3, 3)), self.rng.normal(size=(3, 3))
        C, L, R = (random_rotation(self.rng) for _ in range(3))
        self.assertAlmostEqual(tg.energy(C @ M @ L.T, C @ N @ R.T), tg.energy(M, N), places=10)

    def test_cross_term_can_vanish_with_both_grains_nonzero(self):
        M, N = np.zeros((3, 3)), np.zeros((3, 3))
        M[0] = [1., 2., -1.]; N[0] = [-2., 1., 3.]
        cross = sum(np.sum(np.cross(M[:,i], N[:,j])**2) for i in range(3) for j in range(3))/3
        self.assertEqual(cross, 0.)
        self.assertGreater(np.linalg.norm(M)*np.linalg.norm(N), 0.)
        self.assertAlmostEqual(tg.energy(M, N), tg.one_grain(M)+tg.one_grain(N, -1.), places=10)

    def test_single_grain_limits(self):
        M = self.rng.normal(size=(3, 3))
        self.assertAlmostEqual(tg.energy(M, 0 * M), tg.one_grain(M), places=10)
        self.assertAlmostEqual(tg.energy(0 * M, M), tg.one_grain(M, -1.), places=10)

    def test_E1_only_allowed_bidegrees(self):
        M, N = self.rng.normal(size=(3, 3)), self.rng.normal(size=(3, 3))
        parts = tg.bidegree_parts(M, N)
        allowed = {(2, 0), (3, 0), (4, 0), (0, 2), (0, 3), (0, 4), (2, 2)}
        for k, v in parts.items():
            if k not in allowed:
                self.assertLess(abs(v), 1e-9, k)
        self.assertGreater(abs(parts[(2, 2)]), 1e-3)

    def test_cross_term_is_one_third_of_colour_cross_products(self):
        for _ in range(3):
            M, N = self.rng.normal(size=(3, 3)), self.rng.normal(size=(3, 3))
            c22 = tg.bidegree_parts(M, N)[(2, 2)]
            want = (np.trace(M @ M.T) * np.trace(N @ N.T) - np.trace(M @ M.T @ N @ N.T)) / 3
            cross = sum(np.sum(np.cross(M[:, i], N[:, j]) ** 2) for i in range(3) for j in range(3)) / 3
            self.assertAlmostEqual(c22, want, delta=1e-9 * max(1, abs(want)))
            self.assertAlmostEqual(want, cross, places=10)

    def test_closed_form_matches_average(self):
        for g in (.5, 1., 1.7):
            q, p = self.rng.normal(size=(3, 3)), self.rng.normal(size=(3, 3))
            self.assertAlmostEqual(V18(q, p, g), tg.energy(g * q, g * p) / g ** 2, delta=1e-9)

    def test_E2_projected_gradient_has_opposite_grain_signs(self):
        """P*F = (-grad W_L(M), +grad W_R(N)): the restricted Chern-Simons functional is W_R - W_L."""
        for _ in range(3):
            M, N = self.rng.normal(size=(3, 3)), self.rng.normal(size=(3, 3))
            cM, cN = tg.projected_gradient(M, N)
            np.testing.assert_allclose(cM, -grad_W(M, 1.), atol=1e-10)
            np.testing.assert_allclose(cN, grad_W(N, -1.), atol=1e-10)

    def test_E3_energy_is_projected_gradient_plus_cross_term(self):
        for _ in range(3):
            M, N = self.rng.normal(size=(3, 3)), self.rng.normal(size=(3, 3))
            cM, cN = tg.projected_gradient(M, N)
            C = tg.energy(M, N) - .5 * (np.sum(cM ** 2) + np.sum(cN ** 2))
            want = (np.trace(M @ M.T) * np.trace(N @ N.T) - np.trace(M @ M.T @ N @ N.T)) / 3
            self.assertAlmostEqual(C, want, delta=1e-9 * max(1, want))
            self.assertGreaterEqual(C, 0)

    def test_parity_exchanges_grains(self):
        for g in (.6, 1.4):
            q, p = self.rng.normal(size=(3, 3)), self.rng.normal(size=(3, 3))
            self.assertAlmostEqual(V18(q, p, g), V18(-p, -q, g), places=10)

    def test_floors_depend_on_sweep(self):
        """Laplacian of W_R(p) - W_L(q) (global Chern-Simons) is 0; per-grain W_L(q) + W_R(p) gives 18."""
        g = .8
        WL = lambda q: .5 * np.sum(q * q) - g * np.linalg.det(q)
        WR = lambda p: .5 * np.sum(p * p) + g * np.linalg.det(p)
        x0 = self.rng.normal(size=18) * .5
        lap = lambda f: np.trace(hessian(f, x0))
        self.assertAlmostEqual(lap(lambda x: WR(x[9:].reshape(3, 3)) - WL(x[:9].reshape(3, 3))), 0., places=5)
        self.assertAlmostEqual(lap(lambda x: WR(x[9:].reshape(3, 3)) + WL(x[:9].reshape(3, 3))), 18., places=5)


class Vacua(unittest.TestCase):
    def test_three_vacuum_sets_and_the_mixed_point(self):
        rng = np.random.default_rng(62)
        g = .7
        R1, R2 = random_rotation(rng), random_rotation(rng)
        f = lambda x: V18(x[:9].reshape(3, 3), x[9:].reshape(3, 3), g)
        for q0, p0 in ((0 * R1, 0 * R1), (R1 / g, 0 * R1), (0 * R1, -R2 / g)):
            x0 = np.concatenate([q0.ravel(), p0.ravel()])
            self.assertAlmostEqual(f(x0), 0., places=12)
        self.assertAlmostEqual(f(np.concatenate([(R1 / g).ravel(), (-R2 / g).ravel()])), 2 / g ** 2, places=10)

    def test_hessian_spectra_and_zero_point_offset(self):
        g = .7
        f = lambda x: V18(x[:9].reshape(3, 3), x[9:].reshape(3, 3), g)
        origin = np.linalg.eigvalsh(hessian(f, np.zeros(18)))
        np.testing.assert_allclose(origin, np.ones(18), atol=1e-5)
        want = np.sort([0.] * 3 + [1.] + [7 / 3] * 9 + [4.] * 5)
        for x0 in (np.concatenate([(np.eye(3) / g).ravel(), np.zeros(9)]),
                   np.concatenate([np.zeros(9), (-np.eye(3) / g).ravel()])):
            ev = np.linalg.eigvalsh(hessian(f, x0))
            np.testing.assert_allclose(ev, want, atol=1e-5)
            zp = .5 * np.sum(np.sqrt(np.clip(ev, 0, None)))
            self.assertAlmostEqual(zp - 9., 4.5 * np.sqrt(7 / 3) - 3.5, places=4)


class Phase2(unittest.TestCase):
    """Sphaleron plane (van Baal-Hari Dass eq. (88)): normalization, critical points, Chern-Simons values."""

    def test_plane_is_the_18_mode_potential_and_matches_eq_92(self):
        rng = np.random.default_rng(71)
        for g in (.6, 1.3):
            for u, v in rng.normal(size=(4, 2)):
                self.assertAlmostEqual(tg.plane_potential(u, v), 32 * np.pi ** 2 * g * g * tg.potential(*tg.embed(u, v, g), g),
                                       delta=1e-9 * max(1, tg.plane_potential(u, v)))
        for a in (-.4, .3, .9):                       # eq. (54) at eps = 0, u = a + 1
            self.assertAlmostEqual(tg.plane_potential(a + 1, 0.), 3 * np.pi ** 2 * (1 - a * a) ** 2, places=10)
        u, v = .7, 1.3                                 # eq. (92) cross term with c = -u delta, d = -v delta
        cross = tg.plane_potential(u, v) - tg.plane_potential(u, 0) - tg.plane_potential(0, v)
        self.assertAlmostEqual(cross, 2 * np.pi ** 2 / 3 * (9 * u * u * v * v - 3 * u * u * v * v), places=9)

    def test_critical_points_and_chern_simons(self):
        pts = tg.plane_critical_points()
        want = {(0, 0): (0., 0.), (2, 0): (0., 1.), (0, 2): (0., -1.), (1, 0): (1., .5), (0, 1): (1., -.5)}
        self.assertEqual(len(pts), 5)
        g = .9
        for x in pts:
            key = (int(round(x[0])), int(round(x[1])))
            self.assertIn(key, want)
            np.testing.assert_allclose(x, key, atol=1e-8)
            e, Q = want[key]
            self.assertAlmostEqual(tg.plane_potential(*x) / (3 * np.pi ** 2), e, places=8)
            self.assertAlmostEqual(tg.chern_simons(*tg.embed(*x, g), g), Q, places=8)
        for t in (.2, 1., 1.8):                        # "Q vanishes along the line c = d" (p. 213)
            self.assertAlmostEqual(tg.chern_simons(*tg.embed(t, t, g), g), 0., places=12)

    def test_sphaleron_spectrum_is_coupling_independent(self):
        want = np.sort([-.5] + [0.] * 3 + [4 / 3] * 9 + [2.5] * 5)
        for g in (.6, 1.1):
            f = lambda y: tg.potential(y[:9].reshape(3, 3), y[9:].reshape(3, 3), g)
            for u, v in ((1, 0), (0, 1)):
                q, p = tg.embed(u, v, g)
                ev = np.linalg.eigvalsh(hessian(f, np.concatenate([q.ravel(), p.ravel()])))
                np.testing.assert_allclose(ev, want, atol=2e-5)


if __name__ == "__main__":
    unittest.main()
