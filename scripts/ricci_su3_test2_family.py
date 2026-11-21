"""Test 2 of the spectral-skeleton hypothesis, family 2: SU(3) with the Maurer-Cartan background A = theta/2
(docs/research/skeleton-tests-preregistration-2026-11-20.md, section 4, "Protocol for family 2", fixed before this
script was written).

A general builder of the Yang-Mills Hessian H and the gauge map G at a left-invariant background on a compact group K
with left-invariant metric diagonal in a basis X_a ([X_a, X_b] = C_ab^c X_c), gauge algebra with orthonormal basis
T_alpha ([T_b, T_c] = f_bc^a T_a), background A = sum_i M_i theta^i in the orthonormal coframe, in the Peter-Weyl
sector of a representation rho of k (anti-Hermitian, rho([X, Y]) = [rho X, rho Y]). It is the construction of
scripts/ricci_su2_sector_gauge_check.py with su(2) replaced by general structure constants.

modes:
  controls OUT.json       code controls K1 (builder vs the SU(2) builder), K2 (Berger pipeline), K3 (SU(3) checks)
  family 2a|2b OUT.json   classify family 2a (t on u(2)) or 2b (t on lambda_8), grid t = 0.05..10, nine sectors
usage: python scripts/ricci_su3_test2_family.py MODE [FAMILY] OUT.json [NPROC]
"""
import itertools
import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))

GRID = np.round(np.arange(1, 201) * 0.05, 4)          # 0.05 .. 10.00, 200 points
TOL_FLOOR, TOL_EIG, TOL_Q = 1e-9, 1e-9, 1e-10
SECTORS = [(0, 0), (1, 0), (1, 1), (2, 0), (2, 1), (3, 0), (2, 2), (3, 1), (4, 0)]


# ---------------------------------------------------------------- general builder
def adm(v, f):
    """[v, .] on the gauge algebra: adm(v)[b, c] = sum_a v_a f[a, c, b]."""
    return np.einsum("a,acb->bc", v, f)


def hessian_and_gauge(C, g, f, M, rho):
    """H (Hessian of the Yang-Mills energy) and G (xi -> d_A xi) in the sector of rho.
    C: (n,n,n) frame brackets in the basis X_a; g: metric diagonal; f: (r,r,r) gauge structure constants;
    M: list of n gauge vectors (background in the orthonormal coframe); rho: list of n (d x d) matrices for X_a."""
    n, r, d = len(g), f.shape[0], rho[0].shape[0]
    s = np.sqrt(np.asarray(g, float))
    c = C * s[None, None, :] / (s[:, None, None] * s[None, :, None])
    Ir, Id = np.eye(r), np.eye(d)
    N = n * r * d
    idx = lambda i: slice(i * r * d, (i + 1) * r * d)                                # noqa: E731
    deriv = [np.kron(Ir, rho[i] / s[i]) for i in range(n)]
    ad = [np.kron(adm(M[i], f), Id) for i in range(n)]
    pairs = [(i, j) for i in range(n) for j in range(i + 1, n)]
    H = np.zeros((N, N), complex)
    for (i, j) in pairs:
        D = np.zeros((r * d, N), complex)
        D[:, idx(j)] += deriv[i] + ad[i]; D[:, idx(i)] -= deriv[j] + ad[j]
        for k in range(n):
            if c[i, j, k] != 0:
                D[:, idx(k)] -= c[i, j, k] * np.eye(r * d)
        H += D.conj().T @ D
    for (i, j) in pairs:
        Fij = adm(M[i], f) @ M[j] - sum(c[i, j, k] * M[k] for k in range(n))
        if np.abs(Fij).max() == 0:
            continue
        P = np.einsum("a,bca->bc", Fij, f)                                          # x^T P y = <F, [x, y]>
        blk = np.kron(P, Id)
        H[idx(i), idx(j)] += blk; H[idx(j), idx(i)] += blk.T
    G = np.vstack([deriv[i] + ad[i] for i in range(n)])
    return 0.5 * (H + H.conj().T), G


