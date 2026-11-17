import unittest
import numpy as np
from ricci_lattice_operator import orbit_fixture
from ricci_lattice_convergence import solve,reference_velocity
from test_ricci_operator_native import invoke
from ricci_wilson_oracle import mapped_velocity,wilson,phi


class ScalingTests(unittest.TestCase):
    def test_full_assembled_operator_has_physical_scale_not_a_fitted_clock(self):
        fixture=orbit_fixture(0)
        a=invoke(fixture,(3.,1.,1.));b=invoke(fixture,(21.,7.,7.))
        ba,wa,_=solve(a);bb,wb,_=solve(b)
        np.testing.assert_allclose(b['links'],a['links'],atol=2e-14,rtol=0)
        np.testing.assert_allclose(b['volume'],a['volume']*7**1.5,rtol=2e-13)
        np.testing.assert_allclose(b['energy'],a['energy']/np.sqrt(7),rtol=2e-13)
        np.testing.assert_allclose(b['gradient'],np.array(a['gradient'])/np.sqrt(7),atol=1e-12,rtol=2e-12)
        np.testing.assert_allclose(bb.toarray(),ba.toarray()*np.sqrt(7),atol=1e-12,rtol=2e-12)
        np.testing.assert_allclose(wb,wa/7,atol=1e-11,rtol=2e-11)
        np.testing.assert_allclose(reference_velocity(fixture['edge_points'],[21.,7.,7.]),
            reference_velocity(fixture['edge_points'],[3.,1.,1.])/7,atol=1e-12,rtol=2e-12)

    def test_normalized_and_unnormalized_continuum_targets_agree_in_scale_invariant_map(self):
        for g in [[3.,1.,1.],[.5,1.,2.],[5.,1.,1.],[1.3,.8,2.1]]:
            target=wilson(np.array(g),phi(g))
            np.testing.assert_allclose(mapped_velocity(g,True),target,atol=1e-12,rtol=1e-12)
            np.testing.assert_allclose(mapped_velocity(g,False),target,atol=1e-12,rtol=1e-12)


if __name__=='__main__':unittest.main()
