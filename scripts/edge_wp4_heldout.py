"""Pre-registration A (research/edge-theorem-20261121/wp4_prereg_A_heldout.md, registered in commit 232ab8d, before any
held-out point was computed): the edge at held-out family points.

Gates and measurements exactly as registered:
  A-code-1  Blocks.sector_spectrum = full eigvalsh(H + GG*) at x*_pred, every point, j <= 12 (<= 1e-12 relative)
  A-code-2  P2 recovered (degrees (7,8,16), 113 terms); largest real root in [-3,3] where the lowest eigenvalue of
            B_{4,0} meets lambda_4 (Amendment 1) = edge_A at the 18 rows with height
            1.5 or sqrt_t of exact_edge.json (<= 1e-9)
  A-code-3  P2 on the Berger path vanishes at s = sqrt(t_A), relative to the sum of |terms| (<= 1e-8)
  A-mult    at x*_pred - 0.1, every j <= 12: exactly two full-spectrum eigenvalues within 1e-10 max(1,|lambda_j|)
  A-branch  P1's largest real root in [-3,3] misses x*_meas by > 1e-6 at >= 80% of the P2-class points
  A-bound   x_lo >= r_lo;  A-bound-hi  x*_meas <= r_hi   (roots of lambda_0(x) = <v, Hv>, v = theta^0 (x) tau_0)
  A1        |x*_meas - x*_pred| <= 1e-9 and competitor (4, 0), at every P2-class and grid point
  A2        competitor j in {2, 3} at every small-y point
  A3        no non-twist block eigenvalue below lambda_j for 13 <= j <= 79 at the three x < 0 points
  A4        the floor set (j <= 12, H + GG*) is a single interval on the 0.01 grid over [r_lo - 0.5, x* + 0.5]
usage: OMP_NUM_THREADS=1 python3 scripts/edge_wp4_heldout.py OUT.json
"""
import json
import sys
from pathlib import Path

import numpy as np
import sympy as sp
from scipy.optimize import brentq

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_exact_edge as EE          # noqa: E402
import ricci_su3_test2_family as T2        # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
R = ROOT / "research/su2-floor-family-20261119"
W = ROOT / "research/edge-theorem-20261121"
JMAX = 12
T_A = 2.7311222782381136
x_, y_ = sp.symbols("x y", real=True)
s_ = sp.symbols("s", positive=True)


def recover_factors():
    Pfull = eval((R / "edge_polynomial.srepr").read_text().strip(), {k: getattr(sp, k) for k in dir(sp)})
    Pfull = Pfull.subs({sp.Symbol("x", real=True): x_, sp.Symbol("y", real=True): y_, sp.Symbol("s", positive=True): s_})
    fl = sp.factor_list(sp.expand(Pfull))
    P1 = P2 = None
    for fac, mult in fl[1]:
        pol = sp.Poly(fac, x_, y_, s_)
        degs = (pol.degree(x_), pol.degree(y_), pol.degree(s_))
        if degs == (7, 8, 16):
            P2 = (fac, len(pol.terms()))
        if degs == (5, 6, 12):
            P1 = (fac, len(pol.terms()))
    return P1, P2


_BL = []


def largest_root(fac, yv, tv, lo=-3.0, hi=3.0):
    """Amendment 1: the largest real root r of fac(x, y0, s0) in [lo, hi] at which the LOWEST eigenvalue of B_{4,0}
    equals lambda_4 (to 1e-8 max(1, |lambda_4|)); None if there is none."""
    if not _BL:
        _BL.append(EE.Blocks())
    bl = _BL[0]
    sv = sp.Rational(repr(float(np.sqrt(tv))))
    poly = sp.Poly(sp.expand(fac.subs({y_: sp.Rational(repr(float(yv))), s_: sv})), x_)
    roots = [complex(r) for r in poly.nroots(n=30, maxsteps=200)]
    real = sorted((r.real for r in roots if abs(r.imag) < 1e-12 and lo <= r.real <= hi), reverse=True)
    for r in real:
        _, B = bl.block(bl.data(r, yv, tv), 4, 0.0, tv)
        lam = EE.lam_tw(r, yv, tv, 4)
        if abs(np.linalg.eigvalsh(B)[0] - lam) <= 1e-8 * max(1.0, abs(lam)):
            return r
    return None