def energy(C, g, f, M):
    """Yang-Mills energy density (1/2) sum_{i<j} |F_ij|^2 of a left-invariant field (orthonormal coframe)."""
    n = len(g); s = np.sqrt(np.asarray(g, float))
    c = C * s[None, None, :] / (s[:, None, None] * s[None, :, None])
    e = 0.0
    for i in range(n):
        for j in range(i + 1, n):
            Fij = adm(M[i], f) @ M[j] - sum(c[i, j, k] * M[k] for k in range(n))
            e += 0.5 * float(Fij @ Fij)
    return e


def grad_norm(C, g, f, M, h=1e-6):
    M = [np.array(m, float) for m in M]
    gr = []
    for i in range(len(M)):
        for a in range(len(M[i])):
            Mp = [m.copy() for m in M]; Mm = [m.copy() for m in M]
            Mp[i][a] += h; Mm[i][a] -= h
            gr.append((energy(C, g, f, Mp) - energy(C, g, f, Mm)) / (2 * h))
    return float(np.linalg.norm(gr))


SVD_FALLBACKS = []


def robust_svd(G):
    """np.linalg.svd (gesdd); on non-convergence, scipy's gesvd driver (recorded). Same quantity, ker G^*."""
    try:
        return np.linalg.svd(G, full_matrices=True)
    except np.linalg.LinAlgError:
        import scipy.linalg as sl
        SVD_FALLBACKS.append(G.shape)
        return sl.svd(G, full_matrices=True, lapack_driver="gesvd")


def slice_spectrum(H, G):
    U, sv, _ = robust_svd(G)
    rank = int((sv > 1e-10 * sv.max()).sum()) if sv.size and sv.max() > 0 else 0
    K = U[:, rank:]
    return np.linalg.eigvalsh(K.conj().T @ H @ K)


# ---------------------------------------------------------------- SU(3) data
def gell_mann():
    L = np.zeros((8, 3, 3), complex)
    L[0][0, 1] = L[0][1, 0] = 1
    L[1][0, 1] = -1j; L[1][1, 0] = 1j
    L[2][0, 0] = 1; L[2][1, 1] = -1
    L[3][0, 2] = L[3][2, 0] = 1
    L[4][0, 2] = -1j; L[4][2, 0] = 1j
    L[5][1, 2] = L[5][2, 1] = 1
    L[6][1, 2] = -1j; L[6][2, 1] = 1j
    L[7] = np.diag([1, 1, -2]) / np.sqrt(3)
    return L


def su3_data():
    """X_a = -(i/2) lambda_a; C[a,b,c] = f_abc (X orthonormal for <X,Y> = -2 tr XY); gauge f = C."""
    X = -0.5j * gell_mann()
    C = np.zeros((8, 8, 8))
    for a in range(8):
        for b in range(8):
            Z = X[a] @ X[b] - X[b] @ X[a]
            for k in range(8):
                C[a, b, k] = float((-2 * np.trace(Z @ X[k])).real)
    for a, b in itertools.product(range(8), repeat=2):
        Z = X[a] @ X[b] - X[b] @ X[a]
        assert np.allclose(Z, np.einsum("k,kij->ij", C[a, b], X), atol=1e-13)
    return X, C


def su3_irrep(p, q, X):
    """rho(X_a) on V_(p,q): the submodule of (C^3)^{(x)p} (x) (C^3*)^{(x)q} generated from e1^p (x) e3*^q."""
    def big(Mat):
        facs = [Mat] * p + [-Mat.T] * q
        n = len(facs); dim = 3 ** n; out = np.zeros((dim, dim), complex)
        for k in range(n):
            ops = [np.eye(3)] * n; ops[k] = facs[k]
            T = ops[0]
            for o in ops[1:]:
                T = np.kron(T, o)
            out += T
        return out if n else np.zeros((1, 1), complex)
    n = p + q
    if n == 0:
        return [np.zeros((1, 1), complex) for _ in range(8)]
    v = np.array([1.0 + 0j])
    for k in range(n):
        e = np.zeros(3, complex); e[0 if k < p else 2] = 1
        v = np.kron(v, e)
    E = lambda a, b: np.eye(3)[:, [a]] @ np.eye(3)[[b], :]          # noqa: E731  matrix unit
    lowers = [big(E(1, 0)), big(E(2, 1))]
    basis = [v / np.linalg.norm(v)]; frontier = [basis[0]]
    while frontier:
        new = []
        for w in frontier:
            for Lo in lowers:
                u = Lo @ w
                for bvec in basis:
                    u = u - (bvec.conj() @ u) * bvec
                if np.linalg.norm(u) > 1e-9:
                    u /= np.linalg.norm(u); basis.append(u); new.append(u)
        frontier = new
    B = np.array(basis).T
    dim = (p + 1) * (q + 1) * (p + q + 2) // 2
    assert B.shape[1] == dim, (p, q, B.shape[1], dim)
    return [B.conj().T @ big(X[a]) @ B for a in range(8)]


