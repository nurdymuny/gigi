"""Numerical-range (field-of-values) certificate: min eigenvalue of the symmetric part of the PG
operator against the mass, restricted to the complement of the Whitney gauge kernel."""
import json, sys
from pathlib import Path
import numpy as np
from scipy.linalg import eigh
HERE = Path(__file__).resolve().parent; REPO = HERE.parents[2]
sys.path[:0] = [str(REPO/'scripts'), str(REPO/'e2e/probes')]
import ricci_pg_curvature as pg, ricci_pg_tworing as tr
ORIG = pg.fit_cell; out = {}
for level in (0, 1):
    L = pg.load_level(level, 6); M = L['mass'].toarray(); Kw = L['stiffness'].toarray()
    lw, V = eigh(Kw, M); P = V[:, abs(lw) >= 1]           # M-orthonormal basis of the physical (non-gauge) space
    for rings in (1, 2):
        pg.fit_cell = ORIG if rings == 1 else (tr.use(2) or pg.fit_cell)
        K = pg.build(level, 6, 'reconstruction', data=L)['operator'].toarray()
        Ks = P.T@(0.5*(K+K.T))@P
        mu = np.linalg.eigvalsh(Ks)
        gauge_leak = float(np.max(abs(K@V[:, abs(lw) < 1])))
        out[f'L{level}_r{rings}'] = {'min_symmetric_part': float(mu.min()), 'max_symmetric_part': float(mu.max()),
                                     'negative_count': int((mu < 0).sum()), 'physical_dim': int(P.shape[1]),
                                     'gauge_annihilated_max': gauge_leak}
        print(level, rings, out[f'L{level}_r{rings}'], flush=True)
(HERE/'data'/'numerical_range.json').write_text(json.dumps(out, indent=1))
