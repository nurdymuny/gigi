#!/usr/bin/env python3
"""H8 Step 3: ball-arithmetic certification of the projected-Gaussian 2x2 pencil.

Usage:  python scripts/h8_step3_certify.py OUT.json [--g2 0.002,0.004,...] [--tol 5e-4] [--max-boxes N]

Setting (docs/research/borrowed-routes-20261128/H8-step3-closure.md): t = 3, s = 1, alpha = 1.
Trial space span{phi0, psi1}, psi1 = colour average of phi = exp(-1/2 (q-a)^T A (q-a)),
A = J + 2 w_C e e^T + alpha P_ker  (= |J| + alpha P_ker), a = diag(u,1,1)/g.

Every number below is a python-flint `arb` ball. Every operation is ball arithmetic.
The two SO(3) integrals are reduced (exactly) to a 2-dimensional integral over the
Euler-angle rectangle (beta, gamma) in [0, pi] x [-pi, pi] (conjugation invariance under the
circle, see the note), covered by dyadic boxes; on each box the integral is enclosed by the
midpoint Taylor polynomial (degree 2, point balls) plus a degree-4 Lagrange remainder whose
coefficients are enclosed over the whole box by ball-evaluated Taylor arithmetic. Nothing is
left unenclosed: the boxes partition the full rectangle.

Pencil test (square-root free): M > 0 (M11 > 0, det M > 0), K11 - U M11 < 0, det(K - U M) > 0
with U = 274/(30 sqrt 3). Then mu_2 < U. The certified upper end mu* of mu_2 is checked by the
same test with U replaced by mu*.
"""
import sys, json, time, hashlib, heapq, math, os, random

from flint import arb, arb_mat, ctx

ctx.prec = 64

