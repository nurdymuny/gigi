"""How much can the twist grow before its price turns positive along the family flow?
Frozen-rate estimate: integrate -lambda(tau) over the time the sector-0 price is negative (t = 3, 3.55, where j = 0
is her cheapest sector). Convention: linearised gradient flow d(delta)/dtau = -Hess delta, growth rate -lambda.
"""
import numpy as np, sympy as sp
from scipy.integrate import solve_ivp
x,y,t,j=sp.symbols('x y t j',real=True); st=sp.sqrt(t)
B=y**2-2*st*x; c=y*(x-2/st); E=sp.Rational(1,2)*(B**2+2*c**2)
fx=sp.lambdify((x,y,t),-sp.diff(E,x)); fy=sp.lambdify((x,y,t),-sp.diff(E,y)/2)
pf=sp.lambdify((x,y,t,j),(x+(j+2)/st)**2+2*st*x-y**2)
for T in [3.0,3.55]:
    x0,y0=(2-T)/np.sqrt(T),np.sqrt(T)
    sol=solve_ivp(lambda s,v:[fx(v[0],v[1],T),fy(v[0],v[1],T)],[0,1],[x0,y0],dense_output=True,rtol=1e-11,atol=1e-12)
    ss=np.linspace(0,1,100001); lam=np.array([pf(*sol.sol(s),T,0) for s in ss])
    I=np.trapezoid(np.where(lam<0,-lam,0),ss)
    print(f't={T}: sector-0 price {lam[0]:+.3f} -> {lam[-1]:+.3f} at tau=1; growth integral {I:.4f}, factor e^I = {np.exp(I):.3f}')
