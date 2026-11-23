"""Certificates for the floor at the Berger metric g = (T, 1, 1), any rational T >= 1 (T = 3: the Floor Theorem).
Parametrised copy of scripts/ricci_su2_gap_certificates.py; lambda(j) = lambda_T(j) = [j(j+2) - 2(T-3) j - 2(T-2)(T+4)] / T.

At the Berger metric g = (3, 1, 1), sector of spin j/2. H = Hessian of E = 1/2 |F|^2 against the L2 mass, G = the gauge
map xi -> e^_i xi + [M_i, xi]; X_i = rho_i / sqrt(g_i) + Omega_i. Constructions as scripts/ricci_su2_sector_gauge_check.py
and scripts/ricci_su2_sector_minimality.py.

CERTIFICATE 1 (all j, exact). The Weitzenbock identity H + G G^* = sum_i X_i^* X_i + R0 holds in every sector.
  The rho_i are treated as abstract generators of U(su(2)) with [rho_a, rho_b] = 2 eps_abc rho_c and rho^* = -rho. Both
  sides are 9 x 9 matrices over U(su(2)) of filtration degree <= 2. Every product is put in PBW normal order
  (rho_a rho_b with a <= b), and the coefficients of each PBW monomial are compared exactly (sympy, sqrt 3 symbolic).
  By the PBW theorem the monomials are linearly independent in U(su(2)), so the identity holds there, and therefore in
  every representation, i.e. every j. Also exact: R0, its smallest eigenvalue, and omega^2 = sum_i ||Omega_i||_2^2.

CERTIFICATE 2 (j = 0, 12, 20, 24: the sectors below 30 that survive on the Poincare sphere). The smallest eigenvalue of
  A = H + G G^* is exactly lambda(j) = (j(j+2)-14)/3, and the next is at least lambda(j) + 1/3.
  * A - mu I (mu = lambda(j) -+ 1/3) is assembled in interval arithmetic (the mpmath.iv context: outward-rounded
    endpoints, 200 bits), realified (C^n -> R^2n, [[Re, -Im], [Im, Re]], so every eigenvalue appears twice), in a
    band-preserving order. M = the symmetric midpoint matrix of its lower triangle; rho = sqrt(sum_diag r^2 +
    2 sum_lower r^2) bounds ||exact - M||_2.
  * M is factored LDL^T without pivoting at 200 bits. By Higham (Accuracy and Stability of Numerical Algorithms,
    2nd ed., Thm 9.3), L D L^T = M + dM with |dM| <= gamma_n |L||D||L^T|, gamma_n = n u / (1 - n u), u = 2^-200.
  * Sylvester's law: L D L^T is congruent to D, so M + dM has exactly as many negative eigenvalues as D has negative
    pivots, and no zero eigenvalue (no pivot is zero; all pivots are recorded). With
    eps = rho + gamma_n || |L||D||L^T| ||_F and Weyl's inequality, A has no eigenvalue below mu - eps if the count is
    0, and at most k eigenvalues below mu - eps if the count is k.
  * Counts 0 at lambda - 1/3 and 4 (two complex eigenvalues, doubled by realification) at lambda + 1/3, with eps << 1/3,
    together with lambda(j) being an eigenvalue of multiplicity >= 2 (Proposition: the top-weight eigenvector b; the real
    structure J of the integer-spin representation commutes with A and sends b to the bottom weight), give: the lowest
    eigenvalue is exactly lambda(j), with multiplicity two, and nothing else lies below lambda(j) + 1/3 - eps.

usage: python scripts/ricci_su2_gap_certificates_t.py T OUT.json CUTOFF SECTORS JTAIL
  e.g. T = 4: python scripts/ricci_su2_gap_certificates_t.py 4 out.json 136 0,12,20,24,30,32 36
Error budget: outward-rounded (scripts/ricci_su2_outward.py; repair of the external review, 20 November 2026, finding 1).
"""
import json
import sys
import time
from itertools import product
from pathlib import Path

import mpmath
import sys as _sys
from pathlib import Path as _Path
_sys.path.insert(0, str(_Path(__file__).resolve().parent))
import ricci_su2_outward as O                    # noqa: E402
import sympy as sp

PREC = 200
BR = {}
for i, j, k in [(0, 1, 2), (1, 2, 0), (2, 0, 1)]:
    BR[(i, j, k)] = 2
    BR[(j, i, k)] = -2
PAIRS = [(0, 1), (1, 2), (0, 2)]
EPS3 = {(0, 1, 2): 1, (1, 2, 0): 1, (2, 0, 1): 1, (1, 0, 2): -1, (2, 1, 0): -1, (0, 2, 1): -1}


