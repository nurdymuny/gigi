"""Face holonomy angles on the frozen SU(2) Berger meshes (the paper's Table 2).

For each face, U_f is the registered face holonomy (native definition, as in
scripts/ricci_su2_pg_force.py), written U_f = cos a + n sin a; a is the geodesic
angle on S^3. Reports the median and maximum of a, and the chord-energy gap a^2/12
at the maximum.

usage: GIGI_RICCI_DATA_ROOT=<gigi checkout with output/> python docs/papers/chord-arc-tangent/face_angles.py
"""
import json
import sys
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[2] / "scripts"))
import ricci_su2_pg_force as F    # noqa: E402

rows = []
for level, n in zip(range(4), (1, 2, 3, 5)):
    payload = json.loads((F.BERGER / f"level{level}_berger_q6.input.json").read_text())
    raw = json.loads((F.BERGER / f"level{level}_berger_q6.output.json").read_text())
    links = np.asarray(raw["links"], float)
    a = np.array([np.arctan2(np.linalg.norm(q[1:]), q[0])
                  for c in payload["cells"]
                  for q in F.defects(F.local_links(c, links)) + F.ONE])
    rows.append({"n": n, "faces": int(len(a)), "median_a": float(np.median(a)), "max_a": float(a.max()),
                 "chord_energy_gap_at_max": float(a.max() ** 2 / 12)})
    print(json.dumps(rows[-1]))
(HERE / "face_angles.json").write_text(json.dumps(rows, indent=1) + "\n")
