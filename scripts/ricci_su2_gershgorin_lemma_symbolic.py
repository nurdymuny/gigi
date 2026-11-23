"""Exact Gershgorin lemma for the large-j halves (22 November 2026; answers review findings S6-3 and S5-4).

Family: SU(2) on (t, 1, 1), m_0 = x e_0, m_1 = y e_1, m_2 = y e_2, with x, y real and t > 0 symbolic, l > 0 symbolic.
The charge blocks B_{j,q} (j = 2l) of H + GG^* are built from the closed-form rows of scripts/ricci_su2_exact_edge.py
(symbolic_rows, captured exactly by replacing sympy.lambdify with the identity, as in scripts/edge_wp2_symbolic.py):
  diagonal  D(m, a) = (w_a - 2m/sqrt t)^2 + 4(l(l+1) - m^2) + C_aa,
  (m, a) -> (m, a')    entry C[a', a]                               (same m, a' != a),
  (m, a) -> (m+1, a')  entry 2i sqrt((l-m)(l+m+1)) Omn[a', a],
  (m, a) -> (m-1, a')  entry 2i sqrt((l+m)(l-m+1)) Op[a', a],
  states of block q: (m, a) with m + c_a = q, |m| <= l (c_a = c_f + c_l).
The closed-form quantity of scripts/ricci_su2_family_large_j.py (used by its FA/FB and by edge_wp2_interval_proof.py),
at m = l - xi:
  G(l, xi, a) = 4 xi (2l - xi)(1 - 1/t) + l K_a + E_a(xi) - 2 sqrt(xi(2l - xi + 1)) P-_a - 2 sqrt((2l - xi)(xi + 1)) P+_a.

Lemma (PROVED here, exact sympy; every check below is an exact identity, no floating point):
  L1  Charge conservation: C[a',a] != 0 => c_a' = c_a; Omn[a',a] != 0 => c_a' = c_a - 1; Op[a',a] != 0 => c_a' = c_a + 1.
      So every neighbour of a row of block q named by the rows lies in block q, and exists iff |m'| <= l.
  L2  The block is real symmetric: C real symmetric, 2i Omn and 2i Op real, and Omn[a',a] = Op[a,a'] (the ladder factor
      sqrt((l-m)(l+m+1)) is the same in both directions).  So the Gershgorin radius of a row is the sum of the moduli of
      its off-diagonal entries (row = column).
  L3  Exact moduli: every nonzero off-diagonal C entry has |C[a',a]|^2 = co^2 with co = 2|y||sqrt(t) x - t - 2|/sqrt t;
      every nonzero Omn, Op entry has modulus^2 = 2 y^2 or 2 t.  Summed per column these give exactly the table
      R_a, P-_a, P+_a of family_large_j.ingredients (with |y| and sqrt t), for real x, y and t > 0.
  L4  Row radius.  For the row (m, a), m in {-l, ..., l}, the Gershgorin radius is exactly
        rad(m, a) = R_a + 2 sqrt((l-m)(l+m+1)) P-_a + 2 sqrt((l+m)(l-m+1)) P+_a:
      the same-m partners always exist; the m+1 partners exist iff m <= l-1, and for m = l the factor (l-m) is 0;
      likewise m-1 at m = -l.  (So rad is exact, not merely an upper bound.)
  L5  Diagonal identity (symbolic l, m, x, y, t):  D(m, a) - lambda_{2l} - rad(m, a) = G(l, l - m, a)  identically,
      for all nine types, lambda_{2l} = (x + (2l+2)/sqrt t)^2 + 2 sqrt(t) x - y^2.
      Hence G <= (diagonal - lambda - Gershgorin radius) holds, with equality, for every row of every block, and
      lambda_min(B_{j,q}) >= lambda_j + min_rows G (Gershgorin, real symmetric B).  An interval lower bound G > 0 on
      every row of a block therefore implies lambda_min(B_{j,q}) > lambda_j with no numerical step.
  L6  The proof code computes exactly this G: family_large_j.FA(xi, a, U=1/l) * l = G(l, xi, a) and
      family_large_j.FB(a, XN=xi/l, U=1/l, V=1/xi) * l * xi = G(l, xi, a) (the functions themselves, run on sympy
      objects through a shim for mpmath.iv), and edge_wp2_interval_proof.mirror_check's symmetry
      G(l, 2l - xi, a) = G(l, xi, -a) is re-proved.
  L7  The excluded blocks (S5-4).  For |q| > l + 2 the block is empty; for q = l + 2 the only state is (m = l, (1,1)),
      for q = -(l + 2) the only state is (m = -l, (-1,-1)).  Each is 1 x 1 with radius 0 and entry exactly lambda_{2l}
      (D(l,(1,1)) - lambda = 0 and D(-l,(-1,-1)) - lambda = 0, symbolic), and G = 0 on both rows.  So the mirror block
      q = -(l+2) contributes the eigenvalue lambda_j itself (the second copy), never a value below it; no other block
      has a row with G identically zero, and every other row has min(xi, 2l - xi) such that it is covered by the
      FA/FB/near-end parts of the interval proofs (that coverage is a property of those scripts, not re-proved here).
  Cross-check (COMPUTED, double precision, kept as in S6-3's fix): G versus Blocks.block rows at random points.
This lemma is relative to the closed-form rows; that they are the charge blocks of H + GG^* is Proposition exact /
Lemma blocks of the paper (and the numerical builder agreement in ricci_su2_exact_edge.main), not re-proved here.
usage: python scripts/ricci_su2_gershgorin_lemma_symbolic.py OUT.json
"""
import json
import sys
import time
import types
from pathlib import Path

