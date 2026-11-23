import json, ast, time, sympy as sp
x,y,s=sp.symbols('x y s')
d=json.load(open('./research/edge-theorem-20261121/wp1_block_polynomials.json'))['blocks']
paper="2*s**8*x-5*s**7*x**2-s**7*y**2-s**6*x**3+7*s**6*x*y**2+s**5*x**4+2*s**5*x**2*y**2-23*s**5*x**2-s**5*y**4+7*s**5*y**2-3*s**4*x**3*y**2+9*s**4*x**3+3*s**4*x*y**4-7*s**4*x*y**2-30*s**4*x+2*s**3*x**4-10*s**3*x**2*y**2+4*s**3*x**2+6*s**3*y**4+2*s**3*y**2+20*s**2*x**3-28*s**2*x*y**2-20*s**2*x+56*s*x**2-40*s*y**2+48*x"
pp=sp.sympify(paper)
for k in ['j=1,q=1.5','j=2,q=2','j=12,q=7','j=2,q=1','j=12,q=6','j=1,q=0.5','j=3,q=0.5']:
    fs=d[k]['factors']; fs=ast.literal_eval(fs) if isinstance(fs,str) else fs
    for f in fs:
        if f['degrees_(x,y,s)']==[0,0,1]: continue
        e=sp.sympify(f['expr'],locals={'x':x,'y':y,'s':s}); P=sp.Poly(e,x,y,s)
        t0=time.time(); fl=sp.factor_list(e)
        print(k,f['degrees_(x,y,s)'],len(P.terms()),'nfactors',len(fl[1]),'even y',sp.expand(e-e.subs(y,-y))==0,'%.1fs'%(time.time()-t0))
        if k=='j=1,q=1.5': print(' paper B13/2 matches:', sp.expand(e-pp)==0, sp.expand(e+pp)==0)
