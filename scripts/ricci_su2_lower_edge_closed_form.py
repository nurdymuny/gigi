"""The sector-0 lower edge of the twist's region for H + GG^*, in closed form (22 November 2026; review findings F4, S5-1).

Family: SU(2) on the Berger sphere (t, 1, 1), m_0 = x e_0, m_1 = y e_1, m_2 = y e_2.

What this script does, with no use of the closed-form charge-block rows:
  1. Builds the sector j = 0 of H + GG^* symbolically (sympy, x, y real, s = sqrt t > 0) by a line-for-line sympy port
     of the general builder scripts/ricci_su3_test2_family.py::hessian_and_gauge (Hessian from Q: sum over frame pairs
     of D^* D plus the curvature term <F_ij, [a_i, a_j]>; G xi = d_A xi). In the sector j = 0 every rho_i = 0, so the
     sector is a 9 x 9 symbolic matrix.
  2. Rotates it to the charge basis (U3 (x) U3, the same U3 as scripts/ricci_su2_exact_edge.py) and checks that it is
     block diagonal in the charge q = c_f + c_l, with blocks q = +-2 (1 x 1), q = +-1 (2 x 2), q = 0 (3 x 3).
  3. Checks exactly that the q = +2 and q = -2 entries are the twist eigenvalue lambda_0 = (x + 2/s)^2 + 2 s x - y^2.
  4. Takes the q = -1 block B (2 x 2) and forms P(x, y, s) = s^2 * det(B - lambda_0 I), a polynomial (exact). The
     sector-0 lower edge from this block, x_lo^(0)(y, t), is the root of P at which lambda_min(B) - lambda_0 changes sign
     from negative (x below) to non-negative (x above); P is factored and its degree in x reported.
  5. Cross-checks: the q = +1 block has the same characteristic polynomial as q = -1 (mirror); all blocks equal the
     closed-form rows of scripts/ricci_su2_exact_edge.py at j = 0 (exact symbolic comparison); the q = 0 block's
     crossing lies below the q = -1 crossing at every point checked.
  6. Evaluates x_lo^(0) at the 48 points of pre-registration A (research/edge-theorem-20261121/wp4_heldout.json) with
     mpmath (50 digits) and compares with the measured all-j<=12 x_lo there (brentq on phi_A, xtol 1e-12, double).
usage: python scripts/ricci_su2_lower_edge_closed_form.py OUT.json
"""
import json
import sys
from pathlib import Path

import mpmath
import sympy as sp

ROOT = Path(__file__).resolve().parent.parent
x, y = sp.symbols("x y", real=True)
s = sp.symbols("s", positive=True)          # s = sqrt(t)
I = sp.I


# ---------------------------------------------------------------- sympy port of the general builder
def su2_structure():
    """Frame brackets C (BR = 2 cyclic, scripts/ricci_su2_spin_sector.py) and gauge structure constants eps."""
    C = [[[0] * 3 for _ in range(3)] for _ in range(3)]
    eps = [[[0] * 3 for _ in range(3)] for _ in range(3)]
    for (i, j, k) in [(0, 1, 2), (1, 2, 0), (2, 0, 1)]:
        C[i][j][k] = 2; C[j][i][k] = -2
        eps[i][j][k] = 1; eps[j][i][k] = -1
    return C, eps


def adm(v, f):
    """[v, .] on the gauge algebra: adm(v)[b, c] = sum_a v_a f[a][c][b] (as in ricci_su3_test2_family.adm)."""
    r = len(f)
    return sp.Matrix(r, r, lambda b, c: sum(v[a] * f[a][c][b] for a in range(r)))


