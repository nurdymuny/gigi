"""Spin-0 sector with a second envelope seated on the second classical vacuum ("dome" functions).

The origin basis is the J=0 invariant basis of `ym_helike_j0`: m(T, P, d) exp(-alpha T/2), with q = M/g coordinates
here (the energies do not depend on the coordinate scaling). The second classical vacuum is the orbit
{R/g : R in SO(3)}, where W = T/2 - g det q has its other critical point. A dome function is the Haar average of a
Gaussian of width beta centred on that orbit, times a low-degree invariant polynomial:

    Phi(q) = int_SO(3) dR exp(-beta |q - R/g|^2 / 2) = exp(-3 beta/(2 g^2)) exp(-beta T/2) F(kappa q),
    F(X) = int dR exp(tr(R^T X)),   kappa = beta/g.

Exact series for F. Write X = U diag(s) V^T with U, V in SO(3) (s3 carries the sign of det X). By Haar
invariance tr(R^T X) has the law of sum_i s_i R_ii. With R from a uniform unit quaternion (w, x, y, z),
sum_i s_i R_ii = sum_j lambda_j u_j, where u = (w^2, x^2, y^2, z^2) is Dirichlet(1/2, 1/2, 1/2, 1/2) and

    lambda = (s1+s2+s3, s1-s2-s3, -s1+s2-s3, -s1-s2+s3).

Dirichlet moments give E[(lambda.u)^k] = k!/(2)_k [t^k] prod_j (1 - lambda_j t)^(-1/2), so

    F(X) = sum_k c_k(X) / (k+1)!,   sum_k c_k t^k = (1 - 2T t^2 - 8 d t^3 + (2P - T^2) t^4)^(-1/2),

since prod_j (1 - lambda_j t) = 1 - 2T t^2 - 8d t^3 + (2P - T^2) t^4 in T = tr XX^T, P = tr (XX^T)^2, d = det X.
The c_k are exact invariant polynomials. Checks: c_2/3! = T/6 (so E[(tr R^T X)^2] = T/3) and the k = 3 term
gives E[(tr R^T X)^3] = det X.

The truncated series F_K (degree <= K) is used as given. A dome function is then just a specific invariant
polynomial times a Gaussian of width beta, so every matrix element is exact (Gaussian moments with mixed widths),
Ritz values remain upper bounds, and residuals remain full-operator residuals. Truncation changes the trial
function, not the validity of the bounds.
"""
from __future__ import annotations

from fractions import Fraction as F

import mpmath as mp
import numpy as np

import ym_helike_j0 as j0
from ym_helike_j0 import _mpf, degree, laplacian, mono_mul, moment

ONE, TM, PM, DM = (0, 0, 0), (1, 0, 0), (0, 1, 0), (0, 0, 1)


def _pmul(a, b):
    out = {}
    for m1, c1 in a.items():
        for m2, c2 in b.items():
            k = mono_mul(m1, m2)
            v = out.get(k, 0) + c1 * c2
            if v:
                out[k] = v
            else:
                out.pop(k, None)
    return out


def _padd(a, b, s=1):
    out = dict(a)
    for m, c in b.items():
        v = out.get(m, 0) + s * c
        if v:
            out[m] = v
        else:
            out.pop(m, None)
    return out


# prod_j (1 - lambda_j t) = 1 + u(t), u = -2T t^2 - 8 d t^3 + (2P - T^2) t^4, as {t-power: invariant poly}
HAAR_U = {2: {TM: F(-2)}, 3: {DM: F(-8)}, 4: {PM: F(2), (2, 0, 0): F(-1)}}


