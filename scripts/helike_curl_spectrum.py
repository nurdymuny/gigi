"""The curl spectrum on Berger spheres and Helike's floor: is the twist's floor set by helicity?

In three dimensions sum_{i<j} |D_ij b|^2 = |*D_A b|^2, so the edge paper's second variation is

    Q(b) = |curl_A b|^2 + 2 <F, [b, b]>,   curl_A b = *D_A b  (the Hessian of Chern-Simons at A),

and H = curl_A^2 + Phi with Phi the field-strength coupling. This module builds curl_A in a Peter-Weyl sector j with
the conventions of scripts/ricci_su2_spin_sector.py (orthonormal left frame of (t,1,1), [e_a, e_b] = 2 eps e_c,
cross-product su(2), index (frame i, Lie a, V_j w)), for any member M_0 = x tau_0, M_1 = y tau_1, M_2 = y tau_2 of
the circle-symmetric family. The Berger field is x = (2 - t)/sqrt t, y = sqrt t.

(curl b)_k = D_ij b for (i, j, k) cyclic.
"""
from __future__ import annotations

import numpy as np

import ricci_su2_spin_sector as rs

CYC = [((1, 2), 0), ((2, 0), 1), ((0, 1), 2)]


def frame(t):
    s = np.sqrt(np.array([t, 1., 1.]))
    c = np.zeros((3, 3, 3))
    for (i, j, k), v in rs.BR.items():
        c[i, j, k] = v * s[k] / (s[i] * s[j])
    return s, c


def family_field(x, y):
    """Lie vectors m_i of M_i (tau_a <-> unit vector a under the cross product)."""
    return [np.array([x, 0., 0.]), np.array([0., y, 0.]), np.array([0., 0., y])]


def berger_xy(t):
    return (2 - t) / np.sqrt(t), np.sqrt(t)


def operators(x, y, t, j):
    """(curl, H, Phi, D_all) in sector j at the family member (x, y, t)."""
    s, c = frame(t)
    m = family_field(x, y)
    f = {}
    for (i, jj) in rs.PAIRS:
        f[(i, jj)] = np.cross(m[i], m[jj]) - sum(c[i, jj, k] * m[k] for k in range(3))
    rho = rs.spin_rep(j)
    d = j + 1
    N = 9 * d
    I3, Id = np.eye(3), np.eye(d)
    idx = lambda i: slice(i * 3 * d, (i + 1) * 3 * d)  # noqa: E731

    def D(i, jj):
        out = np.zeros((3 * d, N), complex)
        out[:, idx(jj)] += np.kron(I3, rho[i] / s[i]) + np.kron(rs.cross_mat(m[i]), Id)
        out[:, idx(i)] -= np.kron(I3, rho[jj] / s[jj]) + np.kron(rs.cross_mat(m[jj]), Id)
        for k in range(3):
            out[:, idx(k)] -= c[i, jj, k] * np.eye(3 * d)
        return out

    curl = np.zeros((N, N), complex)
    for (i, jj), k in CYC:
        curl[idx(k), :] = D(i, jj)
    Dall = np.vstack([D(i, jj) for (i, jj) in rs.PAIRS])
    Phi = np.zeros((N, N), complex)
    for (i, jj) in rs.PAIRS:
        blk = np.kron(-rs.cross_mat(f[(i, jj)]), Id)
        Phi[idx(i), idx(jj)] += blk
        Phi[idx(jj), idx(i)] += blk.T
    H = Dall.conj().T @ Dall + Phi
    return curl, H, Phi, Dall, f


def twist_closed_form(x, y, t, j):
    return (x + (j + 2) / np.sqrt(t)) ** 2 + 2 * np.sqrt(t) * x - y * y


def fibre_field(x, y, t):
    """B = y^2 - 2 sqrt(t) x, the field along the long circle (edge paper eq. (B))."""
    return y * y - 2 * np.sqrt(t) * x


def twist_space(H, lam, tol=1e-7):
    """Eigenvectors of H with eigenvalue lam (the twist and its mirror)."""
    w, V = np.linalg.eigh(H)
    sel = np.abs(w - lam) < tol * max(1, abs(lam))
    return V[:, sel]


def curl_on(V, curl):
    """Eigenvalues of curl restricted to span(V), and how invariant that span is (0 = exactly invariant)."""
    C = V.conj().T @ curl @ V
    leak = np.linalg.norm(curl @ V - V @ C)
    return np.linalg.eigvalsh((C + C.conj().T) / 2), float(leak)


