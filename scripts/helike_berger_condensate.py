"""Sourced-condensate solver: mixed-width basis with orbit-envelope (dome) functions on the Berger model.

H_src = -1/2 Lap + 1/2|omega o q - g cof q|^2 - a/2 |q_0|^2 - b/2 (|q_1|^2 + |q_2|^2), colour singlets.
Basis items are (f, beta): a real polynomial f(G, d) (G = q q^T, d = det q) times exp(-1/2 sum_k beta_k |q_k|^2), each
item with its own per-row widths. Mixed products use gamma_k = (alpha_k + beta_k)/2 per row:
    int mono exp(-sum_k gamma_k |q_k|^2) = pi^{9/2} prod_k gamma_k^{-3/2 - deg_k/2} <mono>_iso.

Orbit envelope ("dome") on the background q_bg = diag(X, Y, Y): the Haar average over colour R of
exp(-1/2 sum_k beta_k |q_k - (q_bg R)_k|^2) equals, up to a constant, exp(-1/2 sum_k beta_k |q_k|^2) F(Lambda q) with
Lambda = diag(beta_0 X, beta_1 Y, beta_2 Y) and F(X) = int dR exp(tr R^T X). F depends on X only through
tr(X X^T) = sum_k lambda_k^2 G_kk, tr((X X^T)^2) = sum_kl lambda_k^2 lambda_l^2 G_kl^2 and det X = lambda_0 lambda_1 lambda_2 d,
so a truncated F_K is an exact polynomial in (G, d). It is long-axis charge 0 and parity even.
"""
from __future__ import annotations

import mpmath as mp

import helike_berger_sector as bs
import ym_helike_dome as dm
import ym_helike_j0 as j0


def _poly_mp(p):
    return {k: (v if isinstance(v, mp.mpf) else bs._mpf(v)) for k, v in p.items()}


def lin(p, c):
    return {k: c * v for k, v in p.items()}


def dome_poly(K, lam):
    """F_K(Lambda q) as a polynomial in (G, d); lam = (lambda_0, lambda_1, lambda_2)."""
    TX = {}
    for k in range(3):
        TX = bs.padd(TX, lin(_poly_mp(bs.gvar(k, k)), lam[k] ** 2))
    PX = {}
    for k in range(3):
        for l in range(3):
            PX = bs.padd(PX, lin(_poly_mp(bs.pmul(bs.gvar(k, l), bs.gvar(k, l))), lam[k] ** 2 * lam[l] ** 2))
    DX = lin(_poly_mp(bs.D), lam[0] * lam[1] * lam[2])
    powers = {"T": [{bs.ONE: mp.mpf(1)}], "P": [{bs.ONE: mp.mpf(1)}], "D": [{bs.ONE: mp.mpf(1)}]}

    def power(name, base, n):
        lst = powers[name]
        while len(lst) <= n:
            lst.append(bs.pmul(lst[-1], base))
        return lst[n]
    out = {}
    fact = mp.mpf(1)
    for k, ck in enumerate(dm.haar_series(K)):
        fact *= (k + 1)
        for (a, b, c), v in ck.items():
            term = bs.pmul(bs.pmul(power("T", TX, a), power("P", PX, b)), power("D", DX, c))
            out = bs.padd(out, lin(term, bs._mpf(v) / fact))
    return out


def h_apply(f, beta, g, t, a, b):
    """H_src (f e^{-1/2 sum beta_k |q_k|^2}) = e^{...} h, returned as h."""
    st = mp.sqrt(mp.mpf(t))
    om = [st, 1 / st, 1 / st]
    gg = mp.mpf(g)
    out = {}

    def acc(p, c):
        for k, v in p.items():
            out[k] = out.get(k, 0) + c * v
    acc(bs.laplacian(f), mp.mpf(-.5))
    for e, c in f.items():
        acc({e: c}, sum(beta[k] * bs.row_degree(e, k) for k in range(3)) + 1.5 * sum(beta))
    quad = [(om[k] ** 2 - beta[k] ** 2) / 2 - (a if k == 0 else b) / 2 for k in range(3)]
    for k in range(3):
        acc(bs.pmul(f, bs.gvar(k, k)), quad[k])
    acc(bs.pmul(f, bs.D), -gg * sum(om))
    acc(bs.pmul(f, bs.GRAD_DD), gg * gg / 2)
    return {k: v for k, v in out.items() if v != 0}


class Moments:
    def __init__(self):
        self.cache = {}

    def value(self, e, gam):
        key = (e, tuple(gam))
        v = self.cache.get(key)
        if v is None:
            sc = mp.mpf(1)
            for k in range(3):
                sc *= gam[k] ** (-(mp.mpf(3) / 2 + mp.mpf(bs.row_degree(e, k)) / 2))
            v = self.cache[key] = sc * bs._mpf(bs.iso_moment(e))
        return v

    def inner(self, f, al, h, be):
        gam = [(al[k] + be[k]) / 2 for k in range(3)]
        tot = mp.mpf(0)
        for ka, va in f.items():
            for kb, vb in h.items():
                for k, v in bs.pmul({ka: 1}, {kb: 1}).items():
                    tot += va * vb * v * self.value(k, gam)
        return tot


def solve_items(items, g2, t, a=0., b=0., dps=50, nlev=6):
    """items: list of (poly {mono: mpf}, (beta_0, beta_1, beta_2)). Energies and full-space residuals."""
    with mp.workdps(dps):
        g = mp.sqrt(mp.mpf(g2))
        A, Bs = mp.mpf(a), mp.mpf(b)
        items = [(_poly_mp(f), [mp.mpf(x) for x in be]) for f, be in items]
        hs = [h_apply(f, be, g, t, A, Bs) for f, be in items]
        n = len(items)
        Mo = Moments()
        S, H, K = mp.matrix(n, n), mp.matrix(n, n), mp.matrix(n, n)
        for i in range(n):
            fi, ai = items[i]
            for j in range(i, n):
                fj, aj = items[j]
                S[i, j] = S[j, i] = Mo.inner(fi, ai, fj, aj)
                hij, hji = Mo.inner(fi, ai, hs[j], aj), Mo.inner(fj, aj, hs[i], ai)
                if i != j and abs(hij - hji) > max(abs(hij), 1) * mp.mpf(10) ** (-(dps - 15)):
                    raise ArithmeticError(f"H not symmetric at ({i},{j})")
                H[i, j] = H[j, i] = (hij + hji) / 2
                K[i, j] = K[j, i] = Mo.inner(hs[i], ai, hs[j], aj)
        out = j0.finish(S, H, K, dps, nlev)
    out["dimension"] = n
    return out


def origin_items(N, m, parity, alpha):
    return [(f, alpha) for f in bs.sector_basis(N, m, parity)]


def dome_items(N_d, m, parity, K, qbg_diag, beta):
    """Orbit-envelope functions: (sector-(m,parity) monomials of degree <= N_d) x F_K(Lambda q), widths beta."""
    lam = [mp.mpf(beta[k]) * mp.mpf(qbg_diag[k]) for k in range(3)]
    F = dome_poly(K, lam)
    return [(bs.pmul(_poly_mp(mono), F), beta) for mono in bs.sector_basis(N_d, m, parity)]
