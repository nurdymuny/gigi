"""BW05 + S3 for the Petrov-Galerkin force with the two-ring least-squares fit.

Same build(), mass, data, norm and BW05 measurement as scripts/ricci_pg_curvature.py (read-only);
only the fit is routed through scripts/ricci_pg_tworing.py. Gate decided by the existing
numerical_gate (e2e/probes/ricci_shape_evidence.py); h_max from the frozen RF2F diagnostic for the
same meshes. S3 exactly as `ricci_pg_curvature.py s3`: small-eigenvalue count equals the Whitney
gauge dimension and every physical eigenvalue has strictly positive real part (levels 0, 1, 2).

usage: python scripts/ricci_pg_tworing_gate.py OUTDIR [--rings 2]
"""
import argparse, json, sys, time, hashlib
from pathlib import Path
import numpy as np
from scipy.linalg import eig, eigh
HERE = Path(__file__).resolve().parent
sys.path[:0] = [str(HERE), str(HERE.parent/'e2e/probes')]
import ricci_pg_curvature as pg
import ricci_pg_tworing as tr
from ricci_consistency_math import solve_rhs
from ricci_shape_evidence import numerical_gate


def sha(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()


def main():
    ap = argparse.ArgumentParser(); ap.add_argument('outdir', type=Path); ap.add_argument('--rings', type=int, default=2)
    ap.add_argument('--s3-levels', default='0,1,2'); ap.add_argument('--levels', default='0,1,2,3')
    args = ap.parse_args()
    args.outdir.mkdir(parents=True, exist_ok=True)
    out = args.outdir/'diagnostic.json'
    if out.exists():
        raise SystemExit('refusing to overwrite '+str(out))
    tr.use(args.rings)
    hmax = json.loads((pg.ROOT/'output/ricci-variational-20261117/abelian/diagnostic.json').read_text())['cases']
    rec = {'schema': 'pg_tworing_bw05_s3_v1', 'rings': args.rings, 'cases': {}, 's3': {},
           'sources': {p: sha(HERE/p) for p in ['ricci_pg_curvature.py', 'ricci_pg_tworing.py', 'ricci_pg_tworing_gate.py']},
           'berger': 'NOT_RUN', 'release_gate': 'OPEN'}
    rows, hs, quad, lin = [], [], [], []
    levels = [int(x) for x in args.levels.split(',')]
    for level in levels:
        vel = {}
        for order in (4, 6):
            t = time.perf_counter()
            L = pg.load_level(level, order)
            r = pg.build(level, order, 'reconstruction', data=L)
            M = L['mass']; vol = L['volume']
            norm = lambda v: float(np.sqrt(max(0., v@(M@v))/vol))
            corr = solve_rhs(M, -r['force']-M@r['velocity'])
            row = {k: r[k] for k in ('full_mass_error', 'relative_error', 'curvature_relative_error', 'max_condition', 'edges')}
            row.update(level=level, order=order, h_max=hmax[f'level{level}_q6']['h_max'],
                       linear_correction_mass_estimate=norm(corr), seconds=round(time.perf_counter()-t, 1))
            rec['cases'][f'level{level}_q{order}'] = row; vel[order] = (r['velocity'], M, vol)
            print('CASE', json.dumps({k: row[k] for k in ('level', 'order', 'relative_error', 'curvature_relative_error', 'seconds')}), flush=True)
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
    for level in [int(x) for x in args.s3_levels.split(',') if x]:
        t = time.perf_counter()
        L = pg.load_level(level, 6)
        K = pg.build(level, 6, 'reconstruction', data=L)['operator'].toarray()
        M = L['mass'].toarray(); Kw = L['stiffness'].toarray()
        lw = eigh(Kw, M, eigvals_only=True); gauge = int((abs(lw) < 1).sum())
        lp = eig(K, M, right=False); small = abs(lp) < 1; phys = lp[~small]
        s3 = {'level': level, 'edges': len(M), 'gauge_dimension_whitney': gauge,
              'small_eigenvalues_pg': int(small.sum()),
              'min_real_part_physical': float(phys.real.min()),
              'max_abs_imag_physical': float(abs(phys.imag).max()),
              'negative_real_part_count': int((phys.real < 0).sum()),
              'min_real_part_whitney_physical': float(lw[abs(lw) >= 1].min()),
              'stable': bool(small.sum() == gauge and phys.real.min() > 0),
              'seconds': round(time.perf_counter()-t, 1)}
        rec['s3'][str(level)] = s3
        print('S3', json.dumps(s3), flush=True)
        out.write_text(json.dumps(rec, indent=1))


if __name__ == '__main__':
    main()
