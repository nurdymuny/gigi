"""The alignment test (Test 4; docs/research/alignment-test-preregistration-2026-11-21.md, fixed before this script).

Backgrounds A = s theta (Maurer-Cartan) on the unstretched group, s = 1.05..3.00 (40 points), where the field strength
is aligned with the twist's spin (s_F < 0). Families: 4a SU(3) (nine sectors, as Test 2 family 2), 4b SU(2)
(sectors j = 0..12, theta = sum 2 e_a theta^a in the gauge algebra, phi(e_a) = 2 tau_a).
Prediction: each family E or P (the twist is the floor of every listed sector on a run of >= 2 grid points).
modes:  controls OUT.json  |  family 4a|4b OUT.json [NPROC]
usage: python scripts/ricci_alignment_test.py MODE ...
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su3_test2_family as T            # noqa: E402
import ricci_su3_gyromagnetic_share as GS     # noqa: E402

S_GRID = np.round(np.arange(21, 61) * 0.05, 4)     # 1.05 .. 3.00
J2 = 12


def su2_background(s):
    S, C2, eps = T.su2_data()
    return C2, eps, [2 * s * np.eye(3)[a] for a in range(3)]


def su2_charge(rho, eps, C2):
    return (1j * C2[0].T, 1j * 2 * T.adm(np.eye(3)[0], eps), 1j * rho[0])


def su2_point(s, js=range(J2 + 1)):
    S, _, _ = T.su2_data()
    C2, eps, M = su2_background(s)
    g = [1.0, 1.0, 1.0]
    out = {"s": float(s), "grad_norm": T.grad_norm(C2, g, eps, M), "sectors": {}}
    q, beta = GS.top_frame_state(1j * C2[0].T, 1j * 2 * T.adm(np.eye(3)[0], eps), 3, 3)
    out["ricci_share"], out["field_share"] = GS.shares(C2, g, eps, M, beta)
    for j in js:
        rho = S.spin_rep(j); d = j + 1
        H, G = T.hessian_and_gauge(C2, g, eps, M, rho)
        parts = su2_charge(rho, eps, C2)
        signs = T.fit_signs(parts, H, G @ G.conj().T, 3, 3, d)
        if len(signs) != 1:
            out["sectors"][str(j)] = {"error": f"signs {signs}"}; continue
        Q = T.assemble_Q(parts, signs[0], 3, 3, d)
        if abs(np.linalg.eigvalsh(Q).max() - (j + 4)) > 1e-8:
            Q = -Q
        row = T.classify_point(H, G, Q, j + 4)
        if "error" not in row:
            mu = T.slice_spectrum(H, G)
            row["rank_of_twist"] = int((mu < row["extremal"] - 1e-9 * max(1, abs(row["extremal"]))).sum()) + 1
            row["slice_dim"] = int(len(mu))
        out["sectors"][str(j)] = row
    return out


_SU3 = {}


def su3_point(s):
    if not _SU3:
        X, C = T.su3_data(); _SU3["X"], _SU3["C"] = X, C
        _SU3["rho"] = {pq: T.su3_irrep(*pq, X) for pq in T.SECTORS}
        rho0 = T.su3_irrep(0, 0, X); Af, Ag, _ = T.su3_charge(rho0, X, C)
        _SU3["beta"] = GS.top_frame_state(Af, Ag, 8, 8)[1]
    X, C = _SU3["X"], _SU3["C"]
    g = np.ones(8); M = [s * np.eye(8)[a] for a in range(8)]
    out = {"s": float(s), "grad_norm": T.grad_norm(C, g, C, M), "sectors": {}}
    out["ricci_share"], out["field_share"] = GS.shares(C, g, C, M, _SU3["beta"])
    for pq in T.SECTORS:
        rho = _SU3["rho"][pq]; d = rho[0].shape[0]
        H, G = T.hessian_and_gauge(C, g, C, M, rho)
        parts = T.su3_charge(rho, X, C)
        signs = T.fit_signs(parts, H, G @ G.conj().T, 8, 8, d)
        if len(signs) != 1:
            out["sectors"][str(pq)] = {"error": f"signs {signs}"}; continue
        Q = T.assemble_Q(parts, signs[0], 8, 8, d)
        if abs(np.linalg.eigvalsh(Q).max() - (pq[0] + pq[1] + 4)) > 1e-8:
            Q = -Q
        row = T.classify_point(H, G, Q, pq[0] + pq[1] + 4)
        if "error" not in row:
            mu = T.slice_spectrum(H, G)
            row["rank_of_twist"] = int((mu < row["extremal"] - 1e-9 * max(1, abs(row["extremal"]))).sum()) + 1
            row["slice_dim"] = int(len(mu))
        out["sectors"][str(pq)] = row
    return out


def classify(pts, keys):
    errors = [(p["s"], k, v["error"]) for p in pts for k, v in p["sectors"].items() if "error" in v]
    floor_at = {k: [p["s"] for p in pts if p["sectors"][k].get("is_floor")] for k in keys}
    s_all = [p["s"] for p in pts if all(p["sectors"][k].get("is_floor") for k in keys)]
    runs = []
    for s in s_all:
        if runs and abs(s - runs[-1][1] - 0.05) < 1e-9:
            runs[-1][1] = s
        else:
            runs.append([s, s])
    cls = "E" if len(s_all) == len(pts) else ("P" if any(b - a > 1e-9 for a, b in runs) else "N")
    unaligned = [p["s"] for p in pts if p["field_share"] >= 0]
    return {"stopped": bool(errors), "errors": errors[:20], "class": None if errors else cls, "S_all_runs": runs,
            "n_S_all": len(s_all), "per_sector_runs": {k: [[a, b] for a, b in _runs(v)] for k, v in floor_at.items()},
            "precondition_A2_unaligned_points": unaligned}


def _runs(v):
    r = []
    for s in v:
        if r and abs(s - r[-1][1] - 0.05) < 1e-9:
            r[-1][1] = s
        else:
            r.append([s, s])
    return r


def controls(out):
    res = {}
    S, C2, eps = T.su2_data()
    X, C = T.su3_data()
    e1 = {"SU(2) energy at s=1": T.energy(C2, [1, 1, 1], eps, su2_background(1.0)[2]),
          "SU(3) energy at s=1": T.energy(C, np.ones(8), C, [np.eye(8)[a] for a in range(8)]),
          "SU(2) grad at s=1/2": T.grad_norm(C2, [1, 1, 1], eps, su2_background(0.5)[2]),
          "SU(3) grad at s=1/2": T.grad_norm(C, np.ones(8), C, [0.5 * np.eye(8)[a] for a in range(8)])}
    res["C1"] = {k: float(v) for k, v in e1.items()}
    res["C1"]["pass"] = bool(e1["SU(2) energy at s=1"] <= 1e-20 and e1["SU(3) energy at s=1"] <= 1e-20
                             and e1["SU(2) grad at s=1/2"] <= 1e-8 and e1["SU(3) grad at s=1/2"] <= 1e-8)
    lows = []
    _, _, M = su2_background(0.5)
    for j in range(5):
        H, G = T.hessian_and_gauge(C2, [1, 1, 1], eps, M, S.spin_rep(j))
        lows.append(float(T.slice_spectrum(H, G)[0]))
    target = [-2, 1, 6, 7, 10]
    res["C2"] = {"lowest_by_sector": lows, "target": target,
                 "pass": bool(max(abs(a - b) for a, b in zip(lows, target)) <= 1e-9)}
    k1 = T.control_K1(); k3 = T.control_K3()
    res["C3"] = {"K1": k1, "K3": k3, "pass": bool(k1["pass"] and k3["pass"])}
    res["all_pass"] = all(res[k]["pass"] for k in ("C1", "C2", "C3"))
    Path(out).write_text(json.dumps(res, indent=1, default=str) + "\n")
    print(json.dumps({k: (v["pass"] if isinstance(v, dict) else v) for k, v in res.items()}), res["C2"]["lowest_by_sector"], res["C1"])


def main():
    mode = sys.argv[1]
    if mode == "controls":
        controls(sys.argv[2]); return
    fam, out = sys.argv[2], sys.argv[3]
    nproc = int(sys.argv[4]) if len(sys.argv) > 4 else 4
    import multiprocessing as mp
    if fam == "4b":
        pts = [su2_point(float(s)) for s in S_GRID]
        keys = [str(j) for j in range(J2 + 1)]
    else:
        with mp.Pool(nproc) as pool:
            pts = pool.map(su3_point, [float(s) for s in S_GRID])
        keys = [str(pq) for pq in T.SECTORS]
    summ = classify(pts, keys); summ["family"] = fam
    Path(out).write_text(json.dumps({"summary": summ, "points": pts}, indent=1) + "\n")
    print(json.dumps(summ, indent=1)[:3000])


if __name__ == "__main__":
    main()
