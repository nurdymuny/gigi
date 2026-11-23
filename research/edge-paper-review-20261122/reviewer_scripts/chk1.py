import sys, numpy as np
sys.path.insert(0, "./scripts")
import ricci_su3_test2_family as T2
X, C = T2.su3_data()
XH = -1j*np.diag([1.0,0,-1.0])
h = np.array([float((-2*np.trace(XH@X[a])).real) for a in range(8)]); eH = h/np.linalg.norm(h)
eK = np.zeros(8); eK[2], eK[7] = eH[7], -eH[2]
R = np.eye(8); R[2], R[7] = eH, eK
Cy = np.einsum("ai,bj,ijk,ck->abc", R, R, C, R)
HI, KI = 2, 7
roots = {"12": (0,1), "13": (3,4), "23": (5,6)}
def field(x,w,y12,y23,y13):
    M = np.zeros((8,8)); M[HI,HI]=x; M[KI,KI]=w
    for k,v in (("12",y12),("23",y23),("13",y13)):
        for i in roots[k]: M[i,i]=v
    return [M[i] for i in range(8)]
adH = np.einsum("c,cab->ba", h@R.T, Cy)
def metric(t):
    g=np.ones(8); g[HI]=t; return g
reps = {pq: T2.su3_irrep(*pq, X) for pq in T2.SECTORS}
rho = {pq: [sum(R[a,b]*reps[pq][b] for b in range(8)) for a in range(8)] for pq in T2.SECTORS}
def Qfor(pq, Hm, G):
    d = rho[pq][0].shape[0]
    parts = (1j*adH, 1j*adH, 1j*sum(h[a]*reps[pq][a] for a in range(8)))
    s = T2.fit_signs(parts, Hm, G@G.conj().T, 8, 8, d); assert len(s)==1, s
    Q = T2.assemble_Q(parts, s[0], 8, 8, d)
    if abs(np.linalg.eigvalsh(Q).max()-(sum(pq)+4))>1e-8: Q=-Q
    return Q
rng = np.random.default_rng(int(sys.argv[1]))
for trial in range(2):
    t = rng.uniform(1.5, 9); w, y13 = rng.uniform(0.2,2,2)
    if trial == 0: y12, y23 = rng.uniform(0.2,2,2)
    else: y12 = y23 = rng.uniform(0.2,2)
    print("member t=%.3f w=%.3f y12=%.3f y23=%.3f y13=%.3f" % (t,w,y12,y23,y13))
    for pq in T2.SECTORS:
        x0 = rng.uniform(-2,1)
        Hm, G = T2.hessian_and_gauge(Cy, metric(t), Cy, field(x0,w,y12,y23,y13), rho[pq])
        Q = Qfor(pq, Hm, G); q, U = np.linalg.eigh(Q)
        top = sum(pq)+4
        nbot = int((np.abs(q+top)<1e-8).sum()); ntop=int((np.abs(q-top)<1e-8).sum())
        b = U[:, np.argmax(q)]; m = U[:, np.argmin(q)]
        lb = (b.conj()@Hm@b).real; lm = (m.conj()@Hm@m).real
        resm = np.linalg.norm(Hm@m - lm*m); gm = np.linalg.norm(G.conj().T@m)
        mu = T2.slice_spectrum(Hm, G)
        nb = int((np.abs(mu-lb) <= 1e-9*max(1,abs(lb))).sum())
        print(pq, "qmin", round(q.min(),6), "ntop", ntop, "nbot", nbot, "lam_b-lam_mirror %.2e" % (lb-lm), "mirror res %.1e G*m %.1e" % (resm, gm), "n_b", nb)
