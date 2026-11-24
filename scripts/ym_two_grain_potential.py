"""Two-grain homogeneous Yang-Mills on S^3 = SU(2): the exact potential of A = M e T + N ebar T.

Conventions (all fixed by the Lie algebra; checked against the one-grain model in the tests):
  * su(2) basis tau_j = -(i/2) sigma_j, [tau_j, tau_k] = eps_jkl tau_l, metric <A,B> = -2 tr(AB) (tau orthonormal).
  * Left-invariant fields X_j(x) = x tau_j, so [X_j, X_k] = eps_jkl X_l. Left coframe e^j dual to X_j.
  * Right-invariant fields Y_j(x) = tau_j x = sum_k D_jk(x) X_k, with D(x) the matrix of Ad(x^{-1}) in the tau basis.
    Right coframe ebar^j = D_jk e^k, and X_m D_jk = -sum_l D_jl eps_mlk.
  * Colour: A = A_a T_a with [T_a, T_b] = eps_abc T_c, F = dA + A^A.
  * A(X_k) = B(x)[:, k] with B = M + N D(x). In the left frame
      F_{a,jk} = X_j B_ak - X_k B_aj - eps_jkl B_al + eps_abc B_bj B_ck.
  * Energy E(M, N) = < 1/2 sum_a sum_{j<k} F_{a,jk}^2 > averaged over S^3 (the volume is dropped).

With N = 0 this is E = T/2 - 3 det M + (T^2 - P)/4, which is the one-grain model at g = 1. So the two-grain
Hamiltonian in model units is H = -1/2 (Lap_q + Lap_p) + E(g q, g p)/g^2. The two mode families are orthonormal under
the S^3 average (<D> = 0), so the kinetic term stays diagonal.

The quadrature rule is algebraically exact for this degree class; evaluation here is floating point, not certified
exact arithmetic. The density depends on x only through D(x), as a polynomial of degree <= 4 in D, i.e.
harmonics with l <= 4. The binary icosahedral group (120 elements, 60 distinct rotations D) has no invariant
harmonic with 1 <= l <= 5, so its average equals the Haar average for these integrands.
"""
from __future__ import annotations

import itertools

import numpy as np

EPS = np.zeros((3, 3, 3))
for _i, _j, _k in [(0, 1, 2), (1, 2, 0), (2, 0, 1)]:
    EPS[_i, _j, _k], EPS[_i, _k, _j] = 1., -1.
SIGMA = np.array([[[0, 1], [1, 0]], [[0, -1j], [1j, 0]], [[1, 0], [0, -1]]], dtype=complex)
TAU = -0.5j * SIGMA


def quat_to_su2(w, x, y, z):
    return w * np.eye(2) + 2 * (x * TAU[0] + y * TAU[1] + z * TAU[2])   # w + x i + y j + z k


def binary_icosahedral():
    """The 120 unit quaternions of the binary icosahedral group, by closure from two generators."""
    phi = (1 + 5 ** .5) / 2
    gens = [np.array([.5, .5, .5, .5]), np.array([phi / 2, 1 / (2 * phi), .5, 0.])]

    def mul(a, b):
        w1, x1, y1, z1 = a
        w2, x2, y2, z2 = b
        return np.array([w1 * w2 - x1 * x2 - y1 * y2 - z1 * z2, w1 * x2 + x1 * w2 + y1 * z2 - z1 * y2,
                         w1 * y2 - x1 * z2 + y1 * w2 + z1 * x2, w1 * z2 + x1 * y2 - y1 * x2 + z1 * w2])

    group = [np.array([1., 0, 0, 0])]
    frontier = list(group)
    while frontier:
        new = []
        for a in frontier:
            for g in gens:
                c = mul(a, g)
                if not any(np.allclose(c, h, atol=1e-12) for h in group):
                    group.append(c)
                    new.append(c)
        frontier = new
    return np.array(group)


def adjoint_inverse(x):
    """D_jk(x) = <Ad_{x^{-1}} tau_j, tau_k> = -2 tr(x^{-1} tau_j x tau_k)."""
    xi = np.linalg.inv(x)
    return np.array([[(-2 * np.trace(xi @ TAU[j] @ x @ TAU[k])).real for k in range(3)] for j in range(3)])


_ROTATIONS = None


def rotations():
    global _ROTATIONS
    if _ROTATIONS is None:
        _ROTATIONS = np.array([adjoint_inverse(quat_to_su2(*q)) for q in binary_icosahedral()])
    return _ROTATIONS


def curvature(M, N, D):
    """F_{a,jk} in the left frame at a point with rotation D."""
    B = M + N @ D
    XD = -np.einsum("jl,mlk->mjk", D, EPS)            # XD[m] = X_m D
    XB = np.einsum("ai,mik->amk", N, XD)               # XB[a, m, k] = X_m B_ak
    F = XB - XB.transpose(0, 2, 1) - np.einsum("jkl,al->ajk", EPS, B) + np.einsum("abc,bj,ck->ajk", EPS, B, B)
    return F


def energy(M, N, Ds=None):
    Ds = rotations() if Ds is None else Ds
    return float(np.mean([.25 * np.sum(curvature(M, N, D) ** 2) for D in Ds]))


