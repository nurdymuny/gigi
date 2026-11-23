"""The twist's eigenvalue taken apart across the circle-symmetric family, and the competitor that sets her edge
(21 November 2026; receipt for docs/research/what-she-is-a-law-of-2026-11-21.md).

Family: SU(2) on (t, 1, 1), m_0 = x e_0, m_1 = y e_1, m_2 = y e_2 (as scripts/ricci_su2_twist_family_closed_form.py).
Weitzenbock form (Floor paper, Proposition 'lowest Landau level', and Proposition 'the twist across the family'):
  H + GG* = sum_i X_i^* X_i + R0,  X_i = rho_i/sqrt(g_i) + Omega_i,  R0 = Ric (x) I - 2[F, .].

Part 1 (exact, sympy, identities in x, y, t, j):
  P1  Omega_0 beta = i w beta, w = (t - 2)/sqrt t - x; split into the frame (Levi-Civita) part (t - 2)/sqrt t and the
      gauge part -x. So X_0 b = i kappa b with kappa = (t - 2)/sqrt t - x - j/sqrt t (rho_0 f = -i j f).
  P2  (Omega_1 + i Omega_2) beta = 0 for every x, y, t: with the top weight this is X_+ b = 0 (lowest Landau level)
      across the whole family, not only on the Berger path.
  P3  R0 beta = (Ric_perp - 2B) beta exactly, Ric_perp = 4 - 2t, B = y^2 - 2 sqrt(t) x, and F_12 = B e_0.
  P4  lambda = kappa^2 + (2j + t + y^2) + (4 - 2t - 2B), equal to the published closed form identically.
  P5  On the Berger field: kappa = (2t - 4 - j)/sqrt t, transverse 2(j + t), curvature 12 - 8t (the paper's values).
Part 2 (double precision, sectors j <= 12; exploratory, not a certificate):
  Q1  composition of the lowest state of the competitor block B_{4,0} at three points;
  Q2  slopes in x (Hellmann-Feynman, central difference) of that state and of the twist;
  Q3  phi(x) = lambda_min(B_{4,0}) - lambda(j = 4) strictly decreasing in x on [-3, 3], on a (t, y) grid;
  Q4  the crude operator-norm route to Q3 (||dB/dx|| below the twist's slope) along the computed edges;
  Q5  kappa_4 and B along the computed edges (is the edge a function of one of them alone?).
usage: python scripts/ricci_su2_twist_anatomy.py OUT.json   (reads research/su2-floor-family-20261119/exact_edge.json)
"""
import json
import sys
from pathlib import Path

import numpy as np
import sympy as sp

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
_argv, sys.argv = sys.argv, [sys.argv[0]]
import ricci_su2_gap_certificates_t as GC      # noqa: E402
import ricci_su2_exact_edge as E               # noqa: E402
sys.argv = _argv

x, y = sp.symbols("x y", real=True)
t = sp.symbols("t", positive=True)
j = sp.symbols("j", nonnegative=True)


def cross(v):
    return sp.Matrix([[0, -v[2], v[1]], [v[2], 0, -v[0]], [-v[1], v[0], 0]])


