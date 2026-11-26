"""S_C stage R2.1 (a-c): rigorous monomial moments of the EXACT Robin base eigenfunctions of one block
(docs/research/helike-sc-R2-interval-lehmann-registration-2026-11-26.md, amendment 1). Arb, then midpoint-radius.

Block operator: -1/2 Lap + 1/2 w2 r^2 + c on the 3-ball of radius L, Robin R'(L) = beta R(L).
Eigenfunctions phi = R_{l}(r; E) Y_lm (real harmonics, the dryrun convention: index l^2 + l + m),
    R_l(r; E) = r^l exp(-w r^2/2) M(a, b, w r^2),  b = l + 3/2,  a = b/2 - (E - c)/(2 w),  w = sqrt(w2),
with E the R1.4 enclosure (a ball), so every bound holds for the true eigenvalue.

Radial moments  m_k(i, j) = int_0^L r^(2+k) R_i R_j dr
  = sum_n s_n w^n I(p0 + 2n),  s = c_i * c_j (Kummer coefficients),  p0 = l_i + l_j + 2 + k,
    I(p) = int_0^L r^p exp(-w r^2) dr = gamma_lower((p+1)/2, w L^2) / (2 w^((p+1)/2)),
  truncated at n = N per factor; for n >= N >= |a| the coefficient ratio is <= 2/(n+1) =: q, so on x in [0, X],
  X = w L^2, |M - M_N| <= |c_N| X^N (qX)/(1 - qX) (needs qX < 1), and the product error times I(p0) is added.
Angular: u^alpha = r^|alpha| uhat^alpha; uhat_k acting on Y_l^m is an exact finite combination of Y_{l+-1}; the real
angular matrices of uhat^alpha are exact products of these ladders (acb, transformed to real harmonics).
"""
import math

import numpy as np
from flint import acb, acb_mat, arb, ctx

ctx.prec = 128                               # the registered R1 precision
NSER = 160                                   # Kummer terms kept per factor


class KummerRadial:
    def __init__(self, l, E, w, c, L):
        self.l = l
        b = arb(l) + arb(3) / 2
        a = b / 2 - (E - c) / (2 * w)
        X = w * L * L
        if float(abs(a).upper()) > NSER:
            raise ValueError("series too short for this level")
        co = [arb(1)]
        for n in range(NSER):
            co.append(co[-1] * (a + n) / ((b + n) * (n + 1)))
        self.c = co
        q = arb(2) / (NSER + 1)
        if float((q * X).upper()) >= 1:
            raise ValueError("geometric tail bound needs q X < 1")
        self.eps = abs(co[-1]) * X ** NSER * (q * X) / (1 - q * X)     # sup |M - M_N| on [0, X]
        self.B = sum((abs(x) * X ** n for n, x in enumerate(co)), arb(0))  # sup |M_N| on [0, X]


class ExactBlock:
    def __init__(self, w2, L, beta, c, levels):
        """levels: list of (l, E enclosure) of the radial eigenfunctions to use."""
        self.w = arb(w2).sqrt(); self.L = arb(L); self.c = arb(c)
        self.levels = levels
        self.rad = [KummerRadial(l, E, self.w, self.c, self.L) for l, E in levels]
        pmax = 2 * max(l for l, _ in levels) + 2 + 8 + 4 * NSER + 2
        X = self.w * self.L * self.L
        self.I = [X.gamma_lower(arb(p + 1) / 2) / (2 * self.w ** (arb(p + 1) / 2)) for p in range(pmax + 1)]
        self.norm = [self._raw(i, i, 0).sqrt() for i in range(len(levels))]

    def _raw(self, i, j, k):
        A, B = self.rad[i], self.rad[j]
        p0 = A.l + B.l + 2 + k
        tot = arb(0); wn = arb(1)
        for n in range(2 * NSER + 1):
            lo, hi = max(0, n - NSER), min(n, NSER)
            s = sum((A.c[t] * B.c[n - t] for t in range(lo, hi + 1)), arb(0))
            tot += s * wn * self.I[p0 + 2 * n]
            wn *= self.w
        err = (A.eps * (B.B + B.eps) + A.B * B.eps) * self.I[p0]
        return tot + arb(0, float(err.upper()))

    def radial(self, i, j, k):
        """Normalized radial moment int r^(2+k) R_i R_j dr / (|R_i| |R_j|)."""
        return self._raw(i, j, k) / (self.norm[i] * self.norm[j])


# ------------------------------------------------------------------ angular ladders
def _cidx(l, m):
    return l * l + l + m


