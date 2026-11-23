import unittest
import numpy as np
from scipy.sparse import csr_matrix
from ricci_minimum_energy import recover
from test_ricci_variational_math import auxiliary_fixture


class MinimumEnergyTests(unittest.TestCase):
    def setUp(self):
        self.h, _, self.j, self.z, self.c = auxiliary_fixture()
        self.f = np.array([.3,-.6,1.1])

    def run_operator(self, f=None):
        return recover(csr_matrix(self.h),csr_matrix(self.j),csr_matrix(self.z), self.f if f is None else f)

    def test_independent_kkt_and_full_adjoint(self):
        h,c = self.h,self.c
        kkt = np.block([[h,c.T],[c,np.zeros((3,3))]])
        r = np.linalg.solve(kkt,np.vstack([np.zeros((9,3)),np.eye(3)]))[:9]
        actual = self.run_operator()
        np.testing.assert_allclose(actual['moments'],r@self.f,atol=3e-12)
        np.testing.assert_allclose(actual['gradient'],r.T@h@r@self.f,atol=3e-12)

    def test_stationary_minimum_and_original_flux(self):
        actual = self.run_operator(); c = actual['moments']; lift = self.j@self.f
        np.testing.assert_allclose(self.c@c,self.f,atol=2e-12)
        np.testing.assert_allclose(self.z.T@self.h@c,0.,atol=3e-12)
        self.assertLess(actual['energy'],.5*lift@self.h@lift-1e-3)
        for direction in np.eye(6):
            other = c+.13*self.z@direction
            self.assertGreater(.5*other@self.h@other,actual['energy'])

    def test_reduced_energy_derivative(self):
        direction = np.array([.2,-.7,.4]); step=1e-4
        derivative=(self.run_operator(self.f+step*direction)['energy']-
                    self.run_operator(self.f-step*direction)['energy'])/(2*step)
        self.assertAlmostEqual(derivative,direction@self.run_operator()['gradient'],delta=2e-9)

    def test_zero_curvature_and_invalid_input(self):
        actual = self.run_operator(np.zeros(3))
        np.testing.assert_array_equal(actual['moments'],np.zeros(9))
        self.assertEqual(actual['energy'],0.)
        with self.assertRaises(ValueError): self.run_operator(np.array([np.nan,0.,1.]))


if __name__=='__main__': unittest.main()
