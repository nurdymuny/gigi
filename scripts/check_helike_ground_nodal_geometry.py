"""Post-probe forensic check: independent Hermite evaluation, interior zero, and a labelled slice.

This is not a new acceptance gate or a repair of the saved state.
"""
import argparse
import json
import math
from pathlib import Path
import numpy as np
import scipy
from scipy.special import eval_hermite
from scipy.optimize import brentq
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import helike_carrier_geometry_probe as probe
import helike_m8_geometry as gm


def evaluate(X,points):
    basis=np.asarray(X['basis']); coeff=X['coeff']; points=np.atleast_2d(points)
    out=np.empty((len(points),2)); degrees=np.arange(X['mq']+1)
    scales=np.array([math.sqrt(2.**n*math.factorial(n)) for n in degrees])
    for lo in range(0,len(points),64):
        z=points[lo:lo+64]
        value=np.ones((len(z),len(basis)))
        for axis in range(6):
            tab=eval_hermite(degrees[:,None],z[:,axis][None,:])/scales[:,None]
            value*=tab[basis[:,axis],:].T
        out[lo:lo+len(z)]=value@coeff
    return out


def main():
    ap=argparse.ArgumentParser(); ap.add_argument('run_dir',type=Path); args=ap.parse_args()
    receipt=json.loads((args.run_dir/'receipt.json').read_text()); X=probe.load_source()
    if receipt['status']!='OPEN' or receipt['reason']!='saved ground state violates registered positivity; ratio geometry not evaluated':
        raise ValueError('not the registered positivity refusal')
    witnesses=[]
    for order in ('14','16'):
        w=receipt['positivity'][order]['largest_bad_node']; value=evaluate(X,w['x'])[0]
        error=abs(float(value[0])-w['ground_polynomial'])
        if error>1e-10 or value[0]>=0: raise ValueError('independent witness disagreement')
        witnesses.append(dict(order=int(order),ground_polynomial=float(value[0]),competitor_polynomial=float(value[1]),absolute_disagreement=error))
    target=np.array(receipt['positivity']['16']['largest_bad_node']['x'])
    c,r=X['roles'][2],X['roles'][3]
    base=target.copy(); base[[c,r]]=0.
    path=lambda t:base+t*(target-base)
    fn=lambda t:float(evaluate(X,path(t))[0,0])
    ts=np.linspace(0,1,65); vals=np.array([fn(t) for t in ts])
    intervals=[(ts[j],ts[j+1]) for j in range(64) if vals[j]>0 and vals[j+1]<0]
    if not intervals: raise ValueError('no positive-to-negative bracket on stated slice')
    root=brentq(fn,*intervals[0],xtol=1e-14)
    at_root=evaluate(X,path(root))[0]
    if abs(at_root[0])>1e-10 or abs(at_root[1])<1e-8: raise ValueError('root or noncancellation unresolved')
    P=gm.Point(X,14,nodes=(np.ones(1),np.zeros((1,6))))
    def matrix_at(x):
        u=X['u0']+X['S']@(x/np.sqrt(X['w'])); d=np.exp(u[:3])
        L=np.array([[d[0],0,0],[u[3],d[1],0],[u[4],u[5],d[2]]])
        return L@L.T
    G=matrix_at(path(root)); mineig=float(np.linalg.eigvalsh(G)[0])
    if mineig<=0: raise ValueError('witness is not in the physical cone')
    metric=P.metric((path(root)/np.sqrt(X['w']))[None])[0]
    metric_min=float(np.linalg.eigvalsh(metric)[0])
    if not np.isfinite(metric_min) or metric_min<=0: raise ValueError('metric is not positive at the witness')
    result=dict(scope='post-failure diagnostic, not a new gate',witnesses=witnesses,
                source_sha256_lf=probe.grid.sha(probe.SOURCE),
                probe_receipt_sha256_lf=probe.grid.sha(args.run_dir/'receipt.json'),
                evaluator_sha256_lf=probe.grid.sha(Path(__file__)),scipy=scipy.__version__,
                slice_fixed_coordinates=base.tolist(),varied_modes=[int(c),int(r)],root_parameter=float(root),
                root_bracket=list(intervals[0]),bracket_ground_values=[fn(t) for t in intervals[0]],
                root_method='Brent, xtol=1e-14; floating-point diagnostic, not interval certification',
                root_x=path(root).tolist(),ground_polynomial_at_root=float(at_root[0]),
                competitor_polynomial_at_root=float(at_root[1]),gram_matrix_at_root=G.tolist(),
                gram_min_eigenvalue=mineig,metric_min_eigenvalue=metric_min,
                full_grid_released=False,old_export_gate='FAIL')
    (args.run_dir/'nodal-check.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result,indent=2),flush=True)
    cx=np.linspace(-3.,1.,71); rx=np.linspace(-.5,4.,71)
    C,R=np.meshgrid(cx,rx); points=np.tile(base,(C.size,1)); points[:,c]=C.ravel(); points[:,r]=R.ravel()
    field=evaluate(X,points)[:,0].reshape(C.shape)
    fig,axs=plt.subplots(1,2,figsize=(11.5,5),layout='constrained')
    bound=float(np.max(abs(field)))+1
    axs[0].contourf(C,R,field,levels=[-bound,0,bound],colors=['#f4d9d6','#d4eaeb'])
    axs[0].contour(C,R,field,levels=[0],colors=['#333333'],linewidths=1.2)
    axs[0].plot([0,target[c]],[0,target[r]],color='#555555',linestyle='--')
    axs[0].scatter([path(root)[c]],[path(root)[r]],color='#111111',s=30,zorder=4,label='Numerically located zero')
    axs[0].scatter([target[c]],[target[r]],color='#a83d36',marker='x',s=55,zorder=4,label='Registered negative node')
    axs[0].set(xlabel='Competitor coordinate (Hermite units)',ylabel='Other singlet coordinate (Hermite units)',
               title='Saved ground-state sign on a two-dimensional slice')
    axs[0].legend(fontsize=8,loc='lower left')
    ts=np.linspace(0,1,500); pv=evaluate(X,np.array([path(t) for t in ts]))
    axs[1].plot(ts,pv[:,0],label='Ground polynomial',color='#167d91')
    axs[1].plot(ts,pv[:,1],label='Competitor polynomial',color='#a83d36')
    axs[1].axhline(0,color='#888888',linewidth=.8); axs[1].axvline(root,color='#333333',linestyle='--',linewidth=1)
    axs[1].set(xlabel='Position along dashed path',ylabel='Polynomial factor of the state',title='The ground vanishes; the competitor does not')
    axs[1].legend(fontsize=9)
    for ax in axs: ax.spines[['top','right']].set_visible(False)
    fig.suptitle('An interior sign change in the saved ground-state approximation',fontsize=13)
    fig.supxlabel('Teal: positive. Pink: negative. Gaussian and measure factors are positive; signs are unchanged.\n'
                  'f = competitor / ground is singular here. This is a slice, not a probability-mass map.',fontsize=9)
    fig.savefig(args.run_dir/'nodal-geometry.png',dpi=180)
    fig.savefig(args.run_dir/'nodal-geometry.pdf')
    plt.close(fig)


if __name__=='__main__': main()
