"""Receipt for the Helike book, section 4.2: "Computed in double precision, her real margin on D is about 2.35 or more."
Source of the claim: the edge paper, Remark "D lies inside the region" (docs/papers/by-enough/paper.tex, rem:Dinside):
min phi_A at x in {-3, -1.5} over an 11 x 15 grid of D's (y, t) range is 2.35 (j <= 12), and min phi_S on a 3 x 3 grid
is 2.69. phi_A and phi_S are the edge script's own functions (scripts/ricci_su2_exact_edge.py): the lowest other state
minus her price, over families j <= 12, for H + GG* (A) and for the gauge-orthogonal spectrum (S).
D = {-3 <= x <= -1.5, 0.5 <= y <= 3, 3 <= t <= 10}.
usage (repository root): python scripts/book_receipt_margin_on_D.py research/book-receipts-20261122/margin_on_D.json
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
_a, sys.argv = sys.argv, [sys.argv[0]]
import ricci_su2_exact_edge as EE  # noqa: E402
sys.argv = _a

blocks = EE.Blocks()
ys, ts = np.linspace(0.5, 3.0, 11), np.linspace(3.0, 10.0, 15)
A = []
for xv in (-3.0, -1.5):
    for yv in ys:
        for tv in ts:
            v, arg = blocks.phi_A(xv, yv, tv, with_arg=True)
            A.append(dict(x=xv, y=float(yv), t=float(tv), phi_A=float(v), rival=arg))
ys3, ts3 = np.linspace(0.5, 3.0, 3), np.linspace(3.0, 10.0, 3)
S = []
for xv in (-3.0, -1.5):
    for yv in ys3:
        for tv in ts3:
            v, arg = EE.phi_S(xv, yv, tv, with_arg=True)
            S.append(dict(x=xv, y=float(yv), t=float(tv), phi_S=float(v), sector=arg))
mA = min(A, key=lambda r: r["phi_A"]); mS = min(S, key=lambda r: r["phi_S"])
res = dict(grid_A="x in {-3,-1.5}; y 11 points in [0.5,3]; t 15 points in [3,10]; j <= 12",
           grid_S="x in {-3,-1.5}; y 3 points; t 3 points; j <= 12",
           min_phi_A=mA, min_phi_S=mS, points_A=len(A), points_S=len(S), rows_A=A, rows_S=S)
print(f"min phi_A over {len(A)} points: {mA['phi_A']:.4f} at x={mA['x']}, y={mA['y']:.3f}, t={mA['t']:.3f} (rival {mA['rival']})")
print(f"min phi_S over {len(S)} points: {mS['phi_S']:.4f} at x={mS['x']}, y={mS['y']:.3f}, t={mS['t']:.3f} (sector {mS['sector']})")
Path(sys.argv[1]).parent.mkdir(parents=True, exist_ok=True)
Path(sys.argv[1]).write_text(json.dumps(res, indent=1, default=float))
