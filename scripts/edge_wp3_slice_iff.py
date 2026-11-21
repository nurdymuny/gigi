"""WP3: the 'iff' at the edge on a 1-parameter slice (21 November 2026).

Slice: t = 3 (s = sqrt 3) and y in {sqrt 3, 3/2}; x varies. Claim to prove on [-3, x* + delta]:
  m(x) = lambda_min(B_{4,0}(x)) - lambda_4(x)  is > 0 on [-3, x*), = 0 at x*, < 0 on (x*, r_next),
with x* the root of P2(x, y, sqrt 3) (research/su2-floor-family-20261119/edge_polynomial.srepr), and every OTHER
non-twist block of sectors j <= 12 stays strictly above its sector's twist value on [-3, x* + delta].

Method (all steps exact or rigorous interval):
  E1  Build the slice blocks B_{j,q}(x) exactly in sympy (same construction as scripts/ricci_su2_exact_edge.py, with
      t = 3 and y substituted exactly). Check exactly: real symmetric; the q = -(l+2) block equals the twist value.
  E2  D(x) = det(B_{4,0}(x) - lambda_4(x) I) exactly over Q(sqrt2, sqrt3)[x]; check 3^9 D == P(x, y, sqrt3) exactly
      (P = full srepr polynomial = det of s^2 (B - lambda I)), and P = c * P1 * P2 (factor_list of the srepr).
  E3  Root isolation: P2_slice = A(x) + sqrt3 B(x), A, B in Q[x]; norm N = A^2 - 3 B^2 in Q[x]; sympy isolates the
      real roots of N with rational endpoints; a root of N belongs to P2_slice iff P2_slice changes sign across its
      isolating interval (signs of a + b sqrt3 decided exactly). Same for P1_slice. Squarefreeness of N => every root
      simple. Interval enclosure of P2' on the isolating interval excludes 0 (transversality of det).
  E4  Rigorous interval LDL^T (Cholesky) of B_{4,0}(-3) - lambda_4(-3) I => positive definite at x = -3.
      Argument: D = prod_i (mu_i - lambda) with mu_i analytic eigen-branches (Rellich). D has no zero on [-3, x*),
      so no branch crosses lambda there; all branches > lambda at -3 => m > 0 on [-3, x*). At x* D has a simple zero,
      so exactly one branch has a simple zero and the others are nonzero: that branch changes sign, m < 0 on
      (x*, r_next) where r_next is the next real zero of D.
  E5  Every other block (j <= 12, q non-twist, (j,q) != (4,0)): interval Cholesky of B_{j,q}(X) - lambda_j(X) I on
      sub-intervals X covering [-3, x* + delta], with adaptive bisection. Success of interval Cholesky => every
      symmetric matrix in the interval hull is positive definite (Alefeld-Mayer), hence lambda_min > lambda_j on X.
usage: python scripts/edge_wp3_slice_iff.py OUTDIR
"""
import json
import sys
import time
from pathlib import Path

import mpmath
import numpy as np
import sympy as sp
from sympy.polys.matrices import DomainMatrix

sys.path.insert(0, str(Path(__file__).resolve().parent))
_argv, sys.argv = sys.argv, [sys.argv[0]]
import ricci_su2_exact_edge as EE              # noqa: E402
sys.argv = _argv

iv = mpmath.iv
iv.prec = 96
ROOT = Path(__file__).resolve().parent.parent
X, Y, T = EE.x, EE.y, EE.t
S = sp.symbols("s", positive=True)
JMAX = 12
SQ3 = sp.sqrt(3)


