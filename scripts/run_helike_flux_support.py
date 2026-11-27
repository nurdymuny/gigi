"""Registered one-point support diagnostic; independent recovery from binary64 bit patterns."""
import argparse
import json
import math
import os
from pathlib import Path
import subprocess
import time

import numpy as np
import run_helike_flux_grid as grid

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / 'research/flux-basis-smoke-20261126/receipt.json'
BASE_SHA = 'a2dbf056709b8b8b02a11d3aa0a8ef09bafad9abb80c5e8f3f3aaa8b8e2e4479'
POINT = {'s': 1.18, 't': 3.0, 'g2': 0.03, 'mq': 12, 'probe_mq': 14}


def decode(bits):
    a = np.asarray(bits, dtype=np.uint64).view(np.float64)
    if not np.isfinite(a).all():
        raise ValueError('nonfinite stored coefficients')
    return a


def measure(basis, weights, roles):
    b, p = np.asarray(basis), np.asarray(weights, dtype=float)
    if (b.ndim != 2 or b.shape[1] != 6 or p.shape != (len(b),) or not len(b)
            or not np.isfinite(b).all() or not np.equal(b, np.floor(b)).all() or (b < 0).any()
            or not np.isfinite(p).all() or (p < 0).any() or sorted(roles) != list(range(6))):
        raise ValueError('invalid occupation distribution')
    b = b.astype(int)
    n = b.sum(axis=1)
    g = np.column_stack([b[:,roles[0]]+b[:,roles[1]], b[:,roles[2]], b[:,roles[3]], b[:,roles[4]]+b[:,roles[5]]])
    mass = float(p.sum())
    occupied = float(p[n > 0].sum())
    if not math.isfinite(mass):
        raise ValueError('overflowed mass')
    joint = {}
    for key, w in zip(map(tuple, g), p):
        if w > 0:
            joint[key] = joint.get(key, 0.0) + float(w)
    return {'mass': mass, 'shells': np.bincount(n, weights=p, minlength=int(n.max())+1).tolist(),
            'joint': [{'occupation': [int(k) for k in key], 'mass': w} for key, w in sorted(joint.items())],
            'mean_group_occupation': ((p[:,None]*g).sum(axis=0)/mass).tolist() if mass else None,
            'occupation_fraction': ((p[n>0,None]*g[n>0]/n[n>0,None]).sum(axis=0)/occupied).tolist() if occupied else None}


def agree(a, b):
    if isinstance(a, dict):
        if not isinstance(b, dict) or a.keys() != b.keys():
            raise ValueError('summary fields differ')
        for k in a:
            agree(a[k], b[k])
    elif isinstance(a, list):
        if not isinstance(b, list) or len(a) != len(b):
            raise ValueError('summary dimensions differ')
        for x, y in zip(a, b):
            agree(x, y)
    elif a is None:
        if b is not None:
            raise ValueError('empty distribution gained a normalized summary')
    elif not isinstance(b, (int, float)) or not math.isclose(a, b, rel_tol=1e-10, abs_tol=1e-14):
        raise ValueError(f'independent summary disagrees: {a} != {b}')


def inspect(x, baseline):
    if x['schema'] != 'helike-lab/cone-support/v1' or any(x[k] != v for k, v in POINT.items()):
        raise ValueError('wrong diagnostic point or schema')
    if x['certified'] is not False or x['full_grid_released'] is not False:
        raise ValueError('diagnostic cannot certify or release a grid')
    for key, mq in [('basis', 12), ('probe_basis', 14)]:
        b = x[key]
        if (len(b) != math.comb(mq+6, 6) or len({tuple(n) for n in b}) != len(b)
                or any(len(n) != 6 or any(type(k) is not int or k < 0 for k in n) or sum(n) > mq for n in b)):
            raise ValueError('incomplete or invalid occupation simplex')
    q = x['identification']
    for name in ('helike', 'competitor'):
        for key, tol in [('gap', 1e-8), ('weight', 1e-6), ('dressed_weight', 1e-6)]:
            if not math.isfinite(q[name][key]) or abs(q[name][key] - baseline[name][key]) > tol:
                raise ValueError(f'replay mismatch: {name}.{key}')
    if len(q['levels']) != 12 or len(q['in_space_lanczos_residuals']) != 12:
        raise ValueError('missing finite-space evidence')
    if any(not math.isfinite(e) or abs(e-old)>1e-8 for e,old in zip(q['levels'],baseline['levels'])):
        raise ValueError('level replay mismatch')
    if any(not math.isfinite(r) or not 0 <= r <= 1e-10*max(1,abs(e))
           for r,e in zip(q['in_space_lanczos_residuals'],q['levels'])):
        raise ValueError('finite-space solve unresolved')
    if [r['name'] for r in x['states']] != ['ground','helike_a','helike_b','competitor']:
        raise ValueError('missing or reordered states')
    indices = [r['level_index'] for r in x['states']]
    if len(set(indices)) != 4 or indices[0] != 0 or indices[3] != q['competitor']['level_index'] or q['helike']['level_index'] not in indices[1:3]:
        raise ValueError('state selection mismatch')
    source = {tuple(n) for n in x['basis']}
    inside = np.array([tuple(n) in source for n in x['probe_basis']])
    rows, vectors, residuals = [], [], []
    for state in x['states']:
        if not math.isfinite(state['energy']) or abs(state['energy']-q['levels'][state['level_index']])>1e-10:
            raise ValueError('state energy does not match its level')
        c, r = decode(state['coeff_bits']), decode(state['probe_residual_bits'])
        if c.shape != (len(source),) or r.shape != (len(inside),):
            raise ValueError('coefficient dimensions differ')
        if abs(float(c@c)-1) > 1e-8:
            raise ValueError('unnormalized state')
        outside = r.copy()
        outside[inside] = 0
        p = measure(x['basis'], c*c, x['roles'])
        o = measure(x['probe_basis'], outside*outside, x['roles'])
        agree(p, state['probability'])
        agree(o, state['omitted'])
        for key, want in [('inner_norm',np.linalg.norm(r[inside])), ('outer_norm',np.linalg.norm(outside)), ('probe_norm',np.linalg.norm(r))]:
            agree(float(want), state[key])
        if not 0 <= state['nesting_error'] <= 1e-9*max(1,abs(state['energy'])):
            raise ValueError('nested-block check failed')
        rows.append({k: state[k] for k in ('name','level_index','energy','reference_weight','inner_norm','outer_norm','probe_norm','nesting_error')}
                    | {'probability':p, 'omitted':o})
        vectors.append(c)
        residuals.append(outside)
    pair_p = measure(x['basis'], (vectors[1]**2+vectors[2]**2)/2, x['roles'])
    pair_o = measure(x['probe_basis'], (residuals[1]**2+residuals[2]**2)/2, x['roles'])
    agree(pair_p, x['helike_average_probability'])
    agree(pair_o, x['helike_average_omitted'])
    return {'states':rows, 'helike_average_probability':pair_p, 'helike_average_omitted':pair_o,
            'overall':'UNRESOLVED', 'certified':False, 'full_grid_released':False}