# ---------------------------------------------------------------- the background, generic arithmetic
T = sp.Rational(sys.argv[1]) if len(sys.argv) > 1 else sp.Integer(3)
CUTOFF = sp.Rational(sys.argv[3]) if len(sys.argv) > 3 else sp.Integer(142)
SECTORS = [int(x) for x in sys.argv[4].split(",")] if len(sys.argv) > 4 else [0, 12, 20, 24]
JTAIL = int(sys.argv[5]) if len(sys.argv) > 5 else 30


def lam_T(j):
    return (j * (j + 2) - 2 * (T - 3) * j - 2 * (T - 2) * (T + 4)) / T


def background(sqrt, one, Tv):
    s = [sqrt(Tv * one), one, one]
    c = {(i, j, k): BR.get((i, j, k), 0) * s[k] / (s[i] * s[j]) for i, j, k in product(range(3), repeat=3)}
    Gam = lambda i, j, k: (c[(i, j, k)] - c[(j, k, i)] + c[(k, i, j)]) / 2       # noqa: E731
    M = [[[Gam(i, jj, k) for jj in range(3)] for k in range(3)] for i in range(3)]   # M[i][k][jj]
    m = [[M[i][2][1], M[i][0][2], M[i][1][0]] for i in range(3)]

    def cross(a, b):
        return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
    f = {}
    for (i, jj) in PAIRS:
        cr = cross(m[i], m[jj])
        f[(i, jj)] = [cr[x] - sum(c[(i, jj, k)] * m[k][x] for k in range(3)) for x in range(3)]
    gmat = [[[Gam(i, a, b) for b in range(3)] for a in range(3)] for i in range(3)]
    return s, c, m, f, gmat


def cross_mat(v):
    return [[0, -v[2], v[1]], [v[2], 0, -v[0]], [-v[1], v[0], 0]]


# ---------------------------------------------------------------- certificate 1: the identity in U(su(2))
class Op:
    """A 9 x 9 (or rows x cols) matrix over U(su(2)): {word: sympy Matrix}, word a tuple of generator indices."""

    def __init__(self, rows, cols, terms=None):
        self.rows, self.cols, self.t = rows, cols, dict(terms or {})

    def add(self, word, mat):
        self.t[word] = self.t.get(word, sp.zeros(self.rows, self.cols)) + mat

    def __add__(self, o):
        r = Op(self.rows, self.cols, self.t)
        for w, m in o.t.items():
            r.add(w, m)
        return r

    def __neg__(self):
        return Op(self.rows, self.cols, {w: -m for w, m in self.t.items()})

    def __sub__(self, o):
        return self + (-o)

    def __mul__(self, o):
        r = Op(self.rows, o.cols)
        for w, a in self.t.items():
            for u, b in o.t.items():
                r.add(w + u, a * b)
        return r

    def dag(self):
        # (C rho_w)^* = C^T rho_w^*; for a word of length L, rho_w^* = (-1)^L reversed(w)
        return Op(self.cols, self.rows, {tuple(reversed(w)): (-1) ** len(w) * m.T for w, m in self.t.items()})

    def normal(self):
        """PBW normal order: rho_a rho_b with a > b -> rho_b rho_a + 2 eps_abc rho_c."""
        r = Op(self.rows, self.cols)
        for w, m in self.t.items():
            if len(w) == 2 and w[0] > w[1]:
                a, b = w
                r.add((b, a), m)
                for cc in range(3):
                    e = EPS3.get((a, b, cc), 0)
                    if e:
                        r.add((cc,), 2 * e * m)
            else:
                r.add(w, m)
        return r


def kron(A, B):
    A, B = sp.Matrix(A), sp.Matrix(B)
    return sp.kronecker_product(A, B)


