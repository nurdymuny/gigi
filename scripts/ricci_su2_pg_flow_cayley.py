"""S3b: the SU(2) force in time with the Cayley (Crank-Nicolson) step.
Spec: docs/RICCI_SU2_PG_FLOW_CAYLEY_SPEC.md. Research script, not an engine verb.

usage:
  python scripts/ricci_su2_pg_flow_cayley.py controls LEVEL
  python scripts/ricci_su2_pg_flow_cayley.py buildK LEVEL TRIAL [WORKERS]
  python scripts/ricci_su2_pg_flow_cayley.py run LEVEL TRIAL N [RECEIPT.json]
    TRIAL  pg | native | pg_fixed        N  steps over [0, 0.02]
"""
import json
import sys
import time
from multiprocessing import Pool
from pathlib import Path

import numpy as np
from scipy.sparse import csc_matrix, load_npz, save_npz
from scipy.sparse.linalg import splu

HERE = Path(__file__).resolve().parent
REPO = HERE.parent
sys.path.insert(0, str(HERE))
import ricci_su2_pg_flow as FL                     # noqa: E402
from ricci_su2_pg_flow import compose, inverse, qlog, qexp, T_END  # noqa: E402

DENSE_K = REPO / "outputs" / "su2-numrange"
SPARSE_K = REPO / "outputs" / "su2-cayley"
NEWTON_TOL, NEWTON_MAX = 1e-8, 40   # Amendment 1: the force is only computable to ~3.5e-10


def cay(v):
    """Cayley map of pure quaternions v (..., 3) -> unit quaternions (..., 4)."""
    s = np.sum(v * v, axis=-1, keepdims=True) / 4.0
    return np.concatenate([(1 - s), v], axis=-1) / (1 + s)


# ---------------------------------------------------------------- K0, the linearised gradient at t = 0
_S = None


def _init(level, trial):
    global _S
    import ricci_su2_pg_stability as S2B
    _S = S2B.Setup(level, trial)


def _columns(gs):
    rows, cols, vals = [], [], []
    for g in gs:
        for k in range(3):
            c = _S.column(g, k)
            nz = np.nonzero(c)[0]
            rows.append(nz); cols.append(np.full(len(nz), 3 * g + k)); vals.append(c[nz])
    return np.concatenate(rows), np.concatenate(cols), np.concatenate(vals)


def k_trial(trial):
    return "pg" if trial == "pg_fixed" else trial


def load_K(level, trial):
    kt = k_trial(trial)
    dense = DENSE_K / f"L{level}_{kt}_K.npy"
    if dense.exists():
        return csc_matrix(np.load(dense))
    return load_npz(SPARSE_K / f"L{level}_{kt}_K.npz")


def build_K(level, trial, workers):
    kt = k_trial(trial)
    SPARSE_K.mkdir(parents=True, exist_ok=True)
    path = SPARSE_K / f"L{level}_{kt}_K.npz"
    if path.exists():
        raise SystemExit(f"refusing to overwrite {path}")
    t0 = time.time()
    n = len(json.loads((FL.F.BERGER / f"level{level}_berger_q6.output.json").read_text())["links"])
    chunks = [list(range(i, n, workers * 4)) for i in range(workers * 4)]
    with Pool(workers, initializer=_init, initargs=(level, kt)) as pool:
        parts = pool.map(_columns, chunks)
    r = np.concatenate([p[0] for p in parts]); c = np.concatenate([p[1] for p in parts])
    v = np.concatenate([p[2] for p in parts])
    K = csc_matrix((v, (r, c)), shape=(3 * n, 3 * n))
    save_npz(path, K)
    print(json.dumps({"level": level, "trial": kt, "dof": 3 * n, "nnz": int(K.nnz),
                      "seconds": round(time.time() - t0, 1)}), flush=True)


# ---------------------------------------------------------------- controls
def controls(level):
    out = {}
    rng = np.random.default_rng(1)
    v = rng.normal(size=(1000, 3))
    out["C_cay_unit"] = float(np.abs(np.linalg.norm(cay(v), axis=1) - 1).max())
    ratios = []
    for s in (1e-1, 5e-2, 2.5e-2, 1.25e-2):
        w = s * v / np.linalg.norm(v, axis=1, keepdims=True)
        ratios.append(float(np.linalg.norm(cay(w) - qexp(w), axis=1).max() / s ** 3))
    out["C_cay_third_order_ratios"] = ratios
    lat = FL.Lattice(level, "native")
    met = FL.Metric()
    eps = 1e-6

    def xi_exact(t):
        L = lat.exact_links(met(t))
        Li = inverse(L)
        return (qlog(compose(Li, lat.exact_links(met(t + eps))))
                - qlog(compose(Li, lat.exact_links(met(t - eps))))) / (2 * eps)
    L0, Lend = lat.exact_links(met(0.0)), lat.exact_links(met(T_END))
    errs = {}
    for N in (4, 8, 16):
        U, h = L0.copy(), T_END / N
        for k in range(N):
            U = compose(U, cay(h * xi_exact((k + 0.5) * h)))
        errs[N] = float(np.abs(qlog(compose(inverse(Lend), U))).max())
    out["C_order_errors"] = errs
    out["C_order_orders"] = [float(np.log2(errs[4] / errs[8])), float(np.log2(errs[8] / errs[16]))]
    out["C_order_pass"] = all(1.8 <= o <= 2.2 for o in out["C_order_orders"])
    return out


