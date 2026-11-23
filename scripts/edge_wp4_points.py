"""WP4 prereg A: the held-out family points, drawn once and written before any edge is computed at them.

No spectrum, edge or polynomial is evaluated here. The only check is that no point is within 1e-3 (in t and in y) of a
(t, y) pair used in an earlier computation (exact_edge.json, twist_anatomy.json, edge tests, the map's stretches).
usage: python scripts/edge_wp4_points.py research/edge-theorem-20261121/wp4_prereg_A_points.json
"""
import json
import sys
from pathlib import Path

import numpy as np

R = Path(__file__).resolve().parent.parent / "research/su2-floor-family-20261119"
used = set()
for r in json.loads((R / "exact_edge.json").read_text())["edges"]:
    used.add((r["t"], r["y"]))
an = json.loads((R / "twist_anatomy.json").read_text())["part2_exploratory"]
for r in an["Q3_rows"] + an["Q4_Q5_edges"]:
    used.add((float(r["t"]), float(r["y"])))
used_t = {1.0, 2.0, 2.5, 3.0, 4.0, 5.0, 6.0, 8.0, 10.0, 12.0}          # map + Test 6 + exact-edge stretches

rng = np.random.default_rng(20261121)
cls_P2 = [(float(t), float(y)) for t, y in zip(rng.uniform(2.2, 11.5, 30), rng.uniform(1.1, 3.2, 30))]
cls_small = [(float(t), float(y)) for t, y in zip(rng.uniform(2.2, 11.5, 10), rng.uniform(0.3, 0.7, 10))]
grid = [(t, y) for t in (2.2, 3.5, 7.0, 11.0) for y in (1.2, 2.5)]
pts = {"P2_class_random": cls_P2, "small_y_class_random": cls_small, "fixed_grid": grid}
for name, L in pts.items():
    for (t, y) in L:
        assert all(abs(t - a) > 1e-3 for a in used_t), (name, t)
        assert all(not (abs(t - a) < 1e-3 and abs(y - b) < 1e-3) for a, b in used), (name, t, y)
Path(sys.argv[1]).write_text(json.dumps({"seed": 20261121, "points_(t,y)": pts,
                                         "exclusion_checked_against": len(used)}, indent=1) + "\n")
print({k: len(v) for k, v in pts.items()}, "excluded pairs:", len(used))
