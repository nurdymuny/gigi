import unittest
import numpy as np
from ricci_lattice_mesh import mesh,validate,deck_check,volume


class MeshGate(unittest.TestCase):
    def test_bw01_actual_600_cell(self):
        m=mesh(0)
        self.assertEqual(m["vertices"].shape,(120,4))
        self.assertEqual(m["tetrahedra"].shape,(600,4))

    def test_bw01_real_refinement(self):
        m=mesh(1)
        self.assertEqual(m["tetrahedra"].shape,(7200,4))
        self.assertGreater(len(m["vertices"]),120)

    def test_bw01_closed_oriented_cover_and_quotient(self):
        for level in [0,1]:
            m=mesh(level); stats=validate(m); quotient=deck_check(m)
            self.assertEqual(stats["euler_3d"],0)
            self.assertEqual(quotient["quotient_cell_counts"][-1],len(m["tetrahedra"])//120)
        self.assertLess(validate(mesh(1))["h_max"],validate(mesh(0))["h_max"])

    def test_bw01_missing_tetrahedron_refuses(self):
        m={k:v.copy() for k,v in mesh(0).items()};m["tetrahedra"]=m["tetrahedra"][:-1]
        with self.assertRaisesRegex(ValueError,"incidence"):validate(m)

    def test_bw01_wrong_orientation_refuses(self):
        m={k:v.copy() for k,v in mesh(0).items()};m["tetrahedra"][0,[2,3]]=m["tetrahedra"][0,[3,2]]
        with self.assertRaisesRegex(ValueError,"oriented"):validate(m)

    def test_bw01_physical_volume_refinement_and_scaling(self):
        exact=2*np.pi**2
        a,b=volume(mesh(0)),volume(mesh(1))
        self.assertLess(abs(b-exact),abs(a-exact)/3)
        self.assertAlmostEqual(volume(mesh(1),(3,1,1))/b,np.sqrt(3),places=12)


if __name__=="__main__": unittest.main()
