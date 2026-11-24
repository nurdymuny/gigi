"""Exact sector solver for the Berger-deformed homogeneous matrix model (colour SO(3) x long-axis SO(2)).

H_t = -1/2 Lap_q + 1/2 |omega o q - g cof q|^2, omega = (sqrt t, 1/sqrt t, 1/sqrt t), rows of q = frame (row 0 long).

Colour-singlet polynomials are R[G] + d R[G] with G = q q^T (six algebraically independent entries) and d = det q,
d^2 = det G (first fundamental theorem for SO(3) on three vectors). Long-axis charge: G_00, s = G_11 + G_22 and d
carry 0; w+- = G_01 +- i G_02 carry +-1; v+- = G_11 - G_22 +- 2i G_12 carry +-2. The row-2 flip combined with a colour
reflection preserves H and d and acts as complex conjugation on the charges, so Re and Im parts of charge-m monomials
are separate real sectors (parity + / -).

Basis: real polynomials f(G, d) times exp(-1/2 sum_k alpha_k |q_k|^2). Moments under exp(-sum_k alpha_k |q_k|^2)
reduce, by q_k = r_k / sqrt(alpha_k), to isotropic moments, computed exactly by integration by parts with
    grad G_kl . grad G_mn = d_km G_ln + d_kn G_lm + d_lm G_kn + d_ln G_km,  grad G_kl . grad d = 2 d_kl d,
    |grad d|^2 = 1/2 [(tr G)^2 - tr G^2],  Lap G_kl = 6 d_kl,  Lap d = 0.
"""
from __future__ import annotations

import itertools
from fractions import Fraction as F
from functools import lru_cache

import mpmath as mp

import ym_helike_j0 as j0

GKEYS = [(0, 0), (1, 1), (2, 2), (0, 1), (0, 2), (1, 2)]
GIDX = {k: i for i, k in enumerate(GKEYS)}
ND = 6                      # index of the d exponent


def gidx(k, l):
    return GIDX[(min(k, l), max(k, l))]


def mono(**kw):
    e = [0] * 7
    for k, v in kw.items():
        e[int(k)] = v
    return tuple(e)


def unit(i):
    e = [0] * 7
    e[i] = 1
    return tuple(e)


ONE = (0,) * 7


def padd(a, b, s=1):
    out = dict(a)
    for k, v in b.items():
        nv = out.get(k, 0) + s * v
        if nv:
            out[k] = nv
        else:
            out.pop(k, None)
    return out


def det_G():
    """det G as a polynomial in the six entries."""
    g = {k: unit(GIDX[k]) for k in GKEYS}
    p = {}
    for (terms, c) in [(((0, 0), (1, 1), (2, 2)), 1), (((0, 1), (1, 2), (0, 2)), 2), (((0, 0), (1, 2), (1, 2)), -1),
                       (((1, 1), (0, 2), (0, 2)), -1), (((2, 2), (0, 1), (0, 1)), -1)]:
        e = [0] * 7
        for t in terms:
            e[GIDX[t]] += 1
        p = padd(p, {tuple(e): F(c)})
    return p


DETG = det_G()


def pmul(a, b):
    out = {}
    for ka, va in a.items():
        for kb, vb in b.items():
            e = tuple(x + y for x, y in zip(ka, kb))
            if e[ND] >= 2:                               # d^2 = det G
                base = list(e)
                base[ND] -= 2
                out = padd(out, pmul({tuple(base): va * vb}, DETG))
            else:
                out = padd(out, {e: va * vb})
    return out


def var(i):
    return {unit(i): F(1)}


def gvar(k, l):
    return var(gidx(k, l))


D = var(ND)
TRG = padd(padd(gvar(0, 0), gvar(1, 1)), gvar(2, 2))


def trG2():
    p = {}
    for k in range(3):
        for l in range(3):
            p = padd(p, pmul(gvar(k, l), gvar(k, l)))
    return p


GRAD_DD = {k: v / 2 for k, v in padd(pmul(TRG, TRG), trG2(), -1).items()}


