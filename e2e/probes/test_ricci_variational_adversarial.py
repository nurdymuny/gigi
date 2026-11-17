"""Independent geometric orientation and energy checks on actual quotient data."""
import unittest
import numpy as np
from ricci_consistency_diagnostic import BASE
from ricci_lattice_evidence import read
from ricci_lattice_mesh import plus
from ricci_variational_math import recovered_energy, FACES
from ricci_variational_mesh import topology, recover_patches, deck
from ricci_variational_assembly import assemble


class AdversarialTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.payload = read(BASE/'level0_berger_q6.input.json')
        cls.topo = topology(cls.payload); cls.patches, _ = recover_patches(cls.payload, cls.topo)

    def test_face_vertex_moments_match_geometric_deck_images(self):
        elements = deck()[0]
        for ci, cell in enumerate(self.payload['cells']):
            for local_face, verts in enumerate(FACES):
                fi, gi = self.topo['cell_faces'][ci][local_face]
                canonical = plus(elements[gi], self.topo['faces'][fi])
                for j, vertex in enumerate(verts):
                    moment = self.topo['moment_ids'][ci, 3*local_face+j]
                    np.testing.assert_allclose(canonical[moment%3], cell['vertices'][vertex], atol=3e-12)

    def test_actual_mesh_full_reduced_energy_derivative(self):
        h, q, _, j, z, _, _ = assemble(self.payload, self.topo, self.patches, 4)
        d = self.topo['incidence']; rng = np.random.default_rng(421)
        a = rng.normal(size=d.shape[1]); direction = rng.normal(size=d.shape[1])
        actual = recovered_energy(h, q, j, z, d@a)
        expected = direction@(d.T@actual['gradient'])
        errors = []
        for step in [1e-3, 5e-4, 2.5e-4]:
            ep = recovered_energy(h, q, j, z, d@(a+step*direction))['energy']
            em = recovered_energy(h, q, j, z, d@(a-step*direction))['energy']
            errors.append(abs((ep-em)/(2*step)-expected)/max(abs(expected), 1.))
        self.assertLess(max(errors), 2e-8)
        stationarity = z.T@(h@actual['moments']-q@(d@a))
        self.assertLess(np.linalg.norm(stationarity)/max(np.linalg.norm(q@(d@a)), 1.), 1e-10)


if __name__ == '__main__': unittest.main()
