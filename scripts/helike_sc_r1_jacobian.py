"""S_C rigorous build, stage R1.3: the Jacobian potential v_J and the face-wise Robin coefficients on the product tube
B_u(5) x B_v(2) (docs/research/helike-sc-rigorous-base-R1-registration-2026-11-25.md, R1.3). Arb at 128 bits.

J(z) = det[N | qG_1 | qG_2 | qG_3], q = q_b + N z. With Q = [N | M] orthonormal (M = the normalised gauge tangents at
q_b, which are mutually orthogonal and orthogonal to N by R1.1), det[N | T] = det(Q) det(M^T T), so up to a fixed
sign J(z) = det(M^T (q_b + N z) G_k), a cubic polynomial in z, expanded exactly.

V_J = Lap(sqrt J) / (2 sqrt J) = Lap J / (4 J) - |grad J|^2 / (8 J^2).
v_J  <= min over the tube of V_J;   beta_u >= max_{|u|=5,|v|<=2} |d_uhat J| / (2J);   beta_v likewise on |v| = 2.
"""
import heapq
import itertools
import math

from flint import arb

import helike_sc_r1 as r1
from helike_sc_r1 import Poly, checked, sq

LU, LV = 5.0, 2.0


def jacobian_poly(M):
    gens = (((0, 0, 0), (0, 0, -1), (0, 1, 0)), ((0, 0, 1), (0, 0, 0), (-1, 0, 0)), ((0, -1, 0), (1, 0, 0), (0, 0, 0)))
    z = [Poly.var(6, k) for k in range(6)]
    q = [[sum((M.N[3 * i + j][k] * z[k] for k in range(6)), Poly.const(6, M.qd[i] if i == j else 0)) for j in range(3)]
         for i in range(3)]
    tang = M.gauge_tangents()
    units = []
    for t in tang:
        n = sum((x * x for x in t), arb(0)).sqrt()
        units.append([x / n for x in t])
    # T_k = (q G_k) flattened; entry (a, k) of M^T T = <unit_a, q G_k>
    A = [[None] * 3 for _ in range(3)]
    for k, Gm in enumerate(gens):
        qG = [[sum((q[i][l] * Gm[l][j] for l in range(3) if Gm[l][j] != 0), Poly.const(6, 0)) for j in range(3)]
              for i in range(3)]
        for a in range(3):
            A[a][k] = sum((qG[i][j] * units[a][3 * i + j] for i in range(3) for j in range(3) if units[a][3 * i + j] != 0),
                          Poly.const(6, 0))
    det = (A[0][0] * (A[1][1] * A[2][2] - A[1][2] * A[2][1]) - A[0][1] * (A[1][0] * A[2][2] - A[1][2] * A[2][0])
           + A[0][2] * (A[1][0] * A[2][1] - A[1][1] * A[2][0]))
    J0 = det.t.get((0,) * 6, arb(0))
    return det * (1 if J0 > 0 else -1)


class Box:
    """Product box in z = (u, v); lo/hi are exact dyadic floats."""

    def __init__(self, lo, hi):
        self.lo, self.hi = list(lo), list(hi)

    def X(self):
        return [arb((a + b) / 2, (b - a) / 2) for a, b in zip(self.lo, self.hi)]

    def c(self):
        return [arb((a + b) / 2) for a, b in zip(self.lo, self.hi)]

    def split(self, k):
        m = (self.lo[k] + self.hi[k]) / 2
        a = Box(self.lo, self.hi); a.hi[k] = m
        b = Box(self.lo, self.hi); b.lo[k] = m
        return a, b

    def norm2_range(self, block):
        idx = range(0, 3) if block == "u" else range(3, 6)
        mn = sum(0.0 if self.lo[k] <= 0 <= self.hi[k] else min(abs(self.lo[k]), abs(self.hi[k])) ** 2 for k in idx)
        mx = sum(max(abs(self.lo[k]), abs(self.hi[k])) ** 2 for k in idx)
        return mn, mx


def mv(p, grads, X, c):
    """Mean-value enclosure of polynomial p over the box X, intersected with the natural enclosure."""
    m = p(c) + sum((grads[k](X) * (X[k] - c[k]) for k in range(len(X))), arb(0))
    nat = p(X)
    return m if m.rad() < nat.rad() else nat


