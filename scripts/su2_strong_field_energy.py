"""S4: lattice energies of a strong SU(2) field — chord, arc, tangent, the 2:1 blend, adjoint.
Spec: docs/STRONG_FIELD_ENERGY_SPEC.md. Research script, not an engine verb.

usage: python scripts/su2_strong_field_energy.py [RECEIPT.json]
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import ricci_su2_pg_flow as FL          # noqa: E402
import ricci_su2_pg_force as F          # noqa: E402
from ricci_consistency_math import compose  # noqa: E402

A_VALUES = (3.0, 10.0, 30.0, 100.0)
LEVELS = (0, 1, 2)                       # n = 1, 2, 3
ONE = np.array([1.0, 0, 0, 0])
SERIES = {"chord": (-1 / 12, 2), "tangent": (1 / 6, 2), "blend": (7 / 720, 4), "adjoint": (-1 / 90, 4)}


def continuum_density(g):
    A, B, C = g
    x = np.array([(B + C - A) / np.sqrt(B * C), (C + A - B) / np.sqrt(C * A), (A + B - C) / np.sqrt(A * B)])
    f = np.array([x[1] * x[2] - 2 * x[0], x[2] * x[0] - 2 * x[1], x[0] * x[1] - 2 * x[2]])
    return 0.5 * (f[0] ** 2 / (B * C) + f[1] ** 2 / (C * A) + f[2] ** 2 / (A * B))


def readings(U):
    """U: 4 x 4 registered face holonomies of one cell -> dict of 4 x 4 readings and angles."""
    q0, v = U[:, 0], U[:, 1:]
    s = np.linalg.norm(v, axis=1)
    a = np.arctan2(s, q0)
    n = np.where(s[:, None] > 1e-300, v / np.where(s > 1e-300, s, 1)[:, None], 0.0)
    zero = np.zeros((4, 1))
    U2 = np.array([compose(u, u) for u in U])
    return {"chord": U - ONE,
            "arc": np.hstack([zero, a[:, None] * n]),
            "tangent": np.hstack([zero, 2 * v / (1 + q0)[:, None]]),
            "chord2": U2 - ONE}, a


def energies(lat, links, mp):
    E = {k: 0.0 for k in ("chord", "arc", "tangent", "chord2")}
    pred_num = {k: 0.0 for k in SERIES}
    pred_den = 0.0
    angles, tan_face_max = [], 0.0
    for ci, c in enumerate(lat.cells):
        U = F.defects(F.local_links(c, links)) + ONE
        R, a = readings(U)
        M = mp["m2"][ci]
        for k in E:
            E[k] += c["weight"] * 2 * float(np.einsum("ij,ik,jk->", M, R[k], R[k]))
        diag = c["weight"] * 2 * np.diag(M) * a ** 2           # per-face weights for the series
        pred_den += diag.sum()
        for k, (coef, p) in SERIES.items():
            pred_num[k] += float(np.sum(diag * coef * a ** p))
        tan_face_max = max(tan_face_max, float((c["weight"] * 2 * np.diag(M) * np.sum(R["tangent"] ** 2, axis=1)).max()))
        angles.append(a)
    E["blend"] = (2 * E["chord"] + E["tangent"]) / 3
    E["adjoint"] = (4 / 3) * E["chord"] - (1 / 12) * E["chord2"]
    del E["chord2"]
    pred = {k: pred_num[k] / pred_den for k in SERIES}
    return E, pred, np.concatenate(angles), tan_face_max


def main():
    receipt = Path(sys.argv[1]) if len(sys.argv) > 1 else None
    if receipt and receipt.exists():
        raise SystemExit(f"refusing to overwrite {receipt}")
    t0 = time.time()
    rows, controls = [], {}
    for level in LEVELS:
        lat = FL.Lattice(level, "native")
        if level in (0, 1):                                   # C-native: the engine's own links and energy
            g0 = np.array([3.0, 1.0, 1.0]); mp0 = lat.metric_pieces(g0)
            raw = np.asarray(lat.raw["links"], float)
            E0, _, _, _ = energies(lat, raw, mp0)
            controls[f"C_native_n{(1, 2, 3)[level]}"] = abs(E0["chord"] - lat.raw["energy"]) / abs(lat.raw["energy"])
            controls[f"exact_vs_stored_links_n{(1, 2, 3)[level]}"] = float(np.abs(lat.exact_links(g0) - raw).max())
        for Aval in A_VALUES:
            g = np.array([Aval, 1.0, 1.0]); mp = lat.metric_pieces(g); L = lat.exact_links(g)
            E, pred, a, tmax = energies(lat, L, mp)
            Ec = continuum_density(g) * mp["vol"]
            delta = {k: E[k] / E["arc"] - 1 for k in E if k != "arc"}
            row = {"n": (1, 2, 3)[level], "A": Aval, "E_cont": Ec,
                   "E_over_cont": {k: E[k] / Ec for k in E},
                   "delta_vs_arc": delta, "series_prediction": pred,
                   "e_disc": E["arc"] / Ec - 1,
                   "angle_median": float(np.median(a)), "angle_max": float(a.max()),
                   "faces_above_1": int((a > 1).sum()), "faces_above_2": int((a > 2).sum()), "faces": int(len(a)),
                   "largest_single_face_tangent_share": tmax / E["tangent"]}
            rows.append(row)
            print(json.dumps({k: (round(v, 6) if isinstance(v, float) else v) for k, v in row.items()
                              if k not in ("E_over_cont", "series_prediction")}), flush=True)
    # predictions
    verdict = {"P_cancel": [], "P_series": [], "P_regime": {}}
    for r in rows:
        d = r["delta_vs_arc"]
        if r["angle_max"] <= 1.5:
            verdict["P_cancel"].append({"n": r["n"], "A": r["A"],
                                        "pass": bool(abs(d["blend"]) <= abs(d["chord"]) / 5 and abs(d["blend"]) <= abs(d["tangent"]) / 5)})
        if r["angle_max"] <= 0.6:
            ok = all(abs(d[k] - r["series_prediction"][k]) <= max(0.25 * abs(r["series_prediction"][k]), 1e-6) for k in SERIES)
            verdict["P_series"].append({"n": r["n"], "A": r["A"], "pass": bool(ok),
                                        "measured": d, "predicted": r["series_prediction"]})
    n1 = {r["A"]: r for r in rows if r["n"] == 1}
    verdict["P_regime"] = {
        "A>=30_compactness_exceeds_disc": bool(all(abs(n1[A]["delta_vs_arc"]["chord"]) > abs(n1[A]["e_disc"]) for A in (30.0, 100.0))),
        "A=3_compactness_below_disc": bool(abs(n1[3.0]["delta_vs_arc"]["chord"]) < abs(n1[3.0]["e_disc"]))}
    small = [r for r in rows if r["n"] == 2 and r["A"] == 3.0][0]
    controls["C_arc_small_max_spread"] = max(abs(v) for v in small["delta_vs_arc"].values())
    out = {"kind": "S4 research script (not an engine run)", "controls": controls, "rows": rows,
           "verdict": verdict, "seconds": round(time.time() - t0, 1)}
    print(json.dumps({"controls": controls, "verdict": {k: v if k == "P_regime" else [(x["n"], x["A"], x["pass"]) for x in v] for k, v in verdict.items()}}), flush=True)
    if receipt:
        receipt.parent.mkdir(parents=True, exist_ok=True)
        receipt.write_text(json.dumps(out, indent=1) + "\n")


if __name__ == "__main__":
    main()
