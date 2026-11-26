"""S_C rigorous build, stage R1.4: the enclosed base spectrum
(docs/research/helike-sc-rigorous-base-R1-registration-2026-11-25.md, R1.4). Arb at 128 bits.

Block base: -1/2 Lap + 1/2 w^2 r^2 + c on the 3-ball of radius L, Robin condition R'(L) = beta R(L)
(the natural condition of the form 1/2 int |grad psi|^2 - beta/2 int_sphere |psi|^2).
Regular radial solution: R_l(r) = r^l exp(-w r^2 / 2) M(a, b, w r^2), b = l + 3/2, a = b/2 - (E - c)/(2w).
Up to a positive factor, R'(L) - beta R(L) = f_l(E) = (l/L - wL - beta) M(a, b, X) + 2 w L (a/b) M(a+1, b+1, X),
X = w L^2.

COUNT. With the Pruefer angle theta (R = rho sin theta, L^2 R' = rho cos theta), theta(L, E) is strictly increasing in E
and the n-th eigenvalue solves theta(L, E) = alpha + n pi with cot alpha = L^2 beta. Hence, for E not an eigenvalue,
    #{eigenvalues < E} = Z(E) + [R'(L)/R(L) < beta],
with Z(E) the number of zeros of R_l(., E) in (0, L) = zeros of M(a, b, x) for x in (0, X), counted rigorously
(segments where M excludes 0, or M' = (a/b) M(a+1, b+1, x) excludes 0 with endpoint signs).
ISOLATION by bisection on the count until every interval holds exactly one eigenvalue; REFINEMENT by the sign of f_l
(one simple root per interval). l-RANGE: the forms increase with l, so the first l with no eigenvalue below the cut
ends the scan.
"""
from flint import arb, ctx

import helike_sc_r1 as r1                     # sets the 128-bit context
from helike_sc_r1 import checked

assert ctx.prec == r1.PRECISION


def kummer(a, b, x):
    return arb(x).hypgeom_1f1(arb(a), arb(b))


