"""Regenerate every number and array used in the paper's figures and tables.

Writes data/*.json and data/*.npz next to this file. Read-only use of
scripts/ricci_pg_curvature.py and scripts/ricci_pg_tworing.py on the frozen inputs.
usage: python docs/papers/two-ring-pg/make_data.py [errors] [spectra] [conditioning]
"""
import json, sys, time
from pathlib import Path
import numpy as np
from scipy.linalg import eig, eigh
HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
sys.path[:0] = [str(REPO/'scripts'), str(REPO/'e2e/probes')]
import ricci_pg_curvature as pg
import ricci_pg_tworing as tr
from ricci_consistency_math import solve_rhs

DATA = HERE/'data'; DATA.mkdir(exist_ok=True)
ORIG_FIT = pg.fit_cell
H = [0.388139515370189, 0.20166195340219167, 0.13545926022811192, 0.08159478457777686]
N = [1, 2, 3, 5]


def set_rings(r):
    if r == 1:
        pg.fit_cell = ORIG_FIT
    else:
        tr.use(r)


def rel_of(K, L):
    a, M, vol = L['a'], L['mass'], L['volume']
    norm = lambda v: float(np.sqrt(max(0., v@(M@v))/vol))
    return norm(solve_rhs(M, -(K@a))+4*a)/norm(-4*a)


def errors():
    out = {'h_max': H, 'n': N, 'series': {}}
    plan = [('whitney', 1, 'whitney', 'pg', [0, 1, 2, 3]),
            ('pg_one_ring', 1, 'reconstruction', 'pg', [0, 1, 2, 3]),
            ('pg_two_ring', 2, 'reconstruction', 'pg', [0, 1, 2, 3]),
            ('variational_one_ring', 1, 'reconstruction', 'variational', [0, 1, 2]),
            ('variational_two_ring', 2, 'reconstruction', 'variational', [0, 1, 2])]
    for name, rings, trial, force, levels in plan:
        set_rings(rings); rows = []
        for level in levels:
            t = time.perf_counter(); L = pg.load_level(level, 6)
            r = pg.build(level, 6, trial, data=L, force=force)
            rows.append({'level': level, 'relative_error': r['relative_error'],
                         'curvature_relative_error': r['curvature_relative_error'],
                         'seconds': round(time.perf_counter()-t, 1)})
            print(name, rows[-1], flush=True)
        out['series'][name] = rows
        (DATA/'errors.json').write_text(json.dumps(out, indent=1))
    # symmetrised PG and the transpose term, n = 1, 2, both ring counts
    sym = {}
    for rings in (1, 2):
        set_rings(rings); sym[rings] = []
        for level in (0, 1):
            L = pg.load_level(level, 6)
            Kpg = pg.build(level, 6, 'reconstruction', data=L)['operator']
            Kw = pg.build(level, 6, 'whitney', data=L)['operator']      # Whitney trial: no fit involved
            Ks = (Kpg+Kpg.T-Kw).tocsr()
            a, M, vol = L['a'], L['mass'], L['volume']
            norm = lambda v: float(np.sqrt(max(0., v@(M@v))/vol))
            sym[rings].append({'level': level, 'symmetrised': rel_of(Ks, L),
                               'transpose_term': norm(solve_rhs(M, (Kpg.T-Kw)@a))/norm(-4*a)})
            print('sym', rings, sym[rings][-1], flush=True)
    out['symmetrised'] = {str(k): v for k, v in sym.items()}
    (DATA/'errors.json').write_text(json.dumps(out, indent=1))


def spectra():
    arrays = {}
    for level in (0, 1):
        L = pg.load_level(level, 6); M = L['mass'].toarray()
        arrays[f'whitney_L{level}'] = eigh(L['stiffness'].toarray(), M, eigvals_only=True)
        for rings in (1, 2):
            set_rings(rings)
            K = pg.build(level, 6, 'reconstruction', data=L)['operator'].toarray()
            arrays[f'pg_r{rings}_L{level}'] = eig(K, M, right=False)
            print('spectrum', level, rings, flush=True)
    np.savez_compressed(DATA/'spectra.npz', **arrays)


def conditioning():
    """Per-cell amplification of the fit: largest singular value of the map from neighbourhood
    face fluxes (chart units) to the cell's 11 coefficients, and the number of faces used."""
    out = {}
    for level in (0, 1, 2):
        cells = pg.load_cells(level)['cells']; faces = pg.face_index(cells)
        zero = np.zeros(int(max(max(c['edges']) for c in cells))+1)
        for rings in (1, 2):
            amp, nf, cond = [], [], []
            for ci in range(len(cells)):
                fit = tr.fit_cell_rings(cells, zero, faces, ci, rings=rings)
                amp.append(float(np.linalg.norm(fit['map'], 2))); nf.append(fit['faces']); cond.append(fit['cond'])
            out[f'L{level}_r{rings}'] = {'amplification': amp, 'faces': nf, 'condition': cond}
            print('cond', level, rings, np.median(amp), np.max(amp), flush=True)
    (DATA/'conditioning.json').write_text(json.dumps(out))


if __name__ == '__main__':
    todo = sys.argv[1:] or ['errors', 'spectra', 'conditioning']
    for t in todo:
        globals()[t]()
