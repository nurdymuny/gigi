"""One frozen residual-guided enrichment, never a grid release."""
import argparse
import json
import math
import os
from pathlib import Path
import subprocess
import time
import numpy as np
import run_helike_flux_grid as grid
from run_helike_flux_support import decode,measure

ROOT=Path(__file__).resolve().parents[1]
SOURCE=ROOT/'research/flux-support-20261126/support.json'
PARENT=ROOT/'research/flux-support-20261126/receipt.json'
SOURCE_SHA='15ed4b72a41ab4f1125fbe3a3bb3b6f35775c0ef077f1fc3f8b8f1a8b3969833'
PARENT_SHA='b790b53f6186b6d3b1767c91cc1f087486d2db98c2bd291fce14db40524210a6'
SETTINGS={'nev':12,'block':4,'tol':1e-10,'max_dim':1600}


def require(condition,message):
    if not condition: raise ValueError(message)


def close(a,b,message):
    aa,bb=np.asarray(a),np.asarray(b)
    require(aa.shape==bb.shape and np.isfinite(aa).all() and np.isfinite(bb).all()
            and np.allclose(aa,bb,rtol=1e-10,atol=1e-12),message)


def geometry(positions,columns,vectors,residuals,inside,outside):
    u,v,r=map(lambda x: np.asarray(x,dtype=float),(columns,vectors,residuals))
    require(u.ndim==v.ndim==r.ndim==2,'matrix dimensions')
    p=np.asarray(positions)
    require(len(p)>0 and np.issubdtype(p.dtype,np.integer) and len(set(positions))==len(p)
            and (p>=0).all() and (p<u.shape[1]).all(),'invalid retained positions')
    require(v.shape[1]==len(p)+len(u) and r.shape[1]==u.shape[1],'trial/ambient dimensions')
    require(all(np.isfinite(x).all() for x in (u,v,r)),'nonfinite geometry')
    require(np.count_nonzero(u[:,p])==0,'directions contaminate retained space')
    require(np.max(np.abs(u@u.T-np.eye(len(u))))<=1e-10,'nonorthonormal directions')
    require(np.max(np.abs(v@v.T-np.eye(len(v))))<=1e-8,'nonorthonormal Ritz states')
    projection=(r@u.T)@u
    projection[:,p]=r[:,p]
    a=np.linalg.norm(projection,axis=1)
    b=np.linalg.norm(r-projection,axis=1)
    close(a,inside,'forged inside residual')
    close(b,outside,'forged outside residual')
    close(a*a+b*b,np.sum(r*r,axis=1),'residual Pythagorean identity')
    return {'inside':a.tolist(),'outside':b.tolist()}


def assessment(old,new):
    result={'overall':'UNRESOLVED','certified':False,'full_grid_released':False,'domain_gate':'NOT_RUN'}
    for name in ('helike','competitor'):
        before,after,w=old[name]['gap'],new[name]['gap'],new[name]['dressed_weight']
        require(all(math.isfinite(x) for x in (before,after,w)),'nonfinite assessment')
        step=abs(after-before)
        result[name]={'old_gap':before,'new_gap':after,'absolute_step':step,'w_over_Z':w,
                      'step_gate':'PASS' if step<=.003 else 'FAIL',
                      'grid_weight_gate':'PASS' if w>=.9 else 'FAIL',
                      'export_weight_necessary_gate':'PASS' if w>=.95 else 'FAIL'}
    return result


