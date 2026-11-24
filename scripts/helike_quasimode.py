"""Full physical quasimode diagnostics. Numerical quadrature NEVER certifies an interval.

See helike-quasimode-construction-registration-2026-11-24.md. No model changes.
"""
import math
import time
import numpy as np
from scipy.special import expit, roots_hermite
import torch
import helike_positive_cone as pc
import helike_berger_model as bm

DELTA, R_FACTOR, CHUNK = .01, 4., 32768
ORDERS = (10,12)

def point_action(q,chart,basis,w,coeff,s,t,g,cutoff=True):
    if outside_support(q,chart,cutoff): return 0j
    z=torch.tensor(chart.inverse(np.asarray(q)@np.asarray(q).T),dtype=torch.float64,requires_grad=True)
    value=physical_z(z,chart,basis,w,coeff)
    gradients=[]; hessians=[]
    for part in (value.real,value.imag):
        grad=torch.autograd.grad(part,z,create_graph=True,retain_graph=True)[0]
        hess=torch.stack([torch.autograd.grad(grad[i],z,retain_graph=True)[0] for i in range(6)])
        gradients.append(grad.detach().numpy()); hessians.append(hess.detach().numpy())
    grad=gradients[0]+1j*gradients[1]; hess=hessians[0]+1j*hessians[1]
    geo=geometry(chart,z.detach().numpy()[None,:],s,t,g); value=complex(value.detach().item())
    HP=-.5*(np.sum(geo['B'][0]*hess)+geo['drift'][0]@grad)+geo['V'][0]*value
    if cutoff:
        HP=cutoff_action(value,HP,geo['eta'][0],geo['grad_eta'][0]@geo['B'][0]@grad,geo['lap_eta'][0])
    return complex(HP)


def finite(x):
    x=np.asarray(x)
    if not np.isfinite(x).all(): raise ValueError('nonfinite quasimode evidence')
    return x


def smooth_step(x):
    x=finite(x).astype(float)
    h=np.zeros_like(x); d=h.copy(); dd=h.copy()
    h[x>=2]=1.
    mask=(x>1)&(x<2); a=x[mask]-1; b=2-x[mask]
    v=expit(-1/a+1/b); first=1/a**2+1/b**2; second=-2/a**3+2/b**3
    h[mask]=v; d[mask]=v*(1-v)*first
    dd[mask]=v*(1-v)*((1-2*v)*first**2+second)
    return h,d,dd


def torch_step(x):
    # Clamp only the inactive branches, preventing 0*NaN in automatic differentiation.
    y=torch.clamp(x,1+1e-12,2-1e-12)
    value=torch.sigmoid(-1/(y-1)+1/(2-y))
    return torch.where(x<=1,torch.zeros_like(x),torch.where(x>=2,torch.ones_like(x),value))


def cutoff_q(q,chart):
    G=q@q.T
    D=torch.linalg.det(G); N=torch.linalg.matrix_norm(G)
    return torch_step(D/(DELTA*np.linalg.det(chart.G0)))*(1-torch_step(N/(R_FACTOR*np.linalg.norm(chart.G0))))


def outside_support(q,chart,cutoff):
    q=finite(q).astype(float)
    if q.shape!=(3,3): raise ValueError('Cartesian point must be 3 by 3')
    det=np.linalg.det(q)
    if not cutoff:
        if det<=0: raise ValueError('uncut oracle is defined only on the positive chart')
        return False
    return (det<=0 or det*det<=DELTA*np.linalg.det(chart.G0)
            or np.linalg.norm(q@q.T)>=2*R_FACTOR*np.linalg.norm(chart.G0))


def term(key,x):
    return np.prod(np.power(x,np.asarray(key)),axis=1)


