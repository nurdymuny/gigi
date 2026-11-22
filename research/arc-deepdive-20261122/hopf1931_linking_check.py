"""Check Hopf (1931) p.655: fibres of z = (x1+i x2)/(x3+i x4) are great circles of S^3,
and any two distinct fibres have linking number +-1 (Gauss linking integral after
stereographic projection to R^3). Also check Clifford parallelism: constant distance
between two fibres. Single-threaded, seconds."""
import numpy as np

def fibre(z, n):
    t = np.linspace(0, 2*np.pi, n, endpoint=False)
    w = np.exp(1j*t)
    if np.isinf(z):
        a, b = w, 0*w
    else:
        a, b = z*w, w
    nrm = np.sqrt(abs(a)**2 + abs(b)**2)
    a, b = a/nrm, b/nrm
    return np.stack([a.real, a.imag, b.real, b.imag], axis=1)  # (x1,x2,x3,x4)

def stereo(X, pole=np.array([0.3, -0.2, 0.5, np.sqrt(1-0.09-0.04-0.25)])):
    # project from a generic point of S^3 not on the fibres tested
    p = pole/np.linalg.norm(pole)
    # orthonormal basis of p-perp
    Q, _ = np.linalg.qr(np.column_stack([p, np.eye(4)[:, :3]]))
    B = Q[:, 1:]
    s = X @ p
    return (X @ B) / (1 - s)[:, None]

def gauss_link(A, B):
    dA = np.roll(A, -1, 0) - A; dB = np.roll(B, -1, 0) - B
    mA = A + dA/2; mB = B + dB/2
    r = mA[:, None, :] - mB[None, :, :]
    cr = np.cross(dA[:, None, :], dB[None, :, :])
    return np.sum(np.einsum('ijk,ijk->ij', r, cr) / np.linalg.norm(r, axis=2)**3) / (4*np.pi)

rng = np.random.default_rng(0)
n = 1500
out = []
for k in range(6):
    z1 = complex(*rng.normal(size=2)); z2 = complex(*rng.normal(size=2))
    F1, F2 = fibre(z1, n), fibre(z2, n)
    # great circle check: fibre lies in a 2-plane through origin (rank 2)
    rank = np.linalg.matrix_rank(F1, tol=1e-9)
    # Hopf map check: (x1+ix2)/(x3+ix4) constant on fibre
    zz = (F1[:, 0]+1j*F1[:, 1])/(F1[:, 2]+1j*F1[:, 3])
    # Clifford parallel: distance from each point of F1 to F2 constant
    d = np.arccos(np.clip(np.max(F1 @ F2.T, axis=1), -1, 1))
    L = gauss_link(stereo(F1), stereo(F2))
    out.append((z1, z2, rank, np.ptp(zz.real)+np.ptp(zz.imag), np.ptp(d), L))
    print(f"z1={z1:.3f} z2={z2:.3f} rank={rank} spread(z)={out[-1][3]:.1e} "
          f"dist-spread={np.ptp(d):.1e} (dist={d.mean():.4f}) link={L:+.4f}")