class RadialBlock:
    def __init__(self, w2, L, beta, c):
        self.w2 = checked(w2, positive=True); self.w = self.w2.sqrt()
        self.L, self.beta, self.c = arb(L), arb(beta), arb(c)
        self.X = self.w * self.L * self.L

    def ab(self, l, E):
        b = arb(l) + arb(3) / 2
        return b / 2 - (arb(E) - self.c) / (2 * self.w), b

    def f(self, l, E):
        a, b = self.ab(l, E)
        return ((arb(l) / self.L - self.w * self.L - self.beta) * kummer(a, b, self.X)
                + 2 * self.w * self.L * (a / b) * kummer(a + 1, b + 1, self.X))

    def zeros(self, l, E, nseg=64, depth=40):
        """Number of zeros of M(a, b, x) for x in (0, X)."""
        a, b = self.ab(l, E)
        Xu = float(self.X.upper())
        def M(x):
            return kummer(a, b, x)
        def dM(x):
            return (a / b) * kummer(a + 1, b + 1, x)
        count = 0
        stack = [(Xu * k / nseg, Xu * (k + 1) / nseg, 0) for k in range(nseg)]
        while stack:
            x0, x1, d = stack.pop()
            if x1 > float(self.X.lower()):
                x1e = self.X                                       # last segment ends exactly at X
            else:
                x1e = arb(x1)
            seg = arb(x0).union(x1e)
            Ms = M(seg)
            if not Ms.contains(0):
                continue
            if d > depth:
                raise RuntimeError(f"zero counting did not resolve (l={l}, E={E})")
            Ds = dM(seg)
            m0, m1 = M(arb(x0)), M(x1e)
            if not Ds.contains(0) and not m0.contains(0) and not m1.contains(0) and x0 > 0:
                count += 1 if (m0 > 0) != (m1 > 0) else 0
                continue
            if x0 == 0 and not Ds.contains(0) and not m1.contains(0):
                # M(a, b, 0) = 1 > 0
                count += 1 if not (m1 > 0) else 0
                continue
            xm = (x0 + x1) / 2
            stack += [(x0, xm, d + 1), (xm, x1, d + 1)]
        return count

    def count_below(self, l, E):
        """#{eigenvalues of the l-th radial problem < E}; E must not be an eigenvalue (f excludes 0)."""
        fv = self.f(l, E)
        if fv.contains(0):
            return None
        a, b = self.ab(l, E)
        Mv = kummer(a, b, self.X)
        if Mv.contains(0):
            return None
        q = fv / Mv                                                # = R'(L)/R(L) - beta, up to a positive factor
        return self.zeros(l, E) + (1 if q < 0 else 0)

    def safe_count(self, l, E, nudge=1e-9):
        for k in range(20):
            n = self.count_below(l, E + k * nudge)
            if n is not None:
                return n, E + k * nudge
        raise RuntimeError("could not find a non-eigenvalue near E")

    def levels(self, l, Elo, Ecut, tol=1e-13):
        """Enclosures of all eigenvalues of the l-th radial problem in [Elo, Ecut)."""
        n_lo, Elo = self.safe_count(l, Elo)
        if n_lo != 0:
            raise ValueError(f"start energy {Elo} is above an eigenvalue for l={l}")
        n_hi, Ecut = self.safe_count(l, Ecut)
        if n_hi == 0:
            return [], 0
        # sign-change brackets of f on a grid; when their number equals the certified count, each holds exactly one
        # eigenvalue (each bracket holds an odd number, at least one, and the total in [Elo, Ecut) is n_hi)
        h = 0.05
        while True:
            Es, sg = [], []
            E = Elo
            while E < Ecut:
                fv = self.f(l, E)
                k = 0
                while fv.contains(0):
                    k += 1; fv = self.f(l, E + k * 1e-9)
                Es.append(E + k * 1e-9); sg.append(fv > 0)
                E += h
            fv = self.f(l, Ecut); Es.append(Ecut); sg.append(fv > 0)
            br = [(Es[i], Es[i + 1]) for i in range(len(Es) - 1) if sg[i] != sg[i + 1]]
            if len(br) == n_hi:
                break
            if len(br) > n_hi:
                raise RuntimeError(f"more sign changes than the certified count (l={l})")
            h /= 4
            if h < 1e-6:
                raise RuntimeError(f"could not bracket all {n_hi} eigenvalues (l={l})")
        roots = [self.refine(l, a, b, tol) for a, b in br]
        return roots, n_hi

    def refine(self, l, a, b, tol):
        fa = self.f(l, a); fb = self.f(l, b)
        sa = fa > 0
        if fa.contains(0) or fb.contains(0) or (fb > 0) == sa:
            raise RuntimeError("refinement bracket has no clean sign change")
        while b - a > tol:
            m = (a + b) / 2
            fm = self.f(l, m)
            if fm.contains(0):
                break
            if (fm > 0) == sa:
                a = m
            else:
                b = m
        return arb(a).union(arb(b))

    def spectrum(self, Ecut, Elo=None, lmax=200):
        """All levels below Ecut as (l, enclosure); multiplicity 2l+1 is implied."""
        Elo = float(self.c.mid()) - 2.0 if Elo is None else Elo
        out = []
        for l in range(lmax):
            roots, n = self.levels(l, Elo, Ecut)
            if n == 0:
                return out, l                                       # forms increase with l: no higher l contributes
            out += [(l, r) for r in roots]
        raise RuntimeError("lmax reached")


if __name__ == "__main__":
    import math
    import time
    import numpy as np
    import dryrun_helike_sc_lehmann_homotopy as dr
    # check against the floating Galerkin base at the floating constants of section 13 (g2 = 0.004)
    w2s, w2t, beta, vJ = 0.38097, 5.09595, 0.0581, -1.06e-3
    for name, w2, L, c in (("soft", w2s, 5.0, vJ), ("stiff", w2t, 2.0, 0.0)):
        t0 = time.time()
        B = RadialBlock(arb(w2), arb(L), arb(beta), arb(c))
        spec, lstop = B.spectrum(8.0)
        fl, _, _ = dr.radial_levels(w2, L, beta, c, 8.0)
        fE = sorted((l, E) for l, E, _ in fl)
        enc = sorted((l, float(r.mid()), float(r.rad())) for l, r in spec)
        diffs = [abs(E - e) for (l1, E), (l2, e, _) in zip(fE, enc) if l1 == l2]
        print(f"{name}: {len(spec)} radial levels below 8 (l < {lstop}); floating Galerkin {len(fE)}; "
              f"max |Galerkin - enclosure| {max(diffs):.1e}; max radius {max(r for _, _, r in enc):.1e}; "
              f"{time.time() - t0:.0f} s")
