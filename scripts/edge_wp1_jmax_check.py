"""WP1 side check: does the sector cutoff j <= 12 change the edge or its competitor? (21 November 2026)

At t = 2, y = 0.25 the j <= 12 map finds the competitor (12, 6), i.e. in the top sector allowed. Here the H + GG^*
edge is recomputed with sectors j <= JBIG (same blocks, EE.Blocks.block; same edge procedure as EE.edge) at a few
points, and the per-sector minimum f_j(x) = min_q [lambda_min(B_{j,q}) - lambda_j] at the edge is listed.
Double precision (COMPUTED).
usage: OMP_NUM_THREADS=1 python scripts/edge_wp1_jmax_check.py OUT.json JBIG t:y [t:y ...]
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_exact_edge as EE              # noqa: E402


def per_sector(bl, xv, yv, tv, jbig):
    d = bl.data(xv, yv, tv); out = {}
    for j in range(jbig + 1):
        l = j / 2; lam = EE.lam_tw(xv, yv, tv, j); best = (np.inf, None)
        for q in np.arange(-l - 2, l + 2 + 1e-9, 1.0):
            if abs(abs(q) - (l + 2)) < 1e-9:
                continue
            _, B = bl.block(d, j, q, tv)
            if B.size:
                v = np.linalg.eigvalsh(B)[0] - lam
                if v < best[0]:
                    best = (float(v), float(q))
        out[j] = best
    return out


def main():
    out_path, jbig = Path(sys.argv[1]), int(sys.argv[2])
    pts = [tuple(float(u) for u in a.split(":")) for a in sys.argv[3:]]
    bl = EE.Blocks(); res = {"status": "COMPUTED (double precision)", "JBIG": jbig, "points": []}
    for tv, yv in pts:
        fun = lambda xv, yv_, tv_, jm: min(v for v, _ in per_sector(bl, xv, yv_, tv_, jm).values())   # noqa: E731
        e12 = EE.edge(lambda a, b, c: fun(a, b, c, 12), yv, tv)
        eb = EE.edge(lambda a, b, c: fun(a, b, c, jbig), yv, tv)
        ps = per_sector(bl, eb, yv, tv, jbig) if eb is not None else {}
        arg = min(ps, key=lambda j: ps[j][0]) if ps else None
        rec = {"t": tv, "y": yv, "edge_j<=12": e12, f"edge_j<={jbig}": eb,
               "competitor_at_big_edge": [arg, ps[arg][1]] if arg is not None else None,
               "per_sector_min_at_big_edge": {str(j): ps[j] for j in ps}}
        res["points"].append(rec)
        print({k: v for k, v in rec.items() if k != "per_sector_min_at_big_edge"}, flush=True)
        print("   per-sector minima at the big edge:", {j: round(ps[j][0], 5) for j in ps}, flush=True)
    out_path.write_text(json.dumps(res, indent=1, default=str) + "\n")


if __name__ == "__main__":
    main()
