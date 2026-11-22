"""Check of the Fintushel-Stern triple enumeration and Chern-Simons values for the
Poincare sphere Sigma(2,3,5) = S^3/2I, plus direct construction of the SU(2) reps
and the McKay graph of 2I (affine E8). Single-threaded, seconds."""
import itertools, numpy as np
from fractions import Fraction
from scipy.optimize import least_squares

p, q, r = 2, 3, 5
# 1. Triple enumeration exactly as quoted from arXiv:1401.4815 App. C
triples = []
for k in range(1, p):
    for l in range(1, q):
        for m in range(1, r):
            F = Fraction
            c = [F(l,q)+F(m,r) < 1 and l % 2 == m % 2,
                 F(k,p)+F(l,q)+F(m,r) > 1,
                 F(k,p)-F(l,q)+F(m,r) < 1,
                 F(k,p)+F(l,q)-F(m,r) < 1]
            if all(c):
                e = k*q*r + l*p*r + m*p*q
                cs = Fraction(e*e, 4*p*q*r) % 1
                triples.append(((k,l,m), e, cs, (-cs) % 1))
print("triples (k,l,m), e, +e^2/4pqr mod 1, -e^2/4pqr mod 1:")
for t in triples: print("  ", t)

# 2. Direct construction: x,y in SU(2) with x ~ exp(i pi k/2), y ~ exp(i pi l/3),
#    z=(xy)^-1 ... use presentation x^2=y^3=z^5=xyz=-1 (binary polyhedral, central -1)
def su2(theta, n):
    n = n/np.linalg.norm(n)
    s = np.array([[[0,1],[1,0]],[[0,-1j],[1j,0]],[[1,0],[0,-1]]])
    N = sum(n[i]*s[i] for i in range(3))
    return np.cos(theta)*np.eye(2) + 1j*np.sin(theta)*N
for (k,l,m), *_ in triples:
    # fix x axis z; y axis in xz-plane at angle a; require tr(z)=2cos(pi m/5) where z=(xy)^-1 * (-1)
    def res(a):
        x = su2(np.pi*k/2, np.array([0,0,1.]))
        y = su2(np.pi*l/3, np.array([np.sin(a[0]),0,np.cos(a[0])]))
        z = -np.linalg.inv(x@y)          # xyz = -1
        return [np.trace(z).real - 2*np.cos(np.pi*m/5)]
    sol = least_squares(res, [1.0])
    a = sol.x[0]
    x = su2(np.pi*k/2, np.array([0,0,1.])); y = su2(np.pi*l/3, np.array([np.sin(a),0,np.cos(a)]))
    z = -np.linalg.inv(x@y)
    I = np.eye(2)
    errs = [np.abs(x@x+I).max(), np.abs(np.linalg.matrix_power(y,3)+I).max(),
            np.abs(np.linalg.matrix_power(z,5)+I).max(), np.abs(x@y@z+I).max()]
    # generate image group
    G = [I]; frontier=[I]
    key=lambda g: tuple(np.round(np.concatenate([g.real.ravel(), g.imag.ravel()]),8))
    seen={key(I)}
    while frontier:
        new=[]
        for g in frontier:
            for h in (x,y):
                gh=g@h; kk=key(gh)
                if kk not in seen: seen.add(kk); G.append(gh); new.append(gh)
        frontier=new
    traces = sorted(set(np.round([np.trace(g).real for g in G],6)))
    print(f"  rep {(k,l,m)}: relation residuals max {max(errs):.1e}, image order {len(G)}, "
          f"traces {traces}")

# 3. McKay graph of 2I: character table by Burnside's class-algebra method on the
#    explicit 120-element group G (image of the faithful rep above), then V_2 tensor.
n=len(G); key2=lambda g: tuple(np.round(np.concatenate([g.real.ravel(), g.imag.ravel()]),7))
idx={key2(g):i for i,g in enumerate(G)}
inv=[idx[key2(np.linalg.inv(g))] for g in G]
mul=np.array([[idx[key2(a@b)] for b in G] for a in G])
classes=[]; cl=[-1]*n
for i in range(n):
    if cl[i]>=0: continue
    c=sorted({mul[mul[j][i]][inv[j]] for j in range(n)})
    for j in c: cl[j]=len(classes)
    classes.append(c)
h=len(classes); sizes=np.array([len(c) for c in classes])
M=np.zeros((h,h,h))
for a in range(h):
    for b in range(h):
        z=classes[0][0]  # identity? find representatives
for a in range(h):
    for x in classes[a]:
        for b in range(h):
            for y in classes[b]:
                M[a,b,cl[mul[x][y]]]+=1
# M[a,b,c] counts pairs; normalise by class size of c
M=M/sizes[None,None,:]
rng=np.random.default_rng(0); coef=rng.normal(size=h)
T=sum(coef[a]*M[a] for a in range(h))   # T[b,c]
w,V=np.linalg.eig(T)   # T[b,c] omega_c = lambda omega_b
chars=[]
for k in range(h):
    v=V[:,k]; v=v/v[[i for i in range(h) if 0 in classes[i]][0]]  # omega(identity class)=1
    # omega_c = |c| chi(c)/chi(1) ; normalise via sum |c||chi|^2 = 120
    x=v/sizes  # proportional to chi(c)/chi(1)
    d=np.sqrt(n/np.sum(sizes*np.abs(x)**2).real)
    chars.append(d*x)
chars=sorted(chars,key=lambda c:c[0].real)
dims=[int(round(c[0].real)) for c in chars]
tr=np.array([np.trace(G[c[0]]).real for c in classes])
inner=lambda a,b: (sizes*a*np.conj(b)).sum()/n
print("classes:",h,"sizes",list(sizes),"defining traces",np.round(tr,4))
print("irrep dims:",dims,"sum sq",sum(d*d for d in dims))
A=np.array([[int(round(inner(tr*a,b).real)) for b in chars] for a in chars])
print("McKay adjacency (V_2 (x) rho_i contains rho_j):\n",A)
print("degrees",list(A.sum(1)))
C=2*np.eye(h)-A; ev=np.linalg.eigvalsh(C); print("affine Cartan eigenvalues:",np.round(ev,6))
print("C @ dims =",C@np.array(dims),"(zero vector => dims are the affine-E8 null vector / marks)")
two=[i for i,d in enumerate(dims) if d==2]
for i in two: print("2-dim irrep char values on classes:",np.round(chars[i].real,4))
