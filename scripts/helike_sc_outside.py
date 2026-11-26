"""S_C: the outside of the tube by rays from the origin
(docs/research/helike-sc-outside-bound-registration-2026-11-26.md). Arb at 128 bits.

O1 (near the ring, slice coordinates): P(z) >= tau = rho_C - V_bg for |z| <= R_n, z outside B_u(5) x B_v(2).
O2 (far, symmetric matrices): V(S) >= rho_C for symmetric S with dist(S, orbit) >= R_n.
usage: python scripts/helike_sc_outside.py --g2=0.004 [--o1] [--o2] [--budget=N]
"""
import heapq
import itertools
import math
import sys
import time
from fractions import Fraction

import numpy as np
from flint import arb

import helike_sc_r1 as r1
from helike_sc_r1 import Poly, sq

RN = 12                                       # amended from 8 (see the registration, O-amendment 1)
LU, LV = 5, 2
E0REL = {0.004: "5.40626", 0.002: "5.40917"}


def tau_C(M):
    """rho_C - V_bg = E0 - V_bg + 2 w_H - w_min/4 with w_H = w_min = sqrt(min w2) (recomputed; R1.9.2)."""
    wmin = min(M.w2, key=lambda x: float(x.mid())).sqrt()
    return arb(E0REL[M.g2key]) + 2 * wmin - wmin / 4


def quartic_min(q, p, c, s0, s1):
    """Rigorous lower bound of min over s in [s0, s1] (0 <= s0 <= s1) of f(s) = s^2 q + s^3 p + s^4 c (arb inputs)."""
    f = lambda s: s * s * (q + s * (p + s * c))
    cands = [f(arb(s0)), f(arb(s1))]
    # interior critical points: 4 c s^2 + 3 p s + 2 q = 0 (s = 0 is an endpoint candidate when s0 = 0)
    roots = []
    if float(abs(c).upper()) == 0.0:
        if float(abs(p).upper()) > 0:
            roots.append(-2 * q / (3 * p))
    else:
        disc = 9 * p * p - 32 * c * q
        if float(disc.upper()) >= 0:
            sd = arb(0).union(arb(disc.upper())).sqrt() if float(disc.lower()) < 0 else disc.sqrt()
            roots += [(-3 * p + sd) / (8 * c), (-3 * p - sd) / (8 * c)]
    for rt in roots:
        if not rt.is_finite():
            cands.append(f(arb(s0).union(arb(s1))))
            continue
        if float(rt.upper()) >= s0 and float(rt.lower()) <= s1:
            lo, hi = max(float(rt.lower()), s0), min(float(rt.upper()), s1)
            cands.append(f(arb(lo).union(arb(hi))))
    return math.nextafter(min(float(x.lower()) for x in cands), -math.inf)     # rounded down


def enclose(p, grads, X, ci, free):
    """(lower, upper) as exact arb endpoints: the better of the natural and mean-value forms on each side."""
    nat = p(X)
    mv = p(ci) + sum((grads[k](X) * (X[k] - ci[k]) for k in free), arb(0))
    lo = nat.lower() if float(nat.lower()) >= float(mv.lower()) else mv.lower()
    hi = nat.upper() if float(nat.upper()) <= float(mv.upper()) else mv.upper()
    return arb(lo), arb(hi)


def boxes_X(lo, hi):
    X = [arb((a + b) / 2, (b - a) / 2) if b > a else arb(a) for a, b in zip(lo, hi)]
    ci = [arb((a + b) / 2) for a, b in zip(lo, hi)]
    return X, ci, [k for k in range(len(lo)) if hi[k] > lo[k]]


def absmax(l, h):
    return max(abs(l), abs(h))


def absmin(l, h):
    return 0.0 if l <= 0 <= h else min(abs(l), abs(h))


