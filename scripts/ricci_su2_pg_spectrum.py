"""S2d: spectrum and numerical-range left edge of the linearised SU(2) force, any level.
Spec: docs/RICCI_SU2_PG_SPECTRUM_N3_SPEC.md. Research script, not an engine verb.

K is assembled in parallel by S3b's builder (scripts/ricci_su2_pg_flow_cayley.py buildK, cached
under outputs/su2-cayley/), from S2b's Setup.column. Computes only what the S2b/S2c gates use:
the eigenvalues of G = L^-1 K L^-T and W-left = min eig of its symmetric part.

usage: python scripts/ricci_su2_pg_spectrum.py LEVEL TRIAL [WORKERS] [RECEIPT.json]
  TRIAL  native | pg | pg_onering
"""
import json
import sys
import time
from pathlib import Path

import numpy as np
from scipy.linalg import cholesky, eig, eigh, solve_triangular

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import ricci_su2_pg_flow_cayley as CAY                   # noqa: E402
import ricci_su2_pg_stability as S2B                      # noqa: E402
from ricci_su2_pg_force import compose                    # noqa: E402
from test_ricci_operator_native import sparse_matrix      # noqa: E402

EPS = S2B.EPS_COL


def c_col(S):
    """S2b's C-col: one column from two full gradients against the cell-local column."""
    grad = S.gradient(S.links)
    g0 = int(np.argmax(np.linalg.norm(grad, axis=1)))
    full = np.zeros_like(grad)
    for sign in (1, -1):
        pert = S.links.copy()
        pert[g0] = compose(S.links[g0], S2B.exp_tau(sign * EPS, 0))
        full += sign * S.gradient(pert)
    full = (full / (2 * EPS)).ravel()
    local = S.column(g0, 0)
    return float(np.linalg.norm(full - local) / max(np.linalg.norm(full), 1e-300))


def main():
    level, trial = int(sys.argv[1]), sys.argv[2]
    workers = int(sys.argv[3]) if len(sys.argv) > 3 else 12
    receipt = Path(sys.argv[4]) if len(sys.argv) > 4 else None
    if receipt and receipt.exists():
        raise SystemExit(f"refusing to overwrite {receipt}")
    t0 = time.time()
    path = CAY.SPARSE_K / f"L{level}_{CAY.k_trial(trial)}_K.npz"
    if not path.exists():
        CAY.build_K(level, trial, workers)
    t_build = time.time() - t0
    K = CAY.load_K(level, trial).toarray()
    S = S2B.Setup(level, trial)
    colerr = c_col(S)
    B = sparse_matrix(S.raw).toarray()
    L = cholesky(B, lower=True)
    chol = float(np.linalg.norm(L @ L.T - B) / np.linalg.norm(B))
    G = solve_triangular(L, K, lower=True)
    G = solve_triangular(L, G.T, lower=True).T
    del K
    t1 = time.time()
    lam = eig(G, right=False, overwrite_a=False, check_finite=False)
    t_eig = time.time() - t1
    Sym = 0.5 * (G + G.T)
    gnorm = float(np.linalg.norm(G, 2)) if G.shape[0] <= 4000 else float(np.abs(lam).max())
    s_low = eigh(Sym, eigvals_only=True, subset_by_index=[0, 9], check_finite=False)
    s_neg = int(np.sum(eigh(Sym, eigvals_only=True, check_finite=False) < 0))
    w_left = float(s_low[0])
    re = np.sort(lam.real)
    out = {"level": level, "trial": trial, "dof": int(len(lam)),
           "C_col": colerr, "C_chol": chol,
           "C_W_eig_left_violation": float(max(0.0, w_left - 1e-9 * gnorm - re[0])),
           "rough_real_below_minus10": int(np.sum(lam.real < -10)),
           "small_modulus_below_1": int(np.sum(np.abs(lam) < 1)),
           "negative_real": int(np.sum(lam.real < 0)),
           "min_real": float(re[0]), "five_most_negative_real": [float(x) for x in re[:5]],
           "max_abs_imag": float(np.abs(lam.imag).max()),
           "W_left": w_left, "S_negative_count": s_neg,
           "norm_estimate": gnorm,
           "seconds": {"build": round(t_build, 1), "eig": round(t_eig, 1), "total": round(time.time() - t0, 1)}}
    print(json.dumps(out), flush=True)
    if receipt:
        receipt.parent.mkdir(parents=True, exist_ok=True)
        np.save(receipt.with_suffix(".eigs.npy"), lam)
        receipt.write_text(json.dumps(out, indent=1) + "\n")


if __name__ == "__main__":
    main()
