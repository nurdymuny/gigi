import sys, numpy as np
sys.path.insert(0,'/Users/bdavis/.claude/jobs/c8f0f39d/tmp/rtc/scripts')
_a=sys.argv; sys.argv=[_a[0]]
import ricci_su2_exact_edge as EE
import ricci_su3_test2_family as T2
sys.argv=_a
bl=EE.Blocks()
x,y,t=-0.62460,0.25,2.01
print('x12*',EE.edge(bl.phi_A,y,t,lo=-3,hi=1,n=161),'phiA',bl.phi_A(x,y,t))
d=bl.data(x,y,t); _,B=bl.block(d,16,8.0,t); print('blk',np.linalg.eigvalsh(B)[0]-EE.lam_tw(x,y,t,16))
S,C,eps=T2.su2_data(); M=[np.array([x,0,0.]),np.array([0,y,0.]),np.array([0,0,y])]
H,G=T2.hessian_and_gauge(C,[t,1,1],eps,M,S.spin_rep(16)); ev=np.linalg.eigvalsh(H+G@G.conj().T)-EE.lam_tw(x,y,t,16); print('full',ev[:4])
# Thm slice: x* check numeric
for yv in (np.sqrt(3),1.5):
    print('slice edge',EE.edge(bl.phi_A,yv,3.0,lo=-3,hi=1,n=161))
# worked example threshold numbers
x,y,t=0.1,np.sqrt(10),10
for j in (120,):
    d=bl.data(x,y,t); l=j/2; lam=EE.lam_tw(x,y,t,j)
    print(min((np.linalg.eigvalsh(bl.block(d,j,q,t)[1])[0]-lam,q) for q in np.arange(-l-1,l+2)))
