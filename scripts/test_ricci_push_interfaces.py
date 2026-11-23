import unittest
import numpy as np
from ricci_push_interfaces import hodge, cell_parts


class InterfaceAccountingTests(unittest.TestCase):
    def setUp(self):
        self.points = np.array([[1., 0., 0., 0.], [1., .2, .02, 0.],
                                [1., 0., .18, .01], [1., .01, 0., .22]])
        self.points /= np.linalg.norm(self.points, axis=1)[:, None]
        self.a = np.array([.2, -.4, .5, .7, -.3, .6])
        self.u = np.array([[1., .1], [-.2, .3], [.4, .2], [.7, -.5], [.3, .9], [-.6, .8]])

    def test_analytic_curl_matches_independent_difference(self):
        x = np.array([[.2, .3, .1], [.1, .1, .4]])
        flux = np.array([.3, -.7, .2])
        actual = hodge(self.points, x, flux)[1]
        derivatives = []
        for axis in range(3):
            step = np.zeros(3); step[axis] = 1e-5
            derivatives.append((hodge(self.points, x+step, flux)[0]
                                - hodge(self.points, x-step, flux)[0]) / 2e-5)
        d = np.array(derivatives)
        expected = np.column_stack([d[1, :, 2]-d[2, :, 1], d[2, :, 0]-d[0, :, 2],
                                    d[0, :, 1]-d[1, :, 0]])
        np.testing.assert_allclose(actual, expected, atol=2e-8, rtol=0)

    def test_integration_by_parts_matches_direct_force(self):
        result = cell_parts(self.points, self.a, self.u)
        np.testing.assert_allclose(result["direct"], result["bulk"]+result["faces"].sum(axis=0),
                                   atol=1e-9, rtol=0)

    def test_boundary_removal_breaks_identity(self):
        result = cell_parts(self.points, self.a, self.u)
        self.assertGreater(np.linalg.norm(result["direct"]-result["bulk"]), 1e-3)

    def test_outward_orientation_flip_breaks_identity(self):
        result = cell_parts(self.points, self.a, self.u, flip_normals=True)
        self.assertGreater(np.linalg.norm(result["direct"]-result["bulk"]-result["faces"].sum(axis=0)), 1e-3)


if __name__ == "__main__":
    unittest.main(verbosity=2)
