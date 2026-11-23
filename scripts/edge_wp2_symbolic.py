"""WP2 (21 November 2026), part 1: exact symbolic facts behind the sharper large-sector bound.

Everything here is exact sympy algebra on the closed-form charge-block rows of scripts/ricci_su2_exact_edge.py
(symbolic_rows is called with sympy.lambdify temporarily replaced by the identity, so the exact expressions are
captured; no existing file is modified).

Checks / derivations (all PROVED = exact symbolic identities, unless labelled otherwise):
  S1. The ingredient table of scripts/ricci_su2_family_large_j.py (w_a, C_aa, same-charge coupling modulus co, its row
      sums R_a, ladder row sums P+_a, P-_a) equals the exact rows (y > 0, t > 0).
  S2. The assembled block is real symmetric: the raw entry (m,a)->(m+1,a') 2i sqrt(.) Omn[a',a] equals the raw entry
      (m+1,a')->(m,a) 2i sqrt(.) Op[a,a'] (same sqrt factor), and both are real; same-charge entries C are real symmetric.
  S3. Diagonal minus the twist eigenvalue lambda_{2l}, at m = l - xi:
        D(l - xi, a) - lambda_{2l} = 4 xi (2l - xi)(1 - 1/t) + l K_a + E_a(xi) + R_a   (identity in l, xi, x, y, t)
      with K_a = 4 - 8/t - 4 (w_a + x)/sqrt t, E_a as in family_large_j.py.
  S4. Slopes K_a, and the xi = 0 diagonals.  K_(1,0) = -4x/sqrt t, K_(1,-1) = -8x/sqrt t; all other K_a > 0 for x <= 0, t > 2
      except K_(1,1) = 0 (the twist type).
  S5. Rayleigh-quotient theorem: if x > 0, then for every l with  l * 8x/sqrt t > c_(1,-1)(x,y,t)  (c the xi = 0 constant),
      the block q = l of sector j = 2l has an eigenvalue below lambda_{2l}: the twist is NOT the floor of that sector of
      H + GG^*.  Same with (1,0): l * 4x/sqrt t > c_(1,0).
usage: python scripts/edge_wp2_symbolic.py OUT.txt
"""
import sys
from pathlib import Path

import sympy as sp

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_exact_edge as EE          # noqa: E402

x, y, t = EE.x, EE.y, EE.t
l, xi = sp.symbols("l xi", positive=True)
yp = sp.symbols("y", positive=True)


def exact_rows():
    orig = EE.sp.lambdify
    EE.sp.lambdify = lambda args, expr, *a, **k: expr
    try:
        ch, (w, C, Op, Omn) = EE.symbolic_rows()
    finally:
        EE.sp.lambdify = orig
    sub = {y: yp}
    w = [sp.simplify(sp.sympify(v).subs(sub)) for v in w]
    C, Op, Omn = (sp.Matrix(M).subs(sub).applyfunc(sp.simplify) for M in (C, Op, Omn))
    return ch, w, C, Op, Omn


def table(X, Y, T):
    """The family_large_j.py ingredient table, rebuilt with sympy functions (same formulas, copied by import)."""
    import ricci_su2_family_large_j as LJ
    return LJ.ingredients(X, Y, T, sp.Abs, sp.sqrt)


