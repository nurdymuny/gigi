import sys, numpy as np, math
from scipy.optimize import brentq
sys.path.insert(0,'./scripts')
import ricci_su2_exact_edge as E
bl=E.Blocks(); ch=bl.ch
def g(l,xv,yv,tv,d):
    # sister row second-order estimate minus lambda_j (her), using real-l via block at integer/half l
    j=2*l; st,B=bl.block(d,j,l,tv)
    k=[i for i,(m,a) in enumerate(st) if abs(m-l)<1e-9 and ch[a]==(1,-1)][0]
    Ds=B[k,k].real; corr=sum(abs(B[i,k])**2/(B[i,i].real-Ds) for i in range(len(st)) if i!=k)
    return Ds-corr-E.lam_tw(xv,yv,tv,j), Ds-E.lam_tw(xv,yv,tv,j)
# check Omega+ magnitude ~ sqrt2 y
d=bl.data(0.3,1.7,3.3); w,C,Op,Om=d
print('Op (1,0)<-(1,-1):',abs(Op[ch.index((1,0)),ch.index((1,-1))]),'sqrt2 y',math.sqrt(2)*1.7)
ld=lambda X,Y,T: Y*Y*math.sqrt(T)/(2*X)-T/2-1
print('x y t  l_d  first half-integer l with 2nd-order sister<her  ratio  (t-2)/(2(t-1))')
for X in (0.05,0.2,0.5,1.0,2.0):
  for Y in (0,0.5,1.5,3.0):
    for T in (2,3,4,6,10):
      a=ld(X,Y,T)
      if not (a>=5 and 2*a<=300): continue
      d=bl.data(X,Y,T)
      l2=None
      for jj in range(2,401):
        l=jj/2
        if g(l,X,Y,T,d)[0]<0: l2=l;break
      print(X,Y,T,round(a,1),l2, None if l2 is None else round(l2/a,2), round((T-2)/(2*(T-1)),2))
print('--- persistent: last l with 2nd-order >=0, plus diagonal-only last l')
for X in (0.05,0.2,0.5,1.0,2.0):
  for Y in (1.5,3.0):
    for T in (2,3,4,6,10):
      a=ld(X,Y,T)
      if not (a>=5 and 2*a<=300): continue
      d=bl.data(X,Y,T); last2=lastd=None; minratio=9
      for jj in range(2,401):
        l=jj/2; v2,vd=g(l,X,Y,T,d)
        if v2>=0: last2=l
        if vd>=0: lastd=l
      print(X,Y,T,'l_d',round(a,1),'diag-last',lastd,'pt2-last',last2,'ratio',None if last2 is None else round(last2/a,2),'pred',round((T-2)/(2*(T-1)),2))
