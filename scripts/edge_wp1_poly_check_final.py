"""WP1: re-check every grid edge of the final competitor map against the stored block polynomials (21 November 2026).

Reads research/edge-theorem-20261121/wp1_competitor_map.json (edges, competitors) and
research/edge-theorem-20261121/wp1_block_polynomials.json (factors of det(B_{j,q} - lambda_j), exact, from
scripts/edge_wp1_block_polynomials.py). For each grid point, with (j, |q|) its competitor: every factor of that
block's polynomial, the mpmath root (60 digits) started at the edge and the scale-free residual |F| / sum|terms| there.
Also: for the (4, 0) block, the factor identified as the edge factor is compared with P2 of
research/su2-floor-family-20261119/edge_polynomial.txt (degrees (7, 8, 16)).
Double / 60-digit numerics (COMPUTED).
usage: python scripts/edge_wp1_poly_check_final.py OUT.json
"""
import json
import sys
from pathlib import Path

import sympy as sp

sys.path.insert(0, str(Path(__file__).resolve().parent))
import edge_wp1_block_polynomials as W        # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
D = ROOT / "research/edge-theorem-20261121"


def main():
    mp_ = json.loads((D / "wp1_competitor_map.json").read_text())
    pb = json.loads((D / "wp1_block_polynomials.json").read_text())["blocks"]
    facs = {}
    for key, rec in pb.items():
        j = int(key.split(",")[0][2:]); q = abs(float(key.split(",")[1][2:]))
        if (j, q) in facs:
            continue
        facs[(j, q)] = [sp.Poly(sp.sympify(f["expr"], locals={"x": W.x, "y": W.y, "s": W.s}), W.x, W.y, W.s)
                        for f in rec["factors"]]
    rows = []; worst = {}
    for r in mp_["grid"]:
        K = (r["competitor"][0], abs(r["competitor"][1])); xe, yv, tv = r["edge_x"], r["y"], r["t"]; sv = tv ** 0.5
        per = []
        for i, pol in enumerate(facs[K]):
            if pol.degree(W.x) == 0:
                per.append({"factor": i, "x_free": True}); continue
            nr = W.newton_root(pol, yv, sv, xe)
            per.append({"factor": i, "degrees": [pol.degree(W.x), pol.degree(W.y), pol.degree(W.s)],
                        "root_minus_edge": None if nr is None else nr - xe,
                        "scale_free_residual": W.scale_free(pol.terms(), xe, yv, sv)})
        rows.append({"t": tv, "y": yv, "edge_x": xe, "competitor": list(K), "factors": per})
        best = min((p for p in per if not p.get("x_free")), key=lambda p: p["scale_free_residual"])
        w = worst.setdefault(str(K), {"points": 0, "edge_factor_degrees": best["degrees"], "max_|root-edge|": 0.0,
                                      "max_residual_edge_factor": 0.0, "min_residual_other_factors": None})
        w["points"] += 1
        w["max_|root-edge|"] = max(w["max_|root-edge|"], abs(best["root_minus_edge"]))
        w["max_residual_edge_factor"] = max(w["max_residual_edge_factor"], best["scale_free_residual"])
        others = [p["scale_free_residual"] for p in per if not p.get("x_free") and p is not best]
        if others:
            m = min(others)
            w["min_residual_other_factors"] = m if w["min_residual_other_factors"] is None else min(w["min_residual_other_factors"], m)
        if w["edge_factor_degrees"] != best["degrees"]:
            w["edge_factor_changes"] = True
    # (4, 0): compare the edge factor with P2 from edge_polynomial.txt
    txt = (ROOT / "research/su2-floor-family-20261119/edge_polynomial.txt").read_text().splitlines()
    k = [i for i, ln in enumerate(txt) if ln.strip().startswith("degree (x,y,s) = 7, 8, 16")][0]
    P2 = sp.Poly(sp.sympify(txt[k + 1].strip(), locals={"x": W.x, "y": W.y, "s": W.s}), W.x, W.y, W.s)
    F = [f for f in facs[(4, 0.0)] if [f.degree(W.x), f.degree(W.y), f.degree(W.s)] == [7, 8, 16]][0]
    q_, r_ = sp.div(F, P2)
    out = {"status": "COMPUTED", "per_block_summary": worst, "rows": rows,
           "edge_factor_(4,0)_equals_const_times_P2_of_edge_polynomial.txt": {"quotient": str(q_.as_expr()),
                                                                             "remainder_is_zero": r_.is_zero}}
    Path(sys.argv[1]).write_text(json.dumps(out, indent=1, default=str) + "\n")
    print(json.dumps(worst, indent=1)); print(out["edge_factor_(4,0)_equals_const_times_P2_of_edge_polynomial.txt"])


if __name__ == "__main__":
    main()