import numpy as np
import sympy as sp

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_exact_edge as EE              # noqa: E402
import ricci_su2_family_large_j as LJ          # noqa: E402

x = EE.x                                       # real
t = EE.t                                       # positive
y = EE.y                                       # real (no sign assumed)
l = sp.symbols("l", positive=True)
m = sp.symbols("m", real=True)
xi_, r_ = sp.symbols("xi r", positive=True)    # for L6: xi > 0, r = 2l - xi > 0 (endpoints by continuity, see note)


def exact_rows():
    orig = EE.sp.lambdify
    EE.sp.lambdify = lambda args, expr, *a, **k: expr
    try:
        ch, (w, C, Op, Omn) = EE.symbolic_rows()
    finally:
        EE.sp.lambdify = orig
    w = [sp.simplify(sp.sympify(v)) for v in w]
    C, Op, Omn = (sp.Matrix(M).applyfunc(sp.simplify) for M in (C, Op, Omn))
    return ch, w, C, Op, Omn


def zero(e):
    return sp.simplify(sp.expand(e)) == 0


def G_table(L, XI, a, ing):
    w, Caa, R, Pp, Pm = ing
    st = sp.sqrt(t)
    K = 4 - 8 / t - 4 * (w[a] + x) / st
    E = 4 * w[a] * XI / st + w[a] ** 2 - x ** 2 - 4 / t - 4 * x / st + Caa[a] - R[a] - 2 * st * x + y ** 2
    return (4 * XI * (2 * L - XI) * (1 - 1 / t) + L * K + E - 2 * sp.sqrt(XI * (2 * L - XI + 1)) * Pm[a]
            - 2 * sp.sqrt((2 * L - XI) * (XI + 1)) * Pp[a])