def geometry(chart,z,s,t,g):
    z=finite(z).astype(float)
    if z.ndim!=2 or z.shape[1]!=6: raise ValueError('six chart coordinates required')
    u=chart.u0+z@chart.S.T
    x=u.copy(); x[:,:3]=np.exp(u[:,:3]); finite(x)
    n=len(z); r0,r1,r2,a,b,c=x.T
    L=np.zeros((n,3,3)); L[:,0,0]=r0; L[:,1,1]=r1; L[:,2,2]=r2
    L[:,1,0]=a; L[:,2,0]=b; L[:,2,1]=c
    G=L@L.transpose(0,2,1)
    metrics,potential=pc.laurent_model(s,t,g)
    Bu=np.zeros((n,6,6)); driftu=np.zeros((n,6))
    for i in range(6):
        for j in range(6):
            for key,value in metrics[i][j].items():
                F=term(key,x); Bu[:,i,j]+=value*F
                driftu[:,j]+=pc.MEASURE_POWERS[i]*value*F
                if i<3: driftu[:,j]+=key[i]*value*F
                elif key[i]:
                    reduced=list(key); reduced[i]-=1
                    driftu[:,j]+=key[i]*value*term(reduced,x)
    B=np.einsum('ij,njk,lk->nil',chart.invS,Bu,chart.invS,optimize=True)
    drift=driftu@chart.invS.T
    V=sum(value*term(key,x) for key,value in potential.items())
    vu=np.column_stack((np.zeros(n),-c/r1,c/r1,-b,a,r1-r2*r2/r1))
    velocity=vu@chart.invS.T
    D=(r0*r1*r2)**2; N=np.sqrt(np.sum(G*G,axis=(1,2))); tr=np.trace(G,axis1=1,axis2=2)
    tr3=np.einsum('nij,njk,nki->n',G,G,G)
    cof2=(tr*tr-N*N)/2
    Dscale=DELTA*np.linalg.det(chart.G0); R=R_FACTOR*np.linalg.norm(chart.G0)
    h,dh,ddh=smooth_step(D/Dscale); k,dk,ddk=smooth_step(N/R)
    du=np.zeros((n,6)); du[:,:3]=2*D[:,None]/Dscale
    # d ||G||_F / du, using the six independent Gram entries and their off-diagonal multiplicity.
    J=np.zeros((n,6,6))
    J[:,0,0]=2*r0*r0; J[:,1,1]=2*r1*r1; J[:,1,3]=2*a
    J[:,2,2]=2*r2*r2; J[:,2,4]=2*b; J[:,2,5]=2*c
    J[:,3,0]=r0*a; J[:,3,3]=r0; J[:,4,0]=r0*b; J[:,4,4]=r0
    J[:,5,1]=r1*c; J[:,5,3]=b; J[:,5,4]=a; J[:,5,5]=r1
    gv=np.column_stack((G[:,0,0],G[:,1,1],G[:,2,2],2*G[:,0,1],2*G[:,0,2],2*G[:,1,2]))
    dv=np.einsum('nij,ni->nj',J,gv)/(N*R)[:,None]
    grad_eta=(dh*(1-k))[:,None]*du-(h*dk)[:,None]*dv
    grad_eta=grad_eta@chart.S
    lap_u=2*cof2/Dscale; norm_u=4*D*cof2/Dscale**2
    lap_v=(14*tr/N-4*tr3/N**3)/R; norm_v=4*tr3/(N*R)**2
    mixed=4*D*tr/(N*Dscale*R)
    lap_eta=(1-k)*(ddh*norm_u+dh*lap_u)-h*(ddk*norm_v+dk*lap_v)-2*dh*dk*mixed
    out=dict(B=B,drift=drift,V=V,velocity=velocity,eta=h*(1-k),grad_eta=grad_eta,lap_eta=lap_eta)
    for value in out.values(): finite(value)
    return out


def grid_coordinates(w,order):
    x,weights=roots_hermite(order); weights=weights/np.sqrt(np.pi)
    ix=np.indices((order,)*6).reshape(6,-1).T
    return x[ix]/np.sqrt(w),np.prod(weights[ix],axis=1)


def polynomial_grid(coeff,basis,w,order,derivatives=()):
    coeff=finite(coeff); w=finite(w)
    if coeff.ndim!=2 or len(coeff)!=len(basis) or w.shape!=(6,) or np.any(w<=0):
        raise ValueError('invalid polynomial data')
    degree=max(map(max,basis)); C=np.zeros((degree+1,)*6+(coeff.shape[1],),dtype=coeff.dtype)
    for i,n in enumerate(basis): C[n]=coeff[i]
    for axis in derivatives:
        if axis not in range(6): raise ValueError('invalid derivative axis')
        dest=[slice(None)]*7; src=dest.copy(); dest[axis]=slice(0,degree); src[axis]=slice(1,degree+1)
        shape=[1]*7; shape[axis]=degree
        lowered=np.zeros_like(C)
        lowered[tuple(dest)]=C[tuple(src)]*np.sqrt(2*w[axis]*np.arange(1,degree+1)).reshape(shape)
        C=lowered
    nodes,_=roots_hermite(order); h=pc.normalized_hermites(nodes,degree)
    for axis in range(6): C=np.moveaxis(np.tensordot(h.T,C,axes=(1,axis)),0,axis)
    return C.reshape(order**6,-1)