def projected_gradient(M, N, Ds=None):
    """L^2 projection of *F onto the 18 modes: c_M[a,i] = <(*F)_a(X_i)>, c_N[a,i] = <sum_k (*F)_{a,k} D_ik>."""
    Ds = rotations() if Ds is None else Ds
    cM, cN = np.zeros((3, 3)), np.zeros((3, 3))
    for D in Ds:
        F = curvature(M, N, D)
        starF = .5 * np.einsum("ljk,ajk->al", EPS, F)
        cM += starF
        cN += starF @ D.T
    return cM / len(Ds), cN / len(Ds)


def one_grain(M, sign=1.):
    """E_L(M) = T/2 - 3 sign det M + (T^2 - P)/4."""
    Q = M @ M.T
    T = np.trace(Q)
    return T / 2 - 3 * sign * np.linalg.det(M) + (T * T - np.trace(Q @ Q)) / 4


def bidegree_parts(M, N, Ds=None, maxdeg=4):
    """Floating-point tensor polynomial fit, 0 <= i,j <= 4; total-degree > 4 coefficients should vanish."""
    if type(maxdeg) is not int or maxdeg != 4:
        raise ValueError('bidegree fit requires maxdeg=4 on the fixed five-by-five grid')
    pts = np.arange(-2, 3, dtype=float)
    rows, vals = [], []
    for s, t in itertools.product(pts, pts):
        rows.append([s ** i * t ** j for i in range(maxdeg + 1) for j in range(maxdeg + 1)])
        vals.append(energy(s * M, t * N, Ds))
    coef = np.linalg.lstsq(np.array(rows), np.array(vals), rcond=None)[0]
    out = {}
    k = 0
    for i in range(maxdeg + 1):
        for j in range(maxdeg + 1):
            out[(i, j)] = coef[k]
            k += 1
    return out


def cross_invariants(M, N):
    """The two (2,2) invariants of colour x SU(2)_L x SU(2)_R: T_M T_N and tr(M M^T N N^T)."""
    return np.array([np.trace(M @ M.T) * np.trace(N @ N.T), np.trace(M @ M.T @ N @ N.T)])


# ---------------------------------------------------------------- closed form and Phase 2 (vacuum plane)

def potential(q, p, g):
    """V18 in model units: V(q; g) + V(p; -g) + (g^2/3) sum_ij |q_i x p_j|^2 (Phase 1)."""
    Qq, Qp = q @ q.T, p @ p.T
    return (one_grain(g * q) / g ** 2 + one_grain(g * p, -1.) / g ** 2
            + g * g / 3 * (np.trace(Qq) * np.trace(Qp) - np.trace(Qq @ Qp)))


def embed(u, v, g):
    """Van Baal's sphaleron plane, eq. (88): c = -u delta, d = -v delta  <->  q = u I/(2g), p = -v I/(2g)."""
    return u * np.eye(3) / (2 * g), -v * np.eye(3) / (2 * g)


def plane_potential(u, v):
    """32 pi^2 g^2 V18 on the plane = 3 pi^2 [u^2 (2-u)^2 + v^2 (2-v)^2] + 4 pi^2 u^2 v^2 (van Baal-Hari Dass eq. (92))."""
    return 3 * np.pi ** 2 * (u ** 2 * (2 - u) ** 2 + v ** 2 * (2 - v) ** 2) + 4 * np.pi ** 2 * u ** 2 * v ** 2


def chern_simons(q, p, g):
    """Q = 2 g^2 (W_L(q) - W_R(p)), normalised so the left vacuum has Q = 1 (van Baal-Hari Dass eq. (101))."""
    WL = .5 * np.sum(q * q) - g * np.linalg.det(q)
    WR = .5 * np.sum(p * p) + g * np.linalg.det(p)
    return 2 * g * g * (WL - WR)


def plane_critical_points():
    """All critical points of the plane potential, by Newton's method from a grid, deduplicated."""
    f = plane_potential

    def grad(x, h=1e-6):
        return np.array([(f(x[0] + h, x[1]) - f(x[0] - h, x[1])) / (2 * h),
                         (f(x[0], x[1] + h) - f(x[0], x[1] - h)) / (2 * h)])

    def hess(x, h=1e-4):
        H = np.zeros((2, 2))
        for a in range(2):
            for b in range(2):
                ea, eb = np.eye(2)[a] * h, np.eye(2)[b] * h
                H[a, b] = (f(*(x + ea + eb)) - f(*(x + ea - eb)) - f(*(x - ea + eb)) + f(*(x - ea - eb))) / (4 * h * h)
        return H

    found = []
    for u0 in np.linspace(-1.5, 3, 19):
        for v0 in np.linspace(-1.5, 3, 19):
            x = np.array([u0, v0])
            for _ in range(60):
                try:
                    step = np.linalg.solve(hess(x), grad(x))
                except np.linalg.LinAlgError:
                    break
                x = x - step
                if np.linalg.norm(step) < 1e-12:
                    break
            if np.linalg.norm(grad(x)) < 1e-6 and np.all(np.abs(x) < 10):
                if not any(np.linalg.norm(x - y) < 1e-6 for y in found):
                    found.append(x)
    return sorted(found, key=lambda x: (f(*x), x[0], x[1]))
