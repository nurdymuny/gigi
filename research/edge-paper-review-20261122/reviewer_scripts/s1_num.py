import sys; sys.path.insert(0,'./scripts')
import numpy as np
_a,sys.argv=sys.argv,[sys.argv[0]]
import ricci_su3_test2_family as T2
import ricci_su2_exact_edge as E
S,C2,eps=T2.su2_data()
rng=np.random.default_rng(7); worst=0
for _ in range(15):
    xv,yv,tv=rng.uniform(-4,2),rng.uniform(0,3),rng.uniform(0.3,9); j=int(rng.integers(0,7))
    H,G=T2.hessian_and_gauge(C2,[tv,1,1],eps,[xv*np.eye(3)[0],yv*np.eye(3)[1],yv*np.eye(3)[2]],S.spin_rep(j))
    ev=np.linalg.eigvalsh(H+G@G.conj().T); lam=E.lam_tw(xv,yv,tv,j)
    d=np.sort(np.abs(ev-lam)); worst=max(worst,d[1]/max(1,abs(lam)))
    mu=T2.slice_spectrum(H,G); nm=np.sum(np.abs(np.array(mu)-lam)<1e-8*max(1,abs(lam)))
    print(round(xv,3),round(yv,3),round(tv,3),j,"two closest |ev-lam|:",d[:3],"slice copies",nm)
print("worst 2nd-closest",worst)
