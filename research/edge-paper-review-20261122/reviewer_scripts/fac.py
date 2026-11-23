import sympy as sp, re, time
from sympy import *
R='./research/su2-floor-family-20261119/'
P=eval(open(R+'edge_polynomial.srepr').read())
x=Symbol('x',real=True); y=Symbol('y',real=True); s=Symbol('s',positive=True)
txt=open(R+'edge_polynomial.txt').read().splitlines()
lines=[l.strip() for l in txt if l.strip().startswith(('2*s**12','-8*s**16'))]
P1=sympify(lines[0],locals={'x':x,'y':y,'s':s}); P2=sympify(lines[1],locals={'x':x,'y':y,'s':s})
for name,Q in [('P',P),('P1',P1),('P2',P2)]:
    p=Poly(Q,x,y,s); print(name,len(p.terms()),p.degree(x),p.degree(y),p.degree(s),p.total_degree(), 'even y', expand(Q-Q.subs(y,-y))==0, 'domain',p.domain)
print('P+16384P1P2 =', expand(P+16384*P1*P2))
t0=time.time()
for name,Q in [('P1',P1),('P2',P2)]:
    print(name, factor_list(Q), time.time()-t0)
B=together(expand(P2.subs({x:(2-s**2)/s,y:s})))
Q16=3*s**16+121*s**14+1066*s**12-2836*s**10-18624*s**8+20992*s**6+181248*s**4-425984*s**2+262144
print('restr - claim:', simplify(B+32*(s-1)*(s+1)*Q16))
print(factor(B))
rts=[r for r in Poly(Q16,s).nroots(n=30) if r.is_real and r>0]; print(rts,[r**2 for r in rts])
print(Q16.subs(s,Rational('1.65261')), Q16.subs(s,Rational('1.65262')))