_G = {}
MODELS = json.loads((R / "edge_test_predictions.json").read_text())["models"]


def full_spectrum(xv, yv, tv, j):
    if not _G:
        S, C2, eps = T2.su2_data(); _G.update(C2=C2, eps=eps, rho={k: S.spin_rep(k) for k in range(JMAX + 1)})
    M = [xv * np.eye(3)[0], yv * np.eye(3)[1], yv * np.eye(3)[2]]
    H, G = T2.hessian_and_gauge(_G["C2"], [tv, 1.0, 1.0], _G["eps"], M, _G["rho"][j])
    return np.linalg.eigvalsh(0.5 * (H + G @ G.conj().T + (H + G @ G.conj().T).conj().T))


def remove_twist(ev, lam):
    tol = 1e-10 * max(1.0, abs(lam))
    idx = np.where(np.abs(ev - lam) <= tol)[0]
    return np.delete(ev, idx[:2]), len(idx)


def phi_full(xv, yv, tv):
    best = np.inf
    for j in range(JMAX + 1):
        lam = EE.lam_tw(xv, yv, tv, j)
        ev, _ = remove_twist(full_spectrum(xv, yv, tv, j), lam)
        best = min(best, ev[0] - lam)
    return best


def r_bounds(yv, tv):
    """Roots of lambda_0(x) = <v, Hv>, v = theta^0 (x) tau_0 in sector 0; <v, Hv> read from the block rows."""
    bl = EE.Blocks()
    vals = []
    for xv in (-1.0, 0.0, 1.0):
        d = bl.data(xv, yv, tv); st, B = bl.block(d, 0, 0.0, tv)
        k = [i for i, (m, a) in enumerate(st) if bl.ch[a] == (0, 0)][0]
        vals.append(B[k, k].real)
    slope = (vals[2] - vals[0]) / 2; c0 = vals[1]                   # affine (checked: second difference)
    affine_dev = abs(vals[0] - 2 * vals[1] + vals[2])
    # (x + 2/sqrt t)^2 + 2 sqrt t x - y^2 - (c0 + slope x) = 0
    a, b, c = 1.0, 4 / np.sqrt(tv) + 2 * np.sqrt(tv) - slope, 4 / tv - yv ** 2 - c0
    disc = b * b - 4 * a * c
    return (-b - np.sqrt(disc)) / 2, (-b + np.sqrt(disc)) / 2, affine_dev


def last_down_crossing(f, lo, hi, step):
    xs = np.arange(lo, hi + 1e-12, step); vals = [f(xv) for xv in xs]; k = None
    for i in range(len(xs) - 1):
        if vals[i] >= 0 > vals[i + 1]:
            k = i
    if k is None:
        return None, xs, vals
    return brentq(f, xs[k], xs[k + 1], xtol=1e-13), xs, vals