def su3_metric(family, t):
    g = np.ones(8)
    if family == "2a":
        g[[0, 1, 2, 7]] = t
    elif family == "2b":
        g[7] = t
    else:
        raise ValueError(family)
    return g


def su3_background(g, s=0.5):
    """A = s theta: M_a = s X_a / sqrt(g_a), as gauge vectors in the orthonormal basis X."""
    return [s * np.eye(8)[a] / np.sqrt(g[a]) for a in range(8)]


def su3_charge(rho, X, C):
    """Q for the circle of H = -i diag(1, 0, -1) on frame (x) gauge (x) V; signs fitted later. Returns the 3 parts."""
    XH = -1j * np.diag([1.0, 0, -1.0])
    h = np.array([float((-2 * np.trace(XH @ X[a])).real) for a in range(8)])
    assert np.allclose(np.einsum("a,aij->ij", h, X), XH)
    A = np.einsum("c,cab->ba", h, C)                     # [X_H, X_a] = sum_b A[b, a] X_b
    rH = sum(h[a] * rho[a] for a in range(8))
    return 1j * A, 1j * A, 1j * rH                       # frame, gauge, V (Hermitian)


def assemble_Q(parts, signs, n, r, d):
    Af, Ag, AV = parts
    return (signs[0] * np.kron(np.kron(Af, np.eye(r)), np.eye(d)) + signs[1] * np.kron(np.kron(np.eye(n), Ag), np.eye(d))
            + signs[2] * np.kron(np.kron(np.eye(n), np.eye(r)), AV))


def fit_signs(parts, H, GGs, n, r, d):
    """The sign choices (up to overall sign) with [Q, H] = 0 and [Q, G G^*] = 0."""
    ok = []
    for sg in [(1, 1, 1), (1, 1, -1), (1, -1, 1), (1, -1, -1)]:
        Q = assemble_Q(parts, sg, n, r, d)
        e1 = np.abs(Q @ H - H @ Q).max() / max(np.abs(H).max(), 1e-300)
        e2 = np.abs(Q @ GGs - GGs @ Q).max() / max(np.abs(GGs).max(), 1e-300)
        if max(e1, e2) <= TOL_Q and not any(np.allclose(Q, assemble_Q(parts, o_, n, r, d)) for o_ in ok):
            ok.append(sg)                                # choices giving the same Q (e.g. rho = 0) count once
    return ok


# ---------------------------------------------------------------- one point
def classify_point(H, G, Q, top_charge):
    q, U = np.linalg.eigh(Q)
    top = np.abs(q - top_charge) < 1e-8
    if top.sum() != 1 or q.max() > top_charge + 1e-8:
        return {"error": f"maximal charge not unique or not {top_charge} (count {int(top.sum())}, max {q.max():.6g})"}
    b = U[:, top][:, 0]
    lam_b = float((b.conj() @ H @ b).real)
    res = float(np.linalg.norm(H @ b - lam_b * b)); gst = float(np.linalg.norm(G.conj().T @ b))
    if res > TOL_EIG * np.linalg.norm(H, 2) or gst > TOL_EIG * max(np.linalg.norm(G, 2), 1.0):
        return {"error": f"extremal state not an exact gauge-orthogonal eigenvector (residual {res:.2e}, |G*b| {gst:.2e})"}
    mu = slice_spectrum(H, G)
    is_floor = lam_b <= mu[0] + TOL_FLOOR * max(1.0, abs(mu[0]))
    return {"extremal": lam_b, "slice_floor": float(mu[0]), "slice_second": float(mu[1]), "is_floor": bool(is_floor),
            "residual": res, "G_star_b": gst, "slice_dim": int(len(mu))}


_SU3 = {}


