"""Rigorous cutoff/tail subgate; never a complete spectral certificate.

See helike-cutoff-tail-bounds-registration-2026-11-24.md for the inequalities.
All bound arithmetic uses Arb. Numpy only validates/imports fixed trial data.
"""
from functools import lru_cache
import math
import numpy as np
from flint import arb, arb_mat, ctx
import helike_positive_cone as pc

PRECISION=128
EPSILON=arb(1)/16
POWERS=(3,2,1,0,0,0)


def checked(x,positive=False,nonnegative=False):
    x=arb(x)
    if ctx.prec!=PRECISION: raise ValueError('registered precision is 128 bits')
    if not x.is_finite(): raise ValueError('nonfinite bound')
    if positive and not x>0: raise ValueError('positive bound required')
    if nonnegative and not x>=0: raise ValueError('nonnegative bound required')
    return x


def exact(x):
    x=float(x)
    if not math.isfinite(x): raise ValueError('nonfinite trial constant')
    return checked(arb(x))


def radial_tail(k,rho,decay=1):
    if isinstance(k,bool) or not isinstance(k,(int,np.integer)) or k<0 or k>32:
        raise ValueError('integer radial moment degree in [0,32] required')
    rho=checked(rho,nonnegative=True); decay=checked(decay,positive=True)
    y=decay*rho*rho
    return math.factorial(k+2)*(-y).exp()*sum((y**j/math.factorial(j) for j in range(k+3)),arb(0))/(2*decay**(k+3))


def endpoint(x,side):
    x=checked(x)
    if side not in ('lower','upper'): raise ValueError('directed endpoint required')
    bound=x.upper() if side=='upper' else x.lower()
    mantissa,exponent=bound.man_exp()
    return dict(mantissa=str(mantissa),exponent=int(exponent))


def enclosure(x):
    return dict(lower=endpoint(x,'lower'),upper=endpoint(x,'upper'),display=str(x))


def zero_to(x):
    x=checked(x,nonnegative=True)
    return dict(lower=endpoint(arb(0),'lower'),upper=endpoint(x,'upper'),upper_display=str(x.upper()))


def scaled_chart(u0,S,w):
    u0=np.asarray(u0); S=np.asarray(S); w=np.asarray(w)
    if u0.shape!=(6,) or S.shape!=(6,6) or w.shape!=(6,): raise ValueError('six-dimensional chart required')
    u=[exact(v) for v in u0]; frequencies=[checked(exact(v),positive=True) for v in w]
    matrix=arb_mat([[exact(v) for v in row] for row in S])
    if not matrix.det().abs_lower()>0: raise ValueError('singular or unresolved chart')
    A=[[matrix[i,j]/frequencies[j].sqrt() for j in range(6)] for i in range(6)]
    norms=[sum((v*v for v in row),arb(0)).sqrt() for row in A]
    return u,matrix,frequencies,A,norms


def plateau(u0,S,w,G0):
    u,matrix,frequencies,A,norms=scaled_chart(u0,S,w)
    G0=np.asarray(G0)
    if G0.shape!=(3,3) or not np.array_equal(G0,G0.T): raise ValueError('symmetric G0 required')
    G=arb_mat([[exact(v) for v in row] for row in G0])
    if not (G[0,0]>0 and G[0,0]*G[1,1]-G[0,1]**2>0 and G.det()>0):
        raise ValueError('positive G0 required')
    D=G.det()/100; R=4*sum((G[i,j]**2 for i in range(3) for j in range(3)),arb(0)).sqrt()
    # Enclose the declared real constants and the legacy binary64 evaluator constants.
    Ds=D.union(exact(.01*np.linalg.det(G0))); R=R.union(exact(4*np.linalg.norm(G0)))
    checked(Ds,positive=True); checked(R,positive=True)
    slope=sum(((2*sum((A[i][j] for i in range(3)),arb(0)))**2 for j in range(6)),arb(0)).sqrt()
    best=None
    for quarter in range(1,65):
        rho=arb(quarter)/4
        rs=[(u[i]+rho*norms[i]).exp() for i in range(3)]
        a,b,c=[abs(u[i])+rho*norms[i] for i in range(3,6)]
        g00=rs[0]**2; g11=rs[1]**2+a*a; g22=rs[2]**2+b*b+c*c
        nmax=(g00**2+g11**2+g22**2+2*((rs[0]*a)**2+(rs[0]*b)**2+(a*b+rs[1]*c)**2)).sqrt()
        dmin=(2*sum(u[:3],arb(0))-rho*slope).exp()
        if dmin>=2*Ds and nmax<=R:
            best=dict(rho=rho,Ds=Ds,R=R,det_lower=dmin.lower(),norm_upper=nmax.upper())
    if best is None: raise ValueError('no registered radius proves a complete cutoff plateau ball')
    return best


