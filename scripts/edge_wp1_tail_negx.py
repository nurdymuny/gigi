"""WP1 side check: at grid points whose j <= 12 edge is positive, is the twist still below every near-top block of
every sector 13 <= j <= 2000 for x slightly negative? (21 November 2026)

Uses edge_wp1_tail_check.tail (near-top charges |q| in {l+1, l, l-1, l-2}); j = 13..400 all, 410..2000 step 10.
Also the j <= 12 margin phi_A at the same x. Double precision (COMPUTED).
usage: OMP_NUM_THREADS=1 python scripts/edge_wp1_tail_negx.py MAP.json OUT.json
"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_exact_edge as EE              # noqa: E402
from edge_wp1_tail_check import tail          # noqa: E402

XS = [-0.01, -0.001]


def main():
    mapd = json.loads(Path(sys.argv[1]).read_text()); bl = EE.Blocks(); rows = []
    for r in mapd["grid"]:
        if r["edge_x"] is None or r["edge_x"] <= 0:
            continue
        rec = {"t": r["t"], "y": r["y"], "edge_x_j<=12": r["edge_x"], "at_x": {}}
        for xv in XS:
            a = tail(bl, xv, r["y"], r["t"], 13, 400); b = tail(bl, xv, r["y"], r["t"], 410, 2000, step=10)
            best = a if a[0] <= b[0] else b
            rec["at_x"][str(xv)] = {"phi_A_j<=12": float(bl.phi_A(xv, r["y"], r["t"])), "tail_min": best[0], "tail_argmin": best[1]}
        rows.append(rec); print(rec, flush=True)
    worst = min(min(v["tail_min"], v["phi_A_j<=12"]) for rec in rows for v in rec["at_x"].values()) if rows else None
    Path(sys.argv[2]).write_text(json.dumps({"status": "COMPUTED (double precision); near-top-charge heuristic for j > 12",
                                             "x_values": XS, "rows": rows, "smallest_margin_over_all": worst}, indent=1,
                                            default=str) + "\n")
    print("smallest margin:", worst)


if __name__ == "__main__":
    main()
