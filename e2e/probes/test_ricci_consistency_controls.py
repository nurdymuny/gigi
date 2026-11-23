import io
import itertools
import json
import unittest
from unittest.mock import patch
import numpy as np
from scipy.sparse import csr_matrix
from ricci_consistency_checks import triangle_flux
from ricci_consistency_math import INCIDENCE,base_log,compose
from ricci_consistency_diagnostic import scalar_control,hhe_matrix_control
import ricci_consistency_math as maths
import test_ricci_consistency_math as basetests
DATA=basetests.DATA


class Controls(unittest.TestCase):
    def test_stokes_is_not_the_failed_commuting_operator(self):
        payload=json.loads((DATA/'level0_berger_q6.input.json').read_text());values=[]
        for cell in payload['cells'][::7]:
            points=np.array(cell['vertices'])
            for face in itertools.combinations(range(4),3):
                p=points[list(face)]
                boundary=base_log(p[0],p[1])[0]+base_log(p[1],p[2])[0]-base_log(p[0],p[2])[0]
                flux=triangle_flux(p);values.append(abs(flux))
                self.assertAlmostEqual(flux,boundary,delta=2e-12)
        self.assertGreater(max(values),.001)

    def test_incidence_exact_and_wrong_orientation_detected(self):
        edges=list(itertools.combinations(range(4),2));d0=np.zeros((6,4))
        for e,(i,j) in enumerate(edges):d0[e,i]=-1;d0[e,j]=1
        np.testing.assert_array_equal(INCIDENCE@d0,0)
        wrong=INCIDENCE.copy();wrong[0,0]*=-1
        self.assertGreater(np.linalg.norm(wrong@d0),1.)

    def test_exact_planted_eigenfield_and_mass_whitening(self):
        m=csr_matrix([[2.,.3],[.3,1.]])
        r,_=scalar_control(m,4*m,np.array([1.,2.]),2.)
        self.assertLess(r['full_mass_error'],1e-12)
        self.assertAlmostEqual(r['rayleigh_quotient'],4.)
        h=hhe_matrix_control(m,4*m)
        self.assertLess(h['mass_norm_polynomial'],1e-12)

    def run_removed(self,method,target,replacement):
        with patch(target,side_effect=replacement):
            result=unittest.TextTestRunner(stream=io.StringIO()).run(unittest.TestSuite([basetests.MathTests(method)]))
        self.assertFalse(result.wasSuccessful());self.assertEqual(len(result.failures),1)

    def test_sign_erasure_removal_is_detected(self):
        original=maths.phase_summary
        def folded(q):
            q=np.array(q,copy=True);q[:,0]=abs(q[:,0]);return original(q)
        self.run_removed('test_signed_center_and_hemisphere_observables','test_ricci_consistency_math.phase_summary',folded)

    def test_wrong_half_angle_is_detected(self):
        original=maths.link_quaternions
        def doubled(*args):
            q=original(*args);return compose(q,q)
        self.run_removed('test_batched_links_and_adjoint_against_independent_matrix_oracle',
            'test_ricci_consistency_math.link_quaternions',doubled)

    def test_removed_transport_is_detected(self):
        def identity(q):return np.broadcast_to(np.eye(3),np.asarray(q).shape[:-1]+(3,3))
        self.run_removed('test_batched_geometry_and_load_of_planted_representable_field',
            'ricci_consistency_math.rotations',identity)


if __name__=='__main__':unittest.main()
