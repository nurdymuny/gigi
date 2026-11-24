"""EXPLORATORY (not registered): where is the spin-2 level, which carries Helike's directions, the lowest excitation
of the sphere matrix model, and what are the low levels made of?

The registered survey and its convergence amendment measured three couplings. Their converged-looking energies show
the scalar level lowest at g^2 = 0.5 and 1 and the spin-2 quintet lowest at g^2 = 2, with a spin-4 multiplet nearby.
That motivates a comparison with the edge paper's statement "by enough, but not too much"; it does not establish
the same mechanism or a finite window. This script samples the coupling and describes low levels, so that a
confirmatory study can be pre-registered with held-out couplings. Nothing here is a registered measurement.

For each coupling: alpha* minimises the ground energy at cutoff 10 over ALPHAS (the registered rule, with a wider
grid because g^2 = 2 selected the old grid's edge); then cutoffs 10 and 12 at alpha*. Per complete level:
  gap, spin decomposition, H2 and H0 weights (physical q-observables);
  size <tr(q q^T)> and orientation <det q> (the second classical vacuum sits at q = I/g, det q > 0);
  weight by polynomial degree shell (the Wick-ordered basis is orthogonal across degrees). The highest retained
  shell is a diagnostic, not the omitted full-space residual or the discarded probe-response fraction.

usage: python scripts/explore_ym_helike_edge.py OUT.json [--cutoffs 10 12] [--device cpu]
"""
import argparse
import json
import sys
import time
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import ym_helike_quantum as q  # noqa: E402

COUPLINGS = (0.5, 0.75, 1.0, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0, 4.0, 6.0)
ALPHAS = (0.8, 1.0, 1.2, 1.35, 1.5, 1.7, 2.0, 2.3, 2.7, 3.2, 4.0, 5.0)


def describe(b, ops, e, v, alpha, nlev=8):
    spins = q.level_spins(e, v, ops["J2"])
    probes = q.scaled_probes(ops, alpha)
    h2 = q.probe_report_with_spin(e, v, probes["tensor"], ops["J2"], "H2")
    h0 = q.probe_report_with_spin(e, v, [probes["scalar"]], ops["J2"], "H0")
    degrees = b["degrees"]
    shells = sorted(set(degrees.tolist()))
    S = b["S"]
    out = []
    for k, sp in enumerate(spins[:nlev]):
        idx = sp["indices"]
        V = v[:, idx]
        r = len(idx)
        size = float(np.trace(V.T @ ops["trace"] @ V)) / r / alpha            # <tr q q^T>, q = x/sqrt(alpha)
        orient = float(np.trace(V.T @ ops["D"] @ V)) / r / alpha ** 1.5       # <det q>
        w = {}
        for d in shells:
            m = degrees == d
            w[str(d)] = float(np.trace(V[m].T @ S[np.ix_(m, m)] @ V[m])) / r
        row = {"level": k, "gap": float(np.mean(e[idx]) - e[0]),
               "spins": [(c["J"], c["copies"]) for c in sp["components"]], "multiplicity": r,
               "size_tr_qqT": size, "det_q": orient, "degree_weights": w,
               "top_shell_weight": w[str(shells[-1])]}
        if k > 0:
            row["H2_weight"] = h2["levels"][k - 1]["weight"]
            row["H0_weight"] = h0["levels"][k - 1]["weight"]
        out.append(row)
    return out


def lowest_by_spin(levels):
    best = {}
    for lv in levels[1:]:
        if len(lv["spins"]) == 1:
            J = lv["spins"][0][0]
            best.setdefault(J, lv["gap"])
    return best


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out")
    ap.add_argument("--cutoffs", type=int, nargs="+", default=[10, 12])
    ap.add_argument("--device", default="cpu", choices=("cpu", "cuda"))
    ap.add_argument("--couplings", type=float, nargs="+", default=list(COUPLINGS))
    args = ap.parse_args()
    t0 = time.time()
    cache = {}

    def get(N):
        if N not in cache:
            cache.clear()
            b = q.build_basis(N) if N <= 12 else q.build_basis_extended(N)
            cache[N] = (b, q.build_operators(b))
        return cache[N]

    b10, ops10 = get(10)
    choice = {}
    for g2 in args.couplings:
        table = {a: float(q.solve_generalized(q.hamiltonian(ops10, g2, a), b10["S"], args.device)[0][0]) for a in ALPHAS}
        a_star = min(table, key=table.get)
        choice[g2] = {"alpha_star": a_star, "E0_by_alpha": {str(k): v for k, v in table.items()},
                      "interior": a_star not in (ALPHAS[0], ALPHAS[-1])}
        print(f"g2={g2}: alpha*={a_star}", flush=True)
    results = {}
    for N in args.cutoffs:
        b, ops = get(N)
        for g2 in args.couplings:
            a = choice[g2]["alpha_star"]
            e, v, _ = q.solve_generalized(q.hamiltonian(ops, g2, a), b["S"], args.device)
            levels = describe(b, ops, e, v, a)
            best = lowest_by_spin(levels)
            results[f"N={N},g2={g2}"] = {"cutoff": N, "g2": g2, "alpha": a, "E0": float(e[0]), "levels": levels,
                                         "lowest_gap_by_spin": {str(k): v for k, v in best.items()},
                                         "lowest_level_spins": levels[1]["spins"]}
            print(f"N={N} g2={g2:<5} alpha={a:<4} lowest={levels[1]['spins']} "
                  + " ".join(f"J{J}:{best[J]:.5f}" for J in sorted(best)), flush=True)
    summary = []
    for g2 in args.couplings:
        row = {"g2": g2, "alpha": choice[g2]["alpha_star"]}
        for N in args.cutoffs:
            best = results[f"N={N},g2={g2}"]["lowest_gap_by_spin"]
            row[f"N{N}"] = best
            if "0" in best and "2" in best:
                row[f"N{N}_tensor_minus_scalar"] = best["2"] - best["0"]
            if "2" in best and "4" in best:
                row[f"N{N}_J4_minus_tensor"] = best["4"] - best["2"]
        summary.append(row)
    out = {"label": "EXPLORATORY, not a registered measurement", "couplings": args.couplings,
           "alpha_grid": ALPHAS, "alpha_selection": {str(k): v for k, v in choice.items()},
           "summary": summary, "results": results, "seconds": round(time.time() - t0, 1)}
    Path(args.out).parent.mkdir(parents=True, exist_ok=True)
    Path(args.out).write_text(json.dumps(out, indent=1, default=lambda o: o.item() if hasattr(o, "item") else str(o)))


if __name__ == "__main__":
    main()
