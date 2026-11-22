import sympy as sp
x,y,s=sp.symbols('x y s')
txt=open('/Users/bdavis/.claude/jobs/c8f0f39d/tmp/rtc/research/su2-floor-family-20261119/edge_polynomial.txt').read().splitlines()
L=[l.strip() for l in txt if l.strip().startswith('2*s**12')]
print(sp.factor(sp.together(sp.sympify(L[0]).subs({x:(2-s**2)/s,y:s}))))
