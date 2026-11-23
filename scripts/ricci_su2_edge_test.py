"""The edge test (Test 6; docs/research/edge-test-preregistration-2026-11-21.md, fixed before this script).

phase fit:  edges x*(t, y) by bisection at training stretches t = 2, 3, 4, 6, heights y = 0.5, 1.5, sqrt(t); fit
            x*(t) = c1/t + c2/t^2 per height; write predictions (held-out edges, Berger crossing t_c).
phase test: measure held-out edges at t = 2.5, 5, 8; check E1 (|measured - predicted| <= 0.05) and E2
            (|t_c - 2.6980| <= 0.05; Berger field floor at t_c + 0.05, not at t_c - 0.05).
usage: python scripts/ricci_su2_edge_test.py fit OUT_PRED.json  |  test PRED.json OUT.json
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su3_test2_family as T2       # noqa: E402

JMAX = 12
TRAIN, HELD = [2.0, 3.0, 4.0, 6.0], [2.5, 5.0, 8.0]
HEIGHTS = ["0.5", "1.5", "sqrt_t"]
T_STAR = 2.6980269405200263
_C = {}


def setup():
    if not _C:
        S, C2, eps = T2.su2_data()
        _C.update(C2=C2, eps=eps, rho={j: S.spin_rep(j) for j in range(JMAX + 1)})


def lam(x, y, t, j):
    return (x + (j + 2) / np.sqrt(t)) ** 2 + 2 * np.sqrt(t) * x - y ** 2


def is_floor(x, y, t):
    setup()
    g = [t, 1.0, 1.0]; M = [x * np.eye(3)[0], y * np.eye(3)[1], y * np.eye(3)[2]]
    for j in range(JMAX + 1):
        H, G = T2.hessian_and_gauge(_C["C2"], g, _C["eps"], M, _C["rho"][j])
        mu = T2.slice_spectrum(H, G)
        lt = lam(x, y, t, j)
        if lt > mu[0] + 1e-9 * max(1.0, abs(mu[0])):
            return False
    return True


def height(h, t):
    return np.sqrt(t) if h == "sqrt_t" else float(h)


def edge(t, y):
    lo, hi = -3.0, 1.0
    if not is_floor(lo, y, t) or is_floor(hi, y, t):
        return {"edge": None, "note": f"bracket fails: floor(-3) = {is_floor(lo, y, t)}, floor(1) = {is_floor(hi, y, t)}"}
    while hi - lo > 1e-4:
        mid = 0.5 * (lo + hi)
        if is_floor(mid, y, t):
            lo = mid
        else:
            hi = mid
    grid = np.arange(-3.0, 1.0 + 1e-9, 0.05)
    flags = [is_floor(float(xv), y, t) for xv in grid]
    interval = all(f == (xv <= lo + 1e-9) for f, xv in zip(flags, grid))
    return {"edge": 0.5 * (lo + hi), "floor_set_is_interval_on_0.05_grid": interval}


def job(args):
    t, h = args
    return (t, h, edge(t, height(h, t)))


def fit():
    import multiprocessing as mp
    with mp.Pool(6) as pool:
        res = pool.map(job, [(t, h) for t in TRAIN for h in HEIGHTS])
    train = {h: {str(t): r for (t, hh, r) in res if hh == h} for h in HEIGHTS}
    models, preds = {}, {}
    for h in HEIGHTS:
        ts = np.array(TRAIN); xs = np.array([train[h][str(t)]["edge"] for t in TRAIN])
        A = np.vstack([1 / ts, 1 / ts ** 2]).T
        c, *_ = np.linalg.lstsq(A, xs, rcond=None)
        models[h] = {"c1": float(c[0]), "c2": float(c[1]),
                     "train_residuals": [float(v) for v in xs - A @ c]}
        preds[h] = {str(t): float(c[0] / t + c[1] / t ** 2) for t in HELD}
    # Berger crossing at height sqrt(t): (2 - t)/sqrt(t) = c1/t + c2/t^2
    c1, c2 = models["sqrt_t"]["c1"], models["sqrt_t"]["c2"]
    f = lambda t: (2 - t) / np.sqrt(t) - (c1 / t + c2 / t ** 2)       # noqa: E731
    a, b = 2.0, 3.5
    if f(a) * f(b) > 0:
        tc = None
    else:
        for _ in range(200):
            m = 0.5 * (a + b)
            if f(a) * f(m) <= 0:
                b = m
            else:
                a = m
        tc = 0.5 * (a + b)
    out = {"training_edges": train, "models": models, "predicted_edges": preds, "predicted_berger_crossing": tc}
    Path(sys.argv[2]).write_text(json.dumps(out, indent=1) + "\n")
    print(json.dumps(out, indent=1))


def test():
    import multiprocessing as mp
    pred = json.loads(Path(sys.argv[2]).read_text())
    with mp.Pool(6) as pool:
        res = pool.map(job, [(t, h) for t in HELD for h in HEIGHTS])
    e1 = []
    for (t, h, r) in res:
        p = pred["predicted_edges"][h][str(t)]
        m = r["edge"]
        e1.append({"t": t, "height": h, "predicted": p, "measured": m, "interval": r.get("floor_set_is_interval_on_0.05_grid"),
                   "error": None if m is None else m - p, "pass": m is not None and abs(m - p) <= 0.05})
    tc = pred["predicted_berger_crossing"]
    def berger_floor(t):
        return is_floor((2 - t) / np.sqrt(t), np.sqrt(t), t)
    e2 = {"t_c": tc, "t_star": T_STAR, "distance": None if tc is None else abs(tc - T_STAR),
          "floor_at_tc_plus": None if tc is None else berger_floor(tc + 0.05),
          "floor_at_tc_minus": None if tc is None else berger_floor(tc - 0.05)}
    e2["pass"] = tc is not None and e2["distance"] <= 0.05 and e2["floor_at_tc_plus"] and not e2["floor_at_tc_minus"]
    out = {"E1": e1, "E1_pass": all(r["pass"] for r in e1), "E2": e2}
    Path(sys.argv[3]).write_text(json.dumps(out, indent=1) + "\n")
    print(json.dumps(out, indent=1))


if __name__ == "__main__":
    {"fit": fit, "test": test}[sys.argv[1]]()
