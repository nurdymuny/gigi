"""Bounded existing-control pilot. Never evaluates held-out couplings."""
import argparse
import csv
import gc
import json
import os
import shutil
import subprocess
import sys
import time
import uuid
from datetime import datetime, timezone
from pathlib import Path

import numpy as np
import scipy.linalg as la

import quantum_ordering as o
import run_ym_helike_convergence as base
import ym_helike_quantum as q

ROOT = Path(__file__).resolve().parents[1]
CONTROLS = ((1.,1.7),(2.,2.3))
CUTOFFS = (10,12,14)
FILES = (
    'scripts/quantum_ordering.py', 'scripts/run_quantum_order_controls.py',
    'scripts/test_quantum_ordering.py', 'scripts/test_quantum_order_controls.py',
    'scripts/check_quantum_order_mutations.py', 'scripts/ym_helike_quantum.py',
    'scripts/run_ym_helike_convergence.py', 'scripts/test_ym_helike_convergence.py',
    'scripts/check_ym_helike_convergence_mutations.py', 'scripts/verify_ym_helike_evidence.py',
    'scripts/test_ym_helike_quantum.py',
    'docs/research/quantum-order-controls-protocol-2026-11-23.md',
    'docs/research/helike-quantum-crossing-preregistration-2026-11-23.md',
    'research/helike-quantum-probe-20261123/exploration-edge-20261124/width_residuals_N10.json',
)
GATES = ('ordering_tests','runner_tests','original_tests','amendment_tests','mutations')


def hashes():
    return {n:base.digest(ROOT/n) for n in FILES}


def event(folder, phase, **fields):
    row = dict(utc=datetime.now(timezone.utc).isoformat(), phase=phase, **fields)
    base.atomic(folder/'progress.json', row)
    with (folder/'events.jsonl').open('a',encoding='utf-8') as f:
        f.write(json.dumps(row,allow_nan=False)+'\n'); f.flush(); os.fsync(f.fileno())
    print(json.dumps(row,allow_nan=False),flush=True)


def failure(folder, status, error):
    receipt = base.read_json(folder/'receipt.json')
    receipt.update(status=status,error=str(error),scientific_status='UNRESOLVED')
    base.atomic(folder/'receipt.json',receipt)
    base.atomic(folder/'verification.json',dict(status='FAIL',error=str(error)))


def branch_inputs(levels):
    out=[]
    for name in ('scalar','tensor'):
        target, nxt = levels[name], levels[name+'_next']
        out.append(dict(low=target['low'],high=target['high'],residual=target['residual'],
                        next_low=nxt['low'],next_residual=nxt['residual'],
                        spin_error=max(target['spin_error'],nxt['spin_error'])))
    return out


def measure(b,ops,g,alpha,device,folder,codes):
    n=b['cutoff']; identity=f'N{n}-g2-{g:g}-alpha-{alpha:g}'
    event(folder,'solve',case=identity)
    H=q.hamiltonian(ops,g,alpha)
    e,v,diag=q.solve_generalized(H,b['S'],device)
    ec,vc,_=q.solve_generalized(H,b['S'],'cpu')
    np.testing.assert_allclose(e,ec,rtol=1e-10,atol=1e-10)
    selected=o.select_levels(e,v,b['S'],ops['J2'])
    reference=o.select_levels(ec,vc,b['S'],ops['J2'])
    arrays=dict(H=H,S=b['S'],J2=ops['J2'],energies=e,vectors=v)
    levels={}
    for name, block in selected.items():
        overlap=block['vectors'].conj().T@b['S']@reference[name]['vectors']
        np.testing.assert_allclose(la.svdvals(overlap),np.ones(block['rank']),atol=1e-10,rtol=1e-10)
        event(folder,'full_residual',case=identity,level=name,rank=block['rank'])
        r=o.full_residual(b,block['vectors'],block['energy_block'],g,alpha,codes)
        levels[name]={k:value for k,value in block.items() if k not in ('vectors','energy_block')}
        levels[name]['residual']=r['norm']
        arrays['V_'+name]=block['vectors']; arrays['B_'+name]=block['energy_block']; arrays['R_'+name]=r['gram']
        event(folder,'level_complete',case=identity,level=name,residual=r['norm'])
    decision=o.ordering(*branch_inputs(levels))
    result=dict(cutoff=n,g2=g,alpha=alpha,levels=levels,order=decision,solver=diag,
                absolute_gap=float(e[1]-e[0]),allowed_residual=.01*max(1.,float(e[1]-e[0])),
                scientific_status='UNRESOLVED',certified=False)
    path=folder/(identity+'.npz'); tmp=path.with_suffix('.pending')
    with tmp.open('wb') as f:
        np.savez_compressed(f,**arrays); f.flush(); os.fsync(f.fileno())
    tmp.replace(path)
    result.update(arrays=path.name,arrays_sha256=base.digest(path))
    return result


