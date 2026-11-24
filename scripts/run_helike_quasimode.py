"""Four-control numerical quasimode audit. Never emits a physical certificate or releases a grid."""
import argparse
import gc
import json
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import time
import numpy as np
import torch
import helike_quasimode as qm
import helike_positive_cone as pc
import helike_berger_model as bm
from run_helike_flux_background import save_receipt,sha,write
from run_helike_cone_convergence import SOURCES as BASE_SOURCES

ROOT=Path(__file__).resolve().parents[1]
REG='docs/research/helike-quasimode-construction-registration-2026-11-24.md'
SOURCES=tuple(BASE_SOURCES)+(REG,'scripts/helike_quasimode.py','scripts/run_helike_quasimode.py',
                            'scripts/test_helike_quasimode.py','scripts/check_helike_quasimode_mutations.py')


def utc(): return time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime())


def event(out,name,**data):
    row=dict(utc=utc(),event=name,**data)
    with (out/'events.jsonl').open('a',encoding='utf-8') as f:
        f.write(json.dumps(row,allow_nan=False)+'\n'); f.flush()
    print(json.dumps(row,allow_nan=False),flush=True)


def select(solver,e,v,result):
    wh=np.sqrt(bm.mc_helike_freq2(solver.s,solver.t))
    modes=np.flatnonzero(abs(solver.w-wh)<1e-6)
    amplitudes=solver.reference_amplitudes(v,12)
    weights=np.sum(amplitudes[modes]**2,axis=0)
    pair=sorted((1+np.argsort(weights[1:])[-2:]).tolist())
    if len(modes)!=2 or min(weights[pair])<.95 or result['helike']['level_index'] not in pair:
        raise ValueError('unresolved Helike partner identification')
    comp=result['competitor']['level_index']; indices=[0,*pair,comp]
    if len(set(indices))!=4 or result['competitor']['weight']<.95: raise ValueError('ambiguous state selection')
    coeff=v[:,indices]
    centers=e[indices]+solver.energy_shift
    centers[1:3]=centers[1:3].mean()
    return coeff,centers,indices,weights[pair].tolist()


def oracle_checks(solver,coeff,C):
    chart=solver.chart; base=chart.factor(np.zeros(6))
    points=[('background',base),('offset_a',chart.factor(np.array([.1,-.1,.05,.07,-.09,.04]))),
            ('offset_b',chart.factor(np.array([-.12,.08,.09,-.06,.1,-.03]))),
            ('determinant_shell',base*(.015**(1/6))),('outer_shell',base*np.sqrt(6.))]
    physical=coeff@C; rows=[]
    for label,q in points:
        for j in range(4):
            for cutoff in (False,True):
                actual=qm.point_action(q,chart,solver.basis,solver.w,physical[:,j],solver.s,solver.t,solver.g,cutoff)
                oracle=qm.cartesian_action(q,chart,solver.basis,solver.w,physical[:,j],solver.s,solver.t,solver.g,cutoff)
                error=abs(actual-oracle); limit=1e-9*(1+abs(oracle))
                if not np.isfinite([actual,oracle]).all() or error>limit: raise ArithmeticError('Cartesian oracle disagreement')
                rows.append(dict(point=label,state=j,cutoff=cutoff,reduced=[actual.real,actual.imag],
                                 cartesian=[oracle.real,oracle.imag],error=error,limit=limit))
    return rows


