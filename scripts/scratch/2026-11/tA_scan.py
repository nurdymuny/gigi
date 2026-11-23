import sys, numpy as np
sys.path.insert(0, '.')
import ricci_su2_charge_block_direct as D
lam=lambda t,j:(j*(j+2)-2*(t-3)*j-2*(t-2)*(t+4))/t
tA=2.7311222782381136231
worst={}
for t in np.concatenate([np.linspace(tA+1e-6, 2.75, 60), [tA-1e-4, tA-1e-3]]):
    fd=D.frame_data(t)
    for j in range(0,41):
        l=j/2; m=np.inf; nb=0
        for q in np.arange(-l-2,l+2+1e-9,1.0):
            if abs(abs(q)-(l+2))<1e-9: continue
            st,B=D.block(t,j,q,fd)
            if B.size:
                e=np.linalg.eigvalsh(B); m=min(m,e.min()-lam(t,j)); nb+=int((e<lam(t,j)-1e-9).sum())
        key=('above' if t>tA else 'below', j)
        worst[key]=min(worst.get(key,np.inf),m)
        if t>tA and nb: print("VIOLATION", t, j, m)
print("t > tA, all j <= 40: smallest (block min - lambda) =", min(v for k,v in worst.items() if k[0]=='above'), "at j =", min((v,k[1]) for k,v in worst.items() if k[0]=='above')[1])
print("t just below tA: sectors with a block below lambda:", sorted(k[1] for k,v in worst.items() if k[0]=='below' and v<0))
