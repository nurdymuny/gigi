"""Fold-force: the Abelian PG force on folded stencils. Spec: docs/RING_FOLD_FORCE_SPEC.md.
Research script, not an engine verb.

The fit is ricci_pg_tworing.fit_cell_rings(rings=2) unchanged; only the neighbourhood it
reads is restricted to each cell's chosen set. Build, data, BW05 and S3 replicate
scripts/ricci_pg_tworing_gate.py line for line.

usage:
  python scripts/ring_fold_force.py controls
  python scripts/ring_fold_force.py run MODE OUTDIR [--levels 0,1,2,3] [--s3-levels 0,1]
    MODE  fold | random
"""
import argparse
import gzip
import json
import sys
import time
from pathlib import Path

import numpy as np
from scipy.linalg import eig, eigh

HERE = Path(__file__).resolve().parent
sys.path[:0] = [str(HERE), str(HERE.parent / "e2e/probes")]
import ricci_pg_curvature as pg            # noqa: E402
import ricci_pg_tworing as tw              # noqa: E402
import ring_fold as rf                     # noqa: E402  (also applies GIGI_RICCI_DATA_ROOT to pg)
from ricci_consistency_math import solve_rhs  # noqa: E402
from ricci_shape_evidence import numerical_gate  # noqa: E402

ROOT = HERE.parent
OUT = ROOT / "research" / "ring-fold-force-20261118"
_ORIG_NB = tw.neighbourhood_rings
CHOSEN = {}          # cell index -> set of allowed cell indices (for the level being built)


def restricted_neighbourhood(cells, faces, ci, rings=2):
    nb = _ORIG_NB(cells, faces, ci, rings)
    allowed = CHOSEN.get(ci)
    if allowed is None or rings != 2:
        return nb
    return {k: v for k, v in nb.items() if k in allowed}


def install(chosen):
    CHOSEN.clear()
    CHOSEN.update(chosen)
    tw.neighbourhood_rings = restricted_neighbourhood
    tw.use(2)


def greedy_sets(level, mode):
    """Chosen sets per cell, by the fold spec's greedy rule (or random of the same size)."""
    cache = OUT / f"chosen_L{level}_{mode}.json"
    if cache.exists():
        return {int(k): set(v) for k, v in json.loads(cache.read_text()).items()}
    cells = pg.load_cells(level)["cells"]
    faces = pg.face_index(cells)
    out, ks = {}, {}
    for ci in range(len(cells)):
        nb1 = _ORIG_NB(cells, faces, ci, 1)
        nb2 = _ORIG_NB(cells, faces, ci, 2)
        set1 = set(nb1)
        a2, _ = rf.amp(cells, faces, ci, nb2, set(nb2))
        a1, _ = rf.amp(cells, faces, ci, nb2, set1)
        cand = [k for k in nb2 if k not in set1]
        chosen, cur = set(set1), a1
        while cur > rf.TARGET * a2:
            cur, best = min(((rf.amp(cells, faces, ci, nb2, chosen | {c})[0], c) for c in cand if c not in chosen))
            chosen.add(best)
        k = len(chosen) - len(set1)
        ks[ci] = k
        if mode == "random":
            rng = np.random.default_rng(ci + 1000 * level)
            pick = set(rng.choice(cand, size=k, replace=False).tolist()) if k else set()
            chosen = set1 | pick
        out[ci] = chosen
    OUT.mkdir(parents=True, exist_ok=True)
    cache.write_text(json.dumps({str(k): sorted(int(x) for x in v) for k, v in out.items()}))
    (OUT / f"k_L{level}_{mode}.json").write_text(json.dumps(ks))
    return out


def controls():
    res = {}
    level = 0
    cells = pg.load_cells(level)["cells"]
    faces = pg.face_index(cells)
    L = pg.load_level(level, 6)
    # reference operators, unpatched
    tw.neighbourhood_rings = _ORIG_NB
    tw.use(2)
    K2 = pg.build(level, 6, "reconstruction", data=L)["operator"].toarray()
    tw.use(1)
    K1 = pg.build(level, 6, "reconstruction", data=L)["operator"].toarray()
    # C-full / C-one through the patch
    install({ci: set(_ORIG_NB(cells, faces, ci, 2)) for ci in range(len(cells))})
    Kf = pg.build(level, 6, "reconstruction", data=L)["operator"].toarray()
    res["C_full"] = float(np.abs(Kf - K2).max())
    install({ci: set(_ORIG_NB(cells, faces, ci, 1)) for ci in range(len(cells))})
    Ko = pg.build(level, 6, "reconstruction", data=L)["operator"].toarray()
    res["C_one"] = float(np.abs(Ko - K1).max())
    res["C_one_scale"] = float(np.abs(K1).max())
    # C-chosen: greedy k matches the fold receipt
    tw.neighbourhood_rings = _ORIG_NB
    receipt = json.loads(gzip.open(ROOT / "research/ring-fold-20261118/fold.json.gz", "rt").read())
    want = {r["cell"]: r["k"] for r in receipt["levels"][0]["cells"]}
    greedy_sets(level, "fold")
    got = {int(k): v for k, v in json.loads((OUT / f"k_L{level}_fold.json").read_text()).items()}
    res["C_chosen_mismatches_level0"] = int(sum(want[c] != got[c] for c in want))
    print(json.dumps(res, indent=1))