def certificate_identity():
    s, c, m, f, gmat = background(sp.sqrt, sp.Integer(1), T)
    I3 = sp.eye(3)

    def blockmat(rows, cols, placements):
        M = sp.zeros(3 * rows, 3 * cols)
        for (bi, bj), blk in placements:
            M[3 * bi:3 * bi + 3, 3 * bj:3 * bj + 3] += sp.Matrix(blk)
        return M
    # D: rows (pair p, lie), cols (frame, lie)
    D = Op(9, 9)
    for p, (i, jj) in enumerate(PAIRS):
        D.add((i,), blockmat(3, 3, [((p, jj), I3 / s[i])]))
        D.add((), blockmat(3, 3, [((p, jj), cross_mat(m[i]))]))
        D.add((jj,), blockmat(3, 3, [((p, i), -I3 / s[jj])]))
        D.add((), blockmat(3, 3, [((p, i), -sp.Matrix(cross_mat(m[jj])))]))
        for k in range(3):
            D.add((), blockmat(3, 3, [((p, k), -c[(i, jj, k)] * I3)]))
    crossterm = Op(9, 9)
    for (i, jj) in PAIRS:
        blk = -sp.Matrix(cross_mat(f[(i, jj)]))
        crossterm.add((), blockmat(3, 3, [((i, jj), blk), ((jj, i), blk.T)]))
    Gop = Op(9, 3)
    for i in range(3):
        Gop.add((i,), blockmat(3, 1, [((i, 0), I3 / s[i])]))
        Gop.add((), blockmat(3, 1, [((i, 0), cross_mat(m[i]))]))
    X = []
    for i in range(3):
        om = kron(I3, cross_mat(m[i])) + kron(-sp.Matrix(gmat[i]), I3)
        X.append(Op(9, 9, {(i,): sp.eye(9) / s[i], (): om}))
    lhs = (D.dag() * D) + crossterm + (Gop * Gop.dag())
    rhs = Op(9, 9)
    for x in X:
        rhs = rhs + x.dag() * x
    diff = (lhs - rhs).normal()
    nonconst = {w: sp.simplify(mm) for w, mm in diff.t.items() if len(w) > 0}
    residual_words = [str(w) for w, mm in nonconst.items() if mm != sp.zeros(9, 9)]
    R0 = sp.simplify(diff.t.get((), sp.zeros(9, 9)))
    R0_eigs = {str(sp.nsimplify(sp.simplify(k))): int(v) for k, v in R0.eigenvals().items()}
    R0_min = min(sp.nsimplify(sp.simplify(k)) for k in R0.eigenvals())
    # omega^2 = sum_i ||Omega_i||_2^2, exactly
    om_norms = []
    for i in range(3):
        om = kron(I3, cross_mat(m[i])) + kron(-sp.Matrix(gmat[i]), I3)
        ev = (om.T * om).eigenvals()
        om_norms.append(sp.sqrt(max(sp.nsimplify(sp.simplify(k)) for k in ev)))
    omega2 = sp.nsimplify(sp.simplify(sum(x ** 2 for x in om_norms)))
    # the all-j bound >= CUTOFF from j = JTAIL: sqrt(J(J+2)/T) - omega >= sqrt(CUTOFF - R0_min); it increases with j
    j30 = sp.sqrt(sp.Rational(JTAIL * (JTAIL + 2)) / T) - sp.sqrt(omega2) - sp.sqrt(CUTOFF - R0_min)
    return {"identity_holds_in_U_su2": not residual_words, "nonzero_residual_words": residual_words,
            "R0_is_symmetric": bool(sp.simplify(R0 - R0.T) == sp.zeros(9, 9)),
            "R0_eigenvalues_exact": R0_eigs, "R0_min_exact": str(R0_min),
            "omega_squared_exact": str(omega2), "omega": float(sp.sqrt(omega2)),
            "T": str(T), "cutoff": str(CUTOFF), "jtail": JTAIL,
            "tail_margin_sqrt_form": float(j30), "tail_bound_ge_cutoff_from_jtail": bool(j30 > 0)}