def replay_case(folder,c,b,ops,codes):
    name=f"N{c['cutoff']}-g2-{c['g2']:g}-alpha-{c['alpha']:g}.npz"
    if c['arrays'] != name or base.digest(folder/name) != c['arrays_sha256']:
        raise ValueError('array integrity failure')
    with np.load(folder/name,allow_pickle=False) as a:
        for key,expected in (('H',q.hamiltonian(ops,c['g2'],c['alpha'])),('S',b['S']),('J2',ops['J2'])):
            np.testing.assert_allclose(a[key],expected,rtol=1e-10,atol=1e-10)
        e,v=a['energies'],a['vectors']
        if not np.isfinite(e).all() or not np.isfinite(v).all() or np.any(np.diff(e)<0):
            raise ValueError('invalid saved eigenpairs')
        hv=a['H']@v; sv=(a['S']@v)*e
        if la.norm(hv-sv)/max(1.,la.norm(hv),la.norm(sv))>1e-10:
            raise ValueError('saved eigenpair residual exceeds solver tolerance')
        ec=la.eigvalsh(a['H'],a['S'])
        np.testing.assert_allclose(e,ec,atol=1e-10,rtol=1e-10)
        selected=o.select_levels(e,v,a['S'],a['J2'])
        for name,block in selected.items():
            # Replay the stored subspace itself; its arbitrary frame is retained.
            V,B=a['V_'+name],a['B_'+name]
            overlap=V.conj().T@b['S']@block['vectors']
            np.testing.assert_allclose(la.svdvals(overlap),np.ones(block['rank']),atol=1e-10,rtol=1e-10)
            np.testing.assert_allclose(V.conj().T@a['H']@V,B,atol=1e-10,rtol=1e-10)
            event(folder,'replay_residual',cutoff=c['cutoff'],g2=c['g2'],level=name)
            r=o.full_residual(b,V,B,c['g2'],c['alpha'],codes)
            np.testing.assert_allclose(a['R_'+name],r['gram'],atol=1e-10,rtol=1e-10)
            measured={k:value for k,value in block.items() if k not in ('vectors','energy_block')}
            measured['residual']=r['norm']
            base.compare_report(c['levels'][name],measured,name)
        base.compare_report(c['order'],o.ordering(*branch_inputs(c['levels'])),'order')
        base.compare_report(c['absolute_gap'],float(e[1]-e[0]),'gap')
        base.compare_report(c['allowed_residual'],.01*max(1.,float(e[1]-e[0])),'allowance')
        if c.get('certified') is not False or c.get('scientific_status')!='UNRESOLVED':
            raise ValueError('unsupported scientific acceptance')


def verify_run(folder, replay=True, candidate=None):
    base.atomic(folder/'verification.json',dict(status='RUNNING',scientific_status='UNRESOLVED'))
    try:
        receipt=base.read_json(folder/'receipt.json') if candidate is None else candidate
        if receipt.get('status')!='COMPLETE':
            raise ValueError('incomplete control run')
        if receipt.get('mode')!='EXISTING_CONTROLS' or receipt.get('scientific_status')!='UNRESOLVED':
            raise ValueError('invalid control mode or scientific claim')
        if receipt.get('source_hashes')!=hashes():
            raise ValueError('source hashes differ')
        for name,h in receipt['source_hashes'].items():
            if base.digest(folder/'source'/name)!=h:
                raise ValueError('source snapshot corrupt')
        if receipt.get('gates')!={name:True for name in GATES}:
            raise ValueError('missing or failed software gates')
        validate_cases(receipt['cases'])
        for n in CUTOFFS:
            if replay:
                b,ops=base.prepare_basis(n,'cpu'); codes=q.oscillator_codes(n+4)
            for c in (x for x in receipt['cases'] if x['cutoff']==n):
                expected=f"N{n}-g2-{c['g2']:g}-alpha-{c['alpha']:g}.npz"
                if c['arrays']!=expected:
                    raise ValueError('array identity mismatch')
                if replay:
                    replay_case(folder,c,b,ops,codes)
                elif base.digest(folder/c['arrays'])!=c['arrays_sha256']:
                    raise ValueError('array integrity failure')
            if replay:
                del b,ops,codes; q.build_basis.cache_clear(); gc.collect()
        result=dict(status='PASS' if replay else 'INTEGRITY_ONLY',verified_cases=6,
                    source_hashes=receipt['source_hashes'],scientific_status='UNRESOLVED',
                    separation_verified=False,certified=False)
        base.atomic(folder/'verification.json',result)
        return result
    except BaseException as exc:
        base.atomic(folder/'verification.json',dict(status='FAIL',error=f'{type(exc).__name__}: {exc}',
                                                   scientific_status='UNRESOLVED'))
        raise


