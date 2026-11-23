import sys, numpy as np
sys.path.insert(0,'./scripts')
import ricci_su3_test2_family as T2
S,C,eps = T2.su2_data()
def lam(x,y,t,j): return (x+(j+2)/np.sqrt(t))**2+2*np.sqrt(t)*x-y**2
def run(x,y,t,j):
    g=[t,1,1]; M=[np.array([x,0,0.]),np.array([0,y,0.]),np.array([0,0,y])]
    rho=S.spin_rep(j); H,G=T2.hessian_and_gauge(C,g,eps,M,rho)
    K=H+G@G.conj().T
    d=j+1; l=j/2
    A=(1j*rho[0]/2); A=(A+A.conj().T)/2; w,V=np.linalg.eigh(A)
    tops=[V[:,0],V[:,-1]]
    us={+1:np.array([0,1,1j])/np.sqrt(2), -1:np.array([0,1,-1j])/np.sqrt(2), 0:np.array([1,0,0])}
    st=lambda v,cf,cl: np.kron(np.kron(us[cf],us[cl]),v)
    best=None
    for v in tops:
        for s in (1,-1):
            e=st(v,s,s); r=K@e; val=(e.conj()@r)
            if np.linalg.norm(r-val*e)<1e-8 and abs(val.real-lam(x,y,t,j))<1e-8: best=(v,s)
    v,s=best
    em=st(v,s,-s); e0=st(v,s,0); sq=np.sqrt(t)
    f1=-8*l*x/sq-4*sq*x+4*y**2-8*x/sq
    f2=-4*l*x/sq-2*sq*x-x**2+3*y**2-4*x/sq
    L=lam(x,y,t,j)
    r1=(em.conj()@K@em).real-L; r2=(e0.conj()@K@e0).real-L
    gm=np.linalg.norm(G.conj().T@em); g0=np.linalg.norm(G.conj().T@e0)
    U,sv,Vh=np.linalg.svd(G.conj().T); rank=(sv>1e-9).sum(); Vk=Vh[rank:].conj().T
    def projRQ(e):
        p=Vk@(Vk.conj().T@e); return (p.conj()@H@p).real/(p.conj()@p).real-L
    print(f"j={j} x={x}: e- {r1:.12g} formula {f1:.12g} | e0 {r2:.12g} formula {f2:.12g} | |G*e-|={gm:.3g} |G*e0|={g0:.3g} | slice RQ of P e-: {projRQ(em):.6g}, P e0: {projRQ(e0):.6g}")
for (x,y,t,j) in [(0.37,1.3,2.7,6),(-1.2,0.8,5.0,5),(0.1,np.sqrt(10),10,40),(0.5,0.5,4.0,11)]:
    run(x,y,t,j)
x,y,t=-3.3,1.2,4.0
g=[t,1,1]; M=[np.array([x,0,0.]),np.array([0,y,0.]),np.array([0,0,y])]
H,G=T2.hessian_and_gauge(C,g,eps,M,S.spin_rep(0)); v=np.zeros(9); v[0]=1
print('v RQ', (v@H@v).real, 4*t+2*y**2, np.linalg.norm(G.conj().T@v))
