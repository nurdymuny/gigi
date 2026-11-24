"""Exact-form Hermite trial functions on the positive Gram cone.

No linear continuation of G, Taylor potential, or boundary certification.
See docs/research/helike-positive-cone-basis-spec-2026-11-24.md.
"""
from functools import lru_cache
import math

import numpy as np
import scipy.sparse as sparse
from scipy.special import roots_hermite
import sympy as sp

import helike_berger_model as bm
import helike_condensate_pt as cp
import helike_condensate_fock as lf

states_up_to = lf.states_up_to
ZERO = (0,)*6
MEASURE_POWERS = np.array([3., 2., 1., 0., 0., 0.])


def positive_factor(G):
    G = np.asarray(G, dtype=float)
    if G.shape != (3,3) or not np.isfinite(G).all() or not np.array_equal(G, G.T):
        raise ValueError("G must be a finite symmetric 3 by 3 matrix")
    try:
        L = np.linalg.cholesky(G)
    except np.linalg.LinAlgError as exc:
        raise ValueError("G must be positive definite; projection is forbidden") from exc
    if np.linalg.cond(G) > 1e12:
        raise ValueError("Gram chart is numerically ill-conditioned")
    return L


class ConeChart:
    def __init__(self, G0, M):
        L = positive_factor(G0)
        self.G0 = np.asarray(G0, dtype=float).copy()
        self.M = np.asarray(M, dtype=float).copy()
        if self.M.shape != (6,6) or not np.isfinite(self.M).all() or np.linalg.cond(self.M) > 1e12:
            raise ValueError("normal frame must be finite and nonsingular")
        self.u0 = np.array([*np.log(np.diag(L)), L[1,0], L[2,0], L[2,1]])
        self.S = np.linalg.solve(self.jacobian_u(L), self.M)
        self.invS = np.linalg.inv(self.S)
        self.ell = self.S.T @ MEASURE_POWERS
        self.log_scale = math.log(8.) + np.linalg.slogdet(self.S)[1]

    def coordinates(self, z):
        z = np.asarray(z, dtype=float)
        if z.shape != (6,) or not np.isfinite(z).all():
            raise ValueError("chart coordinate must have six finite components")
        u = self.u0 + self.S @ z
        if not np.isfinite(u).all() or np.max(abs(u[:3])) > 300:
            raise ValueError("chart exponential outside safe floating-point range")
        return u

    def factor(self, z):
        u = self.coordinates(z)
        r = np.exp(u[:3])
        L = np.array([[r[0],0.,0.],[u[3],r[1],0.],[u[4],u[5],r[2]]])
        with np.errstate(over='ignore', invalid='ignore'):
            positive_factor(L @ L.T)
        return L

    def gram(self, z):
        L = self.factor(z)
        return L @ L.T

    def inverse(self, G):
        L = positive_factor(G)
        u = np.array([*np.log(np.diag(L)),L[1,0],L[2,0],L[2,1]])
        return self.invS @ (u-self.u0)

    @staticmethod
    def jacobian_u(L):
        r0,r1,r2 = np.diag(L)
        a,b,c = L[1,0],L[2,0],L[2,1]
        return np.array([[2*r0*r0,0,0,0,0,0],
                         [0,2*r1*r1,0,2*a,0,0],
                         [0,0,2*r2*r2,0,2*b,2*c],
                         [r0*a,0,0,r0,0,0],
                         [r0*b,0,0,0,r0,0],
                         [0,r1*c,0,b,a,r1]], dtype=float)

    def jacobian(self, z):
        return self.jacobian_u(self.factor(z)) @ self.S

    def log_measure(self, z):
        return float(self.log_scale + MEASURE_POWERS @ self.coordinates(z))

    def metric(self, z):
        G = self.gram(z)
        x = np.array([G[i,j] for i,j in cp.GKEYS])
        inv = np.linalg.inv(self.jacobian(z))
        return inv @ cp.A_at(x) @ inv.T


def _terms(expr, variables):
    result = {}
    for term in sp.Add.make_args(sp.expand(expr)):
        powers = term.as_powers_dict()
        key = tuple(int(powers.get(v,0)) for v in variables)
        if any(powers.get(v,0) != k for v,k in zip(variables,key)) or min(key[3:]) < 0:
            raise ArithmeticError("operator is not a finite diagonal-Laurent/offdiagonal polynomial")
        c = float(term / sp.prod(v**k for v,k in zip(variables,key)))
        if c:
            result[key] = result.get(key,0.) + c
    return result


