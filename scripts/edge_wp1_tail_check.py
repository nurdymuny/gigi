"""WP1 side check: sectors beyond j = 12 at every grid point of the competitor map (21 November 2026).

At large j the low-|m| states carry 4(l(l+1) - m^2) ~ 4l, so the only blocks that can stay near the twist are the
near-top charges |q| in {l+1, l, l-1, l-2} (heuristic; the full set of q was checked for j <= 40 at four points in
research/edge-theorem-20261121/wp1_jmax_check.json). For each grid point (t, y) of wp1_competitor_map.json with
edge x*_12 (sectors j <= 12):
  tail(x) = min over 13 <= j <= JT and those near-top q of [lambda_min(B_{j,q}(x)) - lambda_j(x)];
  report tail(x*_12) and its argmin (j, q). tail(x*_12) < 0 means a sector j > 12 undercuts the twist before x*_12,
  i.e. the j <= 12 edge is not the edge.
For points where it is negative, the edge with sectors j <= J (all q for j <= 12; near-top q for j > 12) is
recomputed for J in JLIST at up to MAX_CONV of them, to show how it moves with the cutoff.
Double precision (COMPUTED).
usage: OMP_NUM_THREADS=1 python scripts/edge_wp1_tail_check.py MAP.json OUT.json [JT]
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_exact_edge as EE              # noqa: E402

JLIST = [12, 24, 48, 96, 192]
MAX_CONV = 6          # edge-vs-cutoff only at up to this many undercut points (spread over the list)


def tail(bl, xv, yv, tv, j0, j1, step=1):
    d = bl.data(xv, yv, tv); best = (np.inf, None)
    for j in range(j0, j1 + 1, step):
        l = j / 2; lam = EE.lam_tw(xv, yv, tv, j)
        for qa in (l + 1, l, l - 1, l - 2):
            if qa < 0:
                continue
            for q in {qa, -qa}:
                _, B = bl.block(d, j, q, tv)
                if B.size:
                    v = float(np.linalg.eigvalsh(B)[0] - lam)
                    if v < best[0]:
                        best = (v, (j, float(q)))
    return best


def edge_with_tail(bl, yv, tv, J):
    def f(xv, yv_, tv_):
        v = bl.phi_A(xv, yv_, tv_)
        if J > 12:
            v = min(v, tail(bl, xv, yv_, tv_, 13, J)[0])
        return v
    return EE.edge(f, yv, tv, n=81)


def main():
    mapd = json.loads(Path(sys.argv[1]).read_text()); out_path = Path(sys.argv[2])
    JT = int(sys.argv[3]) if len(sys.argv) > 3 else 200
    bl = EE.Blocks(); t0 = time.time(); rows = []
    for r in mapd["grid"]:
        if r["edge_x"] is None:
            continue
        v, arg = tail(bl, r["edge_x"], r["y"], r["t"], 13, JT)
        rec = {"t": r["t"], "y": r["y"], "edge_x_j<=12": r["edge_x"], "competitor_j<=12": r["competitor"],
               f"tail_min_13..{JT}_at_edge": v, "tail_argmin": arg}
        rows.append(rec)
        print(f"t={r['t']} y={r['y']} tail={v:.5f} at {arg} [{time.time() - t0:.0f}s]", flush=True)
    bad = [rec for rec in rows if rec[f"tail_min_13..{JT}_at_edge"] < 0]
    conv = []
    pick = bad if len(bad) <= MAX_CONV else [bad[int(round(i * (len(bad) - 1) / (MAX_CONV - 1)))] for i in range(MAX_CONV)]
    for rec in pick:
        seq = {}
        for J in JLIST:
            seq[J] = edge_with_tail(bl, rec["y"], rec["t"], J)
            print(f"  t={rec['t']} y={rec['y']} J={J}: edge {seq[J]}", flush=True)
        conv.append({"t": rec["t"], "y": rec["y"], "edge_vs_cutoff_J": seq})
    out = {"status": "COMPUTED (double precision); near-top-charge heuristic for j > 12", "JT": JT, "rows": rows,
           "points_where_j>12_undercuts": [[rec["t"], rec["y"]] for rec in bad], "edge_vs_cutoff": conv,
           "runtime_s": time.time() - t0}
    out_path.write_text(json.dumps(out, indent=1, default=str) + "\n")


if __name__ == "__main__":
    main()