def part1():
    s, c, _, _, gmat = GC.background(sp.sqrt, sp.Integer(1), t)
    m = [sp.Matrix([x, 0, 0]), sp.Matrix([0, y, 0]), sp.Matrix([0, 0, y])]
    I3 = sp.eye(3)
    Omg = [sp.kronecker_product(I3, cross(m[i])) for i in range(3)]
    Omf = [sp.kronecker_product(-sp.Matrix(gmat[i]), I3) for i in range(3)]
    Om = [Omg[i] + Omf[i] for i in range(3)]
    Ric = sp.diag(2 * t, 4 - 2 * t, 4 - 2 * t)
    T = sp.zeros(9, 9); F = {}
    for i in range(3):
        for k in range(3):
            Fik = cross(m[i]) * m[k]
            for l in range(3):
                Fik = Fik - c[(i, k, l)] * m[l]
            F[(i, k)] = Fik.applyfunc(sp.simplify)
            T[3 * i:3 * i + 3, 3 * k:3 * k + 3] = cross(Fik)
    R0 = sp.kronecker_product(Ric, I3) - 2 * T
    r2 = sp.sqrt(2)
    U3 = sp.Matrix([[0, 1, 0], [1 / r2, 0, 1 / r2], [-sp.I / r2, 0, sp.I / r2]])
    beta = sp.kronecker_product(U3[:, 2], U3[:, 2]); bH = beta.H
    ev = lambda M: sp.simplify((bH * M * beta)[0])           # noqa: E731
    is_eig = lambda M, val: (M * beta - val * beta).applyfunc(sp.simplify) == sp.zeros(9, 1)   # noqa: E731

    w_gauge, w_frame = sp.simplify(ev(Omg[0]) / sp.I), sp.simplify(ev(Omf[0]) / sp.I)
    w = sp.simplify(w_gauge + w_frame)
    kappa = w - j / sp.sqrt(t)
    Bf = sp.simplify(y ** 2 - 2 * sp.sqrt(t) * x)
    Ricp = 4 - 2 * t
    trans = sp.simplify(2 * j + ev(-(Om[1] * Om[1] + Om[2] * Om[2])))
    lam_parts = sp.expand(kappa ** 2 + trans + Ricp - 2 * Bf)
    lam_pub = sp.expand((x + (j + 2) / sp.sqrt(t)) ** 2 + 2 * sp.sqrt(t) * x - y ** 2)
    berger = {x: (2 - t) / sp.sqrt(t), y: sp.sqrt(t)}
    out = {
        "P1_Omega0_beta_eigen": is_eig(Om[0], sp.I * w),
        "P1_w_gauge_part": str(w_gauge), "P1_w_frame_part": str(w_frame),
        "P1_kappa": str(sp.simplify(kappa)),
        "P2_Omega_plus_beta_is_zero": (Om[1] + sp.I * Om[2]) * beta == sp.zeros(9, 1)
        or ((Om[1] + sp.I * Om[2]) * beta).applyfunc(sp.simplify) == sp.zeros(9, 1),
        "P3_F_12": [str(v) for v in F[(1, 2)]], "P3_F_01": [str(v) for v in F[(0, 1)]],
        "P3_F_02": [str(v) for v in F[(0, 2)]],
        "P3_F_12_equals_B_e0": [sp.simplify(F[(1, 2)][0] - Bf), F[(1, 2)][1], F[(1, 2)][2]] == [0, 0, 0],
        "P3_R0_beta_eigen_Ricperp_minus_2B": is_eig(R0, Ricp - 2 * Bf),
        "P3_Ric_part": str(ev(sp.kronecker_product(Ric, I3))), "P3_minus2F_part": str(ev(-2 * T)),
        "P4_transverse_term": str(trans),
        "P4_parts_minus_published": str(sp.simplify(lam_parts - lam_pub)),
        "P5_berger_kappa": str(sp.simplify(kappa.subs(berger))),
        "P5_berger_transverse": str(sp.simplify(trans.subs(berger))),
        "P5_berger_curvature": str(sp.simplify((Ricp - 2 * Bf).subs(berger))),
        "P5_berger_B": str(sp.simplify(Bf.subs(berger))),
        "P5_berger_gauge_equals_frame": sp.simplify((w_gauge - w_frame).subs(berger)) == 0,
        "Asth_B": str(sp.factor(Bf.subs({x: 2 * sp.symbols("s") / sp.sqrt(t), y: 2 * sp.symbols("s")}))),
    }
    ok = (out["P1_Omega0_beta_eigen"] and out["P2_Omega_plus_beta_is_zero"] and out["P3_F_12_equals_B_e0"]
          and out["P3_R0_beta_eigen_Ricperp_minus_2B"] and out["P4_parts_minus_published"] == "0"
          and out["P5_berger_gauge_equals_frame"])
    out["part1_all_identities_hold"] = bool(ok)
    return out


BK = E.Blocks()


def comp40(xv, yv, tv):
    st, B = BK.block(BK.data(xv, yv, tv), 4, 0.0, tv)
    evs, V = np.linalg.eigh(B)
    return st, evs, V, B


