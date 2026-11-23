"""Pre-registration S: stress tests (docs/research/stress-tests-preregistration-2026-11-22.md, committed 951f2da0
before this script was written). Writes stress_receipt.json beside this file and prints a summary.
usage (repository root): python docs/research/stress-2026-11-22/stress_tests.py
"""
import json
import sys
from pathlib import Path

import numpy as np
import sympy as sp
from scipy.integrate import solve_ivp
from scipy.optimize import fsolve

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
_a, sys.argv = sys.argv, [sys.argv[0]]
import ricci_su2_exact_edge as EE  # noqa: E402
sys.argv = _a

J = 40
x_, y_, t_ = sp.symbols("x y t", real=True)
st_ = sp.sqrt(t_)
B_ = y_ ** 2 - 2 * st_ * x_
c_ = y_ * (x_ - 2 / st_)
E_ = sp.Rational(1, 2) * (B_ ** 2 + 2 * c_ ** 2)
Ef = sp.lambdify((x_, y_, t_), E_)
gx = sp.lambdify((x_, y_, t_), sp.diff(E_, x_))
gy = sp.lambdify((x_, y_, t_), sp.diff(E_, y_))
H_ = sp.hessian(E_, (x_, y_))
Hf = sp.lambdify((x_, y_, t_), H_)
BLK = EE.Blocks()


def lam(x, y, t, j):
    return (x + (j + 2) / np.sqrt(t)) ** 2 + 2 * np.sqrt(t) * x - y ** 2


def lam_min(x, y, t):
    return min(lam(x, y, t, j) for j in range(J + 1))


def floor_report(x, y, t):
    """Sectors j <= J where some state lies below her, with the gap."""
    below = []
    for j in range(J + 1):
        ev = BLK.sector_spectrum(x, y, t, j)
        l = lam(x, y, t, j)
        if ev.min() < l - 1e-9 * max(1.0, abs(l)):
            below.append((j, float(ev.min() - l)))
    return below


def flow(x0, y0, t, T=200.0, dense=False):
    f = lambda s, v: [-gx(v[0], v[1], t), -0.5 * gy(v[0], v[1], t)]  # noqa: E731
    return solve_ivp(f, [0, T], [x0, y0], method="LSODA", rtol=1e-10, atol=1e-12, dense_output=dense)


def landing(sol, t):
    xe, ye = sol.y[:, -1]
    if np.hypot(xe, ye) < 1e-3:
        return "A=0"
    if np.hypot(xe - 2 / np.sqrt(t), ye - 2) < 1e-3:
        return "A=theta"
    return "other"


def crossing(sol, t, T=5.0, n=20001):
    ss = np.linspace(0, T, n)
    m = np.array([lam_min(*sol.sol(s), t) for s in ss])
    if m[0] >= 0:
        return None, m
    k = int(np.argmax(m >= 0))
    return (float(ss[k]) if m[k] >= 0 else "never"), m


D_POINTS = [(xv, yv, round(3 + 7 * k / 19, 4)) for k, (xv, yv) in
            enumerate([(xv, yv) for xv in (-3.0, -2.5, -2.0, -1.5) for yv in (0.5, 1.125, 1.75, 2.375, 3.0)])]
BERGER_T = [3.0, 3.55, 5.0, 10.0]


def berger(t):
    return (2 - t) / np.sqrt(t), np.sqrt(t)