def sym_hessian_parts(M, rho, svec):
    """Symbolic port of ricci_su3_test2_family.hessian_and_gauge for SU(2) on the frame with lengths svec.
    Returns (Ds, Fblocks, G): H = sum_p D_p^* D_p + sum (curvature blocks), G = vstack(deriv_i + ad_i)."""
    C, f = su2_structure()
    n, r, d = 3, 3, rho[0].shape[0]
    c = [[[sp.nsimplify(C[i][j][k]) * svec[k] / (svec[i] * svec[j]) for k in range(n)] for j in range(n)] for i in range(n)]
    Ir, Id = sp.eye(r), sp.eye(d)
    N = n * r * d
    deriv = [sp.kronecker_product(Ir, rho[i] / svec[i]) for i in range(n)]
    ad = [sp.kronecker_product(adm(M[i], f), Id) for i in range(n)]
    pairs = [(i, j) for i in range(n) for j in range(i + 1, n)]
    Ds = []
    for (i, j) in pairs:
        D = sp.zeros(r * d, N)
        D[:, j * r * d:(j + 1) * r * d] += deriv[i] + ad[i]
        D[:, i * r * d:(i + 1) * r * d] -= deriv[j] + ad[j]
        for k in range(n):
            if c[i][j][k] != 0:
                D[:, k * r * d:(k + 1) * r * d] -= c[i][j][k] * sp.eye(r * d)
        Ds.append(D)
    Fb = []
    for (i, j) in pairs:
        Fij = adm(M[i], f) * M[j] - sum((c[i][j][k] * M[k] for k in range(n)), sp.zeros(3, 1))
        Fij = Fij.applyfunc(sp.expand)
        if Fij == sp.zeros(3, 1):
            continue
        P = sp.Matrix(r, r, lambda b, cc: sum(Fij[a] * f[b][cc][a] for a in range(r)))      # x^T P y = <F, [x, y]>
        Fb.append((i, j, sp.kronecker_product(P, Id)))
    G = sp.Matrix.vstack(*[deriv[i] + ad[i] for i in range(n)])
    return Ds, Fb, G, (n, r, d)


def apply_K(parts, e):
    """(H + G G^*) e, from the parts (no full matrix)."""
    Ds, Fb, G, (n, r, d) = parts
    out = sp.zeros(e.shape[0], 1)
    for D in Ds:
        out += D.H * (D * e)
    out = sp.MutableDenseMatrix(out)
    blk = lambda i: slice(i * r * d, (i + 1) * r * d)          # noqa: E731
    for (i, j, B) in Fb:
        out[blk(i), 0] += B * e[blk(j), 0]
        out[blk(j), 0] += B.T * e[blk(i), 0]
    out += G * (G.H * e)
    return out.applyfunc(sp.expand)


def full_K(parts):
    Ds, Fb, G, (n, r, d) = parts
    N = n * r * d
    cols = [apply_K(parts, sp.eye(N)[:, k]) for k in range(N)]
    return sp.Matrix.hstack(*cols)


def family_fields():
    return [sp.Matrix([x, 0, 0]), sp.Matrix([0, y, 0]), sp.Matrix([0, 0, y])]


SVEC = [s, sp.Integer(1), sp.Integer(1)]
r2 = sp.sqrt(2)
U3 = sp.Matrix([[0, 1, 0], [1 / r2, 0, 1 / r2], [-I / r2, 0, I / r2]])     # columns: charge -1, 0, +1
CHG = [-1, 0, 1]


def lam(j):
    return (x + (j + 2) / s) ** 2 + 2 * s * x - y ** 2


