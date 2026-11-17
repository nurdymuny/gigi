"""Focused tests for the two-ring fit (scripts/ricci_pg_tworing.py)."""
import sys, unittest
from pathlib import Path
import numpy as np
sys.path[:0] = [str(Path(__file__).resolve().parent), str(Path(__file__).resolve().parents[1]/'e2e/probes')]
import ricci_pg_curvature as pg
import ricci_pg_tworing as tr


class TwoRing(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.L = pg.load_cells(1); cls.cells = cls.L['cells']; cls.faces = pg.face_index(cls.cells)

    def test_planted_closed_linear_field_recovered(self):
        rng = np.random.default_rng(0)
        for ci in rng.choice(len(self.cells), 8, replace=False):
            x_true = rng.normal(size=11)
            flux = lambda tri: float(pg.flux_row(tri)@x_true)
            fit = tr.fit_cell_rings(self.cells, flux, self.faces, int(ci))
            self.assertLess(np.max(abs(fit['x']-x_true)), 1e-10)
            self.assertGreater(fit['faces'], 16)            # overdetermined, unlike the one-ring star

    def test_own_fluxes_preserved_and_linear(self):
        rng = np.random.default_rng(1); a1 = rng.normal(size=len(self.L['a'])); a2 = rng.normal(size=a1.size)
        for ci in [0, 101, 517]:
            f1 = tr.fit_cell_rings(self.cells, a1, self.faces, ci)
            f2 = tr.fit_cell_rings(self.cells, a2, self.faces, ci)
            f12 = tr.fit_cell_rings(self.cells, 2*a1-3*a2, self.faces, ci)
            self.assertLess(np.max(abs(f12['x']-(2*f1['x']-3*f2['x']))), 1e-10)
            own = [pg.data_flux(self.cells[ci], a1, f) for f in pg.FACES]
            to_orth, _, _ = pg.chart(np.array(self.cells[ci]['vertices'], float))
            O = to_orth(np.array(self.cells[ci]['vertices'], float))
            pred = [float(pg.flux_row(O[list(f)])@f1['x']) for f in pg.FACES]
            self.assertLess(np.max(abs(np.array(pred)-own)), 1e-10)

    def test_zero_data_gives_zero(self):
        # Gauge invariance is structural (the fit consumes face fluxes only, and the flux of
        # a gradient vanishes by incidence); this test only checks the zero-data control.
        fit = tr.fit_cell_rings(self.cells, np.zeros(len(self.L['a'])), self.faces, 3)
        self.assertEqual(float(np.max(abs(fit['x']))), 0.)

    def test_one_ring_reproduces_original_fit(self):
        rng = np.random.default_rng(3); a = rng.normal(size=len(self.L['a']))
        for ci in [0, 250, 900]:
            new = tr.fit_cell_rings(self.cells, a, self.faces, ci, rings=1)
            old = pg.fit_cell(self.cells, a, self.faces, ci)
            self.assertLess(np.max(abs(new['x']-old['x'])), 1e-10)


if __name__ == '__main__':
    unittest.main(verbosity=2)
