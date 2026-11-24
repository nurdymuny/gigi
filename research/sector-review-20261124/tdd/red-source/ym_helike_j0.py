"""Spin-0 colour-singlet sector of the sphere matrix model, solved in its own invariant basis.

The model (coordinates q = M/g, T = tr(q q^T), P = tr((q q^T)^2), d = det q):

    H = -1/2 Lap + 1/2 T - 3 g d + g^2/4 (T^2 - P).

Colour acts by q -> U q and space by q -> q V^T, with U, V in SO(3). A state that is a colour singlet with spin
J = 0 is invariant under both. Its polynomial part lies in the invariant ring R[T, P, d], which is free in these
three generators: the O(3) x O(3) invariants are the symmetric functions of the squared singular values, and
d^2 = e_3. H maps that ring, times the Gaussian envelope, to itself. So the sector can be solved on its own.
Ritz values in it are upper bounds on the exact spin-0 eigenvalues (min-max inside an invariant subspace), and the
full-space residual of a sector state is the same quantity the 9-variable solver reports.

At cutoff N and width alpha, the basis is { T^a P^b d^c exp(-x.x/2) : 2a + 4b + 3c <= N } in x = sqrt(alpha) q.
That is exactly the spin-0 part of the 9-variable cutoff-N basis, because averaging over SO(3) x SO(3) keeps the
polynomial degree. So at equal N, alpha and g the two solvers must give the same spin-0 Ritz values.

Exact pieces:
  * gradient inner products of the generators, expressed in the generators (Frobenius inner product):
      dT.dT = 4T,  dT.dP = 8P,  dP.dP = 16 tr(Q^3) = -8T^3 + 24TP + 48d^2,
      dT.dd = 6d,  dP.dd = 4Td, dd.dd = (T^2 - P)/2,
    Laplacians: Lap T = 18, Lap P = 28T, Lap d = 0.
  * Gaussian moments M[F] = int F exp(-x.x) / int exp(-x.x), computed exactly in rationals by integration by
    parts against the vector fields grad T, grad P and grad d. Each step lowers the degree by 2.
  * Superpotential: V = 1/2 |grad W|^2 with W = T/2 - g d, so H = 1/2 A^dagger A + 9/2 with A = grad + grad W.
    Every exact eigenvalue exceeds 9/2 for g != 0. That is an exact gate on the ground energy.

Linear algebra runs in mpmath at `dps` digits, because the monomial Gram matrix is ill-conditioned. Residuals use the
exterior Gram X = K~ - H~^2 in the orthonormalised basis, where K = <H m_i, H m_j>, so a small residual is not lost
to cancellation. These are high-precision floating-point results, not interval-certified enclosures.
"""
from __future__ import annotations

import math
from fractions import Fraction
from functools import lru_cache

import mpmath as mp
import numpy as np

T, P, D = 0, 1, 2
ZERO = (0, 0, 0)


def _add(acc, mono, coef):
    if coef:
        v = acc.get(mono, 0) + coef
        if v:
            acc[mono] = v
        else:
            acc.pop(mono, None)


def mono_mul(m1, m2):
    return (m1[0] + m2[0], m1[1] + m2[1], m1[2] + m2[2])


def poly_mul(p1, p2):
    out = {}
    for m1, c1 in p1.items():
        for m2, c2 in p2.items():
            _add(out, mono_mul(m1, m2), c1 * c2)
    return out


def degree(m):
    return 2 * m[0] + 4 * m[1] + 3 * m[2]


F = Fraction
# Frobenius inner products of generator gradients, as polynomials in (T, P, d).
GRAD_DOT = {
    (T, T): {(1, 0, 0): F(4)},
    (T, P): {(0, 1, 0): F(8)},
    (P, P): {(3, 0, 0): F(-8), (1, 1, 0): F(24), (0, 0, 2): F(48)},
    (T, D): {(0, 0, 1): F(6)},
    (P, D): {(1, 0, 1): F(4)},
    (D, D): {(2, 0, 0): F(1, 2), (0, 1, 0): F(-1, 2)},
}
for (x, y), v in list(GRAD_DOT.items()):
    GRAD_DOT[(y, x)] = v
LAPLACIAN = {T: {ZERO: F(18)}, P: {(1, 0, 0): F(28)}, D: {}}


