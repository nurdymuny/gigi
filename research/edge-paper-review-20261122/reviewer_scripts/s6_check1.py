import sys, inspect
sys.path.insert(0,'/Users/bdavis/.claude/jobs/c8f0f39d/tmp/rtc/scripts')
sys.argv=[sys.argv[0]]
import ricci_su2_family_block_cert as FB
import mpmath, numpy as np, sympy as sp
S=FB.symbolic()
# print lambdify source of a dshift and a C entry to see constant handling
for a in range(9):
    src=inspect.getsource(S['dshift'][a]); print(a, src.split('return')[1].strip()[:300])
print(inspect.getsource(S['C'][0][4]))
# symmetry check of interval block at a point
iv=mpmath.iv
for j in (1,2,5):
  l=j/2
  for q in np.arange(-l-1,l+1+1e-9,1.0):
    st,B=FB.block_iv(j,float(q),iv.mpf(-2.1),iv.mpf(1.3),iv.mpf(np.sqrt(5.0)),shifted=True)
    Bm=np.array([[float(mpmath.mpf(e.mid)) for e in r] for r in B])
    print(j,q,np.abs(Bm-Bm.T).max())
