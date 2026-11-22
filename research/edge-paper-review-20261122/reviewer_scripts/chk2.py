import sys, numpy as np
sys.path.insert(0, ".")
exec(open("chk1.py").read().split("rng = np.random")[0])
rng = np.random.default_rng(11)
vidx = HI*8 + HI
for trial in range(6):
    t = rng.uniform(1.5, 9); w, y12, y23, y13 = rng.uniform(0.2,2.5,4)
    if trial % 2: y23 = y12
    lb, lv, gv = [], [], []
    for x in (-1.0, 0.0, 1.0):
        Hm, G = T2.hessian_and_gauge(Cy, metric(t), Cy, field(x,w,y12,y23,y13), rho[(0,0)])
        Q = Qfor((0,0), Hm, G); q,U = np.linalg.eigh(Q); b = U[:, np.argmax(q)]
        lb.append((b.conj()@Hm@b).real); v = np.zeros(64); v[vidx]=1
        lv.append((v@Hm@v).real); gv.append(np.linalg.norm(G.conj().T@v))
    bb = (lb[2]-lb[0])/2; bv = (lv[2]-lv[0])/2
    print("t=%.3f w=%.2f y=%.2f,%.2f,%.2f | lam_b 2nd diff %.3g lin %.6f | <v,Hv> vals %s | |G*v| %.1e | m_b=%.6f, -(2/sqrt t+sqrt t)=%.6f" % (
        t,w,y12,y23,y13, lb[0]-2*lb[1]+lb[2], bb, np.round(lv,10), max(gv), -(bb-bv)/2, -(2/np.sqrt(t)+np.sqrt(t))))
