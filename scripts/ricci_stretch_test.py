"""The stretch test (Test 5; docs/research/stretch-test-preregistration-2026-11-21.md, fixed before this script).

SU(2), A = s theta on the Berger metric (t, 1, 1), M_a = 2 s e_a / sqrt(g_a). Each grid point is classified from the
curvature term R0 = Ric - 2[F, .] alone (L1 singled out, L2 shared, L0 not lowest), before the floor test.
P-nec: no L0/L2 point in T_all.  P-suf: T_all within L1 has two neighbouring grid points.
Controls D1 (meron spectrum), D2 (Weitzenbock form with this R0), D3 (agreement with Test 4b at t = 1).
usage: python scripts/ricci_stretch_test.py OUT.json
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su3_test2_family as T            # noqa: E402
import ricci_su3_gyromagnetic_share as GS     # noqa: E402
import ricci_su2_floor_vs_ceiling as FC       # noqa: E402

S_VALS = [0.25, 0.5, 0.75, 1.25, 1.5, 2.0, 2.5, 3.0]
T_VALS = [0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 4.0, 6.0, 8.0]
JMAX = 12


def background(s, t):
    g = [t, 1.0, 1.0]
    return g, [2 * s * np.eye(3)[a] / np.sqrt(g[a]) for a in range(3)]


def r0_matrix(s, t):
    S, C2, eps = T.su2_data()
    g, M = background(s, t)
    Ric = np.kron(GS.ricci(C2, g), np.eye(3))
    return Ric - 2 * GS.field_strength_op(C2, g, eps, M)


def beta_state():
    S, C2, eps = T.su2_data()
    return GS.top_frame_state(1j * C2[0].T, 1j * 2 * T.adm(np.eye(3)[0], eps), 3, 3)[1]


def l_class(s, t, beta):
    R0 = r0_matrix(s, t); R0 = 0.5 * (R0 + R0.T)
    w = np.linalg.eigvalsh(R0); w0 = w[0]
    m = int((w < w0 + 1e-9).sum())
    rb = float((beta.conj() @ R0 @ beta).real)
    if rb <= w0 + 1e-9:
        cls = "L1" if m == 2 else "L2"
    else:
        cls = "L0"
    nxt = float(w[m]) if m < len(w) else None
    return cls, {"R0_lowest": float(w0), "multiplicity": m, "beta_value": rb, "gap_to_next": (nxt - w0) if nxt is not None else None}


def point(s, t):
    S, C2, eps = T.su2_data()
    g, M = background(s, t)
    out = {"s": s, "t": t, "grad_norm": T.grad_norm(C2, g, eps, M), "sectors": {}}
    beta = beta_state()
    out["ricci_share"], out["field_share"] = GS.shares(C2, g, eps, M, beta)
    out["L"], out["R0"] = l_class(s, t, beta)
    for j in range(JMAX + 1):
        rho = S.spin_rep(j); d = j + 1
        H, G = T.hessian_and_gauge(C2, g, eps, M, rho)
        parts = (1j * C2[0].T, 1j * 2 * T.adm(np.eye(3)[0], eps), 1j * rho[0])
        signs = T.fit_signs(parts, H, G @ G.conj().T, 3, 3, d)
        if len(signs) != 1:
            out["sectors"][str(j)] = {"error": f"signs {signs}"}; continue
        Q = T.assemble_Q(parts, signs[0], 3, 3, d)
        if abs(np.linalg.eigvalsh(Q).max() - (j + 4)) > 1e-8:
            Q = -Q
        out["sectors"][str(j)] = T.classify_point(H, G, Q, j + 4)
    out["floor_all"] = all(v.get("is_floor") for v in out["sectors"].values())
    out["errors"] = [k for k, v in out["sectors"].items() if "error" in v]
    return out


def controls():
    res = {}
    S, C2, eps = T.su2_data()
    g, M = background(0.5, 1.0)
    lows = [float(T.slice_spectrum(*T.hessian_and_gauge(C2, g, eps, M, S.spin_rep(j)))[0]) for j in range(5)]
    res["D1"] = {"lows": lows, "pass": max(abs(a - b) for a, b in zip(lows, [-2, 1, 6, 7, 10])) <= 1e-9}
    worst = 0.0
    for (s, t) in ((0.5, 3.0), (1.5, 1.0), (2.0, 4.0), (3.0, 8.0)):
        g, M = background(s, t); R0 = r0_matrix(s, t)
        for j in (1, 2, 4):
            _, _, R, R0w, _ = FC.weitzenbock_parts(t, np.array(M), j)
            worst = max(worst, float(np.abs(R - np.kron(R0, np.eye(j + 1))).max()))
    res["D2"] = {"max_dev": worst, "pass": worst <= 1e-10}
    ref = json.loads((Path(__file__).resolve().parent.parent / "research/su2-floor-family-20261119/alignment_test_4b.json").read_text())
    refpts = {p["s"]: p for p in ref["points"]}
    mism = []
    for s in (1.5, 2.0, 2.5, 3.0):
        p = point(s, 1.0)
        for j in range(JMAX + 1):
            if bool(p["sectors"][str(j)].get("is_floor")) != bool(refpts[s]["sectors"][str(j)].get("is_floor")):
                mism.append((s, j))
    res["D3"] = {"mismatches": mism, "pass": not mism}
    res["all_pass"] = all(res[k]["pass"] for k in ("D1", "D2", "D3"))
    return res


def main():
    out = Path(sys.argv[1])
    ctl = controls()
    print("controls:", {k: (v["pass"] if isinstance(v, dict) else v) for k, v in ctl.items()}, "D2 dev", ctl["D2"]["max_dev"], flush=True)
    if not ctl["all_pass"]:
        out.write_text(json.dumps({"controls": ctl}, indent=1, default=str) + "\n")
        raise SystemExit("controls failed: no family result is read")
    pts = [point(s, t) for t in T_VALS for s in S_VALS]
    errors = [(p["s"], p["t"], p["errors"]) for p in pts if p["errors"]]
    key = {(p["s"], p["t"]): p for p in pts}
    tall = [k for k, p in key.items() if p["floor_all"]]
    pnec_viol = [k for k in tall if key[k]["L"] != "L1"]
    l1_tall = [k for k in tall if key[k]["L"] == "L1"]
    nb = []
    for (s, t) in l1_tall:
        si, ti = S_VALS.index(s), T_VALS.index(t)
        for (s2, t2) in l1_tall:
            sj, tj = S_VALS.index(s2), T_VALS.index(t2)
            if (abs(si - sj) + abs(ti - tj)) == 1:
                nb.append(((s, t), (s2, t2)))
    table = {}
    for p in pts:
        table.setdefault(p["L"], {"floor": 0, "not_floor": 0})
        table[p["L"]]["floor" if p["floor_all"] else "not_floor"] += 1
    summ = {"stopped": bool(errors), "errors": errors[:10], "T_all": sorted(tall), "P_nec_violations": pnec_viol,
            "P_nec": not pnec_viol, "L1_and_floor": sorted(l1_tall), "neighbour_pairs_in_L1_floor": len(nb) // 2,
            "P_suf": len(nb) > 0, "agreement_table": table,
            "fraction_of_L1_in_T_all": table.get("L1", {}).get("floor", 0) / max(1, sum(table.get("L1", {"floor": 0, "not_floor": 0}).values()))}
    out.write_text(json.dumps({"controls": ctl, "summary": summ, "points": pts}, indent=1, default=str) + "\n")
    print(json.dumps(summ, indent=1, default=str))
    print("map (rows t, cols s): L-class / F if floor of every sector")
    print("t\\s   " + "  ".join(f"{s:>5}" for s in S_VALS))
    for t in T_VALS:
        print(f"{t:<5} " + "  ".join(f"{key[(s, t)]['L'] + ('F' if key[(s, t)]['floor_all'] else '.'):>5}" for s in S_VALS))


if __name__ == "__main__":
    main()