# ------------------------------------------------------------------ O1
def o1(M, budget=400_000):
    tau = tau_C(M)
    parts = [M.P.part(d) for d in (2, 3, 4)]
    polys = [(p, [p.deriv(k) for k in range(6)]) for p in parts]

    def lb(lo, hi):
        X, ci, free = boxes_X(lo, hi)
        q, p, c = (enclose(pp, g, X, ci, free)[0] for pp, g in polys)
        c = c if float(c) > 0 else arb(0)
        uh = math.sqrt(sum(absmax(lo[k], hi[k]) ** 2 for k in range(3)))
        vh = math.sqrt(sum(absmax(lo[k], hi[k]) ** 2 for k in range(3, 6)))
        nl = math.sqrt(sum(absmin(lo[k], hi[k]) ** 2 for k in range(6)))
        s0 = min(LU / uh if uh > 0 else math.inf, LV / vh if vh > 0 else math.inf) * (1 - 1e-12)
        s1 = RN / nl * (1 + 1e-12)
        if s0 > s1:
            return math.inf                           # the whole checked segment lies beyond R_n
        return quartic_min(q, p, c, s0, s1) - float(tau.upper())

    return bnb(lb, r1.c3_symmetric_faces(), budget)


# ------------------------------------------------------------------ O2
def sym_model(M):
    r2 = arb(2).sqrt()
    x = [Poly.var(5, k) for k in range(5)]
    s11, d, m, a, b = x
    s22, s33, s12, s13 = (m + d) * (1 / r2), (m - d) * (1 / r2), a * (1 / r2), b * (1 / r2)
    z = Poly.const(5, 0)
    q = [[s11, s12, s13], [s12, s22, z], [s13, z, s33]]
    return r1.potential(q, M.g, M.om, M.src)


def sym_faces():
    out = []
    for face, sgn in ((0, 1), (0, -1), (2, 1), (2, -1), (1, 1), (3, 1), (4, 1)):
        lo, hi = [], []
        for k in range(5):
            if k == face:
                lo.append(float(sgn)); hi.append(float(sgn))
            elif k in (1, 3, 4):
                lo.append(0.0); hi.append(1.0)
            else:
                lo.append(-1.0); hi.append(1.0)
        out.append((lo, hi))
    return out


def sym_matrix(xs, r2):
    s11, d, m, a, b = xs
    return [[s11, a / r2, b / r2], [a / r2, (m + d) / r2, arb(0)], [b / r2, arb(0), (m - d) / r2]]


R0S = [np.diag(v) for v in ((1, 1, 1), (1, -1, -1), (-1, 1, -1), (-1, -1, 1))]


def exact_rotation(Rf):
    """An exactly orthogonal rotation near Rf: R0 * Cayley(K), K rational antisymmetric; returned as an arb matrix
    (balls containing the exact matrix)."""
    R0 = min(R0S, key=lambda A: np.linalg.norm(A.T @ Rf - np.eye(3)))
    Rr = R0.T @ Rf
    Kf = (Rr - np.eye(3)) @ np.linalg.inv(Rr + np.eye(3))
    Kf = (Kf - Kf.T) / 2
    k = [[arb(Fraction(Kf[i][j]).limit_denominator(10 ** 12).numerator) / Fraction(Kf[i][j]).limit_denominator(10 ** 12).denominator
          for j in range(3)] for i in range(3)]
    for i in range(3):
        k[i][i] = arb(0)
    k = [[k[i][j] if i < j else (-k[j][i] if i > j else arb(0)) for j in range(3)] for i in range(3)]
    I = [[arb(1 if i == j else 0) for j in range(3)] for i in range(3)]
    A = [[I[i][j] + k[i][j] for j in range(3)] for i in range(3)]
    B = [[I[i][j] - k[i][j] for j in range(3)] for i in range(3)]
    # inverse of B (3x3) by adjugate
    det = (B[0][0] * (B[1][1] * B[2][2] - B[1][2] * B[2][1]) - B[0][1] * (B[1][0] * B[2][2] - B[1][2] * B[2][0])
           + B[0][2] * (B[1][0] * B[2][1] - B[1][1] * B[2][0]))
    adj = [[(B[(j + 1) % 3][(i + 1) % 3] * B[(j + 2) % 3][(i + 2) % 3] - B[(j + 1) % 3][(i + 2) % 3] * B[(j + 2) % 3][(i + 1) % 3])
            for j in range(3)] for i in range(3)]
    Binv = [[adj[i][j] / det for j in range(3)] for i in range(3)]
    C = [[sum((A[i][l] * Binv[l][j] for l in range(3)), arb(0)) for j in range(3)] for i in range(3)]
    return [[sum((arb(int(R0[i][l])) * C[l][j] for l in range(3)), arb(0)) for j in range(3)] for i in range(3)]