def main():
    R = {}
    # ---------------- S1 cut
    s1 = []
    starts = [("berger", *berger(t), t) for t in BERGER_T] + [("D", *p) for p in D_POINTS]
    for tag, x0, y0, t in starts:
        sol = flow(x0, 0.0, t, dense=True)
        ymax = float(np.abs(sol.y[1]).max())
        land = landing(sol, t)
        ss = np.linspace(0, 5, 5001)
        m = np.array([lam_min(*sol.sol(s), t) for s in ss])
        mono = bool(np.all(np.diff(m) >= -1e-9))
        s1.append(dict(start=tag, x0=x0, y0=y0, t=t, price_before=lam_min(x0, y0, t), price_after_cut=lam_min(x0, 0, t),
                       floor_violations_after_cut=floor_report(x0, 0.0, t), ymax=ymax, landing=land, monotone=mono,
                       min_step=float(np.diff(m).min())))
    land_floor = {t: floor_report(0.0, 0.0, t) for t in sorted({r["t"] for r in s1})}
    R["S1"] = dict(runs=s1, floor_at_A0=land_floor,
                   S1a=all(r["ymax"] <= 1e-12 and r["landing"] == "A=0" for r in s1),
                   S1b=all(len(v) == 0 for t, v in land_floor.items() if t >= 3),
                   S1c=all(r["monotone"] for r in s1))
    # ---------------- S2 flip
    s2 = []
    for x0, y0, t in D_POINTS:
        xf = -x0
        below = floor_report(xf, y0, t)
        lstar = np.sqrt(t) * (4 * y0 ** 2 - 4 * np.sqrt(t) * xf - 8 * xf / np.sqrt(t)) / (8 * xf)
        sol = flow(xf, y0, t, dense=True)
        cr, _ = crossing(sol, t)
        s2.append(dict(x0=xf, y0=y0, t=t, lstar=float(lstar), theorem_inside=bool(lstar < 20),
                       sectors_below=below[:6], n_sectors_below=len(below), price=lam_min(xf, y0, t),
                       landing=landing(sol, t), price_turns_positive=cr))
    R["S2"] = dict(runs=s2, S2a=all(r["n_sectors_below"] > 0 for r in s2 if r["theorem_inside"]))
    # ---------------- S3 overload
    s3 = []
    for t in (3.0, 3.55, 5.0):
        xb, yb = berger(t)
        for s in (1.5, 2.0, 5.0, 10.0):
            x0, y0 = s * xb, s * yb
            sol = flow(x0, y0, t, dense=True)
            cr, _ = crossing(sol, t)
            s3.append(dict(t=t, s=s, ratio=lam_min(x0, y0, t) / s ** 2, target=(4 - 4 * t) / t,
                           n_sectors_below=len(floor_report(x0, y0, t)), landing=landing(sol, t),
                           price_turns_positive=cr, energy=float(Ef(x0, y0, t))))
    R["S3"] = dict(runs=s3, S3a=all(abs(r["ratio"] / r["target"] - 1) <= 0.05 for r in s3 if r["s"] == 10.0))
    # ---------------- S4 crush and stretch
    s4 = []
    for t in (0.25, 0.5, 1.0, 1.5, 2.0, 2.5, 2.75, 12.0, 15.0, 20.0, 30.0, 50.0):
        below = floor_report(*berger(t), t)
        s4.append(dict(t=t, n_sectors_below=len(below), sectors_below=below[:6]))
    R["S4"] = dict(runs=s4, S4a=all(r["n_sectors_below"] == 0 for r in s4 if r["t"] >= 2.75),
                   S4b=all(r["n_sectors_below"] > 0 for r in s4 if r["t"] <= 2.5))
    # ---------------- S5 basin map at t = 3
    t = 3.0
    xs = np.linspace(-3, 3, 41); ys = np.linspace(0, 3, 41)
    lab = np.empty((41, 41), dtype=object)
    for i, yv in enumerate(ys):
        for k, xv in enumerate(xs):
            lab[i, k] = landing(flow(xv, yv, t), t)
    counts = {v: int((lab == v).sum()) for v in ("A=0", "A=theta", "other")}
    others = [(float(xs[k]), float(ys[i])) for i in range(41) for k in range(41) if lab[i, k] == "other"]
    def on_boundary(i, k):
        nb = {lab[a, b] for a in range(max(0, i - 1), min(41, i + 2)) for b in range(max(0, k - 1), min(41, k + 2))}
        return ("A=0" in nb and "A=theta" in nb)
    other_on_boundary = all(on_boundary(i, k) for i in range(41) for k in range(41) if lab[i, k] == "other")
    # squashed meron: non-flat zeros of grad E with x > 0
    mer = []
    for x0 in np.linspace(0.05, 3, 15):
        for y0 in np.linspace(0.1, 3, 15):
            sol, info, ier, _ = fsolve(lambda v: [gx(v[0], v[1], t), gy(v[0], v[1], t)], [x0, y0], full_output=True)
            if ier == 1 and sol[0] > 0 and sol[1] > 0 and Ef(*sol, t) > 1e-8:
                if not any(np.hypot(*(sol - m_)) < 1e-6 for m_ in mer):
                    mer.append(sol)
    s5b = []
    for m_ in mer:
        H = np.array(Hf(m_[0], m_[1], t), float)
        Minv = np.diag([1.0, 0.5])
        w, V = np.linalg.eig(Minv @ H)
        k = int(np.argmin(w.real)); v = V[:, k].real; v /= np.linalg.norm(v)
        ends = [landing(flow(*(m_ + sg * 1e-4 * v), t), t) for sg in (+1, -1)]
        ki = int(np.argmin(np.abs(xs - m_[0]))); ii = int(np.argmin(np.abs(ys - m_[1])))
        s5b.append(dict(x=float(m_[0]), y=float(m_[1]), E=float(Ef(*m_, t)), eig=[float(a) for a in w.real],
                        unstable_ends=ends, near_boundary=on_boundary(ii, ki)))
    R["S5"] = dict(counts=counts, others=others[:20], other_on_boundary=other_on_boundary, meron=s5b,
                   S5a=(counts["other"] == 0 or other_on_boundary),
                   S5b=bool(s5b) and all(m["near_boundary"] and len(set(m["unstable_ends"])) == 2 for m in s5b),
                   grid=[[lab[i, k] for k in range(41)] for i in range(41)])
    out = HERE / "stress_receipt.json"
    out.write_text(json.dumps(R, indent=1, default=float))
    for key in ("S1", "S2", "S3", "S4", "S5"):
        verdicts = {k: v for k, v in R[key].items() if k.startswith(key) and len(k) == 3}
        print(key, verdicts)
    print("S5 counts", counts, "| merons", [(round(m["x"], 4), round(m["y"], 4), round(m["E"], 4), m["unstable_ends"],
                                             m["near_boundary"]) for m in s5b])
    print("wrote", out)


if __name__ == "__main__":
    main()
