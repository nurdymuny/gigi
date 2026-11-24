"""The Berger-deformed homogeneous SU(2) Yang-Mills matrix model (design stage; no spectra here).

A constant left-invariant field A = sum_i M_i theta^i on the Berger sphere (t, 1, 1) has, in the edge paper's
conventions, F_ij = M_i x M_j - c_ij^k M_k with c = (c_12^0, c_20^1, c_01^2) = (2 sqrt t, 2/sqrt t, 2/sqrt t). Since
*F_k = (cof M)_k - c_k M_k and grad det M = cof M,

    V_t = 1/2 sum_{i<j} |F_ij|^2 = 1/2 |grad W_t|^2,   W_t = 1/2 sum_k c_k |M_k|^2 - det M,

exactly, at every stretch (left-invariant fields are the fixed set of the left translations, which remain isometries).
In model units (rows of q = frame index k, columns = colour), with omega = (sqrt t, 1/sqrt t, 1/sqrt t):

    H_t = -1/2 Lap_q + 1/2 |omega o q - g cof q|^2,   W_t(q) = 1/2 sum_k omega_k |q_k|^2 - g det q,

which is, term by term, an anisotropic oscillator 1/2 sum_k omega_k^2 |q_k|^2, an isotropic cubic
-g (sqrt t + 2/sqrt t) det q, and the isotropic quartic (g^2/2)|cof q|^2. At t = 1 this is the round model exactly.
The omega_k are the helicities (curl eigenvalues) of the homogeneous modes; the cubic is the flux coupling.
"""
from __future__ import annotations

import numpy as np

import ym_two_grain_molien as mo


def omega(t):
    return np.array([np.sqrt(t), 1 / np.sqrt(t), 1 / np.sqrt(t)])


def cof(q):
    return np.array([np.cross(q[1], q[2]), np.cross(q[2], q[0]), np.cross(q[0], q[1])])


def W(q, g, t):
    return .5 * np.sum(omega(t)[:, None] * q * q) - g * np.linalg.det(q)


def grad_W(q, g, t):
    return omega(t)[:, None] * q - g * cof(q)


def V(q, g, t):
    return .5 * np.sum(grad_W(q, g, t) ** 2)


def V_parts(q, g, t):
    """(helicity^2 oscillator, flux cubic, quartic) with V = sum of the three."""
    om = omega(t)
    return (.5 * np.sum((om[:, None] * q) ** 2), -g * om.sum() * np.linalg.det(q), .5 * g * g * np.sum(cof(q) ** 2))


def floor(t):
    """1/2 Lap W_t = (3/2)(sqrt t + 2/sqrt t): H_t = 1/2 A^dag A + floor."""
    return 1.5 * omega(t).sum()


def probe_channels(q):
    """Frame quadrupole components of G = q q^T by long-axis charge m (row 0 is the long axis)."""
    G = q @ q.T
    return {0: G[0, 0] - .5 * (G[1, 1] + G[2, 2]), 1: G[0, 1] + 1j * G[0, 2], 2: G[1, 1] - G[2, 2] + 2j * G[1, 2]}


def sector_counts(D, m):
    """Molien counts (by polynomial degree <= D) of colour-singlet states with long-axis charge m."""
    M = 2 * (D + 4) + 3
    th, w = mo.weyl_grid(M)
    ph = 2 * np.pi * np.arange(M) / M
    a, p = np.meshgrid(th, ph, indexing="ij")
    col = np.stack([a, 0 * a, -a], -1)
    frm = np.stack([0 * p, p, -p], -1)
    eig = np.exp(1j * (col[..., :, None] + frm[..., None, :])).reshape(M, M, 9)
    h = mo.complete_homogeneous(eig, D)
    H = np.einsum("kap,a,p->k", h, w, np.exp(-1j * m * ph) / M)
    Hi = np.rint(H.real).astype(int)
    if np.abs(H - Hi).max() > 1e-8:
        raise ArithmeticError("Molien counts not integral")
    return Hi


# ---------------------------------------------------------------- flux-background variant (sourced condensate)

def mc_background(s, t, g):
    """Background on the Maurer-Cartan line A = s theta: edge family (x, y) = (2s/sqrt t, 2s), q_bg = diag(x,y,y)/(2g)."""
    x, y = 2 * s / np.sqrt(t), 2 * s
    return x, y, np.diag([x, y, y]) / (2 * g)


def sources(qb, g, t, eps=1e-6):
    """(a, b) making q_bg stationary for V - a|q_0|^2/2 - b(|q_1|^2 + |q_2|^2)/2 (gradient of V at q_bg is diagonal)."""
    out = []
    for i in (0, 1):
        e = np.zeros((3, 3))
        e[i, i] = eps
        out.append((V(qb + e, g, t) - V(qb - e, g, t)) / (2 * eps) / qb[i, i])
    return tuple(out)


def V_sourced(q, g, t, a, b):
    return V(q, g, t) - .5 * a * np.sum(q[0] ** 2) - .5 * b * np.sum(q[1:] ** 2)


def bounded(g, t, a, b):
    """Flat (commuting) directions are bounded below iff omega_0^2 - a > 0 and omega_perp^2 - b > 0."""
    om = omega(t)
    return om[0] ** 2 - a > 0 and om[1] ** 2 - b > 0


# closed forms on the Maurer-Cartan line (derived; independent of g)

def mc_sources(s, t):
    """a = (1-s)(t-2s), b = (1-s)((1-s)/t - s)."""
    return (1 - s) * (t - 2 * s), (1 - s) * ((1 - s) / t - s)


def mc_s_max(t):
    """Boundedness window s < (t+2)/(t+1): omega_perp^2 - b = s[(2-s)/t + 1 - s]."""
    return (t + 2) / (t + 1)


def mc_depth(s, t, g):
    """V_src(q_bg) = -s^3 (s-1)(1/2 + 1/t) / g^2."""
    return -s ** 3 * (s - 1) * (.5 + 1 / t) / g ** 2


def mc_helike_freq2(s, t):
    """Helike's vibration (locked charge |Q| = 2, transverse rows only): omega_H^2 = 2 s (1 + 2/t - s)."""
    return 2 * s * (1 + 2 / t - s)


def locked_charge_casimir():
    """Q^2 for the U(1) that preserves q_bg: frame rotation in rows (1,2) together with colour rotation about axis 0."""
    E = np.zeros((3, 3))
    E[1, 2], E[2, 1] = -1, 1
    L = np.zeros((9, 9))
    for c in range(9):
        d = np.zeros(9)
        d[c] = 1
        D = d.reshape(3, 3)
        L[:, c] = (E @ D + D @ E.T).ravel()
    return -(L @ L)


def mc_vibrations(s, t, g, h=1e-4):
    """Sourced vibration frequencies^2 at q_bg by locked charge |Q| (colour-orbit zero modes removed)."""
    from test_ym_two_grain_potential import hessian
    _, _, qb = mc_background(s, t, g)
    a, b = mc_sources(s, t)
    Hs = hessian(lambda z: V_sourced(z.reshape(3, 3), g, t, a, b), qb.ravel(), h=h)
    wq, U = np.linalg.eigh(locked_charge_casimir())
    out = {}
    for Q in (0, 1, 2):
        S = U[:, np.abs(wq - Q * Q) < 1e-6]
        ev = np.linalg.eigvalsh(S.T @ Hs @ S)
        out[Q] = sorted(ev[np.abs(ev) > 1e-5].tolist())
    return out
