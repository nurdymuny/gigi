import json
from pathlib import Path
import unittest
import numpy as np
from scipy.sparse import csr_matrix
from ricci_consistency_math import phase_summary,link_quaternions,rotations,cell_integrals,residual_split
from ricci_lattice_forms import masses
from ricci_lattice_demo import matrix_link,matrix
from test_ricci_operator_native import rotation

ROOT=Path(__file__).resolve().parents[2]
DATA=ROOT/'output/ricci-shape-regular-20261117/berger'


class MathTests(unittest.TestCase):
    def test_signed_center_and_hemisphere_observables(self):
        q=np.array([[1.,0,0,0],[0.,1.,0,0],[-1.,0,0,0]])
        r=phase_summary(q)
        self.assertEqual(r['negative_q0_count'],1)
        self.assertEqual(r['q0_min'],-1.)
        self.assertEqual(r['s_lift_min'],0.)
        self.assertAlmostEqual(r['alpha_max'],np.pi)
        self.assertLess(r['bridge_max_error'],2e-15)
        self.assertLess(r['so3_bridge_max_error'],2e-15)
        self.assertEqual(phase_summary(np.array([[0.,1.,0,0]]))['s_lift_min'],.5)

    def test_invalid_phases_refuse(self):
        for q in [[],[[2.,0,0,0]],[[float('nan'),0,0,0]],[[1.,0,0]]]:
            with self.subTest(q=q),self.assertRaises(ValueError):phase_summary(q)

    def test_batched_links_and_adjoint_against_independent_matrix_oracle(self):
        payload=json.loads((DATA/'level0_berger_q6.input.json').read_text())
        edges=np.array(payload['edge_points'])[::7];q=link_quaternions([3.,1.,1.],edges[:,0],edges[:,1])
        for e,got in zip(edges,q):
            expected=matrix_link([3.,1.,1.],*e)
            np.testing.assert_allclose(matrix(got),expected,atol=2e-13,rtol=0)
            np.testing.assert_allclose(rotations(got),rotation(expected),atol=2e-13,rtol=0)
        np.testing.assert_allclose(link_quaternions([3.,1.,1.],-edges[:,0],-edges[:,1]),q,atol=2e-14,rtol=0)
        np.testing.assert_allclose(rotations(-q),rotations(q),atol=2e-14,rtol=0)

    def test_batched_geometry_and_load_of_planted_representable_field(self):
        payload=json.loads((DATA/'level0_berger_q6.input.json').read_text())
        points=np.array([c['vertices'] for c in payload['cells'][:3]])
        pairs=[(0,1),(0,2),(0,3),(1,2),(1,3),(2,3)]
        links=link_quaternions([3.,1.,1.],points[:,[a for a,b in pairs]],points[:,[b for a,b in pairs]])
        trial=np.random.default_rng(1701).normal(size=(3,6,3))
        for order in [4,6]:
            r=cell_integrals(points,links,[3.,1.,1.],order,trial=trial)
            for c in range(3):
                oracle=masses(points[c],[3.,1.,1.],order)
                for key in ['edge_mass','face_mass','volume']:
                    np.testing.assert_allclose(r[key][c],oracle[key],atol=2e-12,rtol=3e-12)
                roots=[rotation(matrix(links[c,i])) for i in [0,1,2,1,2,2]]
                b=np.block([[4*oracle['edge_mass'][i,j]*roots[i].T@roots[j] for j in range(6)] for i in range(6)])
                np.testing.assert_allclose(r['load'][c].ravel(),b@trial[c].ravel(),atol=2e-12,rtol=3e-12)

    def test_exact_decomposition_and_nonorthogonal_cross_term(self):
        b=csr_matrix([[2.,.2],[.2,3.]])
        # Plant action=(1,2), mass=(2,3): their cross term is exactly 11.7.
        r=residual_split(b,np.zeros(2),np.array([3.,5.]),b@np.array([1.,2.]),2.)
        self.assertLess(r['recombination_error'],1e-12)
        self.assertAlmostEqual(r['total_error']**2,r['action_error']**2+r['mass_error']**2+2*r['cross_inner'],places=12)
        self.assertAlmostEqual(r['cross_inner'],11.7,places=12)


if __name__=='__main__':unittest.main()
