import unittest
import numpy as np
from ricci_lattice_operator import orbit_fixture
from ricci_lattice_mesh import mesh,plus


class OrbitTests(unittest.TestCase):
    def test_all_cover_edges_and_cells_accounted_for(self):
        for level in [0,1]:
            m=mesh(level); f=orbit_fixture(level)
            self.assertEqual(len(f['edge_points']),len(m['edges'])//120)
            self.assertEqual(len(f['cover_edge_orbit']),len(m['edges']))
            np.testing.assert_array_equal(np.bincount(f['cover_edge_orbit']),120)
            self.assertAlmostEqual(sum(c['weight'] for c in f['cells']),len(m['tetrahedra'])/120,places=11)
            for c in f['cells']:
                self.assertGreater(np.linalg.det(c['vertices']),0)

    def test_reversed_edges_preserve_actual_base_transport(self):
        m=mesh(1);f=orbit_fixture(1)
        self.assertTrue(any(f['cover_edge_reverse']))
        def delta(p):
            a,b=np.asarray(p);a=a.copy();a[1:]*=-1
            return plus(a,b)
        reference=np.array([delta(e) for e in f['edge_points']])
        actual=np.array([delta(m['vertices'][e]) for e in m['edges']])
        expected=reference[f['cover_edge_orbit']].copy()
        expected[np.array(f['cover_edge_reverse']),1:]*=-1
        np.testing.assert_allclose(actual,expected,atol=3e-14,rtol=0)


if __name__=='__main__':unittest.main()
