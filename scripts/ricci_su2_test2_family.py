"""Test 2 of the spectral-skeleton hypothesis, family 1 (docs/research/skeleton-tests-preregistration-2026-11-20.md,
section 4, "Protocol for family 1", fixed before this script was written).

For each grid point t = 0.05..10 (step 0.05) and sector j = 0..12: the gauge-orthogonal spectrum (H compressed to
ker G*), the unique extremal state b (charge j/2 + 2 of Q = J0 + R_frame + R_Lie), checks that b is an exact
eigenvector of H and gauge-orthogonal, and whether <b, H b> is the floor. Classes from T_all (the grid points where b
is the floor in every sector): E / P / N.

  family "berger": the Levi-Civita field of (t, 1, 1) - the code control, which must give T_all = {2.70, ..., 10.00}
  family "meron":  the squashed meron (resting field), continued from the meron at t = 1 in both directions

usage: python scripts/ricci_su2_test2_family.py FAMILY OUT.json
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_chiral_bound as CB          # noqa: E402
import ricci_su2_meron_family_soft as MF     # noqa: E402
import ricci_su2_soft_mode_flow as F         # noqa: E402
import ricci_su2_spin_sector as S            # noqa: E402

JMAX = 12
GRID = np.round(np.arange(1, 201) * 0.05, 4)          # 0.05 .. 10.00, 200 points
TOL_FLOOR, TOL_EIG = 1e-9, 1e-9


def h_and_g(m, s, c, j):
    """H and G in the spin-j/2 sector at the left-invariant background m (the construction of F.operator)."""
    rho = S.spin_rep(j); d = j + 1; I3, Id = np.eye(3), np.eye(d); N = 9 * d
    idx = lambda i: slice(i * 3 * d, (i + 1) * 3 * d)                  # noqa: E731
    deriv = lambda i: np.kron(I3, rho[i] / s[i])                       # noqa: E731
    ad = lambda v: np.kron(S.cross_mat(v), Id)                         # noqa: E731
    f = {(i, jj): np.cross(m[i], m[jj]) - sum(c[i, jj, k] * m[k] for k in range(3)) for (i, jj) in F.PAIRS}
    blocks = []
    for (i, jj) in F.PAIRS:
        D = np.zeros((3 * d, N), complex)
        D[:, idx(jj)] += deriv(i) + ad(m[i]); D[:, idx(i)] -= deriv(jj) + ad(m[jj])
        for k in range(3):
            D[:, idx(k)] -= c[i, jj, k] * np.eye(3 * d)
        blocks.append(D)
    Dall = np.vstack(blocks); H = Dall.conj().T @ Dall
    for (i, jj) in F.PAIRS:
        blk = np.kron(-S.cross_mat(f[(i, jj)]), Id)
        H[idx(i), idx(jj)] += blk; H[idx(jj), idx(i)] += blk.T
    G = np.vstack([deriv(i) + ad(m[i]) for i in range(3)])
    return H, G


def charge(j):
    d = j + 1; rho = S.spin_rep(j); I3 = np.eye(3)
    Q = (np.kron(np.eye(9), 0.5j * rho[0]) + np.kron(np.kron(CB.rot0(), I3), np.eye(d))
         + np.kron(np.kron(I3, CB.rot0()), np.eye(d)))
    return 0.5 * (Q + Q.conj().T)


def backgrounds(family):
    if family == "berger":
        return {float(t): F.lc_background(t) for t in GRID}
    up = [t for t in GRID if t >= 1.0]
    down = [t for t in GRID[::-1] if t <= 1.0]
    out = {}
    for ts in (up, down):
        for t, x in zip(ts, MF.follow(ts)):
            out[float(t)] = x.reshape(3, 3)
    return out


def intervals(ts):
    """Compress a sorted list of grid points into runs [a, b]."""
    runs = []
    for t in ts:
        if runs and abs(t - runs[-1][1] - 0.05) < 1e-9:
            runs[-1][1] = t
        else:
            runs.append([t, t])
    return runs


def main():
    family, out = sys.argv[1], Path(sys.argv[2])
    bg = backgrounds(family)
    Qs = {j: np.linalg.eigh(charge(j)) for j in range(JMAX + 1)}
    rows, floor_at = [], {j: [] for j in range(JMAX + 1)}
    for t in map(float, GRID):
        s, c = F.structure(t); m = bg[t]
        for j in range(JMAX + 1):
            H, G = h_and_g(m, s, c, j)
            q, U = Qs[j]
            top = np.abs(q - (j / 2 + 2)) < 1e-8
            if top.sum() != 1:
                raise SystemExit(f"t={t} j={j}: maximal charge not unique ({top.sum()})")
            b = U[:, top][:, 0]
            lam_b = float((b.conj() @ H @ b).real)
            res = np.linalg.norm(H @ b - lam_b * b); gst = np.linalg.norm(G.conj().T @ b)
            if res > TOL_EIG * np.linalg.norm(H, 2) or gst > TOL_EIG * max(np.linalg.norm(G, 2), 1.0):
                raise SystemExit(f"t={t} j={j}: extremal state not an exact gauge-orthogonal eigenvector "
                                 f"(residual {res:.2e}, |G*b| {gst:.2e})")
            Uf, sv, _ = np.linalg.svd(G, full_matrices=True)
            rank = int((sv > 1e-10 * sv.max()).sum())
            K = Uf[:, rank:]                                   # orthonormal basis of ker G*
            mu = np.linalg.eigvalsh(K.conj().T @ H @ K)
            is_floor = lam_b <= mu[0] + TOL_FLOOR * max(1.0, abs(mu[0]))
            if is_floor:
                floor_at[j].append(t)
            rows.append({"t": t, "j": j, "extremal": lam_b, "slice_floor": float(mu[0]),
                         "slice_second": float(mu[1]), "is_floor": bool(is_floor),
                         "residual": float(res), "G_star_b": float(gst), "slice_dim": int(K.shape[1])})
    t_all = [t for t in map(float, GRID) if all(t in floor_at[j] for j in range(JMAX + 1))]
    runs = intervals(t_all)
    if len(t_all) == len(GRID):
        cls = "E"
    elif any(b - a > 1e-9 for a, b in runs):
        cls = "P"
    else:
        cls = "N"
    summary = {"family": family, "class": cls, "T_all_runs": runs, "n_T_all": len(t_all),
               "per_sector_runs": {j: intervals(floor_at[j]) for j in range(JMAX + 1)}}
    if family == "berger":
        expect = [float(t) for t in GRID if t >= 2.70 - 1e-9]
        summary["code_control_pass"] = t_all == expect
    out.write_text(json.dumps({"summary": summary, "rows": rows}, indent=1) + "\n")
    print(json.dumps(summary, indent=1))


if __name__ == "__main__":
    main()