# ---------------------------------------------------------------- certificate 2: rigorous counts
def certificate_counts(j, mu_offsets=(-sp.Rational(1, 3), sp.Rational(1, 3))):
    iv = mpmath.iv
    iv.prec = PREC
    mpmath.mp.prec = PREC
    one = iv.mpf(1)
    s, c, m, f, gmat = background(iv.sqrt, one, iv.mpf(int(T.p)) / iv.mpf(int(T.q)))
    d = j + 1
    half = mpmath.mpf(j) / 2
    mvals = [half - w for w in range(d)]
    # rho_x = -i (Jp + Jp^T), rho_y = -(Jp - Jp^T), rho_z = -2 i Jz ; Jp[w-1, w] = sqrt(s(s+1) - m(m+1))
    jp = {}
    for w in range(1, d):
        mm = mvals[w]
        jp[(w - 1, w)] = iv.sqrt(iv.mpf(half * (half + 1) - mm * (mm + 1)))
    rho = [{}, {}, {}]                       # entries (w, w') -> (re, im)
    for (a, b), v in jp.items():
        rho[0][(a, b)] = (0, -v); rho[0][(b, a)] = (0, -v)
        rho[1][(a, b)] = (-v, 0); rho[1][(b, a)] = (v, 0)
    for w in range(d):
        rho[2][(w, w)] = (0, -2 * iv.mpf(mvals[w]))
    idx = lambda fr, lie, w: w * 9 + fr * 3 + lie       # noqa: E731  band-preserving (w-major)
    n = 9 * d

    def cadd(dct, key, val):
        a = dct.get(key, (0, 0))
        dct[key] = (a[0] + val[0], a[1] + val[1])

    # D rows (p, lie, w); columns (frame, lie, w)
    Dm = {}
    for p, (i, jj) in enumerate(PAIRS):
        for a in range(3):
            for w in range(d):
                row = idx(p, a, w)
                for (sgn, fr, gi, mi) in ((1, jj, i, i), (-1, i, jj, jj)):
                    for (w1, w2), z in rho[gi].items():
                        if w1 == w:
                            cadd(Dm, (row, idx(fr, a, w2)), (sgn * z[0] / s[gi], sgn * z[1] / s[gi]))
                    cm = cross_mat(m[mi])
                    for a2 in range(3):
                        if not (isinstance(cm[a][a2], int) and cm[a][a2] == 0):
                            cadd(Dm, (row, idx(fr, a2, w)), (sgn * cm[a][a2], 0))
                for k in range(3):
                    if not (isinstance(c[(i, jj, k)], int) and c[(i, jj, k)] == 0):
                        cadd(Dm, (row, idx(k, a, w)), (-c[(i, jj, k)], 0))
    Gm = {}                                    # rows (frame, lie, w), cols (lie, w)
    for i in range(3):
        for a in range(3):
            for w in range(d):
                row = idx(i, a, w)
                for (w1, w2), z in rho[i].items():
                    if w1 == w:
                        cadd(Gm, (row, w2 * 3 + a), (z[0] / s[i], z[1] / s[i]))
                cm = cross_mat(m[i])
                for a2 in range(3):
                    if not (isinstance(cm[a][a2], int) and cm[a][a2] == 0):
                        cadd(Gm, (row, w * 3 + a2), (cm[a][a2], 0))

    def gram(S, conj_left):
        """sum_k conj(S[k, x]) S[k, y]  (conj_left) or  sum_k S[x, k] conj(S[y, k])."""
        by = {}
        for (r, cc), z in S.items():
            by.setdefault(r if conj_left else cc, []).append((cc if conj_left else r, z))
        out = {}
        for _, entries in by.items():
            for x, zx in entries:
                for y, zy in entries:
                    if conj_left:   # conj(zx) * zy
                        val = (zx[0] * zy[0] + zx[1] * zy[1], zx[0] * zy[1] - zx[1] * zy[0])
                    else:           # zx * conj(zy)
                        val = (zx[0] * zy[0] + zx[1] * zy[1], zx[1] * zy[0] - zx[0] * zy[1])
                    cadd(out, (x, y), val)
        return out
    A = gram(Dm, True)
    for (x, y), z in gram(Gm, False).items():
        cadd(A, (x, y), z)
    for (i, jj) in PAIRS:
        blk = cross_mat(f[(i, jj)])
        for a, a2, w in product(range(3), range(3), range(d)):
            if not (isinstance(blk[a][a2], int) and blk[a][a2] == 0):
                cadd(A, (idx(i, a, w), idx(jj, a2, w)), (-blk[a][a2], 0))
                cadd(A, (idx(jj, a2, w), idx(i, a, w)), (-blk[a][a2], 0))
    lam = sp.Rational(lam_T(j))
    results = {}
    for off in mu_offsets:
        mu = lam + off
        mu_iv = iv.mpf(mpmath.mpf(mu.p) / mpmath.mpf(mu.q)) if False else iv.mpf(mu.p) / iv.mpf(mu.q)
        # realify: C^n -> R^2n, index 2x (re), 2x+1 (im)
        N = 2 * n
        mid, bud = {}, O.Budget()
        Ar = {}
        for (x, y), z in A.items():
            re, im = z
            for (r_, c_, v) in ((2 * x, 2 * y, re), (2 * x, 2 * y + 1, -im if not isinstance(im, int) else 0),
                                (2 * x + 1, 2 * y, im), (2 * x + 1, 2 * y + 1, re)):
                if isinstance(v, int) and v == 0:
                    continue
                Ar[(r_, c_)] = Ar.get((r_, c_), 0) + v
        for x in range(N):
            Ar[(x, x)] = Ar.get((x, x), 0) - mu_iv
        band = 0
        # M is the symmetric matrix built from the LOWER triangle (and diagonal) of the enclosure: that is exactly
        # what the factorization reads. The exact matrix is symmetric and each lower interval encloses its entry, so
        # ||exact - M||_2 <= ||exact - M||_F <= sqrt(sum_diag r^2 + 2 sum_lower r^2) =: rho.
        for (r_, c_), v in Ar.items():
            band = max(band, abs(r_ - c_))
            if r_ < c_:
                continue
            # stored midpoint and OUTWARD radius about it (ricci_su2_outward; the external review, finding 1)
            m_, r1 = O.mid_rad(v if not isinstance(v, int) else iv.mpf(v))
            mid[(r_, c_)] = m_
            bud.add(r_, c_, r1)
        # LDL^T without pivoting, banded, at PREC bits
        Lrows = [dict() for _ in range(N)]
        dvals = []
        for k in range(N):
            lo = max(0, k - band)
            dk = mid.get((k, k), mpmath.mpf(0)) - mpmath.fsum(Lrows[k].get(t, 0) ** 2 * dvals[t] for t in range(lo, k))
            dvals.append(dk)
            if dk == 0:
                raise RuntimeError("zero pivot: refusing to certify")
            for i2 in range(k + 1, min(N, k + band + 1)):
                aik = mid.get((i2, k), mpmath.mpf(0))
                sm = mpmath.fsum(Lrows[i2].get(t, 0) * Lrows[k].get(t, 0) * dvals[t] for t in range(max(lo, i2 - band), k))
                if aik != 0 or sm != 0:
                    Lrows[i2][k] = (aik - sm) / dk
            Lrows[k][k] = mpmath.mpf(1)
        # || |L||D||L^T| ||_F, outward
        terms = {}
        for i2 in range(N):
            for t, lit in Lrows[i2].items():
                for k2 in range(max(0, t), min(N, t + band + 1)):
                    lkt = Lrows[k2].get(t)
                    if lkt is not None:
                        terms.setdefault((i2, k2), []).append((lit, dvals[t], lkt))
        # a posteriori residual of the stored factors, outward (replaces the Higham-type bound)
        res_iv = O.residual_frob(N, lambda i2, k2: mid.get((i2, k2), mpmath.mpf(0)), Lrows, dvals, band)
        rho = mpmath.mpf(O.up(mpmath.iv.sqrt(bud.rad2)))          # Frobenius bound, as before
        eps, bwd = O.eps_residual(rho, res_iv)
        neg = sum(1 for dv in dvals if dv < 0)
        results[str(off)] = {"mu": str(mu), "negative_pivots_realified": neg,
                             "eps": mpmath.nstr(eps, 5), "eps_below_one_third": bool(eps < mpmath.mpf(1) / 3), "rho_enclosure": mpmath.nstr(rho, 5),
                             "residual_bound": mpmath.nstr(bwd, 5),
                             "min_abs_pivot": mpmath.nstr(min(abs(dv) for dv in dvals), 5),
                             "negative_pivot_positions": [k for k, dv in enumerate(dvals) if dv < 0],
                             "pivots": [mpmath.nstr(dv, 8) for dv in dvals], "band": band, "N": N}
    lo_ok = results[str(mu_offsets[0])]["negative_pivots_realified"] == 0
    hi_ok = results[str(mu_offsets[1])]["negative_pivots_realified"] == 4
    eps_ok = all(r["eps_below_one_third"] for r in results.values())
    return {"j": j, "lambda": str(lam), "counts": results, "certified": bool(lo_ok and hi_ok and eps_ok)}


