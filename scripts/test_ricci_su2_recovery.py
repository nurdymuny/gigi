"""Gates G0, G1 and mechanism removals M1, M2 for the SU(2) recovery prototype
(docs/RECOVER_CURVATURE_SU2_SPEC.md). Reads the frozen meshes and the paper's
Abelian data; set GIGI_RICCI_DATA_ROOT to the repo holding the extracted output/.

usage: python scripts/test_ricci_su2_recovery.py   (levels via GIGI_SU2_LEVELS, default 0,1)
"""
import os
import sys
import unittest
from pathlib import Path

import numpy as np

DATA = Path(os.environ.get("GIGI_RICCI_DATA_ROOT", Path(__file__).resolve().parents[1]))
sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_pg_curvature as pg     # noqa: E402
pg.BASE = DATA / "output/ricci-shape-regular-20261117/berger"
pg.DIAG = DATA / "output/ricci-consistency-20261117/diagnostic"
import ricci_pg_tworing as tw      # noqa: E402
import ricci_su2_recovery as su    # noqa: E402

LEVELS = [int(x) for x in os.environ.get("GIGI_SU2_LEVELS", "0,1").split(",")]


def random_unit(rng):
    q = rng.normal(size=4)
    return q / np.linalg.norm(q)


class Gates(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.meshes = {}
        for level in LEVELS:
            base = pg.load_cells(level)
            cells, a = base["cells"], np.asarray(base["a"], float)
            faces = pg.face_index(cells)
            vid = su.vertex_ids(cells)
            cls.meshes[level] = (cells, a, faces, vid)

    def sample(self, level, k=12):
        cells = self.meshes[level][0]
        rng = np.random.default_rng(level)
        return sorted(rng.choice(len(cells), size=min(k, len(cells)), replace=False).tolist())

    def test_g0_embedded_u1_reproduces_the_paper(self):
        worst = 0.0
        for level, (cells, a, faces, vid) in self.meshes.items():
            links = su.abelian_links(a, axis=2)
            for ci in self.sample(level):
                fit = tw.fit_cell_rings(cells, a, faces, ci, rings=2)
                r = su.recover_cell(cells, faces, vid, links, ci, fit=fit)
                C = r["coefficients"]
                scale = max(1.0, float(np.max(np.abs(fit["x"]))))
                worst = max(worst, float(np.max(np.abs(C[2] - fit["x"]))) / scale,
                            float(np.max(np.abs(C[:2]))) / scale)
        print(f"G0 worst relative gap {worst:.2e}")
        self.assertLess(worst, 1e-12)

    def test_g1_gauge_covariance(self):
        rng = np.random.default_rng(7)
        worst = 0.0
        for level, (cells, a, faces, vid) in self.meshes.items():
            # a genuinely non-Abelian field: random links (small angles)
            links = np.array([su.qexp(0.2 * rng.normal(size=3)) for _ in range(len(a))])
            gauge = {v: random_unit(rng) for v in set(vid.values())}
            moved = su.gauge_transform(cells, vid, links, gauge)
            for ci in self.sample(level, 8):
                fit = tw.fit_cell_rings(cells, a, faces, ci, rings=2)
                r0 = su.recover_cell(cells, faces, vid, links, ci, fit=fit)
                r1 = su.recover_cell(cells, faces, vid, moved, ci, fit=fit)
                g = gauge[r0["frame_vertex"]]
                rotated = np.array([su.ad(g, col) for col in r0["coefficients"].T]).T
                worst = max(worst, float(np.max(np.abs(rotated - r1["coefficients"]))))
        print(f"G1 worst covariance gap {worst:.2e}")
        self.assertLess(worst, 1e-10)

    def test_m1_without_register_covariance_fails(self):
        rng = np.random.default_rng(7)
        level = LEVELS[0]
        cells, a, faces, vid = self.meshes[level]
        links = np.array([su.qexp(0.2 * rng.normal(size=3)) for _ in range(len(a))])
        gauge = {v: random_unit(rng) for v in set(vid.values())}
        moved = su.gauge_transform(cells, vid, links, gauge)
        worst = 0.0
        for ci in self.sample(level, 8):
            fit = tw.fit_cell_rings(cells, a, faces, ci, rings=2)
            r0 = su.recover_cell(cells, faces, vid, links, ci, fit=fit, register=False)
            r1 = su.recover_cell(cells, faces, vid, moved, ci, fit=fit, register=False)
            g = gauge[r0["frame_vertex"]]
            rotated = np.array([su.ad(g, col) for col in r0["coefficients"].T]).T
            worst = max(worst, float(np.max(np.abs(rotated - r1["coefficients"]))))
        print(f"M1 (no register) covariance gap {worst:.2e}")
        self.assertGreater(worst, 1e-3)

    def test_m2_abelianised_flux_is_invisible_to_abelian_data(self):
        level = LEVELS[0]
        cells, a, faces, vid = self.meshes[level]
        links = su.abelian_links(a, axis=2)
        worst = 0.0
        for ci in self.sample(level, 6):
            fit = tw.fit_cell_rings(cells, a, faces, ci, rings=2)
            full = su.recover_cell(cells, faces, vid, links, ci, fit=fit)
            ab = su.recover_cell(cells, faces, vid, links, ci, fit=fit, abelianised=True)
            worst = max(worst, float(np.max(np.abs(full["coefficients"] - ab["coefficients"]))))
        print(f"M2 on Abelian data: gap {worst:.2e} (must be ~0; G2 is where M2 must fail)")
        self.assertLess(worst, 1e-12)


if __name__ == "__main__":
    unittest.main(verbosity=2)