def haar_series(K):
    """Exact invariant polynomials c_0..c_K with F(X) = sum_k c_k(X)/(k+1)!."""
    u = HAAR_U
    c = {0: {ONE: F(1)}}
    power = {0: {ONE: F(1)}}          # u^n truncated, as t-series
    binom = F(1)
    for n in range(1, K // 2 + 1):
        new = {}
        for p, a in power.items():
            for q, b in u.items():
                if p + q <= K:
                    new[p + q] = _padd(new.get(p + q, {}), _pmul(a, b))
        power = new
        binom = binom * (F(-1, 2) - (n - 1)) / n
        for p, a in power.items():
            c[p] = _padd(c.get(p, {}), {m: binom * v for m, v in a.items()})
    return [c.get(k, {}) for k in range(K + 1)]


def dome_polynomial(K, kappa):
    """F_K(kappa q) as {monomial: mpf}, at the current mp precision."""
    out = {}
    fact = mp.mpf(1)
    for k, ck in enumerate(haar_series(K)):
        fact *= (k + 1)
        scale = kappa ** k / fact
        for m, v in ck.items():
            out[m] = out.get(m, 0) + scale * _mpf(v)
    return out


def hamiltonian_poly(f, a, g):
    """H (f e^{-a T/2}) = e^{-a T/2} h, in q coordinates, for an invariant polynomial f with mp coefficients."""
    h = {}

    def add(m, c):
        h[m] = h.get(m, 0) + c

    for m, c in f.items():
        for mm, cc in laplacian(m).items():
            add(mm, -c * _mpf(cc) / 2)
        add(m, c * (a * degree(m) + 9 * a / 2))
        add(mono_mul(m, TM), c * (1 - a * a) / 2)
        add(mono_mul(m, DM), -3 * g * c)
        add(mono_mul(m, (2, 0, 0)), g * g * c / 4)
        add(mono_mul(m, PM), -g * g * c / 4)
    return h


class Moments:
    """<f e^{-aT/2}, h e^{-bT/2}> = sum c c' gamma^{-(deg+9)/2} M[mono], gamma = (a+b)/2 (common pi^{9/2} dropped)."""

    def __init__(self):
        self.cache = {}

    def value(self, mono, gamma):
        key = (mono, gamma)
        v = self.cache.get(key)
        if v is None:
            v = self.cache[key] = gamma ** (-mp.mpf(degree(mono) + 9) / 2) * _mpf(moment(mono))
        return v

    def inner(self, f, a, h, b):
        gamma = (a + b) / 2
        return mp.fsum(c1 * c2 * self.value(mono_mul(m1, m2), gamma) for m1, c1 in f.items() for m2, c2 in h.items())


def dome_monomials(n_d):
    return [m for m in j0.basis(n_d)]


def solve(N, g2, alpha, n_d=None, beta=None, K=30, dps=80, nlev=6):
    """Origin J=0 basis at cutoff N plus dome functions m * F_K(kappa q) e^{-beta T/2}, deg m <= n_d.

    n_d=None means no dome functions (must reproduce ym_helike_j0.solve)."""
    with mp.workdps(dps):
        g = mp.sqrt(_mpf(F(g2).limit_denominator(10 ** 12)))
        al = _mpf(F(alpha).limit_denominator(10 ** 12))
        funcs = [({m: mp.mpf(1)}, al, ("origin", m)) for m in j0.basis(N)]
        n_origin = len(funcs)
        if n_d is not None:
            be = _mpf(F(beta).limit_denominator(10 ** 12))
            Fk = dome_polynomial(K, be / g)
            for m in dome_monomials(n_d):
                funcs.append(({mono_mul(m, k): v for k, v in Fk.items()}, be, ("dome", m)))
        n = len(funcs)
        hs = [hamiltonian_poly(f, a, g) for f, a, _ in funcs]
        Mo = Moments()
        S, H, Kmat = mp.matrix(n, n), mp.matrix(n, n), mp.matrix(n, n)
        for i in range(n):
            fi, ai, _ = funcs[i]
            for j in range(i, n):
                fj, aj, _ = funcs[j]
                S[i, j] = S[j, i] = Mo.inner(fi, ai, fj, aj)
                hij, hji = Mo.inner(fi, ai, hs[j], aj), Mo.inner(fj, aj, hs[i], ai)
                if i != j and abs(hij - hji) > max(abs(hij), abs(hji), 1) * mp.mpf(10) ** (-(dps - 20)):
                    raise ArithmeticError(f"H not symmetric at {funcs[i][2]},{funcs[j][2]}: {abs(hij - hji)}")
                H[i, j] = H[j, i] = (hij + hji) / 2
                Kmat[i, j] = Kmat[j, i] = Mo.inner(hs[i], ai, hs[j], aj)
        out = j0.finish(S, H, Kmat, dps, nlev)
        out["outside_origin_fraction"] = outside_origin(S, out["vectors"], n_origin, dps)
    out.update({"cutoff": N, "g2": float(g2), "alpha": float(alpha), "beta": None if n_d is None else float(beta),
                "n_d": n_d, "K": K, "n_origin": n_origin, "n_dome": n - n_origin,
                "labels": [lab for _, _, lab in funcs]})
    return out


def outside_origin(S, V, n_origin, dps):
    """For each returned state psi (unit norm), 1 - ||P_origin psi||^2: the part the origin basis cannot hold."""
    n = S.rows
    if n == n_origin:
        return [0.0] * V.shape[1]
    with mp.workdps(dps):
        L = mp.cholesky(S)
        Li = mp.inverse(L)
        Soo = mp.matrix([[S[i, j] for j in range(n_origin)] for i in range(n_origin)])
        Soo_inv = mp.inverse(Soo)
        Sall_o = mp.matrix([[S[i, j] for j in range(n_origin)] for i in range(n)])
        out = []
        for k in range(V.shape[1]):
            v = mp.matrix(V[:, k].tolist())
            c = Li.T * v                         # coefficients in the original functions
            b = Sall_o.T * c                     # <origin_i, psi>
            proj = (b.T * Soo_inv * b)[0]
            norm = (c.T * S * c)[0]
            out.append(float(1 - proj / norm))
        return out
