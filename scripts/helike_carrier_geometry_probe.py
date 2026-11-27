"""Bounded ground-weighted probe of saved states; never changes the registered grid gate."""
import argparse
import json
import math
import subprocess
import sys
from pathlib import Path
import time
import numpy as np
import helike_m8_geometry as gm
import helike_positive_cone as pc
import run_helike_flux_grid as grid
from run_helike_flux_support import decode

ROOT=Path(__file__).resolve().parents[1]
SOURCE=ROOT/'research/flux-support-20261126/support.json'
PARENT=ROOT/'research/flux-support-20261126/receipt.json'
SOURCE_SHA='15ed4b72a41ab4f1125fbe3a3bb3b6f35775c0ef077f1fc3f8b8f1a8b3969833'
PARENT_SHA='b790b53f6186b6d3b1767c91cc1f087486d2db98c2bd291fce14db40524210a6'


def slabs(basis,coeff,order,mq,derivatives=True):
    coeff=np.asarray(coeff)
    if coeff.ndim!=2 or len(coeff)!=len(basis) or not np.isfinite(coeff).all():
        raise ValueError('invalid coefficients')
    x,w=gm.product_grid(order); h,dh=gm.hermite_tables(x,mq)
    T=np.zeros((mq+1,)*6+(coeff.shape[1],))
    for n,c in zip(basis,coeff): T[tuple(n)]=c
    ix=np.array(np.unravel_index(np.arange(order**5),(order,)*5)).T
    tail=x[ix]; Wtail=np.prod(w[ix],axis=1)
    for q0 in range(order):
        def contract(derivative):
            first=dh[:,q0] if derivative==0 else h[:,q0]
            v=np.tensordot(first,T,axes=(0,0))
            for axis in range(5):
                table=dh if derivative==axis+1 else h
                v=np.moveaxis(np.tensordot(table.T,v,axes=(1,axis)),0,axis)
            return v.reshape(order**5,coeff.shape[1])
        p=contract(-1)
        dp=np.stack([contract(j) for j in range(6)],axis=1) if derivatives else None
        yield w[q0]*Wtail,np.column_stack([np.full(len(tail),x[q0]),tail]),p,dp

def combine(parts):
    if not parts: raise ValueError('missing slabs')
    result={k:sum(p[k] for p in parts) for k in ('D','R','norm','kinetic','potential','nonpositive_weight')}
    cuts=parts[0]['D_by_cut']; blocks=parts[0]['shares']
    result['D_by_cut']={c:sum(p['D_by_cut'][c] for p in parts) for c in cuts}
    if any(d<=0 or not math.isfinite(d) for d in result['D_by_cut'].values()):
        raise ValueError('no resolved whole-domain energy')
    result['shares_by_cut']={c:{b:sum(p['shares_by_cut'][c][b]*p['D_by_cut'][c] for p in parts)/result['D_by_cut'][c]
                               for b in blocks} for c in cuts}
    result['shares']=result['shares_by_cut'][next(iter(cuts))]
    result['identity']=result['D']+result['R']
    result['raw_sum']=sum((p['raw_sum'] or 0.)*p['D_by_cut'][next(iter(cuts))] for p in parts)/result['D_by_cut'][next(iter(cuts))]
    result['share_sum']=sum((p['share_sum'] or 0.)*p['D_by_cut'][next(iter(cuts))] for p in parts)/result['D_by_cut'][next(iter(cuts))]
    return result


def load_source():
    if grid.sha(SOURCE)!=SOURCE_SHA or grid.sha(PARENT)!=PARENT_SHA: raise ValueError('changed pinned source')
    old=json.loads(SOURCE.read_text())
    if (old['s'],old['t'],old['g2'],old['mq'])!=(1.18,3.,.03,12): raise ValueError('changed point')
    u=np.array(old['chart_origin']); S=np.array(old['chart_matrix']); r=np.exp(u[:3])
    L=np.array([[r[0],0.,0.],[u[3],r[1],0.],[u[4],u[5],r[2]]])
    M=pc.ConeChart.jacobian_u(L)@S
    coeff=decode([old['states'][0]['coeff_bits'],old['states'][3]['coeff_bits']]).T.copy()
    if coeff.shape!=(18564,2) or np.max(np.abs(coeff.T@coeff-np.eye(2)))>1e-8: raise ValueError('invalid saved states')
    zero=old['basis'].index([0]*6)
    if coeff[zero,0]==0: raise ValueError('ground phase unresolved')
    coeff[:,0]*=np.sign(coeff[zero,0])
    X=dict(u0=u,S=S,G0=L@L.T,M=M,w=np.array(old['frequencies']),basis=old['basis'],coeff=coeff,
           levels=old['identification']['levels'],roles=old['roles'],shift=old['identification']['energy_shift'],
           mq=12,s=old['s'],t=old['t'],g2=old['g2'],raw=old)
    return X


