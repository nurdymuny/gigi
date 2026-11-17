import unittest
from ricci_lattice_mesh import mesh
from ricci_lattice_operator import orbit_fixture,cover_fixture
from ricci_mesh_orbits import orbit_fixture_for_mesh,cover_fixture_for_mesh


class AdapterTests(unittest.TestCase):
    def test_identical_old_orbit_data_not_merely_same_counts(self):
        for level in [0,1]:self.assertEqual(orbit_fixture_for_mesh(mesh(level)),orbit_fixture(level))

    def test_identical_old_cover_data(self):
        self.assertEqual(cover_fixture_for_mesh(mesh(0)),cover_fixture(0))


if __name__=='__main__':unittest.main()