def cutoff_action(P,HP,eta,gradient_contraction,lap_eta):
    return eta*HP-gradient_contraction-.5*lap_eta*P


def square_integral(values,weights):
    values=finite(values); weights=finite(weights)
    if values.ndim!=2 or weights.shape!=(len(values),) or np.any(weights<0):
        raise ValueError('nonnegative matching integration weights required')
    return finite(weights@(np.abs(values)**2))


def projection_bound(tail,charge,residual,leakage):
    vals=finite([tail,charge,residual,leakage])
    if np.any(vals<0) or tail>=1: raise ValueError('invalid norm bounds')
    lower=(1-tail)**2-charge**2
    if lower<=0: raise ValueError('projected norm has no positive lower bound')
    # This evaluates a mathematical inequality on supplied numbers, not their validity as enclosures.
    return float(np.nextafter((residual+leakage)/np.sqrt(lower),np.inf))


def integration_status(coarse,fine):
    if not coarse or set(coarse)!=set(fine): raise ValueError('matching nonempty integral sets required')
    finite(list(coarse.values())); finite(list(fine.values()))
    differences={k:abs(float(coarse[k])-float(fine[k])) for k in coarse}
    agrees=all(differences[k]<=1e-9*(1+abs(fine[k])) for k in fine)
    return dict(status='UNRESOLVED_INTEGRATION',certified=False,
                agreement='NUMERICAL_AGREEMENT' if agrees else 'NUMERICAL_DISAGREEMENT',differences=differences)


def sample_grid(solver,coeff,order,deadline=None):
    """Full differential action on a tensor grid; all amplitudes have the common Gaussian/density factor removed."""
    if order not in ORDERS: raise ValueError('unregistered integration order')
    n=order**6; k=coeff.shape[1]
    if n*(70+20*k)*8>12*1024**3: raise MemoryError('audit memory budget exceeded')
    def budget():
        if deadline is not None and time.monotonic()>deadline: raise TimeoutError('per-rung audit budget exhausted')
    budget(); z,weights=grid_coordinates(solver.w,order)
    P=polynomial_grid(coeff,solver.basis,solver.w,order)
    B=np.empty((n,6,6)); drift=np.empty((n,6)); V=np.empty(n); vel=np.empty((n,6))
    eta=np.empty(n); ge=np.empty((n,6)); le=np.empty(n)
    for start in range(0,n,CHUNK):
        budget(); sl=slice(start,min(start+CHUNK,n))
        geo=geometry(solver.chart,z[sl],solver.s,solver.t,solver.g)
        B[sl]=geo['B']; drift[sl]=geo['drift']; V[sl]=geo['V']; vel[sl]=geo['velocity']
        eta[sl]=geo['eta']; ge[sl]=geo['grad_eta']; le[sl]=geo['lap_eta']
    a=z*solver.w+.5*solver.chart.ell
    Ba=np.einsum('nij,nj->ni',B,a)
    factor=V-.5*(np.sum(a*Ba,axis=1)-np.einsum('nii,i->n',B,solver.w)-np.sum(drift*a,axis=1))
    HP=factor[:,None]*P
    LP=(1j*np.sum(vel*a,axis=1))[:,None]*P
    Bge=np.einsum('nij,nj->ni',B,ge)
    contraction=(-np.sum(Bge*a,axis=1))[:,None]*P
    for i in range(6):
        budget(); grad=polynomial_grid(coeff,solver.basis,solver.w,order,(i,))
        HP+=(Ba[:,i]-.5*drift[:,i])[:,None]*grad
        LP+=(-1j*vel[:,i])[:,None]*grad
        contraction+=Bge[:,i,None]*grad
        for j in range(i,6):
            budget(); hess=polynomial_grid(coeff,solver.basis,solver.w,order,(i,j))
            HP-=((.5 if i==j else 1.)*B[:,i,j])[:,None]*hess
    comm=cutoff_action(P,np.zeros_like(HP),eta[:,None],contraction,le[:,None])
    for value in (P,HP,LP,comm): finite(value)
    return dict(P=P,HP=HP,LP=LP,comm=comm,eta=eta,weights=weights)


