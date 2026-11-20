"""Where does the two-ring force's non-variational part live? (follow-up to T1, 19 Nov 2026)

Smooth basis: eigenvectors of the native force's symmetric part (mass-whitened), split into the near-zero
(gauge-dominated) cluster |w| < 1 and physical windows 1 <= |w| <= 60, 200. For the two-ring G = S + W (W skew):
  skew_within  = ||V^T W V|| / ||V^T G V||   (non-variational part as seen by smooth tests)
  skew_total   = ||W V|| / ||G V||            (including smooth-in, rough-out coupling)
  leak         = ||(I - VV^T) G V|| / ||G V||
usage: python scripts/ricci_su2_skew_smooth_modes.py   -> research/henrici-t1-20261119/skew_on_smooth_modes.json
"""
import sys, json, numpy as np
sys.path.insert(0,'scripts')
import ricci_su2_henrici_t1 as T
from scipy.linalg import eigh
res={}
for lv in (0,1,2):
    Gn=T.whitened(lv,'native'); Gp=T.whitened(lv,'pg')
    w,V=eigh(0.5*(Gn+Gn.T))
    Wp=0.5*(Gp-Gp.T); r={}
    for name,mask in (("near-zero |w|<1",np.abs(w)<1),("physical 1<=|w|<=200",(np.abs(w)>=1)&(w<=200)),("physical 1<=|w|<=60",(np.abs(w)>=1)&(w<=60))):
        Vs=V[:,mask]; GV=Gp@Vs; WV=Wp@Vs
        r[name]={"k":int(Vs.shape[1]),"skew_within_over_force_within":float(np.linalg.norm(Vs.T@WV)/np.linalg.norm(Vs.T@GV)),
                 "skew_total_over_force_total":float(np.linalg.norm(WV)/np.linalg.norm(GV)),
                 "force_leak_out_of_smooth":float(np.linalg.norm(GV-Vs@(Vs.T@GV))/np.linalg.norm(GV))}
    res[f"n={lv+1}"]=r; print(f"n={lv+1}",json.dumps(r),flush=True)
json.dump(res,open('research/henrici-t1-20261119/skew_on_smooth_modes.json','w'),indent=1)