def positivity(X,order,event):
    total=float(X['coeff'][:,0]@X['coeff'][:,0]); norms=np.zeros(2); bad=0.; count=0; maximum=0.; example=None
    for j,(W,x,p,_) in enumerate(slabs(X['basis'],X['coeff'],order,X['mq'],False)):
        mass=W*p[:,0]**2; mask=(mass>1e-12*total)&(p[:,0]<=0)
        norms+=np.sum(W[:,None]*p*p,axis=0); bad+=float(mass[mask].sum()); count+=int(mask.sum())
        if mask.any():
            k=int(np.argmax(np.where(mask,mass,0)))
            if mass[k]>maximum:
                maximum=float(mass[k]); example=dict(x=x[k].tolist(),ground_polynomial=float(p[k,0]),vacuum_node_weight=maximum)
        event(dict(stage='positivity',order=order,slab=j+1,slabs=order,bad_nodes=count,bad_weight=bad))
    if np.max(abs(norms-1))>1e-8: raise ValueError('slab norms disagree with saved states')
    return dict(norms=norms.tolist(),bad_nodes=count,bad_weight=bad,largest_bad_node=example,passed=count==0)


def geometry_order(X,order,event):
    P=gm.Point(X,order,nodes=(np.ones(1),np.zeros((1,6))))
    E0=float(X['levels'][0]); parts=[]; moments=np.zeros(7)
    total=float(X['coeff'][:,0]@X['coeff'][:,0]); c=X['roles'][2]
    for j,(W,x,p,dp) in enumerate(slabs(X['basis'],X['coeff'],order,X['mq'])):
        core=gm.Core(W,x,X['w'],P.metric,P.chart.ell,P.potential,P.ycoords,P.dycoords)
        part=core.analyse(p[:,0],dp[:,:,0],p[:,1],dp[:,:,1],E0,P.blocks,P.A,total_vacuum_norm=total)
        if not all(math.isfinite(part[k]) for k in ('D','R','norm','kinetic','potential')): raise ValueError('nonfinite geometry')
        parts.append(part)
        for sl in core.chunks():
            y=P.ycoords(core.z[sl])[:,c]; w=W[sl]; g=p[sl,0]; v=p[sl,1]
            moments+=np.array([np.sum(w*g*g),np.sum(w*g*g*y),np.sum(w*g*g*y*y),np.sum(w*g*v),
                               np.sum(w*g*v*y),np.sum(w*v*v),np.sum(w)])
        event(dict(stage='geometry',order=order,slab=j+1,slabs=order,D=part['D'],residual_term=part['R']))
    result=combine(parts)
    n0,mean,yy,gv,yv,nv,_=moments
    gram=yy-mean*mean/n0
    if gram<=0 or not math.isfinite(gram): raise ValueError('singular ground-weighted coordinate')
    result['alignment']=(yv-mean*gv/n0)**2/(gram*nv)
    gap=X['raw']['identification']['competitor']['gap']
    result.update(identity_rel_error=abs(result['identity']-gap)/gap,
                  residual_share=abs(result['R'])/gap,
                  excluded_fraction={c:1-d/result['D'] for c,d in result['D_by_cut'].items()})
    return result


