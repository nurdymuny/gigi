"""Fold-Lambda: the smallest stable piece of rim. Spec: docs/RING_FOLD_SPEC.md.
Research script, not an engine verb.

For each cell, start from the one-ring stencil and greedily add the second-ring
cell that most lowers the fit amplification ||R_K||_2, until it is within 1.25x of
the full two-ring value. Removal: a random subset of the same size.

usage: python scripts/ring_fold.py LEVELS [RECEIPT.json]     LEVELS e.g. 0,1,2,3
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import ricci_pg_curvature as pg        # noqa: E402
import ricci_pg_tworing as tw          # noqa: E402
import os                              # noqa: E402
_DATA = Path(os.environ.get("GIGI_RICCI_DATA_ROOT", HERE.parent))
pg.BASE = _DATA / "output/ricci-shape-regular-20261117/berger"
pg.DIAG = _DATA / "output/ricci-consistency-20261117/diagnostic"

TARGET = 1.25


def rows_for(cells, faces, ci, nb, order):
    """flux rows in fit_cell_rings' order: own faces, then the rest by `order`."""
    to_orth, _, _ = pg.chart(nb[ci])
    seen, own, rest = set(), [], []
    for nj in [ci] + [k for k in order if k != ci]:
        O = to_orth(nb[nj])
        for f in pg.FACES:
            key = pg.face_key(cells[nj], f)
            if key in seen:
                continue
            seen.add(key)
            (own if nj == ci else rest).append(pg.flux_row(O[list(f)]))
    return np.array(own), np.array(rest)


def fmap_of(A, R):
    """fit_cell_rings' equality-constrained least-squares map (same formulas)."""
    Ap = np.linalg.pinv(A, rcond=pg.RCOND)
    _, sa, vt = np.linalg.svd(A)
    ra = int((sa > pg.RCOND * sa[0]).sum())
    N = vt[ra:].T
    RN = R @ N
    RNp = np.linalg.pinv(RN, rcond=pg.RCOND)
    return np.hstack([Ap - N @ RNp @ R @ Ap, N @ RNp])


def amp(cells, faces, ci, nb2, chosen):
    order = [k for k in nb2 if k in chosen]          # keep nb2's insertion order
    A, R = rows_for(cells, faces, ci, nb2, order)
    return float(np.linalg.norm(fmap_of(A, R), 2)), len(A) + len(R)


def run_level(level):
    cells = pg.load_cells(level)["cells"]
    faces = pg.face_index(cells)
    zero = np.zeros(int(max(max(c["edges"]) for c in cells)) + 1)
    rows = []
    c_fit = 0.0
    for ci in range(len(cells)):
        nb1 = tw.neighbourhood_rings(cells, faces, ci, 1)
        nb2 = tw.neighbourhood_rings(cells, faces, ci, 2)
        set1, set2 = set(nb1), set(nb2)
        a1, f1 = amp(cells, faces, ci, nb2, set1)
        a2, f2 = amp(cells, faces, ci, nb2, set2)
        # C-fit: reproduce the published maps
        for rings, a in ((1, a1), (2, a2)):
            ref = float(np.linalg.norm(tw.fit_cell_rings(cells, zero, faces, ci, rings=rings)["map"], 2))
            c_fit = max(c_fit, abs(a - ref) / ref)
        cand = [k for k in nb2 if k not in set1]
        chosen, cur, path = set(set1), a1, []
        while cur > TARGET * a2:
            best = min(((amp(cells, faces, ci, nb2, chosen | {c})[0], c) for c in cand if c not in chosen))
            cur = best[0]
            chosen.add(best[1])
            path.append(round(cur, 4))
        k = len(chosen) - len(set1)
        _, fk = amp(cells, faces, ci, nb2, chosen)
        rng = np.random.default_rng(ci + 1000 * level)
        pick = set(rng.choice(cand, size=k, replace=False).tolist()) if k else set()
        ar, _ = amp(cells, faces, ci, nb2, set1 | pick)
        rows.append({"cell": ci, "a1": a1, "a2": a2, "a_fold": cur, "k": k, "candidates": len(cand),
                     "faces_one": f1, "faces_two": f2, "faces_fold": fk,
                     "a_random": ar, "random_meets": bool(ar <= TARGET * a2), "path": path})
    k = np.array([r["k"] for r in rows]); C = np.array([r["candidates"] for r in rows])
    ff = np.array([r["faces_fold"] for r in rows]); f2 = np.array([r["faces_two"] for r in rows])
    summary = {"level": level, "cells": len(rows), "C_fit": c_fit,
               "median_k": float(np.median(k)), "max_k": int(k.max()), "median_candidates": float(np.median(C)),
               "saving_cells": 1 - float(np.median(k)) / float(np.median(C)),
               "median_faces_fold": float(np.median(ff)), "median_faces_two": float(np.median(f2)),
               "saving_faces": 1 - float(np.median(ff - np.array([r["faces_one"] for r in rows])))
                               / float(np.median(f2 - np.array([r["faces_one"] for r in rows]))),
               "median_a1": float(np.median([r["a1"] for r in rows])),
               "median_a2": float(np.median([r["a2"] for r in rows])),
               "median_a_fold": float(np.median([r["a_fold"] for r in rows])),
               "random_meets_fraction": float(np.mean([r["random_meets"] for r in rows])),
               "k_histogram": {int(v): int((k == v).sum()) for v in np.unique(k)}}
    return summary, rows


def main():
    levels = [int(x) for x in sys.argv[1].split(",")]
    receipt = Path(sys.argv[2]) if len(sys.argv) > 2 else None
    if receipt and receipt.exists():
        raise SystemExit(f"refusing to overwrite {receipt}")
    out = {"kind": "Fold-Lambda research script (not an engine run)", "target": TARGET, "levels": []}
    for level in levels:
        t = time.time()
        s, rows = run_level(level)
        s["seconds"] = round(time.time() - t, 1)
        print(json.dumps(s), flush=True)
        out["levels"].append({"summary": s, "cells": rows})
        if receipt:
            receipt.write_text(json.dumps(out) + "\n")


if __name__ == "__main__":
    main()
