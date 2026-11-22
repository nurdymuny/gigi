import sys
sys.path.insert(0,'/Users/bdavis/.claude/jobs/c8f0f39d/tmp/rtc/scripts')
sys.argv=[sys.argv[0]]
import numpy as np
import ricci_su2_exact_edge as EE
import ricci_su2_family_block_cert as FB, mpmath
S=FB.symbolic(); print('sqrt in globals is iv.sqrt:', S['Lp'][1][0].__globals__['sqrt'] is mpmath.iv.sqrt)
bl=EE.Blocks()
for j in (0,1,2,3,36):
    l=j/2; best=(1e9,None)
    for xv in np.linspace(-3,-1.5,7):
      for yv in np.linspace(0.5,3,6):
        for tv in np.linspace(3,10,8):
          d=bl.data(xv,yv,tv); lam=EE.lam_tw(xv,yv,tv,j)
          for q in np.arange(-l-2,l+2+1e-9,1.0):
            if abs(abs(q)-(l+2))<1e-9: continue
            _,B=bl.block(d,j,q,tv)
            v=np.linalg.eigvalsh(B)[0]-lam
            if v<best[0]: best=(v,(xv,yv,tv,q))
    print(j,best)
