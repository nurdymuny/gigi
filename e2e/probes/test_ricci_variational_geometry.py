"""Focused actual-mesh checks before the four-level RF2F flow experiment."""
import unittest
import numpy as np
from ricci_consistency_diagnostic import BASE
from ricci_lattice_evidence import read
from ricci_consistency_math import cell_integrals, link_quaternions
from ricci_variational_math import cell_masses, EDGES, recovered_energy
from ricci_variational_mesh import topology, recover_patches
from ricci_variational_assembly import assemble


class GeometryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.payload = read(BASE/'level0_berger_q6.input.json')
        cls.topo = topology(cls.payload)
        cls.patches, cls.diagnostics = recover_patches(cls.payload, cls.topo)

    def test_native_compatible_whitney_mass_embedding(self):
        points = np.array([cell['vertices'] for cell in self.payload['cells'][:3]])
        links = link_quaternions([1., 1., 1.], points[:,[i for i,j in EDGES]], points[:,[j for i,j in EDGES]])
        old = cell_integrals(points, links, [1., 1., 1.], 6, velocity=np.zeros((3, 3)))
        h, m, volume, _ = cell_masses(points, 6)
        inject = np.kron(np.eye(4), np.ones((3, 1))/3)
        np.testing.assert_allclose(m, old['edge_mass'], rtol=3e-12, atol=1e-14)
        np.testing.assert_allclose(np.einsum('ia,cij,jb->cab', inject, h, inject), old['face_mass'], rtol=3e-12, atol=1e-13)
        np.testing.assert_allclose(volume, old['volume'], rtol=3e-12, atol=1e-14)

    def test_face_constraints_closedness_and_patch_conditioning(self):
        a = np.random.default_rng(173).normal(size=len(self.payload['edge_points']))
        flux = self.topo['incidence']@a
        for ci, (ids, recovery) in enumerate(self.patches):
            recovered = (recovery@flux[ids]).reshape(4, 3).sum(axis=1)
            faces = self.topo['moment_ids'][ci].reshape(4, 3)[:,0]//3
            signs = self.topo['moment_signs'][ci].reshape(4, 3)[:,0]
            np.testing.assert_allclose(recovered, signs*flux[faces], atol=2e-11)
            self.assertLess(abs(np.array([-1., 1., -1., 1.])@recovered), 2e-11)
        self.assertTrue(all(row['free_condition'] <= 1e6 and row['radius'] <= 3 for row in self.diagnostics))

    def test_global_compatible_recovery_preserves_flux(self):
        h, q, m, j, z, _, _ = assemble(self.payload, self.topo, self.patches, 4)
        a = np.random.default_rng(919).normal(size=m.shape[0]); flux = self.topo['incidence']@a
        result = recovered_energy(h, q, j, z, flux)
        np.testing.assert_allclose(result['moments'].reshape(-1,3).sum(axis=1), flux, atol=2e-12)
        # Same three canonical moments represent both traces, including permutation/sign.
        for ci in range(len(self.payload['cells'])):
            local = result['moments'][self.topo['moment_ids'][ci]]*self.topo['moment_signs'][ci]
            divergence_integral = np.array([-1., 1., -1., 1.])@local.reshape(4,3).sum(axis=1)
            self.assertLess(abs(divergence_integral), 2e-11)

    def test_geometric_face_collision_is_refused(self):
        import copy
        broken = copy.deepcopy(self.payload)
        v = np.array(broken['cells'][1]['vertices'][0]); v[0] += .001; v /= np.linalg.norm(v)
        broken['cells'][1]['vertices'][0] = v.tolist()
        with self.assertRaises(ValueError): topology(broken)


if __name__ == '__main__': unittest.main()
