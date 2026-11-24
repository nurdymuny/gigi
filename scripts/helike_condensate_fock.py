"""Local Fock diagonalisation of the sourced condensate in the reduced G-space (non-perturbative in g).

Colour singlets reduce exactly to G = q q^T; after the similarity rho^{1/2} (rho = det G^{-1/2}):
    H_s = -1/2 d_i A_ij(G) d_j + V_src(G) + U(G),   U = 1/2 rho^{-1/2} d_i (A_ij d_j rho^{1/2})   (flat measure dG).
In the normal coordinates z of the harmonic problem (eta = G - G_bg = M z, with M M^T = A0):
    H_s = -1/2 sum_np d_n B_np(z) d_p + V(z) + U(z),   B(z) = M^{-1} A(G_bg + M z) M^{-T} = I + linear in z (exact).
V + U are Taylor-expanded to order P in z (convergent: the positive-definite boundary is several sigma away at the
registered depths). The spectrum is computed in the 6-mode oscillator basis with total quanta <= Mq, using sparse
ladder matrices on the space of quanta <= Mq + P + 2. Approximate (Taylor order P, basis Mq, sheet d = +sqrt det G),
convergence-controlled, not residual-certified. Validated against the exact O(g^2) perturbation theory at small g.
"""
from __future__ import annotations

import itertools
import math

import numpy as np
import scipy.sparse as sps
import scipy.sparse.linalg as spla
import sympy as sp

import helike_berger_model as bm
import helike_condensate_pt as cp


def states_up_to(n_modes, nmax):
    out = []
    for tot in range(nmax + 1):
        for c in itertools.combinations_with_replacement(range(n_modes), tot):
            occ = [0] * n_modes
            for i in c:
                occ[i] += 1
            out.append(tuple(occ))
    return out


class LocalFock:
    def __init__(self, s, t, g, P=6, Mq=6, include_U=True, include_kinetic_cubic=True):
        self.s, self.t, self.g, self.P, self.Mq = s, t, g, P, Mq
        self.include_kinetic_cubic = include_kinetic_cubic
        self.pt = cp.CondensatePT(s, t, g)
        self.w, M = self.pt.w, self.pt.M
        self.Minv = np.linalg.inv(M)
        # Taylor of V + U in z
        a, b = bm.mc_sources(s, t)
        x, V = cp.V_symbolic(t, g, a, b)
        Gm = sp.Matrix(3, 3, lambda k, l: x[cp.sym_index(k, l)])
        rho_half = Gm.det() ** sp.Rational(-1, 4)
        Asym = [[sum(sp.Float(cp.C_A[i, j, m]) * x[m] for m in range(6)) for j in range(6)] for i in range(6)]
        U = sum(sp.diff(Asym[i][j] * sp.diff(rho_half, x[j]), x[i]) for i in range(6) for j in range(6)) / (2 * rho_half)
        z = sp.symbols("z0:6", real=True)
        lam = sp.Symbol("lam")
        G0 = self.pt.G0
        sub = {x[i]: sp.Float(G0[i]) + lam * sum(sp.Float(M[i, n]) * z[n] for n in range(6)) for i in range(6)}
        self.poly = {}
        for expr in ((V, U) if include_U else (V,)):
            ser = sp.series(expr.subs(sub, simultaneous=True), lam, 0, P + 1).removeO()
            for mon, c in sp.Poly(sp.expand(ser), lam, *z).terms():
                if 0 < mon[0] <= P or (mon[0] == 0):
                    key = mon[1:]
                    if sum(key) >= 1:
                        self.poly[key] = self.poly.get(key, 0.) + float(c)
        # B(z) = I + sum_r Bl[n, p, r] z_r
        Bl = np.zeros((6, 6, 6))
        for r in range(6):
            dA = np.einsum("ijm,m->ij", cp.C_A, M[:, r])
            Bl[:, :, r] = self.Minv @ dA @ self.Minv.T
        self.Bl = Bl
        self._build()

    def _build(self):
        big = states_up_to(6, self.Mq + self.P + 2)
        index = {s: i for i, s in enumerate(big)}
        N = len(big)

        def ladder(n, sign):
            rows, cols, vals = [], [], []
            for j, occ in enumerate(big):
                if sign > 0:
                    o = list(occ); o[n] += 1
                    k = index.get(tuple(o))
                    if k is not None:
                        rows.append(k); cols.append(j); vals.append(math.sqrt(occ[n] + 1))
                elif occ[n]:
                    o = list(occ); o[n] -= 1
                    rows.append(index[tuple(o)]); cols.append(j); vals.append(math.sqrt(occ[n]))
            return sps.csr_matrix((vals, (rows, cols)), shape=(N, N))
        Z, Dz = [], []
        for n in range(6):
            ad, a = ladder(n, 1), ladder(n, -1)
            Z.append((a + ad) / math.sqrt(2 * self.w[n]))
            Dz.append((a - ad) * math.sqrt(self.w[n] / 2))
        # kinetic: -1/2 d_n B_np d_p = -1/2 [d_n d_n] - 1/2 sum Bl[n,p,r] d_n z_r d_p
        H = sps.csr_matrix((N, N))
        for n in range(6):
            H = H - .5 * (Dz[n] @ Dz[n])
        for n in range(6):
            for p in range(6):
                for r in range(6):
                    c = self.Bl[n, p, r] if self.include_kinetic_cubic else 0.
                    if abs(c) > 1e-14:
                        H = H - .5 * c * (Dz[n] @ Z[r] @ Dz[p])
        # potential polynomial (quadratic part included; it reproduces the oscillators)
        cache = {(0,) * 6: sps.identity(N, format="csr")}

        def mono(key):
            if key in cache:
                return cache[key]
            n = next(i for i in range(6) if key[i])
            prev = list(key); prev[n] -= 1
            m = Z[n] @ mono(tuple(prev))
            cache[key] = m
            return m
        for key, c in self.poly.items():
            H = H + c * mono(key)
        keep = [i for i, s in enumerate(big) if sum(s) <= self.Mq]
        self.basis = [big[i] for i in keep]
        Hk = H[keep][:, keep]
        self.H = ((Hk + Hk.T) / 2).toarray()
        self.asym = float(np.abs(Hk - Hk.T).max())

    def spectrum(self, nlev=12):
        e, v = np.linalg.eigh(self.H)
        return e[:nlev], v[:, :nlev]

    def classify(self, vec):
        """Weight of an eigenvector on single-quantum states of each mode (identifies vibrational character)."""
        w = np.zeros(6)
        for i, occ in enumerate(self.basis):
            if sum(occ) == 1:
                w[occ.index(1)] += vec[i] ** 2
        return w

    def helike_and_competitor(self, nlev=12):
        """Gaps of the lowest states dominated by Helike's doublet and by the Q=0 competitor mode."""
        e, v = self.spectrum(nlev)
        wH = math.sqrt(bm.mc_helike_freq2(self.s, self.t))
        hel = [n for n in range(6) if abs(self.w[n] - wH) < 1e-6]
        single = [n for n in range(6) if n not in hel and sum(abs(self.w[n] - self.w[m]) < 1e-6 for m in range(6)) == 1]
        comp = min(single, key=lambda n: self.w[n])
        out = {"E0": float(e[0])}
        for name, modes in (("helike", hel), ("competitor", [comp])):
            best = max(range(1, nlev), key=lambda k: self.classify(v[:, k])[modes].sum())
            out[name] = {"gap": float(e[best] - e[0]), "weight": float(self.classify(v[:, best])[modes].sum())}
        return out
