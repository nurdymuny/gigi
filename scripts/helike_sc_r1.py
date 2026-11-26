"""S_C rigorous build, stage R1 (docs/research/helike-sc-rigorous-base-R1-registration-2026-11-25.md).

R1.1 exact transverse frame and the polynomial P(z) = V(x0 + N z) - V(x0) with Arb coefficients;
R1.2 C3 >= max_{|y| = 1} |P3(y)| by branch and bound over the faces of [-1, 1]^6.

All bound arithmetic is Arb at 128 bits. NumPy is used only for comparison with the floating-point run.
"""
import heapq
import itertools
import math

from flint import arb, ctx

PRECISION = 128
ctx.prec = PRECISION
S, T = arb(23) / 20, arb(3)                 # background controls s = 1.15, t = 3
G2 = {0.004: arb(1) / 250, 0.002: arb(1) / 500}


def checked(x, positive=False, nonnegative=False):
    x = arb(x)
    if ctx.prec != PRECISION:
        raise ValueError("registered precision is 128 bits")
    if not x.is_finite():
        raise ValueError("nonfinite bound")
    if positive and not x > 0:
        raise ValueError("positive bound required")
    if nonnegative and not x >= 0:
        raise ValueError("nonnegative bound required")
    return x


# ------------------------------------------------------------------ polynomials over Arb
class Poly:
    """Sparse polynomial in n variables with Arb coefficients: {exponent tuple: arb}."""

    def __init__(self, n, terms=None):
        self.n, self.t = n, dict(terms or {})

    @staticmethod
    def const(n, c):
        return Poly(n, {(0,) * n: arb(c)})

    @staticmethod
    def var(n, k):
        return Poly(n, {tuple(1 if i == k else 0 for i in range(n)): arb(1)})

    def __add__(self, o):
        o = o if isinstance(o, Poly) else Poly.const(self.n, o)
        r = dict(self.t)
        for e, c in o.t.items():
            r[e] = r[e] + c if e in r else c
        return Poly(self.n, r)

    __radd__ = __add__

    def __neg__(self):
        return Poly(self.n, {e: -c for e, c in self.t.items()})

    def __sub__(self, o):
        return self + (-(o if isinstance(o, Poly) else Poly.const(self.n, o)))

    def __mul__(self, o):
        if not isinstance(o, Poly):
            o = arb(o)
            return Poly(self.n, {e: c * o for e, c in self.t.items()})
        r = {}
        for e1, c1 in self.t.items():
            for e2, c2 in o.t.items():
                e = tuple(a + b for a, b in zip(e1, e2))
                r[e] = r[e] + c1 * c2 if e in r else c1 * c2
        return Poly(self.n, r)

    __rmul__ = __mul__

    def part(self, d):
        return Poly(self.n, {e: c for e, c in self.t.items() if sum(e) == d})

    def deriv(self, k):
        r = {}
        for e, c in self.t.items():
            if e[k]:
                f = list(e); f[k] -= 1
                r[tuple(f)] = c * e[k]
        return Poly(self.n, r)

    def __call__(self, xs):
        pw = [{0: arb(1), 1: arb(xs[i])} for i in range(self.n)]
        def p(i, k):            # even powers through sq(), so they stay nonnegative on intervals
            if k not in pw[i]:
                pw[i][k] = sq(p(i, k // 2)) if k % 2 == 0 else p(i, k - 1) * pw[i][1]
            return pw[i][k]
        s = arb(0)
        for e, c in self.t.items():
            m = c
            for i, k in enumerate(e):
                if k:
                    m = m * p(i, k)
            s += m
        return s


def sq(x):
    """x^2 as an interval that knows both factors are the same variable (x * x does not: [-1,1]^2 -> [-1,1])."""
    x = arb(x)
    lo, hi = x.abs_lower(), x.abs_upper()
    return arb(lo * lo).union(arb(hi * hi))


def mono_pow(x, e):
    """x^e for an interval x > 0 and e > 0, from the endpoints (monotone), not from the ball."""
    x = checked(x, positive=True)
    lo, hi = x.lower(), x.upper()
    return arb(lo ** arb(e)).union(arb(hi ** arb(e)))


def cross(a, b):
    return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]


def potential(q, g, om, src):
    """Vq of estimate_helike_sc_route1 for q a 3x3 list of Poly (or arb)."""
    cof = [cross(q[1], q[2]), cross(q[2], q[0]), cross(q[0], q[1])]
    V = 0
    for a in range(3):
        for j in range(3):
            w = om[a] * q[a][j] - g * cof[a][j]
            V = V + w * w * arb("0.5") - src[a] * q[a][j] * q[a][j] * arb("0.5")
    return V


# ------------------------------------------------------------------ R1.1 the model at a control
class Model:
    def __init__(self, g2key):
        g2 = G2[g2key]; self.g2key = g2key
        self.g = g = g2.sqrt()
        st = T.sqrt()
        self.om = [st, 1 / st, 1 / st]
        a = (1 - S) * (T - 2 * S); b = (1 - S) * ((1 - S) / T - S)
        self.src = [a, b, b]
        self.qd = [S / (g * st), S / g, S / g]                           # q_b = diag(qd)
        # Hessian at q_b, from the exact expansion in 9 variables
        d = [Poly.var(9, k) for k in range(9)]
        q = [[d[3 * i + j] + (self.qd[i] if i == j else 0) for j in range(3)] for i in range(3)]
        W = potential(q, g, self.om, self.src)
        self.V0 = W.t[(0,) * 9]
        self.grad9 = [W.t.get(tuple(1 if i == k else 0 for i in range(9)), arb(0)) for k in range(9)]
        K = [[arb(0)] * 9 for _ in range(9)]
        for e, c in W.part(2).t.items():
            idx = [i for i, k in enumerate(e) for _ in range(k)]
            if idx[0] == idx[1]:
                K[idx[0]][idx[0]] = 2 * c
            else:
                K[idx[0]][idx[1]] = c; K[idx[1]][idx[0]] = c
        self.K = K
        self.N, self.w2 = self._frame()
        self.P = self._expand()

    def _frame(self):
        K, q1, q2, q3 = self.K, *self.qd
        r2 = arb(2).sqrt()
        e = lambda i, j: 3 * i + j
        def vec(entries):
            v = [arb(0)] * 9
            for (i, j), c in entries.items():
                v[e(i, j)] = arb(c)
            return v
        def unit(v):
            n = sum((x * x for x in v), arb(0)).sqrt()
            return [x / n for x in v]
        soft1 = vec({(1, 1): 1 / r2, (2, 2): -1 / r2})
        soft2 = unit(vec({(1, 2): q3, (2, 1): q2}))
        stiff1 = unit(vec({(0, 1): q2, (1, 0): q1}))
        stiff2 = unit(vec({(0, 2): q3, (2, 0): q1}))
        # diagonal 2x2 block on f1 = e11, f2 = (e22 + e33)/sqrt 2
        f1 = vec({(0, 0): 1}); f2 = vec({(1, 1): 1 / r2, (2, 2): 1 / r2})
        quad = lambda x, y: sum((x[i] * K[i][j] * y[j] for i in range(9) for j in range(9) if K[i][j] != 0), arb(0))
        A, B, C = quad(f1, f1), quad(f1, f2), quad(f2, f2)
        disc = ((A - C) * (A - C) / 4 + B * B).sqrt()
        lo, hi = (A + C) / 2 - disc, (A + C) / 2 + disc
        def eigvec(lam):            # (B, lam - A) normalised, in the f basis
            c1, c2 = B, lam - A
            n = (c1 * c1 + c2 * c2).sqrt()
            return [(c1 * x + c2 * y) / n for x, y in zip(f1, f2)]
        soft3, stiff3 = eigvec(lo), eigvec(hi)
        cols = [soft1, soft2, soft3, stiff1, stiff2, stiff3]
        w2 = [quad(c, c) for c in cols]
        N = [[cols[k][i] for k in range(6)] for i in range(9)]
        return N, w2

    def gauge_tangents(self):
        gens = (((0, 0, 0), (0, 0, -1), (0, 1, 0)), ((0, 0, 1), (0, 0, 0), (-1, 0, 0)), ((0, -1, 0), (1, 0, 0), (0, 0, 0)))
        out = []
        for Gm in gens:
            v = [arb(0)] * 9
            for i in range(3):
                for j in range(3):
                    v[3 * i + j] = self.qd[i] * Gm[i][j]            # (q_b G)_ij with q_b diagonal
            out.append(v)
        return out

    def _expand(self):
        z = [Poly.var(6, k) for k in range(6)]
        q = [[sum((self.N[3 * i + j][k] * z[k] for k in range(6)), Poly.const(6, self.qd[i] if i == j else 0))
              for j in range(3)] for i in range(3)]
        return potential(q, self.g, self.om, self.src) - self.V0

    def checks(self):
        """R1.1 checks; returns dict of (name, value, ok)."""
        out = {}
        cols = [[self.N[i][k] for i in range(9)] for k in range(6)]
        tang = self.gauge_tangents()
        dot = lambda x, y: sum((a * b for a, b in zip(x, y)), arb(0))
        orth = all(dot(cols[a], cols[b]).contains(1 if a == b else 0) for a in range(6) for b in range(6))
        gperp = all(dot(c, t).contains(0) for c in cols for t in tang)
        eig = True
        for k, c in enumerate(cols):
            Kc = [sum((self.K[i][j] * c[j] for j in range(9)), arb(0)) for i in range(9)]
            eig &= all((Kc[i] - self.w2[k] * c[i]).contains(0) for i in range(9))
        out["frame orthonormal"] = orth
        out["frame orthogonal to gauge tangents"] = gperp
        out["frame columns are Hessian eigenvectors"] = eig
        out["gradient at q_b contains 0"] = all(x.contains(0) for x in self.grad9)
        out["linear part of P contains 0"] = all(c.contains(0) for c in self.P.part(1).t.values())
        quad_ok = True
        for e, c in self.P.part(2).t.items():
            if max(e) == 2:
                quad_ok &= (c - self.w2[e.index(2)] / 2).contains(0)
            else:
                quad_ok &= c.contains(0)
        out["quadratic part is 1/2 sum w2 z^2"] = quad_ok
        # quartic part = (g^2/2)|cof(Nz)|^2 identically
        z = [Poly.var(6, k) for k in range(6)]
        dq = [[sum((self.N[3 * i + j][k] * z[k] for k in range(6)), Poly.const(6, 0)) for j in range(3)] for i in range(3)]
        cof = [cross(dq[1], dq[2]), cross(dq[2], dq[0]), cross(dq[0], dq[1])]
        Q = sum((cof[a][j] * cof[a][j] for a in range(3) for j in range(3)), Poly.const(6, 0)) * (self.g * self.g / 2)
        diff = self.P.part(4) - Q
        out["quartic part is (g^2/2)|cof Nz|^2"] = all(c.contains(0) for c in diff.t.values())
        return out


# ------------------------------------------------------------------ R1.2 C3
def c3_bound(P3, tol=arb(1) / 1000, max_boxes=400_000):
    """Rigorous upper bound on max_{|y|=1} |P3(y)| (P3 homogeneous cubic in 6 variables, odd).
    Faces x_i = +1 of [-1,1]^6 suffice by oddness. Returns (upper bound, best sampled value, boxes processed)."""
    n = 6
    grad = [P3.deriv(k) for k in range(n)]
    hess = [[grad[i].deriv(j) for j in range(n)] for i in range(n)]
    best = arb(0); heap = []; seq = itertools.count()
    def box_ub(face, lo, hi):
        X = []; ci = []; it = iter(range(n - 1))
        for k in range(n):
            if k == face:
                X.append(arb(1)); ci.append(arb(1))
            else:
                j = next(it)
                X.append(arb((lo[j] + hi[j]) / 2, (hi[j] - lo[j]) / 2)); ci.append(arb((lo[j] + hi[j]) / 2))
        # f(x) = P3(x) / |x|^3 on the face; mean-value form of f itself:
        # f(X) in f(c) + sum_k df/dx_k(X) (X_k - c_k),  df/dx_k = dP3_k / r^3 - 3 P3 x_k / r^5
        nc2 = sum((sq(x) for x in ci), arb(0))
        c3, c5 = mono_pow(nc2, "1.5"), mono_pow(nc2, "2.5")
        Pc = P3(ci); fc = Pc / c3
        r2 = checked(sum((sq(x) for x in X), arb(0)), positive=True)
        r3, r5, r7 = mono_pow(r2, "1.5"), mono_pow(r2, "2.5"), mono_pow(r2, "3.5")
        PX = P3(X); gX = [grad[k](X) for k in range(n)]
        free = [k for k in range(n) if k != face]
        rad = {k: X[k].rad() for k in free}
        # first order (mean value) and second order (point gradient + interval Hessian) upper bounds on |f|
        mv = fc + sum(((gX[k] / r3 - 3 * PX * X[k] / r5) * (X[k] - ci[k]) for k in free), arb(0))
        ub1 = abs(mv).upper()
        gc = {k: grad[k](ci) / c3 - 3 * Pc * ci[k] / c5 for k in free}
        ub2 = abs(fc).upper() + sum((abs(gc[k]).upper() * rad[k] for k in free), arb(0))
        for i in free:
            for j in free:
                Hij = hess[i][j](X) / r3 - 3 * (gX[i] * X[j] + gX[j] * X[i]) / r5 \
                    - (3 * PX / r5 if i == j else 0) + 15 * PX * X[i] * X[j] / r7
                ub2 += abs(Hij).upper() * rad[i] * rad[j] / 2
        ub = checked(min(ub1, ub2.upper(), key=lambda x: float(x.upper())).upper(), nonnegative=True)
        return arb(ub.upper()), arb(abs(fc).mid())
    for face in range(n):
        lo, hi = [-1.0] * (n - 1), [1.0] * (n - 1)
        ub, sv = box_ub(face, lo, hi)
        best = max(best, sv, key=lambda x: float(x.mid()))
        heapq.heappush(heap, (-float(ub.upper()), next(seq), face, lo, hi, ub))
    processed = 0
    while heap:
        negub, _, face, lo, hi, ub = heapq.heappop(heap)
        if -negub <= float(best.mid()) + float(tol.mid()):
            return arb(-negub).upper(), best, processed
        processed += 1
        if processed > max_boxes:
            # budget reached: the top of the heap bounds every unresolved box, so it is still a rigorous bound
            return arb(max(-negub, float(best.mid()) + float(tol.mid()))).upper(), best, -processed
        j = max(range(n - 1), key=lambda k: hi[k] - lo[k]); m = (lo[j] + hi[j]) / 2
        for a, b in ((lo[j], m), (m, hi[j])):
            l2, h2 = list(lo), list(hi); l2[j], h2[j] = a, b
            u2, s2 = box_ub(face, l2, h2)
            if float(s2.mid()) > float(best.mid()):
                best = s2
            if float(u2.upper()) > float(best.mid()) + float(tol.mid()):
                heapq.heappush(heap, (-float(u2.upper()), next(seq), face, l2, h2, u2))
    return arb(float(best.mid()) + float(tol.mid())).upper(), best, processed


if __name__ == "__main__":
    import sys
    import time
    for key in (0.004, 0.002):
        t0 = time.time(); M = Model(key)
        print(f"=== g2 = {key}: w2 = {[float(w.mid()) for w in M.w2]}")
        for k, v in M.checks().items():
            print(f"  {'PASS' if v else 'FAIL'}  {k}")
        print(f"  P: {len(M.P.t)} terms, max coefficient radius {max(float(c.rad()) for c in M.P.t.values()):.1e}"
              f" ({time.time() - t0:.0f} s)")
        if "--c3" in sys.argv:
            t0 = time.time(); ub, best, nb = c3_bound(M.P.part(3))
            print(f"  C3 <= {float(ub.mid()):.6f} [{'converged' if nb >= 0 else 'box budget reached'}]  "
                  f"(best sampled {float(best.mid()):.6f}; {abs(nb)} boxes; {time.time() - t0:.0f} s)")