def inspect(x,old):
    expected={'schema':'helike-lab/cone-enrichment/v1','s':1.18,'t':3.,'g2':.03,'base_mq':12,'probe_mq':14,
              'base_dimension':18564,'dimension':18568,'ambient_dimension':38760,
              'domain_gate':'NOT_RUN','identification_residual_space':'W'}
    require(all(x[k]==value for k,value in expected.items()),'changed registered contract')
    require(x['certified'] is False and x['full_grid_released'] is False,'diagnostic cannot certify or release')
    require(x['solver']==SETTINGS,'changed solver settings')
    q=x['identification']; levels=np.asarray(q['levels']); rs=np.asarray(q['in_space_lanczos_residuals'])
    require(levels.shape==rs.shape==(12,) and np.isfinite(levels).all() and np.isfinite(rs).all(),'missing solve evidence')
    require((np.diff(levels)>=0).all() and (levels<=np.asarray(old['identification']['levels'])+1e-8).all(),'nested variational sanity check')
    require((rs>=0).all() and (rs<=1e-10*np.maximum(1,np.abs(levels))).all(),'unresolved finite solve')
    require(math.isfinite(q['reference_weight_error']) and 0<=q['reference_weight_error']<=1e-6,'unresolved identification')
    close(q['E0'],levels[0],'ground level mismatch')
    for name in ('helike','competitor'):
        channel=q[name]; idx=channel['level_index']
        require(type(idx) is int and 0<idx<12,'invalid channel index')
        close(channel['gap'],levels[idx]-levels[0],'gap differs from levels')
        close(channel['dressed_weight'],channel['weight']/q['vacuum_weight'],'w/Z mismatch')
    errors=np.asarray(x['input_action_errors']); block_errors=np.asarray(x['block_action_relative_errors'])
    require(errors.shape==(4,) and np.isfinite(errors).all() and (errors>=0).all()
            and (errors<=1e-10*np.maximum(1,[abs(s['energy']) for s in old['states']])).all(),'input replay failure')
    require(block_errors.shape==(3,) and np.isfinite(block_errors).all() and (block_errors>=0).all()
            and (block_errors<=1e-9).all(),'operator projection failure')
    p=x['lift_positions']; index={tuple(n):i for i,n in enumerate(old['probe_basis'])}
    require(p==[index[tuple(n)] for n in old['basis']],'occupation identity lost')
    u=decode(x['lift_columns_bits']); v=decode(x['vectors_bits'])
    require(u.shape==(4,38760) and v.shape==(12,18568),'changed enrichment size')
    states=x['states']; ids=[s['level_index'] for s in states]
    require([s['name'] for s in states]==['ground','helike_a','helike_b','competitor'] and len(set(ids))==4
            and ids[0]==0 and ids[3]==q['competitor']['level_index'] and q['helike']['level_index'] in ids[1:3]
            and all(type(i) is int and 0<=i<12 for i in ids),'wrong selected states')
    r=decode([s['probe_residual_bits'] for s in states])
    geometry(p,u,v,r,[s['inside_norm'] for s in states],[s['outside_norm'] for s in states])
    all_inner=np.asarray(x['inside_via_ambient_norms']); all_outer=np.asarray(x['ambient_residual_norms'])
    require(all_inner.shape==all_outer.shape==(12,) and np.isfinite(all_inner).all() and np.isfinite(all_outer).all()
            and (all_inner>=0).all() and (all_outer>=0).all()
            and (all_inner<=1e-10*np.maximum(1,np.abs(levels))).all(),'ambient projection contradicts solve')
    expanded=v[:,len(p):]@u
    expanded[:,p]=v[:,:len(p)]
    old_vectors=np.zeros((4,38760))
    old_vectors[:,p]=decode([s['coeff_bits'] for s in old['states']])
    # Verify the added space is precisely the saved residual span, not four unrelated orthonormal vectors.
    old_r=decode([s['probe_residual_bits'] for s in old['states']]); old_r[:,p]=0.
    require(np.linalg.norm(old_r-(old_r@u.T)@u)<=1e-10,'added directions miss registered residual span')
    result=assessment(old['identification'],q)
    rows=[]
    for j,state in enumerate(states):
        idx=ids[j]
        close(state['energy'],levels[idx],'selected state energy mismatch')
        close(state['ambient_norm'],np.linalg.norm(r[j]),'ambient norm mismatch')
        close(state['ambient_norm'],all_outer[idx],'all-level norm mismatch')
        close(state['inside_norm'],all_inner[idx],'selected inside norm mismatch')
        baseline=old['states'][j]['outer_norm']
        rows.append({'name':state['name'],'level_index':idx,'old_outer_norm':baseline,
                     'new_complement_norm':state['outside_norm'],'ratio':state['outside_norm']/baseline,
                     'inside_norm':state['inside_norm'],
                     'probability':measure(old['probe_basis'],expanded[idx]**2,old['roles'])})
    pair_overlap=old_vectors[1:3]@expanded[ids[1:3]].T
    result.update(states=rows,ground_overlap_squared=float((old_vectors[0]@expanded[0])**2),
                  competitor_overlap_squared=float((old_vectors[3]@expanded[ids[3]])**2),
                  helike_pair_singular_values=np.linalg.svd(pair_overlap,compute_uv=False).tolist(),
                  helike_pair_residual_rms_ratio=float(np.linalg.norm([s['new_complement_norm'] for s in rows[1:3]])/
                                                      np.linalg.norm([s['old_outer_norm'] for s in rows[1:3]])),
                  max_block_action_relative_error=float(block_errors.max()),max_input_action_error=float(errors.max()),
                  max_in_space_residual=float(rs.max()),ambient_residual_norms=all_outer.tolist())
    return result


