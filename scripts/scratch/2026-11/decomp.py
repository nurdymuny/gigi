import json, sys, numpy as np
sys.path.insert(0, '/Users/bdavis/.claude/jobs/c8f0f39d/tmp/ricci/e2e/probes')
import ricci_rf2h as g
from scipy.sparse import load_npz, kron, eye
from scipy.sparse.linalg import splu
from ricci_lattice_mesh import plus
R='/Users/bdavis/.claude/jobs/c8f0f39d/tmp/ricci/output/'; VAR=R+'ricci-variational-20261117/abelian/'
def true_moments(P, order=9):
    u,v,w=g.tri_rule(order); d1=P[:,1]-P[:,0]; d2=P[:,2]-P[:,0]
    y=P[:,None,0]+u[None,:,None]*d1[:,None]+v[None,:,None]*d2[:,None]; r=np.linalg.norm(y,axis=-1); x=y/r[...,None]
    dp=lambda d:(d[:,None,:]-x*np.einsum('nqi,ni->nq',x,d)[...,None])/r[...,None]
    xu,xv=dp(d1),dp(d2); conj=lambda q:q*np.array([1.,-1,-1,-1])
    val=(plus(conj(xu),xv)-plus(conj(xv),xu))[...,1]
    lam=np.stack([1-u-v,u,v]); return np.einsum('iq,q,nq->ni',lam,w,val)
for L in [int(x) for x in sys.argv[1].split(',')]:
    p=json.load(open(R+f'ricci-shape-regular-20261117/berger/level{L}_berger_q6.input.json'))
    b=g.build_recovery(p, order=6, verbose=False); Phi=b['Phi']; mesh=b['mesh']
    D=load_npz(VAR+f'level{L}_q6.d.npz'); H=load_npz(VAR+f'level{L}_q6.h.npz'); M=load_npz(VAR+f'level{L}_q6.m.npz').tocsc()
    s=np.load(VAR+f'level{L}_q6.arrays.npz'); a=s['a'][:,0]; load=s['load'][:,0]
    vol=json.load(open(VAR+'diagnostic.json'))['cases'][f'level{L}_q6']['volume']
    nf=D.shape[0]; j=kron(eye(nf),np.ones((3,1))/3).tocsr(); z=kron(eye(nf),g.Z_BLOCK).tocsr()
    lu=splu(M); nrm=lambda x: float(np.sqrt(max(0,x@(M@x))/vol)); ref=nrm(4*a); rel=lambda r: nrm(lu.solve(r))/ref
    f=D@a; ct=true_moments(mesh.topo['faces']).ravel(); cR=j@f+z@(Phi@f)
    Hn=lambda c: float(np.sqrt(c@(H@c)))
    out={'level':L,'h':json.load(open(VAR+'diagnostic.json'))['cases'][f'level{L}_q6']['h_max'],
      'whitney_rel': rel(D.T@(j.T@(H@(j@f)))+load),
      'rf2hL_rel': rel(D.T@(g.linear_operator(b['C'],H)@f)+load),
      'termA_whitney_tested_recovery_err': rel(D.T@(j.T@(H@(cR-ct)))),
      'termB_adj_residual': rel(D.T@(Phi.T@(z.T@(H@(j@f))))),
      'bdm_interp_err_vs_load': rel(D.T@(j.T@(H@ct))+load),
      'recovery_rel_H': Hn(cR-ct)/Hn(ct), 'whitney_rel_H': Hn(j@f-ct)/Hn(ct)}
    print('DECOMP', json.dumps(out), flush=True)
