import sys, numpy as np
exec(open("chk1.py").read().split("rng = np.random")[0])
g1=np.ones(8)
for pq in [(0,0),(1,0),(1,1),(2,0)]:
    H0,G0 = T2.hessian_and_gauge(C, g1, C, T2.su3_background(g1), reps[pq])
    H1,G1 = T2.hessian_and_gauge(Cy, g1, Cy, [0.5*np.eye(8)[a] for a in range(8)], rho[pq])
    m0=np.sort(T2.slice_spectrum(H0,G0)); m1=np.sort(T2.slice_spectrum(H1,G1))
    print(pq, "min|mu| %.2e" % np.abs(m0).min(), "n(|mu|<1e-8)", int((np.abs(m0)<1e-8).sum()), "max abs diff %.2e" % np.abs(m0-m1).max(),
          "max rel diff (per eig) %.2e" % (np.abs(m0-m1)/np.maximum(np.abs(m0),1e-300)).max())
# wrong-axis test: t on X3 (index2 in X basis) vs t on e_H at t=1 trivially same; check e_K-stretch vs 2b at t=3 (Weyl conjugate)
t=3.0
for pq in [(1,0),(1,1)]:
    g=np.ones(8); g[7]=t
    H0,G0 = T2.hessian_and_gauge(C, g, C, T2.su3_background(g), reps[pq])
    gk=np.ones(8); gk[KI]=t
    H1,G1 = T2.hessian_and_gauge(Cy, gk, Cy, T2.su3_background(gk), rho[pq])
    m0=np.sort(T2.slice_spectrum(H0,G0)); m1=np.sort(T2.slice_spectrum(H1,G1))
    print("e_K stretch vs 2b", pq, "max diff %.2e" % np.abs(m0-m1).max())