def run(mode, outdir, levels, s3_levels):
    outdir.mkdir(parents=True, exist_ok=True)
    out = outdir / "diagnostic.json"
    if out.exists():
        raise SystemExit("refusing to overwrite " + str(out))
    hmax = json.loads((rf._DATA / "output/ricci-variational-20261117/abelian/diagnostic.json").read_text())["cases"]
    rec = {"schema": "ring_fold_force_bw05_s3_v1", "mode": mode, "cases": {}, "s3": {}, "stencils": {}}
    rows, hs, quad, lin = [], [], [], []
    for level in levels:
        chosen = greedy_sets(level, mode)
        rec["stencils"][str(level)] = {"median_cells": float(np.median([len(v) for v in chosen.values()]))}
        install(chosen)
        vel = {}
        for order in (4, 6):
            t = time.perf_counter()
            L = pg.load_level(level, order)
            r = pg.build(level, order, "reconstruction", data=L)
            M = L["mass"]
            vol = L["volume"]
            norm = lambda v: float(np.sqrt(max(0., v @ (M @ v)) / vol))   # noqa: E731
            corr = solve_rhs(M, -r["force"] - M @ r["velocity"])
            row = {k: r[k] for k in ("full_mass_error", "relative_error", "curvature_relative_error", "max_condition", "edges")}
            row.update(level=level, order=order, h_max=hmax[f"level{level}_q6"]["h_max"],
                       linear_correction_mass_estimate=norm(corr), seconds=round(time.perf_counter() - t, 1))
            rec["cases"][f"level{level}_q{order}"] = row
            vel[order] = (r["velocity"], M, vol)
            print("CASE", json.dumps({k: row[k] for k in ("level", "order", "relative_error", "seconds")}), flush=True)
            out.write_text(json.dumps(rec, indent=1))
        (v6, M6, vol6), (v4, _, _) = vel[6], vel[4]
        dv = v6 - v4
        quad.append(float(np.sqrt(max(0., dv @ (M6 @ dv)) / vol6)))
        lin.append(max(rec["cases"][f"level{level}_q{o}"]["linear_correction_mass_estimate"] for o in (4, 6)))
        hs.append(rec["cases"][f"level{level}_q6"]["h_max"])
        rows.append(rec["cases"][f"level{level}_q6"])
    if len(levels) == 4:
        rec["numerical_gate"] = numerical_gate(rows, hs, quad, lin)
        print("GATE", json.dumps(rec["numerical_gate"]), flush=True)
    out.write_text(json.dumps(rec, indent=1))
    for level in s3_levels:
        t = time.perf_counter()
        install(greedy_sets(level, mode))
        L = pg.load_level(level, 6)
        K = pg.build(level, 6, "reconstruction", data=L)["operator"].toarray()
        M = L["mass"].toarray()
        Kw = L["stiffness"].toarray()
        lw = eigh(Kw, M, eigvals_only=True)
        gauge = int((abs(lw) < 1).sum())
        lp = eig(K, M, right=False)
        small = abs(lp) < 1
        phys = lp[~small]
        s3 = {"level": level, "edges": len(M), "gauge_dimension_whitney": gauge,
              "small_eigenvalues_pg": int(small.sum()),
              "min_real_part_physical": float(phys.real.min()),
              "max_abs_imag_physical": float(abs(phys.imag).max()),
              "negative_real_part_count": int((phys.real < 0).sum()),
              "stable": bool(small.sum() == gauge and phys.real.min() > 0),
              "seconds": round(time.perf_counter() - t, 1)}
        rec["s3"][str(level)] = s3
        print("S3", json.dumps(s3), flush=True)
        out.write_text(json.dumps(rec, indent=1))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("cmd")
    ap.add_argument("mode", nargs="?")
    ap.add_argument("outdir", nargs="?", type=Path)
    ap.add_argument("--levels", default="0,1,2,3")
    ap.add_argument("--s3-levels", default="0,1")
    a = ap.parse_args()
    if a.cmd == "controls":
        controls()
    else:
        run(a.mode, a.outdir, [int(x) for x in a.levels.split(",")], [int(x) for x in a.s3_levels.split(",") if x])


if __name__ == "__main__":
    main()
