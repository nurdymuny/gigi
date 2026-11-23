"""Receipt for the Helike book, section 4.2: "Whether the upper edge equals the smaller of the j <= 12 edge and zero
exactly is a conjecture for t >= 3. Near t = 2.01 with a weak cross field y = 0.25 it is false."

The conjecture (edge paper, conj:edge): for t >= 3 and y > 0 the all-sector edge equals min(x*_12(t, y), 0).
Its evidence (edge paper, subsec:allsector) is the tail check research/edge-theorem-20261121/wp1_tail_check.json:
at each (t, y) of the scan, the lowest state of families 13..200 at the j <= 12 edge x*_12. A tail family that
undercuts her at x*_12 contradicts the conjecture only where x*_12 < 0; where x*_12 > 0 the edge is min(x*_12, 0) = 0
anyway (for x > 0 she loses: the edge paper's theorem thm:xpos). This script classifies every scanned point.
usage (repository root): python scripts/book_receipt_conjecture_scope.py research/book-receipts-20261122/conjecture_scope.json
"""
import json
import sys
from pathlib import Path

d = json.loads(Path("research/edge-theorem-20261121/wp1_tail_check.json").read_text())
rows = d["rows"]
classified = []
for r in rows:
    t, y, xe, tail = r["t"], r["y"], r["edge_x_j<=12"], r["tail_min_13..200_at_edge"]
    undercut = tail < 0
    if not undercut:
        kind = "no tail undercut"
    elif xe > 0:
        kind = "tail undercut, but x*_12 > 0: consistent (edge is 0)"
    else:
        kind = "tail undercut at x*_12 < 0: CONTRADICTS min(x*_12, 0)"
    classified.append(dict(t=t, y=y, x_edge_12=xe, tail_min=tail, tail_argmin=r.get("tail_argmin"), kind=kind))

def summary(sel):
    out = {}
    for c in sel:
        out[c["kind"]] = out.get(c["kind"], 0) + 1
    return out

ge3 = [c for c in classified if c["t"] >= 3]
lt3 = [c for c in classified if c["t"] < 3]
contra_ge3 = [c for c in ge3 if c["kind"].startswith("tail undercut at")]
contra_lt3 = [c for c in lt3 if c["kind"].startswith("tail undercut at")]
res = dict(source="research/edge-theorem-20261121/wp1_tail_check.json", families_checked="13..%d" % d["JT"],
           t_range=[min(c["t"] for c in classified), max(c["t"] for c in classified)],
           y_range=[min(c["y"] for c in classified), max(c["y"] for c in classified)],
           points=len(classified), t_ge_3=summary(ge3), t_lt_3=summary(lt3),
           contradictions_t_ge_3=contra_ge3, contradictions_t_lt_3=contra_lt3,
           t2_01_counterexample=("edge paper subsec:allsector: at (t, y) = (2.01, 0.25) the block q = 8 of j = 16 lies "
                                 "0.00492 below her at x = -0.624598 < x*_12 = -0.62410 "
                                 "(research/edge-theorem-20261121/workflow_reports.json)"),
           rows=classified)
print(f"{len(classified)} scanned points, t in {res['t_range']}, y in {res['y_range']}, tail families {res['families_checked']}")
print("t >= 3:", res["t_ge_3"])
print("t <  3:", res["t_lt_3"])
print(f"contradictions at t >= 3: {len(contra_ge3)}; at t < 3: {len(contra_lt3)} "
      f"{[(c['t'], c['y']) for c in contra_lt3]}")
Path(sys.argv[1]).parent.mkdir(parents=True, exist_ok=True)
Path(sys.argv[1]).write_text(json.dumps(res, indent=1))