@lru_cache(maxsize=1)
def _symbolic_metric():
    r = sp.symbols('r0:3', positive=True)
    a,b,c = sp.symbols('a b c', real=True)
    variables = (*r,a,b,c)
    L = sp.Matrix([[r[0],0,0],[a,r[1],0],[b,c,r[2]]])
    G = L*L.T
    x = sp.Matrix([G[i,j] for i,j in cp.GKEYS])
    J = x.jacobian(variables)*sp.diag(*r,1,1,1)
    A = sp.Matrix(6,6,lambda i,j: sum(int(cp.C_A[i,j,m])*x[m] for m in range(6)))
    inv = J.inv()
    B = (inv*A*inv.T).applyfunc(sp.cancel)
    return variables, L, [[_terms(B[i,j],variables) for j in range(6)] for i in range(6)]


def laurent_model(s,t,g):
    variables,L,metric = _symbolic_metric()
    om = bm.omega(t)
    a,b = bm.mc_sources(s,t)
    co = L.cofactor_matrix()
    V = sum(sp.Float(om[i]**2/2)*sum(L[i,j]**2 for j in range(3)) for i in range(3))
    V -= sp.Float(g*om.sum())*L.det()
    V += sp.Float(g*g/2)*sum(v*v for v in co)
    V -= sp.Float(a/2)*sum(L[0,j]**2 for j in range(3))
    V -= sp.Float(b/2)*sum(L[i,j]**2 for i in (1,2) for j in range(3))
    return metric, _terms(V,variables)


def evaluate_terms(terms, coords):
    return sum(c*math.prod(x**p for x,p in zip(coords,key)) for key,c in terms.items())


def normalized_hermites(x, nmax):
    h = np.ones((nmax+1,len(x)))
    if nmax:
        h[1] = np.sqrt(2.)*x
    for n in range(1,nmax):
        h[n+1] = np.sqrt(2./(n+1))*x*h[n] - np.sqrt(n/(n+1))*h[n-1]
    return h


@lru_cache(maxsize=512)
def one_dimensional_moments(w,alpha,p,nmax):
    """<n| exp(alpha*z) z^p |m> by completed-square polynomial integration."""
    if not np.isfinite([w,alpha]).all() or w <= 0 or any(
            isinstance(v,bool) or not isinstance(v,(int,np.integer)) or v < 0 for v in (p,nmax)):
        raise ValueError("positive frequency, finite exponent and nonnegative integer degrees required")
    if alpha*alpha/(4*w) > 600:
        raise ValueError("exponential moment would exceed the numerical contract")
    order = nmax + 1 + (p+1)//2
    x,weights = roots_hermite(order)
    shifted = x + alpha/(2*np.sqrt(w))
    h = normalized_hermites(shifted,nmax)
    weights = weights/np.sqrt(np.pi)*np.exp(alpha*alpha/(4*w))*(shifted/np.sqrt(w))**p
    result = (h*weights) @ h.T
    if not np.isfinite(result).all():
        raise ArithmeticError("nonfinite Gaussian moment")
    result.setflags(write=False)
    return result


class GaussianMatrices:
    def __init__(self, w, states):
        self.w = np.asarray(w,dtype=float)
        self.states = np.array(states,dtype=int)
        self.nmax = int(self.states.max())

    def matrix(self, alpha, monomial):
        F = np.ones((len(self.states),)*2)
        for j in range(6):
            m = one_dimensional_moments(float(self.w[j]),float(alpha[j]),int(monomial[j]),self.nmax)
            n = self.states[:,j]
            F *= m[n[:,None],n[None,:]]
        if not np.isfinite(F).all():
            raise ArithmeticError("nonfinite multidimensional moment")
        return F


def derivatives(w,Mq):
    """Exact derivative from the trial space into one extra occupation shell."""
    basis = states_up_to(6,Mq)
    extended = states_up_to(6,Mq+1)
    index = {n:i for i,n in enumerate(extended)}
    D = []
    for j in range(6):
        rows,cols,values = [],[],[]
        for col,n in enumerate(basis):
            for sign in (-1,1):
                o = list(n); o[j] += sign
                if o[j] < 0:
                    continue
                rows.append(index[tuple(o)]); cols.append(col)
                values.append(-sign*math.sqrt(w[j]*(n[j]+(sign>0))/2))
        D.append(sparse.csr_matrix((values,(rows,cols)),shape=(len(extended),len(basis))))
    return basis,extended,D