def part2(edge_file):
    out = {"Q1_Q2": []}
    pts = [("Berger at t*", (2 - 2.6980269405) / np.sqrt(2.6980269405), np.sqrt(2.6980269405), 2.6980269405),
           ("t = 4, y = 2, x = -0.19", -0.19, 2.0, 4.0), ("t = 8, y = sqrt 8, x = 0.14", 0.14, np.sqrt(8), 8.0)]
    h = 1e-6
    for name, xv, yv, tv in pts:
        st, evs, V, _ = comp40(xv, yv, tv)
        comps = []
        for (mm, a), amp in zip(st, V[:, 0]):
            cf, cl = BK.ch[a]
            comps.append({"m": mm, "c_f": cf, "c_l": cl, "weight": float(abs(amp) ** 2)})
        dc = (comp40(xv + h, yv, tv)[1][0] - comp40(xv - h, yv, tv)[1][0]) / (2 * h)
        dh = (E.lam_tw(xv + h, yv, tv, 4) - E.lam_tw(xv - h, yv, tv, 4)) / (2 * h)
        out["Q1_Q2"].append({"point": name, "x": xv, "y": yv, "t": tv, "competitor_lowest": float(evs[0]),
                             "twist_j4": float(E.lam_tw(xv, yv, tv, 4)), "composition": comps,
                             "slope_competitor": float(dc), "slope_twist": float(dh)})
    rows, nmono = [], 0
    for tv in [2, 2.5, 3, 4, 5, 6, 8, 10]:
        for yv in [0.5, 1.0, 1.5, 2.0, float(np.sqrt(tv)), 3.0]:
            xs = np.linspace(-3, 3, 241)
            phi = np.array([comp40(xv, yv, tv)[1][0] - E.lam_tw(xv, yv, tv, 4) for xv in xs])
            mono = bool((np.diff(phi) < 0).all()); nmono += mono
            rows.append({"t": tv, "y": yv, "strictly_decreasing": mono, "max_diff": float(np.diff(phi).max())})
    out["Q3_monotone_pairs"] = f"{nmono}/{len(rows)}"; out["Q3_rows"] = rows
    edges = json.loads(Path(edge_file).read_text())["edges"]
    q4 = []
    for e in edges:
        tv, yv, xa = e["t"], e["y"], e["edge_A"]
        dB = (comp40(xa + h, yv, tv)[3] - comp40(xa - h, yv, tv)[3]) / (2 * h)
        q4.append({"t": tv, "y": yv, "edge_A": xa, "competitor_block_A": e["competitor_block_A"],
                   "norm_dB_dx": float(np.linalg.norm(dB, 2)),
                   "twist_slope": float(2 * (xa + 6 / np.sqrt(tv)) + 2 * np.sqrt(tv)),
                   "B_at_edge": float(yv ** 2 - 2 * np.sqrt(tv) * xa),
                   "kappa4_at_edge": float((tv - 2) / np.sqrt(tv) - xa - 4 / np.sqrt(tv))})
    for r in q4:
        r["crude_route_closes"] = r["norm_dB_dx"] < r["twist_slope"]
    out["Q4_Q5_edges"] = q4
    out["Q4_crude_route_closes_at"] = f"{sum(r['crude_route_closes'] for r in q4)}/{len(q4)} edge points"
    return out


def main():
    root = HERE.parent
    res = {"part1_exact": part1(),
           "part2_exploratory": part2(root / "research/su2-floor-family-20261119/exact_edge.json")}
    Path(sys.argv[1]).write_text(json.dumps(res, indent=1, default=str) + "\n")
    p1 = res["part1_exact"]
    print(json.dumps({k: v for k, v in p1.items()}, indent=1, default=str))
    p2 = res["part2_exploratory"]
    print("Q3 monotone:", p2["Q3_monotone_pairs"], "| Q4 crude route:", p2["Q4_crude_route_closes_at"])
    for q in p2["Q1_Q2"]:
        print(q["point"], "slopes", round(q["slope_competitor"], 4), round(q["slope_twist"], 4))


if __name__ == "__main__":
    main()