# ----------------------------------------------------------------------------------------------- E1 exact blocks
def symbolic_data():
    from ricci_su2_exact_edge import GC, cross
    _, c, _, _, gmat = GC.background(sp.sqrt, sp.Integer(1), T)
    m = [sp.Matrix([X, 0, 0]), sp.Matrix([0, Y, 0]), sp.Matrix([0, 0, Y])]
    I3 = sp.eye(3)
    Om = [sp.kronecker_product(I3, cross(m[i])) + sp.kronecker_product(-sp.Matrix(gmat[i]), I3) for i in range(3)]
    Ric = sp.diag(2 * T, 4 - 2 * T, 4 - 2 * T)
    Tm = sp.zeros(9, 9)
    for i in range(3):
        for k in range(3):
            F = cross(m[i]) * m[k]
            for l in range(3):
                F = F - c[(i, k, l)] * m[l]
            Tm[3 * i:3 * i + 3, 3 * k:3 * k + 3] = cross(F)
    R0 = sp.kronecker_product(Ric, I3) - 2 * Tm
    r2 = sp.sqrt(2)
    U3 = sp.Matrix([[0, 1, 0], [1 / r2, 0, 1 / r2], [-sp.I / r2, 0, sp.I / r2]])
    U = sp.kronecker_product(U3, U3)
    tr = lambda M: (U.H * M * U).applyfunc(sp.simplify)          # noqa: E731
    Omc = [tr(O) for O in Om]
    w = [sp.simplify(Omc[0][a, a] / sp.I) for a in range(9)]
    Op, Omn = (Omc[1] + sp.I * Omc[2]).applyfunc(sp.simplify), (Omc[1] - sp.I * Omc[2]).applyfunc(sp.simplify)
    C = (-(Omc[1] * Omc[1] + Omc[2] * Omc[2]) + tr(R0)).applyfunc(sp.simplify)
    ch = [(cf, cl) for cf in (-1, 0, 1) for cl in (-1, 0, 1)]
    return ch, w, C, Op, Omn


def lam_sym(j, yv, tv):
    return (X + sp.Integer(j + 2) / sp.sqrt(tv)) ** 2 + 2 * sp.sqrt(tv) * X - yv ** 2


def slice_block(data, j, q, yv, tv):
    """Exact B_{j,q}(x) at (y, t) = (yv, tv); q a sympy Rational. Returns (states, Matrix)."""
    ch, w, C, Op, Omn = data
    sub = {Y: yv, T: tv}
    l = sp.Rational(j, 2)
    states = [(q - a - b, k) for k, (a, b) in enumerate(ch) if abs(q - a - b) <= l]
    idx = {st: n for n, st in enumerate(states)}
    n = len(states)
    B = sp.zeros(n, n)
    for (mm, a), k in idx.items():
        B[k, k] += (w[a].subs(sub) - 2 * mm / sp.sqrt(tv)) ** 2 + 4 * (l * (l + 1) - mm ** 2)
        for (m2, a2), k2 in idx.items():
            if m2 == mm:
                B[k2, k] += C[a2, a].subs(sub)
            if m2 == mm + 1:
                B[k2, k] += 2 * sp.I * sp.sqrt((l - mm) * (l + mm + 1)) * Omn[a2, a].subs(sub)
            if m2 == mm - 1:
                B[k2, k] += 2 * sp.I * sp.sqrt((l + mm) * (l - mm + 1)) * Op[a2, a].subs(sub)
    B = B.applyfunc(lambda e: sp.expand(sp.radsimp(sp.expand(e))))
    return states, B


# ------------------------------------------------------------------------------------- rigorous sympy -> interval
def to_iv(e):
    e = sp.sympify(e)
    if e.is_Integer:
        return iv.mpf(int(e))
    if e.is_Rational:
        return iv.mpf(int(e.p)) / iv.mpf(int(e.q))
    if e.is_Add:
        r = iv.mpf(0)
        for a in e.args:
            r = r + to_iv(a)
        return r
    if e.is_Mul:
        r = iv.mpf(1)
        for a in e.args:
            r = r * to_iv(a)
        return r
    if e.is_Pow:
        b, ex = e.args
        if ex.is_Integer:
            return to_iv(b) ** int(ex)
        if ex == sp.Rational(1, 2):
            return iv.sqrt(to_iv(b))
        if ex == sp.Rational(-1, 2):
            return 1 / iv.sqrt(to_iv(b))
    raise ValueError(f"to_iv: unsupported {e!r}")


def iv_poly_coeffs(e):
    """Real polynomial in X with algebraic coefficients -> list of interval coefficients (ascending)."""
    p = sp.Poly(sp.expand(e), X)
    cs = p.all_coeffs()[::-1]
    return [to_iv(c) for c in cs]


