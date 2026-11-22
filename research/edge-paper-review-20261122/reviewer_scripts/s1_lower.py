import sys; sys.path.insert(0,'/Users/bdavis/.claude/jobs/c8f0f39d/tmp/rtc/scripts')
import numpy as np
import ricci_su2_exact_edge as E
from scipy.optimize import brentq
bl=E.Blocks()
def marg(xv,yv,tv,j=0):
    d=bl.data(xv,yv,tv); l=j/2; lam=E.lam_tw(xv,yv,tv,j); best=(np.inf,None)
    for q in np.arange(-l-2,l+2+1e-9,1.0):
        if abs(abs(q)-(l+2))<1e-9: continue
        st,B=bl.block(d,j,q,tv)
        if B.size:
            v=np.linalg.eigvalsh(B)[0]-lam
            if v<best[0]: best=(v,q,st)
    return best
for (yv,tv) in [(3,3),(0.5,3),(3,10),(0.5,10),(1.5,6)]:
    f=lambda xv: marg(xv,yv,tv)[0]
    xs=np.linspace(-15,-1.5,541); vals=[f(v) for v in xs]
    k=[i for i in range(len(xs)-1) if vals[i]<0<=vals[i+1]]
    xl=brentq(f,xs[k[0]],xs[k[0]+1]) if k else None
    rlo=-(np.sqrt(tv)+2/np.sqrt(tv))-np.sqrt(5*tv+3*yv**2+4)
    b=marg(xl-1e-3,yv,tv)
    # Rayleigh of theta0 tau0 = state (0,(0,0)) in q=0 block
    print((yv,tv),"x_lo(j=0)=",xl,"r_lo=",rlo,"blocking q:",b[1],b[2])
