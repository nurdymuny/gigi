"""S3b diagnostic: how far the SU(2) force moves when the links are nudged at rounding level.
usage (from the repo root): python research/su2-cayley-20261118/diagnostics/force_noise.py LEVEL TRIAL N"""
import sys
from pathlib import Path
import numpy as np
sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "scripts"))
import ricci_su2_pg_flow as FL

level, trial, N = int(sys.argv[1]), sys.argv[2], int(sys.argv[3])
lat = FL.Lattice(level, trial)
met = FL.Metric()
h = FL.T_END / N
U = np.asarray(lat.raw["links"], float)
g = met(0.5 * h)
mp = lat.metric_pieces(g)
v0 = lat.velocity(U, g)
G0 = lat.gradient(U, mp)
rng = np.random.default_rng(0)
for k in range(3):
    Up = U * (1 + 2.2e-16 * rng.standard_normal(U.shape))
    Up /= np.linalg.norm(Up, axis=1, keepdims=True)
    dv = np.linalg.norm(lat.velocity(Up, g) - v0) / np.linalg.norm(v0)
    dg = np.linalg.norm(lat.gradient(Up, mp) - G0) / np.linalg.norm(G0)
    print(f"rounding-level nudge {k}: velocity moves {dv:.1e}, gradient moves {dg:.1e}")
cells = lat.cells
print("|grad| =", float(np.linalg.norm(G0)), " h|xi| per link (rms) =", float(h * np.sqrt(np.mean(np.sum(v0**2, axis=1)))))
