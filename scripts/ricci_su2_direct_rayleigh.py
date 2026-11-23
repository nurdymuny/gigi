"""Direct Rayleigh quotients from the operator definition (22 November 2026; review finding S5-1, option (b)).

Independent of the closed-form charge-block rows: the operator H + GG^* is built symbolically from Q by the sympy port of
the general builder (scripts/ricci_su2_lower_edge_closed_form.py::sym_hessian_parts, itself a line-for-line port of
scripts/ricci_su3_test2_family.py::hessian_and_gauge), with x, y real and s = sqrt t > 0 symbolic.

Representation. spin_rep uses rho = (-2i Jx, -2i Jy, -2i Jz). Here rho' = (-2i Jz, -2i Jx, -2i Jy), exact sympy
matrices; (Jx, Jy, Jz) -> (Jz, Jx, Jy) is a cyclic permutation of axes (a rotation), so rho' = W rho W^* for a unitary
W on V_j, the operators built from rho and rho' are conjugate by I (x) W, and Rayleigh quotients of corresponding states
agree. With rho', the weight vectors of rho'_0 are the standard basis vectors.

Checks, exact (sympy), for j = 0..JMAX:
  (i)   the twist: e_tw = u_s (x) u_s (x) v (v a top weight vector of rho'_0, s = +-1) satisfies (H+GG^*) e_tw =
        lambda_j e_tw, lambda_j = (x + (j+2)/s)^2 + 2 s x - y^2 (the sign choice that does it is recorded);
  (ii)  e_- = u_s (x) u_{-s} (x) v:  <e_-, K e_-> - lambda_j = -8 l x/s - 4 s x + 4 y^2 - 8 x/s   (Thm xpos item 1);
  (iii) e_0 = u_s (x) u_0 (x) v:    <e_0, K e_0> - lambda_j = -4 l x/s - 2 s x - x^2 + 3 y^2 - 4 x/s  (item 2);
  (iv)  j = 0: v0 = theta^0 (x) tau_0: <v0, K v0> = <v0, H v0> = 4t + 2y^2 and G^* v0 = 0 (Prop lower item 2).
From (ii),(iii) at j = 0..JMAX to all l: <e, K e> is a linear combination, with coefficients independent of l, of
1, v^* rho'_i v and v^* rho'_i^* rho'_k v (K is quadratic in rho), and for the top vector v = |l, +-l> these moments are
polynomials in l of degree <= 2 (<Jz> = +-l, <Jz^2> = l^2, <Jx^2> = <Jy^2> = l/2, <Jx Jy> = -<Jy Jx> = i l/2 (sign with
the weight), all others 0). The moments themselves are checked here for each j. Both sides are therefore polynomials in l
of degree <= 2, and agreement at JMAX + 1 >= 3 values of l proves the identity for every l in (1/2)Z_{>=0}.
A double-precision cross-check against the unmodified numpy builder with spin_rep (s5_rq-style) is included.
usage: python scripts/ricci_su2_direct_rayleigh.py OUT.json [JMAX]
"""
import json
import sys
from pathlib import Path

import numpy as np
import sympy as sp

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_lower_edge_closed_form as LE      # noqa: E402

x, y, s, I = LE.x, LE.y, LE.s, sp.I
ROOT = Path(__file__).resolve().parent.parent


def spin_exact(j):
    l = sp.Rational(j, 2); d = j + 1
    m = [l - k for k in range(d)]
    Jz = sp.diag(*m)
    Jp = sp.zeros(d, d)
    for k in range(1, d):
        Jp[k - 1, k] = sp.sqrt(l * (l + 1) - m[k] * (m[k] + 1))
    Jx, Jy = (Jp + Jp.T) / 2, (Jp - Jp.T) / (2 * I)
    rho = [-2 * I * Jz, -2 * I * Jx, -2 * I * Jy]
    for a, b, c in [(0, 1, 2), (1, 2, 0), (2, 0, 1)]:
        assert (rho[a] * rho[b] - rho[b] * rho[a] - 2 * rho[c]).applyfunc(sp.simplify) == sp.zeros(d, d)
    return rho, (Jx, Jy, Jz)