def iv_eval(coeffs, Xi):
    r = iv.mpf(0)
    for k, c in enumerate(coeffs):
        r = r + c * (Xi ** k if k else iv.mpf(1))
    return r


def iv_cholesky_pd(A):
    """Interval Cholesky (real symmetric interval matrix, lower triangle used). True => all members PD."""
    n = len(A)
    L = [[None] * n for _ in range(n)]
    minpiv = None
    for k in range(n):
        d = A[k][k]
        for i in range(k):
            d = d - L[k][i] ** 2
        if not d.a > 0:
            return False, None
        minpiv = d.a if minpiv is None else min(minpiv, d.a)
        L[k][k] = iv.sqrt(d)
        for r in range(k + 1, n):
            v = A[r][k]
            for i in range(k):
                v = v - L[r][i] * L[k][i]
            L[r][k] = v / L[k][k]
    return True, minpiv


class IvBlock:
    """Interval evaluator of A(X) = B_{j,q}(X) - lambda_j(X) I, entries polynomials in X."""

    def __init__(self, B, lam):
        n = B.shape[0]
        self.n = n
        self.co = [[iv_poly_coeffs(B[r, c] - (lam if r == c else 0)) for c in range(r + 1)] for r in range(n)]

    def __call__(self, Xi):
        A = [[None] * self.n for _ in range(self.n)]
        for r in range(self.n):
            for c in range(r + 1):
                A[r][c] = iv_eval(self.co[r][c], Xi)
                A[c][r] = A[r][c]
        return A


def certify_on(ivb, lo, hi, depth_max=18, w0=0.25):
    """Cover [lo, hi] (mpf endpoints) by sub-intervals each passing interval Cholesky. Returns stats or failure."""
    stack = []
    a = mpmath.mpf(lo)
    while a < hi:
        b = min(a + w0, mpmath.mpf(hi))
        stack.append((a, b, 0))
        a = b
    n_ok, worst_depth, minpiv = 0, 0, None
    while stack:
        a, b, d = stack.pop()
        ok, mp_ = iv_cholesky_pd(ivb(iv.mpf([a, b])))
        if ok:
            n_ok += 1
            worst_depth = max(worst_depth, d)
            minpiv = mp_ if minpiv is None else min(minpiv, mp_)
            continue
        if d >= depth_max:
            return {"ok": False, "failed_at": [float(a), float(b)], "depth": d}
        mid = (a + b) / 2
        stack += [(a, mid, d + 1), (mid, b, d + 1)]
    return {"ok": True, "pieces": n_ok, "max_depth": worst_depth, "min_pivot_lower": float(minpiv)}


# ----------------------------------------------------------------------------------------------- exact sign tools
def split_q3(p):
    """p in Q(sqrt3)[x] -> (A, B) in Q[x] with p = A + sqrt3 B (exact)."""
    pe = sp.expand(p)
    col = sp.collect(pe, SQ3, evaluate=False)
    Ap = sp.expand(col.get(sp.Integer(1), 0))
    Bp = sp.expand(col.get(SQ3, 0))
    assert sp.expand(Ap + SQ3 * Bp - pe) == 0
    for q in (Ap, Bp):
        assert not sp.sympify(q).has(sp.sqrt(2)) and not sp.sympify(q).has(SQ3), "not in Q(sqrt3)[x]"
    return sp.Poly(Ap, X, domain="QQ"), sp.Poly(Bp, X, domain="QQ")


def sign_q3(a, b):
    """exact sign of a + b sqrt3, a, b rational."""
    a, b = sp.Rational(a), sp.Rational(b)
    if b == 0:
        return int(sp.sign(a))
    if a == 0:
        return int(sp.sign(b))
    if sp.sign(a) == sp.sign(b):
        return int(sp.sign(a))
    # opposite signs: sign follows the larger of |a|, |b| sqrt3
    return int(sp.sign(a)) if a * a > 3 * b * b else int(sp.sign(b))


