"""Source-pinned, supervised existing-control sector pilot. No held-out couplings."""
import argparse
import gc
import importlib
import json
import os
import shutil
import subprocess
import sys
import time
import unittest
from pathlib import Path

import numpy as np
from threadpoolctl import threadpool_limits
import quantum_ordering as qo
import ym_helike_j0 as j0
import ym_helike_j2 as j2
import run_quantum_order_controls as control
import run_ym_helike_convergence as base

ROOT=Path(__file__).resolve().parents[1]
DEFAULT_CUTOFFS=tuple(range(10,25,2))
NLEV=6
SOURCES=(
    'scripts/ym_helike_j0.py','scripts/ym_helike_j2.py','scripts/quantum_ordering.py',
    'scripts/run_ym_helike_sectors.py','scripts/run_ym_helike_j0.py',
    'scripts/test_ym_helike_j0.py','scripts/test_ym_helike_j2.py','scripts/test_sector_review.py',
    'scripts/check_ym_helike_j0_mutations.py','scripts/check_ym_helike_j2_mutations.py',
    'scripts/check_sector_review_mutations.py','scripts/test_quantum_ordering.py','scripts/test_quantum_order_controls.py',
    'scripts/ym_helike_quantum.py','scripts/run_ym_helike_convergence.py','scripts/run_quantum_order_controls.py',
    'docs/research/sector-review-protocol-2026-11-24.md','docs/research/helike-j0-sector-solver-2026-11-24.md')
GATES=('test_ym_helike_j0','test_ym_helike_j2','test_sector_review','test_quantum_ordering','test_quantum_order_controls','mutations')
write=base.atomic
sha=base.digest


def hashes():
    return {p:sha(ROOT/p) for p in SOURCES}


def sector_row(sol):
    if len(sol['energies'])<NLEV or len(sol['residuals'])<NLEV:
        raise ValueError('six levels required')
    if not np.isfinite(sol['energies']+sol['residuals']).all():
        raise ValueError('nonfinite sector output')
    if min(sol['residuals'])<0 or np.any(np.diff(sol['energies'])<0):
        raise ValueError('invalid sector spectrum or residual')
    return {k:sol[k] for k in ('dimension','gram_digits_lost','min_exterior_eig','energies','residuals')}


def make_row(N,scalar,tensor):
    s,t=sector_row(scalar),sector_row(tensor)
    e0=s['energies'][0]; gs=s['energies'][1]-e0; gt=t['energies'][0]-e0
    if min(gs,gt)<=0:
        raise ValueError('positive excitation gaps required')
    allowance=.01*max(1.,min(gs,gt))
    residuals={'vacuum':s['residuals'][0],'scalar':s['residuals'][1],'tensor':t['residuals'][0]}
    return dict(cutoff=N,scalar_sector=s,tensor_sector=t,E0=e0,eps0=s['residuals'][0],
                gap_scalar=gs,gap_tensor=gt,D=t['energies'][0]-s['energies'][1],allowance=allowance,
                allowance_scope='J0_J2_ONLY',global_gap_verified=False,scientific_status='UNRESOLVED',
                residual_gate={k:v<=allowance for k,v in residuals.items()},
                conservative_residual_gate={k:v<=.01 for k,v in residuals.items()},
                ordering=qo.ordering(j0.branch(scalar),j2.branch(tensor)),
                superpotential_floor_ok=min(e0,t['energies'][0])>4.5)


def make_scalar_row(N,scalar):
    s=sector_row(scalar)
    return dict(cutoff=N,scalar_sector=s,quantities=j0.ordering_inputs(scalar),
                allowance_scope='J0_ONLY',global_gap_verified=False,scientific_status='UNRESOLVED',
                superpotential_floor_ok=s['energies'][0]>4.5)


def precision_check(low,high,dps):
    for obj in (low,high):
        for key in ('energies','residuals'):
            if len(obj[key])!=NLEV or not np.isfinite(obj[key]).all():
                raise ValueError('precision comparison requires six finite values of equal length')
    de=max(abs(a-b) for a,b in zip(low['energies'],high['energies']))
    dr=max(abs(a-b)/max(abs(a),1e-300) for a,b in zip(low['residuals'],high['residuals']))
    return dict(dps=dps,max_abs_dE=de,max_rel_dR=dr,passed=de<1e-11 and dr<1e-6)


