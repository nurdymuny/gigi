"""S3b gates at n = 3 (level 2), judged from the saved receipts and final states.
usage (from the repo root): python research/su2-cayley-20261118/gates_L2.py > research/su2-cayley-20261118/gates_L2.json

Observed order between n = 2 and n = 3: log(E_2/E_3) / log(3/2). The mesh size goes as 1/n
(links: 142, 1124, 3786 at n = 1, 2, 3; 3786/1124 = 3.37 = 1.5^3), so the n = 1 -> 2 step halves
it (S3's log2) and the n = 2 -> 3 step divides it by 3/2. log2 of the same ratios is also reported.
"""
import json
import math
import sys
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[1]
sys.path.insert(0, str(REPO / "scripts"))
import ricci_su2_pg_flow as FL  # noqa: E402

FLOW = REPO / "research" / "su2-flow-20261118"


def rec(name):
    return json.loads((HERE / name).read_text())


def e_final(d):
    return d["rows"][-1]["E"]


def order(e2, e3):
    r = e2 / e3
    return {"ratio": r, "order_mesh_3_2": math.log(r) / math.log(1.5), "log2": math.log2(r)}


g1 = rec("gates_L1.json")["forces"]
E2 = {k: next(r["E_final"] for r in g1[k]["runs"] if r["N"] == 8) for k in ("native", "pg")}
E2_rk4 = {k: e_final(json.loads((FLOW / f"L1_{k}_N64_q6.json").read_text()))
          for k in ("native", "pg", "pg_fixed")}
L2 = {f"{k}_N{N}": rec(f"L2_{k}_N{N}.json") for k in ("native", "pg", "pg_fixed") for N in (8, 16)}
E3 = {k: L2[k]["E_final"] for k in L2}

lat = FL.Lattice(2, "pg")
gT = FL.Metric()(FL.T_END)
delta = {N: lat.error(np.load(HERE / f"L2_pg_fixed_N{N}.final.npy"),
                      np.load(HERE / f"L2_pg_N{N}.final.npy"), gT) for N in (8, 16)}

o_pg = order(E2["pg"], E3["pg_N8"])
o_nat = order(E2["native"], E3["native_N8"])
o_fix = order(E2_rk4["pg_fixed"], E3["pg_fixed_N8"])
Em, Ef, D = E3["pg_N8"], E3["pg_fixed_N8"], delta[8]
out = {
    "level": 2, "N": 8,
    "runs": {k: {x: v[x] for x in ("status", "R_final", "E_final", "newton_max_iterations",
                                   "newton_worst_final_increment", "force_evaluations",
                                   "seconds_per_evaluation", "seconds")} for k, v in L2.items()},
    "E_n2": {"cayley_N8": E2, "rk4_N64": E2_rk4},
    "G_traj": {"R": L2["pg_N8"]["R_final"], "order": o_pg,
               "pass": L2["pg_N8"]["R_final"] < 0.05 and o_pg["order_mesh_3_2"] >= 0.5},
    "G_time": {"change": abs(E3["pg_N8"] - E3["pg_N16"]) / E3["pg_N16"],
               "pass": abs(E3["pg_N8"] - E3["pg_N16"]) < E3["pg_N16"] / 10},
    "frozen": {"delta_N8": delta[8], "delta_N16": delta[16], "two_E_m": 2 * Em,
               "informative": D > 2 * Em, "E_f": Ef, "E_f_ge_delta_over_2": Ef >= D / 2,
               "E_f_order_vs_rk4_n2": o_fix,
               "E_f_not_converging_while_E_m_does":
                   o_fix["order_mesh_3_2"] < 0.25 and o_pg["order_mesh_3_2"] >= 0.25},
    "native": {"crossover_E_pg_lt_E_native": Em < E3["native_N8"],
               "E_pg_over_E_native": Em / E3["native_N8"], "order": o_nat,
               "stall": o_nat["order_mesh_3_2"] < 0.25},
    "S3a_forecast": {"both_falling": o_pg["ratio"] > 1 and o_nat["ratio"] > 1,
                     "pg_order_exceeds_native": o_pg["order_mesh_3_2"] > o_nat["order_mesh_3_2"]},
}
print(json.dumps(out, indent=1))
