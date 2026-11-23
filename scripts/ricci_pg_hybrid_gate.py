"""BW05 + S3 + numerical range for the Petrov-Galerkin force with the hybrid three-ring fit.
Spec: docs/RICCI_PG_HYBRID_SPEC.md.

Identical to scripts/ricci_pg_tworing_gate.py (build, mass, data, norm, BW05 measurement, the
unchanged numerical_gate, h_max from the frozen RF2F diagnostic, S3 exactly as there) except that the
fit is routed through scripts/ricci_pg_hybrid.py. Adds the paper's numerical-range certificate
(docs/papers/two-ring-pg/numerical_range.py): the smallest eigenvalue of the symmetric part against the
mass on the physical (non-gauge) space.

usage: python scripts/ricci_pg_hybrid_gate.py OUTDIR [--rings 3] [--levels 0,1,2,3]
       [--s3-levels 0,1,2] [--nr-levels 0,1]
"""
import argparse, json, os, sys, time, hashlib
from pathlib import Path
import numpy as np
from scipy.linalg import eig, eigh
HERE = Path(__file__).resolve().parent
sys.path[:0] = [str(HERE), str(HERE.parent/'e2e/probes')]
import ricci_pg_curvature as pg
import ricci_pg_hybrid as hy
from ricci_consistency_math import solve_rhs
from ricci_shape_evidence import numerical_gate

# frozen inputs may live in another checkout (they are not tracked); default is this repo, as before
DATA = Path(os.environ.get('GIGI_RICCI_DATA_ROOT', pg.ROOT))
pg.BASE = DATA/'output/ricci-shape-regular-20261117/berger'
pg.DIAG = DATA/'output/ricci-consistency-20261117/diagnostic'


def sha(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()


def main():
    ap = argparse.ArgumentParser(); ap.add_argument('outdir', type=Path); ap.add_argument('--rings', type=int, default=3)
    ap.add_argument('--s3-levels', default='0,1,2'); ap.add_argument('--levels', default='0,1,2,3')
    ap.add_argument('--nr-levels', default='0,1')
    args = ap.parse_args()
    args.outdir.mkdir(parents=True, exist_ok=True)
    out = args.outdir/'diagnostic.json'
    if out.exists():
        raise SystemExit('refusing to overwrite '+str(out))
    hy.use(args.rings)
    hmax = json.loads((DATA/'output/ricci-variational-20261117/abelian/diagnostic.json').read_text())['cases']
    rec = {'schema': 'pg_hybrid_bw05_s3_nr_v1', 'fit': 'hybrid', 'rings': args.rings, 'cases': {}, 's3': {},
           'numerical_range': {},
           'sources': {p: sha(HERE/p) for p in ['ricci_pg_curvature.py', 'ricci_pg_tworing.py', 'ricci_pg_hybrid.py',
                                                'ricci_pg_hybrid_gate.py']},
           'berger': 'NOT_RUN', 'release_gate': 'OPEN'}
    rows, hs, quad, lin = [], [], [], []
    levels = [int(x) for x in args.levels.split(',') if x]
    for level in levels:
        vel = {}
        for order in (4, 6):
            t = time.perf_counter()
            L = pg.load_level(level, order)
            r = pg.build(level, order, 'reconstruction', data=L)
            M = L['mass']; vol = L['volume']
            norm = lambda v: float(np.sqrt(max(0., v@(M@v))/vol))     # noqa: E731
            corr = solve_rhs(M, -r['force']-M@r['velocity'])
            row = {k: r[k] for k in ('full_mass_error', 'relative_error', 'curvature_relative_error', 'max_condition', 'edges')}
            row.update(level=level, order=order, h_max=hmax[f'level{level}_q6']['h_max'],
                       linear_correction_mass_estimate=norm(corr), seconds=round(time.perf_counter()-t, 1))
            rec['cases'][f'level{level}_q{order}'] = row; vel[order] = (r['velocity'], M, vol)
            print('CASE', json.dumps({k: row[k] for k in ('level', 'order', 'relative_error', 'curvature_relative_error',
                                                          'max_condition', 'seconds')}), flush=True)
            out.write_text(json.dumps(rec, indent=1))
        (v6, M6, vol6), (v4, _, _) = vel[6], vel[4]
        dv = v6-v4
        quad.append(float(np.sqrt(max(0., dv@(M6@dv))/vol6)))
        lin.append(max(rec['cases'][f'level{level}_q{o}']['linear_correction_mass_estimate'] for o in (4, 6)))
        hs.append(rec['cases'][f'level{level}_q6']['h_max']); rows.append(rec['cases'][f'level{level}_q6'])
    if len(levels) == 4:
        rec['numerical_gate'] = numerical_gate(rows, hs, quad, lin)
        print('GATE', json.dumps(rec['numerical_gate']), flush=True)
    out.write_text(json.dumps(rec, indent=1))
    s3_levels = {int(x) for x in args.s3_levels.split(',') if x}
    nr_levels = {int(x) for x in args.nr_levels.split(',') if x}
    for level in sorted(s3_levels | nr_levels):
        t = time.perf_counter()
        L = pg.load_level(level, 6)
        K = pg.build(level, 6, 'reconstruction', data=L)['operator'].toarray()
        M = L['mass'].toarray(); Kw = L['stiffness'].toarray()
        lw, V = eigh(Kw, M); gauge_mask = abs(lw) < 1; gauge = int(gauge_mask.sum())
        if level in s3_levels:
            lp = eig(K, M, right=False); small = abs(lp) < 1; phys = lp[~small]
            s3 = {'level': level, 'edges': len(M), 'gauge_dimension_whitney': gauge,
                  'small_eigenvalues_pg': int(small.sum()),
                  'min_real_part_physical': float(phys.real.min()),
                  'max_abs_imag_physical': float(abs(phys.imag).max()),
                  'negative_real_part_count': int((phys.real < 0).sum()),
                  'min_real_part_whitney_physical': float(lw[~gauge_mask].min()),
                  'stable': bool(small.sum() == gauge and phys.real.min() > 0),
                  'seconds': round(time.perf_counter()-t, 1)}
            rec['s3'][str(level)] = s3
            print('S3', json.dumps(s3), flush=True)
        if level in nr_levels:
            P = V[:, ~gauge_mask]
            mu = np.linalg.eigvalsh(P.T@(0.5*(K+K.T))@P)
            nr = {'min_symmetric_part': float(mu.min()), 'max_symmetric_part': float(mu.max()),
                  'negative_count': int((mu < 0).sum()), 'physical_dim': int(P.shape[1]),
                  'gauge_annihilated_max': float(np.max(abs(K@V[:, gauge_mask])))}
            rec['numerical_range'][str(level)] = nr
            print('NR', level, json.dumps(nr), flush=True)
        out.write_text(json.dumps(rec, indent=1))


if __name__ == '__main__':
    main()