def d_gen(m, x):
    """Partial derivative of the monomial with respect to generator x: (coefficient, monomial)."""
    if m[x] == 0:
        return 0, None
    k = list(m)
    k[x] -= 1
    return m[x], tuple(k)


def grad_dot_gen(m, y):
    """grad(m) . grad(generator y), as a polynomial."""
    out = {}
    for x in (T, P, D):
        c, mx = d_gen(m, x)
        if c:
            for mm, cc in GRAD_DOT[(x, y)].items():
                _add(out, mono_mul(mx, mm), c * cc)
    return out


def laplacian(m):
    out = {}
    for x in (T, P, D):
        c, mx = d_gen(m, x)
        if not c:
            continue
        for mm, cc in LAPLACIAN[x].items():
            _add(out, mono_mul(mx, mm), c * cc)
        for y in (T, P, D):
            c2, mxy = d_gen(mx, y)
            if c2:
                for mm, cc in GRAD_DOT[(x, y)].items():
                    _add(out, mono_mul(mxy, mm), c * c2 * cc)
    return out


@lru_cache(maxsize=None)
def moment(m):
    """Exact <m> under exp(-x.x) on R^9, normalised so that <1> = 1."""
    a, b, c = m
    if degree(m) % 2:
        return F(0)
    if c > 0:   # int div(F grad d e) = 0:  <grad F . grad d> = <F grad T . grad d> = 6 <F d>
        return sum((cc * moment(mm) for mm, cc in grad_dot_gen((a, b, c - 1), D).items()), F(0)) / 6
    if b > 0:   # <grad F . grad P> + <F Lap P> = <F grad T . grad P> = 8 <F P>
        f = (a, b - 1, c)
        s = sum((cc * moment(mm) for mm, cc in grad_dot_gen(f, P).items()), F(0))
        return (s + 28 * moment((a + 1, b - 1, c))) / 8
    if a > 0:   # <grad F . grad T> + 18 <F> = 4 <F T>, and grad F . grad T = 2 deg(F) F
        return F(degree((a - 1, 0, 0)) + 9, 2) * moment((a - 1, 0, 0))
    return F(1)


def poly_moment(p):
    return sum((c * moment(m) for m, c in p.items()), F(0))


