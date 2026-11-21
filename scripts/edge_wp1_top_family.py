"""WP1: the near-top charge blocks q = l + 1 and q = l with the sector j symbolic, and their j -> infinity limit
(21 November 2026).

Why: in the j <= 12 competitor map the competitor is sometimes (12, +-7) = (j, l + 1) or (12, 6) = (j, l), i.e. a
near-top block in the top sector allowed, so the j <= 12 edge there is a cutoff effect. Here those two families are
done for every j at once.

Construction (same rows as scripts/ricci_su2_exact_edge.py, with l = j/2 a symbol): states (m, a) with
m = q - c_f - c_l <= l. For q = l + 1: c_f + c_l >= 1 (3 states, m in {l, l-1}); for q = l: c_f + c_l >= 0
(6 states, m in {l, l-1, l-2}); these are the full blocks for l >= 1/2 (q = l + 1) and l >= 1 (q = l), since then
m >= -l holds automatically (at l = 1/2 the q = l formula carries one spurious state). Ladder factors sqrt((l - m)(l + m + 1)) are kept as symbols r_m; couplings join only adjacent m levels,
so det contains only even powers of each r_m, which are then replaced by (l - m)(l + m + 1) (checked: no odd powers).
Result: D_k(x, y, s, l) = numerator of det(B_{j, l+1-k} - lambda_j I), a polynomial in (x, y, s, l), k = 0, 1.
Checks: (1) the symbolic-l block equals EE.Blocks.block numerically at several j; (2) D_k at l = 6 equals a constant times s times
the main factor of the j = 12 polynomial of research/edge-theorem-20261121/wp1_block_polynomials.json (ratio at rational points);
(3) the grid edges whose j <= 12 competitor is (12, +-7) / (12, +-6) are roots of D_k(., y, s, 6).
Large j: D_k = c_k(x, y, s) l^{d_k} + O(l^{d_k - 1}); the per-family edge tends to the largest real root of c_k in x
(where the other eigenvalues of the block are O(l) away). Edges per j from D_k (sign-change scan of the numeric
f_{j,q} on the same window, refined with brentq) are listed for j up to 2000 next to the root of c_k.
Exact symbolic for D_k and c_k (PROVED as algebra); root/edge numbers double precision (COMPUTED).
usage: OMP_NUM_THREADS=1 python scripts/edge_wp1_top_family.py OUT_PREFIX t:y [t:y ...]
"""
import json
import sys
import time
from pathlib import Path

import mpmath as mp
import numpy as np
import sympy as sp
from scipy.optimize import brentq

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_exact_edge as EE              # noqa: E402
import edge_wp1_block_polynomials as W        # noqa: E402

x, y, t, s = EE.x, EE.y, EE.t, W.s
L = sp.symbols("l", positive=True)
ROOT = Path(__file__).resolve().parent.parent


def family_block(k):
    g = W.ingredients(); w, C, Op, Omn, ch = g["w"], g["C"], g["Op"], g["Omn"], g["ch"]
    q = L + 1 - k
    states = [(q - a - b, n) for n, (a, b) in enumerate(ch) if a + b >= 1 - k]
    idx = {st: n for n, st in enumerate(states)}; n = len(states)
    rsym = {}
    B = sp.zeros(n, n)
    for (mm, a), kk in idx.items():
        B[kk, kk] += (w[a] - 2 * mm / sp.sqrt(t)) ** 2 + 4 * (L * (L + 1) - mm ** 2)
        for (m2, a2), k2 in idx.items():
            if sp.simplify(m2 - mm) == 0:
                B[k2, kk] += C[a2, a]
            if sp.simplify(m2 - mm - 1) == 0:
                r = rsym.setdefault(sp.simplify(L - mm), sp.Symbol(f"r_{sp.simplify(L - mm)}"))   # r for (m, m+1)
                B[k2, kk] += 2 * sp.I * r * Omn[a2, a]
            if sp.simplify(m2 - mm + 1) == 0:
                r = rsym.setdefault(sp.simplify(L - m2), sp.Symbol(f"r_{sp.simplify(L - m2)}"))
                B[k2, kk] += 2 * sp.I * r * Op[a2, a]
    lam = (x + (2 * L + 2) / sp.sqrt(t)) ** 2 + 2 * sp.sqrt(t) * x - y ** 2
    # r_d couples m = l - d and m + 1: (l - m)(l + m + 1) = d (2l + 1 - d)
    rval = {r: d * (2 * L + 1 - d) for d, r in rsym.items()}
    return B, lam, states, rval