def gauge_map(x, y, t, j):
    """G xi = (e^_i xi + [M_i, xi])_i, xi in V_j (x) su(2)."""
    s, c = frame(t)
    m = family_field(x, y)
    rho = rs.spin_rep(j)
    d = j + 1
    return np.vstack([np.kron(np.eye(3), rho[i] / s[i]) + np.kron(rs.cross_mat(m[i]), np.eye(d)) for i in range(3)])


def gauge_orthogonal(A, G):
    """Spectrum of P A P restricted to ker G^*."""
    U, sv, _ = np.linalg.svd(G, full_matrices=True)
    r = int(np.sum(sv > 1e-10 * max(1, sv.max())))
    K = U[:, r:]                                       # orthonormal basis of (range G)^perp = ker G^*
    B = K.conj().T @ A @ K
    return np.linalg.eigvalsh((B + B.conj().T) / 2), K


def floors_on_berger(t, jmax):
    """Per sector: H gauge-orthogonal floor, smallest gauge-orthogonal curl^2, and the twist's lambda and mu^2."""
    x, y = berger_xy(t)
    rows = []
    for j in range(jmax + 1):
        curl, H, Phi, _, _ = operators(x, y, t, j)
        G = gauge_map(x, y, t, j)
        hH, K = gauge_orthogonal(H, G)
        hC, _ = gauge_orthogonal(curl.conj().T @ curl, G)
        lam = twist_closed_form(x, y, t, j)
        mu2 = (x + (j + 2) / np.sqrt(t)) ** 2
        rows.append({"j": j, "H_floor": float(hH[0]), "curl2_floor": float(hC[0]), "twist_lambda": float(lam),
                     "twist_mu2": float(mu2)})
    return rows


def window_report(ts, jmax, tol=1e-9):
    """For each t: is the twist the H floor of every sector; is her mu^2 the curl^2 floor of every sector."""
    out = []
    for t in ts:
        rows = floors_on_berger(t, jmax)
        lam_min = min(r["twist_lambda"] for r in rows)
        h_min = min(r["H_floor"] for r in rows)
        mu_min = min(r["twist_mu2"] for r in rows)
        c_min = min(r["curl2_floor"] for r in rows)
        out.append({"t": float(t), "twist_is_H_floor": bool(lam_min <= h_min + tol),
                    "twist_is_curl_floor": bool(mu_min <= c_min + tol),
                    "H_gap": float(h_min - lam_min), "curl_gap": float(c_min - mu_min),
                    "twist_j_H": int(min(rows, key=lambda r: r["twist_lambda"])["j"]),
                    "twist_j_curl": int(min(rows, key=lambda r: r["twist_mu2"])["j"])})
    return out


def per_sector_status(t, jmax, tol=1e-9):
    """Twist is the floor of EVERY sector j <= jmax: for H (gauge-orthogonal) and for curl^2 (gauge-orthogonal)."""
    rows = floors_on_berger(t, jmax)
    h = all(r["twist_lambda"] <= r["H_floor"] + tol * max(1, abs(r["H_floor"])) for r in rows)
    c = all(r["twist_mu2"] <= r["curl2_floor"] + tol * max(1, abs(r["curl2_floor"])) for r in rows)
    worst_h = min(rows, key=lambda r: r["H_floor"] - r["twist_lambda"])
    worst_c = min(rows, key=lambda r: r["curl2_floor"] - r["twist_mu2"])
    return {"t": float(t), "H_every_sector": h, "curl_every_sector": c,
            "H_worst": (worst_h["j"], worst_h["H_floor"] - worst_h["twist_lambda"]),
            "curl_worst": (worst_c["j"], worst_c["curl2_floor"] - worst_c["twist_mu2"])}


def bisect_edge(key, lo, hi, jmax, iters=40):
    """Threshold in t where per_sector_status(t)[key] changes value between lo and hi."""
    a, b = per_sector_status(lo, jmax)[key], per_sector_status(hi, jmax)[key]
    if a == b:
        return None
    for _ in range(iters):
        mid = (lo + hi) / 2
        if per_sector_status(mid, jmax)[key] == a:
            lo = mid
        else:
            hi = mid
    return (lo + hi) / 2
