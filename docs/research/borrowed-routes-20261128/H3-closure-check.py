import math, itertools
def alpha_direct(d,n,ml):
    a=ml/(n+1)
    pts=list(itertools.product(range(1,n+1),repeat=d)); idx={p:i for i,p in enumerate(pts)}
    nb=[];bb=[]
    for p in pts:
        L=[];c=0
        for k in range(d):
            for s in (-1,1):
                q=list(p); q[k]+=s; q=tuple(q)
                if q in idx: L.append(idx[q])
                else: c+=1
        nb.append(L);bb.append(c)
    u=[0.0]*len(pts); D=2*d+a*a
    for it in range(4000):
        delta=0
        for i in range(len(pts)):
            v=(sum(u[j] for j in nb[i])+bb[i])/D
            delta=max(delta,abs(v-u[i])); u[i]=v
        if delta<1e-13: break
    return sum(u)/len(u), max(u), 2*d/D
def alpha_fourier(d,n,ml):
    a=ml/(n+1); ks=range(1,n+1,2)
    w1={k:2/(n*(n+1))/math.tan(math.pi*k/(2*(n+1)))**2 for k in ks}
    lam1={k:4/a**2*math.sin(math.pi*k*a/(2*ml))**2 for k in ks}
    s=0
    for K in itertools.product(ks,repeat=d):
        wt=1;lam=0
        for k in K: wt*=w1[k]; lam+=lam1[k]
        s+=wt*1/(lam+1)
    return 1-s
def alpha_cont(d,ml,kmax):
    ks=range(1,kmax,2); s=0
    w1={k:8/(math.pi*k)**2 for k in ks}
    if d<=2 or kmax<80:
        for K in itertools.product(ks,repeat=d):
            wt=1
            for k in K: wt*=w1[k]
            s+=wt*ml**2/(math.pi**2*sum(k*k for k in K)+ml**2)
    return 1-s
def bound(d,n,ml):
    a=ml/(n+1); th=2*math.asinh(a/2)
    mean1=math.sinh(n*th/2)/(n*(a/2)*math.cosh((n+1)*th/2))
    return d*mean1, 2*d/(ml-a)
for d,n,ml in [(2,8,4),(2,16,4),(2,8,16),(4,4,4),(4,6,4),(4,4,16),(1,50,3)]:
    ad=alpha_direct(d,n,ml)
    print(d,n,ml,"direct %.4f max %.4f maxbound %.4f"%ad,"fourier %.4f"%alpha_fourier(d,n,ml),"bound %.4f %.4f"%bound(d,n,ml))
print("d=1 cont ml=3", 2/3*math.tanh(1.5), alpha_cont(1,3,20001))
print("d=2 cont ml=4", alpha_cont(2,4,801), "ml=16", alpha_cont(2,16,801))
print("d=4 cont ml=4 (kmax 41)", alpha_cont(4,4,41), "kmax 61", alpha_cont(4,4,61))
