"""B6: stability and numerical range of the SU(2) forces from the exact-derivative K (B5).
Spec: docs/RICCI_SU2_EXACT_STABILITY_SPEC.md. Research script, not an engine verb.

usage:
  python scripts/ricci_su2_exact_stability.py dense  LEVEL TRIAL [RECEIPT.json]   # LEVEL 0, 1, 2
  python scripts/ricci_su2_exact_stability.py sparse LEVEL TRIAL [RECEIPT.json]   # LEVEL 2, 3
  python scripts/ricci_su2_exact_stability.py eps    LEVEL TRIAL [RECEIPT.json]   # LEVEL 0, 1
  TRIAL  native | pg | pg_onering
LEVEL 0, 1, 2, 3 is n = 1, 2, 3, 5.
"""
import hashlib
import json
import sys
import time
from pathlib import Path

import numpy as np
from scipy.linalg import cholesky, eig, eigh, solve_triangular
from scipy.sparse import load_npz
from scipy.sparse.linalg import ArpackNoConvergence, eigs, eigsh, splu, svds

HERE = Path(__file__).resolve().parent
REPO = HERE.parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(REPO / "e2e" / "probes"))
import ricci_su2_pg_force as F                          # noqa: E402
from test_ricci_operator_native import sparse_matrix    # noqa: E402

KDIR = REPO / "outputs" / "su2-batched-K"
KREC = REPO / "research" / "su2-batched-K-20261119"
STAB = REPO / "research" / "su2-pg-stability-20261117"
NUMR = REPO / "research" / "su2-numrange-20261118"
ANGLES = 180
ORDER = 6


def load_K(level, trial):
    """C-hash: the B5 matrix, checked against its kbuild receipt."""
    rec = json.loads((KREC / f"kbuild_L{level}_{trial}.json").read_text())
    path = REPO / rec["file"]
    h = hashlib.sha256(path.read_bytes()).hexdigest()
    if h != rec["sha256"]:
        raise SystemExit(f"C-hash FAILED for {path}: {h} != {rec['sha256']}")
    return load_npz(path).tocsr(), h


def load_B(level):
    raw = json.loads((F.BERGER / f"level{level}_berger_q{ORDER}.output.json").read_text())
    return sparse_matrix(raw).tocsc()