r2 = sp.sqrt(2)
US = {+1: sp.Matrix([0, 1, I]) / r2, -1: sp.Matrix([0, 1, -I]) / r2, 0: sp.Matrix([1, 0, 0])}


def state(cf, cl, v):
    return sp.kronecker_product(sp.kronecker_product(US[cf], US[cl]), v)


def quad(parts, e):
    Ke = LE.apply_K(parts, e)
    return sp.simplify(sp.expand((e.H * Ke)[0, 0])), Ke


def main():
    out = Path(sys.argv[1]); JMAX = int(sys.argv[2]) if len(sys.argv) > 2 else 6
    rec = {"script": "scripts/ricci_su2_direct_rayleigh.py", "JMAX": JMAX, "status": "exact symbolic (sympy)", "rows": []}
    allok = True
    for j in range(JMAX + 1):
        l = sp.Rational(j, 2); d = j + 1
        rho, (Jx, Jy, Jz) = spin_exact(j)
        parts = LE.sym_hessian_parts(LE.family_fields(), rho, LE.SVEC)
        lam = sp.expand(LE.lam(j))
        row = {"j": j}
        tw = None
        for vi in (0, d - 1):
            v = sp.eye(d)[:, vi]
            for sg in (1, -1):
                e = state(sg, sg, v)
                Ke = LE.apply_K(parts, e)
                if (Ke - lam * e).applyfunc(lambda z: sp.simplify(sp.expand(z))) == sp.zeros(e.shape[0], 1):
                    tw = (vi, sg) if tw is None else tw
                    row.setdefault("twist_states_(v_index,s)", []).append([vi, sg])
        assert tw is not None, f"no twist eigenvector found at j={j}"
        vi, sg = tw; v = sp.eye(d)[:, vi]
        row["twist_used"] = {"v_index": vi, "rho0_weight_Jz": str(l - vi), "s": sg}
        # moments of the top vector (degree <= 2 in l)
        mom = {"<Jz>": (v.T * Jz * v)[0], "<Jz^2>": (v.T * Jz * Jz * v)[0], "<Jx^2>": (v.T * Jx * Jx * v)[0],
               "<Jy^2>": (v.T * Jy * Jy * v)[0], "<JxJy>": sp.simplify((v.T * Jx * Jy * v)[0]),
               "<Jx>": (v.T * Jx * v)[0], "<Jy>": sp.simplify((v.T * Jy * v)[0]),
               "<JzJx>": sp.simplify((v.T * Jz * Jx * v)[0]), "<JzJy>": sp.simplify((v.T * Jz * Jy * v)[0])}
        sgn = 1 if vi == 0 else -1
        expect = {"<Jz>": sgn * l, "<Jz^2>": l ** 2, "<Jx^2>": l / 2, "<Jy^2>": l / 2, "<JxJy>": sgn * I * l / 2,
                  "<Jx>": 0, "<Jy>": 0, "<JzJx>": 0, "<JzJy>": 0}
        row["moments_match_degree2_formulas"] = all(sp.simplify(mom[k] - expect[k]) == 0 for k in mom)
        em, e0 = state(sg, -sg, v), state(sg, 0, v)
        qm, _ = quad(parts, em); q0, _ = quad(parts, e0)
        f1 = -8 * l * x / s - 4 * s * x + 4 * y ** 2 - 8 * x / s
        f2 = -4 * l * x / s - 2 * s * x - x ** 2 + 3 * y ** 2 - 4 * x / s
        row["e_minus_RQ_minus_lambda"] = str(sp.collect(sp.expand(qm - lam), x))
        row["e_0_RQ_minus_lambda"] = str(sp.collect(sp.expand(q0 - lam), x))
        row["e_minus_matches_Thm_xpos_item1"] = sp.simplify(qm - lam - f1) == 0
        row["e_0_matches_Thm_xpos_item2"] = sp.simplify(q0 - lam - f2) == 0
        row["e_minus_type_(frame,gauge)"] = [sg, -sg]; row["e_0_type_(frame,gauge)"] = [sg, 0]
        G = parts[2]
        row["|G^* e_-|^2"] = str(sp.simplify(sp.expand(((G.H * em).H * (G.H * em))[0, 0])))
        if j == 0:
            v0 = sp.zeros(9, 1); v0[0] = 1
            qv, _ = quad(parts, v0)
            Gv = (G.H * v0).applyfunc(sp.simplify)
            row["theta0_tau0_RQ"] = str(qv)
            row["theta0_tau0_RQ_equals_4t+2y^2"] = sp.simplify(qv - (4 * s ** 2 + 2 * y ** 2)) == 0
            row["theta0_tau0_Gstar_zero"] = Gv == sp.zeros(Gv.shape[0], 1)
            allok &= row["theta0_tau0_RQ_equals_4t+2y^2"] and row["theta0_tau0_Gstar_zero"]
        allok &= row["e_minus_matches_Thm_xpos_item1"] and row["e_0_matches_Thm_xpos_item2"] and row["moments_match_degree2_formulas"]
        rec["rows"].append(row)
        print(j, {k: v_ for k, v_ in row.items() if k.endswith(("item1", "item2", "formulas", "zero", "4t+2y^2")) or k == "twist_used"}, flush=True)
    rec["distinct_l_values_checked"] = JMAX + 1
    rec["all_exact_checks_pass"] = bool(allok)
    # numpy cross-check with the unmodified builder and spin_rep (rho_0 = -2i Jx)
    import ricci_su3_test2_family as T2
    S, C, eps = T2.su2_data()
    worst = 0.0; num = []
    for (xv, yv, tv, j) in [(0.37, 1.3, 2.7, 6), (-1.2, 0.8, 5.0, 5), (0.1, np.sqrt(10), 10.0, 40), (0.5, 0.5, 4.0, 11)]:
        g = [tv, 1, 1]; M = [np.array([xv, 0, 0.]), np.array([0, yv, 0.]), np.array([0, 0, yv])]
        rr = S.spin_rep(j); H, Gm = T2.hessian_and_gauge(C, g, eps, M, rr); K = H + Gm @ Gm.conj().T
        A = 1j * rr[0] / 2; A = (A + A.conj().T) / 2; w, V = np.linalg.eigh(A)
        us = {+1: np.array([0, 1, 1j]) / np.sqrt(2), -1: np.array([0, 1, -1j]) / np.sqrt(2), 0: np.array([1, 0, 0])}
        st = lambda vv, a, b: np.kron(np.kron(us[a], us[b]), vv)          # noqa: E731
        L = (xv + (j + 2) / np.sqrt(tv)) ** 2 + 2 * np.sqrt(tv) * xv - yv ** 2; sq = np.sqrt(tv); lv = j / 2
        best = None
        for vv in (V[:, 0], V[:, -1]):
            for sg in (1, -1):
                e = st(vv, sg, sg); r_ = K @ e
                if np.linalg.norm(r_ - L * e) < 1e-8:
                    best = (vv, sg)
        vv, sg = best
        a1 = (st(vv, sg, -sg).conj() @ K @ st(vv, sg, -sg)).real - L
        a2 = (st(vv, sg, 0).conj() @ K @ st(vv, sg, 0)).real - L
        f1 = -8 * lv * xv / sq - 4 * sq * xv + 4 * yv ** 2 - 8 * xv / sq
        f2 = -4 * lv * xv / sq - 2 * sq * xv - xv ** 2 + 3 * yv ** 2 - 4 * xv / sq
        worst = max(worst, abs(a1 - f1), abs(a2 - f2))
        num.append({"x": xv, "y": yv, "t": tv, "j": j, "e_minus_RQ_minus_lambda": a1, "formula1": f1,
                    "e_0_RQ_minus_lambda": a2, "formula2": f2})
    rec["numpy_spin_rep_crosscheck"] = {"points": num, "max_abs_dev": worst, "status": "double precision"}
    out.write_text(json.dumps(rec, indent=1, default=str) + "\n")
    print("ALL EXACT CHECKS PASS" if allok else "EXACT CHECK FAILED", "| numpy cross-check max dev", worst)


if __name__ == "__main__":
    main()
    import _receipt_status                     # exit 1 if the receipt records a failed check
    _receipt_status.exit_on_failure(sys.argv[1])