# ---------------------------------------------------------------- the run
def run(level, trial, N):
    t0 = time.time()
    lat = FL.Lattice(level, trial)
    met = FL.Metric(fixed=(trial == "pg_fixed"))
    true_met = FL.Metric()
    h = T_END / N
    U = np.asarray(lat.raw["links"], float).copy()
    L0 = U.copy()
    B0 = lat.mass(L0, lat.metric_pieces(met(0.0))).tocsc()
    A = splu((B0 + 0.5 * h * load_K(level, trial)).tocsc())
    t_setup = time.time() - t0
    samples = sorted(set([N // 4, N // 2, 3 * N // 4, N]))
    rows, iters, evals = [], [], 0
    status, worst_inc = "ok", 0.0
    Om = np.zeros((lat.n, 3))
    t_eval = 0.0
    for k in range(N):
        tm = (k + 0.5) * h
        g = met(tm)
        ok = False
        for it in range(1, NEWTON_MAX + 1):
            te = time.time()
            F = Om - h * lat.velocity(compose(U, cay(0.5 * Om)), g)
            t_eval += time.time() - te
            evals += 1
            d = A.solve(-(B0 @ F.ravel())).reshape(lat.n, 3)
            Om = Om + d
            inc = float(np.linalg.norm(d) / max(np.linalg.norm(Om), 1e-300))
            if not np.isfinite(inc):
                break
            if inc <= NEWTON_TOL:
                ok = True
                break
        iters.append(it)
        worst_inc = max(worst_inc, inc)
        if not ok:
            status = f"failed at step {k + 1}: Newton increment {inc:.2e} after {it} iterations"
            break
        U = compose(U, cay(Om))
        U /= np.linalg.norm(U, axis=1, keepdims=True)
        if (k + 1) in samples:
            t = (k + 1) * h
            gt = true_met(t)
            L = lat.exact_links(gt)
            E, D = lat.error(U, L, gt), lat.error(L0, L, gt)
            fa_U, fa_L, fa_0 = lat.face_angles(U), lat.face_angles(L), lat.face_angles(L0)
            rows.append({"t": t, "E": E, "D": D, "R": E / D,
                         "gauge_invariant_R": float(np.linalg.norm(fa_U - fa_L) / np.linalg.norm(fa_0 - fa_L)),
                         "newton_iterations": iters[-1], "seconds": round(time.time() - t0, 1)})
            print(json.dumps(rows[-1]), flush=True)
    return {"level": level, "trial": trial, "stepper": "cayley_midpoint", "N": N, "h": h,
            "status": status, "rows": rows,
            "R_final": rows[-1]["R"] if rows and abs(rows[-1]["t"] - T_END) < 1e-15 else None,
            "E_final": rows[-1]["E"] if rows and abs(rows[-1]["t"] - T_END) < 1e-15 else None,
            "newton_iterations": iters, "newton_max_iterations": max(iters) if iters else None,
            "newton_worst_final_increment": worst_inc, "force_evaluations": evals,
            "seconds_per_evaluation": round(t_eval / max(evals, 1), 2),
            "setup_seconds": round(t_setup, 1), "seconds": round(time.time() - t0, 1), "_final": U}


def main():
    mode = sys.argv[1]
    if mode == "controls":
        print(json.dumps(controls(int(sys.argv[2])), indent=1))
        return
    if mode == "buildK":
        build_K(int(sys.argv[2]), sys.argv[3], int(sys.argv[4]) if len(sys.argv) > 4 else 12)
        return
    level, trial, N = int(sys.argv[2]), sys.argv[3], int(sys.argv[4])
    receipt = Path(sys.argv[5]) if len(sys.argv) > 5 else None
    if receipt and receipt.exists():
        raise SystemExit(f"refusing to overwrite {receipt}")
    out = run(level, trial, N)
    final = out.pop("_final")
    print(json.dumps({k: v for k, v in out.items() if k not in ("rows", "newton_iterations")}), flush=True)
    if receipt:
        receipt.parent.mkdir(parents=True, exist_ok=True)
        np.save(receipt.with_suffix(".final.npy"), final)
        receipt.write_text(json.dumps(out, indent=1) + "\n")


if __name__ == "__main__":
    main()