def ladder_matrices(lcap):
    """Complex-basis matrices (acb_mat) of uhat_x, uhat_y, uhat_z on Y_l^m, l <= lcap (Condon-Shortley).
    Entry [row (l', m'), col (l, m)] = coefficient of Y_l'^m' in uhat * Y_l^m; rows beyond lcap are dropped, so
    products are exact for inputs with l <= lcap - (number of factors)."""
    n = (lcap + 1) ** 2
    Z = [[acb(0)] * n for _ in range(n)]; P = [[acb(0)] * n for _ in range(n)]; Mm = [[acb(0)] * n for _ in range(n)]
    for l in range(lcap + 1):
        for m in range(-l, l + 1):
            col = _cidx(l, m)
            if l + 1 <= lcap:
                Z[_cidx(l + 1, m)][col] += arb(((l + 1) ** 2 - m * m)) .sqrt() / arb((2 * l + 1) * (2 * l + 3)).sqrt()
                P[_cidx(l + 1, m + 1)][col] += -(arb((l + m + 1) * (l + m + 2)) / arb((2 * l + 1) * (2 * l + 3))).sqrt()
                Mm[_cidx(l + 1, m - 1)][col] += (arb((l - m + 1) * (l - m + 2)) / arb((2 * l + 1) * (2 * l + 3))).sqrt()
            if l - 1 >= 0 and abs(m) <= l - 1:
                Z[_cidx(l - 1, m)][col] += (arb(l * l - m * m) / arb((2 * l - 1) * (2 * l + 1))).sqrt()
            if l - 1 >= 0 and abs(m + 1) <= l - 1:
                P[_cidx(l - 1, m + 1)][col] += (arb((l - m) * (l - m - 1)) / arb((2 * l - 1) * (2 * l + 1))).sqrt()
            if l - 1 >= 0 and abs(m - 1) <= l - 1:
                Mm[_cidx(l - 1, m - 1)][col] += -(arb((l + m) * (l + m - 1)) / arb((2 * l - 1) * (2 * l + 1))).sqrt()
    Zm, Pm, Mmm = acb_mat(Z), acb_mat(P), acb_mat(Mm)
    half = acb(arb(1) / 2)
    Xm = (Pm + Mmm) * half                                   # sin cos(phi) = (sin e^{i phi} + sin e^{-i phi}) / 2
    Ym = (Pm - Mmm) * acb(0, -arb(1) / 2)                    # sin sin(phi) = (... - ...) / (2 i)
    return Xm, Ym, Zm


def real_transform(lcap):
    """T with realY = T complexY in the dryrun convention:
    m = 0: Y_l^0; m > 0: sqrt2 (-1)^m Re Y_l^m; m < 0: sqrt2 (-1)^m Im Y_l^|m|."""
    n = (lcap + 1) ** 2
    T = [[acb(0)] * n for _ in range(n)]
    r2 = arb(2).sqrt()
    for l in range(lcap + 1):
        for m in range(-l, l + 1):
            row = _cidx(l, m)
            if m == 0:
                T[row][_cidx(l, 0)] = acb(1)
            else:
                k = abs(m); s = arb(-1) ** k
                # Y_l^{-k} = (-1)^k conj(Y_l^k):  Re Y^k = (Y^k + (-1)^k Y^{-k})/2,  Im Y^k = (Y^k - (-1)^k Y^{-k})/(2i)
                if m > 0:
                    T[row][_cidx(l, k)] = acb(r2 * s / 2)
                    T[row][_cidx(l, -k)] = acb(r2 * s * s / 2)
                else:
                    T[row][_cidx(l, k)] = acb(0, r2 * s / 2)        # sign fixed against the quadrature (y-ladder check)
                    T[row][_cidx(l, -k)] = acb(0, -r2 * s * s / 2)
    return acb_mat(T)


def angular_matrices(lmax, dmax=8):
    """Real angular matrices <Y_a, uhat^alpha Y_b>: rows l_a <= lmax + dmax - |alpha| (all needed rows), columns
    l_b <= lmax; dict alpha -> acb_mat of shape ((lmax+dmax+1)^2, (lmax+1)^2). Exact: each uhat_k changes l by +-1,
    so the cap lmax + dmax is never crossed. Built degree by degree, M^(alpha + e_k) = U_k M^alpha, in the real basis
    U_k = T X_k T^dagger (unitary T)."""
    lcap = lmax + dmax
    Xm, Ym, Zm = ladder_matrices(lcap)
    T = real_transform(lcap)
    Td = T.conjugate().transpose()
    U = [T * M * Td for M in (Xm, Ym, Zm)]
    Ncap, n = (lcap + 1) ** 2, (lmax + 1) ** 2
    E = acb_mat([[acb(1) if r == c else acb(0) for c in range(n)] for r in range(Ncap)])
    out = {(0, 0, 0): E}
    for d in range(1, dmax + 1):
        for i in range(d + 1):
            for j in range(d + 1 - i):
                k = d - i - j
                a = (i, j, k)
                # peel one factor from the first nonzero exponent
                if i:
                    out[a] = U[0] * out[(i - 1, j, k)]
                elif j:
                    out[a] = U[1] * out[(i, j - 1, k)]
                else:
                    out[a] = U[2] * out[(i, j, k - 1)]
    return out, n


def to_midrad(M, rows, cols):
    """acb_mat (real part) restricted to rows x cols -> (mid, rad) float arrays enclosing it."""
    mid = np.empty((len(rows), len(cols))); rad = np.empty_like(mid)
    for p, r in enumerate(rows):
        for q, c in enumerate(cols):
            x = M[r, c].real
            mid[p, q] = float(x.mid()); rad[p, q] = float(x.rad()) + abs(float(x.mid())) * 2.3e-16
    return mid, rad
