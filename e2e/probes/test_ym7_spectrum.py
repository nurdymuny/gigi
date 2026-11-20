"""YM7 (docs/YANG_MILLS_ENGINE_SPEC.md): the engine's linearised SU(2) force against the exact spectrum.
Reported, not gated.

The engine builds K0 (YM3) and the native tangent mass B at the stored Berger links, g = (3, 1, 1), q6.
The generalized eigenvalues of (K0, B) are compared with the exact values of the linearised Yang-Mills
operator at the Berger field on the Poincare sphere (Reading the Curve, Table 9 and Proposition 4):

  spin-0 sector (exact closed forms): -14/3 (x2), (29 - sqrt 1345)/3 (x2, 40% gauge), 16 - 2 sqrt 73,
                                       8 (pure gauge), (29 + sqrt 1345)/3 (x2), 16 + 2 sqrt 73;
  bottom of each higher sector on the gauge-orthogonal slice, (j(j+2) - 14)/3: spin 6 154/3,
  spin 10 142, spin 12 610/3; and the full-Hessian spin-6 bottom 51.333 (x2).

Pairing is by nearest value only. At n = 1 that pairing misread 16 - 2 sqrt 73 in the paper's first
Table 9 (the Mac's eigenvector check found the true mode at -0.674 +- 0.80i); eigenvector-confirmed
pairings are in the paper's Table 9 receipts. Here the columns are value-matched, and say so.

For each exact value: the real part of the nearest engine eigenvalue (n = 1, 2: dense; n = 3: shift-invert
near the value), and the same for the research spectra (S2b / S2d receipts) as a parity column.

Requires GIGI_LINK_FORCE_PROBE (with jacobian_eps and mass_only) and GIGI_RICCI_DATA_ROOT.
Optional GIGI_PARITY_LEVELS (default "0,1,2"), GIGI_PARITY_TRIALS (default "native,pg"), GIGI_PARITY_RECEIPT.
"""
import json
import math
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

import numpy as np
from scipy.linalg import eig
from scipy.sparse import coo_matrix
from scipy.sparse.linalg import eigs

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[1]
DATA_ROOT = Path(os.environ.get("GIGI_RICCI_DATA_ROOT", REPO))
BERGER = DATA_ROOT / "output/ricci-shape-regular-20261117/berger"
LEVELS = [int(x) for x in os.environ.get("GIGI_PARITY_LEVELS", "0,1,2").split(",")]
TRIALS = os.environ.get("GIGI_PARITY_TRIALS", "native,pg").split(",")
METHOD = {"native": "native", "pg": "pg2"}
R1345, R73 = math.sqrt(1345), math.sqrt(73)
EXACT = [("-14/3", -14 / 3), ("(29-sqrt1345)/3 [40% gauge]", (29 - R1345) / 3), ("16-2sqrt73", 16 - 2 * R73),
         ("8 [pure gauge]", 8.0), ("(29+sqrt1345)/3", (29 + R1345) / 3), ("16+2sqrt73", 16 + 2 * R73),
         ("spin-6 full bottom 51.333", 51.3333), ("spin-6 slice bottom 154/3", 154 / 3),
         ("spin-10 slice bottom 142", 142.0), ("spin-12 slice bottom 610/3", 610 / 3)]
RESEARCH = {0: REPO / "research/su2-pg-stability-20261117/L0_{t}.eigs.npy",
            1: REPO / "research/su2-pg-stability-20261117/L1_{t}.eigs.npy",
            2: REPO / "research/su2-spectrum-n3-20261119/L2_{t}.eigs.npy"}


def probe(level, trial, mode):
    payload = json.loads((BERGER / f"level{level}_berger_q6.input.json").read_text())
    raw = json.loads((BERGER / f"level{level}_berger_q6.output.json").read_text())
    body = {"metric": [3.0, 1.0, 1.0], "order": 6, "method": METHOD[trial], "rings": 2, "links": raw["links"],
            "cells": [{"vertices": c["vertices"], "edges": c["edges"], "reverse": [bool(r) for r in c["reverse"]],
                       "weight": float(c["weight"])} for c in payload["cells"]]}
    body["jacobian_eps" if mode == "K" else "mass_only"] = 1e-6 if mode == "K" else True
    with tempfile.TemporaryDirectory(prefix="gigi-ym7-") as d:
        inp, out = Path(d) / "in.json", Path(d) / "out.json"
        inp.write_text(json.dumps(body))
        subprocess.run([os.environ["GIGI_LINK_FORCE_PROBE"], str(inp), str(out)], check=True, timeout=36000)
        meta = json.loads(out.read_text())
        r = np.fromfile(out.with_suffix(".rows.bin"), dtype="<u4")
        c = np.fromfile(out.with_suffix(".cols.bin"), dtype="<u4")
        v = np.fromfile(out.with_suffix(".vals.bin"), dtype="<f8")
    return coo_matrix((v, (r, c)), shape=(meta["n"], meta["n"])).tocsc()


def nearest(lam, x):
    z = lam[np.argmin(np.abs(lam - x))]
    return [float(z.real), float(z.imag)]


class Spectrum(unittest.TestCase):
    def test_engine_spectrum_against_exact(self):
        rec = {"kind": "YM7 (reported, not gated): engine (K0, B) spectrum vs exact sector values",
               "exact": {n: v for n, v in EXACT}}
        for level in LEVELS:
            for trial in TRIALS:
                K, B = probe(level, trial, "K"), probe(level, trial, "B")
                if level <= 1:
                    lam = eig(K.toarray(), B.toarray(), right=False)
                    near = {n: nearest(lam, v) for n, v in EXACT}
                else:
                    near = {}
                    for n, v in EXACT:
                        w = eigs(K, k=4, M=B, sigma=v, which="LM", return_eigenvectors=False)
                        near[n] = nearest(w, v)
                    lam = None
                res = np.load(str(RESEARCH[level]).format(t=trial))
                row = {name: {"exact": v, "engine": near[name], "research": nearest(res, v),
                              "engine_minus_exact": near[name][0] - v,
                              "engine_minus_research": near[name][0] - nearest(res, v)[0]}
                       for name, v in EXACT}
                rec[f"level{level}_{trial}"] = row
                print(level, trial, {k: round(v["engine"][0], 4) for k, v in row.items()}, flush=True)
        out = os.environ.get("GIGI_PARITY_RECEIPT")
        if out:
            p = Path(out)
            if p.exists():
                raise SystemExit(f"refusing to overwrite {p}")
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_text(json.dumps(rec, indent=2) + "\n")


if __name__ == "__main__":
    unittest.main()
