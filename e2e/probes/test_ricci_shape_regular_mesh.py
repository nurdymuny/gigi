import itertools
import unittest
import numpy as np
from ricci_shape_regular_mesh import mesh,reference_subdivision,RESOLUTIONS
from ricci_lattice_mesh import validate,deck_check
from ricci_lattice_mesh_quality import quality


def boundary(points,cells):
    faces=np.concatenate([np.delete(cells,i,axis=1) for i in range(4)])
    faces,counts=np.unique(np.sort(faces,axis=1),axis=0,return_counts=True)
    return faces[counts==1]


class ReferenceTests(unittest.TestCase):
    def test_six_scaled_shapes_and_orthogonal_flag_geometry(self):
        from ricci_lattice_mesh import mesh as old_mesh
        old=old_mesh(0);p=old['vertices'][old['tetrahedra'][0]]
        flag=np.array([p[:i+1].mean(axis=0) for i in range(4)])
        successive=np.diff(flag,axis=0);length_sq=2*(1-(1+np.sqrt(5))/4)
        np.testing.assert_allclose(successive@successive.T,
            np.diag(length_sq*np.array([1/4,1/12,1/24])),atol=2e-16,rtol=2e-14)
        for n in [2,3,5,7]:
            bary,cells=reference_subdivision(n);q=(bary@flag/n)[cells]
            edges=q[:,1:]-q[:,0,None];gram=edges@edges.transpose(0,2,1)
            volume=np.sqrt(np.linalg.det(gram))/6
            diameter=np.max(np.stack([np.linalg.norm(q[:,i]-q[:,j],axis=1)
                for i,j in itertools.combinations(range(4),2)]),axis=0)
            np.testing.assert_allclose(volume,length_sq**1.5/(144*np.sqrt(2)*n**3),rtol=2e-13,atol=1e-17)
            np.testing.assert_allclose(diameter**2,3*length_sq/(8*n*n),rtol=2e-13,atol=1e-16)
            # Shape classification is diagnostic only; node identity never rounds.
            # The n=2 boundary truncation contains five orientations; all six
            # occur from n=3 onward. Both have the same proved size bounds.
            self.assertEqual(len(np.unique(np.round(gram*n*n,12).reshape(-1,9),axis=0)),5 if n==2 else 6)

    def test_exact_count_volume_and_internal_faces(self):
        for n in [1,2,3,5,7]:
            points,cells=reference_subdivision(n)
            self.assertEqual(len(cells),n**3)
            self.assertEqual(len(points),(n+1)*(n+2)*(n+3)//6)
            np.testing.assert_array_equal(points.sum(axis=1),n)
            p=points[cells,1:]/n
            determinants=np.linalg.det(p[:,1:]-p[:,0,None])
            np.testing.assert_allclose(determinants,1/n**3,atol=2e-15,rtol=2e-14)
            self.assertAlmostEqual(determinants.sum(),1.,places=12)
            outer=boundary(points,cells)
            self.assertEqual(len(outer),4*n*n)
            self.assertTrue(np.all(np.any(np.all(points[outer]==0,axis=1),axis=1)))

    def test_all_face_restrictions_agree_in_integer_coordinates(self):
        points,cells=reference_subdivision(5)
        self.assertEqual(len(cells),125)
        face_sets=[]
        for omitted in range(4):
            faces=[face for face in boundary(points,cells) if np.all(points[face,omitted]==0)]
            face_sets.append({tuple(sorted(tuple(row) for row in np.delete(points[f],omitted,axis=1))) for f in faces})
        for f in face_sets[1:]:self.assertEqual(f,face_sets[0])

    def test_invalid_resolutions_refuse(self):
        for n in [0,-1,True,1.5,1000]:
            with self.assertRaises(ValueError):reference_subdivision(n)


class MeshTests(unittest.TestCase):
    def test_one_barycentric_split_not_repeated_skinny_split(self):
        m=mesh(0)
        self.assertEqual(len(m['vertices']),2640)
        self.assertEqual(len(m['tetrahedra']),14400)
        self.assertEqual(m['denominator'],12)
        self.assertEqual(len(m['node_keys']),len(m['vertices']))
        self.assertEqual(validate(m)['euler_3d'],0)
        self.assertEqual(deck_check(m)['quotient_cell_counts'],[22,142,240,120])

    def test_refinement_uses_fixed_macro_geometry(self):
        for level in [0,1]:
            m=mesh(level);n=RESOLUTIONS[level]
            self.assertEqual(len(m['tetrahedra']),14400*n**3)
            q=quality(m['vertices'],m['tetrahedra'])
            self.assertLessEqual(q['max_diameter_over_inradius'],14.)
            self.assertEqual(validate(m)['euler_3d'],0)
            self.assertEqual(deck_check(m)['deck_order'],120)

    def test_all_shape_constants_are_derived_from_geometry(self):
        c=(1+np.sqrt(5))/4;rho=np.sqrt((1+3*c)/4)
        bound=18*np.sqrt(3)/rho**4
        self.assertLess(bound,43.)
        m=mesh(0)
        self.assertIn('affine_vertices',m)
        p=m['affine_vertices'][m['tetrahedra']]
        b=p[:,1:]-p[:,0,None]
        gram=b@b.transpose(0,2,1)
        vol=np.sqrt(np.linalg.det(gram))/6
        diameter=np.max(np.stack([np.linalg.norm(p[:,i]-p[:,j],axis=1) for i,j in itertools.combinations(range(4),2)]),axis=0)
        np.testing.assert_allclose(diameter**3/vol,27*np.sqrt(3),atol=2e-12,rtol=1e-13)
        norm=np.linalg.norm(p,axis=2)
        self.assertGreaterEqual(norm.min(),rho-1e-14)
        u=m['vertices'][m['tetrahedra']]
        np.testing.assert_allclose(np.linalg.det(u),np.linalg.det(p)/np.prod(norm,axis=1),atol=2e-15,rtol=2e-13)
        self.assertLess(quality(m['vertices'],m['tetrahedra'])['max_diameter_over_inradius'],bound)


if __name__=='__main__':unittest.main()