def match(a, b, scale):
    """max over a of the distance to the nearest b, relative to scale."""
    worst = 0.0
    for chunk in np.array_split(a, max(1, len(a) // 500)):
        d = np.abs(chunk[:, None] - b[None, :]).min(axis=1)
        worst = max(worst, float(d.max()))
    return worst / scale


def stab_counts(lam):
    """S2b's quantities, from the generalized spectrum of (K, B)."""
    phys = lam[np.abs(lam) >= 1]
    return {"small_modulus_below_1": int(np.sum(np.abs(lam) < 1)),
            "rough_real_below_minus10": int(np.sum(lam.real < -10)),
            "negative_real_physical": int(np.sum(phys.real < 0)),
            "min_real": float(lam.real.min()), "min_real_physical": float(phys.real.min()),
            "max_abs_imag": float(np.abs(lam.imag).max()),
            "nearest_unit_circle": sorted(float(abs(abs(x) - 1)) for x in lam)[:3]}


def whiten(Kd, Bd):
    L = cholesky(Bd, lower=True)
    chol = float(np.linalg.norm(L @ L.T - Bd) / np.linalg.norm(Bd))
    G = solve_triangular(L, Kd, lower=True)
    G = solve_triangular(L, G.T, lower=True).T
    return G, L, chol


def gates(level, trial, w_left, rough=None):
    """Verdicts are formed later against native (G-W needs the native left edge)."""
    return {"W_left_at_least_minus10": bool(w_left >= -10),
            "rough_none": None if rough is None else bool(rough == 0)}


# ---------------------------------------------------------------- dense (n = 1, 2, 3)
def dense(level, trial):
    t0 = time.time()
    K, h = load_K(level, trial)
    Bs = load_B(level)
    Kd, Bd = K.toarray(), Bs.toarray()
    G, L, chol = whiten(Kd, Bd)
    lam = eig(G, right=False)
    t_eig = time.time() - t0
    S = 0.5 * (G + G.T)
    s_eigs = eigh(S, eigvals_only=True)
    w_left = float(s_eigs[0])
    A = 0.5 * (G - G.T)
    support, points, sweep_check = [], [], []
    thetas = np.linspace(0, np.pi, ANGLES, endpoint=False)
    if level <= 1:                                   # exactly S2c
        for th in thetas:
            H = np.cos(th) * S - 1j * np.sin(th) * A
            vals, vecs = eigh(H)
            for idx, sgn in ((-1, 1), (0, -1)):
                x = vecs[:, idx]
                points.append(complex(np.vdot(x, G @ x)))
                support.append((th if sgn > 0 else th + np.pi, float(vals[idx]) * sgn))
    else:                                            # n = 3: sparse generalized sweep
        Ks = 0.5 * (K + K.T)
        Ka = 0.5 * (K - K.T)
        Blu = splu(Bs)
        Minv = _as_op(Blu, Bs.shape[0])
        check_idx = set(range(0, 2 * ANGLES, 2 * ANGLES // 6))
        for j, th in enumerate(np.concatenate([thetas, thetas + np.pi])):
            Hk = (np.cos(th) * Ks - 1j * np.sin(th) * Ka).tocsc()
            vals, vecs = eigsh(Hk, k=1, M=Bs.astype(complex), Minv=Minv, which="LA",
                               tol=1e-10, maxiter=10_000)
            v = vecs[:, 0]
            z = complex(np.vdot(v, K @ v) / np.vdot(v, Bs @ v))
            points.append(z)
            support.append((float(th), float(vals[0])))
            if j in check_idx:                         # C-sweep
                Hd = np.cos(th) * S - 1j * np.sin(th) * A
                dense_max = float(eigh(Hd, eigvals_only=True, subset_by_index=[len(S) - 1, len(S) - 1])[0])
                sweep_check.append({"theta": float(th), "sparse": float(vals[0]), "dense": dense_max,
                                    "rel": abs(float(vals[0]) - dense_max) / abs(dense_max)})
    points = np.array(points)
    w = float(max(v for _, v in support))
    viol = 0.0
    for th, hv in support:
        viol = max(viol, float((points * np.exp(-1j * th)).real.max() - hv))
    eig_left_violation = float(max(0.0, w_left - lam.real.min()))
    if level <= 1:
        gnorm = float(np.linalg.norm(G, 2))
        comm = G @ G.T - G.T @ G
        eta = float(np.abs(eigh(comm, eigvals_only=True)).max() / gnorm ** 2)
    else:
        gnorm = float(svds(G, k=1, return_singular_vectors=False, tol=1e-10)[0])
        comm = G @ G.T - G.T @ G
        eta = float(abs(eigsh(comm, k=1, which="LM", tol=1e-10, return_eigenvectors=False)[0]) / gnorm ** 2)
    m = -w_left

    def ok(hh):
        return float(np.abs(1 - hh * points).max()) <= np.exp(hh * max(m, 0.0))
    lo, hi = 0.0, 1.0
    while ok(hi) and hi < 1e6:
        lo, hi = hi, hi * 2
    for _ in range(80):
        mid = 0.5 * (lo + hi)
        lo, hi = (mid, hi) if ok(mid) else (lo, mid)
    out = {"level": level, "trial": trial, "dof": int(G.shape[0]), "K_sha256": h,
           "C_chol": chol, "C_W_support_violation": viol, "C_W_eig_left_violation": eig_left_violation,
           "C_W_eig_tolerance": 1e-9 * gnorm,
           "C_sweep": sweep_check or None,
           **stab_counts(lam),
           "W_left": w_left, "S_negative_count": int((s_eigs < 0).sum()),
           "S_below_minus10_count": int((s_eigs < -10).sum()),
           "S_below_minus20_count": int((s_eigs < -20).sum()),
           "numerical_radius": w, "spectral_norm": gnorm, "F1": gnorm / w, "eta": eta,
           "implicit_euler_h_max": (1.0 / m) if m > 0 else None, "explicit_euler_h": lo,
           "eig_seconds": round(t_eig, 1), "seconds": round(time.time() - t0, 1),
           "kind": "B6 dense (research script, not an engine run)"}
    out.update(gates(level, trial, w_left, out["rough_real_below_minus10"]))
    if level <= 1:                                     # comparison with S2b / S2c
        saved = np.load(STAB / f"L{level}_{trial}.eigs.npy")
        old_b = json.loads((STAB / f"L{level}_{trial}.json").read_text())
        old_c = json.loads((NUMR / f"L{level}_{trial}.json").read_text())
        # generalized eigenvalues of (K, B) equal the eigenvalues of G
        out["vs_S2b_eigs"] = max(match(saved, lam, float(np.abs(saved).max())),
                                 match(lam, saved, float(np.abs(saved).max())))
        out["S2b_old"] = {k: old_b[k] for k in ("small_modulus_below_1", "rough_real_below_minus10",
                                                "negative_real_physical", "min_real_physical", "max_abs_imag")}
        out["S2c_old"] = {k: old_c[k] for k in ("W_left", "S_negative_count", "min_real_eigenvalue",
                                                "numerical_radius", "F1", "eta")}
    out["_eigs"] = lam
    out["_points"] = [[p.real, p.imag] for p in points]
    return out


def _as_op(lu, n):
    from scipy.sparse.linalg import LinearOperator
    return LinearOperator((n, n), matvec=lambda x: lu.solve(np.asarray(x, dtype=complex).real) +
                          1j * lu.solve(np.asarray(x, dtype=complex).imag), dtype=complex)


# ---------------------------------------------------------------- sparse (n = 3, 5)
def inertia(A, B, sigma):
    """Number of generalized eigenvalues of (A, B) below sigma (Sylvester), A, B symmetric."""
    M = (A - sigma * B).tocsc()
    t = time.time()
    lu = splu(M, permc_spec="MMD_AT_PLUS_A", diag_pivot_thresh=0.0,
              options={"SymmetricMode": True})
    symmetric_perm = bool(np.array_equal(lu.perm_r, lu.perm_c))
    d = lu.U.diagonal()
    return {"sigma": sigma, "below": int(np.sum(d < 0)), "zero_pivots": int(np.sum(d == 0)),
            "min_abs_pivot": float(np.abs(d).min()), "symmetric_permutation": symmetric_perm,
            "nnz_LU": int(lu.L.nnz + lu.U.nnz), "seconds": round(time.time() - t, 1)}


def rel_resid(A, B, mu, x):
    r = A @ x - mu * (B @ x)
    anorm = float(abs(A).sum(axis=0).max())          # 1-norm, an upper bound on the 2-norm
    xb = float(np.sqrt(abs(np.vdot(x, B @ x))))
    return float(np.linalg.norm(r) / (anorm * xb)), float(np.linalg.norm(r))


def sparse(level, trial):
    t0 = time.time()
    K, h = load_K(level, trial)
    B = load_B(level)
    A = (0.5 * (K + K.T)).tocsc()
    out = {"level": level, "trial": trial, "dof": int(K.shape[0]), "K_sha256": h,
           "kind": "B6 sparse (research script, not an engine run)"}
    out["inertia"] = []
    for sigma in (-10.0, 0.0, -20.0):
        try:
            out["inertia"].append(inertia(A, B, sigma))
        except Exception as e:                       # noqa: BLE001
            out["inertia"].append({"sigma": sigma, "failed": repr(e)})
        print(json.dumps(out["inertia"][-1]), flush=True)
    below20 = next((i.get("below") for i in out["inertia"] if i["sigma"] == -20.0), None)
    # value: shift-invert at sigma = -20
    t = time.time()
    try:
        vals, vecs = eigsh(A, k=6, M=B, sigma=-20.0, which="LM", tol=1e-10)
        order = np.argsort(vals)
        vals, vecs = vals[order], vecs[:, order]
        out["shift_invert_values"] = [float(v) for v in vals]
        out["shift_invert_residuals"] = [rel_resid(A, B, vals[i], vecs[:, i])[0] for i in range(len(vals))]
        out["W_left"] = float(vals[0]) if below20 == 0 else None
        out["W_left_certified_by_inertia"] = below20 == 0
    except Exception as e:                           # noqa: BLE001
        out["shift_invert_failed"] = repr(e)
        out["W_left"] = None
    out["shift_invert_seconds"] = round(time.time() - t, 1)
    # witness of failure: a Rayleigh quotient below -10
    below10 = next((i.get("below") for i in out["inertia"] if i["sigma"] == -10.0), None)
    if below10 is None or below10 > 0:
        t = time.time()
        try:
            v, x = eigsh(A, k=1, M=B, which="SA", tol=1e-8, maxiter=20_000)
            x = x[:, 0]
        except ArpackNoConvergence as e:
            x = e.eigenvectors[:, 0] if e.eigenvectors.shape[1] else None
        if x is not None:
            rq = float(np.vdot(x, A @ x).real / np.vdot(x, B @ x).real)
            out["witness_rayleigh_quotient"] = rq
            out["witness_below_minus10"] = bool(rq < -10)
        out["witness_seconds"] = round(time.time() - t, 1)
    # slowest eigenvalues of (K, B), nearest -5 (comparison only)
    t = time.time()
    try:
        lam, vec = eigs(K.tocsc(), k=20, M=B, sigma=-5.0, which="LM", tol=1e-10)
        order = np.argsort(lam.real)
        out["eigs_near_minus5"] = [[float(z.real), float(z.imag)] for z in lam[order]]
        out["eigs_near_minus5_residuals"] = [
            float(np.linalg.norm(K @ vec[:, i] - lam[i] * (B @ vec[:, i])) /
                  (float(abs(K).sum(axis=0).max()) * np.linalg.norm(vec[:, i]))) for i in order]
    except Exception as e:                           # noqa: BLE001
        out["eigs_near_minus5_failed"] = repr(e)
    out["eigs_seconds"] = round(time.time() - t, 1)
    out["W_left_at_least_minus10"] = (below10 == 0) if below10 is not None else None
    out["seconds"] = round(time.time() - t0, 1)
    return out


# ---------------------------------------------------------------- eps (n = 1, 2)
def eps_test(level, trial):
    import ricci_su2_batched as RB
    t0 = time.time()
    Bs = load_B(level)
    Bd = Bs.toarray()
    bat = RB.BatchedLattice(level, trial)
    rows = []
    for e in (1e-5, 1e-6, 1e-7):
        K = load_K(level, trial)[0] if e == 1e-6 else bat.build_K(eps=e)
        G, _, _ = whiten(K.toarray(), Bd)
        lam = eig(G, right=False)
        w_left = float(eigh(0.5 * (G + G.T), eigvals_only=True)[0])
        r = {"eps": e, "W_left": w_left, **stab_counts(lam)}
        rows.append(r)
        print(json.dumps(r), flush=True)
    ref = rows[1]
    same_count = all(r["small_modulus_below_1"] == ref["small_modulus_below_1"] for r in rows)
    w_rel = max(abs(r["W_left"] - ref["W_left"]) / abs(ref["W_left"]) for r in rows)
    return {"level": level, "trial": trial, "rows": rows,
            "small_modulus_count_identical": same_count, "W_left_max_rel_diff": w_rel,
            "G_eps_pass": bool(same_count and w_rel <= 1e-6),
            "seconds": round(time.time() - t0, 1),
            "kind": "B6 G-eps (research script, not an engine run)"}


def main():
    mode, level, trial = sys.argv[1], int(sys.argv[2]), sys.argv[3]
    receipt = Path(sys.argv[4]) if len(sys.argv) > 4 else None
    if receipt and receipt.exists():
        raise SystemExit(f"refusing to overwrite {receipt}")
    out = {"dense": dense, "sparse": sparse, "eps": eps_test}[mode](level, trial)
    lam = out.pop("_eigs", None)
    pts = out.pop("_points", None)
    print(json.dumps(out), flush=True)
    if receipt:
        receipt.parent.mkdir(parents=True, exist_ok=True)
        if lam is not None:
            np.save(receipt.with_suffix(".eigs.npy"), lam)
            out["boundary_points"] = pts
        receipt.write_text(json.dumps(out, indent=1) + "\n")


if __name__ == "__main__":
    main()
