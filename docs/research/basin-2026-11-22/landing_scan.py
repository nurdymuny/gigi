import numpy as np, sympy as sp
from scipy.integrate import solve_ivp
x,y,t,j=sp.symbols('x y t j',real=True); st=sp.sqrt(t)
B=y**2-2*st*x; c=y*(x-2/st); E=sp.Rational(1,2)*(B**2+2*c**2)
fx=sp.lambdify((x,y,t),-sp.diff(E,x)); fy=sp.lambdify((x,y,t),-sp.diff(E,y)/2); Ef=sp.lambdify((x,y,t),E)
pf=sp.lambdify((x,y,t,j),(x+(j+2)/st)**2+2*st*x-y**2)
print(' t     end x    end y   2/sqrt t   E_end      lands on   her price zero-crossing tau  E there')
for T in [0.5,0.9,1.0,1.1,1.5,2.0,2.75,3.0,3.55,5,7,10]:
    x0,y0=(2-T)/np.sqrt(T),np.sqrt(T)
    sol=solve_ivp(lambda s,v:[fx(v[0],v[1],T),fy(v[0],v[1],T)],[0,200],[x0,y0],dense_output=True,rtol=1e-10,atol=1e-12,method='LSODA')
    xe,ye=sol.y[:,-1]
    d0=np.hypot(xe,ye); d1=np.hypot(xe-2/np.sqrt(T),ye-2)
    land='A=0' if d0<1e-3 else ('A=theta' if d1<1e-3 else 'other')
    ss=np.linspace(0,5,50001); m=np.array([min(pf(*sol.sol(s),T,jj) for jj in range(13)) for s in ss])
    if m[0]>0: zc='never negative'; ez=''
    else:
        k=np.argmax(m>0); zc=f'{ss[k]:.4f}' if m[k]>0 else 'stays negative'; ez=f'{Ef(*sol.sol(ss[k]),T):.3f}'
    print(f'{T:5.2f} {xe:+8.4f} {ye:8.4f} {2/np.sqrt(T):8.4f} {Ef(xe,ye,T):9.2e}   {land:8s}   {zc:>14s}  {ez}')