def main():
    ap=argparse.ArgumentParser(description=__doc__); ap.add_argument('out_dir',type=Path); args=ap.parse_args()
    out=args.out_dir.resolve(); out.mkdir(parents=True,exist_ok=False)
    rec={'status':'RUNNING','overall':'UNRESOLVED','certified':False,'full_grid_released':False,'domain_gate':'NOT_RUN'}
    ledger=out/'receipt.json'; grid.save(ledger,rec); start=time.time()
    try:
        require(grid.sha(SOURCE)==SOURCE_SHA and grid.sha(PARENT)==PARENT_SHA,'changed registered input')
        raw=out/'enriched.json'
        cmd=[str(grid.HELIKE_BIN),'--out',str(out/'native-receipt.json'),'cone-enrich','--support',str(SOURCE),'--states-out',str(raw)]
        rec.update(command=cmd,source_revision=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
                   binary_sha256=grid.raw_sha(grid.HELIKE_BIN),harness_sha256_lf=grid.sha(Path(__file__)),
                   input_sha256_lf=SOURCE_SHA,parent_sha256_lf=PARENT_SHA,threads=8,timeout_seconds=900)
        grid.save(ledger,rec)
        print('Starting ONE enriched solve, 18568 directions; full grid held.',flush=True)
        with (out/'stdout.log').open('w') as stdout,(out/'stderr.log').open('w') as stderr:
            proc=subprocess.run(cmd,cwd=ROOT,env={**os.environ,'RAYON_NUM_THREADS':'8'},stdout=stdout,stderr=stderr,timeout=900)
        rec['returncode']=proc.returncode
        require(proc.returncode==0,f'run exited {proc.returncode}; see stderr.log')
        native=json.loads((out/'native-receipt.json').read_text())
        require(native['status']=='COMPUTED' and native['params']['solver']==SETTINGS,'native receipt contract')
        require(native['params']['support_sha256_lf']==SOURCE_SHA and native['results']['enrichment_sha256_lf']==grid.sha(raw),'native digest mismatch')
        require(grid.sha(SOURCE)==SOURCE_SHA and grid.raw_sha(grid.HELIKE_BIN)==rec['binary_sha256'],'input or binary changed during run')
        rec['analysis']=inspect(json.loads(raw.read_text()),json.loads(SOURCE.read_text()))
        rec.update(status='DIAGNOSTIC_COMPLETE',enrichment_sha256_lf=grid.sha(raw),native_receipt_sha256_lf=grid.sha(out/'native-receipt.json'))
    except BaseException as exc:
        rec.update(status='STOPPED',error=f'{type(exc).__name__}: {exc}')
    finally:
        rec['elapsed_seconds']=time.time()-start; grid.save(ledger,rec)
    print(json.dumps(rec.get('analysis',{k:rec[k] for k in ('status','overall')}),indent=2),flush=True)
    if 'error' in rec: print(rec['error'],flush=True)
    return 0 if rec['status']=='DIAGNOSTIC_COMPLETE' else 2


if __name__=='__main__': raise SystemExit(main())
