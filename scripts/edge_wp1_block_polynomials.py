"""WP1: closed-form edge polynomials det(B_{j,q} - lambda_j I) for every competitor block (21 November 2026).

For each competitor block (j, q) listed in the competitor map (research/edge-theorem-20261121/wp1_competitor_map.json):
  1. build B_{j,q}(x, y, t) symbolically (same construction as scripts/ricci_su2_exact_edge.py /
     scripts/ricci_su2_edge_polynomial.py, generalised from (4, 0) to any (j, q));
  2. check it numerically against EE.Blocks.block at random points;
  3. M = s^2 (B - lambda_j I) with t = s^2 (s = sqrt t), entries polynomial in (x, y, s) over Q(algebraic numbers
     that occur: collected automatically from the entries);
  4. det over that field with DomainMatrix, expanded to a sympy polynomial P_{j,q}(x, y, s); its coefficients are
     checked to be rational and it is factored over Q (factor_list; irreducibility is over Q only);
  5. at every grid point where (j, q) is the competitor: for each factor, the real roots in x of the univariate
     polynomial at (y, s) are found with mpmath.polyroots (60 digits), and a root is polished by mpmath.findroot
     started at the edge; report the root nearest the edge, its distance
     to the numerically located edge, and the factor value at the edge relative to its sum of |terms| (a scale-free
     residual).
For (4, 0) the full determinant is also compared with research/su2-floor-family-20261119/edge_polynomial.srepr
(ratio at random rational points).
The determinant and factorisation are exact symbolic (PROVED as algebra, given the block construction); the root
comparison is double / 60-digit numerics (COMPUTED).
usage: OMP_NUM_THREADS=1 python scripts/edge_wp1_block_polynomials.py MAP.json OUT_PREFIX [j:q ...]
"""
import json
import sys
import time
from pathlib import Path

import mpmath as mp
import numpy as np
import sympy as sp
from sympy.polys.matrices import DomainMatrix

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_exact_edge as EE              # noqa: E402
from ricci_su2_exact_edge import GC, cross     # noqa: E402

x, y, t = EE.x, EE.y, EE.t
s = sp.symbols("s", positive=True)
ROOT = Path(__file__).resolve().parent.parent
mp.mp.dps = 60

_ING = {}


def ingredients():
    if _ING:
        return _ING
    _, c, _, _, gmat = GC.background(sp.sqrt, sp.Integer(1), t)
    m = [sp.Matrix([x, 0, 0]), sp.Matrix([0, y, 0]), sp.Matrix([0, 0, y])]
    I3 = sp.eye(3)
    Om = [sp.kronecker_product(I3, cross(m[i])) + sp.kronecker_product(-sp.Matrix(gmat[i]), I3) for i in range(3)]
    Ric = sp.diag(2 * t, 4 - 2 * t, 4 - 2 * t)
    T = sp.zeros(9, 9)
    for i in range(3):
        for k in range(3):
            Fik = cross(m[i]) * m[k]
            for l in range(3):
                Fik = Fik - c[(i, k, l)] * m[l]
            T[3 * i:3 * i + 3, 3 * k:3 * k + 3] = cross(Fik)
    R0 = sp.kronecker_product(Ric, I3) - 2 * T
    r2 = sp.sqrt(2)
    U3 = sp.Matrix([[0, 1, 0], [1 / r2, 0, 1 / r2], [-sp.I / r2, 0, sp.I / r2]])
    U = sp.kronecker_product(U3, U3)
    tr = lambda M: (U.H * M * U).applyfunc(sp.simplify)          # noqa: E731
    Omc = [tr(O) for O in Om]
    _ING["w"] = [sp.simplify(Omc[0][a, a] / sp.I) for a in range(9)]
    _ING["Op"] = (Omc[1] + sp.I * Omc[2]).applyfunc(sp.simplify)
    _ING["Omn"] = (Omc[1] - sp.I * Omc[2]).applyfunc(sp.simplify)
    _ING["C"] = (-(Omc[1] * Omc[1] + Omc[2] * Omc[2]) + tr(R0)).applyfunc(sp.simplify)
    _ING["ch"] = [(cf, cl) for cf in (-1, 0, 1) for cl in (-1, 0, 1)]
    return _ING


