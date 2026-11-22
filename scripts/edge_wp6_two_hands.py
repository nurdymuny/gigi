"""Pre-registration E (research/edge-theorem-20261122-e/wp6_prereg_E_two_hands.md, revised after review): the two hands.

Per point and sector j <= 400 (H + GG*, closed-form charge blocks, ALL blocks including hers): the floor eigenspace
(every eigenvector within 1e-10 max(1,|mu|) of the lowest), its across-reaching projector weight A (states with
c_l*m*x < 0), its sister weight S (states (m=l,(1,-1)) and (m=-l,(-1,1))), the onset j0 (A >= 0.5 from j0 to 400), and the
registered first-order impurity (1-S)_pred at j = 400 from the rows of block q = l.
Gates E-code-1 (incl. j = 120, 200), E-code-2, E-y0, E-mirror, E-ctrl stop the run before any prediction is read.
usage: OMP_NUM_THREADS=1 python scripts/edge_wp6_two_hands.py OUT.json
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_exact_edge as EE          # noqa: E402
import ricci_su3_test2_family as T2        # noqa: E402

JMAX = 400
BL = EE.Blocks()
CH = BL.ch
XS, YS, TS = (0.05, 0.2, 0.5, 1.0, 2.0), (0.0, 0.5, 1.5, 3.0), (2.0, 3.0, 4.0, 6.0, 10.0)


def blocks_of(d, j, tv):
    l = j / 2; out = []
    for q in np.arange(-l - 2, l + 2 + 1e-9, 1.0):
        st, B = BL.block(d, j, q, tv)
        if B.size:
            out.append((float(q), st, B))
    return out


def floor_info(xv, yv, tv, j, d):
    l = j / 2; eig = []
    for q, st, B in blocks_of(d, j, tv):
        w, V = np.linalg.eigh(B)
        eig.append((q, st, w, V))
    mu = min(e[2][0] for e in eig); tol = 1e-10 * max(1.0, abs(mu))
    A = S = 0.0; n = 0; qs = []
    for q, st, w, V in eig:
        for k in np.where(w <= mu + tol)[0]:
            v = np.abs(V[:, k]) ** 2; n += 1; qs.append(q)
            A += sum(v[i] for i, (m, a) in enumerate(st) if CH[a][1] * m * xv < 0)
            S += sum(v[i] for i, (m, a) in enumerate(st)
                     if (abs(m - l) < 1e-9 and CH[a] == (1, -1)) or (abs(m + l) < 1e-9 and CH[a] == (-1, 1)))
    return {"j": j, "mu": float(mu), "dim": n, "qs": sorted(set(qs)), "A": float(A / n), "S": float(S / n)}


def impurity_pred(d, j, tv):
    l = j / 2
    st, B = BL.block(d, j, l, tv)
    s = [i for i, (m, a) in enumerate(st) if abs(m - l) < 1e-9 and CH[a] == (1, -1)][0]
    D = np.real(np.diag(B))
    return float(sum(abs(B[s, k]) ** 2 / (D[k] - D[s]) ** 2 for k in range(len(st)) if k != s))


def rel(a, b):
    return float(np.abs(np.asarray(a) - np.asarray(b)).max() / max(1.0, np.abs(np.asarray(a)).max()))


def main():
    out_path = Path(sys.argv[1]); res = {"protocol": "wp6_prereg_E_two_hands.md (revised after review)"}

    def save():
        out_path.write_text(json.dumps(res, indent=1, default=str) + "\n")

    def stop(msg):
        res["stopped"] = msg; save(); raise SystemExit("stop: " + msg)
    S_, C2, eps = T2.su2_data(); rng = np.random.default_rng(622); c1 = 0.0
    checks = [(rng.uniform(0.05, 2), rng.uniform(0, 3), rng.uniform(2, 10), int(rng.integers(0, 13))) for _ in range(10)]
    checks += [(0.5, 1.5, 4.0, 120), (1.0, 3.0, 10.0, 200)]
    for (xv, yv, tv, j) in checks:
        H, G = T2.hessian_and_gauge(C2, [tv, 1, 1], eps, [xv * np.eye(3)[0], yv * np.eye(3)[1], yv * np.eye(3)[2]], S_.spin_rep(j))
        c1 = max(c1, rel(np.sort(np.linalg.eigvalsh(H + G @ G.conj().T)), BL.sector_spectrum(xv, yv, tv, j)))
    res["E_code_1"] = c1; print("E-code-1", c1, flush=True)
    if c1 > 1e-12:
        stop("E-code-1")
    pts = [(x, y, t) for x in XS for y in YS for t in TS]
    c2 = y0 = mir = 0.0
    for (xv, yv, tv) in pts:
        d = BL.data(xv, yv, tv)
        for j in range(JMAX + 1):
            l = j / 2; lam = EE.lam_tw(xv, yv, tv, j)
            _, B = BL.block(d, j, l + 2, tv); c2 = max(c2, abs(B[0, 0].real - lam) / max(1.0, abs(lam)))
            if yv == 0.0:
                st, B = BL.block(d, j, l, tv)
                k = [i for i, (m, a) in enumerate(st) if abs(m - l) < 1e-9 and CH[a] == (1, -1)]
                if k:
                    e = np.zeros(len(st)); e[k[0]] = 1; val = float((e @ B @ e).real)
                    y0 = max(y0, float(np.linalg.norm(B @ e - val * e)) / max(1.0, np.linalg.norm(B)),
                             abs(val - EE.lam_tw(-xv, 0.0, tv, j)) / max(1.0, abs(val)))
            if j in (50, 200, 400):
                for q in np.arange(0.5 if j % 2 else 1.0, l + 2 + 1e-9, 1.0):
                    _, Bp = BL.block(d, j, q, tv); _, Bm = BL.block(d, j, -q, tv)
                    if Bp.size:
                        mir = max(mir, rel(np.linalg.eigvalsh(Bp), np.linalg.eigvalsh(Bm)))
    res["E_code_2"], res["E_y0"], res["E_mirror"] = c2, y0, mir
    print("E-code-2", c2, "E-y0", y0, "E-mirror", mir, flush=True)
    if c2 > 1e-12 or y0 > 1e-12 or mir > 1e-12:
        stop("E-code-2 / E-y0 / E-mirror")
    ctrl = 1e9
    for (xv, yv, tv) in ((-2.0, 1.0, 4.0), (-1.75, 2.5, 8.0), (-2.5, 0.75, 6.0)):
        d = BL.data(xv, yv, tv)
        for j in range(JMAX + 1):
            l = j / 2; lam = EE.lam_tw(xv, yv, tv, j)
            others = min(np.linalg.eigvalsh(B)[0] for q, st, B in blocks_of(d, j, tv) if abs(abs(q) - (l + 2)) > 1e-9)
            ctrl = min(ctrl, others - lam)
    res["E_ctrl_min_margin"] = ctrl; print("E-ctrl", ctrl, flush=True)
    if ctrl <= 0:
        stop("E-ctrl")
    save()
    rows = []
    for (xv, yv, tv) in pts:
        d = BL.data(xv, yv, tv)
        F = [floor_info(xv, yv, tv, j, d) for j in range(JMAX + 1)]
        j0 = None
        for j in range(JMAX, -1, -1):
            if F[j]["A"] >= 0.5:
                j0 = j
            else:
                break
        f400 = F[JMAX]; l = JMAX / 2
        in_pm_l = all(abs(abs(q) - l) < 1e-9 for q in f400["qs"])
        r = {"x": xv, "y": yv, "t": tv, "scored": yv > 0 and tv >= 3, "j0": j0, "A400": f400["A"], "S400": f400["S"],
             "qs400": f400["qs"], "floor400_in_pm_l": in_pm_l, "impurity_pred": impurity_pred(d, JMAX, tv),
             "impurity_meas": 1 - f400["S"],
             "sample": [(f["j"], f["qs"], round(f["A"], 4), round(f["S"], 4)) for f in F if f["j"] in (0, 4, 12, 50, 100, 200, 300, 400)]}
        rows.append(r); res["points"] = rows; save()
        print(xv, yv, tv, "j0", j0, "qs400", f400["qs"], "A400", round(f400["A"], 4), "1-S meas/pred",
              round(r["impurity_meas"], 5), round(r["impurity_pred"], 5), flush=True)
    sc = [r for r in rows if r["scored"]]
    res["E1_fail"] = [(r["x"], r["y"], r["t"], r["j0"]) for r in sc if r["j0"] is None or r["j0"] > 300]
    res["E1_fraction_pass"] = 1 - len(res["E1_fail"]) / len(sc); res["E1"] = not res["E1_fail"]
    get = {(r["x"], r["y"], r["t"]): r["j0"] for r in rows}
    ey = []
    for xv in XS:
        for tv in TS[1:]:
            a, b = get[(xv, 1.5, tv)], get[(xv, 3.0, tv)]
            if a is not None and b is not None and a >= 10:
                ey.append((xv, tv, a, b, b / a, 2 <= b / a <= 8))
    ex = []
    for yv in YS[1:]:
        for tv in TS[1:]:
            a, b = get[(0.2, yv, tv)], get[(1.0, yv, tv)]
            if a is not None and b is not None and b >= 10:
                ex.append((yv, tv, a, b, a / b, 2.5 <= a / b <= 10))
    for name, lst in (("E2_y", ey), ("E2_x", ex)):
        res[name + "_pairs"] = lst
        res[name] = "not scored" if len(lst) < 5 else (sum(p[-1] for p in lst) / len(lst) >= 0.8)
    e3 = [(r["x"], r["y"], r["t"], r["impurity_meas"], r["impurity_pred"],
           (r["impurity_pred"] > 0 and 1 / 1.5 <= r["impurity_meas"] / r["impurity_pred"] <= 1.5)) for r in sc if r["floor400_in_pm_l"]]
    res["E3_points"] = e3
    res["E3"] = "not scored" if len(e3) < 10 else (sum(p[-1] for p in e3) / len(e3) >= 0.8)
    save()
    print(json.dumps({k: v for k, v in res.items() if k != "points"}, indent=1, default=str))


if __name__ == "__main__":
    main()
