"""S2c: numerical range of the linearised SU(2) force in the native mass norm.
Spec: docs/RICCI_SU2_PG_NUMRANGE_SPEC.md. Research script, not an engine verb.

Builds K exactly as S2b (scripts/ricci_su2_pg_stability.py, imported), caches it
under outputs/su2-numrange/ (gitignored), whitens with B = L L^T, and measures
W-left, the numerical radius, a boundary polygon, eta, F1 and step-size bounds.

usage: python scripts/ricci_su2_pg_numrange.py LEVEL TRIAL [RECEIPT.json]
"""
import json
import sys
import time
from pathlib import Path

import numpy as np
from scipy.linalg import cholesky, eig, eigh, solve_triangular

HERE = Path(__file__).resolve().parent
REPO = HERE.parent
sys.path.insert(0, str(HERE))
import ricci_su2_pg_stability as S2B          # noqa: E402
from test_ricci_operator_native import sparse_matrix  # noqa: E402

CACHE = REPO / "outputs" / "su2-numrange"
STAB = REPO / "research" / "su2-pg-stability-20261117"
ANGLES = 180


def build_K(level, trial):
    CACHE.mkdir(parents=True, exist_ok=True)
    path = CACHE / f"L{level}_{trial}_K.npy"
    S = S2B.Setup(level, trial)
    B = sparse_matrix(S.raw).toarray()
    if path.exists():
        return np.load(path), B
    N = 3 * S.n
    K = np.zeros((N, N))
    t1 = time.time()
    for g in range(S.n):
        for k in range(3):
            K[:, 3 * g + k] = S.column(g, k)
        if g % 200 == 0:
            print("columns", g, "/", S.n, round(time.time() - t1, 1), "s", flush=True)
    np.save(path, K)
    return K, B


def match(a, b, scale):
    """max over a of distance to nearest b, relative to scale."""
    worst = 0.0
    for chunk in np.array_split(a, max(1, len(a) // 500)):
        d = np.abs(chunk[:, None] - b[None, :]).min(axis=1)
        worst = max(worst, float(d.max()))
    return worst / scale


def main():
    level, trial = int(sys.argv[1]), sys.argv[2]
    receipt = Path(sys.argv[3]) if len(sys.argv) > 3 else None
    if receipt and receipt.exists():
        raise SystemExit(f"refusing to overwrite {receipt}")
    t0 = time.time()
    K, B = build_K(level, trial)
    print("K ready", round(time.time() - t0, 1), "s", flush=True)
    # C-chol
    L = cholesky(B, lower=True)
    chol_err = float(np.linalg.norm(L @ L.T - B) / np.linalg.norm(B))
    # G = L^-1 K L^-T
    G = solve_triangular(L, K, lower=True)
    G = solve_triangular(L, G.T, lower=True).T
    # C-eig
    lam = eig(G, right=False)
    saved = np.load(STAB / f"L{level}_{trial}.eigs.npy")
    scale = float(np.abs(saved).max())
    c_eig = max(match(saved, lam, scale), match(lam, saved, scale))
    # W-left
    S = 0.5 * (G + G.T)
    s_eigs = eigh(S, eigvals_only=True)
    w_left = float(s_eigs[0])
    # theta sweep: support lines and boundary points
    A = 0.5 * (G - G.T)
    support, points = [], []
    for th in np.linspace(0, np.pi, ANGLES, endpoint=False):
        H = np.cos(th) * S - 1j * np.sin(th) * A        # Re(e^{-i th} G) as Hermitian
        vals, vecs = eigh(H)
        for idx, sgn in ((-1, 1), (0, -1)):
            x = vecs[:, idx]
            z = complex(np.vdot(x, G @ x))
            points.append(z)
            support.append((th if sgn > 0 else th + np.pi, float(vals[idx]) * sgn))
    points = np.array(points)
    w = float(max(v for _, v in support))
    w_num = float(np.abs(points).max())
    # C-W: boundary points inside every supporting half-plane; eigenvalues right of W-left
    gnorm = float(np.linalg.norm(G, 2))
    viol = 0.0
    for th, h in support:
        proj = (points * np.exp(-1j * th)).real
        viol = max(viol, float(proj.max() - h))
    eig_left_violation = float(max(0.0, w_left - lam.real.min()))
    # eta, F1
    comm = G @ G.T - G.T @ G
    eta = float(np.abs(eigh(comm, eigvals_only=True)).max() / gnorm ** 2)
    F1 = gnorm / w
    # step sizes
    m = -w_left
    implicit = (1.0 / m) if m > 0 else None

    def ok(h):
        return float(np.abs(1 - h * points).max()) <= np.exp(h * max(m, 0.0))
    lo, hi = 0.0, 1.0
    while ok(hi) and hi < 1e6:
        lo, hi = hi, hi * 2
    for _ in range(80):
        mid = 0.5 * (lo + hi)
        lo, hi = (mid, hi) if ok(mid) else (lo, mid)
    out = {"level": level, "trial": trial, "dof": int(G.shape[0]),
           "C_chol": chol_err, "C_eig": c_eig,
           "C_W_support_violation": viol, "C_W_eig_left_violation": eig_left_violation,
           "W_left": w_left, "S_negative_count": int((s_eigs < 0).sum()),
           "min_real_eigenvalue": float(lam.real.min()),
           "numerical_radius": w, "numerical_radius_from_points": w_num,
           "spectral_norm": gnorm, "F1": F1, "eta": eta,
           "implicit_euler_h_max": implicit, "explicit_euler_h": lo,
           "boundary_points": [[p.real, p.imag] for p in points],
           "seconds": round(time.time() - t0, 1)}
    print(json.dumps({k: v for k, v in out.items() if k != "boundary_points"}), flush=True)
    if receipt:
        receipt.write_text(json.dumps(out, indent=1) + "\n")


if __name__ == "__main__":
    main()
