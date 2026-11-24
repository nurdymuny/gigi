"""O(g^2) corrections to the vibrational gaps of the sourced condensate, by exact perturbation theory in G-space.

Colour singlets are functions of G = q q^T. On them the Hamiltonian reduces exactly to
    H_red = -1/2 [A_ij(G) d_i d_j + b_i d_i] + V_src(G),   A_ij = grad G_i . grad G_j (linear in G),  b_i = Lap G_i,
symmetric with respect to rho dG, rho = (det G)^{-1/2} (checked numerically). After the similarity rho^{1/2}:
    H_s = -1/2 d_i A_ij(G) d_j + V_src(G) + U(G),   U = 1/2 rho^{-1/2} d_i (A_ij d_j rho^{1/2}).
Around G_bg ~ 1/g^2 with eta = G - G_bg ~ 1/g: A = A0 + A1(eta). The cubic potential and the kinetic cubic
-1/2 d_i A1_ij(eta) d_j are O(g); the quartic potential is O(g^2); U is O(g^2) and constant at that order, so it
cancels in gaps. The O(g^2) gap corrections are second order in (V3 + T3) plus first order in V4: exact finite
sums in the normal-mode ladder operators.

Sheet d = +sqrt(det G) (the condensate has det q > 0); tunnelling to the other sheet is beyond all orders here.
"""
from __future__ import annotations

import itertools
import math
from collections import defaultdict

import numpy as np
import sympy as sp

GKEYS = [(0, 0), (1, 1), (2, 2), (0, 1), (0, 2), (1, 2)]


def sym_index(k, l):
    return GKEYS.index((min(k, l), max(k, l)))


def mat_from_vec(x):
    M = np.zeros((3, 3), dtype=object if any(isinstance(v, sp.Basic) for v in x) else float)
    for i, (k, l) in enumerate(GKEYS):
        M[k, l] = M[l, k] = x[i]
    return M


def A_coeffs():
    """C[i, j, m] with A_ij(G) = sum_m C[i, j, m] G_m (from grad G_kl . grad G_mn)."""
    C = np.zeros((6, 6, 6))
    for i, (k, l) in enumerate(GKEYS):
        for j, (m, n) in enumerate(GKEYS):
            for (a, b, c, d) in [(k, m, l, n), (k, n, l, m), (l, m, k, n), (l, n, k, m)]:
                if a == b:
                    C[i, j, sym_index(c, d)] += 1
    return C


C_A = A_coeffs()
B_VEC = np.array([6., 6., 6., 0., 0., 0.])


def A_at(Gv):
    return np.einsum("ijm,m->ij", C_A, Gv)


def omega(t):
    return np.array([math.sqrt(t), 1 / math.sqrt(t), 1 / math.sqrt(t)])


def V_symbolic(t, g, a, b):
    """V_src as a sympy expression in the six G entries (sheet d = +sqrt(det G))."""
    x = sp.symbols("G0:6", real=True)
    G = sp.Matrix(3, 3, lambda k, l: x[sym_index(k, l)])
    om = omega(t)
    T = G.trace()
    trG2 = (G * G).trace()
    d = sp.sqrt(G.det())
    V = sum(sp.Float(om[k] ** 2 / 2) * G[k, k] for k in range(3)) - sp.Float(g * om.sum()) * d \
        + sp.Float(g * g / 4) * (T ** 2 - trG2) - sp.Float(a / 2) * G[0, 0] - sp.Float(b / 2) * (G[1, 1] + G[2, 2])
    return x, V


def taylor(expr, x, x0, order):
    """Taylor coefficients of expr at x0 up to 'order': dict {exponent tuple: coefficient}."""
    lam = sp.Symbol("lam")
    eta = sp.symbols("e0:6", real=True)
    shifted = expr.subs({x[i]: sp.Float(x0[i]) + lam * eta[i] for i in range(6)}, simultaneous=True)
    ser = sp.series(shifted, lam, 0, order + 1).removeO()
    poly = sp.Poly(sp.expand(ser), lam, *eta)
    out = {}
    for mon, c in poly.terms():
        if mon[0] <= order:
            out[mon[1:]] = float(c)
    return out


class Fock:
    """Six normal modes with frequencies w; operators z_n and d/dz_n on occupation-number states."""

    def __init__(self, w):
        self.w = np.asarray(w, float)

    def z(self, st, n):
        out = defaultdict(float)
        c = 1 / math.sqrt(2 * self.w[n])
        for occ, amp in st.items():
            if occ[n]:
                o = list(occ); o[n] -= 1
                out[tuple(o)] += amp * c * math.sqrt(occ[n])
            o = list(occ); o[n] += 1
            out[tuple(o)] += amp * c * math.sqrt(occ[n] + 1)
        return dict(out)

    def dz(self, st, n):
        out = defaultdict(float)
        c = math.sqrt(self.w[n] / 2)
        for occ, amp in st.items():
            if occ[n]:
                o = list(occ); o[n] -= 1
                out[tuple(o)] += amp * c * math.sqrt(occ[n])
            o = list(occ); o[n] += 1
            out[tuple(o)] -= amp * c * math.sqrt(occ[n] + 1)
        return dict(out)

    def energy(self, occ):
        return float(np.dot(occ, self.w))


