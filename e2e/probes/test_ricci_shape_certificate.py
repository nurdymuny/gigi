import copy
import unittest
from ricci_shape_regular_mesh import mesh
from ricci_lattice_mesh import mesh as old_mesh
from ricci_shape_certificate import certify


class CertificateTests(unittest.TestCase):
    def test_real_mesh_is_certified(self):
        r=certify(mesh(0),0)
        self.assertEqual(r['tetrahedra'],14400)
        self.assertEqual(r['deck_order'],120)
        self.assertLessEqual(r['max_diameter_over_inradius'],14.)
        self.assertTrue(r['exact_node_identity_checked'])

    def test_removed_mechanisms_are_refused(self):
        for attack in ['old_mesh','missing_cell','reversed_cell','wrong_key','wrong_affine','wrong_unit','wrong_n']:
            with self.subTest(attack=attack):
                m=copy.deepcopy(mesh(0))
                if attack=='old_mesh':m=old_mesh(0)
                elif attack=='missing_cell':m['tetrahedra']=m['tetrahedra'][:-1]
                elif attack=='reversed_cell':m['tetrahedra'][0,[0,1]]=m['tetrahedra'][0,[1,0]]
                elif attack=='wrong_key':m['node_keys'][0,4]+=1
                elif attack=='wrong_affine':m['affine_vertices'][0,0]+=.001
                elif attack=='wrong_unit':m['vertices'][0,0]+=.001
                elif attack=='wrong_n':m['subdivision_n']=2
                with self.assertRaises(ValueError):certify(m,0)


if __name__=='__main__':unittest.main()
