"""Soft-mode flow test (19 November 2026, night): does the field give way along the twist past t_j?

Exact (no lattice). On the Berger sphere g = (t, 1, 1):
  background  A(tau) = sum_i m_i(tau) theta^i, left-invariant, flowing by the Yang-Mills gradient
              dm/dtau = -grad E(m), E = 1/2 sum_{i<j} |m_i x m_j - sum_k c_ij^k m_k|^2, from the Levi-Civita field;
              a left-invariant field stays left-invariant, so the background never leaves the spin-0 sector;
  disturbance b(tau) in the spin-j/2 sector, gauge-fixed linearised flow db/dtau = -(H + G G^*)(A(tau)) b
              (left-invariance keeps every sector separate for all tau).
The twist crossing: lambda_t(j) = 0 at t_j = [sqrt((j+2)^2 + 2(j+4)^2) - (j+2)] / 2.

Predictions (recorded in this file before the first run):
  P1 sign       just past t_j the disturbance grows at first (d log|b|/dtau > 0 at tau = 0); just below, it decays.
  P2 direction  past t_j a random disturbance turns toward the twist: its overlap with the twist subspace at tau = 0
                (the lambda_t(j) eigenspace, twice) rises toward 1.
  P3 reported   how the moving background changes the floor and the overlap over the window.

usage: python scripts/ricci_su2_soft_mode_flow.py OUT.json
"""
import json
import sys
from pathlib import Path

import numpy as np
from scipy.integrate import solve_ivp

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_spin_sector as S       # noqa: E402

PAIRS = S.PAIRS


def structure(t):
    s = np.sqrt(np.array([t, 1.0, 1.0]))
    c = np.zeros((3, 3, 3))
    for (i, j, k), v in S.BR.items():
        c[i, j, k] = v * s[k] / (s[i] * s[j])
    return s, c


def lc_background(t):
    _, _, m, _ = S.background([t, 1, 1])
    return np.array(m)


def energy(mv, c):
    m = mv.reshape(3, 3); E = 0.0
    for (i, j) in PAIRS:
        f = np.cross(m[i], m[j]) - sum(c[i, j, k] * m[k] for k in range(3))
        E += 0.5 * f @ f
    return E


def grad(mv, c, h=1e-7):
    g = np.zeros(9)
    for a in range(9):
        e = np.zeros(9); e[a] = h
        g[a] = (energy(mv + e, c) - energy(mv - e, c)) / (2 * h)
    return g


def operator(m, s, c, j):
    """H + G G^* in the spin-j/2 sector at the left-invariant background m (as ricci_su2_sector_gauge_check)."""
    rho = S.spin_rep(j); d = j + 1; I3, Id = np.eye(3), np.eye(d); N = 9 * d
    idx = lambda i: slice(i * 3 * d, (i + 1) * 3 * d)                  # noqa: E731
    deriv = lambda i: np.kron(I3, rho[i] / s[i])                       # noqa: E731
    ad = lambda v: np.kron(S.cross_mat(v), Id)                         # noqa: E731
    f = {(i, jj): np.cross(m[i], m[jj]) - sum(c[i, jj, k] * m[k] for k in range(3)) for (i, jj) in PAIRS}
    blocks = []
    for (i, jj) in PAIRS:
        D = np.zeros((3 * d, N), complex)
        D[:, idx(jj)] += deriv(i) + ad(m[i]); D[:, idx(i)] -= deriv(jj) + ad(m[jj])
        for k in range(3):
            D[:, idx(k)] -= c[i, jj, k] * np.eye(3 * d)
        blocks.append(D)
    Dall = np.vstack(blocks); H = Dall.conj().T @ Dall
    for (i, jj) in PAIRS:
        blk = np.kron(-S.cross_mat(f[(i, jj)]), Id)
        H[idx(i), idx(jj)] += blk; H[idx(jj), idx(i)] += blk.T
    Gm = np.vstack([deriv(i) + ad(m[i]) for i in range(3)])
    return H + Gm @ Gm.conj().T


def lam(t, j):
    return (j * (j + 2) - 2 * (t - 3) * j - 2 * (t - 2) * (t + 4)) / t


def t_cross(j):
    return (np.sqrt((j + 2) ** 2 + 2 * (j + 4) ** 2) - (j + 2)) / 2


def run(t, j, T=2.0, seed=0):
    s, c = structure(t)
    m0 = lc_background(t).ravel()
    A0 = operator(m0.reshape(3, 3), s, c, j)
    w, V = np.linalg.eigh(A0)
    tw = V[:, np.abs(w - lam(t, j)) < 1e-8]                             # the twist subspace (2 complex dims)
    N = A0.shape[0]
    rng = np.random.default_rng(seed)
    b0 = rng.normal(size=N) + 1j * rng.normal(size=N); b0 /= np.linalg.norm(b0)

    def rhs(tau, y):
        mv = y[:9]; b = y[9:9 + N] + 1j * y[9 + N:]
        A = operator(mv.reshape(3, 3), s, c, j)
        db = -A @ b
        return np.concatenate([-grad(mv, c), db.real, db.imag])
    taus = np.linspace(0, T, 21)
    sol = solve_ivp(rhs, (0, T), np.concatenate([m0, b0.real, b0.imag]), t_eval=taus, rtol=1e-9, atol=1e-12)
    rows = []
    for tau, y in zip(sol.t, sol.y.T):
        mv = y[:9]; b = y[9:9 + N] + 1j * y[9 + N:]
        A = operator(mv.reshape(3, 3), s, c, j); wi, Vi = np.linalg.eigh(A)
        rows.append({"tau": float(tau), "log_norm_b": float(np.log(np.linalg.norm(b))),
                     "overlap_twist0": float(np.linalg.norm(tw.conj().T @ b) / np.linalg.norm(b)),
                     "overlap_lowest2_now": float(np.linalg.norm(Vi[:, :2].conj().T @ b) / np.linalg.norm(b)),
                     "floor_now": float(wi[0]), "second_distinct_now": float(wi[2]),
                     "background_energy": float(energy(mv, c))})
    rate0 = float(-(b0.conj() @ A0 @ b0).real)                          # d log|b| / dtau at tau = 0
    return {"t": t, "j": j, "t_j": float(t_cross(j)), "lambda_t_j": float(lam(t, j)),
            "eig_A0_lowest4": [float(x) for x in w[:4]], "initial_growth_rate": rate0,
            "growth_rate_twist_only": float(-lam(t, j)), "trajectory": rows}


def main():
    out = Path(sys.argv[1]); res = []
    for j in (3, 4):
        tj = t_cross(j)
        for dt in (-0.05, 0.05, 0.5):
            r = run(round(tj + dt, 4), j)
            res.append(r)
            a, z = r["trajectory"][0], r["trajectory"][-1]
            print(f"j={j} t={r['t']:.4f} (t_j={tj:.4f}) lambda={r['lambda_t_j']:+.4f} floor0={r['eig_A0_lowest4'][0]:+.4f} "
                  f"| log|b| {a['log_norm_b']:+.3f} -> {z['log_norm_b']:+.3f}; overlap twist {a['overlap_twist0']:.3f} -> "
                  f"{z['overlap_twist0']:.3f}; floor now {z['floor_now']:+.4f}; E {a['background_energy']:.3f} -> {z['background_energy']:.3f}", flush=True)
    out.write_text(json.dumps(res, indent=1) + "\n")


if __name__ == "__main__":
    main()