def validate_cases(cases):
    expected=[(n,g,a) for n in CUTOFFS for g,a in CONTROLS]
    if [(c.get('cutoff'),c.get('g2'),c.get('alpha')) for c in cases]!=expected:
        raise ValueError('incomplete, reordered, duplicated or unregistered controls')
    return True


def supervise(process, folder):
    import psutil
    start=time.monotonic(); peak=0
    try:
        with (folder/'resources.csv').open('w',newline='',encoding='utf-8') as f:
            writer=csv.writer(f); writer.writerow(['utc','elapsed_seconds','rss_bytes','cpu_seconds'])
            while process.poll() is None:
                try:
                    p=psutil.Process(process.pid); rss=p.memory_info().rss; cpu=p.cpu_times()
                except psutil.NoSuchProcess:
                    break
                elapsed=time.monotonic()-start; peak=max(peak,rss)
                writer.writerow([datetime.now(timezone.utc).isoformat(),elapsed,rss,cpu.user+cpu.system]); f.flush()
                if rss>base.HOST_BUDGET_BYTES or elapsed>base.TIME_BUDGET_SECONDS:
                    process.kill(); process.wait()
                    failure(folder,'RESOURCE_STOP','host memory or elapsed-time budget reached')
                    receipt=base.read_json(folder/'receipt.json'); receipt['peak_host_rss_bytes']=peak
                    base.atomic(folder/'receipt.json',receipt)
                    return 2
                time.sleep(.5)
        code=process.wait()
        receipt=base.read_json(folder/'receipt.json')
        receipt['peak_host_rss_bytes']=peak; receipt['supervised_seconds']=time.monotonic()-start
        base.atomic(folder/'receipt.json',receipt)
        if code or receipt['status']!='COMPLETE':
            failure(folder,'FAILED',receipt.get('error') or f'worker exited {code}')
            return code or 2
        return 0
    except BaseException as exc:
        process.kill(); process.wait(); failure(folder,'FAILED',f'supervisor interrupted: {exc}'); raise


def prerequisites(folder):
    import importlib
    import unittest
    modules={'ordering_tests':'test_quantum_ordering','runner_tests':'test_quantum_order_controls',
             'original_tests':'test_ym_helike_quantum','amendment_tests':'test_ym_helike_convergence'}
    gates={}
    for name,module in modules.items():
        event(folder,'test_gate',gate=name)
        with (folder/(name+'.txt')).open('w',encoding='utf-8') as f:
            result=unittest.TextTestRunner(stream=f,verbosity=2).run(
                unittest.defaultTestLoader.loadTestsFromModule(importlib.import_module(module)))
        gates[name]=result.wasSuccessful() and result.testsRun>0
        if not gates[name]:
            raise ValueError(name+' failed')
    import check_quantum_order_mutations as m
    results=m.run(); base.atomic(folder/'mutations.json',results)
    gates['mutations']=bool(results) and all(r['assertion_rejected'] for r in results)
    if not gates['mutations']:
        raise ValueError('mutation gate failed')
    return gates