class JacobianBounds:
    def __init__(self, M):
        self.J = jacobian_poly(M)
        self.dJ = [self.J.deriv(k) for k in range(6)]
        self.d2J = [[self.dJ[i].deriv(j) for j in range(6)] for i in range(6)]
        self.lapJ = sum((self.d2J[k][k] for k in range(6)), Poly.const(6, 0))
        self.dlap = [self.lapJ.deriv(k) for k in range(6)]

    def VJ_point(self, c):
        J = self.J(c); g2 = sum((sq(self.dJ[k](c)) for k in range(6)), arb(0))
        return self.lapJ(c) / (4 * J) - g2 / (8 * J * J)

    def VJ_box(self, X, c):
        J = checked(mv(self.J, self.dJ, X, c), positive=True)        # J > 0 proved on the box
        g = [mv(self.dJ[k], self.d2J[k], X, c) for k in range(6)]
        g2 = sum((sq(x) for x in g), arb(0))
        L = mv(self.lapJ, self.dlap, X, c)
        return L / (4 * J) - g2 / (8 * sq(J))

    def vJ_lower(self, tol=5e-4, max_boxes=100_000):
        """Rigorous lower bound on min over B_u(5) x B_v(2) of V_J. Returns (bound, best point value, boxes)."""
        root = Box([-LU] * 3 + [-LV] * 3, [LU] * 3 + [LV] * 3)
        best = float(self.VJ_point(root.c()).mid()); heap = []; seq = itertools.count()
        def push(b):
            nonlocal best
            mnu, _ = b.norm2_range("u"); mnv, _ = b.norm2_range("v")
            if mnu > LU * LU or mnv > LV * LV:
                return                                                 # box misses the tube
            X, c = b.X(), b.c()
            try:
                v = self.VJ_box(X, c)
            except ValueError:                                         # J > 0 not yet proved here: split further
                v = None
            pv = float(self.VJ_point(c).mid())
            cu = sum(float(x.mid()) ** 2 for x in c[:3]); cv = sum(float(x.mid()) ** 2 for x in c[3:])
            if cu <= LU * LU and cv <= LV * LV:
                best = min(best, pv)
            heapq.heappush(heap, (float(v.lower().mid()) if v is not None and v.is_finite() else -math.inf, next(seq), b, v))
        push(root)
        n = 0
        while heap:
            lb, _, b, v = heapq.heappop(heap)
            if lb >= best - tol:
                return arb(lb).lower(), best, n
            n += 1
            if n > max_boxes:                                          # budget: heap top bounds every open box
                return arb(lb).lower(), best, -n
            k = max(range(6), key=lambda i: (b.hi[i] - b.lo[i]) * (1.0 if i in (2, 5) else 0.6))
            for ch in b.split(k):
                push(ch)
        raise RuntimeError("empty")

    def beta_upper(self, face, tol=2e-3, max_boxes=100_000):
        """Rigorous upper bound on max |d_n J| / (2J) on the face |u| = 5 (face 'u') or |v| = 2 (face 'v').
        On the face, d_n J = (w . grad_w J)/|w| with w the face block's coordinates and |w| = L_face."""
        Lf = LU if face == "u" else LV
        fidx = (0, 1, 2) if face == "u" else (3, 4, 5)
        root = Box([-LU] * 3 + [-LV] * 3, [LU] * 3 + [LV] * 3)
        best = 0.0; heap = []; seq = itertools.count()
        def val(X, c, box):
            J = checked(mv(self.J, self.dJ, X, c), positive=True)
            dn = sum((X[k] * mv(self.dJ[k], self.d2J[k], X, c) for k in fidx), arb(0)) / Lf
            return abs(dn) / (2 * J)
        def push(b):
            nonlocal best
            mnf, mxf = b.norm2_range(face)
            other = "v" if face == "u" else "u"; Lo = LV if face == "u" else LU
            mno, _ = b.norm2_range(other)
            if not (mnf <= Lf * Lf <= mxf) or mno > Lo * Lo:
                return                                                 # box misses the face
            X, c = b.X(), b.c()
            try:
                v = val(X, c, b); key = -float(v.upper().mid())
            except ValueError:
                v = None; key = -math.inf
            # a point on the face: project the centre's face block onto the sphere of radius Lf
            cf = [float(x.mid()) for x in c]; nf = math.sqrt(sum(cf[k] ** 2 for k in fidx)) or 1.0
            p = list(cf)
            for k in fidx:
                p[k] = cf[k] * Lf / nf if nf > 0 else (Lf if k == fidx[0] else 0.0)
            po = [p[k] for k in range(6) if k not in fidx]
            if sum(x * x for x in po) <= Lo * Lo:
                pa = [arb(x) for x in p]
                best = max(best, float(val(pa, pa, None).mid()))
            heapq.heappush(heap, (key, next(seq), b, v))
        push(root)
        n = 0
        while heap:
            nub, _, b, v = heapq.heappop(heap)
            if -nub <= best + tol:
                return arb(-nub).upper(), best, n
            n += 1
            if n > max_boxes:
                return arb(-nub).upper(), best, -n
            k = max(range(6), key=lambda i: b.hi[i] - b.lo[i])
            for ch in b.split(k):
                push(ch)
        raise RuntimeError("empty")


if __name__ == "__main__":
    import sys
    import time
    import numpy as np
    import estimate_helike_sc_gbase as gb
    for key in (0.004, 0.002):
        M = r1.Model(key); JB = JacobianBounds(M)
        m = gb.setup(key)
        # check against the floating Jacobian of the G-base script (up to the frame's sign/rotation: compare J/J(0))
        rng = np.random.default_rng(3); worst = 0.0
        Nm = np.array([[float(M.N[i][k].mid()) for k in range(6)] for i in range(9)])
        def Jf(z):
            q = (m["x0"] + Nm @ z).reshape(3, 3)
            return abs(np.linalg.det(np.column_stack([Nm] + [(q @ G).ravel() for G in gb.GEN])))
        for _ in range(50):
            z = rng.normal(size=6) * [2, 2, 2, 0.8, 0.8, 0.8]
            worst = max(worst, abs(float(JB.J([arb(x) for x in z]).mid()) / Jf(z) - 1))
        print(f"=== g2 = {key}: J(0) = {float(JB.J([arb(0)] * 6).mid()):.6f}, {len(JB.J.t)} terms, "
              f"max rel. difference vs floating det {worst:.1e}")
        t0 = time.time(); lb, best, nb = JB.vJ_lower()
        print(f"  v_J >= {float(lb.mid()):.6f}  (best point {best:.6f}; {nb} boxes{' (budget)' if nb < 0 else ''}; {time.time() - t0:.0f} s)")
        for face in ("u", "v"):
            t0 = time.time(); ub, bp, nb = JB.beta_upper(face)
            print(f"  beta_{face} <= {float(ub.mid()):.6f}  (best point {bp:.6f}; {nb} boxes; {time.time() - t0:.0f} s)")