def chiral_frame(samples):
    P=samples['P'][:,1:3]; LP=samples['LP'][:,1:3]; weights=samples['weights']
    L=P.conj().T@(weights[:,None]*LP)
    if np.max(abs(L-L.conj().T))>1e-9: raise ValueError('generator compression is not resolved Hermitian')
    e,v=np.linalg.eigh((L+L.conj().T)/2)
    if not e[0]<0<e[1]: raise ValueError('Helike partners do not straddle charge zero')
    C=np.eye(4,dtype=complex); C[1:3,1]=v[:,1]; C[1:3,2]=v[:,0]
    return C,e.tolist()


def summarize(samples,C,centers):
    P=samples['P']@C; HP=samples['HP']@C; LP=samples['LP']@C; comm=samples['comm']@C
    eta=samples['eta'][:,None]; weights=samples['weights']
    cut=eta*P; HC=eta*HP+comm
    data={}
    for j,(name,m) in enumerate(zip(('ground','helike_plus','helike_minus','competitor'),(0,2,-2,0))):
        one=lambda v:float(square_integral(v[:,j:j+1],weights)[0])
        norm=one(P); cut_norm=one(cut)
        if min(norm,cut_norm)<=0: raise ValueError('normalization unresolved')
        residual=one(HP-P*np.asarray(centers)[None,:])
        row=dict(norm=norm,cut_norm=cut_norm,residual_sq=residual,
                 cutoff_residual_sq=one(HC-cut*np.asarray(centers)[None,:]),
                 charge_sq=one(LP-P*np.array([0,2,-2,0])[None,:]),
                 tail_sq=one((1-eta)*P),leakage_sq=one(comm))
        expectation=np.sum(weights*cut[:,j].conj()*HC[:,j])/cut_norm
        row.update(rayleigh=float(expectation.real),rayleigh_imag=float(expectation.imag))
        finite(list(row.values())); data[name]=row
    return data


def physical_q(q,chart,basis,w,coeff):
    """Independent Cartesian oracle path, differentiated by PyTorch, not the reduced operator."""
    L=torch.linalg.cholesky(q@q.T)
    u=torch.stack((torch.log(L[0,0]),torch.log(L[1,1]),torch.log(L[2,2]),L[1,0],L[2,0],L[2,1]))
    z=torch.as_tensor(chart.invS)@(u-torch.as_tensor(chart.u0))
    return physical_z(z,chart,basis,w,coeff)


def physical_z(z,chart,basis,w,coeff):
    u=torch.as_tensor(chart.u0)+torch.as_tensor(chart.S)@z
    w=torch.as_tensor(w); degree=max(map(max,basis)); hs=[torch.ones(6,dtype=z.dtype),np.sqrt(2.)*torch.sqrt(w)*z]
    for n in range(1,degree): hs.append(np.sqrt(2/(n+1))*torch.sqrt(w)*z*hs[-1]-np.sqrt(n/(n+1))*hs[-2])
    hs=torch.stack(hs[:degree+1]); states=torch.as_tensor(basis,dtype=torch.long)
    values=torch.ones(len(basis),dtype=z.dtype)
    for j in range(6): values=values*hs[states[:,j],j]
    poly=values.to(torch.complex128)@torch.as_tensor(coeff,dtype=torch.complex128)
    logf=.25*torch.log(w/np.pi).sum()-.5*(w*z*z).sum()-.5*(chart.log_scale+torch.as_tensor(pc.MEASURE_POWERS)@u)
    return torch.exp(logf)*poly


def cartesian_action(q,chart,basis,w,coeff,s,t,g,cutoff=True):
    if outside_support(q,chart,cutoff): return 0j
    q=torch.tensor(q,dtype=torch.float64,requires_grad=True)
    value=physical_q(q,chart,basis,w,coeff)
    if cutoff: value=value*cutoff_q(q,chart)
    lap=[]
    for part in (value.real,value.imag):
        grad=torch.autograd.grad(part,q,create_graph=True,retain_graph=True)[0]
        lap.append(sum(torch.autograd.grad(grad.ravel()[j],q,retain_graph=True)[0].ravel()[j] for j in range(9)))
    a,b=bm.mc_sources(s,t); V=bm.V_sourced(q.detach().numpy(),g,t,a,b)
    return complex((-.5*(lap[0]+1j*lap[1])+V*value).detach().item())