def su3_point(args):
    family, t = args
    if not _SU3:
        X, C = su3_data(); _SU3["X"], _SU3["C"] = X, C
        _SU3["rho"] = {pq: su3_irrep(*pq, X) for pq in SECTORS}
    X, C = _SU3["X"], _SU3["C"]
    g = su3_metric(family, t); M = su3_background(g)
    out = {"t": float(t), "grad_norm": grad_norm(C, g, C, M), "energy": energy(C, g, C, M), "sectors": {}}
    for pq in SECTORS:
        rho = _SU3["rho"][pq]; d = rho[0].shape[0]
        H, G = hessian_and_gauge(C, g, C, M, rho)
        parts = su3_charge(rho, X, C)
        signs = fit_signs(parts, H, G @ G.conj().T, 8, 8, d)
        if len(signs) != 1:
            out["sectors"][str(pq)] = {"error": f"charge signs commuting with H and GG*: {signs}"}
            continue
        Q = assemble_Q(parts, signs[0], 8, 8, d)
        qv = np.linalg.eigvalsh(Q)
        if abs(qv.max() - (pq[0] + pq[1] + 4)) > 1e-8:       # pick the overall sign giving +max
            Q = -Q
        nfb = len(SVD_FALLBACKS)
        row = classify_point(H, G, Q, pq[0] + pq[1] + 4)
        row["signs"] = list(signs[0])
        row["svd_fallback"] = len(SVD_FALLBACKS) > nfb
        out["sectors"][str(pq)] = row
    return out


def intervals(ts):
    runs = []
    for t in ts:
        if runs and abs(t - runs[-1][1] - 0.05) < 1e-9:
            runs[-1][1] = t
        else:
            runs.append([t, t])
    return runs


def run_family(family, out, nproc):
    import multiprocessing as mp
    with mp.Pool(nproc) as pool:
        pts = pool.map(su3_point, [(family, float(t)) for t in GRID])
    errors = [(p["t"], k, v["error"]) for p in pts for k, v in p["sectors"].items() if "error" in v]
    floor_at = {str(pq): [p["t"] for p in pts if p["sectors"][str(pq)].get("is_floor")] for pq in SECTORS}
    t_all = [p["t"] for p in pts if all(p["sectors"][str(pq)].get("is_floor") for pq in SECTORS)]
    runs = intervals(t_all)
    cls = "E" if len(t_all) == len(GRID) else ("P" if any(b - a > 1e-9 for a, b in runs) else "N")
    summary = {"family": family, "stopped": bool(errors), "errors": errors[:20], "class": None if errors else cls,
               "T_all_runs": runs, "n_T_all": len(t_all), "per_sector_runs": {k: intervals(v) for k, v in floor_at.items()}}
    Path(out).write_text(json.dumps({"summary": summary, "points": pts}, indent=1) + "\n")
    print(json.dumps(summary, indent=1))


# ---------------------------------------------------------------- controls
def su2_data():
    import ricci_su2_spin_sector as S
    C = np.zeros((3, 3, 3))
    for (i, j, k), v in S.BR.items():
        C[i, j, k] = v
    eps = np.zeros((3, 3, 3))
    for (i, j, k) in [(0, 1, 2), (1, 2, 0), (2, 0, 1)]:
        eps[i, j, k] = 1; eps[j, i, k] = -1
    return S, C, eps


def control_K1():
    import ricci_su2_sector_gauge_check as SG
    S, C, eps = su2_data()
    worst = 0.0
    for t in (1.0, 3.0, 4.0):
        g = [t, 1, 1]; _, _, m, _ = S.background(g)
        for j in range(5):
            H0, G0 = SG.hessian_and_gauge(g, j)
            H1, G1 = hessian_and_gauge(C, g, eps, m, S.spin_rep(j))
            worst = max(worst, np.abs(H1 - H0).max(), np.abs(G1 - G0).max())
    return {"max_entry_difference": float(worst), "pass": bool(worst <= 1e-12)}


