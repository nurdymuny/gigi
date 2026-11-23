import re, sympy as sp
from sympy import symbols,sympify,Poly,expand,Rational,Symbol,Add,Mul,Integer,Pow
L=open('$SCRATCH/lean/docs/papers/berger-spectrum/lean/Helike/Edge.lean').read()
R='./research/su2-floor-family-20261119/'
x,y,s=symbols('x y s')
defs={}
for m in re.finditer(r'def (\w+) \(([^)]*)\) : ℝ :=\n(.*?)(?=\n\n|\n/--|\ntheorem)',L,re.S):
    defs[m.group(1)]=m.group(3).strip()
print(list(defs))
loc={'x':x,'y':y,'s':s}
parts=[sympify(defs['edgeDetPart%d'%i].replace('^','**'),locals=loc) for i in range(1,11)]
print('part term counts',[len(Poly(p,x,y,s).terms()) for p in parts])
D=sum(parts)
P=eval(open(R+'edge_polynomial.srepr').read()).subs({Symbol('x',real=True):x,Symbol('y',real=True):y,Symbol('s',positive=True):s})
print('Lean edgeDet - srepr:', expand(D-P))
txt=open(R+'edge_polynomial.txt').read().splitlines()
Ls=[l.strip() for l in txt if l.strip().startswith(('2*s**12','-8*s**16'))]
print('P1',expand(sympify(defs['edgeP1'].replace('^','**'),locals=loc)-sympify(Ls[0],locals=loc)))
print('P2',expand(sympify(defs['edgeP2'].replace('^','**'),locals=loc)-sympify(Ls[1],locals=loc)))
Q=sympify(defs['tAPoly'].replace('^','**'),locals=loc)
print('count roots s>1.6527:',Poly(Q,s).count_roots(Rational(16527,10000),None), ' in [1.65261,1.65262]:',Poly(Q,s).count_roots(Rational(165261,100000),Rational(165262,100000)))
print(Poly(Q,s).intervals(eps=Rational(1,10**20))[-1])
