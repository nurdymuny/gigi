"""S3b diagnostic: Newton increments at the first Cayley step, iterative vs direct mass solve.
usage (from the repo root): python research/su2-cayley-20261118/diagnostics/newton_floor.py LEVEL TRIAL N"""
import sys
from pathlib import Path
import numpy as np
from scipy.sparse.linalg import splu
sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "scripts"))
import ricci_su2_pg_flow_cayley as C
import ricci_su2_pg_flow as FL
from ricci_su2_pg_flow import compose

level, trial, N = int(sys.argv[1]), sys.argv[2], int(sys.argv[3])
lat = FL.Lattice(level, trial)
met = FL.Metric()
h = FL.T_END / N
U = np.asarray(lat.raw["links"], float)
B0 = lat.mass(U, lat.metric_pieces(met(0.0))).tocsc()
A = splu((B0 + 0.5 * h * C.load_K(level, trial)).tocsc())


def vel_direct(links, g):
    mp = lat.metric_pieces(g)
    B = lat.mass(links, mp).tocsc()
    return splu(B).solve(-lat.gradient(links, mp).ravel()).reshape(lat.n, 3)


g = met(0.5 * h)
v_cg, v_dir = lat.velocity(U, g), vel_direct(U, g)
print("velocity CG vs direct, relative:", float(np.linalg.norm(v_cg - v_dir) / np.linalg.norm(v_dir)))
for name, vel in (("cg", lat.velocity), ("direct", vel_direct)):
    Om = np.zeros((lat.n, 3))
    incs = []
    for it in range(25):
        F = Om - h * vel(compose(U, C.cay(0.5 * Om)), g)
        d = A.solve(-(B0 @ F.ravel())).reshape(lat.n, 3)
        Om = Om + d
        incs.append(float(np.linalg.norm(d) / np.linalg.norm(Om)))
    print(name, " ".join(f"{x:.1e}" for x in incs))
