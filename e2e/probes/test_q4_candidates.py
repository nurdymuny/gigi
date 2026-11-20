"""Q4 candidates (docs/HALCYON_Q4_RUN_SPEC.md), tested on the Q3 receipt. Committed before any Q3
correlator was computed. Exploratory: a surviving candidate is a conjecture, nothing more.

usage: python test_q4_candidates.py Q3_RECEIPT.json OUT.json
"""
import json
import sys
from itertools import combinations

import numpy as np
from scipy.special import iv

rec = json.load(open(sys.argv[1]))
FLOOR = 3.158
LTW_BETA = {2.2986: (0, 1), 2.3715: (2, 3), 2.4265: (4, 5)}

pts = []
for beta, (kc, _kv) in LTW_BETA.items():
    if rec["gates"].get(f"Q3-a_beta{beta}") and rec["gates"].get(f"Q3-v_beta{beta}"):
        r = rec["rows"][str(kc)]
        if "ratio" in r:
            pts.append({"beta": beta, "L": r["L"], "a2sigma": r["a2sigma"], "ratio": r["ratio"],
                        "ratio_err": r["ratio_err"], "am0": r["scalar"]["plateau"]["mass"],
                        "am0_err": r["scalar"]["plateau"]["error"]})

c1_fail = [(p["beta"], p["L"], q["beta"], q["L"]) for p, q in combinations(pts, 2)
           for (hi, lo) in [((p, q) if p["a2sigma"] > q["a2sigma"] else (q, p))]
           if hi["ratio"] - lo["ratio"] > 2 * np.hypot(hi["ratio_err"], lo["ratio_err"])]
c2_fail = [(p["beta"], p["L"]) for p in pts if p["ratio"] < FLOOR - 2 * p["ratio_err"]]
k_hold = [(p["beta"], p["L"]) for p in pts
          if p["am0"] + 2 * p["am0_err"] >= -4 * np.log(iv(2, p["beta"]) / iv(1, p["beta"]))]

out = {"kind": "Q4 candidates (exploratory)", "points_tested": pts,
       "C1_monotone": {"survives": not c1_fail, "violations": c1_fail, "n_points": len(pts)},
       "C2_floor_3.158": {"survives": not c2_fail, "fails_at": c2_fail},
       "K_control_strong_coupling": {"holds_at": k_hold,
                                     "stop": bool(k_hold),
                                     "note": "expected to hold nowhere; if it holds, stop and find why"}}
json.dump(out, open(sys.argv[2], "w"), indent=1)
print(json.dumps(out, indent=1))
