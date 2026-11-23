"""Symmetrised Petrov-Galerkin (RF2H x PG synthesis): K_sym = K_PG + K_PG^T - K_W.

K_PG = D^T (M2 + X) D is the consistent Petrov-Galerkin operator (feat/ricci-pg-curvature);
K_W  = D^T M2 D the original Whitney stiffness. K_sym = D^T (M2 + X + X^T) D is symmetric, the
Hessian of E(a) = 1/2 a^T K_sym a, and equals K_PG + D^T X^T D. Its BW05 error is K_PG's plus
the transpose term D^T X^T Phi_F (RF2H term B''). Read-only use of the PG branch's build().
"""
import json, sys, time
from pathlib import Path
import numpy as np
from scipy.sparse.linalg import splu
sys.path[:0] = [str(Path(__file__).resolve().parent), str(Path(__file__).resolve().parents[1]/'e2e/probes')]
import ricci_pg_curvature as pg
from ricci_consistency_math import solve_rhs

levels = [int(x) for x in sys.argv[1].split(',')]
spectrum = 'spectrum' in sys.argv[2:]
rings = next((int(x[6:]) for x in sys.argv[2:] if x.startswith('rings=')), None)
if rings is not None:
    import ricci_pg_tworing as tr; tr.use(rings)
for level in levels:
    t = time.perf_counter()
    L = pg.load_level(level, 6)
    PG = pg.build(level, 6, trial='reconstruction', force='pg', data=L)
    W = pg.build(level, 6, trial='whitney', force='pg', data=L)
    VAR = pg.build(level, 6, trial='reconstruction', force='variational', data=L)
    a, mass, vol = L['a'], L['mass'], L['volume']
    Kpg, Kw = PG['operator'], W['operator']
    Ksym = (Kpg + Kpg.T - Kw).tocsr()
    norm = lambda v: float(np.sqrt(max(0., v @ (mass @ v))/vol))
    ref = norm(-4*a)
    rel = lambda K: norm(solve_rhs(mass, -(K @ a)) + 4*a)/ref
    extra = norm(solve_rhs(mass, (Kpg.T - Kw) @ a))/ref          # D^T X^T Phi_F in BW05 units
    out = {'variational': rel(VAR['operator']), 'level': level, 'rings': rings, 'curvature_rel': PG['curvature_relative_error'], 'whitney': rel(Kw), 'pg': rel(Kpg), 'sym': rel(Ksym),
           'transpose_term': extra, 'asym_norm_ratio': float(abs(Kpg-Kpg.T).max()/abs(Kpg).max()),
           'seconds': round(time.perf_counter()-t, 1)}
    if spectrum:
        from scipy.linalg import eigh
        A = Ksym.toarray(); Bm = mass.toarray()
        ev = eigh(A, Bm, eigvals_only=True)
        out.update({'sym_min_eig': float(ev.min()), 'sym_max_eig': float(ev.max()),
                    'n_negative': int((ev < -1e-8*abs(ev).max()).sum()),
                    'whitney_max_eig': float(eigh(Kw.toarray(), Bm, eigvals_only=True).max())})
    print('SYM', json.dumps(out), flush=True)