def sign_at(AB, r):
    A, B = AB
    return sign_q3(A.eval(r), B.eval(r))


def isolate_roots(AB, lo, hi, tol=sp.Rational(1, 10 ** 30)):
    """Real roots in (lo, hi) of p = A + sqrt3 B. Returns list of rational isolating intervals (each: exactly one
    root of p, simple). Exact."""
    A, B = AB
    N = A ** 2 - 3 * B ** 2
    g = sp.gcd(N, N.diff(X))
    sqf = g.degree() == 0
    Nr = sp.quo(N, g) if not sqf else N
    out = []
    for (a, b), mult in Nr.intervals(eps=tol):
        a, b = sp.Rational(a), sp.Rational(b)
        if b < lo or a > hi:
            continue
        if a == b:
            if sign_at(AB, a) == 0:
                out.append((a, b))
            continue
        sa, sb = sign_at(AB, a), sign_at(AB, b)
        if sa * sb < 0:
            out.append((a, b))
    return out, sqf, N


def poly_iv(AB, Xi):
    A, B = AB
    ca = [to_iv(c) for c in A.all_coeffs()[::-1]]
    cb = [to_iv(c) for c in B.all_coeffs()[::-1]]
    return iv_eval(ca, Xi) + iv.sqrt(3) * iv_eval(cb, Xi)


