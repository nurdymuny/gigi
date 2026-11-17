"""S3 (same criterion as scripts/ricci_pg_s3_modes.py) for the PG operator with the ring-r fit.
Read-only; prints one JSON line per level."""
import json, sys
from pathlib import Path
import numpy as np
from scipy.linalg import eig
sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_pg_curvature as pg
import ricci_pg_tworing as tr
rings = int(sys.argv[1]); levels = [int(x) for x in sys.argv[2].split(',')]
tr.use(rings)
for level in levels:
    L = pg.load_level(level, 6)
    K = pg.build(level, 6, "reconstruction", data=L)["operator"].toarray()
    M = L["mass"].toarray()
    lam = eig(K, M, right=False)
    phys = abs(lam) >= 1
    neg = phys & (lam.real < 0)
    print('S3', json.dumps({"rings": rings, "level": level, "physical_modes": int(phys.sum()),
        "negative_real_part": int(neg.sum()),
        "most_negative_real_part": float(lam[phys].real.min()),
        "max_abs_imag": float(abs(lam[phys].imag).max()),
        "max_real": float(lam[phys].real.max())}), flush=True)