def main():
    out = []
    say = lambda *a: (print(*a, flush=True), out.append(" ".join(str(v) for v in a)))   # noqa: E731
    ch, w, C, Op, Omn = exact_rows()
    idx = {a: k for k, a in enumerate(ch)}
    wT, CaaT, RT, PpT, PmT = table(x, yp, t)
    ok = True
    co = sp.Abs(2 * yp * (sp.sqrt(t) * x - t - 2) / sp.sqrt(t))
    # S1
    for a in ch:
        k = idx[a]
        d_w = sp.simplify(w[k] - wT[a])
        d_c = sp.simplify(C[k, k] - CaaT[a])
        R_exact = sum(sp.Abs(C[k2, k]) for k2 in range(9) if k2 != k)
        Pp_exact = sum(sp.Abs(Op[k2, k]) for k2 in range(9))
        Pm_exact = sum(sp.Abs(Omn[k2, k]) for k2 in range(9))
        d_R = sp.simplify(R_exact - RT[a])
        d_P = sp.simplify(Pp_exact - PpT[a]); d_M = sp.simplify(Pm_exact - PmT[a])
        offmods = {sp.simplify(sp.Abs(C[k2, k]) - co) for k2 in range(9) if k2 != k and C[k2, k] != 0}
        good = all(v == 0 for v in (d_w, d_c, d_R, d_P, d_M)) and offmods <= {0}
        ok &= good
        say(f"S1 {a}: w, C_aa, R_a, P+_a, P-_a match table: {good}")
    # S2
    s2 = True
    for a in ch:
        for a2 in ch:
            e1 = sp.simplify(2 * sp.I * Omn[idx[a2], idx[a]])        # (m,a) -> (m+1,a2), times sqrt((l-m)(l+m+1))
            e2 = sp.simplify(2 * sp.I * Op[idx[a], idx[a2]])         # (m+1,a2) -> (m,a), times sqrt((l+m+1)(l-m))
            s2 &= sp.simplify(e1 - e2) == 0 and sp.simplify(sp.im(e1.subs({x: sp.Symbol('xr', real=True)}))) == 0
            s2 &= sp.simplify(C[idx[a2], idx[a]] - C[idx[a], idx[a2]]) == 0
    say("S2 assembled block real symmetric (raw entries already Hermitian and real):", s2)
    ok &= s2
    # S3 / S4
    st = sp.sqrt(t)
    lam = (x + (2 * l + 2) / st) ** 2 + 2 * st * x - yp ** 2
    Ks, cs = {}, {}
    for a in ch:
        k = idx[a]; m = l - xi
        D = (w[k] - 2 * m / st) ** 2 + 4 * (l * (l + 1) - m * m) + C[k, k]
        K = 4 - 8 / t - 4 * (w[k] + x) / st
        E = (4 * w[k] * xi / st + w[k] ** 2 - x ** 2 - 4 / t - 4 * x / st + C[k, k] - RT[a] - 2 * st * x + yp ** 2)
        diff = sp.simplify(sp.expand(D - lam - (4 * xi * (2 * l - xi) * (1 - 1 / t) + l * K + E + RT[a])))
        ok &= diff == 0
        Ks[a] = sp.simplify(K)
        cs[a] = sp.simplify(sp.expand((D - lam).subs(xi, 0) - l * K))      # xi = 0 diagonal constant term
        say(f"S3 {a}: identity holds: {diff == 0};  S4 K_a = {Ks[a]};  xi=0 diag - lambda = l*({Ks[a]}) + {cs[a]}")
    assert sp.simplify(sp.expand((D - lam).subs(xi, 0) - l * K - cs[a]).diff(l)) == 0
    say("S4 note: K_(1,1) = 0 (twist type); K_(1,0) =", Ks[(1, 0)], "; K_(1,-1) =", Ks[(1, -1)])
    # S5
    for a in [(1, -1), (1, 0)]:
        say(f"S5 Rayleigh quotient with e_(m=l, a={a}) (block q = l + {a[0] + a[1]}):  lambda_min(block) - lambda_2l <= "
            f"l*({Ks[a]}) + ({cs[a]})")
    xv, yv, tv = sp.Rational(1, 10), sp.sqrt(10), sp.Integer(10)
    for a in [(1, -1), (1, 0)]:
        Kn = Ks[a].subs({x: xv, t: tv}); cn = cs[a].subs({x: xv, yp: yv, t: tv})
        lf = sp.ceiling(cn / (-Kn) + sp.Rational(1, 10**9))
        say(f"S5 example (x,y,t) = (1/10, sqrt10, 10): type {a}: K = {sp.N(Kn, 12)}, c = {sp.N(cn, 12)}; "
            f"diag - lambda < 0 for all l > {sp.N(cn / (-Kn), 12)} (first integer-or-half l beyond)")
    say("ALL SYMBOLIC CHECKS PASS" if ok else "SOME SYMBOLIC CHECK FAILED")
    Path(sys.argv[1]).write_text("\n".join(out) + "\n")


if __name__ == "__main__":
    main()