def validate_request(cases,cutoffs,dps,check_dps):
    if cases!=['1:1.7','2:2.3']:
        raise ValueError('only frozen existing controls 1:1.7 and 2:2.3 are registered')
    if len(cutoffs)<3 or tuple(cutoffs)!=DEFAULT_CUTOFFS[:len(cutoffs)]:
        raise ValueError('registered increasing cutoff prefix 10,12,...,24 required')
    if (dps,check_dps)!=(60,100):
        raise ValueError('registered assembly precisions are 60 and 100')
    return True


def initialize(folder,cases,cutoffs,dps,check_dps,scalar_only=False):
    validate_request(cases,cutoffs,dps,check_dps)
    folder.mkdir(parents=True,exist_ok=False)
    rec=dict(status='RUNNING',mode='J0_ONLY' if scalar_only else 'J0_J2',scientific_status='UNRESOLVED',cases=[],
             request=dict(cases=cases,cutoffs=cutoffs,dps=dps,check_dps=check_dps),
             source_revision=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
             source_hashes=hashes(),gates={},budgets=dict(host_bytes=base.HOST_BUDGET_BYTES,seconds=base.TIME_BUDGET_SECONDS))
    write(folder/'receipt.json',rec)
    try:
        for name,h in rec['source_hashes'].items():
            target=folder/'source'/name; target.parent.mkdir(parents=True,exist_ok=True)
            shutil.copyfile(ROOT/name,target)
            if sha(target)!=h:
                raise ValueError('source drift while snapshotting')
    except BaseException as exc:
        control.failure(folder,'FAILED',str(exc)); raise
    return rec


def save_solution(folder,name,sol):
    path=folder/(name+'.npz'); tmp=path.with_suffix('.pending')
    with tmp.open('wb') as f:
        np.savez_compressed(f,H=np.array(sol['matrix_H']),X=np.array(sol['matrix_X']),V=sol['vectors'],
                            E=sol['energies'],R=sol['residuals'],dps=sol['dps'],
                            basis=json.dumps(sol['basis']),roundoff=sol['residual_square_roundoff'])
        f.flush(); os.fsync(f.fileno())
    tmp.replace(path)
    return dict(file=path.name,sha256=sha(path),dimension=sol['dimension'],dps=sol['dps'],
                gram_digits_lost=sol['gram_digits_lost'],min_exterior_eig=sol['min_exterior_eig'])


def replay_solution(folder,info,expected):
    if info['file']!=expected or sha(folder/expected)!=info['sha256']:
        raise ValueError('array identity or integrity failure')
    with np.load(folder/expected,allow_pickle=False) as a, j0.mp.workdps(info['dps']):
        H=j0.mp.matrix(a['H'].tolist()); X=j0.mp.matrix(a['X'].tolist()); E=a['E']; V=a['V']
        n=info['dimension']
        if H.rows!=n or H.cols!=n or X.rows!=n or X.cols!=n or V.shape!=(n,NLEV) or len(E)!=NLEV:
            raise ValueError('array dimensions invalid')
        np.testing.assert_allclose(V.T@V,np.eye(NLEV),atol=1e-10,rtol=0)
        Hf=np.array(H.tolist(),dtype=float); Xf=np.array(X.tolist(),dtype=float)
        if np.linalg.norm(Hf-Hf.T)>1e-10 or np.linalg.norm(Xf-Xf.T)>1e-10:
            raise ValueError('saved matrices not symmetric')
        np.testing.assert_allclose(np.linalg.eigvalsh(Hf)[:NLEV],E,atol=1e-11,rtol=0)
        residual,tol=j0.residuals_from_matrices(H,X,E,V,info['dps'])
        np.testing.assert_allclose(residual,a['R'],atol=1e-14,rtol=1e-10)
        np.testing.assert_allclose(tol,a['roundoff'],rtol=1e-10,atol=0)
        minimum=float(np.linalg.eigvalsh(Xf).min())
        if minimum < -1e-12:
            raise ValueError('exterior Gram failed')
        base.compare_report(minimum,info['min_exterior_eig'],'exterior minimum')
        return dict(dimension=n,gram_digits_lost=info['gram_digits_lost'],min_exterior_eig=minimum,
                    energies=E.tolist(),residuals=residual)


