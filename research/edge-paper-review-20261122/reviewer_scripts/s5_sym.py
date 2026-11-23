import sympy as sp
x,y=sp.symbols('x y',real=True); t,l=sp.symbols('t l',positive=True); s=sp.sqrt(t)
lam=(x+(2*l+2)/s)**2+2*s*x-y**2
w=lambda cf,cl: cf*(t-2)/s-cl*x
C={(1,-1):-4*s*x-t+3*y**2+4,(1,0):-t+2*y**2+4,(0,0):4*t+2*y**2}
D=lambda a,m:(w(*a)-2*m/s)**2+4*(l*(l+1)-m**2)+C[a]
print(sp.simplify(D((1,-1),l)-lam-(-8*l*x/s-4*s*x+4*y**2-8*x/s)))
print(sp.simplify(D((1,0),l)-lam-(-4*l*x/s-2*s*x-x**2+3*y**2-4*x/s)))
print(sp.simplify(D((0,0),0).subs(l,0)))
lam0=lam.subs(l,0); r=sp.solve(sp.Eq(lam0,4*t+2*y**2),x); print([sp.simplify(ri) for ri in r])
# threshold
c=4*y**2-4*s*x-8*x/s; print(sp.solve(sp.Eq(-8*l*x/s+c,0),l))
