"""Planted independent face-space oracle for RF2E, before implementation."""
import unittest
import numpy as np
from scipy.sparse import csr_matrix
from ricci_curvature_reconstruction import evaluate, geometry_scale


def fixture():
    d = np.array([[1., -1., 1., 0.], [0., 1., -1., 1.]])
    m = np.array([[2., .2, 0., .1], [.2, 1.5, .1, 0.],
                  [0., .1, 1., .2], [.1, 0., .2, 2.5]])
    n = np.array([[1.2, .3], [.3, 2.]])
    return m, n, d, d.T @ n @ d


def face_oracle(m, n, d, a, epsilon):
    h = d @ np.linalg.solve(m, d.T @ n)
    p = np.linalg.inv(np.eye(len(n)) + epsilon * h)
    r = 2*p - p@p
    f = r @ d @ a
    g = d.T @ r.T @ n @ f
    return float(.5*f@n@f), g, -np.linalg.solve(m, g)


class ReconstructionTests(unittest.TestCase):
    def setUp(self):
        self.m, self.n, self.d, self.k = fixture()
        self.a = np.array([.7, -.2, .4, 1.1])
        self.epsilon = .17

    def run_operator(self, a=None, m=None, k=None, epsilon=None):
        return evaluate(csr_matrix(self.m if m is None else m),
                        csr_matrix(self.k if k is None else k),
                        self.a if a is None else a,
                        self.epsilon if epsilon is None else epsilon)

    def test_independent_face_energy_and_adjoint(self):
        e, g, v = face_oracle(self.m, self.n, self.d, self.a, self.epsilon)
        actual = self.run_operator()
        self.assertAlmostEqual(actual['energy'], e, delta=2e-12)
        np.testing.assert_allclose(actual['gradient'], g, rtol=2e-11, atol=2e-12)
        np.testing.assert_allclose(actual['velocity'], v, rtol=2e-11, atol=2e-12)

    def test_declared_energy_directional_derivative(self):
        z = np.array([.1, .3, -.5, .9])
        actual = self.run_operator()
        numerical = []
        for step in [1e-3, 5e-4, 2.5e-4]:
            plus = face_oracle(self.m, self.n, self.d, self.a+step*z, self.epsilon)[0]
            minus = face_oracle(self.m, self.n, self.d, self.a-step*z, self.epsilon)[0]
            numerical.append((plus-minus)/(2*step))
        # Quadratic energy has an exact centered difference apart from roundoff.
        np.testing.assert_allclose(numerical, actual['gradient']@z, rtol=2e-10, atol=2e-11)

    def test_planted_low_high_modes_and_no_cutoff(self):
        m = np.diag([2., 3., 5.]); lam = np.array([0., 4., 1200.])
        k = m@np.diag(lam); a = np.array([.3, .7, -.4])
        result = self.run_operator(a, m, k, .03)
        t = .03*lam; r = (1+2*t)/(1+t)**2
        np.testing.assert_allclose(result['reconstructed'], r*a, atol=2e-12)
        np.testing.assert_allclose(result['velocity'], -lam*r*r*a, atol=2e-10)
        self.assertGreater(abs(result['velocity'][-1]), 0.)

    def test_nullspace_linearity_energy_and_weighted_symmetry(self):
        null = np.array([0., 1., 1., 0.])
        np.testing.assert_allclose(self.run_operator(null)['velocity'], 0., atol=2e-12)
        b = np.array([.2, -.1, .9, -.4])
        x = self.run_operator(); y = self.run_operator(b)
        z = self.run_operator(2*self.a-3*b)
        np.testing.assert_allclose(z['gradient'], 2*x['gradient']-3*y['gradient'], atol=2e-11)
        self.assertGreaterEqual(x['energy'], 0.)
        self.assertAlmostEqual(self.a@self.m@y['velocity'], b@self.m@x['velocity'], delta=2e-12)

    def test_orientation_covariance(self):
        signs = np.diag([-1., 1., -1., 1.]); x = self.run_operator()
        y = self.run_operator(signs@self.a, signs@self.m@signs, signs@self.k@signs)
        self.assertAlmostEqual(x['energy'], y['energy'], delta=2e-12)
        np.testing.assert_allclose(y['velocity'], signs@x['velocity'], atol=2e-11)

    def test_length_scaling_preserves_dimensionless_filter(self):
        eps = geometry_scale(.1, 1.); scale = 7.
        self.assertAlmostEqual(geometry_scale(.1*scale, scale), eps*scale**2)
        # In dimension three M1 scales as length, M2 as inverse length.
        x = self.run_operator(epsilon=eps)
        y = self.run_operator(m=scale*self.m, k=self.k/scale,
                              epsilon=geometry_scale(.1*scale, scale))
        np.testing.assert_allclose(y['reconstructed'], x['reconstructed'], atol=2e-12)
        np.testing.assert_allclose(y['velocity'], x['velocity']/scale**2, atol=2e-11)

    def test_wrong_mechanisms_are_detectable(self):
        expected = face_oracle(self.m, self.n, self.d, self.a, self.epsilon)[1]
        p = np.linalg.solve(self.m+self.epsilon*self.k, self.m)
        r = 2*p-p@p
        mutants = {'missing_reconstruction': self.k@self.a,
                   'missing_adjoint': self.k@r@self.a,
                   'wrong_euclidean_transpose': r@self.k@r@self.a}
        for name, gradient in mutants.items():
            with self.subTest(name=name):
                self.assertGreater(np.linalg.norm(gradient-expected), 1e-3)

    def test_scale_refuses_invalid(self):
        for h, length in [(0., 1.), (-1., 1.), (.1, 0.), (.1, -1.),
                          (float('nan'), 1.), (.1, float('inf')), (2., 1.)]:
            with self.subTest(h=h, length=length):
                with self.assertRaises(ValueError): geometry_scale(h, length)

    def test_operator_refuses_invalid(self):
        for eps in [0., -1., float('nan'), float('inf')]:
            with self.subTest(epsilon=eps):
                with self.assertRaises(ValueError): self.run_operator(epsilon=eps)
        for a in [np.array([1.]), np.array([0., 0., float('nan'), 0.])]:
            with self.assertRaises(ValueError): self.run_operator(a)
        bad = self.m.copy(); bad[0,1] += .1
        with self.assertRaises(ValueError): self.run_operator(m=bad)

    def test_recomputed_solve_receipts(self):
        result = self.run_operator()
        self.assertEqual(len(result['solves']), 5)
        for receipt in result['solves']:
            self.assertLessEqual(receipt['residual_norm'], receipt['residual_limit'])
            self.assertGreater(receipt['iterations'], 0)


if __name__ == '__main__': unittest.main()
