"""Group machinery for pre-registration F (research/edge-theorem-20261122-f/wp7_prereg_F_principal_circle.md).

For a compact matrix group G in {SU(n), Sp(2)} (and SU(3) as a validation case):
  - an orthonormal basis X_a of its Lie algebra (anti-Hermitian, <X, Y> = -2 tr XY), structure constants C[a,b,c];
  - the maximal torus (Cartan) as diagonal elements, the roots (weights of ad on the complexified algebra) in torus
    coordinates, rho-vee and theta-vee computed from the root data;
  - irreducible representations generated from a highest-weight vector inside tensor powers of the fundamental and its
    dual (the construction of ricci_su3_test2_family.su3_irrep, generalised), restricted to the basis X_a;
  - a rotated orthonormal basis Y with the unit stretched generator e_K first, then an orthonormal Cartan complement,
    then each root plane (Re, Im of the root vector), and the structure constants in that basis.
All quantities are computed, nothing typed in by hand except the defining condition of the group.
"""
import itertools

import numpy as np
from scipy.linalg import null_space


def inner(X, Y):
    return float((-2 * np.trace(X @ Y)).real)


def orthonormalize(mats):
    out = []
    for M in mats:
        for B in out:
            M = M - inner(B, M) * B
        nrm = np.sqrt(max(inner(M, M), 0.0))
        if nrm > 1e-10:
            out.append(M / nrm)
    return out


def algebra(name):
    """Orthonormal basis of the Lie algebra (list of n x n anti-Hermitian matrices) and the torus basis (diagonal)."""
    if name.startswith("su"):
        n = int(name[2:])
        mats = []
        for i in range(n):
            for j in range(i + 1, n):
                E = np.zeros((n, n), complex); E[i, j] = 1; E[j, i] = -1; mats.append(E)
                E = np.zeros((n, n), complex); E[i, j] = 1j; E[j, i] = 1j; mats.append(E)
        tor = []
        for k in range(n - 1):
            d = np.zeros(n); d[: k + 1] = 1; d[k + 1] = -(k + 1)
            tor.append(1j * np.diag(d).astype(complex))
        tor = orthonormalize(tor)
        basis = orthonormalize(tor + mats)
        return n, basis, tor
    if name == "sp2":
        n = 4
        J = np.block([[np.zeros((2, 2)), np.eye(2)], [-np.eye(2), np.zeros((2, 2))]])
        # u(4) matrices X with X^T J + J X = 0: solve linearly over a real basis of u(4)
        u4 = []
        for i in range(n):
            E = np.zeros((n, n), complex); E[i, i] = 1j; u4.append(E)
            for j in range(i + 1, n):
                E = np.zeros((n, n), complex); E[i, j] = 1; E[j, i] = -1; u4.append(E)
                E = np.zeros((n, n), complex); E[i, j] = 1j; E[j, i] = 1j; u4.append(E)
        A = np.array([np.concatenate([(M.T @ J + J @ M).real.ravel(), (M.T @ J + J @ M).imag.ravel()]) for M in u4]).T
        N = null_space(A)
        mats = [sum(N[k, c] * u4[k] for k in range(len(u4))) for c in range(N.shape[1])]
        tor = orthonormalize([1j * np.diag([1, 0, -1, 0]).astype(complex), 1j * np.diag([0, 1, 0, -1]).astype(complex)])
        basis = orthonormalize(tor + mats)
        return n, basis, tor
    raise ValueError(name)


def structure(basis):
    d = len(basis)
    C = np.zeros((d, d, d))
    for a in range(d):
        for b in range(d):
            Z = basis[a] @ basis[b] - basis[b] @ basis[a]
            for c in range(d):
                C[a, b, c] = inner(basis[c], Z)
    return C


def roots(basis, tor):
    """Root vectors (complex combinations of the basis) and their weights in torus coordinates (eigenvalue of i*ad)."""
    d = len(basis); r = len(tor)
    coords = lambda M: np.array([inner(B, M) for B in basis]) + 0j                     # noqa: E731
    ad = [np.array([coords(T @ B - B @ T) for B in basis]).T for T in tor]               # ad(T) in basis coords
    # simultaneous diagonalisation: generic combination
    rng = np.random.default_rng(0); c = rng.standard_normal(r)
    Mg = sum(ci * 1j * A for ci, A in zip(c, ad))
    w, V = np.linalg.eig(Mg)
    out = []
    for k in range(d):
        if abs(w[k]) < 1e-8:
            continue
        v = V[:, k]
        wt = np.array([(v.conj() @ (1j * A) @ v) / (v.conj() @ v) for A in ad]).real
        E = sum(v[a] * basis[a] for a in range(d))
        out.append((wt, E))
    return out


def rep_tensor(mats_fund, p, q):
    """Generators acting on (C^n)^{(x)p} (x) (C^n*)^{(x)q}."""
    def big(M):
        facs = [M] * p + [-M.T] * q
        nf = len(facs); n = M.shape[0]; out = np.zeros((n ** nf, n ** nf), complex)
        for k in range(nf):
            T = np.eye(1)
            for i in range(nf):
                T = np.kron(T, facs[k] if i == k else np.eye(n))
            out += T
        return out
    return big


def irrep_from(basis, lowers, hw_vec, big, dim_expected=None):
    """Submodule generated from hw_vec by the lowering operators; generators restricted to it."""
    Bs = [hw_vec / np.linalg.norm(hw_vec)]; frontier = [Bs[0]]
    Lo = [big(L) for L in lowers]
    while frontier:
        new = []
        for w in frontier:
            for L in Lo:
                u = L @ w
                for b in Bs:
                    u = u - (b.conj() @ u) * b
                if np.linalg.norm(u) > 1e-9:
                    u = u / np.linalg.norm(u); Bs.append(u); new.append(u)
        frontier = new
    P = np.array(Bs).T
    if dim_expected is not None:
        assert P.shape[1] == dim_expected, (P.shape[1], dim_expected)
    return [P.conj().T @ big(X) @ P for X in basis]


def rotated(basis, tor, eK_coords, rts):
    """Orthonormal basis Y: e_K (given in torus-basis coordinates), Cartan complement, then root planes (Re, Im)."""
    eK = sum(c * T for c, T in zip(eK_coords, tor)); eK = eK / np.sqrt(inner(eK, eK))
    cart = orthonormalize([eK] + list(tor))
    planes, plane_roots = [], []
    seen = []
    for wt, E in rts:
        if any(np.allclose(wt, -s) for s in seen):
            continue
        seen.append(wt)
        Re = 0.5 * (E - E.conj().T); Im = 0.5j * (E + E.conj().T)                      # anti-Hermitian parts
        pl = orthonormalize([Re, Im])
        assert len(pl) == 2
        planes.append(pl); plane_roots.append(wt)
    Y = cart + [m for pl in planes for m in pl]
    assert len(Y) == len(basis)
    R = np.array([[inner(Yk, B) for B in basis] for Yk in Y])                            # Y = R basis
    assert np.allclose(R @ R.T, np.eye(len(Y)), atol=1e-10)
    return Y, R, plane_roots