# ---------------------------------------------------------------------------------------------------------- main
def run_slice(data, yv, ylabel, P, P1, P2, Pfac_const, out):
    tv = sp.Integer(3)
    t0 = time.time()
    rec = {"t": 3, "y": ylabel}
    # E1: blocks of (4,0) and twists
    st40, B40 = slice_block(data, 4, sp.Integer(0), yv, tv)
    lam4 = sp.expand(lam_sym(4, yv, tv))
    herm = all(sp.simplify(B40[i, k] - sp.conjugate(B40[k, i])) == 0 for i in range(9) for k in range(9))
    real = all(sp.simplify(sp.im(e)) == 0 for e in B40)
    rec["E1_B40_exactly_real_symmetric"] = bool(herm and real)
    tw = []
    for j in range(JMAX + 1):
        l = sp.Rational(j, 2)
        for q in (l + 2, -(l + 2)):
            _, Bt = slice_block(data, j, q, yv, tv)
            tw.append(bool(Bt.shape == (1, 1) and sp.simplify(Bt[0, 0] - lam_sym(j, yv, tv)) == 0))
    rec["E1_both_max_charge_blocks_equal_lambda_j_exactly_j<=12"] = all(tw)
    # numeric cross-check vs EE.Blocks (double precision)
    bl = EE.Blocks()
    diffs = []
    for xv in (-2.5, -1.0, -0.37):
        d = bl.data(xv, float(yv), 3.0)
        for (j, q) in ((4, 0.0), (1, 1.5), (5, 0.5), (12, -3.0)):
            _, Bn = bl.block(d, j, q, 3.0)
            _, Bs = slice_block(data, j, sp.nsimplify(q), yv, tv)
            Bs_n = np.array(Bs.subs(X, xv).evalf(), dtype=complex)
            diffs.append(float(np.abs(Bs_n - Bn).max()))
    rec["E1_check_exact_blocks_vs_EE_Blocks_maxabs_COMPUTED"] = max(diffs)
    print(ylabel, "E1 done", time.time() - t0, rec, flush=True)

    # E2: exact determinant on the slice, compare to P
    M = (B40 - lam4 * sp.eye(9)).applyfunc(sp.expand)
    K = sp.QQ.algebraic_field(sp.sqrt(2), SQ3)[X]
    dM = DomainMatrix.from_Matrix(M).convert_to(K)
    D = sp.expand(dM.domain.to_sympy(dM.det()))
    D = sp.expand(sp.radsimp(D))
    Pslice = sp.expand(P.subs({Y: yv, S: SQ3}))
    rec["E2_P_slice_equals_3^9_det_exactly"] = bool(sp.expand(Pslice - 3 ** 9 * D) == 0)
    P1s = sp.expand(P1.subs({Y: yv, S: SQ3}))
    P2s = sp.expand(P2.subs({Y: yv, S: SQ3}))
    rec["E2_P_equals_const_P1_P2_const"] = str(Pfac_const)
    print(ylabel, "E2 done", time.time() - t0, rec["E2_P_slice_equals_3^9_det_exactly"], flush=True)

    # E3: root isolation
    lo, hi = sp.Integer(-3), sp.Integer(1)
    AB2, AB1 = split_q3(P2s), split_q3(P1s)
    r2, sqf2, N2 = isolate_roots(AB2, lo, hi)
    r1, sqf1, N1 = isolate_roots(AB1, lo, hi)
    rec["E3_norm_P2_squarefree"] = bool(sqf2)
    rec["E3_norm_P1_squarefree"] = bool(sqf1)
    rec["E3_P2_roots_in_[-3,1]"] = [[str(a), str(b), float(a)] for a, b in r2]
    rec["E3_P1_roots_in_[-3,1]"] = [[str(a), str(b), float(a)] for a, b in r1]
    roots = sorted([(a, b, "P2") for a, b in r2] + [(a, b, "P1") for a, b in r1])
    assert roots, "no real root of P1 P2 in [-3, 1]"
    # the edge: the smallest root of D = c P1 P2 in [-3, 1]
    xa, xb, fac = roots[0]
    rec["E3_first_root_of_det_in_[-3,1]_factor"] = fac
    rec["xstar_isolating_interval"] = [str(xa), str(xb)]
    rec["xstar_float"] = float((xa + xb) / 2)
    rec["xstar_width"] = float(xb - xa)
    nxt = roots[1] if len(roots) > 1 else None
    rec["next_root_of_det_after_xstar"] = None if nxt is None else {"interval": [str(nxt[0]), str(nxt[1])],
                                                                     "float": float(nxt[0]), "factor": nxt[2]}
    # the isolating intervals do not overlap (so the ordering is rigorous)
    rec["E3_isolating_intervals_disjoint"] = all(roots[i][1] < roots[i + 1][0] for i in range(len(roots) - 1))
    # transversality: interval enclosure of P2' on the isolating interval (widened to [xa - 1e-6, xb + 1e-6])
    ABd = (AB2[0].diff(X), AB2[1].diff(X)) if fac == "P2" else (AB1[0].diff(X), AB1[1].diff(X))
    Xi = iv.mpf([mpmath.mpf(xa) - mpmath.mpf("1e-6"), mpmath.mpf(xb) + mpmath.mpf("1e-6")])
    dP = poly_iv(ABd, Xi)
    rec["E3_dP2dx_enclosure_on_xstar+-1e-6"] = [float(dP.a), float(dP.b)]
    rec["E3_dP2dx_excludes_0"] = bool(dP.a > 0 or dP.b < 0)
    # the other factor is nonzero at x*: interval enclosure
    other = AB1 if fac == "P2" else AB2
    oth = poly_iv(other, Xi)
    rec["E3_other_factor_enclosure_at_xstar"] = [float(oth.a), float(oth.b)]
    rec["E3_other_factor_nonzero_at_xstar"] = bool(oth.a > 0 or oth.b < 0)
    # compare with the double-precision edge of exact_edge.json
    print(ylabel, "E3 done", time.time() - t0, rec["xstar_float"], rec["next_root_of_det_after_xstar"], flush=True)

    # E4: B40 - lambda4 positive definite at x = -3 (point interval)
    ivb40 = IvBlock(B40, lam4)
    ok, mp_ = iv_cholesky_pd(ivb40(iv.mpf(-3)))
    rec["E4_B40_minus_lam4_PD_at_x=-3"] = bool(ok)
    rec["E4_min_pivot_lower"] = float(mp_) if ok else None
    # cross-check by interval Cholesky away from the edge (redundant, independent of E3)
    c40 = certify_on(ivb40, mpmath.mpf(-3), mpmath.mpf(xa) - mpmath.mpf("0.02"))
    rec["E4b_B40_interval_Cholesky_on_[-3,xstar-0.02]"] = c40
    # numeric margin just above x* and at the next root (COMPUTED)
    d = bl.data(float(xb) + 1e-3, float(yv), 3.0)
    _, Bn = bl.block(d, 4, 0.0, 3.0)
    rec["COMPUTED_m(xstar+1e-3)"] = float(np.linalg.eigvalsh(Bn)[0] - EE.lam_tw(float(xb) + 1e-3, float(yv), 3.0, 4))
    print(ylabel, "E4 done", time.time() - t0, ok, c40, flush=True)

    # E5: every other block of j <= 12 on [-3, x* + delta]
    delta = mpmath.mpf("0.02")
    hi5 = mpmath.mpf(xb) + delta
    rec["E5_interval"] = [-3.0, float(hi5)]
    res5, allok = {}, True
    for j in range(JMAX + 1):
        l = sp.Rational(j, 2)
        lamj = sp.expand(lam_sym(j, yv, tv))
        q = -l - 2
        while q <= l + 2:
            if abs(q) != l + 2 and not (j == 4 and q == 0):
                _, Bq = slice_block(data, j, q, yv, tv)
                if Bq.shape[0]:
                    assert all(sp.simplify(sp.im(e)) == 0 for e in Bq), "block not real"
                    r = certify_on(IvBlock(Bq, lamj), mpmath.mpf(-3), hi5)
                    res5[f"{j},{q}"] = r
                    allok &= r["ok"]
                    if not r["ok"]:
                        print("FAIL", j, q, r, flush=True)
            q += 1
        print(ylabel, "E5 j =", j, "ok so far:", allok, time.time() - t0, flush=True)
    rec["E5_all_other_blocks_certified"] = bool(allok)
    rec["E5_blocks"] = res5
    rec["E5_min_pivot_lower_over_blocks"] = min(r["min_pivot_lower"] for r in res5.values() if r["ok"])
    rec["seconds"] = time.time() - t0
    out[ylabel] = rec