# ----------------------------------------------------------------------------------------
# Taylor models in the two box variables (db, dg) in [-rb, rb] x [-rg, rg].
# A Taylor model X = (P, r) means: for EVERY point of the box, ||X(db,dg) - P(db,dg)||_F <= r,
# where P is a polynomial of total degree <= n whose coefficients are balls computed at the box
# midpoint, and r is the upper end of a ball (all bound arithmetic is ball arithmetic on
# nonnegative quantities, so the upper end of the result bounds the true quantity).
# Coefficients are arb (scalars) or arb_mat (matrices / column vectors); None means 0.
# ----------------------------------------------------------------------------------------
MAXN = 6
MON = [(i, d - i) for d in range(MAXN + 1) for i in range(d, -1, -1)]
IDX = {m: k for k, m in enumerate(MON)}
DEG = [i + j for (i, j) in MON]
NM = {n: (n + 1) * (n + 2) // 2 for n in range(MAXN + 1)}
PAIRS = []
for (p, q) in MON:
    PAIRS.append([(IDX[(i, j)], IDX[(p - i, q - j)]) for i in range(p + 1) for j in range(q + 1)])


class _Ctx:
    n = 0
    rp = [arb(1)]


CX = _Ctx()


def set_ctx(n, rb, rg):
    """Order n and box half-widths rb, rg (balls; their upper ends are used in bounds)."""
    CX.n = n
    rbu, rgu = arb(rb.upper()), arb(rg.upper())
    CX.rp = [rbu ** i * rgu ** j for (i, j) in MON[:NM[n]]]


def cnorm(x):
    """Upper bound (as a ball whose upper end bounds) of |x| or ||x||_F."""
    if isinstance(x, arb):
        return x.abs_upper()
    return (x.transpose() * x).trace().abs_upper().sqrt() if x.nrows() >= x.ncols() else \
        (x * x.transpose()).trace().abs_upper().sqrt()


def s_mul(X, Y, n):
    out = []
    for o in range(NM[n]):
        acc = None
        for (i, j) in PAIRS[o]:
            xi, yj = X[i], Y[j]
            if xi is None or yj is None:
                continue
            t = xi * yj
            acc = t if acc is None else acc + t
        out.append(acc)
    return out


def _padd(X, Y):
    return [y if x is None else (x if y is None else x + y) for x, y in zip(X, Y)]


class TM:
    __slots__ = ("P", "r", "_nd")

    def __init__(self, P, r):
        self.P = P
        self.r = r
        self._nd = None

    def nd(self):
        if self._nd is None:
            nd = [arb(0)] * (CX.n + 1)
            for k, x in enumerate(self.P):
                if x is not None:
                    nd[DEG[k]] = nd[DEG[k]] + cnorm(x) * CX.rp[k]
            self._nd = nd
        return self._nd

    def bar(self):            # bound of ||X|| over the box
        return sum(self.nd(), arb(0)) + self.r


ZERO = arb(0)


def tconst(X):
    return TM([X] + [None] * (NM[CX.n] - 1), ZERO)


def tadd(X, Y):
    return TM(_padd(X.P, Y.P), X.r + Y.r)


def tneg(X):
    return TM([None if x is None else -x for x in X.P], X.r)


def tsub(X, Y):
    return tadd(X, tneg(Y))


def taddc(X, v):              # X + constant v
    P = list(X.P)
    P[0] = v if P[0] is None else P[0] + v
    return TM(P, X.r)


def tscale(X, c):             # c a scalar ball
    return TM([None if x is None else x * c for x in X.P], X.r * c.abs_upper())


def tlmul(K, X, nK=None):     # constant K (left) times X
    if nK is None:
        nK = cnorm(K)
    return TM([None if x is None else K * x for x in X.P], X.r * nK)


def trmul(X, K, nK=None):
    if nK is None:
        nK = cnorm(K)
    return TM([None if x is None else x * K for x in X.P], X.r * nK)


def tT(X):
    return TM([None if x is None else x.transpose() for x in X.P], X.r)


def ttr(X):                   # |tr E| <= sqrt(dim) ||E||_F
    dim = X.P[0].nrows() if X.P[0] is not None else 9
    return TM([None if x is None else x.trace() for x in X.P], X.r * arb(dim).sqrt())


def tentry(X, i, j):
    return TM([None if x is None else x[i, j] for x in X.P], X.r)


def tmul(X, Y):
    n = CX.n
    P = s_mul(X.P, Y.P, n)
    nx, ny = X.nd(), Y.nd()
    over = arb(0)
    for d1 in range(n + 1):
        for d2 in range(n + 1):
            if d1 + d2 > n:
                over = over + nx[d1] * ny[d2]
    Xb, Yb = sum(nx, arb(0)), sum(ny, arb(0))
    return TM(P, over + Xb * Y.r + X.r * Yb + X.r * Y.r)


class TMFail(Exception):
    pass


def tinv_logdet(X):
    """Z = X^{-1} and L = log det X as Taylor models.  X = P0 (I + Y), Y = Z0 (X - P0),
    Z0 = P0^{-1}; requires sup ||Y||_F < 1 on the box (else TMFail). Neumann series to order n
    plus the tail bound Ybar^{n+1}/(1 - Ybar); log det(I+Y) = tr log(I+Y) likewise."""
    n = CX.n
    P0 = X.P[0]
    Z0 = P0.inv()
    D = TM([None] + list(X.P[1:]), X.r)
    nZ0 = cnorm(Z0)
    Y = tlmul(Z0, D, nZ0)
    Ybar = Y.bar()
    if not (Ybar < 1):
        raise TMFail("Neumann bound >= 1")
    I9 = arb_mat(P0.nrows(), P0.nrows())
    for i in range(P0.nrows()):
        I9[i, i] = 1
    S = tconst(I9)
    for _ in range(n):
        S = tsub(tconst(I9), tmul(Y, S))
    tail = Ybar ** (n + 1) / (1 - Ybar)
    S = TM(S.P, S.r + tail)
    Z = trmul(S, Z0, nZ0)
    # log det
    D0 = P0.det()
    if not (D0 > 0):
        raise TMFail("det not positive")
    if n >= 1:
        T = tscale(tconst(I9), arb(1) / n)
        for k in range(n - 1, 0, -1):
            T = tsub(tscale(tconst(I9), arb(1) / k), tmul(Y, T))
        T = tmul(Y, T)
        Ltr = ttr(T)
    else:
        Ltr = tconst(arb(0))
    ltail = 3 * Ybar ** (n + 1) / ((n + 1) * (1 - Ybar))
    L = taddc(TM(Ltr.P, Ltr.r + ltail), D0.log())
    return Z, L


def texp(F):
    n = CX.n
    e0 = F.P[0].exp()
    G = TM([None] + list(F.P[1:]), F.r)
    Gb = G.bar()
    S = tconst(arb(1))
    for k in range(n, 0, -1):
        S = taddc(tscale(tmul(G, S), arb(1) / k), arb(1))
    fact = arb(1)
    for k in range(1, n + 2):
        fact = fact * k
    tail = Gb ** (n + 1) / fact * Gb.exp()
    return tscale(TM(S.P, S.r + tail), e0)


def ttrig(v, var):
    """sin and cos Taylor models of (v + d_var), var 0: beta, 1: gamma; v a point ball.
    Lagrange remainder rho^{n+1}/(n+1)!."""
    n = CX.n
    s, c = v.sin(), v.cos()
    ders_s = [s, c, -s, -c]
    ders_c = [c, -s, -c, s]
    S = [None] * NM[n]
    C = [None] * NM[n]
    fact = arb(1)
    for k in range(n + 1):
        if k > 0:
            fact = fact * k
        mon = (k, 0) if var == 0 else (0, k)
        S[IDX[mon]] = ders_s[k % 4] / fact
        C[IDX[mon]] = ders_c[k % 4] / fact
    fact = fact * (n + 1)
    rho = CX.rp[IDX[(1, 0)]] if (var == 0 and n >= 1) else (CX.rp[IDX[(0, 1)]] if n >= 1 else arb(0))
    rem = rho ** (n + 1) / fact
    return TM(S, rem), TM(C, rem)


def tmat3(E):
    """3x3 matrix Taylor model from scalar Taylor models E[i][j] (None = 0)."""
    n = CX.n
    P = []
    for o in range(NM[n]):
        ent = [[None if E[i][j] is None else E[i][j].P[o] for j in range(3)] for i in range(3)]
        if all(x is None for row in ent for x in row):
            P.append(None)
        else:
            P.append(arb_mat(3, 3, [arb(0) if x is None else x for row in ent for x in row]))
    r2 = arb(0)
    for i in range(3):
        for j in range(3):
            if E[i][j] is not None:
                r2 = r2 + E[i][j].r * E[i][j].r
    return TM(P, r2.sqrt())


def tvec81(X):                # reshape a 9x9 model to 81x1 (Frobenius norm unchanged)
    return TM([None if x is None else arb_mat(81, 1, x.entries()) for x in X.P], X.r)


# ----------------------------------------------------------------------------------------
# Exact constants (balls)
# ----------------------------------------------------------------------------------------
PI = arb.pi()
SQ3 = arb(3).sqrt()
SQ11 = arb(11).sqrt()
U_RT3 = 1 / SQ3
OM = [SQ3, U_RT3, U_RT3]                     # omega = (sqrt t, 1/sqrt t, 1/sqrt t), t = 3
AP = [U_RT3, arb(1), arb(1)]                 # a' = (u, 1, 1)
WC = (SQ11 - SQ3) / 2                        # omega_C
RR = (SQ3 + SQ11) / 4                        # competitor ratio r
U_TARGET = arb(274) / (30 * SQ3)             # 274/(30 sqrt 3)
TWO_PI_9_2 = (2 * PI) ** 4 * (2 * PI).sqrt()  # (2 pi)^{9/2}


def idx(k, c):
    return 3 * k + c


def cof_terms():
    T = {}
    for k in range(3):
        for c in range(3):
            k1, k2 = (k + 1) % 3, (k + 2) % 3
            c1, c2 = (c + 1) % 3, (c + 2) % 3
            T[idx(k, c)] = [(1, idx(k1, c1), idx(k2, c2)), (-1, idx(k1, c2), idx(k2, c1))]
    return T


CT = cof_terms()

# Cof (9x81): cof(x)_i = sum_t s x_p x_q = Cof . vec(x x^T); also E cof(z) = Cof . vec(Sigma)
COF = arb_mat(9, 81)
for i in range(9):
    for (s, p, q) in CT[i]:
        COF[i, 9 * p + q] = COF[i, 9 * p + q] + s
# L^{(l)} with D cof(x) = sum_l x_l L^{(l)}
LMATS = [arb_mat(9, 9) for _ in range(9)]
for i in range(9):
    for (s, p, q) in CT[i]:
        LMATS[q][i, p] = LMATS[q][i, p] + s
        LMATS[p][i, q] = LMATS[p][i, q] + s
# T (81x81): E|cof(z)|^2 = vec(Sigma)^T T vec(Sigma)  (Isserlis)
TQ = arb_mat(81, 81)
for i in range(9):
    for (s1, p1, q1) in CT[i]:
        for (s2, p2, q2) in CT[i]:
            ss = s1 * s2
            for (r_, c_) in [(9 * p1 + q1, 9 * p2 + q2), (9 * p1 + p2, 9 * q1 + q2), (9 * p1 + q2, 9 * q1 + p2)]:
                TQ[r_, c_] = TQ[r_, c_] + ss


def Lmat_point(x):           # D cof(x) for a point vector (list of arb)
    Lm = arb_mat(9, 9)
    for l in range(9):
        Lm = Lm + x[l] * LMATS[l]
    return Lm


def diagvec(v):
    return [v[k] if k == c else arb(0) for k in range(3) for c in range(3)]


OMEGA = arb_mat(9, 9)
for k in range(3):
    for c in range(3):
        OMEGA[idx(k, c), idx(k, c)] = OM[k]
# J = Hess W(a) = Omega - g D cof(a) = Omega - D cof(diag a')   (g-free)
JMAT = OMEGA - Lmat_point(diagvec(AP))

# competitor eigenvector e (unit) and colour-tangent kernel vectors
_nrm = (1 + 2 * RR * RR).sqrt()
E_C = arb_mat(9, 1)
E_C[idx(0, 0), 0] = 1 / _nrm
E_C[idx(1, 1), 0] = RR / _nrm
E_C[idx(2, 2), 0] = RR / _nrm
KERS = []
for (i, j) in [(0, 1), (0, 2), (1, 2)]:
    v = arb_mat(9, 1)
    v[idx(i, j), 0] = AP[i]
    v[idx(j, i), 0] = -AP[j]
    KERS.append(v)


def amat(alpha):
    A = JMAT + (2 * WC) * (E_C * E_C.transpose())
    for k in KERS:
        nk = (k.transpose() * k)[0, 0]
        A = A + (alpha / nk) * (k * k.transpose())
    return A


def contains_zero_mat(X):
    return all(X[i, j].contains(0) for i in range(X.nrows()) for j in range(X.ncols()))


def max_rad(X):
    return max(float(X[i, j].rad()) for i in range(X.nrows()) for j in range(X.ncols()))


def structure_checks(A, alpha):
    """Consistency checks of the Hessian eigenstructure in ball arithmetic, plus the
    rigorous facts actually used: A > 0 (Sylvester minors) and [A, L_phi] = 0."""
    out = {}
    # explicit eigenvectors of J (closure note 4.1)
    ev = []
    def vec(entries):
        v = arb_mat(9, 1)
        for (kc, val) in entries:
            v[kc, 0] = val
        return v
    ev.append(("comp", -WC, vec([(0, arb(1)), (4, RR), (8, RR)])))
    ev.append(("mu+", (SQ3 + SQ11) / 2, vec([(0, 2 * RR), (4, arb(-1)), (8, arb(-1))])))
    ev.append(("2u_diag", 2 * U_RT3, vec([(4, arb(1)), (8, arb(-1))])))
    ev.append(("4/rt3_12", 4 / SQ3, vec([(idx(0, 1), SQ3), (idx(1, 0), arb(1))])))
    ev.append(("4/rt3_13", 4 / SQ3, vec([(idx(0, 2), SQ3), (idx(2, 0), arb(1))])))
    ev.append(("2u_23", 2 * U_RT3, vec([(idx(1, 2), arb(1)), (idx(2, 1), arb(1))])))
    for k_i, k in enumerate(KERS):
        ev.append(("ker%d" % k_i, arb(0), k))
    res = {}
    for (name, lam, v) in ev:
        R = JMAT * v - lam * v
        res[name] = {"eigenvalue": str(lam), "residual_contains_0": contains_zero_mat(R), "residual_rad": max_rad(R)}
    out["J_eigenvectors"] = res
    # orthogonality of the 9 eigenvectors
    V = [v for (_, _, v) in ev]
    orth = all((V[i].transpose() * V[j])[0, 0].contains(0) for i in range(9) for j in range(i + 1, 9))
    out["eigenvectors_mutually_orthogonal"] = orth
    # A > 0 by Sylvester: all leading principal minors certified positive
    minors = []
    for kk in range(1, 10):
        sub = arb_mat(kk, kk, [A[i, j] for i in range(kk) for j in range(kk)])
        minors.append(sub.det())
    out["A_leading_minors_lower"] = [float(m.lower()) for m in minors]
    out["A_positive_definite_certified"] = all(m > 0 for m in minors)
    # [A, L_phi] = 0 at a sample angle (exact identity by the note 4.5; ball check)
    ph = arb(7) / 10
    R = [[arb(1), arb(0), arb(0)], [arb(0), ph.cos(), -ph.sin()], [arb(0), ph.sin(), ph.cos()]]
    LR = arb_mat(9, 9, [R[k][l] * R[c][d] for k in range(3) for c in range(3) for l in range(3) for d in range(3)])
    out["A_commutes_with_L_phi_ball"] = contains_zero_mat(LR * A - A * LR)
    # trace check: 1/2 tr|J| = 3S/2 + w_C
    absJ = JMAT + (2 * WC) * (E_C * E_C.transpose())
    out["half_tr_absJ"] = str(absJ.trace() / 2)
    out["harmonic_3S/2+wC"] = str(15 / (2 * SQ3) + WC)
    return out


# ----------------------------------------------------------------------------------------
# Gaussian pair formula (closure 6.2) in shifted coordinates x = q - a, with
# grad W(a + x) = J x - g cof(x)  (exact: W cubic, grad W(a) = 0).
# ----------------------------------------------------------------------------------------
NCOF = cnorm(COF)
NTQ = cnorm(TQ)
NJ = cnorm(JMAT)
NL = sum((cnorm(L) ** 2 for L in LMATS), arb(0)).sqrt()     # ||sum_l e_l L^(l)||_F <= |e| NL


def potential_tm(m, Sig, g):
    """E_{x ~ N(m, Sig)} V0 = 1/2 E|J x - g cof(x)|^2
       = 1/2 [ |r|^2 - 2 g r.E cof(z) + tr(G^T G Sig) + g^2 E|cof z|^2 ],
       r = J m - g cof(m), G = J - g Dcof(m), z ~ N(0, Sig)  (Isserlis; odd moments vanish)."""
    mmT = tmul(m, tT(m))
    cofm = tlmul(COF, tvec81(mmT), NCOF)
    r = tsub(tlmul(JMAT, m, NJ), tscale(cofm, g))
    LP = []
    for X in m.P:
        if X is None:
            LP.append(None)
            continue
        acc = arb_mat(9, 9)
        for l in range(9):
            acc = acc + X[l, 0] * LMATS[l]
        LP.append(acc)
    Lm = TM(LP, m.r * NL)
    G = taddc(tscale(Lm, -g), JMAT)
    vecS = tvec81(Sig)
    Ecof = tlmul(COF, vecS, NCOF)
    rr = tentry(tmul(tT(r), r), 0, 0)
    rE = tentry(tmul(tT(r), Ecof), 0, 0)
    trG = ttr(tmul(tmul(tT(G), G), Sig))
    Q = tentry(tmul(tT(vecS), tlmul(TQ, vecS, NTQ)), 0, 0)
    tot = tadd(tadd(rr, tscale(rE, -2 * g)), tadd(trG, tscale(Q, g * g)))
    return tscale(tot, arb(1) / 2)


def pair_tm(A1, a1, A2, a2, aMC, g):
    """A1, a1 constant (arb_mat); A2, a2 Taylor models. Returns Taylor models (logN, h) with
    <G1,G2> = exp(logN), <G1, H G2> = <G1,G2> h (closure note 6.2):
      C = A1 + A2, Sigma = C^{-1}, d = a1 - a2, w = c - a2 = Sigma A1 d, c - a1 = w - d,
      kappa = d^T A1 Sigma A2 d = (A1 d)^T (d - w),
      kinetic = 1/2 [tr(A1 Sigma A2) + (c-a1)^T A1 A2 (c-a2)], tr(A1 Sigma A2) = tr A1 - tr(A1 Sigma A1),
      potential = E_{N(c,Sigma)} V0 with c - aMC = Sigma (A1 (a1 - aMC) + A2 (a2 - aMC))."""
    nA1 = cnorm(A1)
    C = taddc(A2, A1)
    Z, L = tinv_logdet(C)
    d = taddc(tneg(a2), a1)
    A1d = tlmul(A1, d, nA1)
    w = tmul(Z, A1d)
    m = tmul(Z, taddc(tmul(A2, taddc(a2, -aMC)), A1 * (a1 - aMC)))
    ca1 = tsub(w, d)
    kappa = tentry(tmul(tT(A1d), tsub(d, w)), 0, 0)
    kin1 = taddc(tneg(ttr(tlmul(A1, trmul(Z, A1, nA1), nA1))), A1.trace())
    kin2 = tentry(tmul(tT(ca1), tlmul(A1, tmul(A2, w), nA1)), 0, 0)
    kin = tscale(tadd(kin1, kin2), arb(1) / 2)
    pot = potential_tm(m, Z, g)
    h = tadd(kin, pot)
    logN = taddc(tscale(tadd(L, kappa), arb(-1) / 2), TWO_PI_9_2.log())
    return logN, h


def pair_point(A1, a1, A2, a2, aMC, g):
    set_ctx(0, arb(0), arb(0))
    logN, h = pair_tm(A1, a1, tconst(A2), tconst(a2), aMC, g)
    assert float(logN.r.upper()) == 0 and float(h.r.upper()) == 0
    return logN.P[0], h.P[0]


def kron_I3(O):              # 9x9 matrix of q -> q O^T (row-major vec): I3 (x) O
    M = arb_mat(9, 9)
    for k in range(3):
        for c in range(3):
            for dd in range(3):
                M[idx(k, c), idx(k, dd)] = O[c][dd]
    return M


SQRT3 = arb(3).sqrt()


class Model:
    def __init__(self, g2, alpha):
        self.g2 = arb(g2) if not isinstance(g2, arb) else g2
        self.g = self.g2.sqrt()
        self.alpha = arb(alpha)
        self.A = amat(self.alpha)
        self.a = arb_mat(9, 1, diagvec([x / self.g for x in AP]))
        self.U = U_TARGET

    def NO_point(self, O):
        """log N1(O) and h(O) at a point rotation O (3x3 list of arb) -- for cross-checks."""
        M = kron_I3(O)
        B = M.transpose() * self.A * M
        b = M.transpose() * self.a
        return pair_point(self.A, self.a, B, b, self.a, self.g)

    def integrand(self, bm, gm):
        """Taylor models (context order/box set by caller) of F1 = N1(O) sin(beta)/(4 pi),
        FG = (h - U) F1, FH = h F1 at O = R1(gamma) R2(beta), (beta, gamma) = (bm, gm) + box."""
        SB, CB = ttrig(bm, 0)
        SG, CG = ttrig(gm, 1)
        O = tmat3([[CB, None, SB],
                   [tmul(SG, SB), CG, tneg(tmul(SG, CB))],
                   [tneg(tmul(CG, SB)), SG, tmul(CG, CB)]])
        M = TM([None if X is None else kron_I3([[X[i, j] for j in range(3)] for i in range(3)]) for X in O.P],
               O.r * SQRT3)                       # ||I3 (x) E||_F = sqrt3 ||E||_F
        MT = tT(M)
        B = tmul(MT, tlmul(self.A, M))
        b = trmul(MT, self.a)
        logN, h = pair_tm(self.A, self.a, B, b, self.a, self.g)
        N1 = texp(logN)
        F1 = tscale(tmul(N1, SB), 1 / (4 * PI))
        FG = tmul(taddc(h, -self.U), F1)
        FH = tmul(h, F1)
        return F1, FG, FH


# ----------------------------------------------------------------------------------------
# Box enclosure of the integral over [beta_m +- rb] x [gamma_m +- rg]
# ----------------------------------------------------------------------------------------
PM1 = arb(0, 1)              # the interval [-1, 1]
ORDER = 4


def box_integral(model, sl, sh, tl, th, order=None):
    """Box in (s,t) with beta = pi s, gamma = pi t; s,t endpoints dyadic floats (exact).
    int_box F = vol * [ sum_{k1,k2 even} P_k rb^k1 rg^k2 / ((k1+1)(k2+1)) + [-r, r] ]."""
    n = ORDER if order is None else order
    sm, tm = (sl + sh) / 2, (tl + th) / 2
    bm, gm = PI * arb(sm), PI * arb(tm)
    rb, rg = PI * arb((sh - sl) / 2), PI * arb((th - tl) / 2)
    vol = 4 * rb * rg
    set_ctx(n, rb, rg)
    try:
        Fs = model.integrand(bm, gm)
    except (ZeroDivisionError, ValueError, TMFail):
        return None
    outs = []
    for F in Fs:
        val = arb(0)
        for k, c in enumerate(F.P):
            (p, q) = MON[k]
            if c is None:
                continue
            if p % 2 or q % 2:
                # odd monomial: the midpoint polynomial integrates to 0 exactly on the symmetric
                # box; the coefficient's own ball radius contributes at most rad(c) * sup|x^p y^q|.
                val = val + PM1 * (c.rad() * CX.rp[k])
                continue
            val = val + c * rb ** p * rg ** q / ((p + 1) * (q + 1))
        enc = vol * (val + PM1 * F.r.upper())
        if not enc.is_finite():
            return None
        outs.append(enc)
    return outs


def integrate_so3(model, tol, max_boxes, time_cap, log=print):
    """Adaptive dyadic cover of [0,1] x [0,1] in (s,t) (gamma >= 0 half, doubled). Returns enclosures of
    int F1, int FG, int FH (= M22, K22 - U M22, K22) and stats."""
    t0 = time.time()
    heap = []
    cnt = 0
    Uf = float(model.U.mid())
    def push(box):
        nonlocal cnt
        enc = box_integral(model, *box)
        cnt += 1
        if enc is None:
            score = float("inf")
        else:
            score = 2 * (float(enc[2].rad()) + Uf * float(enc[0].rad()) + float(enc[1].rad()))
        heapq.heappush(heap, (-score, cnt, box, enc))
    # gamma -> -gamma symmetry (conjugation by P = diag(1,-1,1), see the note): integrate
    # t in [0, 1] and double.
    ns, nt = 8, 8
    for i in range(ns):
        for j in range(nt):
            push((i / ns, (i + 1) / ns, j / nt, (j + 1) / nt))
    it = 0
    while True:
        it += 1
        if it % 50 == 0 or len(heap) >= max_boxes:
            tot = [arb(0), arb(0), arb(0)]
            fail = 0
            sc = 0.0
            for (negs, _, _, enc) in heap:
                if enc is None:
                    fail += 1
                    continue
                for k in range(3):
                    tot[k] += enc[k]
                sc += -negs
            M22 = abs(float(tot[0].mid()))     # half-domain values: ratio unaffected
            done = fail == 0 and M22 > 0 and sc <= tol * M22
            if it % 500 == 0:
                log("    boxes=%d evals=%d fail=%d score/M22=%.3e t=%.0fs" % (len(heap), cnt, fail, sc / M22 if M22 else float('inf'), time.time() - t0))
            if done or len(heap) >= max_boxes or time.time() - t0 > time_cap:
                break
        negs, _, box, enc = heapq.heappop(heap)
        sl, sh, tl, th = box
        # bisect the longer side (beta = pi s, gamma = pi t: same units)
        if (sh - sl) >= (th - tl):
            smid = (sl + sh) / 2
            push((sl, smid, tl, th)); push((smid, sh, tl, th))
        else:
            tmid = (tl + th) / 2
            push((sl, sh, tl, tmid)); push((sl, sh, tmid, th))
    tot = [arb(0), arb(0), arb(0)]
    fail = 0
    for (_, _, _, enc) in heap:
        if enc is None:
            fail += 1
            continue
        for k in range(3):
            tot[k] += enc[k]
    tot = [2 * x for x in tot]
    stats = {"boxes": len(heap), "evaluations": cnt, "failed_boxes": fail,
             "seconds": round(time.time() - t0, 1),
             "min_box_width_rad": min(math.pi * min(b[1] - b[0], b[3] - b[2]) for (_, _, b, _) in heap)}
    return tot, fail, stats


def bstr(x):
    return {"mid": float(x.mid()), "rad": float(x.rad()), "lower": float(x.lower()), "upper": float(x.upper()),
            "ball": x.str(20, radius=True)}


def certify(g2, alpha, tol, max_boxes, time_cap, log=print):
    t0 = time.time()
    mdl = Model(arb(g2) if isinstance(g2, str) else arb(str(g2)), alpha)
    A, a, g = mdl.A, mdl.a, mdl.g
    zero = arb_mat(9, 1)
    # (1,1): phi0, phi0 ; (1,2): phi0, phi ; via the same pair code
    l11, h11 = pair_point(OMEGA, zero, OMEGA, zero, a, g)
    l12, h12 = pair_point(OMEGA, zero, A, a, a, g)
    M11 = l11.exp(); K11 = M11 * h11
    M12 = l12.exp(); K12 = M12 * h12
    M11_closed = PI ** 4 * PI.sqrt() * arb(3).root(4) ** 3
    R11_closed = 5 * SQ3 / 2 + 15 * mdl.g2 / 4
    tot, fail, stats = integrate_so3(mdl, tol, max_boxes, time_cap, log)
    M22, XG, K22 = tot
    U = mdl.U
    res = {"g2": g2, "alpha": alpha, "U": bstr(U), "integration": stats,
           "M11": bstr(M11), "M12": bstr(M12), "M22": bstr(M22),
           "K11": bstr(K11), "K12": bstr(K12), "K22": bstr(K22),
           "K22_minus_U_M22_direct": bstr(XG),
           "closed_form_checks": {"M11_closed": bstr(M11_closed), "M11_overlaps": M11.overlaps(M11_closed),
                                  "K11/M11": bstr(h11), "K11/M11_closed": bstr(R11_closed),
                                  "K11/M11_overlaps": h11.overlaps(R11_closed)},
           "R_phi0": bstr(h11), "R_psi1": bstr(K22 / M22), "normalised_overlap": bstr(M12 / (M11 * M22).sqrt())}
    if fail:
        res["verdict"] = "NOT CERTIFIED (unenclosed boxes remain)"
        return res
    # pencil test at U (square-root free)
    detM = M11 * M22 - M12 * M12
    a11 = K11 - U * M11
    a12 = K12 - U * M12
    a22 = XG                                         # K22 - U M22, enclosed directly
    a22_alt = K22 - U * M22
    detKU = a11 * a22 - a12 * a12
    passU = bool(M11 > 0 and detM > 0 and a11 < 0 and a22 < 0 and detKU > 0)
    res["pencil_at_U"] = {"det_M": bstr(detM), "K11-U*M11": bstr(a11), "K12-U*M12": bstr(a12),
                          "K22-U*M22": bstr(a22), "K22-U*M22_from_separate": bstr(a22_alt),
                          "det(K-U*M)": bstr(detKU), "M_pos_def": bool(M11 > 0 and detM > 0),
                          "K-UM_negative_definite": passU}
    # Certified upper end of mu_2: the smallest mu* (on a 1e-5 grid, found by bisection) at which
    # the same square-root-free test certifies K - mu* M < 0, hence mu_2 < mu*.
    def negdef_at(mu):
        b11 = K11 - mu * M11
        b12 = K12 - mu * M12
        b22 = XG + (U - mu) * M22
        b22_alt = K22 - mu * M22
        if b22_alt.rad() < b22.rad():
            b22 = b22_alt
        detKm = b11 * b22 - b12 * b12
        ok = bool(M11 > 0 and detM > 0 and b11 < 0 and b22 < 0 and detKm > 0)
        return ok, b11, b22, detKm
    R2 = K22 / M22                          # mu_2 >= R(psi1) (Rayleigh quotient of psi1)
    lo_i = int(math.floor(float(R2.lower()) * 1e5))
    hi_i = int(math.ceil(float(U.upper()) * 1e5))
    mustar_f, best = None, None
    if passU:
        while hi_i - lo_i > 1:
            mid_i = (lo_i + hi_i) // 2
            ok = negdef_at(arb(mid_i) / 100000)
            if ok[0]:
                hi_i = mid_i
            else:
                lo_i = mid_i
        mustar = arb(hi_i) / 100000
        best = negdef_at(mustar)
        if best[0]:
            mustar_f = hi_i / 1e5
    res["mu2_lower_bound_R_psi1"] = bstr(R2)
    if best is not None:
        res["mu2_certified_upper"] = {"mu_star": mustar_f, "mu_star_exact": "%d/100000" % hi_i,
                                      "K11-mu*M11": bstr(best[1]), "K22-mu*M22": bstr(best[2]),
                                      "det(K-mu*M)": bstr(best[3]), "K-mu*M_negative_definite": best[0]}
        res["mu2_enclosure"] = [float(R2.lower()), mustar_f]
        res["margin_U_minus_mu_star"] = float((U - mustar).lower())
    res["verdict"] = "CERTIFIED: mu_2 < U" if passU else "NOT CERTIFIED"
    res["seconds"] = round(time.time() - t0, 1)
    return res


def conjugation_and_crosscheck(model, nsamp=3, seed=1):
    """Ball checks that N(R1(p1) R2(b) R1(p2)) = N(R1(p1+p2) R2(b)) = N(R1(-p1-p2) R2(b)) at random
    points (the exact identities are proved in the note; this is a consistency check)."""
    rnd = random.Random(seed)
    out = []
    def R1(x):
        return [[arb(1), arb(0), arb(0)], [arb(0), x.cos(), -x.sin()], [arb(0), x.sin(), x.cos()]]
    def R2(x):
        return [[x.cos(), arb(0), x.sin()], [arb(0), arb(1), arb(0)], [-x.sin(), arb(0), x.cos()]]
    def mm(X, Y):
        return [[sum((X[i][k] * Y[k][j] for k in range(3)), arb(0)) for j in range(3)] for i in range(3)]
    sc = float(model.g.mid())
    for _ in range(nsamp):
        p1, b, p2 = (arb(str(round(rnd.uniform(-2, 2) * sc, 6))) for _ in range(3))
        b = abs(b)
        O3 = mm(mm(R1(p1), R2(b)), R1(p2))
        O2 = mm(R1(p1 + p2), R2(b))
        l3, h3 = model.NO_point(O3)
        l2, h2 = model.NO_point(O2)
        lr, hr = model.NO_point(mm(R1(-(p1 + p2)), R2(b)))      # gamma -> -gamma reflection
        out.append({"angles": [float(p1.mid()), float(b.mid()), float(p2.mid())],
                    "logN1_3d": float(l3.mid()), "logN1_2d": float(l2.mid()), "logN1_2d_reflected": float(lr.mid()),
                    "h_3d": float(h3.mid()), "h_2d": float(h2.mid()), "h_2d_reflected": float(hr.mid()),
                    "overlap": bool(l3.overlaps(l2) and h3.overlaps(h2) and lr.overlaps(l2) and hr.overlaps(h2))})
    return out


def main():
    args = sys.argv[1:]
    if not args or args[0].startswith("-"):
        print(__doc__)
        sys.exit(2)
    outpath = args[0]
    g2list = ["0.002", "0.004", "0.03", "0.05", "0.07"]
    tol, max_boxes, time_cap, alpha = 3e-4, 6000, 600.0, 1
    i = 1
    while i < len(args):
        if args[i] == "--g2":
            g2list = args[i + 1].split(","); i += 2
        elif args[i] == "--tol":
            tol = float(args[i + 1]); i += 2
        elif args[i] == "--max-boxes":
            max_boxes = int(args[i + 1]); i += 2
        elif args[i] == "--time-cap":
            time_cap = float(args[i + 1]); i += 2
        else:
            raise SystemExit("unknown arg " + args[i])
    t0 = time.time()
    with open(os.path.abspath(__file__), "rb") as fh:
        sha = hashlib.sha256(fh.read()).hexdigest()
    rec = {"script": "scripts/h8_step3_certify.py", "script_sha256": sha,
           "python_flint": __import__("flint").__version__, "arb_prec_bits": ctx.prec,
           "setting": {"t": 3, "s": 1, "alpha": alpha, "U": "274/(30 sqrt 3)"},
           "taylor_model_order": ORDER,
           "params": {"tol": tol, "max_boxes": max_boxes, "time_cap_s": time_cap},
           "structure_checks": structure_checks(amat(arb(alpha)), alpha),
           "couplings": []}
    for g2 in g2list:
        print("g2 =", g2, flush=True)
        mdl = Model(arb(g2), alpha)
        cc = conjugation_and_crosscheck(mdl)
        res = certify(g2, alpha, tol, max_boxes, time_cap)
        res["conjugation_reduction_checks"] = cc
        rec["couplings"].append(res)
        print("  verdict:", res["verdict"], " mu2 in", res.get("mu2_enclosure"),
              " mu* =", res.get("mu2_certified_upper", {}).get("mu_star"), " stats:", res["integration"], flush=True)
        with open(outpath, "w") as fh:
            json.dump(rec, fh, indent=1)
    rec["all_certified"] = all(c["verdict"].startswith("CERTIFIED") for c in rec["couplings"])
    rec["runtime_s"] = round(time.time() - t0, 1)
    with open(outpath, "w") as fh:
        json.dump(rec, fh, indent=1)
    print("all certified:", rec["all_certified"], " runtime", rec["runtime_s"], "s")


if __name__ == "__main__":
    main()