def harmonic_matrix(w,Mq):
    basis,extended,D = derivatives(w,Mq)
    moments = GaussianMatrices(w,extended)
    F0 = moments.matrix(np.zeros(6),ZERO)
    H = sum(.5*(d.T @ F0 @ d) for d in D)
    for j in range(6):
        mon = tuple(2*int(i==j) for i in range(6))
        H += .5*w[j]**2*moments.matrix(np.zeros(6),mon)[:len(basis),:len(basis)]
    return lf.checked_symmetric(H)


def multiply_linear(poly, offset, row):
    out = {}
    for key,c in poly.items():
        if offset:
            out[key] = out.get(key,0.) + c*offset
        for j,value in enumerate(row):
            if value:
                k = list(key); k[j] += 1; k = tuple(k)
                out[k] = out.get(k,0.) + c*value
    return out


def laurent_matrix(key, chart, moments):
    alpha = np.array(key[:3]) @ chart.S[:3]
    scale = np.exp(np.dot(key[:3],chart.u0[:3]))
    poly = {ZERO:float(scale)}
    for i in range(3,6):
        for _ in range(key[i]):
            poly = multiply_linear(poly,chart.u0[i],chart.S[i])
    return sum(c*moments.matrix(alpha,mon) for mon,c in poly.items())


