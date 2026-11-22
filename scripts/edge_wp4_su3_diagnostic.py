"""POST-HOC diagnostic for prereg C (not part of the registered protocol; run after its verdict was known):
at t = 4 and the four (u, y) shapes, per-sector margin mu_min(non-b) - lambda_b on a 0.25 grid over [r_lo, r_hi],
the best x (largest all-sector margin), and the sector that blocks her there.
usage: OMP_NUM_THREADS=1 python scripts/edge_wp4_su3_diagnostic.py OUT.json"""
import json, sys
from pathlib import Path
import numpy as np
sys.path.insert(0, str(Path(__file__).resolve().parent))
import edge_wp4_su3_free_fibre as C
g = C.Group("su3"); res = []
st_all = json.loads((Path(__file__).resolve().parent.parent / "research/edge-theorem-20261121/wp4_su3_free_fibre.json").read_text())["structure"]
for (u, y) in ((0.5, 0.5), (0.5, 1.0), (1.0, 1.0), (1.0, 2.0)):
    t = 4.0; st = st_all[str((t, u, y))]
    nmul = {s: (s[0] + 3) * (2 if s[0] == s[1] else 1) for s in C.SECTORS}
    rows = []
    for x in np.arange(st["r_lo"], st["r_hi"] + 1e-9, 0.25):
        m = {}
        for s in C.SECTORS:
            e = C.evaluate(g, x, (u, y), t, s)
            m[str(s)] = float(C.drop(e["mu"], e["lam"], nmul[s])[0] - e["lam"])
        rows.append({"x": float(x), "margins": m, "min": min(m.values()), "blocker": min(m, key=m.get)})
        print(u, y, round(x, 3), round(rows[-1]["min"], 4), rows[-1]["blocker"], flush=True)
    best = max(rows, key=lambda r: r["min"])
    res.append({"t": t, "u": u, "y": y, "best": best, "scan": rows})
Path(sys.argv[1]).write_text(json.dumps(res, indent=1) + "\n")
for r in res:
    print(r["u"], r["y"], "best x", r["best"]["x"], "min margin", r["best"]["min"], "blocker", r["best"]["blocker"], r["best"]["margins"])
