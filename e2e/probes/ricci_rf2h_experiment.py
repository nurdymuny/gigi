"""RF2H BW05 measurement on the frozen RF2F/RF2G artifacts (same links, masses, target, norm).

usage: python ricci_rf2h_experiment.py OUTDIR [--levels 0,1,2,3] [--no-acc] [--no-adj]
"""
import argparse
import hashlib
import json
import sys
import time
from pathlib import Path

import numpy as np
from scipy.sparse import load_npz
from scipy.sparse.linalg import splu

import ricci_rf2h as rf
from ricci_shape_evidence import numerical_gate

ROOT = Path(__file__).resolve().parents[2]
VAR = ROOT/'output/ricci-variational-20261117/abelian'
SHAPE = ROOT/'output/ricci-shape-regular-20261117/berger'


def sha(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()


def measure(level, order, use_acc, use_adj, payload):
    key = f'level{level}_q{order}'
    prior = json.loads((VAR/'diagnostic.json').read_text())['cases'][key]
    for name, digest in prior['artifacts'].items():
        if sha(VAR/name) != digest:
            raise ValueError('frozen artifact drift: '+name)
    H = load_npz(VAR/(key+'.h.npz')); M = load_npz(VAR/(key+'.m.npz')).tocsc(); D = load_npz(VAR/(key+'.d.npz'))
    with np.load(VAR/(key+'.arrays.npz'), allow_pickle=False) as s:
        a = s['a'][:, 0]; load = s['load'][:, 0]
    build = rf.build_recovery(payload, order=order, verbose=False, use_acc=use_acc, use_adj=use_adj)
    rec = build['receipt']
    if use_acc and rec['acc_relative_residual'] > 1e-9:
        raise ValueError('Acc not satisfied: %g' % rec['acc_relative_residual'])
    if use_adj and rec['adj_relative_residual'] > 1e-9:
        raise ValueError('Adj not satisfied: %g' % rec['adj_relative_residual'])
    if np.max(abs(D-rf.Mesh.__dict__ and D)) if False else False:
        pass
    # incidence must be the frozen one
    Dn = build['mesh'].topo['incidence'].tocsr()
    if (abs(Dn-D)).max() > 0:
        raise ValueError('incidence differs from frozen artifact')
    C = build['C']
    g = rf.gradient(C, H, D, a)
    lu = splu(M)
    v = lu.solve(-g)
    solve_residual = float(np.linalg.norm(M@v+g)/np.linalg.norm(g))
    v = v+lu.solve(-g-M@v)                                     # one refinement
    vol = prior['volume']
    norm = lambda x: float(np.sqrt(max(0., x@(M@x))/vol))
    full = norm(v+4*a)
    load_velocity = lu.solve(load)                              # Whitney-tested continuum velocity
    action = norm(v-load_velocity)
    mass = norm(load_velocity+4*a)
    old = prior['controls']['0']
    return {'level': level, 'order': order, 'h_max': prior['h_max'], 'volume': vol,
            'full_mass_error': full, 'relative_error': full/norm(-4*a),
            'action_error': action, 'mass_interpolation_error': mass,
            'solve_relative_residual': solve_residual,
            'linear_correction_mass_estimate': norm(lu.solve(-g-M@v)),
            'energy': rf.energy(C, H, D, a),
            'rf2f_relative_error_same_case': old['relative_error'],
            'construction': rec}, v


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('outdir', type=Path)
    ap.add_argument('--levels', default='0,1,2,3')
    ap.add_argument('--no-acc', action='store_true'); ap.add_argument('--no-adj', action='store_true')
    args = ap.parse_args()
    args.outdir.mkdir(parents=True, exist_ok=True)
    out = args.outdir/'diagnostic.json'
    if out.exists():
        raise ValueError('preserve prior evidence; choose a new directory')
    levels = [int(x) for x in args.levels.split(',')]
    receipt = {'schema': 'rf2h_v1', 'use_acc': not args.no_acc, 'use_adj': not args.no_adj,
               'sources': {p.name: sha(p) for p in [Path(__file__), Path(rf.__file__)]},
               'cases': {}, 'berger': 'NOT_RUN', 'release_gate': 'OPEN'}
    hs, quad, lin, rows = [], [], [], []
    t0 = time.perf_counter()
    for level in levels:
        payload = json.loads((SHAPE/f'level{level}_berger_q6.input.json').read_text())
        vel = {}
        for order in (4, 6):
            row, v = measure(level, order, not args.no_acc, not args.no_adj, payload)
            receipt['cases'][f'level{level}_q{order}'] = row; vel[order] = v
            print(f'level{level}_q{order}', json.dumps({k: row[k] for k in
                  ('relative_error', 'full_mass_error', 'action_error', 'mass_interpolation_error')}), flush=True)
        M6 = load_npz(VAR/f'level{level}_q6.m.npz'); dv = vel[6]-vel[4]
        quad.append(float(np.sqrt(max(0., dv@(M6@dv))/receipt['cases'][f'level{level}_q6']['volume'])))
        lin.append(max(receipt['cases'][f'level{level}_q{o}']['linear_correction_mass_estimate'] for o in (4, 6)))
        hs.append(receipt['cases'][f'level{level}_q6']['h_max']); rows.append(receipt['cases'][f'level{level}_q6'])
        out.write_text(json.dumps(receipt, indent=1))
    if len(levels) == 4:
        receipt['numerical_gate'] = numerical_gate(rows, hs, quad, lin)
        print(json.dumps(receipt['numerical_gate'], indent=1))
    receipt['elapsed_seconds'] = time.perf_counter()-t0
    out.write_text(json.dumps(receipt, indent=1))


if __name__ == '__main__':
    sys.exit(main())