def main():
    t0 = time.time()
    out = {"date": "2026-11-22", "script": "scripts/ricci_su2_gershgorin_lemma_symbolic.py",
           "assumptions": "x, y real; t > 0; l > 0; rows m in {-l,...,l}; closed-form rows of ricci_su2_exact_edge",
           "status_label": "PROVED (exact sympy identities) except 'numeric_cross_check' (COMPUTED)"}
    say = lambda *a: print(*a, flush=True)    # noqa: E731
    ch, w, C, Op, Omn = exact_rows()
    idx = {a: k for k, a in enumerate(ch)}
    c = {a: a[0] + a[1] for a in ch}
    ing = LJ.ingredients(x, y, t, sp.Abs, sp.sqrt)
    wT, CaaT, RT, PpT, PmT = ing
    st = sp.sqrt(t)
    co = sp.Abs(2 * y * (st * x - t - 2) / st)
    ok_all = True

    # L1 charge conservation
    L1 = True
    for a2 in ch:
        for a in ch:
            if C[idx[a2], idx[a]] != 0 and a2 != a:
                L1 &= c[a2] == c[a]
            if Omn[idx[a2], idx[a]] != 0:
                L1 &= c[a2] == c[a] - 1
            if Op[idx[a2], idx[a]] != 0:
                L1 &= c[a2] == c[a] + 1
    out["L1_charge_conservation"] = L1; ok_all &= L1; say("L1 charge conservation:", L1)

    # L2 real symmetric
    L2 = True
    for a2 in ch:
        for a in ch:
            L2 &= zero(C[idx[a2], idx[a]] - C[idx[a], idx[a2]])
            L2 &= zero(sp.im(C[idx[a2], idx[a]]))
            L2 &= zero(sp.im(2 * sp.I * Omn[idx[a2], idx[a]])) and zero(sp.im(2 * sp.I * Op[idx[a2], idx[a]]))
            L2 &= zero(Omn[idx[a2], idx[a]] - Op[idx[a], idx[a2]])
    out["L2_real_symmetric"] = L2; ok_all &= L2; say("L2 real symmetric:", L2)

    # L3 exact moduli and column sums
    L3 = {}
    lad_mods = [sp.sqrt(2) * sp.Abs(y), sp.sqrt(2) * st]
    for a in ch:
        k = idx[a]
        # same-charge off-diagonal C entries: modulus co (squares equal, both sides >= 0)
        cmods = []
        for k2 in range(9):
            if k2 != k and C[k2, k] != 0:
                e = C[k2, k]
                assert zero(e * sp.conjugate(e) - co ** 2), (a, ch[k2], e)
                cmods.append(co)
        R_exact = sum(cmods, sp.Integer(0))

        def ladder_sum(M):
            s = sp.Integer(0)
            for k2 in range(9):
                e = M[k2, k]
                if e == 0:
                    continue
                sq = sp.simplify(sp.expand(e * sp.conjugate(e)))
                hit = [mm for mm in lad_mods if zero(sq - mm ** 2)]
                assert len(hit) == 1, (a, ch[k2], e, sq)
                s += hit[0]
            return s
        Pm_exact, Pp_exact = ladder_sum(Omn), ladder_sum(Op)
        # direct Abs as a second exact route
        R_abs = sum((sp.Abs(C[k2, k]) for k2 in range(9) if k2 != k), sp.Integer(0))
        Pm_abs = sum((sp.Abs(Omn[k2, k]) for k2 in range(9)), sp.Integer(0))
        Pp_abs = sum((sp.Abs(Op[k2, k]) for k2 in range(9)), sp.Integer(0))
        good = (zero(R_exact - RT[a]) and zero(Pm_exact - PmT[a]) and zero(Pp_exact - PpT[a])
                and zero(R_abs - RT[a]) and zero(Pm_abs - PmT[a]) and zero(Pp_abs - PpT[a])
                and zero(w[k] - wT[a]) and zero(C[k, k] - CaaT[a]))
        L3[str(a)] = {"ok": bool(good), "n_same_charge_partners": len(cmods), "R_a": str(R_exact),
                      "P-_a": str(Pm_exact), "P+_a": str(Pp_exact)}
        ok_all &= good
        say(f"L3 {a}: moduli and column sums equal the table: {good};  R={R_exact}  P-={Pm_exact}  P+={Pp_exact}")
    out["L3_exact_moduli"] = L3

    # L4 + L5: diagonal minus lambda minus exact radius equals G, symbolic l, m
    lam = (x + (2 * l + 2) / st) ** 2 + 2 * st * x - y ** 2
    L5 = {}
    for a in ch:
        k = idx[a]
        D = (w[k] - 2 * m / st) ** 2 + 4 * (l * (l + 1) - m * m) + C[k, k]
        rad = RT[a] + 2 * sp.sqrt((l - m) * (l + m + 1)) * PmT[a] + 2 * sp.sqrt((l + m) * (l - m + 1)) * PpT[a]
        diff = D - lam - rad - G_table(l, l - m, a, ing)
        good = zero(diff)
        L5[str(a)] = bool(good); ok_all &= good
        say(f"L5 {a}: D - lambda - rad - G == 0 (symbolic l, m, x, y, t): {good}")
    out["L4_radius_exact_note"] = ("rad(m,a) = R_a + 2 sqrt((l-m)(l+m+1)) P-_a + 2 sqrt((l+m)(l-m+1)) P+_a is the exact "
                                   "Gershgorin radius: same-m partners always exist (L1); m+1 partners exist iff m<=l-1 "
                                   "and the factor (l-m) vanishes at m=l; m-1 likewise at m=-l")
    out["L5_diagonal_identity_G_equals_D_minus_lambda_minus_radius"] = L5

    # L6: the proof code computes this G (FA, FB run on sympy objects); mirror symmetry
    shim = types.SimpleNamespace(sqrt=sp.sqrt, mpf=sp.sympify)
    saved = LJ.iv
    LJ.iv = shim
    try:
        Lsub = (xi_ + r_) / 2                       # 2l - xi = r
        L6 = {}
        for a in ch:
            G0 = G_table(Lsub, xi_, a, ing)
            fa = LJ.FA(xi_, a, 1 / Lsub, x, y, t) * Lsub
            fb = LJ.FB(a, xi_ / Lsub, 1 / Lsub, 1 / xi_, x, y, t) * Lsub * xi_
            dA = sp.simplify(sp.powsimp(sp.expand(fa - G0), force=True))
            dB = sp.simplify(sp.powsimp(sp.expand(fb - G0), force=True))
            if dA != 0:
                dA = sp.simplify(sp.radsimp(sp.factor(dA)))
            if dB != 0:
                dB = sp.simplify(sp.radsimp(sp.factor(dB)))
            mir = zero(G_table(l, 2 * l - xi_, a, ing) - G_table(l, xi_, (-a[0], -a[1]), ing))
            L6[str(a)] = {"FA_times_l_equals_G": dA == 0, "FB_times_l_xi_equals_G": dB == 0, "mirror": bool(mir)}
            ok_all &= (dA == 0) and (dB == 0) and mir
            say(f"L6 {a}: FA*l == G: {dA == 0}  FB*l*xi == G: {dB == 0}  G(l,2l-xi,a) == G(l,xi,-a): {mir}")
    finally:
        LJ.iv = saved
    out["L6_proof_code_computes_G"] = L6
    out["L6_note"] = ("checked with xi > 0 and r = 2l - xi > 0 symbolic (so every square-root argument is a product of "
                      "positive factors and powsimp(force=True) is valid); both sides are continuous in (xi, r) >= 0, "
                      "so the identities extend to the endpoints xi = 0 (FA) and xi = 2l")

    # L7: excluded blocks
    def states(q):
        return [(q - c[a], a) for a in ch]          # membership needs |q - c_a| <= l
    q_tw, q_mir = l + 2, -(l + 2)
    mem_tw = [a for (mm, a) in states(q_tw) if sp.simplify(sp.Abs(mm) - l) <= 0]
    mem_mir = [a for (mm, a) in states(q_mir) if sp.simplify(sp.Abs(mm) - l) <= 0]
    # |q| > l + 2: q = l + 2 + s, s > 0: m = l + 2 + s - c_a >= l + s > l for every type -> empty (and mirror)
    s = sp.symbols("s", positive=True)
    empty_hi = all(sp.simplify(l + 2 + s - c[a] - l) > 0 for a in ch)
    empty_lo = all(sp.simplify(-(l + 2 + s) - c[a] + l) < 0 for a in ch)
    D_tw = (w[idx[(1, 1)]] - 2 * l / st) ** 2 + 4 * (l * (l + 1) - l * l) + C[idx[(1, 1)], idx[(1, 1)]]
    D_mir = (w[idx[(-1, -1)]] + 2 * l / st) ** 2 + 4 * (l * (l + 1) - l * l) + C[idx[(-1, -1)], idx[(-1, -1)]]
    tw_eq = zero(D_tw - lam); mir_eq = zero(D_mir - lam)
    rad0_tw = (RT[(1, 1)] == 0) and (PpT[(1, 1)] == 0)            # m = l: the P- factor (l-m) is 0
    rad0_mir = (RT[(-1, -1)] == 0) and (PmT[(-1, -1)] == 0)       # m = -l: the P+ factor (l+m) is 0
    G_tw = zero(G_table(l, 0, (1, 1), ing)); G_mir = zero(G_table(l, 2 * l, (-1, -1), ing))
    # no other row has G identically zero in (l, xi): check the coefficient structure is not identically 0
    others_nonzero = all(not zero(G_table(l, xi_, a, ing)) for a in ch)
    L7 = {"block_q=l+2_states": [str(a) for a in mem_tw], "block_q=-(l+2)_states": [str(a) for a in mem_mir],
          "blocks_|q|>l+2_empty": bool(empty_hi and empty_lo),
          "twist_entry_equals_lambda": bool(tw_eq), "mirror_entry_equals_lambda": bool(mir_eq),
          "twist_radius_zero": bool(rad0_tw), "mirror_radius_zero": bool(rad0_mir),
          "G_twist_row_zero": bool(G_tw), "G_mirror_row_zero": bool(G_mir),
          "G_not_identically_zero_as_function_of_xi_for_any_type": bool(others_nonzero)}
    good7 = (mem_tw == [(1, 1)] and mem_mir == [(-1, -1)] and empty_hi and empty_lo and tw_eq and mir_eq
             and rad0_tw and rad0_mir and G_tw and G_mir)
    L7["ok"] = bool(good7); ok_all &= good7
    out["L7_excluded_blocks"] = L7
    say("L7 excluded blocks:", L7)

    # COMPUTED cross-check against Blocks.block (double precision), every row incl. the two excluded ones
    bl = EE.Blocks(); rng = np.random.default_rng(22); worst = 0.0; nrows = 0
    for _ in range(40):
        X, Y, T = rng.uniform(-3, 1), rng.uniform(-3, 3), rng.uniform(0.5, 10); j = int(rng.integers(0, 120)); L = j / 2
        d = bl.data(X, Y, T); lamn = EE.lam_tw(X, Y, T, j)
        for q in np.arange(-L - 2, L + 2 + 1e-9, 1.0):
            st_, B = bl.block(d, j, q, T)
            for kk, (mm, ai) in enumerate(st_):
                a = bl.ch[ai]; xi = L - mm
                gersh = B[kk, kk].real - lamn - (np.abs(B[kk]).sum() - abs(B[kk, kk]))
                worst = max(worst, abs(LJ.G_exact(L, xi, a, X, Y, T) - gersh) / max(1.0, abs(gersh)))
                nrows += 1
    out["numeric_cross_check"] = {"label": "COMPUTED", "rows": nrows, "points": 40, "j_range": [0, 119],
                                  "max_rel_abs_diff_G_vs_actual_gershgorin": worst}
    say("COMPUTED cross-check |G - actual Gershgorin quantity| (relative), max over", nrows, "rows:", worst)

    out["all_proved"] = bool(ok_all); out["seconds"] = round(time.time() - t0, 1)
    Path(sys.argv[1]).parent.mkdir(parents=True, exist_ok=True)
    Path(sys.argv[1]).write_text(json.dumps(out, indent=1, default=str) + "\n")
    say("ALL PROVED" if ok_all else "NOT PROVED")


if __name__ == "__main__":
    import _receipt_status                     # exit 1 if the receipt is missing, stale or records a failed check
    _receipt_status.begin()
    main()
    _receipt_status.exit_on_failure(sys.argv[1])
