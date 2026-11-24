"""Bounded finite-reference audit. Run OUT or replay OUT --verify; no eigensolves."""
import argparse
import json
import math
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import time
import traceback

from threadpoolctl import threadpool_limits
import ym_helike_dome_truncation as diagnostic
import run_ym_helike_dome as shared

ROOT = Path(__file__).resolve().parent.parent
SOURCES = shared.SOURCES + tuple('scripts/'+s for s in (
    'ym_helike_dome_truncation.py','test_ym_helike_dome_truncation.py',
    'test_ym_helike_second_vacuum.py','run_ym_helike_dome_local_review.py',
    'test_dome_local_review.py','check_dome_local_review_mutations.py')) + (
    'docs/research/dome-local-review-protocol-2026-11-24.md',)


def jobs():
    return [dict(N=n,g2=g,alpha=a,beta=b,orders=[24,30,36,42],K_ref=ref,dps=p)
            for g,a,b in ((1.,1.7,2.),(2.,2.3,3.))
            for n,ref,p in ((12,48,80),(18,48,80),(18,54,80),(18,48,120))]


def hashes():
    return {p:shared.sha(ROOT/p) for p in SOURCES}


def initialize(out):
    if out.exists() and any(out.iterdir()):
        raise ValueError('refusing non-empty output directory')
    out.mkdir(parents=True,exist_ok=True)
    rec = dict(status='READY',label='post-hoc finite-reference overlap audit',
        source_revision=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
        sources_sha256=hashes(),jobs=jobs(),batches=[],certified=False,
        started_utc=time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),
        python=sys.version,platform=platform.platform(),command=sys.argv,
        arithmetic='mpmath '+diagnostic.mp.__version__,
        budgets=dict(host_bytes=shared.HOST_LIMIT,seconds=shared.TIME_LIMIT))
    for p,h in rec['sources_sha256'].items():
        target=out/'source'/p
        target.parent.mkdir(parents=True,exist_ok=True)
        shutil.copyfile(ROOT/p,target)
        if shared.sha(target)!=h:
            raise ValueError('source drift during snapshot')
    shared.write(out/'receipt.json',rec)
    return rec


def precision(low,high):
    if len(low)!=len(high) or not low:
        raise ValueError('precision comparison requires complete rows')
    da=dr=0.
    for a,b in zip(low,high):
        for key in ('N','g2','alpha','beta','K','K_ref','m'):
            if a[key]!=b[key]:
                raise ValueError('precision metadata mismatch')
        for key in ('outside_fraction_of_dome','total','outside'):
            if not math.isfinite(a[key]) or not math.isfinite(b[key]):
                raise ValueError('nonfinite precision value')
            delta=abs(a[key]-b[key])
            da=max(da,delta); dr=max(dr,delta/max(abs(a[key]),1e-300))
    return dict(max_abs=da,max_rel=dr,passed=da<=1e-12 and dr<=1e-8)


def precision_summary(batches):
    out=[]
    for high in batches:
        spec=high['job']
        if spec['dps']!=120:
            continue
        lo_spec=dict(spec,dps=80)
        lows=[b for b in batches if b['job']==lo_spec]
        if len(lows)!=1:
            raise ValueError('missing precision comparator')
        out.append(dict(g2=spec['g2'],**precision(lows[0]['rows'],high['rows'])))
    if not out:
        raise ValueError('no precision checks')
    return out


def verify(out):
    shared.write(out/'verification.json',dict(status='RUNNING'))
    try:
        rec=json.loads((out/'receipt.json').read_text())
        if rec['status'] not in ('COMPLETE','INVALID') or rec['jobs']!=jobs() or rec['sources_sha256']!=hashes():
            raise ValueError('incomplete run, changed design or source drift')
        if [b['job'] for b in rec['batches']]!=jobs():
            raise ValueError('incomplete or duplicated batch coverage')
        for p,h in rec['sources_sha256'].items():
            if shared.sha(out/'source'/p)!=h:
                raise ValueError('changed source snapshot')
        for i,b in enumerate(rec['batches']):
            expected=f'batch-{i:03}.json'
            if b['file']!=expected or shared.sha(out/expected)!=b['sha256']:
                raise ValueError('changed batch evidence')
            payload=json.loads((out/expected).read_text())
            if payload['meta']!=dict(b['job'],m=[0,0,0]) or payload['rows']!=b['rows']:
                raise ValueError('batch provenance or reported rows differ')
            diagnostic.replay_batch(payload)
        checks=precision_summary(rec['batches'])
        status='COMPLETE' if all(c['passed'] for c in checks) else 'INVALID'
        if checks!=rec['precision_checks'] or status!=rec['status']:
            raise ValueError('precision verdict differs from replay')
        result=dict(status='PASS',replayed_batches=len(rec['batches']),
                    rows=sum(len(b['rows']) for b in rec['batches']),certified=False,
                    scope='saved overlap projection replay; no infinite-tail or physical-gap claim')
        shared.write(out/'verification.json',result)
        return result
    except BaseException as exc:
        shared.write(out/'verification.json',dict(status='FAIL',error=str(exc)))
        raise


def worker(out):
    rec=json.loads((out/'receipt.json').read_text())
    if rec['status']!='READY' or rec['jobs']!=jobs() or rec['sources_sha256']!=hashes():
        raise ValueError('worker requires fresh source-pinned initialization')
    rec['status']='RUNNING'; shared.write(out/'receipt.json',rec)
    try:
        for i,job in enumerate(jobs()):
            shared.event(out,'batch_start',index=i,job=job)
            partial=dict(job=job)
            def progress(phase,**data):
                partial[phase+str(data.get('K',''))]=data
                shared.write(out/f'partial-{i:03}.json',partial)
                shared.event(out,phase,index=i,K=data.get('K'))
            start=time.monotonic()
            batch=diagnostic.truncation_sweep(**job,progress=progress)
            filename=f'batch-{i:03}.json'
            shared.write(out/filename,batch)
            rec['batches'].append(dict(job=job,file=filename,sha256=shared.sha(out/filename),rows=batch['rows']))
            shared.write(out/'receipt.json',rec)
            shared.event(out,'batch_complete',index=i,seconds=time.monotonic()-start,rows=batch['rows'])
        rec['precision_checks']=precision_summary(rec['batches'])
        rec['status']='COMPLETE' if all(c['passed'] for c in rec['precision_checks']) else 'INVALID'
        rec['finished_utc']=time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime())
        shared.write(out/'receipt.json',rec)
        verify(out)
    except BaseException as exc:
        rec.update(status='FAILED',error=''.join(traceback.format_exception(type(exc),exc,exc.__traceback__)))
        shared.write(out/'receipt.json',rec)
        shared.write(out/'verification.json',dict(status='FAIL',error=str(exc)))
        raise
    return 0 if rec['status']=='COMPLETE' else 1


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('out_dir'); parser.add_argument('--worker',action='store_true')
    parser.add_argument('--verify',action='store_true')
    args=parser.parse_args(); out=Path(args.out_dir).resolve()
    if args.verify:
        print(json.dumps(verify(out))); return 0
    if args.worker:
        return worker(out)
    initialize(out)
    env=dict(os.environ,OMP_NUM_THREADS='1',OPENBLAS_NUM_THREADS='1',MKL_NUM_THREADS='1')
    with (out/'worker.log').open('w',encoding='utf-8') as log:
        process=subprocess.Popen([sys.executable,'-u',str(Path(__file__).resolve()),str(out),'--worker'],
                                 cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT)
        return shared.supervise(process,out)


if __name__=='__main__':
    with threadpool_limits(limits=1):
        raise SystemExit(main())