def grad_dot_var(i, j):
    """grad x_i . grad x_j for x in (G entries, d)."""
    if i == ND and j == ND:
        return GRAD_DD
    if ND in (i, j):
        k, l = GKEYS[j if i == ND else i]
        return {unit(ND): F(2)} if k == l else {}
    (k, l), (m, n) = GKEYS[i], GKEYS[j]
    out = {}
    for (a, b, c, dd) in [(k, m, l, n), (k, n, l, m), (l, m, k, n), (l, n, k, m)]:
        if a == b:
            out = padd(out, gvar(c, dd))
    return out


def lap_var(i):
    if i == ND:
        return {}
    k, l = GKEYS[i]
    return {ONE: F(6)} if k == l else {}


def pderiv(p, i):
    out = {}
    for e, c in p.items():
        if e[i]:
            f = list(e)
            f[i] -= 1
            out = padd(out, {tuple(f): c * e[i]})
    return out


def grad_dot(p, q):
    out = {}
    for i in range(7):
        pi = pderiv(p, i)
        if not pi:
            continue
        for j in range(7):
            qj = pderiv(q, j)
            if qj:
                out = padd(out, pmul(pmul(pi, qj), grad_dot_var(i, j)))
    return out


def laplacian(p):
    out = {}
    for i in range(7):
        pi = pderiv(p, i)
        if not pi:
            continue
        out = padd(out, pmul(pi, lap_var(i)))
        for j in range(7):
            pij = pderiv(pi, j)
            if pij:
                out = padd(out, pmul(pij, grad_dot_var(i, j)))
    return out


def row_degree(e, k):
    n = e[ND]
    for i, (a, b) in enumerate(GKEYS):
        n += e[i] * ((a == k) + (b == k))
    return n


@lru_cache(maxsize=None)
def iso_moment(e):
    """Exact <monomial> under exp(-|r|^2) on R^9, <1> = 1."""
    if sum(e[:ND]) * 2 + 3 * e[ND] == 0:
        return F(1)
    if e[ND]:
        rest = list(e)
        rest[ND] -= 1
        return sum((c * iso_moment(m) for m, c in grad_dot({tuple(rest): F(1)}, D).items()), F(0)) / 6
    i = next(i for i in range(ND) if e[i])
    rest = list(e)
    rest[i] -= 1
    rest = tuple(rest)
    k, l = GKEYS[i]
    s = sum((c * iso_moment(m) for m, c in grad_dot({rest: F(1)}, var(i)).items()), F(0))
    if k == l:
        s += 6 * iso_moment(rest)
    return s / 4


def charge_generators():
    """Complex generators as {G-monomial: complex Fraction pair (re, im)} polynomials, with charges."""
    def cplx(p, im=None):
        out = {k: (v, F(0)) for k, v in p.items()}
        for k, v in (im or {}).items():
            r, ii = out.get(k, (F(0), F(0)))
            out[k] = (r, ii + v)
        return out
    G = gvar
    return {
        "z": (0, cplx(G(0, 0))), "s": (0, cplx(padd(G(1, 1), G(2, 2)))), "d": (0, cplx(D)),
        "w+": (1, cplx(G(0, 1), G(0, 2))), "w-": (-1, cplx(G(0, 1), {k: -v for k, v in G(0, 2).items()})),
        "v+": (2, cplx(padd(G(1, 1), G(2, 2), -1), {k: 2 * v for k, v in G(1, 2).items()})),
        "v-": (-2, cplx(padd(G(1, 1), G(2, 2), -1), {k: -2 * v for k, v in G(1, 2).items()})),
    }


def cmul(a, b):
    out = {}
    for ka, (ar, ai) in a.items():
        for kb, (br, bi) in b.items():
            prod_r = pmul({ka: F(1)}, {kb: F(1)})
            for k, v in prod_r.items():
                r, i = out.get(k, (F(0), F(0)))
                out[k] = (r + v * (ar * br - ai * bi), i + v * (ar * bi + ai * br))
    return {k: v for k, v in out.items() if v != (0, 0)}


