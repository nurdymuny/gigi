"""Independent small mathematical controls for the spec, NOT a physical certificate implementation."""
import unittest
import numpy as np


class QuasimodeMathControls(unittest.TestCase):
    def test_complex_norm_requires_conjugation(self):
        v = np.array([1., 1j])/np.sqrt(2.)
        self.assertAlmostEqual(np.vdot(v, v).real, 1.)
        self.assertAlmostEqual(abs(np.dot(v, v)), 0.)
        for theta in (.2, 1.7):
            w = np.exp(1j*theta)*v
            self.assertAlmostEqual(np.vdot(w, w).real, 1.)

    def test_galerkin_square_discards_true_residual(self):
        H = np.array([[1., .5], [.5, 2.]])
        psi = np.array([1., 0.])
        E = np.vdot(psi, H@psi).real
        residual = H@psi-E*psi
        self.assertAlmostEqual(np.vdot(residual, residual).real, .25)
        self.assertEqual(H[0, 0]**2-E**2, 0.)

    def test_n_and_2n_agreement_can_be_exact_aliasing(self):
        def projection(n, frequency):
            angle = 2*np.pi*np.arange(n)/n
            return np.mean(np.exp(1j*(frequency-2)*angle))
        self.assertLess(abs(projection(16, 34)-1), 1e-12)
        self.assertLess(abs(projection(32, 34)-1), 1e-12)
        self.assertLess(abs(projection(128, 34)), 1e-12)

    def test_covariants_have_the_declared_charge(self):
        G = np.array([[3., .2, .3], [.2, 2., .4], [.3, .4, 1.]])
        def charged(g):
            return g[0, 1]+1j*g[0, 2], .5*(g[1, 1]-g[2, 2])+1j*g[1, 2]
        v, w = charged(G)
        for theta in (.2, .7, 1.4):
            c, s = np.cos(theta), np.sin(theta)
            R = np.array([[1., 0., 0.], [0., c, -s], [0., s, c]])
            actual = charged(R@G@R.T)
            np.testing.assert_allclose(actual, [np.exp(1j*theta)*v, np.exp(2j*theta)*w], atol=1e-14)

    def test_temple_bound_and_lower_level_admixture(self):
        levels = np.array([0., 2., 5.])
        weights = np.array([.02, .96, .02])
        E = weights@levels
        variance = weights@((levels-E)**2)
        rho = 5.
        self.assertLess(E, rho)
        self.assertGreater(weights[0], 0.)
        lower = E-variance/(rho-E)
        self.assertLessEqual(lower, 2.)
        self.assertGreater(lower, 0.)
        self.assertGreaterEqual(weights@((levels-2.)*(levels-rho)), 0.)

    def test_temple_without_positive_denominator_can_lie(self):
        levels, weights = np.array([0., 2.]), np.array([.25, .75])
        E = weights@levels
        variance = weights@((levels-E)**2)
        rho = 1.
        self.assertGreater(E, rho)
        alleged_lower = E-variance/(rho-E)
        self.assertEqual(alleged_lower, 3.)
        self.assertGreater(alleged_lower, levels[0])

    def test_even_energy_changes_when_partner_dropped(self):
        R = np.array([[0., 1.], [1., 0.]])
        H = np.eye(2)-.5*R
        np.testing.assert_array_equal(H@R, R@H)
        np.testing.assert_array_equal((H-R@H@R)/2, np.zeros((2, 2)))
        self.assertEqual(np.linalg.eigvalsh(H)[0], .5)
        self.assertEqual(H[0, 0], 1.)

    def test_schur_correction_is_even_and_requires_resolvent(self):
        H = np.array([[1., -.5], [-.5, 1.]])
        E = .5
        correction = H[0, 1]*(1/(E-H[1, 1]))*H[1, 0]
        self.assertEqual(correction, -.5)
        self.assertEqual(H[0, 0]+correction, E)
        with self.assertRaises(np.linalg.LinAlgError):
            np.linalg.inv(np.array([[1.-H[1, 1]]]))


if __name__ == '__main__':
    unittest.main()
