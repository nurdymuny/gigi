"""RECOVER_CURVATURE GROUP SU2 parity (YM1, docs/YANG_MILLS_ENGINE_SPEC.md): the engine
kernel against the research script scripts/ricci_su2_recovery.py, on the paper's
quotient meshes with the stored Berger Levi-Civita links (level{L}_berger_q6.output.json).

Both sides use the canonical chart (the Python chart is canonicalised exactly as
in test_recover_curvature_parity.py). Compared per cell: the 3 x 11 coefficients,
the frame vertex, amplification, condition, faces and cells.

Arms:
  full          production reading (register on)
  no_register   mechanism removal (register off): parity must hold here too
  gauged        the links under a random gauge transformation (non-trivial fibre frames)

Requires:
  GIGI_RECOVER_CURVATURE_PROBE  path to the built probe binary (release)
  GIGI_RICCI_DATA_ROOT          repository root holding the frozen inputs under output/
Optional:
  GIGI_PARITY_LEVELS            comma list, default "0,1,2"
  GIGI_PARITY_RECEIPT           write the receipt JSON here (refuses to overwrite)
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
sys.path.insert(0, str(REPO / "scripts"))
sys.path.insert(0, str(REPO / "e2e/probes"))
import ricci_pg_curvature as pg                                    # noqa: E402
pg.BASE = DATA_ROOT / "output/ricci-shape-regular-20261117/berger"
pg.DIAG = DATA_ROOT / "output/ricci-consistency-20261117/diagnostic"
import ricci_pg_tworing as tw                                      # noqa: E402
import ricci_su2_recovery as su                                    # noqa: E402
from test_recover_curvature_parity import canonical_chart          # noqa: E402

LEVELS = [int(x) for x in os.environ.get("GIGI_PARITY_LEVELS", "0,1,2").split(",")]
TOL = 1e-12


def load(level):
    payload = json.loads((pg.BASE / f"level{level}_berger_q6.input.json").read_text())
    raw = json.loads((pg.BASE / f"level{level}_berger_q6.output.json").read_text())
    return payload["cells"], np.asarray(raw["links"], float)


def run_engine(cells, links, rings, register=True):
    with tempfile.TemporaryDirectory(prefix="gigi-su2-") as d:
        inp, out = Path(d) / "in.json", Path(d) / "out.json"
        inp.write_text(json.dumps({
            "group": "su2", "rings": rings, "register": register,
            "links": [[float(x) for x in q] for q in links],
            "cells": [{"vertices": c["vertices"], "edges": c["edges"],
                       "reverse": [bool(r) for r in c["reverse"]]} for c in cells]}))
        subprocess.run([os.environ["GIGI_RECOVER_CURVATURE_PROBE"], str(inp), str(out)],
                       check=True, timeout=3600)
        return json.loads(out.read_text())


def run_python(cells, links, rings, register=True):
    original = pg.chart
    pg.chart = canonical_chart
    try:
        faces = pg.face_index(cells)
        vid = su.vertex_ids(cells)
        out = []
        for ci in range(len(cells)):
            fit = tw.fit_cell_rings(cells, np.zeros(len(links)), faces, ci, rings=rings)
            r = su.recover_cell(cells, faces, vid, links, ci, rings=rings, register=register, fit=fit)
            out.append({"C": np.asarray(r["coefficients"]), "frame": r["frame_vertex"],
                        "amp": float(np.linalg.norm(fit["map"], 2)), "cond": fit["cond"],
                        "faces": fit["faces"], "cells": fit["cells"]})
        return out
    finally:
        pg.chart = original


def compare(eng, py):
    scale = max(float(np.max(np.abs(p["C"]))) for p in py)
    w = {"coef_rel": 0.0, "amplification_rel": 0.0, "condition_rel": 0.0,
         "frame_mismatch": 0, "faces": 0, "cells": 0}
    for e, p in zip(eng["cells"], py):
        w["coef_rel"] = max(w["coef_rel"], float(np.max(np.abs(np.asarray(e["coefficients"]) - p["C"]))) / scale)
        w["amplification_rel"] = max(w["amplification_rel"], abs(e["amplification"] - p["amp"]) / p["amp"])
        w["condition_rel"] = max(w["condition_rel"], abs(e["condition"] - p["cond"]) / p["cond"])
        w["frame_mismatch"] += int(e["frame_vertex"] != p["frame"])
        w["faces"] = max(w["faces"], abs(e["faces"] - p["faces"]))
        w["cells"] = max(w["cells"], abs(e["cells"] - p["cells"]))
    w["coef_scale"] = scale
    return w


def gauged(cells, links, seed):
    rng = np.random.default_rng(seed)
    vid = su.vertex_ids(cells)
    nv = 1 + max(vid.values())
    g = rng.normal(size=(nv, 4))
    g /= np.linalg.norm(g, axis=1, keepdims=True)
    return su.gauge_transform(cells, vid, links, g)


class Parity(unittest.TestCase):
    def test_engine_matches_research_script(self):
        receipt = {"tolerance": TOL, "kind": "YM1 parity (engine probe vs scripts/ricci_su2_recovery.py)"}
        for level in LEVELS:
            cells, links = load(level)
            arms = {"full": (links, True), "no_register": (links, False),
                    "gauged": (gauged(cells, links, 1000 + level), True)}
            for name, (lk, register) in arms.items():
                eng = run_engine(cells, lk, 2, register)
                py = run_python(cells, lk, 2, register)
                w = compare(eng, py)
                receipt[f"level{level}_{name}"] = dict(w, n_cells=len(cells), engine_seconds=eng["seconds"])
                print(level, name, w, f"{eng['seconds']:.2f}s", flush=True)
                self.assertEqual(len(eng["cells"]), len(cells))
                self.assertLess(w["coef_rel"], TOL)
                self.assertLess(w["amplification_rel"], 1e-9)
                self.assertLess(w["condition_rel"], 1e-7)
                self.assertEqual(w["frame_mismatch"], 0)
                self.assertEqual(w["faces"], 0)
                self.assertEqual(w["cells"], 0)
        out = os.environ.get("GIGI_PARITY_RECEIPT")
        if out:
            p = Path(out)
            if p.exists():
                raise SystemExit(f"refusing to overwrite {p}")
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_text(json.dumps(receipt, indent=2) + "\n")


if __name__ == "__main__":
    unittest.main()