def o2(M, budget=1_000_000):
    """Cells (x-box, r-interval). Two lower bounds of V(r x) on a cell:
    L1 = r^2 A_lo + r^3 B_lo + r^4 C_lo (the ray quartic), and
    L2 = r^2 A_lo + min over c in [r0^4 K_lo, r1^4 K_hi] of h(c),  h(c) = (g^2/2) c - g W (c/3)^(3/4),
    with K = |cof x|^2 and W = sum omega_k, from B r^3 = -g W det(r x) and |det| <= (|cof|^2/3)^(3/4)
    (AM-GM on the singular-value products). h has one minimum, at c* = (3^(1/4) W/(2 g))^4, value -g^2 c*/6."""
    Vs = sym_model(M)
    rhoC = M.V0 + tau_C(M)
    parts = [Vs.part(d) for d in (2, 3, 4)]
    polys = [(p, [p.deriv(k) for k in range(5)]) for p in parts]
    Vgrad = [Vs.deriv(k) for k in range(5)]
    r2 = arb(2).sqrt(); qd = [float(x.mid()) for x in M.qd]; qb = np.diag(qd)
    qbA = [[M.qd[i] if i == j else arb(0) for j in range(3)] for i in range(3)]
    rho = float(rhoC.upper())

    def L3(lo, hi, r0, r1_):
        """V on the Cartesian hull Y = [r0, r1] * X of the cell: natural and mean-value forms (L3)."""
        R = arb((r0 + r1_) / 2, (r1_ - r0) / 2)
        X = [arb((a + b) / 2, (b - a) / 2) if b > a else arb(a) for a, b in zip(lo, hi)]
        Y = [R * x for x in X]
        yc = [arb(float(y.mid())) for y in Y]
        nat = Vs(Y)
        mv = Vs(yc) + sum((Vgrad[k](Y) * (Y[k] - yc[k]) for k in range(5)), arb(0))
        return max(float(nat.lower()), float(mv.lower()))

    g = M.g; W = sum(M.om, arb(0)); half_g2 = g * g / 2
    cstar = (arb(3).root(4) * W / (2 * g)) ** 4
    h = lambda c: half_g2 * c - g * W * (c / 3).root(4) ** 3
    hmin_global = h(cstar)

    def hmin(c0, c1):
        if float(c1.upper()) <= float(cstar.lower()):
            return h(c1)
        if float(c0.lower()) >= float(cstar.upper()):
            return h(c0)
        return hmin_global

    def coeffs(lo, hi):
        X, ci, free = boxes_X(lo, hi)
        return [enclose(pp, g_, X, ci, free) for pp, g_ in polys], X

    def near(lo, hi, r0, r1_, X):
        ci = [(a + b) / 2 for a, b in zip(lo, hi)]
        Sc = np.array([[float(v.mid()) for v in row] for row in sym_matrix([arb(c) for c in ci], r2)]) * (r0 + r1_) / 2
        U, _, Vt = np.linalg.svd(qb.T @ Sc)
        Rf = U @ np.diag([1, 1, np.sign(np.linalg.det(U @ Vt))]) @ Vt
        Ra = exact_rotation(Rf)
        Q = [[sum((qbA[i][l] * Ra[l][j] for l in range(3)), arb(0)) for j in range(3)] for i in range(3)]
        R = arb((r0 + r1_) / 2, (r1_ - r0) / 2)
        S = sym_matrix(X, r2)
        tot = sum((sq(R * S[i][j] - Q[i][j]) for i in range(3) for j in range(3)), arb(0))
        return float(tot.upper()) < RN * RN

    heap = []; seq = itertools.count(); processed = 0; clear = near_n = 0
    for lo, hi in sym_faces():
        heapq.heappush(heap, (0, next(seq), lo, hi, None))
    while heap:
        _, _, lo, hi, rint = heapq.heappop(heap)
        processed += 1
        if processed > budget:
            return "UNRESOLVED", processed, (lo, hi, rint), clear, near_n
        ((A, _), (B, _), (C, Chi)), X = coeffs(lo, hi)
        A = A if float(A) > 0 else arb(0)
        C = C if float(C) > 0 else arb(0)
        Klo, Khi = C / half_g2, arb(Chi) / half_g2
        if rint is None:
            if float(B) >= 0:
                clear += 1; continue
            caps = []
            if float(C) > 0:
                disc = B * B - 4 * A * C
                if float(disc.upper()) <= 0:
                    clear += 1; continue
                caps.append(float(((-B + disc.sqrt()) / (2 * C)).upper()))
            if float(A) > 0:
                t = (rhoC - hmin_global) / A               # beyond sqrt(t): r^2 A_lo + min h >= rho_C
                caps.append(float(t.sqrt().upper()) if float(t) > 0 else 0.0)
            if not caps:
                split_x(heap, seq, lo, hi, None); continue
            rint = (0.0, min(caps) * (1 + 1e-9) + 1e-9)
        r0, r1_ = rint
        if quartic_min(A, B, C, r0, r1_) >= rho:
            clear += 1; continue
        L2 = arb(r0) ** 2 * A + hmin(arb(r0) ** 4 * Klo, arb(r1_) ** 4 * Khi)
        if float(L2.lower()) >= rho:
            clear += 1; continue
        if L3(lo, hi, r0, r1_) >= rho:
            clear += 1; continue
        if near(lo, hi, r0, r1_, X):
            near_n += 1; continue
        xw = max(hh - l for l, hh in zip(lo, hi)) * r1_
        if (r1_ - r0) > xw:
            m = (r0 + r1_) / 2
            for iv in ((r0, m), (m, r1_)):
                heapq.heappush(heap, (-(iv[1] - iv[0]), next(seq), lo, hi, iv))
        else:
            split_x(heap, seq, lo, hi, rint)
    return "PROVED", processed, None, clear, near_n


