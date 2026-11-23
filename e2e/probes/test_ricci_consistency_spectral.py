import unittest
import numpy as np
from ricci_consistency_spectral import accounting


class SpectralTests(unittest.TestCase):
    def test_planted_small_mode_has_large_derivative_error(self):
        m=np.diag([2.,3.]);k=np.diag([8.,300.]);a=np.array([1.,.01])
        r=accounting(m,k,a)
        self.assertAlmostEqual(r['mass_squared'],2.0003,places=12)
        self.assertAlmostEqual(r['residual_squared'],96**2*.0003,places=12)
        self.assertAlmostEqual(r['high_field_mass_fraction'],.0003/2.0003,places=12)
        self.assertAlmostEqual(r['high_residual_fraction'],1.,places=12)
        self.assertLess(r['identity_error'],1e-12)
        self.assertNotAlmostEqual(r['residual_squared'],.0003,places=3)


if __name__=='__main__':unittest.main()
