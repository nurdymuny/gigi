import json, numpy as np, sympy as sp
R='./research/'
x,y,s=sp.symbols('x y s')
txt=open(R+'su2-floor-family-20261119/edge_polynomial.txt').read().splitlines()
L=[l.strip() for l in txt if l.strip().startswith(('2*s**12','-8*s**16'))]
P1=sp.Poly(sp.sympify(L[0]),x); P2=sp.Poly(sp.sympify(L[1]),x)
d=json.load(open(R+'edge-theorem-20261121/wp1_poly_check_final.json'))
dr=[];dc=[]
for r in d['rows']:
    if tuple(r['competitor'])!=(4,0.0): continue
    sub={y:r['y'],s:np.sqrt(r['t'])}
    c=[float(sp.N(cc.subs(sub))) for cc in P1.all_coeffs()]
    rt=np.roots(c); e=r['edge_x']
    dc.append(min(abs(rt-e))); re=[z.real for z in rt if abs(z.imag)<1e-9]
    dr.append(min([abs(z-e) for z in re]) if re else np.inf)
print(len(dr),'P1 nearest real root dist min/max',min(dr),max(dr),' complex min',min(dc))
rows=[r for r in d['rows'] if tuple(r['competitor'])==(4,0.0)]
for r,v in sorted(zip(rows,dr),key=lambda a:a[1])[:4]: print(r['t'],r['y'],r['edge_x'],v)
