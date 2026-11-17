"""Independent RF2D diagnostic operators; frozen production code is untouched."""
import itertools
import numpy as np
from scipy.sparse.linalg import cg,LinearOperator
from ricci_wilson_oracle import phi,wilson

EDGES=list(itertools.combinations(range(4),2))
FACES=list(itertools.combinations(range(4),3))
INCIDENCE=np.array([[1,-1,0,1,0,0],[1,0,-1,0,1,0],[0,1,-1,0,0,1],[0,0,0,1,-1,1]],dtype=float)


def phase_summary(q):
    q=np.asarray(q,dtype=float)
    if q.ndim!=2 or q.shape[1]!=4 or len(q)==0 or not np.isfinite(q).all() or np.max(abs(np.linalg.norm(q,axis=1)-1))>1e-12:
        raise ValueError('nonempty finite unit quaternions required; no normalization/sign folding')
    q0=q[:,0];alpha=np.arctan2(np.linalg.norm(q[:,1:],axis=1),q0)
    s=(1+q0)/2;d=(1-q0)/2
    return {'count':len(q),'q0_min':float(q0.min()),'q0_max':float(q0.max()),
        'negative_q0_count':int(np.sum(q0<0)),'alpha_max':float(alpha.max()),
        'minimum_hemisphere_distance':float(np.min(abs(q0))),
        'minimum_distance_to_minus_identity':float(np.min(np.linalg.norm(q+np.array([1.,0,0,0]),axis=1))),
        's_lift_min':float(s.min()),'so3_overlap_min':float((q0*q0).min()),
        'bridge_max_error':float(np.max(abs(s+d-1))),
        'so3_bridge_max_error':float(np.max(abs(q0*q0+np.sum(q[:,1:]**2,axis=1)-1)))}


def base_log(p,q):
    p,q=np.asarray(p),np.asarray(q)
    real=np.sum(p*q,axis=-1)
    imag=p[...,0,None]*q[...,1:]-q[...,0,None]*p[...,1:]-np.cross(p[...,1:],q[...,1:])
    norm=np.linalg.norm(imag,axis=-1);alpha=np.arctan2(norm,real)
    if np.any(alpha>=np.pi-1e-8):raise ValueError('ambiguous antipodal short path')
    return imag*np.divide(alpha,norm,out=np.ones_like(norm),where=norm>0)[...,None]


def link_quaternions(metric,p,q):
    a=base_log(p,q)*phi(np.array(metric));half=np.linalg.norm(a,axis=-1)/2
    return np.concatenate([np.cos(half)[...,None],-.5*np.sinc(half/np.pi)[...,None]*a],axis=-1)


def compose(a,b):
    a,b=np.asarray(a),np.asarray(b)
    return np.concatenate([(a[...,0]*b[...,0]-np.sum(a[...,1:]*b[...,1:],axis=-1))[...,None],
        a[...,0,None]*b[...,1:]+b[...,0,None]*a[...,1:]-np.cross(a[...,1:],b[...,1:])],axis=-1)


def inverse(q):return np.asarray(q)*np.array([1.,-1.,-1.,-1.])


def rotations(q):
    q=np.asarray(q);v=q[...,1:];cross=np.zeros(q.shape[:-1]+(3,3))
    cross[...,0,1]=-v[...,2];cross[...,0,2]=v[...,1];cross[...,1,0]=v[...,2]
    cross[...,1,2]=-v[...,0];cross[...,2,0]=-v[...,1];cross[...,2,1]=v[...,0]
    return (2*q[...,0]**2-1)[...,None,None]*np.eye(3)+2*v[..., :,None]*v[...,None,:]-2*q[...,0,None,None]*cross