def main():
    outdir = Path(sys.argv[1]); outdir.mkdir(parents=True, exist_ok=True)
    t0 = time.time()
    srepr = (ROOT / "research/su2-floor-family-20261119/edge_polynomial.srepr").read_text()
    P = sp.expand(eval(srepr, {k: getattr(sp, k) for k in dir(sp) if not k.startswith("_")}))
    syms = {str(v): v for v in P.free_symbols}
    P = P.subs({syms["x"]: X, syms["y"]: Y, syms["s"]: S})
    const, fl = sp.factor_list(P, X, Y, S)
    facs = sorted(fl, key=lambda f: sp.Poly(f[0], X, Y, S).degree(X))
    assert len(facs) == 2 and all(m_ == 1 for _, m_ in facs)
    P1, P2 = facs[0][0], facs[1][0]
    degs = [sp.Poly(f, X, Y, S).degree_list() for f in (P1, P2)]
    print("P loaded, factors", degs, "const", const, time.time() - t0, flush=True)
    data = symbolic_data()
    out = {"P1_degrees_xys": list(degs[0]), "P2_degrees_xys": list(degs[1]), "P_const": str(const),
           "P1_even_in_y": bool(sp.expand(P1 - P1.subs(Y, -Y)) == 0),
           "P2_even_in_y": bool(sp.expand(P2 - P2.subs(Y, -Y)) == 0)}
    ref = json.loads((ROOT / "research/su2-floor-family-20261119/exact_edge.json").read_text())
    for yv, lab in ((SQ3, "sqrt3"), (sp.Rational(3, 2), "1.5")):
        run_slice(data, yv, lab, P, P1, P2, const, out)
        for r in ref["edges"]:
            if r["t"] == 3.0 and r["height"] == ("sqrt_t" if lab == "sqrt3" else "1.5"):
                out[lab]["exact_edge.json_edge_A"] = r["edge_A"]
                out[lab]["xstar_minus_edge_A_COMPUTED"] = out[lab]["xstar_float"] - r["edge_A"]
        (outdir / "wp3_slice_iff.json").write_text(json.dumps(out, indent=1, default=str) + "\n")
    print("total", time.time() - t0)


if __name__ == "__main__":
    main()
