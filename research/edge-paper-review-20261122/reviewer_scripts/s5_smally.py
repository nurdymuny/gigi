import sys, numpy as np, time
sys.path.insert(0,'/Users/bdavis/.claude/jobs/c8f0f39d/tmp/rtc/scripts')
_a=sys.argv; sys.argv=[_a[0]]
import ricci_su2_exact_edge as EE
sys.argv=_a
bl=EE.Blocks()
def tailmin(x,y,t,js):
    d=bl.data(x,y,t); best=(np.inf,None)
    for j in js:
        l=j/2; lam=EE.lam_tw(x,y,t,j)
        for q in (l+1,l,l-1,l-2):
            _,B=bl.block(d,j,q,t)
            if B.size:
                v=np.linalg.eigvalsh(B)[0]-lam
                if v<best[0]: best=(v,(j,q))
    return best
js=list(range(13,200))+list(range(200,3001,7))
for (t,y) in [(3,0.25),(3,0.1),(3,0.05),(2.5,0.25),(2.5,0.1),(3,0.01)]:
    t0=time.time()
    xs=EE.edge(bl.phi_A,y,t,lo=-3,hi=1,n=81)
    r=tailmin(xs-1e-4,y,t,js) if xs is not None else None
    print(t,y,'x12*',xs,'tail at x*-1e-4',r, round(time.time()-t0,1),'s',flush=True)