def plot(result, path):
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    series = [('Ground',result['states'][0]['probability'],result['states'][0]['omitted'],'#555555'),
              ('Helike pair average',result['helike_average_probability'],result['helike_average_omitted'],'#167d91'),
              ('Competitor',result['states'][3]['probability'],result['states'][3]['omitted'],'#b54845')]
    fig, axs = plt.subplots(1,2,figsize=(12,4.8),layout='constrained')
    for label, p, o, color in series:
        axs[0].semilogy(range(len(p['shells'])),p['shells'],marker='o',ms=3,label=label,color=color)
    axs[0].set(xlabel='Total occupation in the solved basis',ylabel='Probability in shell',title='Where the computed states have support')
    axs[0].legend(fontsize=9)
    for i,(label,p,o,color) in enumerate(series):
        shares = o['occupation_fraction']
        if shares is not None:
            axs[1].bar(np.arange(4)+(i-1)*.25,shares,width=.24,color=color,label=label)
    axs[1].set(xticks=np.arange(4),xticklabels=['H pair','Competitor mode','Other singlet','Stiff pair'],
               ylabel='Occupation-weighted share of omitted residual',ylim=(0,1),title='Where shells 13-14 need additional support')
    for ax in axs:
        ax.spines[['top','right']].set_visible(False)
        ax.grid(axis='y',alpha=.15)
    fig.suptitle('One-point basis diagnostic: s=1.18, t=3, g²=0.03, Mq=12',fontsize=13)
    fig.savefig(path,dpi=180)
    plt.close(fig)


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('out_dir',type=Path)
    args = ap.parse_args()
    out = args.out_dir.resolve()
    out.mkdir(parents=True,exist_ok=False)
    rec = {'status':'RUNNING','point':POINT,'overall':'UNRESOLVED','certified':False,'full_grid_released':False}
    ledger = out/'receipt.json'
    grid.save(ledger,rec)
    started = time.time()
    try:
        if grid.sha(BASE) != BASE_SHA:
            raise ValueError('baseline digest mismatch')
        baseline = json.loads(BASE.read_text())['rungs'][-1]['result']['result']
        raw_path = out/'support.json'
        cmd = [str(grid.HELIKE_BIN),'--out',str(out/'native-receipt.json'),'cone-support','--s','1.18','--t','3','--g2','0.03','--mq','12','--support-out',str(raw_path)]
        rec.update(command=cmd,source_revision=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
                   binary_sha256=grid.raw_sha(grid.HELIKE_BIN),harness_sha256_lf=grid.sha(Path(__file__)),baseline_sha256_lf=BASE_SHA)
        grid.save(ledger,rec)
        print('Starting one diagnostic; no Mq 14 eigensolve.',flush=True)
        with (out/'stdout.log').open('w') as stdout, (out/'stderr.log').open('w') as stderr:
            proc = subprocess.run(cmd,cwd=ROOT,env={**os.environ,'RAYON_NUM_THREADS':'8'},stdout=stdout,stderr=stderr,timeout=900)
        rec['returncode'] = proc.returncode
        if proc.returncode:
            raise RuntimeError(f'diagnostic exited {proc.returncode}; see stderr.log')
        native = json.loads((out/'native-receipt.json').read_text())
        if native['status'] != 'COMPUTED':
            raise ValueError('unexpected native status')
        if native['results']['support_sha256_lf'] != grid.sha(raw_path):
            raise ValueError('export digest mismatch')
        rec['analysis'] = inspect(json.loads(raw_path.read_text()),baseline)
        rec['support_sha256_lf'] = grid.sha(raw_path)
        rec['native_receipt_sha256_lf'] = grid.sha(out/'native-receipt.json')
        plot(rec['analysis'],out/'support.png')
        rec['status'] = 'DIAGNOSTIC_COMPLETE'
    except BaseException as exc:
        rec.update(status='STOPPED',error=f'{type(exc).__name__}: {exc}')
    finally:
        rec['elapsed_seconds'] = time.time()-started
        grid.save(ledger,rec)
    print(json.dumps({k:rec[k] for k in ('status','overall','elapsed_seconds')}),flush=True)
    if 'error' in rec:
        print(rec['error'],flush=True)
    return 0 if rec['status']=='DIAGNOSTIC_COMPLETE' else 2


if __name__ == '__main__':
    raise SystemExit(main())
