import sys
sys.path.insert(0,'./scripts')
sys.argv=[sys.argv[0]]
import numpy as np
import ricci_su2_exact_edge as EE
T2=EE.T2
bl=EE.Blocks(); S,C2,eps=T2.su2_data()
rng=np.random.default_rng(7); worst=0
pts=[(-3,0.5,10,20),(-3,3,10,25),(-1.5,3,3,1),(-2.2,1.7,9.3,31)]+[(rng.uniform(-3,-1.5),rng.uniform(0.5,3),rng.uniform(3,10),int(rng.integers(0,40))) for _ in range(6)]
for xv,yv,tv,jv in pts:
    H,G=T2.hessian_and_gauge(C2,[tv,1,1],eps,[xv*np.eye(3)[0],yv*np.eye(3)[1],yv*np.eye(3)[2]],S.spin_rep(jv))
    ref=np.linalg.eigvalsh(H+G@G.conj().T); got=bl.sector_spectrum(xv,yv,tv,jv)
    r=float(np.abs(np.sort(ref)-got).max()/max(1,np.abs(ref).max())); worst=max(worst,r)
    lam=EE.lam_tw(xv,yv,tv,jv); print(jv, r, 'lowest two minus lam', ref[:3]-lam)
print('worst',worst)
