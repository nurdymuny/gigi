"""RF2H focused tests (see docs/RICCI_ADJOINT_CONSISTENT_RECOVERY_SPEC.md)."""
import json
import unittest
from pathlib import Path

import numpy as np
from scipy.sparse import load_npz

ROOT = Path(__file__).resolve().parents[2]
SHAPE = ROOT/'output/ricci-shape-regular-20261117/berger'
VAR = ROOT/'output/ricci-variational-20261117/abelian'


def level_payload(level):
    return json.loads((SHAPE/f'level{level}_berger_q6.input.json').read_text())


class GeometryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        import ricci_rf2h as g
        cls.g = g
        cls.mesh = g.Mesh(level_payload(1))

    def test_basis_closed_on_every_cell_image(self):
        g, mesh = self.g, self.mesh
        rng = np.random.default_rng(1)
        for c in rng.choice(len(mesh.cells), 12, replace=False):
            state = (int(c), int(rng.integers(120)))
            P = np.array([mesh.face_points(fi, d) for fi, d in mesh.cell_face_images(state)])
            flux = g.face_moments(P).sum(axis=1)                                   # (4,26)
            total = sum(mesh.outward_sign(int(c), k)*flux[k] for k in range(4))
            self.assertLess(np.max(abs(total)), 1e-12)

    def test_stokes_flux_equals_potential_circulation(self):
        # omega = d beta, beta = x_0 x_1 dx_2 + x_3 dx_1 (quadratic 1-form): flux = circulation
        g, mesh = self.g, self.mesh
        P = mesh.face_points(5, 7)
        def beta(x):
            return np.stack([np.zeros_like(x[..., 0]), x[..., 3], x[..., 0]*x[..., 1], np.zeros_like(x[..., 0])], -1)
        def dbeta(x):
            m = np.zeros(x.shape[:-1]+(4, 4))
            m[..., 0, 2] = x[..., 1]; m[..., 1, 2] = x[..., 0]; m[..., 3, 1] = 1.
            return m-np.swapaxes(m, -1, -2)
        def circ(pa, pb):
            r, w = np.polynomial.legendre.leggauss(12); t = (r+1)/2; w = w/2
            y = pa+t[:, None]*(pb-pa); n = np.linalg.norm(y, axis=1); x = y/n[:, None]
            dx = ((pb-pa)-x*(x@(pb-pa))[:, None])/n[:, None]
            return float(np.sum(w*np.sum(beta(x)*dx, axis=1)))
        u, v, w = g.tri_rule(9)
        d1, d2 = P[1]-P[0], P[2]-P[0]; y = P[0]+u[:, None]*d1+v[:, None]*d2
        n = np.linalg.norm(y, axis=1); x = y/n[:, None]
        xu = (d1-x*(x@d1)[:, None])/n[:, None]; xv = (d2-x*(x@d2)[:, None])/n[:, None]
        flux = float(np.sum(w*np.einsum('qi,qij,qj->q', xu, dbeta(x), xv)))
        self.assertLess(abs(flux-(circ(P[0], P[1])+circ(P[1], P[2])-circ(P[0], P[2]))), 1e-12)
        # and dbeta lies in the ambient closed affine family
        coeffs = np.linalg.lstsq(np.concatenate([g.AMB_A.reshape(26, -1), g.AMB_B.reshape(26, -1)], 1).T,
            np.concatenate([dbeta(np.zeros(4)).ravel(), np.stack([dbeta(e)-dbeta(np.zeros(4)) for e in np.eye(4)]).ravel()]), rcond=None)[0]
        self.assertLess(abs(g.face_moments(P[None])[0].sum(axis=0)@coeffs-flux), 1e-12)

    def test_deck_transport_of_moments(self):
        g, mesh = self.g, self.mesh
        canon = g.face_moments(mesh.topo['faces'][[9]])[0]
        for d in [3, 17, 88]:
            T = g.deck_matrix(g.left_matrix(mesh.values[d]))
            direct = g.face_moments(mesh.face_points(9, d)[None])[0]
            self.assertLess(np.max(abs(direct-canon@T)), 1e-12)

    def test_flux_of_quotient_face_is_deck_invariant_for_invariant_form(self):
        g, mesh = self.g, self.mesh
        a = mesh.maurer_cartan_flux(9, 0)
        for d in [3, 17, 88]:
            self.assertLess(abs(mesh.maurer_cartan_flux(9, d)-a), 1e-12)

    def test_face_images_of_neighbor_match_shared_face(self):
        g, mesh = self.g, self.mesh
        for c in range(0, len(mesh.cells), 97):
            state = (c, 0)
            for k, (fi, d) in enumerate(mesh.cell_face_images(state)):
                nb = mesh.neighbor(state, k)[0]
                self.assertIn((fi, d), mesh.cell_face_images(nb))


class RecoveryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        import ricci_rf2h as g
        cls.g = g
        cls.level = 1
        cls.build = g.build_recovery(level_payload(1), order=6, verbose=False)
        cls.d = load_npz(VAR/'level1_q6.d.npz'); cls.h = load_npz(VAR/'level1_q6.h.npz')

    def test_constraints_hold_exactly(self):
        r = self.build['receipt']
        self.assertLess(r['acc_relative_residual'], 1e-9)
        self.assertLess(r['adj_relative_residual'], 1e-9)

    def test_fluxes_are_preserved_for_every_input(self):
        rng = np.random.default_rng(3); a = rng.normal(size=self.d.shape[1])
        c = self.build['C']@(self.d@a)
        self.assertLess(np.max(abs(c.reshape(-1, 3).sum(axis=1)-self.d@a)), 1e-12)

    def test_energy_derivative_matches_finite_difference(self):
        rng = np.random.default_rng(4); a = rng.normal(size=self.d.shape[1]); eta = rng.normal(size=a.size)
        E = lambda x: self.g.energy(self.build['C'], self.h, self.d, x)
        grad = self.g.gradient(self.build['C'], self.h, self.d, a)
        fd = (E(a+1e-6*eta)-E(a-1e-6*eta))/2e-6
        self.assertLess(abs(fd-grad@eta)/max(1., abs(fd)), 1e-7)

    def test_zero_and_pure_gauge_give_zero_force(self):
        z = np.zeros(self.d.shape[1])
        self.assertEqual(np.max(abs(self.g.gradient(self.build['C'], self.h, self.d, z))), 0.)


if __name__ == '__main__':
    unittest.main(verbosity=2)
