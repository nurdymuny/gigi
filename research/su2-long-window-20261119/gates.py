"""S3c verdicts (docs/RICCI_SU2_PG_FLOW_LONG_WINDOW_SPEC.md), from the receipts in this folder and
S3b's N = 8 receipts. Thresholds as written in the spec.

  C-prefix  E(0.02) at N = 40 matches S3b's N = 8 to 1e-6 relative
  G-time    halving h (N = 40 -> 80) changes E by < E/10 at every sample time
  P-long    p_native(0.1) < 1.63 - 0.25;  p_two-ring(0.1) >= 2.94 - 0.25   (p = log2 E(n=1)/E(n=2))
  P-cross   at n = 2, E_two-ring(t) < E_native(t) at every sample time

usage (from the repo root): python research/su2-long-window-20261119/gates.py
"""
import json
from math import log2
from pathlib import Path

HERE = Path(__file__).resolve().parent
S3B = HERE.parent / "su2-cayley-20261118"
P02 = {"native": 1.63, "pg": 2.94}


def load(p):
    return json.loads(p.read_text()) if p.exists() else None


out = {"runs": {}, "C_prefix": {}, "G_time": {}, "p": {}, "P_long": {}, "P_cross": None}
R = {}
for lv in (0, 1):
    for t in ("native", "pg"):
        for N in (40, 80):
            r = load(HERE / f"L{lv}_{t}_N{N}.json")
            R[(lv, t, N)] = r
            if r:
                out["runs"][f"L{lv}_{t}_N{N}"] = {k: r[k] for k in ("status", "E_final", "R_final", "newton_max_iterations",
                                                                     "mass_refreshes", "force_evaluations")}
        r40, r80, s3b = R[(lv, t, 40)], R[(lv, t, 80)], load(S3B / f"L{lv}_{t}_N8.json")
        key = f"L{lv}_{t}"
        if r40 and r40["rows"] and s3b:
            e = r40["rows"][0]["E"]
            gap = abs(e - s3b["E_final"]) / s3b["E_final"]
            out["C_prefix"][key] = {"E_0.02": e, "S3b_N8": s3b["E_final"], "relative": gap, "pass": gap < 1e-6}
        if r40 and r80 and r40["status"] == "ok" and r80["status"] == "ok":
            ch = max(abs(a["E"] - b["E"]) / a["E"] for a, b in zip(r40["rows"], r80["rows"]))
            out["G_time"][key] = {"worst_relative_change": ch, "pass": ch < 0.1}
for t in ("native", "pg"):
    a, b = R[(0, t, 40)], R[(1, t, 40)]
    if a and b and a["status"] == "ok" and b["status"] == "ok":
        out["p"][t] = {f"{x['t']:.2f}": log2(x["E"] / y["E"]) for x, y in zip(a["rows"], b["rows"])}
if "native" in out["p"]:
    pn = out["p"]["native"]["0.10"]
    out["P_long"]["native"] = {"p_0.1": pn, "p_0.02_S3b": P02["native"], "pass": pn < P02["native"] - 0.25}
if "pg" in out["p"]:
    pp = out["p"]["pg"]["0.10"]
    out["P_long"]["pg"] = {"p_0.1": pp, "p_0.02_S3b": P02["pg"], "pass": pp >= P02["pg"] - 0.25}
nat, pg = R[(1, "native", 40)], R[(1, "pg", 40)]
if nat and pg and nat["status"] == "ok" and pg["status"] == "ok":
    pairs = [(x["t"], y["E"], x["E"]) for x, y in zip(nat["rows"], pg["rows"])]
    out["P_cross"] = {"by_time": [{"t": t, "E_two_ring": e2, "E_native": en} for t, e2, en in pairs],
                      "pass": all(e2 < en for _, e2, en in pairs)}
print(json.dumps(out, indent=1))
