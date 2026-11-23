import sys, numpy as np, sympy as sp
sys.path.insert(0,'./scripts')
import ricci_su2_exact_edge as E
bl=E.Blocks(); ch=bl.ch
x,y,t=E.x,E.y,E.t
# (a) table numerically at random point
rng=np.random.default_rng(7)
for _ in range(3):
    xv,yv,tv=rng.uniform(-2,2),rng.uniform(0,3),rng.uniform(1,9)
    w,C,Op,Om=bl.data(xv,yv,tv); s=np.sqrt(tv)
    tab={(-1,-1):(-s+xv+2/s,4*s*xv-tv-yv**2+4),(-1,0):((2-tv)/s,-tv+2*yv**2+4),(-1,1):(-s-xv+2/s,-4*s*xv-tv+3*yv**2+4),
     (0,-1):(xv,4*tv+yv**2),(0,0):(0,4*tv+2*yv**2),(0,1):(-xv,4*tv+yv**2),(1,-1):(s+xv-2/s,-4*s*xv-tv+3*yv**2+4),
     (1,0):((tv-2)/s,-tv+2*yv**2+4),(1,1):(s-xv-2/s,4*s*xv-tv-yv**2+4)}
    err=max(max(abs(w[k]-tab[c][0]),abs(C[k,k]-tab[c][1])) for k,c in enumerate(ch))
    print('table err',err)
# (b) mirror q vs -q at off-grid points
worst=0
for _ in range(6):
    xv,yv,tv=rng.uniform(-2,2),rng.uniform(0.1,3),rng.uniform(1.5,9); j=int(rng.integers(1,60))
    d=bl.data(xv,yv,tv); l=j/2
    for q in np.arange(-l-2,l+2+1e-9,1.0):
        if q<=0: continue
        _,B1=bl.block(d,j,q,tv); _,B2=bl.block(d,j,-q,tv)
        e1,e2=np.linalg.eigvalsh(B1),np.linalg.eigvalsh(B2)
        worst=max(worst,np.abs(e1-e2).max()/max(1,np.abs(e1).max()))
print('mirror q<->-q worst rel',worst)
# (c) sister at y=0, off-grid x
for xv,tv,j in [(0.37,2.7,7),(1.3,5.5,40),(0.11,8.3,399),(-0.8,3.3,20)]:
    d=bl.data(xv,0.0,tv); l=j/2
    st,B=bl.block(d,j,l,tv)
    k=[i for i,(m,a) in enumerate(st) if abs(m-l)<1e-9 and ch[a]==(1,-1)][0]
    v=np.zeros(len(st)); v[k]=1
    r=B@v; lam=r[k].real
    lamneg=E.lam_tw(-xv,0,tv,j)
    print('y0 sister',xv,tv,j,'resid',np.linalg.norm(r-lam*v),'val-lam(-x)',lam-lamneg, 'states',[(m,ch[a]) for m,a in st])
# also y!=0 residual (should be nonzero)
d=bl.data(0.37,1.0,2.7); st,B=bl.block(d,7,3.5,2.7)
k=[i for i,(m,a) in enumerate(st) if abs(m-3.5)<1e-9 and ch[a]==(1,-1)][0]
v=np.zeros(len(st));v[k]=1;r=B@v;print('y=1 resid',np.linalg.norm(r-r[k]*v))
# (d) across-reaching states per block near top
for l in (1,5,100):
    for q in (l,-l,l+1,l-1,-(l-1)):
        st=[(q-a-b,(a,b)) for (a,b) in ch if abs(q-a-b)<=l]
        acr=[s_ for s_ in st if s_[1][1]*s_[0]*1>0 and False or s_[1][1]*s_[0]<0]
        print('l',l,'q',q,'across(x>0):',acr)
