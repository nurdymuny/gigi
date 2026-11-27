"""Diagnostic only (decides nothing): where does the vacuum's non-Gaussian shape live? Mardia skewness and kurtosis of
psi0^2 in four coordinate systems at the smoke point's final dressed Mq 10 state, and at the bare one."""
import sys, json, numpy as np
import helike_m8_geometry as gm
import helike_condensate_pt as cp

def stats(W, xi):
    m = W / W.sum(); mu = m @ xi; d = xi - mu
    C = (d * m[:, None]).T @ d
    e, V = np.linalg.eigh(C); Z = d @ V / np.sqrt(e)
    b1 = 0.0
    # Mardia b1 = E[(Zi.Zj)^3] over independent pairs = sum_{abc} (E[Za Zb Zc])^2
    T = np.einsum("n,na,nb,nc->abc", m, Z, Z, Z, optimize=True)
    b1 = float(np.sum(T * T))
    b2 = float(m @ np.sum(Z * Z, axis=1) ** 2) - Z.shape[1] * (Z.shape[1] + 2)   # excess Mardia kurtosis
    axis_skew = [float(m @ Z[:, k] ** 3) for k in range(Z.shape[1])]
    return b1, b2, max(axis_skew, key=abs)

def coords(P, z):
    u = P.chart.u0 + z @ P.chart.S.T
    r = np.exp(u[:, :3])
    lin = np.column_stack((r, u[:, 3:]))                          # linear Cholesky factor entries
    x, _, _ = P.gram(z)
    y = (x - P.x0) @ P.invM.T                                     # Gram-linear normal coordinates
    # symmetric square root of G: closest to the unfolded q-space (q = G^{1/2} up to colour)
    G = np.stack([cp.mat_from_vec(v) for v in x[:4]])             # sanity of shape only
    return {"chart z (log-Cholesky)": z, "linear Cholesky (r,a,b,c)": lin, "Gram-linear y": y}

def sqrtG(x):
    n = len(x); out = np.empty((n, 6))
    for a in range(0, n, 200000):
        Gs = np.zeros((min(200000, n - a), 3, 3))
        xs = x[a:a + 200000]
        Gs[:, 0, 0], Gs[:, 1, 1], Gs[:, 2, 2] = xs[:, 0], xs[:, 1], xs[:, 2]
        Gs[:, 0, 1] = Gs[:, 1, 0] = xs[:, 3]; Gs[:, 0, 2] = Gs[:, 2, 0] = xs[:, 4]; Gs[:, 1, 2] = Gs[:, 2, 1] = xs[:, 5]
        e, V = np.linalg.eigh(Gs)
        R = np.einsum("nij,nj,nkj->nik", V, np.sqrt(np.clip(e, 0, None)), V)
        out[a:a + len(xs)] = np.column_stack((R[:, 0, 0], R[:, 1, 1], R[:, 2, 2], R[:, 0, 1], R[:, 0, 2], R[:, 1, 2]))
    return out

for tag, path in (("bare", sys.argv[1]), ("dressed", sys.argv[2])):
    X = gm.load_levels(path)
    P = gm.Point(X, 12)
    p, _ = P.values([0]); W = P.core.W * p[:, 0] ** 2
    keep = W > 1e-14 * W.sum()
    z = P.core.z[keep]; Wk = W[keep]
    cs = coords(P, z)
    x, _, _ = P.gram(z)
    cs["sqrt(G) (unfolded q-space)"] = sqrtG(x)
    print(f"== {tag}: {path.split('/')[-1]}  nodes kept {keep.sum()}")
    for name, xi in cs.items():
        b1, b2, ax = stats(Wk, xi)
        print(f"   {name:32s} Mardia skew b1 = {b1:.4e}   excess kurtosis = {b2:+.4e}   largest axis skew = {ax:+.4f}")
