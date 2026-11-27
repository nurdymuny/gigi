"""M3 at the background from the deep (jmax 200) spectra: curvature of the fast zero-point energy along Helike's and the
competitor's modes, with its spread over heat-trace windows, power bases and finite-difference steps reported as the
uncertainty (no single fit is chosen). Check at every combination: the curvature of the log coefficient against its
prediction (b0/8pi^2) Vol Hess(density) (the running coupling, K5 at the background).
usage: python scripts/helike_m3_deep_analysis.py RESEARCH_DIR OUT.json"""
import copy
import gzip
import glob
import json
import math
import sys
from pathlib import Path

import numpy as np

import helike_m3_zeta as z

B0 = 22.0 / 3.0
WINDOWS = [(0.003, 0.02), (0.003, 0.03), (0.004, 0.03), (0.005, 0.04)]
BASES = [[0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0], [0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5]]
G2 = [0.002, 0.004]


def main(d, out):
    d = Path(d)
    meta = json.loads((d / "deep_meta.json").read_text())
    f = [p for p in glob.glob(str(d / "zp_t3.0_j200_*.json*")) if "fields" not in p][0]
    P = json.loads((gzip.open(f, "rt") if f.endswith(".gz") else open(f)).read())["results"]["points"]
    t = 3.0
    vol = 2 * math.pi ** 2 * math.sqrt(t)
    omega = meta["omega"]
    centre = P[0]
    # truncation check: cap at 180
    def cap(p, jc):
        p = copy.deepcopy(p); p["sectors"] = [x for x in p["sectors"] if x["j"] <= jc]; return p
    rep = {"t": t, "s": 1.15, "jmax": 200, "windows": WINDOWS, "bases": BASES, "modes": []}
    for name, k in (("helike", 3), ("competitor", 5)):
        rows = []
        for h in (0.05, 0.1):
            ip = [i for i, m in enumerate(meta["meta"]) if m[0] == name and abs(m[2] - h) < 1e-12][0]
            im = [i for i, m in enumerate(meta["meta"]) if m[0] == name and abs(m[2] + h) < 1e-12][0]
            dens = (P[ip]["energy_density"] + P[im]["energy_density"] - 2 * centre["energy_density"]) / h ** 2
            for W in WINDOWS:
                for B in BASES:
                    z.TAU_FIT = np.geomspace(W[0], W[1], 48); z.POWERS = B
                    rp, rm = z.zeta_fast(centre, P[ip], tau0=W[1]), z.zeta_fast(centre, P[im], tau0=W[1])
                    rows.append({"h": h, "window": W, "basis_top": B[-1], "H": (rp["dE0_fast"] + rm["dE0_fast"]) / h ** 2,
                                 "H_C": (rp["dC"] + rm["dC"]) / h ** 2, "H_C_pred": B0 / (8 * math.pi ** 2) * vol * dens,
                                 "fit_resid": max(rp["fit_resid"], rm["fit_resid"])})
            # truncation at the first window/basis
            z.TAU_FIT = np.geomspace(*WINDOWS[0], 48); z.POWERS = BASES[0]
            rp2, rm2 = z.zeta_fast(cap(centre, 180), cap(P[ip], 180), tau0=WINDOWS[0][1]), z.zeta_fast(cap(centre, 180), cap(P[im], 180), tau0=WINDOWS[0][1])
            rows[-len(WINDOWS) * len(BASES)]["H_jmax180"] = (rp2["dE0_fast"] + rm2["dE0_fast"]) / h ** 2
        Hs = np.array([r["H"] for r in rows])
        HC = np.array([r["H_C"] / r["H_C_pred"] for r in rows])
        m = {"name": name, "index": k, "mult": 2 if name == "helike" else 1, "omega": omega[k], "rows": rows,
             "H_median": float(np.median(Hs)), "H_min": float(Hs.min()), "H_max": float(Hs.max()),
             "H_C_over_pred_range": [float(HC.min()), float(HC.max())],
             "H_C_pred": rows[0]["H_C_pred"], "per_g2": {}}
        for g2 in G2:
            m["per_g2"][str(g2)] = {"omega": omega[k], "d_omega_zeta": g2 * m["H_median"] / omega[k],
                                    "d_omega_zeta_range": [g2 * m["H_min"] / omega[k], g2 * m["H_max"] / omega[k]],
                                    "d_omega_per_kappa": g2 * m["H_C_pred"] / omega[k]}
        m["H_zeta"] = m["H_median"]; m["H_C"] = m["H_C_pred"]
        rep["modes"].append(m)
        print(json.dumps({x: m[x] for x in ("name", "omega", "H_median", "H_min", "H_max", "H_C_over_pred_range", "H_C_pred")}), flush=True)
        for r in rows:
            print("   ", json.dumps(r), flush=True)
    hel, comp = rep["modes"]
    for g2 in G2:
        a, b = hel["per_g2"][str(g2)], comp["per_g2"][str(g2)]
        sep0 = comp["omega"] - hel["omega"]
        dsep = b["d_omega_zeta"] - a["d_omega_zeta"]
        lo = b["d_omega_zeta_range"][0] - a["d_omega_zeta_range"][1]
        hi = b["d_omega_zeta_range"][1] - a["d_omega_zeta_range"][0]
        dk = b["d_omega_per_kappa"] - a["d_omega_per_kappa"]
        rep[f"g2_{g2}"] = {"harmonic_separation": sep0, "fast_shift_zeta": dsep, "fast_shift_range": [lo, hi],
                           "separation_with_fibre": sep0 + dsep, "separation_range": [sep0 + lo, sep0 + hi],
                           "shift_per_unit_kappa": dk}
        print(g2, json.dumps(rep[f"g2_{g2}"]), flush=True)
    Path(out).write_text(json.dumps(rep, indent=1) + "\n")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