def main():
    out = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "research/edge-theorem-20261122-proofs/lower_edge_closed_form.json"
    rec = {"script": "scripts/ricci_su2_lower_edge_closed_form.py", "status_note":
           "Steps 1-5 exact symbolic (sympy). Step 6 is the evaluation of an exact polynomial root at 50 digits "
           "(mpmath polyroots) compared with a double-precision measurement."}
    rho0 = [sp.zeros(1, 1)] * 3
    parts = sym_hessian_parts(family_fields(), rho0, SVEC)
    K = full_K(parts).applyfunc(sp.simplify)
    assert (K - K.H).applyfunc(sp.simplify) == sp.zeros(9, 9)
    U = sp.kronecker_product(U3, U3)
    Kc = (U.H * K * U).applyfunc(sp.simplify)
    q = [CHG[a] + CHG[b] for a in range(3) for b in range(3)]         # index a*3+b: frame charge a, gauge charge b
    offblock = [(i, k) for i in range(9) for k in range(9) if q[i] != q[k] and sp.simplify(Kc[i, k]) != 0]
    rec["sector0_block_diagonal_in_charge"] = not offblock
    assert not offblock
    blocks = {}
    for qq in (-2, -1, 0, 1, 2):
        ids = [i for i in range(9) if q[i] == qq]
        blocks[qq] = (ids, Kc.extract(ids, ids).applyfunc(sp.simplify))
    L0 = sp.expand(lam(0))
    rec["twist_q+2_equals_lambda0"] = sp.simplify(blocks[2][1][0, 0] - L0) == 0
    rec["twist_q-2_equals_lambda0"] = sp.simplify(blocks[-2][1][0, 0] - L0) == 0
    assert rec["twist_q+2_equals_lambda0"] and rec["twist_q-2_equals_lambda0"]
    ids, B = blocks[-1]
    rec["q-1_states_(frame_charge,gauge_charge)"] = [(CHG[i // 3], CHG[i % 3]) for i in ids]
    rec["q-1_block_entries"] = [[str(sp.factor(B[i, k])) for k in range(2)] for i in range(2)]
    A = (B - L0 * sp.eye(2)).applyfunc(sp.expand)
    rec["q-1_block_minus_lambda0"] = [[str(sp.collect(sp.expand(A[i, k]), x)) for k in range(2)] for i in range(2)]
    detA = sp.expand(A.det())
    P = sp.expand(detA * s ** 3)
    assert sp.Poly(P, x, y, s) is not None and all(e >= 0 for mon in sp.Poly(P, x, y, s).monoms() for e in mon)
    rec["P_definition"] = ("P = s^3 * det(B_{0,-1} - lambda_0 I) (s = sqrt t), a polynomial in (x, y, s). Its leading "
                           "x-coefficient 2 s^2 (s^2 + 2) > 0. Where P < 0 the 2x2 matrix B - lambda_0 I is indefinite, so "
                           "lambda_min(B) < lambda_0; hence x_lo^(0) >= the smallest real root of P, with equality where "
                           "the trace of B - lambda_0 I is positive at that root (checked at each A point below).")
    rec["P"] = str(sp.collect(P, x))
    rec["P_factored"] = str(sp.factor(P))
    rec["P_degree_in_x"] = int(sp.Poly(P, x).degree())
    rec["trace_B_minus_lambda0"] = str(sp.collect(sp.expand(A.trace()), x))
    # mirror block
    _, Bp = blocks[1]
    lamv = sp.symbols("lamv")
    cp_m = sp.expand((B - lamv * sp.eye(2)).det()); cp_p = sp.expand((Bp - lamv * sp.eye(2)).det())
    rec["q+1_same_charpoly_as_q-1"] = sp.simplify(cp_m - cp_p) == 0
    # cross-check against the closed-form rows of ricci_su2_exact_edge (j = 0: diag w_a^2 + C_aa, off-diag C)
    sys.path.insert(0, str(ROOT / "scripts"))
    import ricci_su2_exact_edge as EE
    from ricci_su2_exact_edge import GC, cross
    tt = EE.t
    s_, c_, _, _, gmat = GC.background(sp.sqrt, sp.Integer(1), tt)
    m = [sp.Matrix([EE.x, 0, 0]), sp.Matrix([0, EE.y, 0]), sp.Matrix([0, 0, EE.y])]
    I3 = sp.eye(3)
    Om = [sp.kronecker_product(I3, cross(m[i])) + sp.kronecker_product(-sp.Matrix(gmat[i]), I3) for i in range(3)]
    Ric = sp.diag(2 * tt, 4 - 2 * tt, 4 - 2 * tt); T = sp.zeros(9, 9)
    for i in range(3):
        for k in range(3):
            F = cross(m[i]) * m[k]
            for l in range(3):
                F = F - c_[(i, k, l)] * m[l]
            T[3 * i:3 * i + 3, 3 * k:3 * k + 3] = cross(F)
    R0 = sp.kronecker_product(Ric, I3) - 2 * T
    tr = lambda M_: (U.H * M_ * U).applyfunc(sp.simplify)          # noqa: E731
    Omc = [tr(O_) for O_ in Om]
    w = [sp.simplify(Omc[0][a, a] / I) for a in range(9)]
    Crow = (-(Omc[1] * Omc[1] + Omc[2] * Omc[2]) + tr(R0)).applyfunc(sp.simplify)
    rows0 = sp.Matrix(9, 9, lambda i, k: (w[i] ** 2 if i == k else 0) + Crow[i, k])
    rows0 = rows0.subs({EE.x: x, EE.y: y, tt: s ** 2}).applyfunc(sp.simplify)
    diffm = (rows0 - Kc).applyfunc(lambda e: sp.simplify(sp.expand(e)))
    rec["closed_form_rows_j0_equal_operator_sector0"] = diffm == sp.zeros(9, 9)
    # q = 0 block crossing polynomial (for comparison)
    _, B0 = blocks[0]
    A0 = (B0 - L0 * sp.eye(3)).applyfunc(sp.expand)
    # theta^0 (x) tau_0 = frame 0, gauge 0 = original index 0
    rec["theta0_tau0_RQ_sector0"] = str(sp.simplify(K[0, 0]))
    rec["theta0_tau0_RQ_equals_4t+2y^2"] = sp.simplify(K[0, 0] - (4 * s ** 2 + 2 * y ** 2)) == 0
    rec["Gstar_theta0_tau0_zero"] = (parts[2].H * sp.eye(9)[:, 0]).applyfunc(sp.simplify) == sp.zeros(parts[2].shape[1], 1)

    # ---------------- step 6: the A points
    A_rec = json.loads((ROOT / "research/edge-theorem-20261121/wp4_heldout.json").read_text())
    mpmath.mp.dps = 50
    fA = sp.lambdify((x, y, s), A, "mpmath"); fA0 = sp.lambdify((x, y, s), A0, "mpmath")
    Pc = sp.Poly(P, x)
    coef_f = [sp.lambdify((y, s), cf, "mpmath") for cf in Pc.all_coeffs()]

    def lmin(fun, xv, yv, sv):
        Mx = mpmath.matrix(fun(xv, yv, sv))
        return min(mpmath.re(e) for e in mpmath.eigsy(Mx)[0])

    rows, worst = [], mpmath.mpf(0)
    for p in A_rec["points"]:
        yv, tv = mpmath.mpf(p["y"]), mpmath.mpf(p["t"]); sv = mpmath.sqrt(tv)
        roots = mpmath.polyroots([cf(yv, sv) for cf in coef_f], maxsteps=500, extraprec=200)
        real = sorted(mpmath.re(r_) for r_ in roots if abs(mpmath.im(r_)) < mpmath.mpf(10) ** -30)
        # crossing from below: margin < 0 just below, >= 0 just above
        tr0 = mpmath.re(sum(mpmath.matrix(fA(real[0], yv, sv))[i, i] for i in range(2)))
        cand = [r_ for r_ in real if lmin(fA, r_ - mpmath.mpf(10) ** -8, yv, sv) < 0 < lmin(fA, r_ + mpmath.mpf(10) ** -8, yv, sv)]
        xl0 = min(cand) if cand else None
        # q = 0 block margin at the q = -1 crossing (positive => q = 0 crosses lower, q = -1 sets the sector-0 edge)
        m0 = lmin(fA0, xl0, yv, sv) if xl0 is not None else None
        dev = abs(xl0 - mpmath.mpf(p["x_lo"])) if (xl0 is not None and p["x_lo"] is not None) else None
        if dev is not None:
            worst = max(worst, dev)
        rows.append({"class": p["class"], "y": p["y"], "t": p["t"], "real_roots_of_P": [float(r_) for r_ in real],
                     "x_lo0_closed_form": mpmath.nstr(xl0, 20) if xl0 is not None else None,
                     "x_lo0_is_smallest_real_root": bool(xl0 == real[0]), "trace_at_smallest_root": float(tr0),
                     "x_lo_measured_all_j_le_12": p["x_lo"], "abs_dev": float(dev) if dev is not None else None,
                     "measured_block": p.get("x_lo_block"), "q0_block_margin_at_x_lo0": float(m0) if m0 is not None else None,
                     "r_lo": p["r_lo"]})
    rec["A_points"] = rows
    rec["A_max_abs_dev"] = float(worst)
    rec["A_all_x_lo0_smallest_root"] = all(r_["x_lo0_is_smallest_real_root"] for r_ in rows)
    rec["A_min_trace_at_smallest_root"] = min(r_["trace_at_smallest_root"] for r_ in rows)
    rec["A_all_q0_margin_positive_at_x_lo0"] = all(r_["q0_block_margin_at_x_lo0"] > 0 for r_ in rows)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(rec, indent=1, default=str) + "\n")
    for k, v in rec.items():
        if k != "A_points":
            print(k, ":", v)
    print("max |x_lo^(0) - x_lo measured| over", len(rows), "A points:", float(worst))


if __name__ == "__main__":
    main()
