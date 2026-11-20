"""YM3 (docs/YANG_MILLS_ENGINE_SPEC.md): the engine's linearised force K0 against the B5 matrices.

The engine builds K by central differences of LINK_FORCE (eps = 1e-6, right-trivialised, only the
cells that read each link recomputed), at the stored native links, Berger g = (3, 1, 1), q6. The
reference is outputs/su2-batched-K/L{level}_{trial}_K.npz (B5, scripts/ricci_su2_batched.py), whose
SHA-256 is checked against research/su2-batched-K-20261119/kbuild_L{level}_{trial}.json first.

Gate: whole-matrix relative Frobenius difference <= 1e-9. Reported: per-column worst, nnz, seconds.

Requires GIGI_LINK_FORCE_PROBE and GIGI_RICCI_DATA_ROOT (holding outputs/ and the frozen inputs).
Optional GIGI_PARITY_LEVELS (default "0,1,2"), GIGI_PARITY_TRIALS (default "native,pg"),
GIGI_PARITY_RECEIPT (refuses to overwrite).
"""
import hashlib
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

import numpy as np
from scipy.sparse import coo_matrix, load_npz

REPO = Path(__file__).resolve().parents[2]
DATA_ROOT = Path(os.environ.get("GIGI_RICCI_DATA_ROOT", REPO))
BERGER = DATA_ROOT / "output/ricci-shape-regular-20261117/berger"
KDIR = DATA_ROOT / "outputs/su2-batched-K"
KREC = DATA_ROOT / "research/su2-batched-K-20261119"
LEVELS = [int(x) for x in os.environ.get("GIGI_PARITY_LEVELS", "0,1,2").split(",")]
TRIALS = os.environ.get("GIGI_PARITY_TRIALS", "native,pg").split(",")
METHOD = {"native": "native", "pg": "pg2"}
TOL = 1e-9


def reference(level, trial):
    rec = json.loads((KREC / f"kbuild_L{level}_{trial}.json").read_text())
    path = KDIR / f"L{level}_{trial}_K.npz"
    h = hashlib.sha256(path.read_bytes()).hexdigest()
    if h != rec["sha256"]:
        raise RuntimeError(f"B5 matrix {path} does not match its receipt")
    return load_npz(path).tocsc(), h


def engine(level, trial):
    payload = json.loads((BERGER / f"level{level}_berger_q6.input.json").read_text())
    raw = json.loads((BERGER / f"level{level}_berger_q6.output.json").read_text())
    with tempfile.TemporaryDirectory(prefix="gigi-K-") as d:
        inp, out = Path(d) / "in.json", Path(d) / "out.json"
        inp.write_text(json.dumps({"metric": [3.0, 1.0, 1.0], "order": 6, "method": METHOD[trial], "rings": 2,
                                   "jacobian_eps": 1e-6, "links": raw["links"],
                                   "cells": [{"vertices": c["vertices"], "edges": c["edges"],
                                              "reverse": [bool(r) for r in c["reverse"]],
                                              "weight": float(c["weight"])} for c in payload["cells"]]}))
        subprocess.run([os.environ["GIGI_LINK_FORCE_PROBE"], str(inp), str(out)], check=True, timeout=36000)
        meta = json.loads(out.read_text())
        rows = np.fromfile(out.with_suffix(".rows.bin"), dtype="<u4")
        cols = np.fromfile(out.with_suffix(".cols.bin"), dtype="<u4")
        vals = np.fromfile(out.with_suffix(".vals.bin"), dtype="<f8")
    K = coo_matrix((vals, (rows, cols)), shape=(meta["n"], meta["n"])).tocsc()
    return K, meta


class Jacobian(unittest.TestCase):
    def test_engine_K_matches_B5(self):
        receipt = {"kind": "YM3: engine K0 (link_force_probe, central differences eps 1e-6) vs B5 matrices",
                   "tolerance": TOL}
        for level in LEVELS:
            for trial in TRIALS:
                Kr, h = reference(level, trial)
                Ke, meta = engine(level, trial)
                diff = Ke - Kr
                rel = float(np.sqrt(diff.multiply(diff).sum()) / np.sqrt(Kr.multiply(Kr).sum()))
                cn = np.sqrt(np.asarray(Kr.multiply(Kr).sum(axis=0))).ravel()
                cd = np.sqrt(np.asarray(diff.multiply(diff).sum(axis=0))).ravel()
                worst_col = float(np.max(cd[cn > 0] / cn[cn > 0]))
                row = {"matrix_rel": rel, "worst_column_rel": worst_col, "nnz_engine": int(meta["nnz"]),
                       "nnz_B5": int(Kr.nnz), "engine_seconds": meta["seconds"], "B5_sha256": h}
                receipt[f"level{level}_{trial}"] = row
                print(level, trial, row, flush=True)
                self.assertLess(rel, TOL)
        out = os.environ.get("GIGI_PARITY_RECEIPT")
        if out:
            p = Path(out)
            if p.exists():
                raise SystemExit(f"refusing to overwrite {p}")
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_text(json.dumps(receipt, indent=2) + "\n")


if __name__ == "__main__":
    unittest.main()
