"""Independent polynomial integration and constrained KKT RF2F oracles."""
import itertools
import unittest
import numpy as np
from scipy.sparse import csr_matrix
from ricci_variational_math import moment_matrix, fit_patch, recovered_energy

VERTICES = np.vstack([np.zeros(3), np.eye(3)])
FACES = list(itertools.combinations(range(4), 3))


def flux(triangle, coefficient, weighted_vertex=None):
    # Independent degree-two exact triangle quadrature.
    bary = np.array([[2/3, 1/6, 1/6], [1/6, 2/3, 1/6], [1/6, 1/6, 2/3]])
    points = bary@triangle
    lam = np.column_stack([1-points.sum(axis=1), points])
    values = lam@coefficient.reshape(4, 3)
    normal = np.cross(triangle[1]-triangle[0], triangle[2]-triangle[0])
    measure = np.ones(3) if weighted_vertex is None else bary[:,weighted_vertex]
    return float(np.sum((values@normal)*measure)/6)


def patch():
    center = VERTICES[np.array(FACES)]
    rng = np.random.default_rng(671)
    extra = rng.uniform(-.8, 1.4, size=(30, 3, 3))
    return np.concatenate([center, extra])


def closed_coefficients():
    c = np.array([[.3, -.2, .7], [1.2, .4, -.5], [-.3, .6, .2], [.8, -.7, .1]])
    c[3,2] = c[0,2]-(c[1,0]-c[0,0])-(c[2,1]-c[0,1])
    return c.ravel()


def auxiliary_fixture():
    rng = np.random.default_rng(613)
    b = rng.normal(size=(9, 9)); h = b.T@b+np.eye(9)
    q = rng.normal(size=(9, 3))
    j = np.kron(np.eye(3), np.ones((3, 1))/3)
    z = np.kron(np.eye(3), np.array([[1., 0.], [0., 1.], [-1., -1.]]))
    c = np.kron(np.eye(3), np.ones((1, 3)))
    return h, q, j, z, c


class VariationalTests(unittest.TestCase):
    def test_moment_basis_against_independent_quadrature(self):
        expected = np.empty((12, 12))
        for face, ids in enumerate(FACES):
            for vertex in range(3):
                for coefficient in range(12):
                    expected[3*face+vertex, coefficient] = flux(
                        VERTICES[list(ids)], np.eye(12)[coefficient], vertex)
        np.testing.assert_allclose(moment_matrix(), expected, atol=1e-14)
        np.testing.assert_allclose(moment_matrix()@np.linalg.inv(expected), np.eye(12), atol=1e-13)

    def test_closed_affine_polynomial_reproduction(self):
        triangles = patch(); c = closed_coefficients()
        observations = np.array([flux(t, c) for t in triangles])
        recovery, diagnostics = fit_patch(triangles)
        np.testing.assert_allclose(recovery@observations, c, atol=3e-12)
        self.assertEqual(diagnostics['central_rank'], 3)
        self.assertEqual(diagnostics['free_rank'], 8)

    def test_exact_central_fluxes_even_for_inconsistent_neighbor_data(self):
        triangles = patch(); c = closed_coefficients()
        values = np.array([flux(t, c) for t in triangles]); values[4:] += .3*np.sin(np.arange(len(values)-4))
        recovery, _ = fit_patch(triangles)
        result = recovery@values
        np.testing.assert_allclose([flux(t, result) for t in triangles[:4]], values[:4], atol=3e-12)

    def test_face_orientation_changes_only_observation_sign(self):
        triangles = patch(); recovery, _ = fit_patch(triangles)
        flipped = triangles.copy(); flipped[6] = flipped[6, [0, 2, 1]]
        other, _ = fit_patch(flipped); signs = np.ones(len(triangles)); signs[6] = -1
        np.testing.assert_allclose(other*signs, recovery, atol=3e-12)

    def test_rank_deficient_and_nonfinite_patches_refuse(self):
        for triangles in [np.repeat(patch()[:4], 8, axis=0), np.full((20, 3, 3), np.nan)]:
            with self.assertRaises(ValueError): fit_patch(triangles)

    def test_reduced_recovery_agrees_with_independent_kkt(self):
        h, q, j, z, c = auxiliary_fixture(); f = np.array([.3, -.6, 1.1])
        kkt = np.block([[h, c.T], [c, np.zeros((3, 3))]])
        r = np.linalg.solve(kkt, np.vstack([q, np.eye(3)]))[:9]
        result = recovered_energy(csr_matrix(h), csr_matrix(q), csr_matrix(j), csr_matrix(z), f)
        np.testing.assert_allclose(result['moments'], r@f, atol=3e-12)
        np.testing.assert_allclose(result['gradient'], r.T@h@r@f, atol=3e-11)
        np.testing.assert_allclose(c@result['moments'], f, atol=3e-12)

    def test_actual_reduced_energy_derivative(self):
        h, q, j, z, _ = auxiliary_fixture(); f = np.array([.3, -.6, 1.1]); direction = np.array([.2, -.8, .4])
        run = lambda x: recovered_energy(csr_matrix(h), csr_matrix(q), csr_matrix(j), csr_matrix(z), x)
        result = run(f)
        step = 1e-4
        difference = (run(f+step*direction)['energy']-run(f-step*direction)['energy'])/(2*step)
        self.assertAlmostEqual(difference, direction@result['gradient'], delta=3e-9)
        self.assertGreater(np.linalg.norm(j.T@h@result['moments']-result['gradient']), .1)


if __name__ == '__main__': unittest.main()
