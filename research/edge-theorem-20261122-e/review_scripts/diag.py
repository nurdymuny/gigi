import sympy as sp, itertools
x,y,l=sp.symbols('x y l',real=True); t=sp.symbols('t',positive=True); s=sp.sqrt(t)
W={(1,-1):s+x-2/s,(1,0):(t-2)/s,(1,1):s-x-2/s,(0,-1):x,(0,0):0,(0,1):-x}
C={(1,-1):-4*s*x-t+3*y**2+4,(1,0):-t+2*y**2+4,(1,1):4*s*x-t-y**2+4,(0,-1):4*t+y**2,(0,0):4*t+2*y**2,(0,1):4*t+y**2}
D=lambda a,m:(W[a]-2*m/s)**2+4*(l*(l+1)-m**2)+C[a]
for a in [(1,1),(1,0),(0,-1),(0,0),(0,1)]:
    df=sp.expand(D((1,-1),l)-D(a,l)); sol=sp.solve(df,l)
    print('sister - ',a,':',sp.simplify(df),' root',[sp.simplify(r) for r in sol])
print('her - (1,0):',sp.solve(sp.expand(D((1,1),l)-D((1,0),l)),l))
import math
ld=lambda X,Y,T: Y*Y*math.sqrt(T)/(2*X)-T/2-1
l10=lambda X,Y,T:(X*X+Y*Y)*math.sqrt(T)/(4*X)-T/2-1
n=0;bad=0;rows=[]
for X in (0.05,0.2,0.5,1.0,2.0):
  for Y in (0,0.5,1.5,3.0):
    for T in (2,3,4,6,10):
      a,b=ld(X,Y,T),l10(X,Y,T)
      e1=2*max(a,0)<=300; e2=a>=5 and 2*a<=300
      t2=(-4*math.sqrt(2)*X+2*Y*Y-6) if T==2 else None
      if e2:
        n+=1; bad+= b<0.5*a
      rows.append((X,Y,T,round(a,1),round(b,1),e1,e2,t2))
print('E2-eligible',n,'with l10<0.5 l_d:',bad)
for r in rows:
  if r[6] or r[2]==2: print(r)
print('E1 in-range count',sum(r[5] for r in rows))
print('E1 out-of-range', [r[:5] for r in rows if not r[5]])
