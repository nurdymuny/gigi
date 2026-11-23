"""RECOVER_CURVATURE parity: the engine kernel against the paper's Python fit.

Runs the Rust kernel (via examples/recover_curvature_probe.rs) and
scripts/ricci_pg_tworing.py's fit_cell_rings on the paper's frozen quotient
meshes, with the paper's Abelian edge data, and compares every cell.

The Python chart takes its frame from numpy's Householder QR, whose column
signs are arbitrary. The engine uses the canonical frame (R's diagonal
positive). The two differ only by axis signs, which change no amplification,
condition or flux, but do flip coefficient signs. So the Python chart is
canonicalised here before coefficients are compared. Removing that
canonicalisation must make the coefficient check fail (M4).

Requires:
  GIGI_RECOVER_CURVATURE_PROBE  path to the built probe binary
  GIGI_RICCI_DATA_ROOT          repository root holding the extracted frozen
                                inputs under output/ (default: this repo)
Optional:
  GIGI_PARITY_LEVELS            comma list, default "0,1,2"
"""
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

import numpy as np

REPO = Path(__file__).resolve().parents[2]
DATA_ROOT = Path(os.environ.get("GIGI_RICCI_DATA_ROOT", REPO))
sys.path.insert(0, str(DATA_ROOT / "scripts"))
sys.path.insert(0, str(DATA_ROOT / "e2e/probes"))
import ricci_pg_curvature as pg        # noqa: E402
import ricci_pg_tworing as tw          # noqa: E402

LEVELS = [int(x) for x in os.environ.get("GIGI_PARITY_LEVELS", "0,1,2").split(",")]
_original_chart = pg.chart


def canonical_chart(K):
    """pg.chart with Q's column signs fixed so R's diagonal is positive."""
    p0, b = K[0], (K[1:] - K[0]).T
    n = np.linalg.svd(b.T)[2][-1]
    c = float(n @ p0)
    if c < 0:
        n, c = -n, -c
    Q, R = np.linalg.qr(b)
    Q = Q * np.sign(np.diag(R))
    cen = K.mean(axis=0)
    cen = cen * (c / (cen @ n))
    diam = max(np.linalg.norm(K[i] - K[j]) for i in range(4) for j in range(i + 1, 4))
    J = (Q.T @ b) / diam
    t0 = ((p0 - cen) @ Q) / diam

    def to_orth(V):
        den = V @ n
        if np.any(den <= 0):
            raise ValueError("neighbourhood vertex outside the projective chart")
        return ((V * (c / den)[:, None] - cen) @ Q) / diam
    return to_orth, J, t0


def run_engine(cells, a, rings):
    with tempfile.TemporaryDirectory(prefix="gigi-recover-") as d:
        inp, out = Path(d) / "in.json", Path(d) / "out.json"
        inp.write_text(json.dumps({
            "rings": rings, "a": [float(x) for x in a],
            "cells": [{"vertices": c["vertices"], "edges": c["edges"],
                       "reverse": [bool(r) for r in c["reverse"]]} for c in cells]}))
        subprocess.run([os.environ["GIGI_RECOVER_CURVATURE_PROBE"], str(inp), str(out)],
                       check=True, timeout=900)
        return json.loads(out.read_text())


def run_python(cells, a, rings, chart):
    pg.chart = chart
    try:
        faces = pg.face_index(cells)
        return [tw.fit_cell_rings(cells, a, faces, ci, rings=rings) for ci in range(len(cells))]
    finally:
        pg.chart = _original_chart


def compare(eng, py):
    """Worst relative coefficient gap, and the other fields' worst gaps."""
    worst = {"coef_rel": 0.0, "amplification_rel": 0.0, "condition_rel": 0.0,
             "faces": 0, "cells": 0}
    for e, p in zip(eng["cells"], py):
        x = np.asarray(p["x"])
        scale = max(1.0, float(np.max(np.abs(x))))
        worst["coef_rel"] = max(worst["coef_rel"],
                                float(np.max(np.abs(np.asarray(e["coefficients"]) - x))) / scale)
        amp = float(np.linalg.norm(p["map"], 2))
        worst["amplification_rel"] = max(worst["amplification_rel"], abs(e["amplification"] - amp) / amp)
        worst["condition_rel"] = max(worst["condition_rel"], abs(e["condition"] - p["cond"]) / p["cond"])
        worst["faces"] = max(worst["faces"], abs(e["faces"] - p["faces"]))
        worst["cells"] = max(worst["cells"], abs(e["cells"] - p["cells"]))
    return worst


class Parity(unittest.TestCase):
    def test_engine_matches_paper_fit(self):
        receipt = {}
        for level in LEVELS:
            base = pg.load_cells(level)
            cells, a = base["cells"], base["a"]
            for rings in (1, 2):
                eng = run_engine(cells, a, rings)
                py = run_python(cells, a, rings, canonical_chart)
                w = compare(eng, py)
                receipt[f"level{level}_r{rings}"] = dict(w, n_cells=len(cells),
                                                         engine_seconds=eng["seconds"])
                print(level, rings, w, f"{eng['seconds']:.2f}s", flush=True)
                self.assertEqual(len(eng["cells"]), len(cells))
                self.assertLess(w["coef_rel"], 1e-9)
                self.assertLess(w["amplification_rel"], 1e-9)
                self.assertLess(w["condition_rel"], 1e-7)
                self.assertEqual(w["faces"], 0)
                self.assertEqual(w["cells"], 0)
        out = os.environ.get("GIGI_PARITY_RECEIPT")
        if out:
            p = Path(out)
            if p.exists():
                raise SystemExit(f"refusing to overwrite {p}")
            p.write_text(json.dumps(receipt, indent=2) + "\n")

    def test_m4_without_canonical_chart_coefficients_disagree(self):
        """Mechanism removal: compare against the raw numpy chart. Amplification
        must still agree; coefficients must not (somewhere a sign is flipped)."""
        level = LEVELS[0]
        base = pg.load_cells(level)
        eng = run_engine(base["cells"], base["a"], 2)
        py = run_python(base["cells"], base["a"], 2, _original_chart)
        w = compare(eng, py)
        self.assertLess(w["amplification_rel"], 1e-9)
        self.assertGreater(w["coef_rel"], 1e-6)


if __name__ == "__main__":
    unittest.main()