def main():
    prefix = Path(sys.argv[1]); pts = [tuple(float(u) for u in a.split(":")) for a in sys.argv[2:]]
    bl = EE.Blocks(); rng = np.random.default_rng(5); res = {"families": {}}; lines = []
    polyj = json.loads((ROOT / "research/edge-theorem-20261121/wp1_block_polynomials.json").read_text())["blocks"]
    for k in (0, 1):
        t0 = time.time(); name = "q=l+1" if k == 0 else "q=l"
        B, lam, states, rval = family_block(k); n = B.shape[0]
        # check 1: numeric block at several j
        worst = 0.0
        for jv in (4, 7, 12, 25, 60):
            lv = sp.Rational(jv, 2)
            subs_r = {r: sp.sqrt(v.subs(L, lv)) for r, v in rval.items()}
            f = sp.lambdify((x, y, t), B.subs(subs_r).subs(L, lv), "numpy")
            for _ in range(2):
                xv, yv, tv = rng.uniform(-1, 1), rng.uniform(0.2, 3), rng.uniform(2, 12)
                _, Bn = bl.block(bl.data(xv, yv, tv), jv, float(lv + 1 - k), tv)
                Bs = np.array(f(xv, yv, tv), complex)
                worst = max(worst, float(np.abs(0.5 * (Bs + Bs.conj().T) - Bn).max()))
        M = (B - lam * sp.eye(n)).subs(t, s ** 2) * s ** 2
        M = M.applyfunc(lambda e: sp.expand(sp.simplify(e)))
        det = sp.expand(M.det(method="berkowitz"))
        rs = list(rval)
        odd = False
        pr = sp.Poly(det, *rs) if rs else None
        if pr is not None:
            for mon in pr.monoms():
                if any(e % 2 for e in mon):
                    odd = True
        det = sp.expand(det.subs({r: sp.sqrt(v) for r, v in rval.items()}))
        assert not det.has(sp.I), "imaginary part left"
        P = sp.Poly(det, x, y, s, L)
        fl = sp.factor_list(det, x, y, s, L)
        lead_deg = sp.Poly(det, L).degree()
        c = sp.factor(sp.Poly(det, L).LC())
        rec = {"block_size": n, "states": [[str(a), b] for a, b in states], "numeric_check_max_abs_diff": worst,
               "odd_powers_of_ladder_symbols": odd, "degrees_(x,y,s,l)": [P.degree(x), P.degree(y), P.degree(s), P.degree(L)],
               "terms": len(P.terms()), "factors": [{"expr": str(f_), "mult": m_,
                                                     "degrees_(x,y,s,l)": [sp.Poly(f_, x, y, s, L).degree(v) for v in (x, y, s, L)]}
                                                    for f_, m_ in fl[1]], "content": str(fl[0]),
               "leading_degree_in_l": lead_deg, "leading_coefficient_c(x,y,s)": str(c), "seconds": time.time() - t0}
        # check 2: proportional to the j = 12 polynomial at l = 6
        key = "j=12,q=7" if k == 0 else "j=12,q=6"
        if key in polyj:
            ex = sp.sympify(max(polyj[key]["factors"], key=lambda f_: f_["total_degree"])["expr"], locals={"x": x, "y": y, "s": s})
            D6 = det.subs(L, 6); rat = []
            for _ in range(3):
                pt = {x: sp.Rational(int(rng.integers(-20, 20)), 7), y: sp.Rational(int(rng.integers(1, 20)), 5),
                      s: sp.Rational(int(rng.integers(10, 40)), 9)}
                rat.append(str(sp.nsimplify(D6.subs(pt) / (pt[s] * ex.subs(pt)))))
            rec["ratio_D(l=6)_to_s_times_j12_factor_at_rational_points"] = rat
        # large-j: edges per j vs root of c
        cpol = sp.Poly(sp.Poly(det, L).LC(), x, y, s)
        rows = []
        for tv, yv in pts:
            sv = float(np.sqrt(tv))
            croots = W.univariate_roots(cpol, yv, sv)
            row = {"t": tv, "y": yv, "real_roots_of_c_in_x": sorted(croots), "edges_by_j": {}}
            for jv in (12, 24, 48, 96, 200, 400, 1000, 2000):
                qv = jv / 2 + 1 - k
                fj = lambda xv: float(np.linalg.eigvalsh(bl.block(bl.data(xv, yv, tv), jv, qv, tv)[1])[0]   # noqa: E731
                                      - EE.lam_tw(xv, yv, tv, jv))
                xs = np.linspace(-3, 1, 401); vals = [fj(xv) for xv in xs]; kk = None
                for i in range(400):
                    if vals[i] >= 0 > vals[i + 1]:
                        kk = i
                row["edges_by_j"][jv] = brentq(fj, xs[kk], xs[kk + 1], xtol=1e-13) if kk is not None else None
            rows.append(row)
            print(name, row, flush=True)
        rec["large_j"] = rows
        res["families"][name] = rec
        lines.append(f"== family {name}: block size {n}; numeric check {worst:.2e}; odd ladder powers {odd}; "
                     f"degrees (x,y,s,l) {rec['degrees_(x,y,s,l)']}; {rec['terms']} terms; {rec['seconds']:.0f} s")
        lines.append(f"   content {fl[0]}; factors:")
        for fr in rec["factors"]:
            lines.append(f"     degrees {fr['degrees_(x,y,s,l)']} mult {fr['mult']}: {fr['expr']}")
        lines.append(f"   leading degree in l: {lead_deg}; leading coefficient c(x, y, s) = {c}")
        if "ratio_D(l=6)_to_s_times_j12_factor_at_rational_points" in rec:
            lines.append(f"   D(l=6) / (s * j=12 factor) at rational points: {rec['ratio_D(l=6)_to_s_times_j12_factor_at_rational_points']}")
        for row in rows:
            lines.append(f"   t={row['t']}, y={row['y']}: real roots of c in x {['%.10f' % r for r in row['real_roots_of_c_in_x']]}; "
                         f"edges by j {({j_: (None if e is None else round(e, 10)) for j_, e in row['edges_by_j'].items()})}")
        print("\n".join(lines[-(4 + len(rec['factors']) + len(rows)):]), flush=True)
    prefix.with_suffix(".json").write_text(json.dumps(res, indent=1, default=str) + "\n")
    prefix.with_suffix(".txt").write_text("\n".join(lines) + "\n")


if __name__ == "__main__":
    main()