def basis(N):
    out = [(a, b, c) for a in range(N // 2 + 1) for b in range(N // 4 + 1) for c in range(N // 3 + 1)
           if degree((a, b, c)) <= N]
    return sorted(out, key=lambda m: (degree(m), m))


def operator_parts(m):
    """H m e^{-x.x/2} = e^{-x.x/2} [alpha*A + B/(2 alpha) - 3 g alpha^{-3/2} C + g^2/(4 alpha^2) Dq], in x coords."""
    A = {k: -v / 2 for k, v in laplacian(m).items()}
    _add(A, m, F(degree(m)) + F(9, 2))
    _add(A, mono_mul(m, (1, 0, 0)), F(-1, 2))
    B = {mono_mul(m, (1, 0, 0)): F(1)}
    C = {mono_mul(m, (0, 0, 1)): F(1)}
    Dq = {mono_mul(m, (2, 0, 0)): F(1), mono_mul(m, (0, 1, 0)): F(-1)}
    return A, B, C, Dq


def _mpf(fr):
    return mp.mpf(fr.numerator) / fr.denominator


def solve(N, g2, alpha, dps=60, nlev=None):
    """Solve the J=0 sector at cutoff N. Returns energies and full-space residual norms for all levels."""
    with mp.workdps(dps):
        coef = coefficients(g2, alpha)
        ms = basis(N)
        n = len(ms)
        hs = []
        for m in ms:
            h = {}
            for c, part in zip(coef, operator_parts(m)):
                for mm, cc in part.items():
                    h[mm] = h.get(mm, 0) + c * _mpf(cc)
            hs.append(h)
        cache = {}

        def M(mono):
            v = cache.get(mono)
            if v is None:
                v = cache[mono] = _mpf(moment(mono))
            return v

        S = mp.matrix(n, n)
        H = mp.matrix(n, n)
        K = mp.matrix(n, n)
        for i in range(n):
            for j in range(i, n):
                S[i, j] = S[j, i] = M(mono_mul(ms[i], ms[j]))
                hij = mp.fsum(c * M(mono_mul(ms[i], mm)) for mm, c in hs[j].items())
                hji = mp.fsum(c * M(mono_mul(ms[j], mm)) for mm, c in hs[i].items())
                H[i, j] = H[j, i] = (hij + hji) / 2
                if i != j:
                    asym = abs(hij - hji)
                    scale = max(abs(hij), abs(hji), 1)
                    if asym > scale * mp.mpf(10) ** (-(dps - 15)):
                        raise ArithmeticError(f"H not symmetric at {ms[i]},{ms[j]}: {asym}")
                K[i, j] = K[j, i] = mp.fsum(ci * cj * M(mono_mul(mi, mj))
                                            for mi, ci in hs[i].items() for mj, cj in hs[j].items())
        out = finish(S, H, K, dps, nlev)
    out.update({"cutoff": N, "g2": float(g2), "alpha": float(alpha), "basis": ms})
    return out


def coefficients(g2, alpha):
    """(alpha, 1/(2 alpha), -3 g alpha^{-3/2}, g^2/(4 alpha^2)) at the current mp precision."""
    g = mp.sqrt(_mpf(F(g2).limit_denominator(10 ** 12)))
    al = _mpf(F(alpha).limit_denominator(10 ** 12))
    return (al, 1 / (2 * al), -3 * g * al ** mp.mpf(-1.5), g ** 2 / (4 * al ** 2))


def finish(S, H, K, dps, nlev=None):
    """Orthonormalise by Cholesky at the current precision; return energies and full-space residual norms.

    S = <m_i, m_j>, H = <m_i, H m_j>, K = <H m_i, H m_j>. The exterior Gram X = K~ - H~^2 is formed before
    leaving high precision, so a small residual is not lost to cancellation."""
    n = S.rows
    L = mp.cholesky(S)
    pivots = [L[i, i] ** 2 / S[i, i] for i in range(n)]
    digits_lost = float(-mp.log10(min(pivots)))
    if digits_lost > dps - 25:
        raise ArithmeticError(f"Gram conditioning loses {digits_lost:.1f} digits at dps={dps}")
    Li = mp.inverse(L)
    Ht = Li * H * Li.T
    Kt = Li * K * Li.T
    X = Kt - Ht * Ht
    Hf = np.array(Ht.tolist(), dtype=float)
    Xf = np.array(X.tolist(), dtype=float)
    Hf = (Hf + Hf.T) / 2
    Xf = (Xf + Xf.T) / 2
    E, V = np.linalg.eigh(Hf)
    res = np.sqrt(np.clip(np.einsum("ik,ij,jk->k", V, Xf, V), 0, None))
    k = n if nlev is None else min(nlev, n)
    return {"dimension": n, "dps": dps, "gram_digits_lost": digits_lost, "energies": E[:k].tolist(),
            "residuals": res[:k].tolist(), "min_exterior_eig": float(np.linalg.eigvalsh(Xf).min()),
            "vectors": V[:, :k]}


def ordering_inputs(sol):
    """Vacuum, lowest excited scalar and the next scalar, with the registered Temple quantities for the scalar."""
    E, R = sol["energies"], sol["residuals"]
    rho, eps = E[1], R[1]
    rho_n, eps_n = E[2], R[2]
    beta = rho_n - eps_n
    out = {"E0": E[0], "eps0": R[0], "E0_minus_9_2": E[0] - 4.5, "E_s": rho, "eps_s": eps,
           "E_s_next": rho_n, "eps_s_next": eps_n, "gap_s": rho - E[0],
           "allowance": 0.01 * max(1.0, abs(rho - E[0])), "certified": False, "separation_verified": False}
    out["residual_gate_scalar"] = bool(eps <= out["allowance"])
    for name, b in (("beta", beta), ("beta_half", rho + (beta - rho) / 2)):
        usable = b > rho + eps
        out[name] = b
        out[f"temple_lower_{name}"] = rho - eps ** 2 / (b - rho) if usable else None
        out[f"tau_{name}"] = eps ** 2 / (b - rho) if usable else None
    return out


def branch(sol, index=1):
    """Branch dict in the shape quantum_ordering.ordering expects (index 1 = lowest excited scalar)."""
    E, R = sol["energies"], sol["residuals"]
    return {"low": E[index], "high": E[index], "residual": R[index],
            "next_low": E[index + 1], "next_residual": R[index + 1], "spin_error": 0.0}
