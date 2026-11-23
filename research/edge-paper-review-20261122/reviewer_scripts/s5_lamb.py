import sys, inspect
sys.path.insert(0,'./scripts')
import sympy as sp, mpmath
import edge_wp2_symbolic as WS
ch,w,C,Op,Omn = WS.exact_rows()
x,yp,t = WS.x,WS.yp,WS.t
print(ch)
for name,M in (("dn",(2*sp.I*Omn).applyfunc(sp.simplify)),("up",(2*sp.I*Op).applyfunc(sp.simplify)),("C",C)):
    for i in range(9):
        for k in range(9):
            e=sp.simplify(M[i,k])
            if e!=0:
                f=sp.lambdify((x,yp,t),e,modules=[{"sqrt":mpmath.iv.sqrt},"mpmath"])
                src=inspect.getsource(f).split('return')[1].strip()
                print(name,ch[i],ch[k],e,'|',src)
print('w',w)
print('Cdiag',[sp.simplify(C[i,i]) for i in range(9)])