def point_job(args):
    cls, tv, yv, xpred = args
    out = {"class": cls, "t": tv, "y": yv, "x_pred": xpred}
    f = lambda xv: phi_full(xv, yv, tv)            # noqa: E731
    xm, _, _ = last_down_crossing(f, -3.0, 3.0, 0.01)
    out["x_meas"] = xm
    bl = EE.Blocks()
    if xm is not None:
        out["competitor"] = bl.phi_A(xm + 1e-4, yv, tv, True)[1]
        out["x_S"] = EE.edge(EE.phi_S, yv, tv, lo=-3.0, hi=3.0, n=601)
    # Test 6 baseline (descriptive): model of the nearest fitted height (0.5, 1.5 or sqrt t)
    hs = {"0.5": 0.5, "1.5": 1.5, "sqrt_t": np.sqrt(tv)}; h = min(hs, key=lambda k: abs(hs[k] - yv))
    mdl = MODELS[h]; out["test6_model_height"] = h; out["test6_model_x"] = mdl["c1"] / tv + mdl["c2"] / tv ** 2
    # A-code-1 and A-mult (at x_pred when defined, else at x_meas)
    xref = xpred if xpred is not None else xm
    if xref is not None:
        c1 = 0.0; mult = []
        for j in range(JMAX + 1):
            ref = full_spectrum(xref, yv, tv, j); got = bl.sector_spectrum(xref, yv, tv, j)
            c1 = max(c1, float(np.abs(np.sort(ref) - got).max() / max(1, np.abs(ref).max())))
            _, n = remove_twist(full_spectrum(xref - 0.1, yv, tv, j), EE.lam_tw(xref - 0.1, yv, tv, j))
            mult.append(n)
        out["A_code_1"] = c1; out["A_mult_counts"] = mult
    # bounds and A4
    r_lo, r_hi, adev = r_bounds(yv, tv)
    out["r_lo"], out["r_hi"], out["affine_dev"] = r_lo, r_hi, adev
    if xm is not None:
        xs = np.arange(r_lo - 0.5, xm + 0.5 + 1e-12, 0.01); vals = [f(xv) for xv in xs]
        sign = [v >= 0 for v in vals]
        runs = sum(1 for i in range(len(sign)) if sign[i] and (i == 0 or not sign[i - 1]))
        out["A4_runs"] = runs
        first = next((i for i, sg in enumerate(sign) if sg), None)
        if first is not None and first > 0:
            out["x_lo"] = brentq(f, xs[first - 1], xs[first], xtol=1e-12)
        else:
            out["x_lo"] = None
        # A3 at three x < 0 points, sectors 13..79, closed-form blocks
        top = min(xpred if xpred is not None else xm, 0.0) - 0.02
        pts = [top, x_lo_mid(out["x_lo"], xm), (out["x_lo"] + 0.02) if out["x_lo"] is not None else None]
        pts = [p_ for p_ in pts if p_ is not None and p_ < 0]
        worst = np.inf; arg = None
        for xv in pts:
            d = bl.data(xv, yv, tv)
            for j in range(13, 80):
                l = j / 2; lam = EE.lam_tw(xv, yv, tv, j)
                for q in np.arange(-l - 2, l + 2 + 1e-9, 1.0):
                    if abs(abs(q) - (l + 2)) < 1e-9:
                        continue
                    _, B = bl.block(d, j, q, tv)
                    if B.size:
                        mg = np.linalg.eigvalsh(B)[0] - lam
                        if mg < worst:
                            worst, arg = mg, (xv, j, float(q))
        out["x_lo_block"] = bl.phi_A(out["x_lo"] - 1e-4, yv, tv, True)[1] if out["x_lo"] is not None else None
        out["A3_points"] = pts; out["A3_min_margin"] = float(worst); out["A3_argmin"] = arg
    return out


def x_lo_mid(xlo, xm):
    if xlo is None:
        return None
    return 0.5 * (xlo + xm)


