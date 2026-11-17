import unittest
import numpy as np
from ricci_shared_curvature_math import (powers, divergence, closed_basis, evaluate,
    potential, curl, edge_integrals, face_fluxes, minimum_energy, energy_gradient)


class SharedCurvatureMathTests(unittest.TestCase):
    def test_closed_basis_dimensions_and_coefficient_closure(self):
        for degree, size in enumerate([3, 11, 26]):
            basis = closed_basis(degree)
            self.assertEqual(basis.shape, (3*len(powers(degree)), size))
            np.testing.assert_allclose(divergence(degree)@basis, 0., atol=1e-13, rtol=0)

    def test_homotopy_curl_recovers_every_closed_basis_column(self):
        for degree in range(3):
            for coefficients in closed_basis(degree).T:
                np.testing.assert_allclose(curl(potential(coefficients, degree), degree+1), coefficients,
                                           atol=2e-13, rtol=0)

    def test_nonclosed_curvature_refused(self):
        coefficients = np.zeros(3*len(powers(1)))
        coefficients[powers(1).index((1, 0, 0))] = 1.
        with self.assertRaisesRegex(ValueError, "closed"):
            potential(coefficients, 1)

    def test_constant_flux_stokes_and_orientation(self):
        f = np.array([0., 0., 2.])
        a = potential(f, 0)
        triangle = np.array([[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]])
        edges = np.array([[triangle[0], triangle[1]], [triangle[1], triangle[2]],
                          [triangle[2], triangle[0]]])
        self.assertAlmostEqual(edge_integrals(a, 1, edges).sum(), 1.)
        self.assertAlmostEqual(face_fluxes(f, 0, triangle[None])[0], 1.)
        self.assertAlmostEqual(face_fluxes(f, 0, triangle[[0, 2, 1]][None])[0], -1.)

    def test_minimum_energy_matches_independent_analytic_solution(self):
        mass = np.diag([2., 3., 5.]); q = np.array([[1., 1., 0.], [2., 2., 0.]])
        result = minimum_energy(mass, q, np.array([1., 2.]))
        np.testing.assert_allclose(result["coefficients"], [.6, .4, 0.], atol=1e-13, rtol=0)
        self.assertEqual(result["rank"], 1)
        self.assertAlmostEqual(result["energy"], .6)

    def test_incompatible_flux_refused_not_least_squares_fitted(self):
        q = np.array([[1., 1., 0.], [2., 2., 0.]])
        with self.assertRaisesRegex(ValueError, "incompatible"):
            minimum_energy(np.eye(3), q, np.array([1., 3.]))

    def test_full_derivative_includes_reconstruction(self):
        mass = np.diag([2., 3., 5.]); q = np.array([[1., 0., 1.], [0., 1., 1.]])
        d = np.array([[1., -1., 0., 1.], [0., 1., -1., 0.]])
        a = np.array([.2, .4, -.3, .7])
        result = minimum_energy(mass, q, d@a)
        reconstruction = result["map"]@d
        gradient = energy_gradient(reconstruction, mass, result["coefficients"])
        numerical = []
        for i in range(4):
            step = np.zeros(4); step[i] = 1e-5
            ep = minimum_energy(mass, q, d@(a+step))["energy"]
            em = minimum_energy(mass, q, d@(a-step))["energy"]
            numerical.append((ep-em)/2e-5)
        self.assertGreater(np.linalg.norm(gradient), .01)
        np.testing.assert_allclose(gradient, numerical, atol=1e-9, rtol=0)

    def test_null_constraint_direction_increases_energy(self):
        mass = np.diag([2., 3., 5.]); q = np.array([[1., 1., 0.]])
        result = minimum_energy(mass, q, np.array([1.]))
        c = result["coefficients"]; changed = c+np.array([.2, -.2, .1])
        np.testing.assert_allclose(q@changed, [1.], atol=1e-14, rtol=0)
        self.assertGreater(.5*changed@mass@changed, result["energy"])

    def test_nonfinite_data_refused(self):
        with self.assertRaisesRegex(ValueError, "finite"):
            minimum_energy(np.eye(2), np.eye(2), np.array([np.nan, 1.]))


if __name__ == "__main__":
    unittest.main(verbosity=2)