def symbolic_block(j, q):
    g = ingredients(); w, C, Op, Omn, ch = g["w"], g["C"], g["Op"], g["Omn"], g["ch"]
    l = sp.Rational(j, 2); q = sp.nsimplify(q)
    states = [(q - a - b, k) for k, (a, b) in enumerate(ch) if abs(q - a - b) <= l]
    idx = {st: n for n, st in enumerate(states)}; n = len(states)
    B = sp.zeros(n, n)
    for (mm, a), k in idx.items():
        B[k, k] += (w[a] - 2 * mm / sp.sqrt(t)) ** 2 + 4 * (l * (l + 1) - mm ** 2)
        for (m2, a2), k2 in idx.items():
            if m2 == mm:
                B[k2, k] += C[a2, a]
            if m2 == mm + 1:
                B[k2, k] += 2 * sp.I * sp.sqrt((l - mm) * (l + mm + 1)) * Omn[a2, a]
            if m2 == mm - 1:
                B[k2, k] += 2 * sp.I * sp.sqrt((l + mm) * (l - mm + 1)) * Op[a2, a]
    lam = (x + (j + 2) / sp.sqrt(t)) ** 2 + 2 * sp.sqrt(t) * x - y ** 2
    return B, lam, states


def check_numeric(B, j, q, bl, rng, npts=5):
    f = sp.lambdify((x, y, t), B, "numpy"); worst = 0.0
    for _ in range(npts):
        xv, yv, tv = rng.uniform(-2, 1), rng.uniform(0.2, 3), rng.uniform(2, 12)
        _, Bn = bl.block(bl.data(xv, yv, tv), j, q, tv)
        Bs = np.array(f(xv, yv, tv), complex)
        worst = max(worst, float(np.abs(0.5 * (Bs + Bs.conj().T) - Bn).max()), float(np.abs(Bs - Bs.conj().T).max()))
    return worst


def algebraic_gens(M):
    gens = set()
    for e in M:
        for a in sp.preorder_traversal(e):
            if a.is_Pow and a.exp == sp.Rational(1, 2) and a.base.is_Rational:
                gens.add(a)
            if a == sp.I:
                gens.add(sp.I)
    return sorted(gens, key=sp.default_sort_key)


def scale_free(poly_expr_terms, xv, yv, sv):
    """|f| / sum |term| at the point (60 digits)."""
    tot = mp.mpf(0); ab = mp.mpf(0)
    for (ex, ey, es), cf in poly_expr_terms:
        v = mp.mpf(str(cf)) if cf.is_Rational else mp.mpf(sp.N(cf, 70))
        term = v * mp.mpf(xv) ** ex * mp.mpf(yv) ** ey * mp.mpf(sv) ** es
        tot += term; ab += abs(term)
    return float(abs(tot) / ab) if ab else float("nan")


def univariate_roots(pol, yv, sv):
    """Real roots in x of pol(x, yv, sv) (pol a sympy Poly in x, y, s with real coefficients)."""
    dx = pol.degree(x); co = [mp.mpf(0)] * (dx + 1)
    for (ex, ey, es), cf in pol.terms():
        v = mp.mpf(str(cf)) if cf.is_Rational else mp.mpf(sp.N(cf, 70))
        co[dx - ex] += v * mp.mpf(yv) ** ey * mp.mpf(sv) ** es
    while co and co[0] == 0:
        co = co[1:]
    if len(co) < 2:
        return []
    rts = mp.polyroots(co, maxsteps=800, extraprec=800)
    return [float(mp.re(r)) for r in rts if abs(mp.im(r)) < mp.mpf(10) ** -20 * max(1, abs(r))]