def control_K2():
    """Family 1's code control with the general builder: Berger family, j <= 12, must give T_all = {2.70..10.00}."""
    import ricci_su2_soft_mode_flow as F
    S, C, eps = su2_data()
    floor_at = {j: [] for j in range(13)}; errors = []
    for t in map(float, GRID):
        g = [t, 1, 1]; m = F.lc_background(t)
        m = [np.asarray(m)[i] for i in range(3)]
        for j in range(13):
            rho = S.spin_rep(j); d = j + 1
            H, G = hessian_and_gauge(C, g, eps, m, rho)
            # circle about axis 0: frame ad(e_0) (brackets 2 eps), gauge 2 x cross(e_0) (phi(e_a) = 2 tau_a), V rho_0
            Af = 1j * C[0].T                               # [e_0, e_a] = sum_b C[0, a, b] e_b -> A[b, a] = C[0, a, b]
            Ag = 1j * 2 * adm(np.eye(3)[0], eps)
            parts = (Af, Ag, 1j * rho[0])
            signs = fit_signs(parts, H, G @ G.conj().T, 3, 3, d)
            if len(signs) != 1:
                errors.append((t, j, f"signs {signs}")); continue
            Q = assemble_Q(parts, signs[0], 3, 3, d)
            if abs(np.linalg.eigvalsh(Q).max() - (j + 4)) > 1e-8:
                Q = -Q
            row = classify_point(H, G, Q, j + 4)
            if "error" in row:
                errors.append((t, j, row["error"])); continue
            if row["is_floor"]:
                floor_at[j].append(t)
    t_all = [t for t in map(float, GRID) if all(t in floor_at[j] for j in range(13))]
    expect = [float(t) for t in GRID if t >= 2.70 - 1e-9]
    return {"errors": errors[:10], "T_all_runs": intervals(t_all), "pass": (not errors) and t_all == expect}


def control_K3():
    X, C = su3_data()
    out = {}
    worst = 0.0
    for pq in SECTORS + [(0, 1)]:
        rho = su3_irrep(*pq, X)
        for a in range(8):
            for b in range(8):
                Z = rho[a] @ rho[b] - rho[b] @ rho[a] - sum(C[a, b, k] * rho[k] for k in range(8))
                worst = max(worst, np.abs(Z).max())
            worst = max(worst, np.abs(rho[a] + rho[a].conj().T).max())
    out["rep_error"] = float(worst)
    g1 = np.ones(8)
    gn = grad_norm(C, g1, C, su3_background(g1)); E = energy(C, g1, C, su3_background(g1))
    out["grad_at_t1"] = gn; out["energy_at_t1"] = E
    zero = [np.zeros(8) for _ in range(8)]
    mins = {}
    for pq in SECTORS:
        H, G = hessian_and_gauge(C, su3_metric("2a", 2.0), C, zero, su3_irrep(*pq, X))
        mins[str(pq)] = float(slice_spectrum(H, G)[0]) if G.shape[1] else None
    out["A0_min_slice_eig"] = mins
    g2 = su3_metric("2a", 2.0); M2 = su3_background(g2)
    e10 = np.sort(slice_spectrum(*hessian_and_gauge(C, g2, C, M2, su3_irrep(1, 0, X))))
    e01 = np.sort(slice_spectrum(*hessian_and_gauge(C, g2, C, M2, su3_irrep(0, 1, X))))
    out["conjugate_sector_diff"] = float(np.abs(e10 - e01).max())
    out["pass"] = bool(worst <= 1e-12 and gn <= 1e-6 * max(1.0, E) and all(v is None or v >= -1e-9 for v in mins.values())
                       and out["conjugate_sector_diff"] <= 1e-10)
    return out


def main():
    mode = sys.argv[1]
    if mode == "controls":
        out = Path(sys.argv[2])
        res = {"K1": control_K1()}; print("K1", res["K1"], flush=True)
        res["K3"] = control_K3(); print("K3", res["K3"], flush=True)
        res["K2"] = control_K2(); print("K2", res["K2"], flush=True)
        res["all_pass"] = all(res[k]["pass"] for k in ("K1", "K2", "K3"))
        out.write_text(json.dumps(res, indent=1, default=str) + "\n")
        print("ALL CONTROLS PASS" if res["all_pass"] else "CONTROLS FAIL")
    elif mode == "family":
        family, out = sys.argv[2], sys.argv[3]
        nproc = int(sys.argv[4]) if len(sys.argv) > 4 else 4
        run_family(family, out, nproc)
    else:
        raise SystemExit(__doc__)


if __name__ == "__main__":
    main()