def cell_integrals(points,links,metric,order,velocity=None,trial=None):
    points=np.asarray(points);links=np.asarray(links);metric=np.asarray(metric)
    if points.ndim!=3 or points.shape[1:]!=(4,4) or order not in [4,6]:raise ValueError('batch of radial cells, order 4 or 6 required')
    if not np.isfinite(points).all() or np.any(np.linalg.det(points)<=1e-16):raise ValueError('positive oriented finite cells required')
    roots,weights=np.polynomial.legendre.leggauss(order);roots=(roots+1)/2;weights/=2
    indices=np.array(list(itertools.product(range(order),repeat=3)));u,v,w=roots[indices].T
    local=np.column_stack([u,(1-u)*v,(1-u)*(1-v)*w]);lam=np.column_stack([1-local.sum(axis=1),local])
    weight=np.prod(weights[indices],axis=1)*(1-u)**2*(1-v)
    b=(points[:,1:]-points[:,0,None]).transpose(0,2,1)
    y=np.einsum('qa,cai->cqi',lam,points);norm=np.linalg.norm(y,axis=2);x=y/norm[...,None]
    dx=(b[:,None]-x[...,None]*np.einsum('cqi,cia->cqa',x,b)[:,:,None,:])/norm[...,None,None]
    # Independent ambient left-invariant frame, as in the frozen mass oracle.
    frame=np.empty(x.shape[:-1]+(4,3));frame[...,0,:]=-x[...,1:]
    skew=np.zeros(x.shape[:-1]+(3,3));a,bb,c=np.moveaxis(x[...,1:],-1,0)
    skew[...,0,1]=-c;skew[...,0,2]=bb;skew[...,1,0]=c;skew[...,1,2]=-a;skew[...,2,0]=-bb;skew[...,2,1]=a
    frame[...,1:,:]=x[...,0,None,None]*np.eye(3)+skew
    maurer=np.einsum('cqia,cqib->cqab',frame,dx)
    g=np.einsum('cqia,i,cqib->cqab',maurer,metric,maurer);det=np.linalg.det(g);inv=np.linalg.inv(g)
    if np.any(det<=0):raise ValueError('positive metric required')
    measure=weight[None,:]*np.sqrt(det)
    grads=np.vstack([-np.ones(3),np.eye(3)])
    one=np.array([lam[:,i,None]*grads[j]-lam[:,j,None]*grads[i] for i,j in EDGES]).transpose(1,0,2)
    two=np.array([2*(lam[:,i,None]*np.cross(grads[j],grads[k])-lam[:,j,None]*np.cross(grads[i],grads[k])+lam[:,k,None]*np.cross(grads[i],grads[j])) for i,j,k in FACES]).transpose(1,0,2)
    m1=np.einsum('qea,cqab,qfb,cq->cef',one,inv,one,measure,optimize=True)
    m2=np.einsum('qea,cqab,qfb,cq->cef',two,g,two,measure/det,optimize=True)
    r=rotations(links[:,[0,1,2,1,2,2]])
    if trial is not None:
        target=-2*np.einsum('cejk,cek,qea->cqja',r,trial,one,optimize=True)
    else:
        if velocity is None:velocity=np.diag(wilson(metric,phi(metric)))
        to_root=rotations(link_quaternions(metric,points[:,0,None,:],x))
        target=np.einsum('cqjk,ik,cqia->cqja',to_root,velocity,maurer,optimize=True)
    load=-2*np.einsum('cejk,cqja,cqab,qeb,cq->cek',r,target,inv,one,measure,optimize=True)
    return {'edge_mass':m1,'face_mass':m2,'volume':measure.sum(axis=1),'load':load}


def solve_rhs(b,rhs):
    diagonal=b.diagonal()
    if np.any(diagonal<=0) or not np.isfinite(rhs).all():raise ValueError('finite rhs and positive mass required')
    x,status=cg(b,rhs,rtol=1e-12,atol=1e-14,maxiter=4000,M=LinearOperator(b.shape,matvec=lambda z:z/diagonal))
    error=float(np.linalg.norm(b@x-rhs))
    if status or error>3*max(1e-12*np.linalg.norm(rhs),1e-14):raise ValueError('diagnostic solve failed')
    return x


def residual_split(b,gradient,reference,load,volume):
    if not np.isfinite(volume) or volume<=0:raise ValueError('positive finite volume required')
    action=solve_rhs(b,gradient+load);mass=solve_rhs(b,b@reference-load)
    total=solve_rhs(b,gradient+b@reference)
    norm=lambda v:float(np.sqrt(max(0.,v@(b@v))/volume))
    return {'total_error':norm(total),'action_error':norm(action),'mass_error':norm(mass),
        'cross_inner':float(action@(b@mass)/volume),'recombination_error':norm(total-action-mass)}