def newton_root(pol, yv, sv, x0):
    """Root of pol(., yv, sv) by mpmath secant/Newton from x0 (60 digits); None if it fails."""
    dx = pol.degree(x); co = [mp.mpf(0)] * (dx + 1)
    for (ex, ey, es), cf in pol.terms():
        co[ex] += mp.mpf(str(cf)) * mp.mpf(yv) ** ey * mp.mpf(sv) ** es
    f = lambda z: mp.fsum(c * z ** k for k, c in enumerate(co))          # noqa: E731
    try:
        return float(mp.findroot(f, mp.mpf(x0), tol=mp.mpf(10) ** -40))
    except Exception:
        return None


def fmt(v):
    return "None" if v is None else f"{v:.3e}"


def main():
    mapd = json.loads(Path(sys.argv[1]).read_text()); prefix = Path(sys.argv[2])
    if len(sys.argv) > 3:
        blocks = [(int(a.split(":")[0]), float(a.split(":")[1])) for a in sys.argv[3:]]
    else:
        blocks = [tuple(K) for K in mapd["competitors_with_sign"]]
    bl = EE.Blocks(); rng = np.random.default_rng(11)
    lines = []; res = {"status": "determinant/factorisation exact symbolic; root comparison numeric", "blocks": {}}
    for (j, q) in blocks:
        t0 = time.time(); key = f"j={j},q={q:g}"
        B, lam, states = symbolic_block(j, q)
        err = check_numeric(B, j, q, bl, rng)
        n = B.shape[0]
        M = (B - lam * sp.eye(n)).subs(t, s ** 2)
        M = (M * s ** 2).applyfunc(lambda e: sp.expand(sp.simplify(e)))
        gens = algebraic_gens(M)
        K = (sp.QQ.algebraic_field(*gens) if gens else sp.QQ)[x, y, s]
        dM = DomainMatrix.from_Matrix(M).convert_to(K)
        det = dM.det()
        P = sp.expand(dM.domain.to_sympy(det))
        Ppol = sp.Poly(P, x, y, s)
        tdet = time.time() - t0
        # the determinant is checked to have rational coefficients; factor over Q (irreducibility claimed over Q only)
        fl = sp.factor_list(P, x, y, s)
        rec = {"block_size": n, "states_(m,channel)": [[str(a), b] for a, b in states],
               "symbolic_vs_Blocks.block_max_abs_diff": err, "algebraic_numbers_in_entries": [str(g) for g in gens],
               "det_domain": str(dM.domain), "det_degrees_(x,y,s)": [Ppol.degree(x), Ppol.degree(y), Ppol.degree(s)],
               "det_terms": len(Ppol.terms()), "det_coeffs_rational": all(cf.is_Rational for cf in Ppol.coeffs()),
               "seconds_det": tdet, "content": str(fl[0]), "factors": []}
        facs = []
        for fac, mult in fl[1]:
            pol = sp.Poly(fac, x, y, s); facs.append(pol)
            rec["factors"].append({"degrees_(x,y,s)": [pol.degree(x), pol.degree(y), pol.degree(s)],
                                   "total_degree": pol.total_degree(), "terms": len(pol.terms()), "mult": mult,
                                   "coeffs_rational": all(cf.is_Rational for cf in pol.coeffs()),
                                   "expr": str(fac)})
        # roots at the grid points where this block (or its mirror) is the competitor
        pts = [r for r in mapd["grid"] if r.get("competitor") and r["competitor"][0] == j
               and abs(r["competitor"][1]) == abs(q)]
        checks = []
        for r in pts:
            xe, yv, tv = r["edge_x"], r["y"], r["t"]; sv = float(np.sqrt(tv))
            row = {"t": tv, "y": yv, "edge_x": xe, "competitor_signed": r["competitor"], "per_factor": []}
            for pol in facs:
                rts = univariate_roots(pol, yv, sv)
                near = min(rts, key=lambda z: abs(z - xe)) if rts else None
                nr = newton_root(pol, yv, sv, xe) if pol.degree(x) > 0 else None
                row["per_factor"].append({"newton_root_from_edge": nr,
                                          "newton_root_minus_edge": (nr - xe) if nr is not None else None,
                                          "nearest_real_root": near,
                                          "root_minus_edge": (near - xe) if near is not None else None,
                                          "scale_free_residual_at_edge": scale_free(pol.terms(), xe, yv, sv),
                                          "largest_real_root_in_[-3,1]": max([z for z in rts if -3 <= z <= 1], default=None)})
            checks.append(row)
        rec["edge_root_checks"] = checks
        if checks:
            best = []
            for i in range(len(facs)):
                d = [abs(c["per_factor"][i]["root_minus_edge"]) for c in checks
                     if c["per_factor"][i]["root_minus_edge"] is not None]
                best.append(max(d) if len(d) == len(checks) else None)
            rec["max_|root-edge|_per_factor"] = best
        if (j, q) == (4, 0.0):
            old = sp.sympify((ROOT / "research/su2-floor-family-20261119/edge_polynomial.srepr").read_text().strip(),
                             locals={"Symbol": sp.Symbol})
            sy = {str(a): a for a in old.free_symbols}
            oldf = sp.lambdify([sy.get("x"), sy.get("y"), sy.get("s")], old)
            newf = sp.lambdify([x, y, s], P)
            ratios = []
            for _ in range(4):
                pt = [sp.Rational(int(rng.integers(-30, 30)), 7), sp.Rational(int(rng.integers(1, 30)), 9),
                      sp.Rational(int(rng.integers(10, 40)), 11)]
                a_, b_ = P.subs({x: pt[0], y: pt[1], s: pt[2]}), old.subs({sy["x"]: pt[0], sy["y"]: pt[1], sy["s"]: pt[2]})
                ratios.append(str(sp.nsimplify(a_ / b_)))
            rec["ratio_to_existing_edge_polynomial_srepr_at_rational_points"] = ratios
        res["blocks"][key] = rec
        n0 = len(lines)
        lines.append(f"== block (j, q) = ({j}, {q:g}); size {n}; symbolic vs numeric block diff {err:.2e}")
        lines.append(f"   algebraic numbers in entries: {gens}; det domain {dM.domain}; det degrees (x,y,s) "
                     f"{rec['det_degrees_(x,y,s)']}, {rec['det_terms']} terms, rational coeffs {rec['det_coeffs_rational']}; "
                     f"{tdet:.1f} s")
        lines.append(f"   content {fl[0]}")
        for k_, fr in enumerate(rec["factors"]):
            lines.append(f"   factor F{k_}: degrees (x,y,s) {fr['degrees_(x,y,s)']}, total {fr['total_degree']}, "
                         f"{fr['terms']} terms, mult {fr['mult']}, rational coeffs {fr['coeffs_rational']}")
            lines.append("      " + fr["expr"])
        for c in checks:
            lines.append(f"   edge check t={c['t']}, y={c['y']}, x*={c['edge_x']:.12f}: " + "; ".join(
                f"F{k_}: newton-edge={fmt(pf['newton_root_minus_edge'])}, nearest-real-root-edge={fmt(pf['root_minus_edge'])}, resid={pf['scale_free_residual_at_edge']:.2e}"
                for k_, pf in enumerate(c["per_factor"])))
        if "ratio_to_existing_edge_polynomial_srepr_at_rational_points" in rec:
            lines.append(f"   ratio to existing edge_polynomial.srepr: {rec['ratio_to_existing_edge_polynomial_srepr_at_rational_points']}")
        print("\n".join(lines[n0:]), flush=True)
        prefix.with_suffix(".json").write_text(json.dumps(res, indent=1, default=str) + "\n")
        prefix.with_suffix(".txt").write_text("\n".join(lines) + "\n")


if __name__ == "__main__":
    main()