def main():
    out_path = Path(sys.argv[1])
    pts = json.loads((W / "wp4_prereg_A_points.json").read_text())["points_(t,y)"]
    res = {"protocol": "wp4_prereg_A_heldout.md @ 232ab8d + Amendment 1 @ 9098bd9"}
    P1, P2 = recover_factors()
    res["A_code_2_degrees_terms"] = {"P2_terms": P2[1] if P2 else None, "P1_terms": P1[1] if P1 else None}
    rows = [r for r in json.loads((R / "exact_edge.json").read_text())["edges"] if r["height"] in ("1.5", "sqrt_t")]
    roots_ = [largest_root(P2[0], r["y"], r["t"]) for r in rows]
    dev = max((abs(a - r["edge_A"]) if a is not None else float("inf")) for a, r in zip(roots_, rows))
    res["A_code_2_max_dev"] = dev; res["A_code_2"] = bool(P2 and P2[1] == 113 and dev <= 1e-9)
    sA = sp.Rational(str(np.sqrt(T_A)))
    Pb = sp.expand(P2[0].subs({x_: (2 - s_ ** 2) / s_, y_: s_}))
    terms = sp.Add.make_args(sp.expand(Pb * s_ ** 0))
    val = float(sp.N(Pb.subs(s_, sp.sqrt(sp.Float(T_A, 40))), 40)); scale = float(sum(abs(sp.N(tm.subs(s_, sp.sqrt(sp.Float(T_A, 40))), 40)) for tm in terms))
    res["A_code_3_rel"] = abs(val) / scale; res["A_code_3"] = abs(val) / scale <= 1e-8
    print("A-code-2:", res["A_code_2"], dev, " A-code-3:", res["A_code_3"], res["A_code_3_rel"], flush=True)
    if not (res["A_code_2"] and res["A_code_3"]):
        out_path.write_text(json.dumps(res, indent=1, default=str)); raise SystemExit("stop: code gate")
    jobs = []
    for cls, key in (("P2", "P2_class_random"), ("grid", "fixed_grid"), ("small_y", "small_y_class_random")):
        for tv, yv in pts[key]:
            xp = largest_root(P2[0], yv, tv) if cls != "small_y" else None
            jobs.append((cls, float(tv), float(yv), xp))
    out = []
    for k, jb in enumerate(jobs):                    # one process, as registered
        out.append(point_job(jb)); o = out[-1]
        print(k, o["class"], round(o["t"], 4), round(o["y"], 4), "pred", o["x_pred"], "meas", o["x_meas"],
              "comp", o.get("competitor"), "x_lo", o.get("x_lo"), "A3", o.get("A3_min_margin"), "runs", o.get("A4_runs"), flush=True)
        Path(str(out_path) + ".partial").write_text(json.dumps(out, default=str))
    res["points"] = out
    # gates
    c1 = max(o.get("A_code_1", 0) for o in out)
    mult_ok = all(all(n == 2 for n in o.get("A_mult_counts", [2])) for o in out)
    bound_ok = all((o.get("x_lo") is None or o["x_lo"] >= o["r_lo"] - 1e-9) and (o["x_meas"] is None or o["x_meas"] <= o["r_hi"] + 1e-9) for o in out)
    res["A_code_1_max"] = c1; res["A_code_1"] = c1 <= 1e-12; res["A_mult"] = mult_ok; res["A_bound_both"] = bound_ok
    p2pts = [o for o in out if o["class"] in ("P2", "grid")]
    P1f = P1[0]
    miss = []
    for o in [o for o in out if o["class"] == "P2"]:
        r1 = largest_root(P1f, o["y"], o["t"])
        miss.append(r1 is None or o["x_meas"] is None or abs(r1 - o["x_meas"]) > 1e-6)
    res["A_branch_fraction"] = sum(miss) / len(miss); res["A_branch"] = res["A_branch_fraction"] >= 0.8
    a1 = [(o["t"], o["y"], o["x_pred"], o["x_meas"], o.get("competitor")) for o in p2pts]
    res["A1_failures"] = [r for r in a1 if r[3] is None or r[2] is None or abs(r[3] - r[2]) > 1e-9 or tuple(r[4] or ()) not in ((4, 0.0),)]
    res["A1"] = not res["A1_failures"]
    res["A2_failures"] = [(o["t"], o["y"], o.get("competitor")) for o in out if o["class"] == "small_y" and (o.get("competitor") is None or o["competitor"][0] not in (2, 3))]
    res["A2"] = not res["A2_failures"]
    res["A3_failures"] = [(o["t"], o["y"], o["A3_min_margin"], o["A3_argmin"]) for o in out if o.get("A3_min_margin", 1) <= 0]
    res["A3"] = not res["A3_failures"]
    res["A4_failures"] = [(o["t"], o["y"], o.get("A4_runs")) for o in out if o.get("A4_runs") != 1]
    res["A4"] = not res["A4_failures"]
    out_path.write_text(json.dumps(res, indent=1, default=str) + "\n")
    print(json.dumps({k: v for k, v in res.items() if k != "points"}, indent=1, default=str))


if __name__ == "__main__":
    main()