def verify_run(folder,candidate=None):
    write(folder/'verification.json',dict(status='RUNNING',scientific_status='UNRESOLVED'))
    try:
        rec=base.read_json(folder/'receipt.json') if candidate is None else candidate
        if rec.get('status')!='COMPLETE':
            raise ValueError('incomplete or failed run')
        if rec.get('scientific_status')!='UNRESOLVED' or rec.get('mode') not in ('J0_ONLY','J0_J2'):
            raise ValueError('unsupported scientific claim or mode')
        validate_request(**rec['request'])
        if rec['source_hashes']!=hashes():
            raise ValueError('source drift')
        for name,h in rec['source_hashes'].items():
            if sha(folder/'source'/name)!=h:
                raise ValueError('source snapshot corrupt')
        if rec.get('gates')!={g:True for g in GATES}:
            raise ValueError('missing prerequisites')
        expected=[(g,a,n) for g,a in ((1.,1.7),(2.,2.3)) for n in rec['request']['cutoffs']]
        if [(c['g2'],c['alpha'],c['cutoff']) for c in rec['cases']]!=expected:
            raise ValueError('missing, duplicated or reordered cases')
        previous={}; sectors=(0,) if rec['mode']=='J0_ONLY' else (0,2)
        for case in rec['cases']:
            solved={}
            for spin in sectors:
                name=f"g{case['g2']:g}-N{case['cutoff']}-J{spin}.npz"
                solved[spin]=replay_solution(folder,case['arrays'][str(spin)],name)
            row=make_scalar_row(case['cutoff'],solved[0]) if len(sectors)==1 else make_row(case['cutoff'],solved[0],solved[2])
            base.compare_report(case['measurements'],row,'measurements')
            if not row['superpotential_floor_ok']:
                raise ValueError('superpotential floor failed')
            for spin,s in solved.items():
                key=(case['g2'],spin)
                if key in previous and np.any(np.array(s['energies'][:3])>np.array(previous[key])+1e-12):
                    raise ValueError('variational monotonicity failed')
                previous[key]=s['energies'][:3]
            if case['cutoff']==rec['request']['cutoffs'][-1]:
                for spin in sectors:
                    name=f"g{case['g2']:g}-N{case['cutoff']}-J{spin}-high.npz"
                    hi=replay_solution(folder,case['high_arrays'][str(spin)],name)
                    check=precision_check(solved[spin],hi,rec['request']['check_dps'])
                    base.compare_report(check,case['precision'][str(spin)],'precision')
                    if not check['passed']:
                        raise ValueError('precision check failed')
        result=dict(status='PASS',cases=len(expected),replay='saved high-precision matrices; not fresh assembly',
                    certified=False,separation_verified=False,scientific_status='UNRESOLVED')
        write(folder/'verification.json',result); return result
    except BaseException as exc:
        write(folder/'verification.json',dict(status='FAIL',error=str(exc),scientific_status='UNRESOLVED')); raise


def prerequisites(folder):
    from test_ym_helike_j0 import CONTROLS
    old=base.read_json(CONTROLS/'receipt.json')
    if len(old['cases'])!=6:
        raise ValueError('all six previous control archives required')
    for c in old['cases']:
        if sha(CONTROLS/c['arrays'])!=c['arrays_sha256']:
            raise ValueError('previous controls corrupt')
    gates={}
    for name in GATES[:-1]:
        control.event(folder,'test_gate',gate=name)
        with (folder/(name+'.txt')).open('w',encoding='utf-8') as f:
            result=unittest.TextTestRunner(stream=f,verbosity=2).run(
                unittest.defaultTestLoader.loadTestsFromModule(importlib.import_module(name)))
        if not result.wasSuccessful() or result.skipped or result.testsRun==0:
            raise ValueError('prerequisite failed or skipped: '+name)
        gates[name]=True
    import check_sector_review_mutations as mutations
    results=mutations.run(); write(folder/'mutations.json',results)
    if not results or not all(x['assertion_rejected'] for x in results):
        raise ValueError('mutation gate failed')
    gates['mutations']=True
    return gates


