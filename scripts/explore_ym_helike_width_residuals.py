"""EXPLORATORY (not registered): full-space residual of each reported level versus the basis width alpha, at fixed
cutoff, so the per-level-width question can be judged from saved data rather than an unsaved table.

usage: python scripts/explore_ym_helike_width_residuals.py OUT.json [--cutoff 10] [--couplings 1 2]
"""
import argparse
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import ym_helike_quantum as q  # noqa: E402

ALPHAS = (1.0, 1.35, 1.7, 2.0, 2.3, 2.7)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out")
    ap.add_argument("--cutoff", type=int, default=10)
    ap.add_argument("--couplings", type=float, nargs="+", default=[1.0, 2.0])
    args = ap.parse_args()
    b = q.build_basis(args.cutoff)
    ops = q.build_operators(b)
    rows = []
    for g2 in args.couplings:
        for a in ALPHAS:
            e, v, _ = q.solve_generalized(q.hamiltonian(ops, g2, a), b["S"], "cpu")
            x = q.exterior_diagnostics_scaled(b, ops, e, v, g2, a)
            spins = q.level_spins(e, v, ops["J2"])
            for enc, res in zip(x["enclosures"], x["level_worst_residuals"]):
                k = enc["level"]
                rows.append({"g2": g2, "alpha": a, "cutoff": args.cutoff, "E0": float(e[0]), "level": k,
                             "gap": float(enc["E"] - e[0]), "spins": [c["J"] for c in spins[k]["components"]],
                             "full_residual": res})
            print(g2, a, [(r["level"], r["spins"], round(r["full_residual"], 4)) for r in rows[-3:]], flush=True)
    Path(args.out).write_text(json.dumps({"label": "EXPLORATORY, not a registered measurement",
                                          "alphas": ALPHAS, "rows": rows}, indent=1))


if __name__ == "__main__":
    main()
