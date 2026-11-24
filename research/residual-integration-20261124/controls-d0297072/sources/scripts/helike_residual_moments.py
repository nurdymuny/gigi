"""Finite exponential-polynomial residual integration; never interval certification."""
import math
import time
import numpy as np
from scipy.special import roots_hermite
import helike_positive_cone as pc
from helike_quasimode import finite

class Space:
    def __init__(self,w,degree):
        self.w=finite(w).astype(float)
        if self.w.ndim!=1 or not 1<=len(self.w)<=6 or np.any(self.w<=0):
            raise ValueError('one to six positive frequencies required')
        if isinstance(degree,bool) or not isinstance(degree,int) or not 0<=degree<=16:
            raise ValueError('degree must be an integer in 0..16')
        self.degree=degree; self.basis=pc.states_up_to(len(self.w),degree)
        self.index={n:i for i,n in enumerate(self.basis)}; self.states=np.array(self.basis)
        self.top=self.states.sum(axis=1)==degree; self.transitions={}
        for axis in range(len(self.w)):
            for r in range(1,degree+1):
                src=np.flatnonzero(self.states[:,axis]>=r)
                dest=self.states[src].copy(); dest[:,axis]-=r
                self.transitions[axis,r]=(src,np.array([self.index[tuple(n)] for n in dest],dtype=int))

    def check(self,C):
        C=finite(C)
        if C.ndim!=2 or C.shape[0]!=len(self.basis) or not 1<=C.shape[1]<=4:
            raise ValueError('one to four complete coefficient vectors required')
        return C

    def op(self,C,axis,kind):
        C=self.check(C); out=np.zeros_like(C)
        if kind not in ('physical','coordinate','derivative') or not 0<=axis<len(self.w):
            raise ValueError('invalid polynomial operation')
        if kind!='derivative' and np.any(C[self.top]!=0):
            raise ValueError('nonzero coefficient would be truncated')
        if self.degree==0: return out
        src,dst=self.transitions[axis,1]; n=self.states[src,axis]; w=self.w[axis]
        lower=np.sqrt(n/(2*w)) if kind=='coordinate' else np.sqrt((2 if kind=='derivative' else .5)*w*n)
        out[dst]+=C[src]*lower[:,None]
        if kind!='derivative':
            upper=np.sqrt(n/(2*w)) if kind=='coordinate' else -np.sqrt(w*n/2)
            out[src]+=C[dst]*upper[:,None]
        return out

    def shift(self,C,mu,oracle=False):
        return finite(C+self.shift_increment(C,mu,oracle))

    def shift_increment(self,C,mu,oracle=False):
        C=self.check(C); mu=finite(mu)
        if mu.shape!=self.w.shape: raise ValueError('translation dimension mismatch')
        increment=np.zeros_like(C)
        for axis,amount in enumerate(mu):
            if amount==0: continue
            result=C+increment
            delta=np.zeros_like(C)
            if oracle:
                x,weights=roots_hermite(self.degree+1)
                h=pc.normalized_hermites(x,self.degree)
                offset=np.sqrt(self.w[axis])*amount
                difference=np.zeros_like(h)
                if self.degree: difference[1]=np.sqrt(2.)*offset
                for n in range(1,self.degree):
                    difference[n+1]=(np.sqrt(2/(n+1))*((x+offset)*difference[n]+offset*h[n])
                                     -np.sqrt(n/(n+1))*difference[n-1])
                matrix=(h*(weights/np.sqrt(np.pi)))@difference.T
                for r in range(1,self.degree+1):
                    src,dst=self.transitions[axis,r]; n=self.states[src,axis]
                    delta[dst]+=result[src]*matrix[n-r,n,None]
            else:
                term=result.copy()
                for r in range(1,self.degree+1):
                    term=self.op(term,axis,'derivative')*(amount/r)
                    delta+=term
            increment+=delta
        return finite(increment)


def build_residual(chart,basis,w,coeff,centers,s,t,g):
    coeff=finite(coeff); centers=finite(centers)
    if len(w)!=6 or coeff.ndim!=2 or coeff.shape[0]!=len(basis) or centers.shape!=(coeff.shape[1],):
        raise ValueError('six-dimensional saved state data required')
    space=Space(w,int(max(map(sum,basis)))+6)
    P=np.zeros((len(space.basis),coeff.shape[1]),complex)
    for n,C in zip(basis,coeff): P[space.index[tuple(n)]]=C
    def D(C,i):
        out=-.5*pc.MEASURE_POWERS[i]*C
        for k in range(6):
            if chart.invS[k,i]!=0: out=out+chart.invS[k,i]*space.op(C,k,'physical')
        return out
    first=[D(P,i) for i in range(6)]
    second={(i,j):D(first[j],i) for i in range(6) for j in range(i,6)}
    groups={(0,0,0):-P*centers[None,:]}
    def add(key,C,factor):
        if factor==0: return
        value=C
        for axis in range(3,6):
            for _ in range(key[axis]):
                out=chart.u0[axis]*value
                for k in range(6):
                    if chart.S[axis,k]!=0: out=out+chart.S[axis,k]*space.op(value,k,'coordinate')
                value=out
        prefix=tuple(key[:3]); value=value*(factor*np.exp(np.dot(prefix,chart.u0[:3])))
        groups[prefix]=groups.get(prefix,0.)+value
    metric,V=pc.laurent_model(s,t,g)
    for i in range(6):
        for j in range(6):
            for key,c in metric[i][j].items():
                add(key,second[min(i,j),max(i,j)],-.5*c)
                add(key,first[j],-.5*c*pc.MEASURE_POWERS[i])
                if i<3: add(key,first[j],-.5*c*key[i])
                elif key[i]:
                    lowered=list(key); lowered[i]-=1
                    add(lowered,first[j],-.5*c*key[i])
    for key,c in V.items(): add(key,P,c)
    return space,[(np.array(key)@chart.S[:3],finite(value)) for key,value in groups.items() if np.any(value!=0)]