def polynomial_bounds(basis,C,rho):
    rho=checked(rho,positive=True); C=np.asarray(C)
    basis=[tuple(n) for n in basis]
    if (not basis or C.shape!=(len(basis),) or len(set(basis))!=len(basis)
            or any(len(n)!=6 or any(isinstance(v,bool) or not isinstance(v,(int,np.integer)) or v<0 for v in n) for n in basis)):
        raise ValueError('unique six-index nonnegative Hermite basis and matching vector required')
    m=max(map(sum,basis))
    if m>8: raise ValueError('registered trial degree exceeds eight')

    @lru_cache(None)
    def h(n):
        return (arb(math.factorial(n))/(arb(2)**n*math.factorial(n)).sqrt()
                *sum((arb(2)**(n-2*k)/(math.factorial(k)*math.factorial(n-2*k)*rho**(2*k))
                      for k in range(n//2+1)),arb(0)))

    def kh(n): return math.prod(h(int(v)) for v in n)
    aP=arb(0); b=[arb(0) for _ in range(6)]; norm=arb(0)
    for n,c in zip(basis,C):
        re,im=exact(c.real),exact(c.imag)
        c2=re*re+im*im; norm+=c2; magnitude=c2.sqrt()
        scale=magnitude/rho**int(m-sum(n)); aP+=scale*kh(n)
        for j in range(6):
            if n[j]:
                lowered=list(n); lowered[j]-=1
                b[j]+=scale*arb(int(2*n[j])).sqrt()*kh(lowered)
    checked(norm,positive=True)
    return aP,b,norm


def derivative_bounds(aP,b,S,w,rho):
    aP=checked(aP,nonnegative=True); rho=checked(rho,positive=True)
    if len(b)!=6: raise ValueError('six polynomial derivative bounds required')
    b=[checked(v,nonnegative=True) for v in b]
    _,matrix,freq,_,_=scaled_chart(np.zeros(6),S,w)
    inv=matrix.inv()
    if not all(inv[i,j].is_finite() for i in range(6) for j in range(6)): raise ValueError('unresolved inverse')
    return [sum((abs(inv[j,i])*freq[j].sqrt()*(aP+b[j]/rho**2) for j in range(6)),arb(0))
            +POWERS[i]*aP/(2*rho) for i in range(6)]


def cutoff_constants(Ds,R):
    Ds=checked(Ds,positive=True); R=checked(R,positive=True)
    U=2*R.upper(); denominator=Ds.lower(); radial=R.lower()
    ga=(8*U**2/denominator).sqrt(); gb=2*U.sqrt()/radial
    la=2*U**2/denominator; lb=14*arb(3).sqrt()/radial
    g=8*(ga+gb)
    lap=128*ga**2+8*la+128*gb**2+8*lb+128*ga*gb
    return g,lap


def metric_tail(u0,S,w,aD,m,rho,metric=None):
    u,_,_,A,norms=scaled_chart(u0,S,w); rho=checked(rho,positive=True)
    if len(aD)!=6: raise ValueError('six derivative bounds required')
    aD=[checked(v,nonnegative=True) for v in aD]
    if metric is None: metric=pc._symbolic_metric()[2]
    if len(metric)!=6 or any(len(row)!=6 for row in metric): raise ValueError('six by six metric required')
    count=0; total=arb(0)
    for i in range(6):
        for j in range(6):
            for key,value in metric[i][j].items():
                count+=1
                if count>100000: raise ValueError('metric term budget exhausted')
                if (len(key)!=6 or any(not isinstance(p,(int,np.integer)) for p in key)
                        or min(key[3:])<0 or not float(value).is_integer()):
                    raise ValueError('exact integer Laurent metric required')
                q=int(sum(key[3:]))
                beta=[sum((int(key[i])*A[i][j] for i in range(3)),arb(0)) for j in range(6)]
                exponent=sum((int(key[i])*u[i] for i in range(3)),arb(0))
                exponent+=sum((v*v for v in beta),arb(0))/(4*EPSILON)
                F=arb(abs(int(value)))*exponent.exp()
                for axis in range(3,6): F*=(abs(u[axis])/rho+norms[axis])**int(key[axis])
                k=m+1+(q+1)//2
                total+=aD[i]*aD[j]*F*radial_tail(k,rho,1-EPSILON)/rho**(q%2)
    return checked(total,nonnegative=True)


def combine(mass,gradient_sq,Geta,Leta,norm):
    mass=checked(mass,nonnegative=True); gradient_sq=checked(gradient_sq,nonnegative=True)
    Geta=checked(Geta,nonnegative=True); Leta=checked(Leta,nonnegative=True); norm=checked(norm,positive=True)
    tail_sq=mass/norm.lower()
    leakage_sq=(Geta*gradient_sq.sqrt()+Leta*mass.sqrt()/2)**2/norm.lower()
    cut_lower=(1-tail_sq).lower()
    if cut_lower<0: cut_lower=arb(0)
    return dict(tail_sq=tail_sq,leakage_sq=leakage_sq,cut_norm_sq_lower=cut_lower)


def classify(tail_sq,leakage_sq,norm):
    tail_sq=checked(tail_sq,nonnegative=True); leakage_sq=checked(leakage_sq,nonnegative=True)
    norm=checked(norm,positive=True)
    passed=(tail_sq.upper()<=arb('1e-12') and leakage_sq.upper()<=arb('1e-20')
            and norm.upper()-norm.lower()<=arb('1e-10'))
    return dict(cutoff_tail_gate_passed=bool(passed),status='UNRESOLVED_INTEGRATION',
                certified=False,full_grid_released=False)


def bound_state(u0,S,w,G0,basis,C,plateau_data=None):
    p=plateau(u0,S,w,G0) if plateau_data is None else plateau_data
    rho=p['rho']; m=max(map(sum,basis))
    aP,b,norm=polynomial_bounds(basis,C,rho)
    aD=derivative_bounds(aP,b,S,w,rho)
    mass=aP**2*radial_tail(m,rho)
    gradient=metric_tail(u0,S,w,aD,m,rho)
    Geta,Leta=cutoff_constants(p['Ds'],p['R'])
    out=combine(mass,gradient,Geta,Leta,norm)
    return dict(**classify(out['tail_sq'],out['leakage_sq'],norm),
        method='analytic_gaussian_majorant_arb128',
        rigorous_scope='saved_trial_cutoff_tail_and_normalization_only',
        plateau={k:enclosure(v) for k,v in p.items()},norm_sq=enclosure(norm),
        tail_sq=zero_to(out['tail_sq']),leakage_sq=zero_to(out['leakage_sq']),
        tail_norm=zero_to(out['tail_sq'].sqrt()),leakage_norm=zero_to(out['leakage_sq'].sqrt()),
        cut_norm_sq=dict(lower=endpoint(out['cut_norm_sq_lower'],'lower'),upper=endpoint(arb(1),'upper')),
        majorants=dict(polynomial=enclosure(aP),derivatives=[enclosure(v) for v in aD],
                       gradient_sq=zero_to(gradient),cutoff_gradient=enclosure(Geta),cutoff_laplacian=enclosure(Leta)))
