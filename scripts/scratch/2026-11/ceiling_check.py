import sys, numpy as np
sys.path.insert(0,'.')
import ricci_su2_test2_family as T2, ricci_su2_soft_mode_flow as F, ricci_su2_meron_family_soft as MF
grid=[t for t in T2.GRID if t<=1.0+1e-9]
down=[t for t in T2.GRID[::-1] if t<=1.0]
bg={float(t):x.reshape(3,3) for t,x in zip(down, MF.follow(down))}
Qs={j:np.linalg.eigh(T2.charge(j)) for j in range(13)}
notop=[]; worst=np.inf
for t in map(float,grid):
    s,c=F.structure(t); m=bg[t]
    for j in range(13):
        H,G=T2.h_and_g(m,s,c,j); q,U=Qs[j]; b=U[:,np.abs(q-(j/2+2))<1e-8][:,0]
        lb=(b.conj()@H@b).real
        Uf,sv,_=np.linalg.svd(G,full_matrices=True); r=int((sv>1e-10*sv.max()).sum()); K=Uf[:,r:]
        mu=np.linalg.eigvalsh(K.conj().T@H@K)
        gap=lb-mu[-1]
        worst=min(worst,gap)
        if gap < -1e-9*max(1,abs(mu[-1])): notop.append((t,j,round(float(gap),4)))
print("points", len(grid)*13, "not top:", len(notop), notop[:15]); print("min (extremal - top):", worst)
