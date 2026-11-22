import sympy as sp
x,y=sp.symbols('x y',real=True); s=sp.symbols('s',positive=True); l,xi=sp.symbols('l xi',real=True)
t=s**2
w={(1,1):s-x-2/s,(1,0):(t-2)/s,(0,1):-x,(1,-1):s+x-2/s,(0,0):0}
for a in list(w): w[(-a[0],-a[1])]=-w[a]
Caa={(1,1):4*s*x-t-y**2+4,(1,0):-t+2*y**2+4,(0,1):4*t+y**2,(1,-1):-4*s*x-t+3*y**2+4,(0,0):4*t+2*y**2}
for a in list(Caa): Caa[(-a[0],-a[1])]=Caa[a]
lam=(x+(2*l+2)/s)**2+2*s*x-y**2
m=l-xi
for a in w:
    diag=(w[a]-2*m/s)**2+4*(l*(l+1)-m**2)+Caa[a]-lam
    K=4-8/t-4*(w[a]+x)/s
    E=4*w[a]*xi/s+w[a]**2-x**2-4/t-4*x/s+Caa[a]-2*s*x+y**2   # without -R
    G=4*xi*(2*l-xi)*(1-1/t)+l*K+E
    print(a, sp.simplify(sp.expand(diag-G)))
# twist value check: a=(1,1), xi=0
