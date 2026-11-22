"""Fast sector spectra of curl (scalar or ad-twisted, for the circle-symmetric family) on the Berger sphere (t,1,1):
the charge K = J_z + (frame rotation) [+ Lie rotation] commutes with curl (checked), so after sorting by K the
matrix is banded and scipy's banded Hermitian solver gives the spectrum in O(dim). Used by eta_berger_scalar_fast.py
and eta_family_eta.py. Conventions as eta_family.py; rho_0 = -2i J_z, rho_1 = -2i J_x, rho_2 = -2i J_y (a cyclic
relabelling of SS.spin_rep, unitarily equivalent).
"""
import numpy as np
import scipy.sparse as sp
from scipy.linalg import eigvals_banded

BR = {}
for i, j, k in [(0, 1, 2), (1, 2, 0), (2, 0, 1)]:
    BR[(i, j, k)] = 2.0
    BR[(j, i, k)] = -2.0
STAR = {(0, 1): (2, 1.0), (1, 2): (0, 1.0), (0, 2): (1, -1.0)}
PAIRS = [(0, 1), (1, 2), (0, 2)]
LF = np.array([[0, 0, 0], [0, 0, -1j], [0, 1j, 0]])           # rotation generator in the (1,2) plane
WL, UL = np.linalg.eigh(LF)                                    # eigenvalues -1, 0, 1


def rep(j):
    s = j / 2
    m = np.arange(s, -s - 1, -1)
    d = len(m)
    jp = np.sqrt(s * (s + 1) - m[1:] * (m[1:] + 1))
    Jp = sp.diags([jp], [1], shape=(d, d), dtype=complex)
    Jx = (Jp + Jp.conj().T) / 2
    Jy = (Jp - Jp.conj().T) / 2j
    Jz = sp.diags([m], [0], dtype=complex)
    return [-2j * Jz, -2j * Jx, -2j * Jy], m


def curl_spectrum(t, j, m=None, adjoint=False):
    """Eigenvalues of curl_A in sector j. adjoint=False: scalar 1-forms. adjoint=True: su(2)-valued, field m."""
    s = np.sqrt(np.array([t, 1.0, 1.0]))
    c = np.zeros((3, 3, 3))
    for (i, jj, k), v in BR.items():
        c[i, jj, k] = v * s[k] / (s[i] * s[jj])
    rho, wts = rep(j)
    d = j + 1
    na = 3 if adjoint else 1
    Iv = sp.identity(d, dtype=complex, format="csr")
    Ia = sp.identity(na, dtype=complex, format="csr")

    def cross(v):
        a, b, cc = v
        return np.array([[0, -cc, b], [cc, 0, -a], [-b, a, 0]], float)

    def op(i):     # e_i + ad M_i on (Lie (x) V_j)
        o = sp.kron(Ia, rho[i] / s[i])
        if adjoint:
            o = o + sp.kron(sp.csr_matrix(cross(m[i])), Iv)
        return o
    blk = na * d
    rows = [[None] * 3 for _ in range(3)]
    for (i, jj), (k, sg) in STAR.items():
        # component k of curl: sg * (op(i) b_jj - op(jj) b_i - sum c_ij^l b_l)
        contrib = {jj: sg * op(i), i: -sg * op(jj)}
        for l in range(3):
            if c[i, jj, l] != 0:
                contrib[l] = contrib.get(l, 0) - sg * c[i, jj, l] * sp.identity(blk, dtype=complex)
        for l, v in contrib.items():
            rows[k][l] = v if rows[k][l] is None else rows[k][l] + v
    C = sp.bmat(rows, format="csr")
    # charge: frame rotation + Jz (+ Lie rotation about tau_0)
    Uf = sp.kron(sp.csr_matrix(UL), sp.identity(blk, dtype=complex))
    if adjoint:
        Ul = sp.kron(sp.identity(3), sp.kron(sp.csr_matrix(UL), Iv))
        U = Uf @ Ul
        kval = (WL[:, None, None] + WL[None, :, None] + wts[None, None, :]).ravel()
    else:
        U = Uf
        kval = (WL[:, None] + wts[None, :]).ravel()
    Cp = (U.conj().T @ C @ U).tocsr()
    P = np.argsort(np.round(kval * 2).astype(int), kind="stable")
    Cp = Cp[P][:, P].tocoo()
    keep = np.abs(Cp.data) > 1e-13
    r, cl, v = Cp.row[keep], Cp.col[keep], Cp.data[keep]
    bw = int(np.max(np.abs(r - cl)))
    n = Cp.shape[0]
    ab = np.zeros((bw + 1, n), complex)
    low = r >= cl
    ab[r[low] - cl[low], cl[low]] = v[low]
    return eigvals_banded(ab, lower=True), bw


