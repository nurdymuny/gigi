"""Focused tests for docs/RICCI_PG_CURVATURE_SPEC.md (T1-T8). Level 0 unless noted."""
import sys
import unittest
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_pg_curvature as pg   # noqa: E402

FACES = [(0, 1, 2), (0, 1, 3), (0, 2, 3), (1, 2, 3)]
EDGES = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)]
RNG = np.random.default_rng(20261117)


def global_vertices(cells):
    """Test-owned oracle: global vertex id of each (cell, local vertex), by
    union-find over shared endpoints of global edges. Independent of pg."""
    parent = {}

    def find(x):
        parent.setdefault(x, x)
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    for ci, c in enumerate(cells):
        for k, (i, j) in enumerate(EDGES):
            tail, head = (j, i) if c["reverse"][k] else (i, j)
            e = c["edges"][k]
            parent.setdefault((ci, tail), (ci, tail))
            parent.setdefault((ci, head), (ci, head))
            for local, end in ((tail, "t"), (head, "h")):
                ra, rb = find((ci, local)), find((e, end))
                if ra != rb:
                    parent[ra] = rb
    return {key: find(key) for key in list(parent) if isinstance(key[1], int)}, find


class PG(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.L0 = pg.load_level(0, 6)
        cls.cells, cls.a = cls.L0["cells"], cls.L0["a"]
        cls.faces = pg.face_index(cls.cells)

    # T1 ---------------------------------------------------------------------
    def test_T1_planted_closed_linear_recovered(self):
        c = RNG.normal(size=3)
        B = RNG.normal(size=(3, 3)); B -= np.eye(3) * np.trace(B) / 3

        def planted(tri):
            z1, z2, z3 = tri
            return (c + B @ ((z1 + z2 + z3) / 3)) @ (np.cross(z2 - z1, z3 - z1) / 2)

        for ci in (0, 17, 63, 119):
            fit = pg.fit_cell(self.cells, planted, self.faces, ci)
            self.assertLess(np.max(abs(fit["c"] - c)), 1e-12)
            self.assertLess(np.max(abs(fit["B"] - B)), 1e-12)

    # T2 ---------------------------------------------------------------------
    def test_T2_own_face_fluxes_preserved_in_reference_coordinates(self):
        worst = 0.
        for ci in range(0, 120, 7):
            fit = pg.fit_cell(self.cells, self.a, self.faces, ci)
            for f in FACES:
                got = pg.reference_flux(fit, f)
                want = pg.data_flux(self.cells[ci], self.a, f)
                worst = max(worst, abs(got - want))
        self.assertLess(worst, 1e-12)

    # T3 ---------------------------------------------------------------------
    def test_T3_linear_in_the_data(self):
        b = RNG.normal(size=self.a.shape)
        for ci in (3, 40, 99):
            f1 = pg.fit_cell(self.cells, self.a, self.faces, ci)
            f2 = pg.fit_cell(self.cells, b, self.faces, ci)
            f3 = pg.fit_cell(self.cells, 2 * self.a - 3 * b, self.faces, ci)
            self.assertLess(np.max(abs(f3["c"] - (2 * f1["c"] - 3 * f2["c"]))), 1e-12)
            self.assertLess(np.max(abs(f3["B"] - (2 * f1["B"] - 3 * f2["B"]))), 1e-12)

    # T4 ---------------------------------------------------------------------
    def test_T4_pure_gauge_reconstructs_to_zero(self):
        vid, _ = global_vertices(self.cells)
        phi = {v: RNG.normal() for v in set(vid.values())}
        gauge = np.zeros_like(self.a)
        for ci, c in enumerate(self.cells):
            for k, (i, j) in enumerate(EDGES):
                tail, head = (j, i) if c["reverse"][k] else (i, j)
                gauge[c["edges"][k]] = phi[vid[(ci, head)]] - phi[vid[(ci, tail)]]
        for ci in range(0, 120, 11):
            for f in FACES:
                self.assertLess(abs(pg.data_flux(self.cells[ci], gauge, f)), 1e-12)
            fit = pg.fit_cell(self.cells, gauge, self.faces, ci)
            self.assertLess(max(np.max(abs(fit["c"])), np.max(abs(fit["B"]))), 1e-12)

    # T5 ---------------------------------------------------------------------
    def test_T5_exact_curvature_convention_matches_stokes(self):
        worst = 0.
        for ci, c in enumerate(self.cells):
            pts = np.array(c["vertices"])
            for f in FACES:
                worst = max(worst, abs(pg.exact_face_flux(pts, f) - pg.data_flux(c, self.a, f)))
        self.assertLess(worst, 1e-9)

    # T6 ---------------------------------------------------------------------
    def test_T6_whitney_trial_reproduces_native_stiffness(self):
        q = pg.quadrature(np.array([c["vertices"] for c in self.cells[:5]]), 6)
        for f in range(4):
            for g, face in enumerate(FACES):
                # flux of Whitney face function f through reference face g: Kronecker delta
                self.assertAlmostEqual(pg.reference_flux(("whitney", f), face), float(f == g), places=12)
        built = pg.build(0, 6, trial="whitney")
        want = self.L0["stiffness"] @ self.a
        self.assertLess(np.max(abs(built["force"] - want)) / np.max(abs(want)), 1e-12)

    # T7 ---------------------------------------------------------------------
    def test_T7_cell_alone_refuses_on_rank(self):
        with self.assertRaisesRegex(ValueError, "rank"):
            pg.fit_cell(self.cells, self.a, self.faces, 5, include_neighbours=False)

    # T8 ---------------------------------------------------------------------
    def test_T8_untransported_off_sheet_neighbour_refuses(self):
        cells = pg.load_cells(1)["cells"]
        faces = pg.face_index(cells)
        refused = 0
        for ci in range(len(cells)):
            pg.neighbourhood(cells, faces, ci, transport=True)   # must never refuse
            try:
                pg.neighbourhood(cells, faces, ci, transport=False)
            except ValueError as err:
                self.assertIn("coincide", str(err))
                refused += 1
        self.assertGreater(refused, 0, "no off-sheet neighbour found: T8 would test nothing")


class PGInterpolation(unittest.TestCase):
    """T9, added after the first mutation run showed the fit is exactly determined."""

    @classmethod
    def setUpClass(cls):
        L = pg.load_cells(0)
        cls.cells, cls.a = L["cells"], L["a"]
        cls.faces = pg.face_index(cls.cells)

    def test_T9_all_star_fluxes_interpolated_and_violations_refused(self):
        for ci in range(0, 120, 9):
            fit = pg.fit_cell(self.cells, self.a, self.faces, ci)
            self.assertLess(fit["residual"], 1e-12)
        # Break closure on one face of one star: no closed field can interpolate it.
        z = {}

        def broken(tri):
            key = tuple(np.round(tri.ravel(), 9))
            z.setdefault(key, len(z))
            return 1.0 if z[key] == 0 else 0.0     # one face carries flux, all others zero
        with self.assertRaisesRegex(ValueError, "interpolat"):
            pg.fit_cell(self.cells, broken, self.faces, 7)


class Variational(unittest.TestCase):
    """T10-T13, S4 addendum: variational energy with the same reconstruction."""

    @classmethod
    def setUpClass(cls):
        cls.L0 = pg.load_level(0, 6)
        cls.a = cls.L0["a"]
        cls._kv = None

    @property
    def KV(self):
        # Built lazily so each test reaches its own assertion in the RED phase.
        if Variational._kv is None:
            Variational._kv = pg.build(0, 6, "reconstruction", data=self.L0,
                                       force="variational")["operator"].toarray()
        return Variational._kv

    def test_T10_symmetric_psd_with_gauge_kernel(self):
        from scipy.linalg import eigh
        K = self.KV
        self.assertLess(np.max(abs(K - K.T)) / np.max(abs(K)), 1e-12)
        lam = eigh((K + K.T) / 2, self.L0["mass"].toarray(), eigvals_only=True)
        self.assertEqual(int((abs(lam) < 1).sum()), 21)
        self.assertGreater(float(lam[abs(lam) >= 1].min()), 0.)

    def test_T11_force_is_gradient_of_independent_energy(self):
        for _ in range(3):
            d = RNG.normal(size=self.a.shape)
            eps = 1e-3
            fd = (pg.energy(self.L0, self.a + eps * d, 6) - pg.energy(self.L0, self.a - eps * d, 6)) / (2 * eps)
            an = float((self.KV @ self.a) @ d)
            self.assertLess(abs(fd - an) / max(1., abs(an)), 1e-8)

    def test_T12_whitney_variational_is_native_stiffness(self):
        f = pg.build(0, 6, "whitney", data=self.L0, force="variational")["force"]
        want = self.L0["stiffness"] @ self.a
        self.assertLess(np.max(abs(f - want)) / np.max(abs(want)), 1e-12)

    def test_T13_pure_gauge_gives_zero_force(self):
        cells = self.L0["cells"]
        vid, _ = global_vertices(cells)
        phi = {v: RNG.normal() for v in set(vid.values())}
        gauge = np.zeros_like(self.a)
        for ci, c in enumerate(cells):
            for k, (i, j) in enumerate(EDGES):
                tail, head = (j, i) if c["reverse"][k] else (i, j)
                gauge[c["edges"][k]] = phi[vid[(ci, head)]] - phi[vid[(ci, tail)]]
        self.assertLess(np.max(abs(self.KV @ gauge)), 1e-10 * np.max(abs(self.KV)) * np.max(abs(gauge)))


if __name__ == "__main__":
    unittest.main(verbosity=2)
