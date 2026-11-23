import sys, numpy as np
exec(open("chk1.py").read().split("rng = np.random")[0])
rng = np.random.default_rng(3)
for trial in range(2):
    t = rng.uniform(1.5, 9); w, y12, y13 = rng.uniform(0.2,2.5,3); y23 = y12 if trial else rng.uniform(0.2,2.5)
    lam = {}
    for pq in [(0,0),(1,0),(1,1),(2,0),(4,0),(2,2)]:
        vals=[]
        for x in (0.0,1.0):
            Hm, G = T2.hessian_and_gauge(Cy, metric(t), Cy, field(x,w,y12,y23,y13), rho[pq])
            Q = Qfor(pq, Hm, G); q,U=np.linalg.eigh(Q); b=U[:,np.argmax(q)]
            vals.append((b.conj()@Hm@b).real)
        lam[pq]=vals
    for pq in lam:
        if pq!=(0,0):
            k = (lam[pq][1]-lam[(0,0)][1])-(lam[pq][0]-lam[(0,0)][0])
            print(trial, pq, "kappa=%.6f  (p+q)/sqrt t=%.6f" % (k, sum(pq)/np.sqrt(t)))
