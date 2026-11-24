"""Local rank-two matching inherited from regular Cartesian q-space.

The signed chart is not a global atlas or a boundary-complete spectral solver.
See docs/research/helike-qspace-boundary-spec-2026-11-24.md.
"""
from functools import lru_cache
import numpy as np
import sympy as sp

import helike_berger_model as bm

MATCH_TOLERANCE = 1e-10
LOG_DENSITY_GRADIENT = (3,2,0,0,0,0)


def finite_array(value,shape,name,dtype=float):
    result=np.asarray(value,dtype=dtype)
    if result.shape!=shape or not np.isfinite(result).all():
        raise ValueError(f'{name} must have shape {shape} and finite entries')
    return result


def chart_values(u):
    u=finite_array(u,(6,),'chart coordinate')
    if np.max(abs(u[:2]))>300:
        raise ValueError('logarithmic coordinates outside safe numerical range')
    return np.array([*np.exp(u[:2]),*u[2:]])


def factor(u):
    r0,r1,w,a,b,c=chart_values(u)
    return np.array([[r0,0.,0.],[a,r1,0.],[b,c,w]])


def coordinates(q):
    q=finite_array(q,(3,3),'Cartesian matrix')
    scale=np.max(abs(q))
    if scale==0 or scale>1e150:
        raise ValueError('first-row-pair chart is singular or outside numerical range')
    r0=np.linalg.norm(q[0])
    if r0<=1e-12*scale:
        raise ValueError('first row is not resolved; use another row-pair chart')
    e0=q[0]/r0; a=q[1]@e0
    residual=q[1]-a*e0; r1=np.linalg.norm(residual)
    if r1<=1e-12*scale:
        raise ValueError('first two rows are not independently resolved; use another chart')
    e1=residual/r1; e2=np.cross(e0,e1)
    return np.array([np.log(r0),np.log(r1),q[2]@e2,a,q[2]@e0,q[2]@e1])


@lru_cache(maxsize=1)
def symbolic_chart():
    r0,r1=sp.symbols('r0 r1',positive=True)
    w,a,b,c=sp.symbols('w a b c',real=True)
    x=(r0,r1,w,a,b,c)
    L=sp.Matrix([[r0,0,0],[a,r1,0],[b,c,w]])
    columns=[]
    for i,v in enumerate(x):
        columns.append(sp.Matrix(list(L.diff(v)*(v if i<2 else 1))))
    for i,j in ((0,1),(0,2),(1,2)):
        E=sp.zeros(3); E[i,j]=-1; E[j,i]=1
        columns.append(sp.Matrix(list(L*E)))
    tangent=sp.Matrix.hstack(*columns)
    if sp.simplify(tangent.det()**2-r0**6*r1**4)!=0:
        raise ArithmeticError('Cartesian orbit Jacobian identity failed')
    K=tangent.inv()[:6,:]
    B=(K*K.T).applyfunc(sp.simplify)
    def du(expr,j): return sp.diff(expr,x[j])*(x[j] if j<2 else 1)
    drift=sp.Matrix([sp.simplify(sum(du(B[j,i],j)+LOG_DENSITY_GRADIENT[j]*B[j,i]
                                   for j in range(6))) for i in range(6)])
    return x,L,B,drift,8*r0**3*r1**2


@lru_cache(maxsize=1)
def numeric_chart():
    x,_,B,drift,mu=symbolic_chart()
    return sp.lambdify(x,(B,drift,mu),'numpy')


def geometry(u):
    with np.errstate(over='ignore',invalid='ignore',divide='ignore'):
        B,drift,mu=numeric_chart()(*chart_values(u))
    B=finite_array(B,(6,6),'reduced cometric')
    drift=finite_array(np.asarray(drift).reshape(6),(6,),'reduced drift')
    if not np.isfinite(mu) or mu<=0 or np.linalg.cond(B)>1e12:
        raise ValueError('signed chart is numerically singular or has unresolved density')
    return B,drift,float(mu)


def metric(u): return geometry(u)[0]


def density(u): return geometry(u)[2]


def laplacian(u,gradient,hessian):
    gradient=finite_array(gradient,(6,),'gradient',complex)
    hessian=finite_array(hessian,(6,6),'Hessian',complex)
    if not np.allclose(hessian,hessian.T,rtol=0,atol=1e-10):
        raise ValueError('Hessian must be symmetric, without conjugation')
    B,drift,_=geometry(u)
    result=np.sum(B*hessian)+drift@gradient
    if not np.isfinite(result): raise ValueError('nonfinite reduced Laplacian')
    return result


def potential(u,s,t,g):
    if not np.isfinite([s,t,g]).all() or t<=0 or g<=0:
        raise ValueError('finite model parameters with positive t and g required')
    L=factor(u); G=L@L.T; d=np.linalg.det(L)
    a,b=bm.mc_sources(s,t); om=bm.omega(t)
    value=(.5*np.dot(om*om-np.array([a,b,b]),np.diag(G))
           -g*om.sum()*d + .25*g*g*(np.trace(G)**2-np.trace(G@G)))
    if not np.isfinite(value): raise ValueError('nonfinite sourced potential')
    return float(value)


def matching_report(u,plus_value,minus_value,plus_gradient,minus_gradient):
    """Check supplied jets at w=0; gradients use h>=0 on their own half.

    Pointwise jet agreement cannot certify a whole function or a spectrum.
    """
    u=finite_array(u,(6,),'interface coordinate')
    if u[2]!=0: raise ValueError('matching requires the exact interface w=0')
    plus_value=finite_array(plus_value,(),'positive trace',complex).item()
    minus_value=finite_array(minus_value,(),'negative trace',complex).item()
    plus_gradient=finite_array(plus_gradient,(6,),'positive half-gradient',complex)
    minus_gradient=finite_array(minus_gradient,(6,),'negative half-gradient',complex)
    B,_,mu=geometry(u)
    positive_conormal=mu*(B[2]@plus_gradient)
    negative_conormal=mu*(B[2]@minus_gradient)
    value_jump=float(abs(plus_value-minus_value))
    conormal_sum=float(abs(positive_conormal+negative_conormal))
    if not np.isfinite([value_jump,conormal_sum]).all():
        raise ValueError('interface residuals exceed numerical range')
    return dict(matches=value_jump<=MATCH_TOLERANCE and conormal_sum<=MATCH_TOLERANCE,
                value_jump=value_jump,conormal_sum=conormal_sum,
                positive_conormal=[float(positive_conormal.real),float(positive_conormal.imag)],
                negative_conormal=[float(negative_conormal.real),float(negative_conormal.imag)],
                tolerance=MATCH_TOLERANCE,scope='local_rank_two_matching_only',spectral_certified=False)
