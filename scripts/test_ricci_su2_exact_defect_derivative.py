"""Checks D-agree, D-group, D-engine and D-noise for the exact SU(2) defect derivative
(docs/RICCI_SU2_EXACT_DEFECT_DERIVATIVE_SPEC.md). Set GIGI_RICCI_DATA_ROOT to the repo holding
the extracted output/.

usage: python scripts/test_ricci_su2_exact_defect_derivative.py
"""
import json
import sys
import unittest
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_pg_force as F     # noqa: E402
import ricci_su2_pg_flow as FL     # noqa: E402


def random_unit(rng, n):
    q = rng.normal(size=(n, 4))
    return q / np.linalg.norm(q, axis=1, keepdims=True)


def fd4(u, e, k, h=1e-3):
    """fourth-order central difference of the face defects along u_e -> u_e exp(t tau_k)"""
    def at(t):
        v = u.copy()
        v[e] = F.compose(u[e], np.r_[np.cos(t), np.sin(t) * F.TAU[k][1:]])
        return F.defects(v)
    return (-at(2 * h) + 8 * at(h) - 8 * at(-h) + at(-2 * h)) / (12 * h)


class ExactDefectDerivative(unittest.TestCase):
    def test_d_group(self):
        rng = np.random.default_rng(0)
        worst = 0.0
        for _ in range(20):
            u = random_unit(rng, 6)
            dD = F.defect_derivatives(u)
            for e in range(6):
                for k in range(3):
                    ref = fd4(u, e, k)
                    worst = max(worst, np.abs(dD[e, k] - ref).max() / max(np.abs(ref).max(), 1e-300))
        self.assertLess(worst, 1e-9, f"D-group {worst:.1e}")

    def test_d_agree(self):
        payload = json.loads((F.BERGER / "level0_berger_q6.input.json").read_text())
        raw = json.loads((F.BERGER / "level0_berger_q6.output.json").read_text())
        links = np.asarray(raw["links"], float)
        worst = 0.0
        for cell in payload["cells"]:
            u = F.local_links(cell, links)
            ex, fd = F.defect_derivatives(u), F.defect_derivatives_fd(u)
            worst = max(worst, np.abs(ex - fd).max() / np.abs(ex).max())
        self.assertLess(worst, 1e-8, f"D-agree {worst:.1e}")

    def test_d_engine(self):
        for level in (0, 1):
            r = F.run(level, 6, "native")
            self.assertLess(r["gradient_vs_native_rel"], 1e-12,
                            f"D-engine n-level {level}: {r['gradient_vs_native_rel']:.1e}")

    def test_d_noise(self):
        met = FL.Metric()
        g = met(0.0)
        for trial in ("native", "pg"):
            lat = FL.Lattice(0, trial)
            mp = lat.metric_pieces(g)
            U = np.asarray(lat.raw["links"], float)
            G0 = lat.gradient(U, mp)
            rng = np.random.default_rng(0)
            for _ in range(3):
                Up = U * (1 + 2.2e-16 * rng.standard_normal(U.shape))
                Up /= np.linalg.norm(Up, axis=1, keepdims=True)
                rel = np.linalg.norm(lat.gradient(Up, mp) - G0) / np.linalg.norm(G0)
                self.assertLess(rel, 1e-12, f"D-noise {trial}: {rel:.1e}")


if __name__ == "__main__":
    unittest.main(verbosity=2)
