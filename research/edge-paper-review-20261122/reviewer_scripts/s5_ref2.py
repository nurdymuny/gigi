import sys, numpy as np
sys.path.insert(0,'/Users/bdavis/.claude/jobs/c8f0f39d/tmp/rtc/scripts')
_a=sys.argv; sys.argv=[_a[0]]
import ricci_su2_exact_edge as EE
sys.argv=_a
bl=EE.Blocks(); y,t=0.25,2.01
for x in (-0.6245982106562898,-0.62460,-0.6246982,-0.6247):
    d=bl.data(x,y,t); _,B=bl.block(d,16,8.0,t); print(x,np.linalg.eigvalsh(B)[0]-EE.lam_tw(x,y,t,16), bl.phi_A(x,y,t))
y,t=0.25,2.001
for x in (-0.64263,-0.64513):
    d=bl.data(x,y,t); _,B=bl.block(d,58,29.0,t); print(x,np.linalg.eigvalsh(B)[0]-EE.lam_tw(x,y,t,58), min(np.linalg.eigvalsh(bl.block(d,58,q,t)[1])[0]-EE.lam_tw(x,y,t,58) for q in (30.,29.,28.)))