class ConeRitz:
    """Finite interior trial calculation; boundary realization remains unresolved."""
    boundary_status = 'UNRESOLVED_PHYSICAL_REALIZATION'
    classify = lf.LocalFock.classify
    spectrum = lf.LocalFock.spectrum

    def __init__(self,s,t,g,Mq=4):
        if isinstance(Mq,bool) or not isinstance(Mq,int) or not 0 <= Mq <= 6:
            raise ValueError("reviewed same-control resource cap is 0 <= Mq <= 6")
        if not np.isfinite([s,t,g]).all() or t <= 1 or g <= 0 or not 1 < s < bm.mc_s_max(t):
            raise ValueError("positive coupling and stable registered-background regime required")
        self.s,self.t,self.g,self.Mq = s,t,g,Mq
        pt = cp.CondensatePT(s,t,g)
        self.w = pt.w.copy()
        if not np.isfinite(self.w).all() or np.min(self.w) <= 0:
            raise ValueError("background has no stable harmonic normal frame")
        self.chart = ConeChart(cp.mat_from_vec(pt.G0),pt.M)
        self.basis,extended,Dz = derivatives(self.w,Mq)
        N = len(self.basis)
        embedding = sparse.eye(len(extended),N,format='csr')
        Du = [sum(self.chart.invS[j,i]*Dz[j] for j in range(6)) - .5*MEASURE_POWERS[i]*embedding for i in range(6)]
        moments = GaussianMatrices(self.w,extended)
        self.mass = moments.matrix(np.zeros(6),ZERO)[:N,:N]
        if not np.allclose(self.mass,np.eye(N),rtol=0,atol=1e-10):
            raise ArithmeticError("unitary pullback mass is not identity")
        metric,potential = laurent_model(s,t,g)
        groups = {}
        for i in range(6):
            for j in range(i,6):
                for key,c in metric[i][j].items():
                    groups.setdefault(key,{'metric':[],'potential':0.})['metric'].append((i,j,c))
        for key,c in potential.items():
            groups.setdefault(key,{'metric':[],'potential':0.})['potential'] += c
        T,V = np.zeros((N,N)),np.zeros((N,N))
        for key,group in groups.items():
            F = laurent_matrix(key,self.chart,moments)
            for i,j,c in group['metric']:
                item = .5*c*(Du[i].T @ F @ Du[j])
                T += item if i==j else item+item.T
            V += group['potential']*F[:N,:N]
        self.T = lf.checked_symmetric(T)
        if np.linalg.eigvalsh(self.T).min() < -1e-10*max(1.,np.linalg.norm(self.T,2)):
            raise ArithmeticError("kinetic form lost positivity")
        a,b = bm.mc_sources(s,t)
        self.energy_shift = float(bm.V_sourced(self.chart.factor(np.zeros(6)),g,t,a,b))
        self.V = lf.checked_symmetric(V-self.energy_shift*self.mass)
        self.asym = float(np.max(abs(T+V-(T+V).T)))
        self.H = lf.checked_symmetric(self.T+self.V)

    def reference_amplitudes(self,vectors,order):
        """Original linear-Gram harmonic projectors, not coefficients in log coordinates."""
        if not isinstance(order,int) or not 2 <= order <= 12 or vectors.ndim != 2 or vectors.shape[0] != len(self.basis) or vectors.shape[1] > 12:
            raise ValueError("overlap resource contract requires order 2..12 and at most 12 eigenvectors")
        x,weights = roots_hermite(order)
        weights /= np.sqrt(np.pi)
        h = normalized_hermites(x,self.Mq)
        values = np.zeros((self.Mq+1,)*6+(vectors.shape[1],))
        for i,n in enumerate(self.basis):
            values[n] = vectors[i]
        for axis in range(6):
            values = np.moveaxis(np.tensordot(h.T,values,axes=(1,axis)),0,axis)
        values = values.reshape(order**6,-1)
        amplitudes = np.zeros((6,vectors.shape[1]))
        invM = np.linalg.inv(self.chart.M)
        x0 = np.array([self.chart.G0[i,j] for i,j in cp.GKEYS])
        logdet = math.log(8.)+np.linalg.slogdet(self.chart.S)[1]-np.linalg.slogdet(self.chart.M)[1]
        for start in range(0,order**6,32768):
            stop=min(start+32768,order**6)
            ix=np.array(np.unravel_index(np.arange(start,stop),(order,)*6)).T
            z=x[ix]/np.sqrt(self.w)
            u=self.chart.u0+z@self.chart.S.T
            r=np.exp(u[:,:3]); a,b,c=u[:,3],u[:,4],u[:,5]
            G=np.column_stack((r[:,0]**2,a*a+r[:,1]**2,b*b+c*c+r[:,2]**2,
                               r[:,0]*a,r[:,0]*b,a*b+r[:,1]*c))
            y=(G-x0)@invM.T
            log_ratio=.5*(logdet+u[:,:3]@np.array([4.,3.,2.])-np.sum(self.w*(y*y-z*z),axis=1))
            if not np.isfinite(log_ratio).all() or np.max(log_ratio)>600:
                raise ArithmeticError("unresolved reference-overlap numerical range")
            measure=np.prod(weights[ix],axis=1)*np.exp(log_ratio)
            targets=y*np.sqrt(2*self.w)
            amplitudes += (targets*measure[:,None]).T @ values[start:stop]
        return amplitudes

    def helike_and_competitor(self,nlev=12):
        e,v=self.spectrum(nlev)
        wh=math.sqrt(bm.mc_helike_freq2(self.s,self.t))
        hel=[j for j in range(6) if abs(self.w[j]-wh)<1e-6]
        single=[j for j in range(6) if j not in hel and np.count_nonzero(abs(self.w-self.w[j])<1e-6)==1]
        if len(hel)!=2 or not single or len(e)<2:
            raise ArithmeticError("ambiguous harmonic reference channel")
        comp=min(single,key=lambda j:self.w[j])
        coarse=self.reference_amplitudes(v,10)
        fine=self.reference_amplitudes(v,12)
        error=float(max(np.max(abs(coarse*coarse-fine*fine)),
                        np.max(abs(np.sum(coarse[hel]**2,axis=0)-np.sum(fine[hel]**2,axis=0)))))
        if error>1e-6 or np.max(np.sum(fine*fine,axis=0))>1+1e-6:
            raise ArithmeticError(f"reference weights unresolved at frozen overlap tolerance: {error}")
        out=dict(E0=float(e[0]),scope='positive-cone interior trial values; no boundary or sector convergence certificate',
                 boundary_status=self.boundary_status,energy_shift=self.energy_shift,reference_weight_error=error,
                 overlap_orders=[10,12])
        for name,modes in (('helike',hel),('competitor',[comp])):
            weights=np.sum(fine[modes]**2,axis=0)
            best=1+int(np.argmax(weights[1:]))
            out[name]=dict(gap=float(e[best]-e[0]),weight=float(weights[best]),
                           log_basis_weight=float(self.classify(v[:,best])[modes].sum()),level_index=best)
        return out
