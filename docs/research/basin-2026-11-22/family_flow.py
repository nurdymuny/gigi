import numpy as np, sympy as sp
from scipy.integrate import solve_ivp
x,y,t,j=sp.symbols('x y t j',real=True); st=sp.sqrt(t)
B=y**2-2*st*x; c=y*(x-2/st)
E=sp.Rational(1,2)*(B**2+2*c**2)
# L2 metric on the family: |A|^2 = x^2 + 2 y^2  -> gradient flow
fx=sp.lambdify((x,y,t),-sp.diff(E,x)); fy=sp.lambdify((x,y,t),-sp.diff(E,y)/2)
Ef=sp.lambdify((x,y,t),E)
price=(x+(j+2)/st)**2+2*st*x-y**2
pf=sp.lambdify((x,y,t,j),price)
# checks: flat members
for name,(xx,yy) in {'A=0':(0,0),'A=theta':(2/st,2)}.items():
    print(name,'E=',sp.simplify(E.subs({x:xx,y:yy})),' price=',sp.factor(sp.simplify(price.subs({x:xx,y:yy}))))
for T in [3.55,3.0]:
    x0,y0=(2-T)/np.sqrt(T),np.sqrt(T)
    sol=solve_ivp(lambda s,v:[fx(v[0],v[1],T),fy(v[0],v[1],T)],[0,2],[x0,y0],dense_output=True,rtol=1e-10,atol=1e-12)
    print(f'\nt={T}: E(0)={Ef(x0,y0,T):.3f}')
    for s in [0,0.01,0.02,0.05,0.1,0.2,0.5,1,2]:
        xx,yy=sol.sol(s); jstar=max(0,round(T-4))
        lows=[pf(xx,yy,T,jj) for jj in range(0,13)]
        print(f' tau={s:<5} x={xx:+.4f} y={yy:.4f} E={Ef(xx,yy,T):9.4f}  B={yy**2-2*np.sqrt(T)*xx:+.4f}  her min over j<=12 = {min(lows):+.4f} (j={int(np.argmin(lows))})')
    # first time her lowest price crosses zero
    ss=np.linspace(0,2,20001); m=np.array([min(pf(*sol.sol(s),T,jj) for jj in range(13)) for s in ss])
    k=np.argmax(m>0); print(' her price (min over j<=12) turns positive at tau ~',ss[k], ' energy there', Ef(*sol.sol(ss[k]),T))