def norm_squared(space,groups,oracle=False,deadline=None):
    if not groups: raise ValueError('nonempty exponential-polynomial required')
    combined=np.zeros_like(space.check(groups[0][1])); compensation=combined.copy()
    for _,C in groups:
        value=space.check(C)-compensation; updated=combined+value
        compensation=(updated-combined)-value; combined=updated
    contributions=[np.sum(abs(combined)**2,axis=0)]
    for i,(alpha,A) in enumerate(groups):
        space.check(A); alpha=finite(alpha)
        if alpha.shape!=space.w.shape: raise ValueError('exponent dimension mismatch')
        for j in range(i,len(groups)):
            if deadline is not None and time.monotonic()>deadline: raise TimeoutError('moment budget exhausted')
            beta,B=groups[j]; space.check(B); beta=finite(beta)
            if beta.shape!=alpha.shape or B.shape!=A.shape: raise ValueError('incompatible terms')
            total=alpha+beta; exponent=np.sum(total*total/(4*space.w))
            if exponent>600: raise ValueError('completed-square factor exceeds numerical contract')
            mu=total/(2*space.w)
            da=space.shift_increment(A,mu,oracle); db=space.shift_increment(B,mu,oracle)
            baseline=np.sum(np.conjugate(A)*B,axis=0).real
            cross=np.sum(np.conjugate(A)*db+np.conjugate(da)*B+np.conjugate(da)*db,axis=0).real
            pair=cross+np.expm1(exponent)*(baseline+cross)
            contributions.append(pair*(1 if i==j else 2))
    contributions=finite(contributions)
    values=np.array([math.fsum(contributions[:,j]) for j in range(contributions.shape[1])])
    if np.any(values<0): raise ArithmeticError('negative norm: unresolved cancellation')
    scale=np.sum(abs(contributions),axis=0)
    return dict(value=finite(values).tolist(),pair_absolute_sum=scale.tolist(),
                epsilon_times_pair_sum=(np.finfo(float).eps*scale).tolist(),
                roundoff_bound_certified=False,accumulation='centered_before_squaring',
                method='quadrature_translation' if oracle else 'taylor_translation')


def evaluate(space,groups,z):
    z=finite(z)
    if z.ndim!=2 or z.shape[1]!=len(space.w): raise ValueError('point dimension mismatch')
    tables=[pc.normalized_hermites(z[:,i]*np.sqrt(space.w[i]),space.degree) for i in range(len(space.w))]
    values=np.ones((len(z),len(space.basis)))
    for i,h in enumerate(tables): values*=h[space.states[:,i]].T
    out=np.zeros((len(z),groups[0][1].shape[1]),complex)
    for alpha,C in groups: out+=np.exp(z@alpha)[:,None]*(values@space.check(C))
    return finite(out)


def tensor_norm(space,groups,order,deadline=None):
    if order not in (10,12): raise ValueError('only original diagnostic orders allowed')
    dim=len(space.w)
    x,weights=roots_hermite(order); weights/=np.sqrt(np.pi)
    ix=np.indices((order,)*dim).reshape(dim,-1).T
    z=x[ix]/np.sqrt(space.w); W=np.prod(weights[ix],axis=1)
    out=np.zeros((order**dim,groups[0][1].shape[1]),complex)
    for alpha,C in groups:
        if deadline is not None and time.monotonic()>deadline: raise TimeoutError('tensor replay budget exhausted')
        nonzero=np.flatnonzero(np.any(C!=0,axis=1))
        extent=np.max(space.states[nonzero],axis=0)+1
        if int(np.prod(extent))*C.shape[1]*16*3>12*1024**3:
            raise MemoryError('tensor replay exceeds memory budget')
        tensor=np.zeros(tuple(extent)+(C.shape[1],),complex)
        tensor[tuple(space.states[nonzero].T)]=C[nonzero]
        for axis in range(dim):
            h=pc.normalized_hermites(x,int(extent[axis]-1))
            tensor=np.moveaxis(np.tensordot(h.T,tensor,axes=(1,axis)),0,axis)
        out+=np.exp(z@alpha)[:,None]*tensor.reshape(out.shape)
    return finite(W@(abs(out)**2)).tolist()


def numerical_result(primary,oracle):
    primary=finite(primary); oracle=finite(oracle)
    if primary.ndim!=1 or primary.shape!=oracle.shape or not primary.size or np.any(primary<0) or np.any(oracle<0):
        raise ValueError('matching nonnegative norm vectors required')
    difference=abs(primary-oracle)
    return dict(status='UNRESOLVED_INTEGRATION',certified=False,full_grid_released=False,
                agreement=bool(np.all(difference<=1e-9*(1+abs(oracle)))),differences=difference.tolist())