def odd_signature_spectrum(t, j, m):
    """Eigenvalues of the ad(su(2))-twisted odd signature operator L_A(xi, b) = (G^* b, G xi + curl_A b) on
    (Omega^0 + Omega^1) (x) su(2), sector j, constant field m (G xi = (e_i xi + [M_i, xi])_i)."""
    s = np.sqrt(np.array([t, 1.0, 1.0]))
    c = np.zeros((3, 3, 3))
    for (i, jj, k), v in BR.items():
        c[i, jj, k] = v * s[k] / (s[i] * s[jj])
    rho, wts = rep(j)
    d = j + 1
    Iv = sp.identity(d, dtype=complex, format="csr")

    def cross(v):
        a, b, cc = v
        return np.array([[0, -cc, b], [cc, 0, -a], [-b, a, 0]], float)

    def op(i):
        return sp.kron(sp.identity(3), rho[i] / s[i]) + sp.kron(sp.csr_matrix(cross(m[i])), Iv)
    blk = 3 * d
    rows = [[None] * 4 for _ in range(4)]          # index 0..2: 1-form components, 3: Omega^0
    for (i, jj), (k, sg) in STAR.items():
        contrib = {jj: sg * op(i), i: -sg * op(jj)}
        for l in range(3):
            if c[i, jj, l] != 0:
                contrib[l] = contrib.get(l, 0) - sg * c[i, jj, l] * sp.identity(blk, dtype=complex)
        for l, v in contrib.items():
            rows[k][l] = v if rows[k][l] is None else rows[k][l] + v
    for i in range(3):
        rows[i][3] = op(i)
        rows[3][i] = op(i).conj().T
    rows[3][3] = sp.csr_matrix((blk, blk), dtype=complex)
    L = sp.bmat(rows, format="csr")
    U4 = np.zeros((4, 4), complex)
    U4[:3, :3] = UL
    U4[3, 3] = 1
    Uf = sp.kron(sp.csr_matrix(U4), sp.identity(blk, dtype=complex))
    Ul = sp.kron(sp.identity(4), sp.kron(sp.csr_matrix(UL), Iv))
    U = Uf @ Ul
    wf = np.concatenate([WL, [0.0]])
    kval = (wf[:, None, None] + WL[None, :, None] + wts[None, None, :]).ravel()
    Lp = (U.conj().T @ L @ U).tocsr()
    P = np.argsort(np.round(kval * 2).astype(int), kind="stable")
    return block_eigs(Lp, kval)


def block_eigs(Mp, kval):
    """Mp commutes with diag(kval): eigenvalues via batched dense eigh on the charge blocks."""
    key = np.round(kval * 2).astype(int)
    P = np.argsort(key, kind="stable")
    ks = key[P]
    Mp = Mp[P][:, P].tocoo()
    starts = np.flatnonzero(np.r_[True, ks[1:] != ks[:-1]])
    bid = np.cumsum(np.r_[True, ks[1:] != ks[:-1]]) - 1
    pos = np.arange(len(ks)) - starts[bid]
    sizes = np.diff(np.r_[starts, len(ks)])
    bmax = int(sizes.max())
    keep = np.abs(Mp.data) > 1e-13
    r, cl, v = Mp.row[keep], Mp.col[keep], Mp.data[keep]
    assert np.all(bid[r] == bid[cl]), "matrix does not commute with the charge"
    nb = len(starts)
    B = np.zeros((nb, bmax, bmax), complex)
    B[bid[r], pos[r], pos[cl]] = v
    pad = np.arange(bmax)[None, :] >= sizes[:, None]
    B[np.nonzero(pad)[0], np.nonzero(pad)[1], np.nonzero(pad)[1]] = 1e30
    w = np.linalg.eigvalsh(B)
    w = w[np.abs(w) < 1e29]
    return w, bmax
