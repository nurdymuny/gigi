"""Local geometry of W = T/2 - g det q at its two critical sets (docs/research/helike-second-vacuum-weak-coupling-2026-11-24.md).

At q = 0, Hess W = I. At q = I/g, Hess W = I + L_colour . L_space, with eigenvalue mu_J = J(J+1)/2 - 1 on total
spin J = 0, 1, 2 (multiplicities 1, 3, 5), independent of g. Also Hess V = (Hess W)^2 at both critical points.
"""
import unittest

import numpy as np

EPS = np.zeros((3, 3, 3))
for i, j, k in [(0, 1, 2), (1, 2, 0), (2, 0, 1)]:
    EPS[i, j, k], EPS[i, k, j] = 1, -1
GEN = [-EPS[a] for a in range(3)]          # real antisymmetric spin-1 generators


def W(q, g):
    return .5 * np.sum(q * q) - g * np.linalg.det(q)


def V(q, g):
    Q = q @ q.T
    t = np.trace(Q)
    return t / 2 - 3 * g * np.linalg.det(q) + g * g / 4 * (t * t - np.trace(Q @ Q))


def hessian(f, q0, h=1e-4):
    H = np.zeros((9, 9))
    for a in range(9):
        for b in range(9):
            ea, eb = np.zeros(9), np.zeros(9)
            ea[a], eb[b] = h, h
            val = lambda e: f(q0 + e.reshape(3, 3))
            H[a, b] = (val(ea + eb) - val(ea - eb) - val(eb - ea) + val(-ea - eb)) / (4 * h * h)
    return (H + H.T) / 2


def colour_space_coupling():
    """L_c . L_s acting on delta (row = colour, column = space): -sum_a T_a delta T_a^T."""
    M = np.zeros((9, 9))
    for c in range(9):
        d = np.zeros(9)
        d[c] = 1
        M[:, c] = (-sum(T @ d.reshape(3, 3) @ T.T for T in GEN)).ravel()
    return M


def total_casimir():
    """J^2 = (L_c + L_s)^2 on delta, with L_c acting on rows and L_s on columns."""
    gens = []
    for T in GEN:
        G = np.zeros((9, 9))
        for c in range(9):
            d = np.zeros(9)
            d[c] = 1
            D = d.reshape(3, 3)
            G[:, c] = (T @ D + D @ T.T).ravel()
        gens.append(G)
    return -sum(G @ G for G in gens)


class SecondVacuum(unittest.TestCase):
    def test_origin_hessian_is_identity(self):
        np.testing.assert_allclose(hessian(lambda q: W(q, .7), np.zeros((3, 3))), np.eye(9), atol=1e-6)

    def test_second_vacuum_hessian_is_colour_space_coupling(self):
        LL = colour_space_coupling()
        for g in (.4, .8, 1.5):
            H = hessian(lambda q: W(q, g), np.eye(3) / g)
            np.testing.assert_allclose(H, np.eye(9) + LL, atol=1e-6)

    def test_eigenvalues_follow_total_spin(self):
        J2 = total_casimir()
        np.testing.assert_allclose(J2, J2.T, atol=1e-12)
        LL = colour_space_coupling()
        np.testing.assert_allclose(J2 @ LL, LL @ J2, atol=1e-12)
        mu = np.eye(9) + LL
        vals, vecs = np.linalg.eigh(J2)
        for J, mult, want in ((0, 1, -1.), (1, 3, 0.), (2, 5, 2.)):
            sel = np.abs(vals - J * (J + 1)) < 1e-9
            self.assertEqual(int(sel.sum()), mult)
            block = vecs[:, sel].T @ mu @ vecs[:, sel]
            np.testing.assert_allclose(block, want * np.eye(mult), atol=1e-12)
            self.assertAlmostEqual(want, J * (J + 1) / 2 - 1)

    def test_potential_hessian_is_square(self):
        for g, q0 in ((.9, np.zeros((3, 3))), (.9, np.eye(3) / .9)):
            HW = hessian(lambda q: W(q, g), q0)
            HV = hessian(lambda q: V(q, g), q0)
            np.testing.assert_allclose(HV, HW @ HW, atol=1e-5)

    def test_witten_local_energy_count(self):
        """1/2 sum(|mu| - mu): 0 at the origin, 1 at the second vacuum."""
        mu0 = np.ones(9)
        mu1 = np.linalg.eigvalsh(np.eye(9) + colour_space_coupling())
        self.assertAlmostEqual(.5 * np.sum(np.abs(mu0) - mu0), 0.)
        self.assertAlmostEqual(.5 * np.sum(np.abs(mu1) - mu1), 1.)

    def test_direct_eigenspaces_not_only_eigenvalues(self):
        H = np.eye(9) + colour_space_coupling()
        D = np.array([[2., 3., 5.], [7., 11., 13.], [17., 19., 23.]])
        breathing = np.trace(D) * np.eye(3) / 3
        orbit = (D - D.T) / 2
        shape = (D + D.T) / 2 - breathing
        for part, value in ((breathing, -1), (orbit, 0), (shape, 2)):
            np.testing.assert_allclose((H @ part.ravel()).reshape(3, 3), value * part, atol=1e-12)
        np.testing.assert_allclose(breathing+orbit+shape, D, atol=1e-12)
        for a,b in ((breathing,orbit),(breathing,shape),(orbit,shape)):
            self.assertAlmostEqual(np.sum(a*b), 0.)

    def test_frame_transports_around_the_same_orbit(self):
        H0 = np.eye(9) + colour_space_coupling()
        for angle in (.37, 1.2):
            c,s = np.cos(angle),np.sin(angle)
            R = np.array([[c,-s,0],[s,c,0],[0,0,1.]])
            U = np.kron(R,np.eye(3))
            for g in (1.,np.sqrt(2.)):
                H = hessian(lambda q: W(q,g), R/g)
                np.testing.assert_allclose(H, U @ H0 @ U.T, atol=1e-6)
                np.testing.assert_allclose(hessian(lambda q: V(q,g),R/g),H @ H,atol=1e-5)

    def test_square_identity_is_not_claimed_away_from_critical_points(self):
        q = np.diag([.2,.3,.4])
        HW = hessian(lambda x: W(x,1.),q)
        HV = hessian(lambda x: V(x,1.),q)
        self.assertGreater(np.linalg.norm(HV-HW@HW), .1)

    def test_barrier_and_action_endpoint_difference(self):
        for g in (1.,np.sqrt(2.)):
            for a in (0.,.25,.5,.75,1.):
                self.assertAlmostEqual(V(a*np.eye(3)/g,g),1.5*a*a*(1-a)**2/(g*g))
            self.assertAlmostEqual(V(.5*np.eye(3)/g,g),3/(32*g*g))
            self.assertAlmostEqual(W(np.eye(3)/g,g)-W(np.zeros((3,3)),g),1/(2*g*g))


if __name__ == "__main__":
    unittest.main()
