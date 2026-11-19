"""S3b gates G-agree and G-time at one level, from the Cayley receipts and S3's RKMK4 receipts
(docs/RICCI_SU2_PG_FLOW_CAYLEY_SPEC.md; thresholds as written there).

  G-agree  |E_cay(t) - E_rk4(t)| < E_rk4(t)/20 at all four sample times
  G-time   halving h changes E_cay(0.02) by less than E/10   (needs the 2N receipt)
  N        the smallest of 4, 8, 16, 32 passing both gates for both forces

usage (from the repo root): python research/su2-cayley-20261118/gates.py LEVEL
"""
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
RK4 = HERE.parents[0] / "su2-flow-20261118"
REF = {0: {"native": "L0_native_N32_q6.json", "pg": "L0_pg_N32_q6.json"},
       1: {"native": "L1_native_N64_q6.json", "pg": "L1_pg_N64_q6.json"}}

level = int(sys.argv[1])
verdict = {"level": level, "forces": {}}
passing = {}
for trial in ("native", "pg"):
    ref = {round(r["t"], 12): r["E"] for r in json.loads((RK4 / REF[level][trial]).read_text())["rows"]}
    runs = {}
    for N in (4, 8, 16, 32, 64):
        p = HERE / f"L{level}_{trial}_N{N}.json"
        if p.exists():
            runs[N] = json.loads(p.read_text())
    rows = []
    for N, r in sorted(runs.items()):
        row = {"N": N, "status": r["status"], "newton_max_iterations": r["newton_max_iterations"],
               "force_evaluations": r["force_evaluations"]}
        if r["status"] == "ok":
            gaps = [abs(x["E"] - ref[round(x["t"], 12)]) / ref[round(x["t"], 12)] for x in r["rows"]]
            row["E_final"] = r["E_final"]
            row["G_agree_worst_relative_gap"] = max(gaps)
            row["G_agree"] = max(gaps) < 1 / 20
            if 2 * N in runs and runs[2 * N]["status"] == "ok":
                ch = abs(r["E_final"] - runs[2 * N]["E_final"]) / r["E_final"]
                row["G_time_relative_change"] = ch
                row["G_time"] = ch < 1 / 10
        rows.append(row)
    verdict["forces"][trial] = {"rk4_reference": REF[level][trial], "runs": rows}
    passing[trial] = {x["N"] for x in rows if x.get("G_agree") and x.get("G_time")}
both = sorted(passing["native"] & passing["pg"] & {4, 8, 16, 32})
verdict["N_passing_both_forces"] = both
verdict["N_chosen_at_this_level"] = both[0] if both else None
print(json.dumps(verdict, indent=1))
