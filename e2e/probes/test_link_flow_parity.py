"""YM4 (docs/YANG_MILLS_ENGINE_SPEC.md): LINK_FLOW against the S3b receipts.

For each level and trial (native, pg, pg_fixed = the frozen-metric control), N = 8 over [0, 0.02]:
the engine's Cayley flow (examples/link_flow_probe.rs) is run with
  * the same preconditioner K0 the research run used (ricci_su2_pg_flow_cayley.load_K), and
  * the same metric: g(t) samples from ricci_su2_pg_flow.Metric (DOP853), the frozen g(0) for
    pg_fixed, and the true flowed metric for the exact reference links.
Compared with research/su2-cayley-20261118/L{level}_{trial}_N8.json and .final.npy:
  every sample's E to 1e-8 relative; identical Newton iteration counts; final links to 1e-8.

Requires GIGI_LINK_FLOW_PROBE and GIGI_RICCI_DATA_ROOT (with output/ and outputs/).
Optional GIGI_PARITY_LEVELS (default "0,1"), GIGI_PARITY_TRIALS (default "native,pg,pg_fixed"),
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
import ricci_su2_pg_flow as FL                      # noqa: E402
import ricci_su2_pg_flow_cayley as CAY              # noqa: E402
CAY.DENSE_K = DATA_ROOT / "outputs" / "su2-numrange"
CAY.SPARSE_K = DATA_ROOT / "outputs" / "su2-cayley"

LEVELS = [int(x) for x in os.environ.get("GIGI_PARITY_LEVELS", "0,1").split(",")]
TRIALS = os.environ.get("GIGI_PARITY_TRIALS", "native,pg,pg_fixed").split(",")
RECEIPTS = REPO / "research" / "su2-cayley-20261118"
N = 8
T_END = FL.T_END


def run_engine(level, trial):
    lat = FL.Lattice(level, "pg" if trial == "pg_fixed" else trial)
    met_step = FL.Metric(fixed=(trial == "pg_fixed"))
    met_true = FL.Metric()
    h = T_END / N
    step_times = [0.0] + [(k + 0.5) * h for k in range(N)]
    ref_times = [(k + 1) * h for k in range(N)]
    K = CAY.load_K(level, trial).tocoo()
    with tempfile.TemporaryDirectory(prefix="gigi-flow-") as d:
        d = Path(d)
        K.row.astype("<u4").tofile(d / "r.bin")
        K.col.astype("<u4").tofile(d / "c.bin")
        K.data.astype("<f8").tofile(d / "v.bin")
        inp, out = d / "in.json", d / "out.json"
        inp.write_text(json.dumps({
            "order": 6, "method": "native" if trial == "native" else "pg2", "steps": N, "t_end": T_END,
            "metric_step": [[t, [float(x) for x in met_step(t)]] for t in step_times],
            "metric_ref": [[t, [float(x) for x in met_true(t)]] for t in ref_times],
            "k0": {"rows": str(d / "r.bin"), "cols": str(d / "c.bin"), "vals": str(d / "v.bin")},
            "links": lat.raw["links"], "edge_points": lat.edge_points,
            "cells": [{"vertices": c["vertices"], "edges": c["edges"], "reverse": [bool(r) for r in c["reverse"]],
                       "weight": float(c["weight"])} for c in lat.cells]}))
        subprocess.run([os.environ["GIGI_LINK_FLOW_PROBE"], str(inp), str(out)], check=True, timeout=36000)
        res = json.loads(out.read_text())
        final = np.fromfile(out.with_suffix(".final.bin"), dtype="<f8").reshape(-1, 4)
    return res, final


class Flow(unittest.TestCase):
    def test_engine_flow_matches_s3b(self):
        receipt = {"kind": "YM4: engine LINK_FLOW (Cayley, dense-LU preconditioner) vs S3b receipts", "N": N}
        for level in LEVELS:
            for trial in TRIALS:
                rpath = RECEIPTS / f"L{level}_{trial}_N{N}.json"
                if not rpath.exists():
                    receipt[f"level{level}_{trial}"] = {"skipped": "no research receipt at this level"}
                    print(level, trial, "skipped: no research receipt", flush=True)
                    continue
                ref = json.loads(rpath.read_text())
                ref_final = np.load(RECEIPTS / f"L{level}_{trial}_N{N}.final.npy")
                t = time.time()
                eng, final = run_engine(level, trial)
                self.assertEqual(eng["status"], "ok", eng["status"])
                e_rel = max(abs(a["E"] - b["E"]) / abs(b["E"]) for a, b in zip(eng["rows"], ref["rows"]))
                same_iters = eng["newton_iterations"] == ref["newton_iterations"]
                links_diff = float(np.abs(final - ref_final).max())
                row = {"E_rel_max": e_rel, "E_final_engine": eng["rows"][-1]["E"], "E_final_research": ref["rows"][-1]["E"],
                       "R_final_engine": eng["rows"][-1]["R"], "newton_iterations_identical": same_iters,
                       "engine_iterations": eng["newton_iterations"], "research_iterations": ref["newton_iterations"],
                       "final_links_max_abs_diff": links_diff, "variational": eng["variational"],
                       "engine_seconds": eng["seconds"], "wall_seconds": round(time.time() - t, 1)}
                receipt[f"level{level}_{trial}"] = row
                print(level, trial, row, flush=True)
                self.assertEqual(len(eng["rows"]), len(ref["rows"]))
                self.assertLess(e_rel, 1e-8)
                self.assertTrue(same_iters)
                self.assertLess(links_diff, 1e-8)
        out = os.environ.get("GIGI_PARITY_RECEIPT")
        if out:
            p = Path(out)
            if p.exists():
                raise SystemExit(f"refusing to overwrite {p}")
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_text(json.dumps(receipt, indent=2) + "\n")


if __name__ == "__main__":
    unittest.main()