def addto(acc, st, c=1.):
    for k, v in st.items():
        acc[k] = acc.get(k, 0.) + c * v


class CondensatePT:
    def __init__(self, s, t, g):
        import helike_berger_model as bm
        self.s, self.t, self.g = s, t, g
        _, _, qb = bm.mc_background(s, t, g)
        a, b = bm.mc_sources(s, t)
        Gb = qb @ qb.T
        self.G0 = np.array([Gb[k, l] for (k, l) in GKEYS])
        x, V = V_symbolic(t, g, a, b)
        self.tay = taylor(V, x, self.G0, 4)
        lin = np.array([self.tay.get(tuple(int(i == j) for i in range(6)), 0.) for j in range(6)])
        self.grad_norm = float(np.abs(lin).max())                    # must vanish: G_bg is stationary
        K = np.zeros((6, 6))
        for mon, c in self.tay.items():
            if sum(mon) == 2:
                idx = [i for i in range(6) for _ in range(mon[i])]
                if idx[0] == idx[1]:
                    K[idx[0], idx[0]] += 2 * c
                else:
                    K[idx[0], idx[1]] += c
                    K[idx[1], idx[0]] += c
        self.K = K
        A0 = A_at(self.G0)
        wA, UA = np.linalg.eigh(A0)
        W = UA @ np.diag(np.sqrt(wA)) @ UA.T
        w2, O = np.linalg.eigh(W @ K @ W)
        self.w = np.sqrt(w2)
        self.M = W @ O                           # eta = M z
        self.Minv_T = np.linalg.inv(self.M).T    # d/deta = Minv_T d/dz
        self.fock = Fock(self.w)

    # polynomial operators on Fock states
    def apply_eta(self, st, a):
        out = {}
        for n in range(6):
            if abs(self.M[a, n]) > 0:
                addto(out, self.fock.z(st, n), self.M[a, n])
        return out

    def apply_deta(self, st, a):
        out = {}
        for n in range(6):
            if abs(self.Minv_T[a, n]) > 0:
                addto(out, self.fock.dz(st, n), self.Minv_T[a, n])
        return out

    def apply_monomial(self, st, mon):
        for a in range(6):
            for _ in range(mon[a]):
                st = self.apply_eta(st, a)
        return st

    def apply_V(self, st, degree):
        out = {}
        for mon, c in self.tay.items():
            if sum(mon) == degree:
                addto(out, self.apply_monomial(st, mon), c)
        return out

    def apply_T3(self, st):
        """-1/2 d_i A1_ij(eta) d_j = -1/2 [C_ijm eta_m d_i d_j + C_iji d_j]."""
        out = {}
        for i in range(6):
            for j in range(6):
                dj = self.apply_deta(st, j)
                if not dj:
                    continue
                dij = self.apply_deta(dj, i)
                for m in range(6):
                    if C_A[i, j, m]:
                        addto(out, self.apply_eta(dij, m), -.5 * C_A[i, j, m])
                if C_A[i, j, i]:
                    addto(out, dj, -.5 * C_A[i, j, i])
        return out

    def second_order_matrix(self, states, tol=1e-9):
        """Effective O(g^2) matrix on a degenerate set of H0 eigenstates (list of occupation tuples)."""
        e0 = self.fock.energy(states[0])
        imgs = []
        for occ in states:
            st = {occ: 1.}
            v = self.apply_V(st, 3)
            addto(v, self.apply_T3(st))
            imgs.append(v)
        n = len(states)
        Meff = np.zeros((n, n))
        for p in range(n):
            v4 = self.apply_V({states[p]: 1.}, 4)
            for r in range(n):
                Meff[p, r] += v4.get(states[r], 0.)
                for occ, amp in imgs[r].items():
                    de = e0 - self.fock.energy(occ)
                    if abs(de) < tol:
                        if occ in states:
                            continue
                        raise ArithmeticError(f"resonant intermediate state {occ}")
                    Meff[p, r] += imgs[p].get(occ, 0.) * amp / de
        return (Meff + Meff.T) / 2

    def gaps(self):
        """Harmonic gap and O(g^2)-corrected gap for each single-quantum level (degenerate modes grouped)."""
        vac = (0,) * 6
        e_vac = self.second_order_matrix([vac])[0, 0]
        out, used = [], set()
        for n in np.argsort(self.w):
            if n in used:
                continue
            group = [m for m in range(6) if abs(self.w[m] - self.w[n]) < 1e-7]
            used.update(group)
            states = [tuple(int(k == m) for k in range(6)) for m in group]
            ev = np.linalg.eigvalsh(self.second_order_matrix(states))
            out.append({"omega": float(self.w[n]), "multiplicity": len(group),
                        "dE2": [float(e - e_vac) for e in ev]})
        return out