def split_x(heap, seq, lo, hi, rint):
    j = max(range(len(lo)), key=lambda k: hi[k] - lo[k])
    if hi[j] - lo[j] < 1e-9:
        raise ArithmeticError("x-box below resolution")
    m = (lo[j] + hi[j]) / 2
    for a, b in ((lo[j], m), (m, hi[j])):
        l2, h2 = list(lo), list(hi); l2[j], h2[j] = a, b
        heapq.heappush(heap, (-max(x - y for x, y in zip(h2, l2)), next(seq), l2, h2, rint))


def bnb(lb, faces, budget):
    heap = []; seq = itertools.count(); worst = math.inf
    for lo, hi in faces:
        heapq.heappush(heap, (lb(lo, hi), next(seq), lo, hi))
    processed = 0
    while heap:
        v, _, lo, hi = heapq.heappop(heap)
        if v >= 0:
            return "PROVED", min(worst, v), processed, None
        processed += 1
        if processed > budget:
            return "UNRESOLVED", worst, -processed, (v, lo, hi)
        j = max(range(len(lo)), key=lambda k: hi[k] - lo[k])
        if hi[j] - lo[j] < 1e-9:
            return "FAILED", worst, processed, (v, lo, hi)
        m = (lo[j] + hi[j]) / 2
        for a, b in ((lo[j], m), (m, hi[j])):
            l2, h2 = list(lo), list(hi); l2[j], h2[j] = a, b
            w = lb(l2, h2)
            if w >= 0:
                worst = min(worst, w)
            else:
                heapq.heappush(heap, (w, next(seq), l2, h2))
    return "PROVED", worst, processed, None


if __name__ == "__main__":
    g2 = float([a.split("=")[1] for a in sys.argv if a.startswith("--g2=")][0])
    budget = [int(a.split("=")[1]) for a in sys.argv if a.startswith("--budget=")]
    M = r1.Model(g2)
    print(f"=== g2 = {g2}: tau = rho_C - V_bg = {float(tau_C(M).mid()):.5f}; V_bg = {float(M.V0.mid()):.4f}; R_n = {RN}",
          flush=True)
    if "--o1" in sys.argv:
        t0 = time.time(); v, worst, nb, ob = o1(M, *(budget or []))
        print(f"  O1: {v}; min margin over closed boxes {worst:.4f}; {abs(nb)} boxes; {time.time() - t0:.0f} s"
              + (f"; open {ob[0]:.3e}" if ob else ""), flush=True)
    if "--o2" in sys.argv:
        t0 = time.time(); v, nb, ob, nc, nn = o2(M, *(budget or []))
        print(f"  O2: {v}; {nb} cells ({nc} clear, {nn} near the ring); {time.time() - t0:.0f} s"
              + (f"; open cell {ob}" if ob else ""), flush=True)
