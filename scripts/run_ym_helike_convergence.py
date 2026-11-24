"""Amendment 2 runner (draft, NOT RUN): the registered Helike quantum probe with a basis width alpha chosen by a frozen
ground-state rule, extended cutoffs, spin labels with exact selection-rule zeros, and residual enclosures.

The Hamiltonian, probes, couplings and acceptance thresholds are the registered ones
(docs/research/helike-quantum-probe-preregistration-2026-11-23.md). What changes, per the draft amendment
(docs/research/helike-quantum-convergence-amendment-2026-11-23.md):
  1. alpha: for each coupling, alpha* = argmin over ALPHAS of the ground energy at cutoff N_SELECT. Only E0 is read to
     choose it; no excited state, probe or gap enters the choice. The whole E0(alpha) table is published.
  2. cutoffs: CUTOFFS at alpha*, plus the registered alpha = 1 at the same cutoffs for comparison.
  3. each level carries its spatial spin; a channel whose spin rule forbids the lowest level reports
     SELECTION_RULE_ZERO; Weinstein intervals (rigorous) and Kato-Temple intervals (conditional) are reported.

usage: python scripts/run_ym_helike_convergence.py OUT_DIR [--device cuda|cpu] [--max-cutoff 16|18|20]
The registered runner, scripts/run_ym_helike_quantum.py, is unchanged.
"""
import argparse
import hashlib
import json
import os
import sys
import time
import uuid
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import ym_helike_quantum as q  # noqa: E402

COUPLINGS = (1.0, 0.5, 2.0)
ALPHAS = (0.8, 0.9, 1.0, 1.1, 1.2, 1.35, 1.5, 1.7, 2.0)
N_SELECT = 10
CUTOFFS = (8, 10, 12, 14, 16)
GPU_BUDGET_BYTES = 4 * 1024**3


def atomic(path, obj):
    tmp = path.with_name(path.name + f".tmp{os.getpid()}")
    tmp.write_text(json.dumps(obj, indent=1, default=lambda o: o.item() if hasattr(o, "item") else str(o)))
    os.replace(tmp, path)


def source_hashes():
    return {p.name: hashlib.sha256(p.read_bytes()).hexdigest()
            for p in (HERE / "ym_helike_quantum.py", Path(__file__))}


def basis_for(cutoff):
    return q.build_basis(cutoff) if cutoff <= 12 else q.build_basis_extended(cutoff)


def case(b, ops, g2, alpha, device):
    h = q.hamiltonian(ops, g2, alpha)
    if device == "cuda" and 12 * h.nbytes > GPU_BUDGET_BYTES:
        raise MemoryError(f"dimension {len(h)} exceeds the GPU budget; refusing rather than falling back")
    e, v, diag = q.solve_generalized(h, b["S"], device)
    out = {"cutoff": b["cutoff"], "alpha": alpha, "dimension": len(e), "E0": float(e[0]), "E1": float(e[1]),
           "absolute_gap": float(e[1] - e[0]), "solver": diag,
           "spins": q.level_spins(e, v, ops["J2"])[:12],
           "H2": q.probe_report_with_spin(e, v, ops["tensor"], ops["J2"], "H2"),
           "H0": q.probe_report_with_spin(e, v, [ops["scalar"]], ops["J2"], "H0")}
    out["exterior"] = q.exterior_diagnostics_scaled(b, ops, e, v, g2, alpha)
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out")
    ap.add_argument("--device", default="cuda", choices=("cuda", "cpu"))
    ap.add_argument("--max-cutoff", type=int, default=16, choices=(12, 14, 16, 18, 20))
    args = ap.parse_args()
    run_id = time.strftime("%Y%m%dT%H%M%SZ", time.gmtime()) + "-" + uuid.uuid4().hex[:8]
    folder = Path(args.out) / f"convergence-{run_id}"
    folder.mkdir(parents=True)
    receipt_path = folder / "receipt.json"
    receipt = {"status": "RUNNING", "run_id": run_id, "amendment": "draft Amendment 2", "device": args.device,
               "source_hashes": source_hashes(), "alphas": ALPHAS, "n_select": N_SELECT}
    atomic(receipt_path, receipt)
    try:
        cutoffs = tuple(c for c in CUTOFFS + (18, 20) if c <= args.max_cutoff)
        cache = {}

        def get(cutoff):
            if cutoff not in cache:
                t0 = time.time()
                b = basis_for(cutoff)
                cache.clear()                       # keep memory bounded: one cutoff at a time
                cache[cutoff] = (b, q.build_operators(b), round(time.time() - t0, 1))
            return cache[cutoff]

        # 1. the frozen alpha rule: ground energy only
        b, ops, _ = get(N_SELECT)
        choice = {}
        for g2 in COUPLINGS:
            table = {str(a): float(q.solve_generalized(q.hamiltonian(ops, g2, a), b["S"], args.device)[0][0])
                     for a in ALPHAS}
            best = min(table, key=table.get)
            choice[str(g2)] = {"alpha_star": float(best), "E0_by_alpha": table,
                               "interior": best not in (str(ALPHAS[0]), str(ALPHAS[-1]))}
            print(f"g2={g2}: alpha* = {best}", flush=True)
        receipt["alpha_selection"] = choice
        atomic(receipt_path, receipt)
        # 2. the cutoff sequence at alpha* and at the registered alpha = 1
        results = {}
        for g2 in COUPLINGS:
            astar = choice[str(g2)]["alpha_star"]
            for alpha in sorted({1.0, astar}):
                hist = []
                for cutoff in cutoffs:
                    b, ops, build_s = get(cutoff)
                    t0 = time.time()
                    c = case(b, ops, g2, alpha, args.device)
                    c["seconds"], c["build_seconds"] = round(time.time() - t0, 1), build_s
                    hist.append(c)
                    print(f"g2={g2} alpha={alpha} N={cutoff}: gap={c['absolute_gap']:.10f} "
                          f"worst residual={max(c['exterior']['level_worst_residuals']):.3e}", flush=True)
                    np.savez_compressed(folder / f"N{cutoff}-g2-{g2:g}-alpha-{alpha:g}.npz", spins=json.dumps(c["spins"]))
                results[f"g2={g2:g},alpha={alpha:g}"] = {"history": hist, "converged": bool(q.convergence(hist))}
                receipt["results"] = results
                atomic(receipt_path, receipt)
        receipt["status"] = "COMPLETE"
        receipt["scientific_status"] = {k: ("numerically_converged_model_estimate" if v["converged"]
                                            else "finite_cutoff_observations_only") for k, v in results.items()}
        atomic(receipt_path, receipt)
        print(json.dumps(receipt["scientific_status"], indent=1))
    except BaseException as exc:  # noqa: BLE001
        receipt["status"] = "FAILED"
        receipt["error"] = f"{type(exc).__name__}: {exc}"[:500]
        atomic(receipt_path, receipt)
        raise


if __name__ == "__main__":
    main()