def sector_basis(N, m, parity):
    """Real basis polynomials (as {monomial: Fraction}) for long-axis charge m >= 0 and parity +1 (Re) / -1 (Im)."""
    gens = charge_generators()
    names = ["z", "s", "w+", "w-", "v+", "v-"]
    out, seen = [], set()
    maxe = N // 2
    for ex in itertools.product(range(maxe + 1), repeat=6):
        if 2 * sum(ex) > N:
            continue
        q = sum(e * gens[n][0] for e, n in zip(ex, names))
        for ed in (0, 1):
            if 2 * sum(ex) + 3 * ed > N or q != m:
                continue
            key = ex + (ed,)
            conj = (ex[0], ex[1], ex[3], ex[2], ex[5], ex[4], ed)
            if m == 0 and conj in seen:
                continue
            seen.add(key)
            p = {ONE: (F(1), F(0))}
            for e, n in zip(ex, names):
                for _ in range(e):
                    p = cmul(p, gens[n][1])
            if ed:
                p = cmul(p, gens["d"][1])
            part = {k: (v[0] if parity > 0 else v[1]) for k, v in p.items()}
            part = {k: v for k, v in part.items() if v}
            if part:
                out.append(part)
    return out


def _mpf(x):
    return mp.mpf(x.numerator) / x.denominator


def solve(N, g2, t, m=0, parity=1, alpha=None, dps=50, nlev=6):
    """Energies and full-space residuals in sector (m, parity) at cutoff N (polynomial degree in q)."""
    with mp.workdps(dps):
        st = mp.sqrt(mp.mpf(t))
        om = [st, 1 / st, 1 / st]
        al = [mp.mpf(a) for a in (alpha or om)]
        g = mp.sqrt(mp.mpf(g2))
        stot = sum(om)
        basis = [{k: _mpf(v) for k, v in b.items()} for b in sector_basis(N, m, parity)]
        n = len(basis)

        # build H f exactly per monomial, then combine
        cache = {}

        def h_mono(e):
            if e in cache:
                return cache[e]
            p = {e: F(1)}
            out = {}

            def acc(q, c):
                for k, v in q.items():
                    out[k] = out.get(k, 0) + c * _mpf(v)
            acc(laplacian(p), mp.mpf(-.5))
            acc(p, sum(al[k] * row_degree(e, k) for k in range(3)) + 1.5 * sum(al))
            for k in range(3):
                acc(pmul(p, gvar(k, k)), (om[k] ** 2 - al[k] ** 2) / 2)
            acc(pmul(p, D), -g * stot)
            acc(pmul(p, GRAD_DD), g * g / 2)
            cache[e] = out
            return out

        hs = []
        for b in basis:
            h = {}
            for e, c in b.items():
                for k, v in h_mono(e).items():
                    h[k] = h.get(k, 0) + c * v
            hs.append(h)
        mcache = {}

        def M(e):
            v = mcache.get(e)
            if v is None:
                scale = mp.mpf(1)
                for k in range(3):
                    scale *= al[k] ** (-mp.mpf(row_degree(e, k)) / 2)
                v = mcache[e] = scale * _mpf(iso_moment(e))
            return v

        def ip(a, b):
            tot = mp.mpf(0)
            for ka, va in a.items():
                for kb, vb in b.items():
                    for k, v in pmul({ka: F(1)}, {kb: F(1)}).items():
                        tot += va * vb * _mpf(v) * M(k)
            return tot

        S, Hm, K = mp.matrix(n, n), mp.matrix(n, n), mp.matrix(n, n)
        for i in range(n):
            for j in range(i, n):
                S[i, j] = S[j, i] = ip(basis[i], basis[j])
                hij, hji = ip(basis[i], hs[j]), ip(basis[j], hs[i])
                if i != j and abs(hij - hji) > max(abs(hij), 1) * mp.mpf(10) ** (-(dps - 15)):
                    raise ArithmeticError(f"H not symmetric at ({i},{j})")
                Hm[i, j] = Hm[j, i] = (hij + hji) / 2
                K[i, j] = K[j, i] = ip(hs[i], hs[j])
        out = j0.finish(S, Hm, K, dps, nlev)
    out.update({"N": N, "g2": float(g2), "t": float(t), "m": m, "parity": parity, "dimension": n})
    return out
