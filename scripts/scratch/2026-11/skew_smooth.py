import sys, numpy as np
sys.path.insert(0,'scripts')
import ricci_su2_henrici_t1 as T
from scipy.linalg import eigh
for lv in (0,1,2):
    Gn=T.whitened(lv,'native'); Gp=T.whitened(lv,'pg')
    Sn=0.5*(Gn+Gn.T); w,V=eigh(Sn)
    out=[]
    for cap in (10,60,200):
        Vs=V[:,w<=cap]
        Sp=0.5*(Gp+Gp.T); Wp=0.5*(Gp-Gp.T)
        GV=Gp@Vs; WV=Wp@Vs
        inn=np.linalg.norm(Vs.T@WV)/np.linalg.norm(Vs.T@GV)
        tot=np.linalg.norm(WV)/np.linalg.norm(GV)
        out.append(f"cap {cap}: k={Vs.shape[1]} skew/force on smooth: within {inn:.3g}, total {tot:.3g}")
    print(f"n={lv+1}", " | ".join(out), flush=True)