def worker(folder):
    rec=base.read_json(folder/'receipt.json')
    try:
        if hashes()!=rec['source_hashes']:
            raise ValueError('source drift before execution')
        control.event(folder,'startup',backend=j0.mp.libmp.BACKEND,eigensolver='float64')
        with threadpool_limits(limits=1):
            rec['gates']=prerequisites(folder); write(folder/'receipt.json',rec)
            solvers={0:j0.solve} if rec['mode']=='J0_ONLY' else {0:j0.solve,2:j2.solve}
            for spec in rec['request']['cases']:
                g,a=map(float,spec.split(':'))
                for n in rec['request']['cutoffs']:
                    case=dict(g2=g,alpha=a,cutoff=n,arrays={},precision={},high_arrays={}); solved={}
                    for spin,solve in solvers.items():
                        control.event(folder,'solve',g2=g,cutoff=n,spin=spin,dps=60)
                        start=time.monotonic(); sol=solve(n,g,a,dps=60,nlev=NLEV)
                        case['arrays'][str(spin)]=save_solution(folder,f'g{g:g}-N{n}-J{spin}',sol)
                        solved[spin]=sector_row(sol)
                        control.event(folder,'sector_complete',g2=g,cutoff=n,spin=spin,seconds=time.monotonic()-start,
                                      energies=sol['energies'][:3],residuals=sol['residuals'][:3])
                        del sol; gc.collect()
                    case['measurements']=make_scalar_row(n,solved[0]) if rec['mode']=='J0_ONLY' else make_row(n,solved[0],solved[2])
                    rec['cases'].append(case); write(folder/'receipt.json',rec)
                    control.event(folder,'case_complete',g2=g,cutoff=n,measurements=case['measurements'])
                    if n==rec['request']['cutoffs'][-1]:
                        for spin,solve in solvers.items():
                            control.event(folder,'precision_reassembly',g2=g,cutoff=n,spin=spin,dps=100)
                            hi=solve(n,g,a,dps=100,nlev=NLEV)
                            case['high_arrays'][str(spin)]=save_solution(folder,f'g{g:g}-N{n}-J{spin}-high',hi)
                            case['precision'][str(spin)]=precision_check(solved[spin],sector_row(hi),100)
                            write(folder/'receipt.json',rec)
                            if not case['precision'][str(spin)]['passed']:
                                raise ValueError('precision reassembly failed')
                            del hi; gc.collect()
            rec['status']='COMPLETE'; control.event(folder,'final_replay'); verify_run(folder,candidate=rec)
        if hashes()!=rec['source_hashes']:
            raise ValueError('source drift during execution')
        write(folder/'receipt.json',rec); control.event(folder,'complete',scientific_status='UNRESOLVED')
        return 0
    except BaseException as exc:
        control.failure(folder,'FAILED',f'{type(exc).__name__}: {exc}'); raise


def main(scalar_only=False):
    ap=argparse.ArgumentParser(description=__doc__); ap.add_argument('out_dir',type=Path)
    ap.add_argument('--cases',nargs='+',default=['1:1.7','2:2.3'])
    ap.add_argument('--cutoffs',nargs='+',type=int,default=list(DEFAULT_CUTOFFS))
    ap.add_argument('--dps',type=int,default=60); ap.add_argument('--check-dps',type=int,default=100)
    ap.add_argument('--worker',action='store_true'); ap.add_argument('--verify',action='store_true')
    args=ap.parse_args(); folder=args.out_dir.resolve()
    if args.worker:
        return worker(folder)
    if args.verify:
        with threadpool_limits(limits=1):
            print(verify_run(folder))
        return 0
    if subprocess.check_output(['git','status','--porcelain'],cwd=ROOT,text=True).strip():
        raise ValueError('commit and push before the registered run')
    initialize(folder,args.cases,args.cutoffs,args.dps,args.check_dps,scalar_only)
    env=dict(os.environ,OMP_NUM_THREADS='1',OPENBLAS_NUM_THREADS='1',MKL_NUM_THREADS='1')
    try:
        with (folder/'worker.log').open('w',encoding='utf-8') as f:
            p=subprocess.Popen([sys.executable,str(Path(__file__).resolve()),str(folder),'--worker'],
                               cwd=ROOT,env=env,stdout=f,stderr=subprocess.STDOUT)
            return control.supervise(p,folder)
    except BaseException as exc:
        control.failure(folder,'FAILED',str(exc)); raise


if __name__=='__main__':
    raise SystemExit(main())
