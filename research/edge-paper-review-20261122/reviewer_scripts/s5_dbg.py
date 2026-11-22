import sys, numpy as np
sys.path.insert(0,'/Users/bdavis/.claude/jobs/c8f0f39d/tmp/rtc/scripts')
import ricci_su3_test2_family as T2
S,C,eps = T2.su2_data()
def lam(x,y,t,j): return (x+(j+2)/np.sqrt(t))**2+2*np.sqrt(t)*x-y**2
x,y,t,j=0.37,1.3,2.7,6
g=[t,1,1]; M=[np.array([x,0,0.]),np.array([0,y,0.]),np.array([0,0,y])]
rho=S.spin_rep(j); H,G=T2.hessian_and_gauge(C,g,eps,M,rho); K=H+G@G.conj().T
print(np.round(np.diag(rho[0]),3), lam(x,y,t,j))
us={+1:np.array([0,1,1j])/np.sqrt(2), -1:np.array([0,1,-1j])/np.sqrt(2)}
d=j+1
for k in (0,d-1):
  for sf in (1,-1):
    for sl in (1,-1):
      e=np.kron(np.kron(us[sf],us[sl]),np.eye(d)[k]); r=K@e; val=e.conj()@r
      print(k,sf,sl,val.real,np.linalg.norm(r-val*e))
print(np.sort(np.linalg.eigvalsh(K))[:5])
