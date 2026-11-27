"""EXPLORATORY prototype (decides nothing; not registered): Rayleigh-Ritz in the unfolded chart P = sqrt(G).

The reduced field space is flat R^9 (q) folded by colour O(3); G = q q^T. Write G = P^2 with P a symmetric 3x3 matrix
(any symmetric P gives a positive semidefinite G; P with flipped eigenvalue signs is the other sheet of the fold).
Coordinates: the six entries of P, centred and scaled to the vacuum: P = mu + A zeta.
  measure        dmu = det(G)^{-1/2} dG  =>  rho_P = prod_{i<j} |l_i + l_j| (times 8 and |det A|; constants cancel)
  kinetic        1/2 A(x)_{ab} d_a f d_b f in Gram entries x  =>  cometric g_zeta = Ai J A(x) J^T Ai^T, J = dP/dx
  basis          phi_s(zeta) = prod_j h_{s_j}(zeta_j) exp(-|zeta|^2/2), total degree <= Mq (NOT orthonormal in dmu)
  generalized    H c = E S c,  S = int phi phi rho, H = 1/2 int grad phi . g grad phi rho + int V phi phi rho
Quadrature: product Gauss-Hermite in zeta (weight e^{-|zeta|^2}).
The frame (mu, C) is made self-consistent from the prototype's own ground state (like 5.10), symmetrised over the charge
rotation is NOT applied here (exploratory)."""
import itertools, json, math, sys, time
import numpy as np
from scipy.special import roots_hermite
from scipy.linalg import eigh
import helike_condensate_pt as cp
import helike_positive_cone as pc
import helike_m8_geometry as gm

KEYS = [(0, 0), (1, 1), (2, 2), (0, 1), (0, 2), (1, 2)]


def mat(v):
    M = np.zeros((len(v), 3, 3))
    for k, (i, j) in enumerate(KEYS):
        M[:, i, j] = v[:, k]; M[:, j, i] = v[:, k]
    return M


def vec(M):
    return np.column_stack([M[:, i, j] for i, j in KEYS])


def setup(s, t, g2):
    metric, V = pc.laurent_model(s, t, math.sqrt(g2))
    return V


def potential(Vterms, G):
    L = np.linalg.cholesky(G)
    var = np.column_stack((L[:, 0, 0], L[:, 1, 1], L[:, 2, 2], L[:, 1, 0], L[:, 2, 0], L[:, 2, 1]))
    out = np.zeros(len(G))
    for key, c in Vterms.items():
        out += c * np.prod(var ** np.array(key), axis=1)
    return out


def node_data(zeta, mu, A, Vterms):
    """rho, cometric in zeta, potential at the nodes."""
    Pm = mat(mu + zeta @ A.T)
    G = Pm @ Pm
    lam, U = np.linalg.eigh(Pm)
    rho = np.abs(lam[:, 0] + lam[:, 1]) * np.abs(lam[:, 0] + lam[:, 2]) * np.abs(lam[:, 1] + lam[:, 2])
    # dx/dP: for each P-coordinate direction E_k, dG = P E + E P
    dxdP = np.zeros((len(zeta), 6, 6))
    for k, (i, j) in enumerate(KEYS):
        E = np.zeros((3, 3)); E[i, j] = 1.0; E[j, i] = 1.0
        dxdP[:, :, k] = vec(Pm @ E + E @ Pm)
    J = np.linalg.inv(dxdP)                                     # dP/dx
    Ax = np.einsum("ijm,nm->nij", cp.C_A, vec(G))
    Ai = np.linalg.inv(A)
    gz = np.einsum("ab,nbc,ncd,ned,fe->naf", Ai, J, Ax, J, Ai, optimize=True)
    V = potential(Vterms, G)
    return rho, gz, V