def decide(readings,reference):
    if set(readings)!={'14','16'}: raise ValueError('missing registered quadrature order')
    for row in readings.values():
        values=[row[k] for k in ('identity_rel_error','alignment','raw_sum','share_sum','norm','nonpositive_weight')]
        if set(row['shares'])!=set(gm.BLOCKS) or set(row['shares_by_cut'])!={'1e-12','1e-14'}:
            raise ValueError('missing geometry blocks or fixed cuts')
        values.extend(row['shares'].values())
        for shares in row['shares_by_cut'].values():
            if set(shares)!=set(gm.BLOCKS): raise ValueError('missing cut shares')
            values.extend(shares.values())
        if not all(math.isfinite(v) for v in values): raise ValueError('nonfinite decision evidence')
    a,b=readings['14'],readings['16']
    seam=max(abs(a['shares'][k]-b['shares'][k]) for k in gm.BLOCKS)
    cut=max(abs(b['shares_by_cut']['1e-12'][k]-b['shares_by_cut']['1e-14'][k]) for k in gm.BLOCKS)
    vals=[seam,cut,b['identity_rel_error'],b['alignment'],reference,b['raw_sum'],b['share_sum'],b['norm'],b['nonpositive_weight'],*b['shares'].values()]
    if not all(math.isfinite(v) for v in vals): raise ValueError('nonfinite decision evidence')
    gates=dict(identity=b['identity_rel_error']<=1e-4,order_seam=seam<=1e-3,cut_seam=cut<=1e-3,
               alignment=abs(b['alignment']-reference)<=1e-6,positivity=all(r['nonpositive_weight']==0 for r in readings.values()),
               sums=abs(b['raw_sum']-1)<=1e-10 and abs(b['share_sum']-1)<=1e-10,
               norm=all(abs(r['norm']-1)<=1e-8 for r in readings.values()))
    aligned=b['shares']['C']>=.5 and max(b['shares'],key=b['shares'].get)=='C'
    return dict(status=('RESOLVED_ALIGNED' if aligned else 'RESOLVED_NOT_ALIGNED') if all(gates.values()) else 'OPEN',
                gates=gates,order_seam=seam,cut_seam=cut,own_share=b['shares']['C'],alignment=b['alignment'],
                old_export_gate='FAIL',full_grid_released=False)


def main():
    ap=argparse.ArgumentParser(); ap.add_argument('out',type=Path); args=ap.parse_args()
    out=args.out.resolve(); out.mkdir(parents=True,exist_ok=False)
    rec=dict(status='RUNNING',old_export_gate='FAIL',full_grid_released=False,positivity={},orders={})
    start=time.monotonic()
    def save(): grid.save(out/'receipt.json',rec)
    def event(e):
        e['seconds']=time.monotonic()-start
        with (out/'events.jsonl').open('a') as f: f.write(json.dumps(e)+'\n')
        print(json.dumps(e),flush=True)
        if e['seconds']>900: raise TimeoutError('registered resource limit')
    save()
    try:
        X=load_source()
        rec.update(input_sha256_lf=SOURCE_SHA,parent_sha256_lf=PARENT_SHA,
                   source_revision=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
                   source_dirty=bool(subprocess.check_output(['git','status','--porcelain','--','scripts'],cwd=ROOT,text=True).strip()),
                   command=sys.argv,python=sys.version,numpy=np.__version__,
                   sources_sha256_lf={p:grid.sha(ROOT/p) for p in ('scripts/helike_carrier_geometry_probe.py','scripts/helike_m8_geometry.py','scripts/helike_positive_cone.py')},
                   point=dict(s=1.18,t=3.,g2=.03,mq=12),seam_orders=[14,16],cuts=[1e-12,1e-14],blas_threads=4)
        save()
        for order in (14,16):
            rec['positivity'][str(order)]=positivity(X,order,event); save()
        if any(not r['passed'] for r in rec['positivity'].values()):
            rec.update(status='OPEN',reason='saved ground state violates registered positivity; ratio geometry not evaluated')
        else:
            for order in (14,16):
                rec['orders'][str(order)]=geometry_order(X,order,event); save()
            rec['decision']=decide(rec['orders'],X['raw']['identification']['competitor']['dressed_overlap'])
            rec['status']=rec['decision']['status']
        if grid.sha(SOURCE)!=SOURCE_SHA: raise ValueError('source changed during probe')
    except BaseException as error:
        rec.update(status='OPEN',error=f'{type(error).__name__}: {error}')
    finally:
        rec['seconds']=time.monotonic()-start; save()
    print(json.dumps({k:v for k,v in rec.items() if k not in ('orders','sources_sha256_lf')},indent=2),flush=True)
    return 0 if rec['status']=='RESOLVED_ALIGNED' else 2


if __name__=='__main__': raise SystemExit(main())
