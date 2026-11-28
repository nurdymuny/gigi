import numpy as np, itertools, sys
from numpy.polynomial.hermite_e import hermegauss
t=3.0; om=np.array([np.sqrt(t),1/np.sqrt(t),1/np.sqrt(t)]); S=om.sum()
def cof(q):
    return np.array([[ (q[(i+1)%3,(j+1)%3]*q[(i+2)%3,(j+2)%3]-q[(i+1)%3,(j+2)%3]*q[(i+2)%3,(j+1)%3]) for j in range(3)] for i in range(3)])
def V(qv,g):
    q=qv.reshape(3,3); r=om[:,None]*q-g*cof(q); return 0.5*(r**2).sum()
def gradW(qv,g):
    q=qv.reshape(3,3); return (om[:,None]*q-g*cof(q)).ravel()
x,w=hermegauss(3); w=w/w.sum()
pts=np.array(list(itertools.product(range(3),repeat=9)))
Z=x[pts]; WT=np.prod(w[pts],axis=1)
def EV(c,Sig,g):
    L=np.linalg.cholesky(Sig); Q=c[None,:]+Z@L.T
    q=Q.reshape(-1,3,3)
    c_=np.empty_like(q)
    for i in range(3):
        for j in range(3):
            i1,i2=(i+1)%3,(i+2)%3; j1,j2=(j+1)%3,(j+2)%3
            c_[:,i,j]=q[:,i1,j1]*q[:,i2,j2]-q[:,i1,j2]*q[:,i2,j1]
    r=om[None,:,None]*q-g*c_
    return (0.5*(r**2).sum(axis=(1,2))*WT).sum()
def rot(th):
    a=np.linalg.norm(th)
    if a<1e-14: return np.eye(3)
    k=th/a; K=np.array([[0,-k[2],k[1]],[k[2],0,-k[0]],[-k[1],k[0],0]])
    return np.eye(3)+np.sin(a)*K+(1-np.cos(a))*K@K
def run(g2,alpha,nq=9,cutsig=7):
    g=np.sqrt(g2); u=1/np.sqrt(t)
    a=(np.diag([u,1,1])/g).ravel()
    # Hessian of W at a
    J=np.zeros((9,9)); e=1e-6
    for i in range(9):
        d=np.zeros(9); d[i]=e; J[:,i]=(gradW(a+d,g)-gradW(a-d,g))/(2*e)
    J=(J+J.T)/2; ev,U=np.linalg.eigh(J)
    lam=np.where(np.abs(ev)<1e-6,alpha,np.abs(ev)); A=U@np.diag(lam)@U.T
    def elem(O):
        M=np.kron(np.eye(3),O)  # vec(q O^T)? check below
        return M
    # vec row-major: (qO^T)_{kc}=sum_d q_{kd} O_{cd} -> matrix kron(I,O)
    def pair(O):
        M=np.kron(np.eye(3),O); B=M.T@A@M; b=np.linalg.solve(M,a)
        C=A+B; Sig=np.linalg.inv(C); c=Sig@(A@a+B@b)
        # overlap
        expo=0.5*(c@C@c - a@A@a - b@B@b)
        N1=np.exp(expo)*np.sqrt((2*np.pi)**9/np.linalg.det(C))
        # kinetic 1/2 E[(A(q-a)).(B(q-b))]
        ma=c-a; mb=c-b
        kin=0.5*(np.trace(A@Sig@B)+ (A@ma)@(B@mb))
        pot=EV(c,Sig,g)
        return N1, N1*(kin+pot), expo
    # quadrature over rotation vectors near 0
    N0,H0,_=pair(np.eye(3))
    # width estimate: second derivative of expo in theta
    h=1e-3; ex=[pair(rot(np.array(v)*h))[2] for v in np.eye(3)]
    sig=np.sqrt(np.max([h*h/(-2*x) for x in ex]))
    R=cutsig*sig; gr=np.linspace(-R,R,nq); dv=(gr[1]-gr[0])**3
    num=den=0
    for th in itertools.product(gr,repeat=3):
        th=np.array(th); n=np.linalg.norm(th)
        if n>min(R,np.pi): continue
        jac=(2-2*np.cos(n))/n**2 if n>1e-12 else 1.0
        N1,NH,_=pair(rot(th)); num+=NH*jac; den+=N1*jac
    return H0/N0, num/den, sig
g2=float(sys.argv[1]); alpha=float(sys.argv[2])
r0,r1,sig=run(g2,alpha,nq=int(sys.argv[3]),cutsig=float(sys.argv[4]))
print(f"g2={g2} alpha={alpha} unaveraged={r0:.5f} averaged={r1:.5f} harm={1.5*S+0.5*(np.sqrt(11)-np.sqrt(3)):.5f} sig={sig:.3g}")