def main():
    out = Path(sys.argv[2]) if len(sys.argv) > 2 else None
    t0 = time.time()
    res = {"kind": "certificates for the gap theorem (exact identity in U(su(2)); rigorous inertia counts)",
           "precision_bits": PREC}
    res["certificate_1"] = certificate_identity()
    print(json.dumps(res["certificate_1"], indent=1), flush=True)
    res["certificate_2"] = []
    for j in SECTORS:
        r = certificate_counts(j)
        res["certificate_2"].append(r)
        print(json.dumps(r), flush=True)
    res["all_certified"] = (res["certificate_1"]["identity_holds_in_U_su2"] and res["certificate_1"]["tail_bound_ge_cutoff_from_jtail"]
                            and all(r["certified"] for r in res["certificate_2"]))
    res["seconds"] = round(time.time() - t0, 1)
    print("ALL CERTIFIED" if res["all_certified"] else "NOT CERTIFIED", res["seconds"], "s", flush=True)
    if out:
        if out.exists():
            raise SystemExit(f"refusing to overwrite {out}")
        out.write_text(json.dumps(res, indent=1) + "\n")


if __name__ == "__main__":
    import _receipt_status                     # exit 1 if the receipt is missing, stale or records a failed check
    _receipt_status.begin()
    main()
    _receipt_status.exit_on_failure(sys.argv[1])
