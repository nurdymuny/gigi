"""LINK_FORCE parity and BW05 (YM2, docs/YANG_MILLS_ENGINE_SPEC.md).

For each level, on the paper's quotient meshes with the stored native links (q6, Berger g = (3, 1, 1)):

  1. the engine gradient (examples/link_force_probe.rs) against scripts/ricci_su2_pg_force.py's
     gradient for the same method: relative difference <= 1e-12;
  2. BW05: the independent measure (ricci_lattice_convergence.measures) applied to the engine
     gradient with the frozen native mass; its relative error must equal the paper's Table 6 to
     the printed digit (two decimals, in percent).

Methods: pg2 (two-ring PG), pg2_noregister (the Table 6 removal row), native (also checked
against the frozen native engine gradient).

Requires GIGI_LINK_FORCE_PROBE (built probe) and GIGI_RICCI_DATA_ROOT.
Optional GIGI_PARITY_LEVELS (default "0,1,2"), GIGI_PARITY_METHODS (default "pg2,pg2_noregister,native"),
GIGI_PARITY_RECEIPT (refuses to overwrite).
"""
import json
import os
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path

import numpy as np

REPO = Path(__file__).resolve().parents[2]
DATA_ROOT = Path(os.environ.get("GIGI_RICCI_DATA_ROOT", REPO))
os.environ.setdefault("GIGI_RICCI_DATA_ROOT", str(DATA_ROOT))
sys.path.insert(0, str(REPO / "scripts"))
sys.path.insert(0, str(REPO / "e2e/probes"))
import ricci_su2_pg_force as F                       # noqa: E402
from ricci_lattice_convergence import measures       # noqa: E402

LEVELS = [int(x) for x in os.environ.get("GIGI_PARITY_LEVELS", "0,1,2").split(",")]
METHODS = os.environ.get("GIGI_PARITY_METHODS", "pg2,pg2_noregister,native").split(",")
TRIAL = {"pg2": "pg", "pg2_noregister": "pg_noregister", "native": "native"}
# Reading the Curve, Table 6 (relative full-flow error, %, q6), n = 1, 2, 3, 5
TABLE6 = {"pg2": [36.25, 7.43, 4.69, 2.51], "pg2_noregister": [39.02, 28.72, 28.95, 29.19],
          "native": [26.25, 24.74, 24.96, 25.51]}
METRIC = [3.0, 1.0, 1.0]
ORDER = 6


def run_engine(payload, links, method):
    with tempfile.TemporaryDirectory(prefix="gigi-force-") as d:
        inp, out = Path(d) / "in.json", Path(d) / "out.json"
        inp.write_text(json.dumps({"metric": METRIC, "order": ORDER, "method": method, "rings": 2,
                                   "links": [[float(x) for x in q] for q in links],
                                   "cells": [{"vertices": c["vertices"], "edges": c["edges"],
                                              "reverse": [bool(r) for r in c["reverse"]],
                                              "weight": float(c["weight"])} for c in payload["cells"]]}))
        subprocess.run([os.environ["GIGI_LINK_FORCE_PROBE"], str(inp), str(out)], check=True, timeout=7200)
        return json.loads(out.read_text())


class Parity(unittest.TestCase):
    def test_force_parity_and_bw05(self):
        receipt = {"kind": "YM2 parity (engine link_force_probe vs scripts/ricci_su2_pg_force.py) and BW05 via measures()",
                   "metric": METRIC, "order": ORDER}
        for level in LEVELS:
            payload = json.loads((F.BERGER / f"level{level}_berger_q{ORDER}.input.json").read_text())
            raw = json.loads((F.BERGER / f"level{level}_berger_q{ORDER}.output.json").read_text())
            links = np.asarray(raw["links"], float)
            for method in METHODS:
                eng = run_engine(payload, links, method)
                ge = np.asarray(eng["gradient"], float)
                t = time.time()
                ref = F.run(level, ORDER, TRIAL[method])
                py_s = time.time() - t
                gp = ref["_grad"]
                rel = float(np.linalg.norm(ge - gp) / np.linalg.norm(gp))
                bw = measures(payload, METRIC, dict(raw, gradient=ge.tolist()))["relative_error"] * 100
                want = TABLE6[method][level]
                row = {"gradient_rel_vs_research": rel, "bw05_percent": bw, "table6_percent": want,
                       "table6_match_2dp": round(bw, 2) == want, "variational": eng["variational"],
                       "engine_seconds": eng["seconds"], "research_seconds": round(py_s, 1)}
                if method == "native":
                    gn = np.asarray(raw["gradient"], float)
                    row["gradient_rel_vs_frozen_native_engine"] = float(np.linalg.norm(ge - gn) / np.linalg.norm(gn))
                receipt[f"level{level}_{method}"] = row
                print(level, method, row, flush=True)
                self.assertLess(rel, 1e-12)
                self.assertTrue(row["table6_match_2dp"], f"BW05 {bw:.4f}% vs Table 6 {want}%")
                self.assertEqual(eng["variational"], method == "native")
        out = os.environ.get("GIGI_PARITY_RECEIPT")
        if out:
            p = Path(out)
            if p.exists():
                raise SystemExit(f"refusing to overwrite {p}")
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_text(json.dumps(receipt, indent=2) + "\n")


if __name__ == "__main__":
    unittest.main()
