import unittest
import numpy as np
from scipy.sparse import csr_matrix
from ricci_variational_math import fit_patch, recovered_energy
from test_ricci_variational_math import auxiliary_fixture, patch, flux


class StabilityTests(unittest.TestCase):
    def test_entire_closed_affine_polynomial_space_is_reproduced(self):
        triangles = patch(); recovery, _ = fit_patch(triangles)
        basis = []
        for i in range(11):
            c = np.eye(12)[i].reshape(4,3)
            c[3,2] = c[0,2]-(c[1,0]-c[0,0])-(c[2,1]-c[0,1])
            basis.append(c.ravel())
            observed = np.array([flux(t, c) for t in triangles])
            np.testing.assert_allclose(recovery@observed, c.ravel(), atol=4e-12)
        self.assertEqual(np.linalg.matrix_rank(np.array(basis)), 11)

    def test_variational_projection_is_nonexpansive_about_feasible_lift(self):
        h, _, j, z, c = auxiliary_fixture()
        f = np.array([.3, -.6, 1.1]); target = np.array([.2, .4, -.5, .9, -.3, .8, .1, .7, -.2])
        q = np.outer(h@target, f)/(f@f)
        result = recovered_energy(csr_matrix(h),csr_matrix(q),csr_matrix(j),csr_matrix(z),f)
        before = target-j@f; after = result['moments']-j@f
        self.assertLessEqual(after@h@after, before@h@before+1e-11)
        np.testing.assert_allclose(z.T@h@(target-result['moments']), 0., atol=2e-11)
        np.testing.assert_allclose(c@result['moments'], f, atol=2e-12)


if __name__ == '__main__': unittest.main()