def solve(s, t, g2, mq, order, mu, A, nlev=12, chunk=20000):
    Vterms = setup(s, t, g2)
    x1, w1 = roots_hermite(order); w1 = w1 / math.sqrt(math.pi)
    states = [st for st in itertools.product(range(mq + 1), repeat=6) if sum(st) <= mq]
    N = len(states); S_ = np.array(states)
    Smat = np.zeros((N, N)); Hm = np.zeros((N, N))
    idx = np.array(np.unravel_index(np.arange(order ** 6), (order,) * 6)).T
    for a in range(0, len(idx), chunk):
        ix = idx[a:a + chunk]
        zeta = x1[ix]; W = np.prod(w1[ix], axis=1)
        rho, gz, V = node_data(zeta, mu, A, Vterms)
        Wr = W * rho
        h = [gm.hermite_tables(zeta[:, j], mq) for j in range(6)]   # (values, derivatives), each (mq+1, n)
        B = np.ones((len(zeta), N))
        for j in range(6):
            B *= h[j][0][S_[:, j]].T
        Smat += B.T @ (Wr[:, None] * B)
        Hm += B.T @ ((Wr * V)[:, None] * B)
        # gradient of phi in zeta, divided by the Gaussian: d_j h - zeta_j h
        Q = np.empty((6, len(zeta), N))
        for j in range(6):
            Dj = np.ones((len(zeta), N))
            for k in range(6):
                Dj *= (h[k][1] if k == j else h[k][0])[S_[:, k]].T
            Q[j] = Dj - zeta[:, j:j + 1] * B
        e, Vv = np.linalg.eigh(gz)
        gh = np.einsum("nij,nj,nkj->nik", Vv, np.sqrt(np.clip(e, 0, None)), Vv)
        for k in range(6):
            Rk = np.einsum("nj,jnm->nm", gh[:, k, :], Q)
            Hm += 0.5 * Rk.T @ (Wr[:, None] * Rk)
    E, C = eigh(Hm, Smat, subset_by_index=[0, nlev - 1])
    return E, C, states


def ground_moments(C0, states, order, mu, A, mq, s, t, g2):
    """Mean and covariance of P in psi0^2 dmu, from the prototype's own ground state (for self-consistency)."""
    Vterms = setup(s, t, g2)
    x1, w1 = roots_hermite(order); w1 = w1 / math.sqrt(math.pi)
    S_ = np.array(states)
    idx = np.array(np.unravel_index(np.arange(order ** 6), (order,) * 6)).T
    m0 = 0.0; m1 = np.zeros(6); m2 = np.zeros((6, 6))
    for a in range(0, len(idx), 50000):
        ix = idx[a:a + 50000]; zeta = x1[ix]; W = np.prod(w1[ix], axis=1)
        Pm = mu + zeta @ A.T
        lam = np.linalg.eigvalsh(mat(Pm))
        rho = np.abs(lam[:, 0] + lam[:, 1]) * np.abs(lam[:, 0] + lam[:, 2]) * np.abs(lam[:, 1] + lam[:, 2])
        B = np.ones((len(zeta), len(states)))
        for j in range(6):
            B *= gm.normalized_hermites(zeta[:, j], mq)[S_[:, j]].T
        f = B @ C0
        wt = W * rho * f * f
        m0 += wt.sum(); m1 += wt @ Pm; m2 += (Pm * wt[:, None]).T @ Pm
    mean = m1 / m0
    return mean, m2 / m0 - np.outer(mean, mean)


if __name__ == "__main__":
    mq_list = [int(v) for v in sys.argv[1].split(",")]
    order = int(sys.argv[2])
    s, t, g2 = (float(v) for v in sys.argv[3].split(",")) if len(sys.argv) > 3 else (1.18, 3.0, 0.03)
    # starting frame: sqrt of the classical background, widths from a quick isotropic guess refined by self-consistency
    bg = cp.CondensatePT(s, t, math.sqrt(g2))
    G0 = cp.mat_from_vec(bg.G0)
    l, U = np.linalg.eigh(G0); P0 = (U * np.sqrt(l)) @ U.T
    mu = vec(P0[None])[0]
    Cp = np.eye(6) * 0.05
    out = {"point": [s, t, g2], "order": order, "runs": []}
    for mq in mq_list:
        for it in range(4):
            e, V = np.linalg.eigh(2 * Cp); A = (V * np.sqrt(e)) @ V.T
            t0 = time.time()
            E, C, states = solve(s, t, g2, mq, order, mu, A)
            mu_new, C_new = ground_moments(C[:, 0], states, order, mu, A, mq, s, t, g2)
            change = float(np.max(np.abs(mu_new - mu)))
            rec = {"Mq": mq, "iteration": it, "dim": len(states), "E0": float(E[0]), "levels": E.tolist(),
                   "gaps": (E[1:] - E[0]).tolist(), "frame_change": change, "seconds": time.time() - t0}
            out["runs"].append(rec)
            print(json.dumps({k: (v if k != "levels" else None) for k, v in rec.items() if k != "levels"}), flush=True)
            mu, Cp = mu_new, C_new
            if change < 1e-5:
                break
    print(json.dumps(out))