def worker(folder):
    import torch
    from threadpoolctl import threadpool_limits
    receipt=base.read_json(folder/'receipt.json')
    try:
        if receipt['source_hashes']!=hashes():
            raise ValueError('source drift before worker startup')
        if not torch.cuda.is_available():
            raise RuntimeError('CUDA unavailable; no fallback')
        torch.set_num_threads(1)
        torch.cuda.set_per_process_memory_fraction(min(1.,base.GPU_BUDGET_BYTES/torch.cuda.get_device_properties(0).total_memory))
        torch.cuda.reset_peak_memory_stats()
        receipt['gpu']=dict(name=torch.cuda.get_device_name(0),torch=torch.__version__,cuda=torch.version.cuda)
        event(folder,'startup',gpu=receipt['gpu'])
        with threadpool_limits(limits=1):
            receipt['gates']=prerequisites(folder); base.atomic(folder/'receipt.json',receipt)
            q.build_basis.cache_clear(); q.independent_reference.cache_clear(); gc.collect()
            for n in CUTOFFS:
                event(folder,'basis',cutoff=n)
                b,ops=base.prepare_basis(n,'cuda'); codes=q.oscillator_codes(n+4)
                for g,a in CONTROLS:
                    started=time.monotonic(); result=measure(b,ops,g,a,'cuda',folder,codes)
                    result['seconds']=time.monotonic()-started
                    receipt['cases'].append(result); base.atomic(folder/'receipt.json',receipt)
                    with (folder/'cases.jsonl').open('a',encoding='utf-8') as f:
                        f.write(json.dumps(result,allow_nan=False)+'\n'); f.flush(); os.fsync(f.fileno())
                    event(folder,'case_complete',cutoff=n,g2=g,order=result['order']['conditional_order'],
                          gpu_allocated_bytes=torch.cuda.memory_allocated(),
                          gpu_peak_bytes=torch.cuda.max_memory_allocated())
                del b,ops,codes; q.build_basis.cache_clear(); gc.collect()
            receipt['status']='COMPLETE'
            event(folder,'final_replay')
            verify_run(folder,candidate=receipt)
        if hashes()!=receipt['source_hashes']:
            raise ValueError('source drift during run')
        receipt['peak_gpu_allocated_bytes']=torch.cuda.max_memory_allocated()
        receipt['peak_gpu_reserved_bytes']=torch.cuda.max_memory_reserved()
        if receipt['peak_gpu_reserved_bytes']>base.GPU_BUDGET_BYTES:
            raise MemoryError('GPU budget exceeded')
        base.atomic(folder/'receipt.json',receipt)
        event(folder,'complete',scientific_status='UNRESOLVED')
        return 0
    except BaseException as exc:
        failure(folder,'FAILED',f'{type(exc).__name__}: {exc}')
        raise


def main():
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--run',action='store_true')
    ap.add_argument('--worker',type=Path)
    ap.add_argument('--verify',type=Path)
    args=ap.parse_args()
    if args.worker:
        return worker(args.worker)
    if args.verify:
        from threadpoolctl import threadpool_limits
        with threadpool_limits(limits=1):
            print(json.dumps(verify_run(args.verify),indent=2))
        return 0
    if not args.run:
        ap.error('explicit --run required; no implicit numerical work')
    if subprocess.check_output(['git','status','--porcelain'],cwd=ROOT,text=True).strip():
        raise ValueError('commit and push the implementation before the control run')
    run_id=datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ')+'-'+uuid.uuid4().hex[:8]
    folder=ROOT/'research/quantum-order-controls-20261123'/('controls-'+run_id)
    folder.mkdir(parents=True,exist_ok=False)
    receipt=dict(status='RUNNING',mode='EXISTING_CONTROLS',scientific_status='UNRESOLVED',run_id=run_id,
                 source_revision=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
                 source_hashes=hashes(),gates={},cases=[],budgets=dict(host_bytes=base.HOST_BUDGET_BYTES,
                 gpu_bytes=base.GPU_BUDGET_BYTES,seconds=base.TIME_BUDGET_SECONDS))
    base.atomic(folder/'receipt.json',receipt)
    try:
        for name,h in receipt['source_hashes'].items():
            target=folder/'source'/name; target.parent.mkdir(parents=True,exist_ok=True); shutil.copyfile(ROOT/name,target)
            if base.digest(target)!=h:
                raise ValueError('source changed while snapshotting')
        print(str(folder),flush=True)
        env=dict(os.environ,OMP_NUM_THREADS='1',OPENBLAS_NUM_THREADS='1',MKL_NUM_THREADS='1')
        with (folder/'worker.log').open('w',encoding='utf-8') as log:
            proc=subprocess.Popen([sys.executable,str(Path(__file__).resolve()),'--worker',str(folder)],
                                  cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT)
            return supervise(proc,folder)
    except BaseException as exc:
        failure(folder,'FAILED',f'launch or supervision failed: {exc}')
        raise


if __name__=='__main__':
    raise SystemExit(main())