def main():
    ap=argparse.ArgumentParser(description=__doc__); ap.add_argument('out_dir'); args=ap.parse_args()
    out=Path(args.out_dir)
    if out.exists() and any(out.iterdir()): raise ValueError('refusing nonempty output folder')
    out.mkdir(parents=True,exist_ok=True)
    rec=dict(execution='RUNNING',status='UNRESOLVED_INTEGRATION',certified=False,full_grid_released=False,
             source_revision=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
             source_dirty=bool(subprocess.check_output(['git','status','--porcelain','--',*SOURCES],cwd=ROOT,text=True).strip()),
             sources={s:sha(ROOT/s) for s in SOURCES},registration=REG,started_utc=utc(),controls=[],
             design=dict(s=1.15,t=3.,g2=[.004,.002],Mq=[6,8],orders=[10,12],delta=.01,R_factor=4.),
             python=sys.version,platform=platform.platform(),torch=torch.__version__,numpy=np.__version__,
             missing_gates=['validated_integral_enclosures','separation_and_target_identification','method_amendment'])
    path=out/'receipt.json'; save_receipt(path,rec)
    torch.set_num_threads(1)
    try:
        if rec['source_dirty']: raise ValueError('commit frozen construction before control measurement')
        for source in SOURCES:
            dest=out/'sources'/source; dest.parent.mkdir(parents=True,exist_ok=True); shutil.copyfile(ROOT/source,dest)
            if sha(dest)!=rec['sources'][source]: raise ValueError('source changed while snapshotting')
        for g2 in (.004,.002):
            for m in (6,8):
                start=time.monotonic(); deadline=start+900
                event(out,'control_start',g2=g2,Mq=m)
                solver=pc.ConeRitz(1.15,3.,np.sqrt(g2),Mq=m,max_Mq=8)
                original=solver.helike_and_competitor(); e,v=solver.spectrum()
                coeff,centers,indices,weights=select(solver,e,v,original)
                row=dict(g2=g2,Mq=m,s=1.15,t=3.,original=original,indices=indices,centers=centers.tolist(),
                         pair_weights=weights,integrals={},oracle=[])
                rec['controls'].append(row); save_receipt(path,rec)
                C=None
                for order in qm.ORDERS:
                    event(out,'integration_start',g2=g2,Mq=m,order=order,points=order**6)
                    samples=qm.sample_grid(solver,coeff,order,deadline)
                    if C is None:
                        C,charges=qm.chiral_frame(samples); row['compressed_charges']=charges
                        filename=f'states-g2-{g2}-Mq-{m}.npz'
                        np.savez_compressed(out/filename,coeff=coeff,centers=centers,mixing=C,basis=np.array(solver.basis),
                                            w=solver.w,G0=solver.chart.G0,M=solver.chart.M,S=solver.chart.S,eigenvalues=e)
                        import hashlib
                        row.update(state_file=filename,state_sha256=hashlib.sha256((out/filename).read_bytes()).hexdigest())
                    row['integrals'][str(order)]=qm.summarize(samples,C,centers)
                    del samples; gc.collect(); save_receipt(path,rec)
                    event(out,'integration_complete',g2=g2,Mq=m,order=order,integrals=row['integrals'][str(order)])
                row['integration_checks']={name:qm.integration_status(row['integrals']['10'][name],row['integrals']['12'][name])
                                           for name in row['integrals']['12']}
                row['projection_estimates']={}
                for name,values in row['integrals']['12'].items():
                    try:
                        estimate=qm.projection_bound(*[np.sqrt(values[k]) for k in ('tail_sq','charge_sq','residual_sq','leakage_sq')])
                        row['projection_estimates'][name]=dict(value=estimate,bound_inputs_enclosed=False)
                    except ValueError as exc:
                        row['projection_estimates'][name]=dict(value=None,bound_inputs_enclosed=False,refusal=str(exc))
                event(out,'oracle_start',g2=g2,Mq=m)
                row['oracle']=oracle_checks(solver,coeff,C)
                if time.monotonic()>deadline: raise TimeoutError('per-rung audit budget exhausted')
                row['seconds']=time.monotonic()-start; save_receipt(path,rec)
                event(out,'control_complete',g2=g2,Mq=m,seconds=row['seconds'],status='UNRESOLVED_INTEGRATION')
                del solver,e,v,coeff; gc.collect()
        rec.update(execution='COMPLETE_NUMERICAL_AUDIT',finished_utc=utc()); save_receipt(path,rec)
        event(out,'complete',status=rec['status'],full_grid_released=False)
        return 0
    except (Exception,KeyboardInterrupt) as exc:
        rec.update(execution='FAILED',error=f'{type(exc).__name__}: {exc}',finished_utc=utc()); save_receipt(path,rec)
        event(out,'refusal',error=rec['error'],full_grid_released=False)
        return 2


if __name__=='__main__': sys.exit(main())
