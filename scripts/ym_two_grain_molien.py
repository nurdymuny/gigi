"""Phase 3 budget: Molien-Weyl counts for the two-grain model's sectors.

Group G = SO(3)_colour x SO(3)_L x SO(3)_R acting on V = (3,3,1) + (3,1,3): q carries colour and the left spatial
index, p carries colour and the right spatial index. A sector (j_L, j_R) counts multiplets transforming as spin j_L
under SO(3)_L and j_R under SO(3)_R, colour singlets. Its bigraded Hilbert series is

    H(s, t) = < chi_{j_L}(b) chi_{j_R}(c) / (det(1 - s rho_q(a,b)) det(1 - t rho_p(a,c))) >_{a,b,c},

with Weyl averages <f>_{SO(3)} = (1/2pi) int f(theta) (1 - cos theta) d theta over the maximal-torus angle (f even).
The eigenvalues of rho_q at angles (a, b) are e^{i(x+y)} with x in {a, 0, -a} and y in {b, 0, -b}. The series
coefficients are h_k (complete homogeneous symmetric polynomials of the eigenvalues), from Newton's identities.
The coefficient of s^k t^l is a trigonometric polynomial in each angle of degree <= max(k, l) + 1 + 2 max(j), so
the uniform M-point rule is exact once M exceeds twice that degree. The q-part depends on (a, b) and the p-part on
(a, c), so the triple average factorises.

Counts returned are exact integers after rounding (the test checks the rounding error is tiny). The cutoff-N basis
size of a sector is the number of multiplets with total degree k + l <= N.
"""
from __future__ import annotations

import numpy as np


def weyl_grid(M):
    th = 2 * np.pi * np.arange(M) / M
    return th, (1 - np.cos(th)) / M


def character(j, th):
    return sum(np.cos(m * th) for m in range(-j, j + 1))


def complete_homogeneous(eigs, D):
    """h_0..h_D of the eigenvalues along the last axis, by Newton's identities. eigs: (..., n)."""
    p = [None] + [np.sum(eigs ** k, axis=-1) for k in range(1, D + 1)]
    h = [np.ones(eigs.shape[:-1], dtype=complex)]
    for k in range(1, D + 1):
        h.append(sum(p[i] * h[k - i] for i in range(1, k + 1)) / k)
    return np.array(h)                                    # (D+1, ...)


def hilbert_bigraded(D, sector=(0, 0), M=None):
    """Integer array H[k, l] (k, l <= D): number of sector multiplets of bidegree (k, l) in (q, p)."""
    jL, jR = sector
    M = M or 2 * (D + 2 + 2 * max(jL, jR)) + 3
    th, w = weyl_grid(M)
    a, b = np.meshgrid(th, th, indexing="ij")
    shifts = lambda x: np.stack([x, 0 * x, -x], -1)
    eig = np.exp(1j * (shifts(a)[..., :, None] + shifts(b)[..., None, :])).reshape(M, M, 9)
    hq = complete_homogeneous(eig, D)                     # (D+1, M_a, M_b); the p-part is the same with c for b
    Lq = np.einsum("kab,b->ka", hq, w * character(jL, th))
    Lp = np.einsum("kab,b->ka", hq, w * character(jR, th))
    H = np.einsum("ka,la,a->kl", Lq, Lp, w)
    Hi = np.rint(H.real).astype(int)
    err = float(np.max(np.abs(H - Hi)))
    if err > 1e-8:
        raise ArithmeticError(f"Molien coefficients not integral (max error {err}); increase M")
    return Hi


def cutoff_counts(H, cutoffs):
    D = H.shape[0] - 1
    return {N: int(sum(H[k, l] for k in range(D + 1) for l in range(D + 1) if k + l <= N)) for N in cutoffs}


def one_grain_counts(H, cutoffs):
    return {N: int(sum(H[k, 0] for k in range(min(N, H.shape[0] - 1) + 1))) for N in cutoffs}


# ---------------------------------------------------------------- independent check: Lie-algebra null space

def _generators():
    """The nine so(3) generators as 18 x 18 linear maps on (q, p), flattened row-major (q first)."""
    eps = np.zeros((3, 3, 3))
    for i, j, k in [(0, 1, 2), (1, 2, 0), (2, 0, 1)]:
        eps[i, j, k], eps[i, k, j] = 1, -1
    T = [-eps[a] for a in range(3)]
    gens = []
    for a in range(3):                                    # colour acts on both
        G = np.zeros((18, 18))
        for c in range(18):
            x = np.zeros(18); x[c] = 1
            q, p = x[:9].reshape(3, 3), x[9:].reshape(3, 3)
            G[:, c] = np.concatenate([(T[a] @ q).ravel(), (T[a] @ p).ravel()])
        gens.append(G)
    for a in range(3):                                    # left space acts on q only
        G = np.zeros((18, 18))
        for c in range(9):
            x = np.zeros(9); x[c] = 1
            G[:9, c] = (x.reshape(3, 3) @ T[a].T).ravel()
        gens.append(G)
    for a in range(3):                                    # right space acts on p only
        G = np.zeros((18, 18))
        for c in range(9):
            x = np.zeros(9); x[c] = 1
            G[9:, 9 + c] = (x.reshape(3, 3) @ T[a].T).ravel()
        gens.append(G)
    return gens


def _monomials(k, l):
    from itertools import combinations_with_replacement as cwr
    out = []
    for qm in cwr(range(9), k):
        for pm in cwr(range(9, 18), l):
            e = [0] * 18
            for i in qm + pm:
                e[i] += 1
            out.append(tuple(e))
    return out


def nullspace_count(k, l):
    """dim of polynomials of bidegree (k, l) killed by all nine generators (exact up to float rank tolerance)."""
    mons = _monomials(k, l)
    index = {m: i for i, m in enumerate(mons)}
    blocks = []
    for G in _generators():
        A = np.zeros((len(mons), len(mons)))
        for col, m in enumerate(mons):
            for v in range(18):                           # derivation: sum_v (G x)_v d/dx_v
                if m[v] == 0:
                    continue
                for u in range(18):
                    if G[u, v] == 0:
                        continue
                    e = list(m); e[v] -= 1; e[u] += 1
                    A[index[tuple(e)], col] += m[v] * G[u, v]
        blocks.append(A)
    S = np.vstack(blocks)
    sv = np.linalg.svd(S, compute_uv=False)
    return int(len(mons) - np.sum(sv > 1e-9 * sv.max()))


# ---------------------------------------------------------------- generators of the (0,0) ring, proved by exact rank

PRIME = 2147483629                                         # < 2^31, so products fit in int64


def _matmul_mod(A, B):
    return (A[:, :, None] * B[None, :, :] % PRIME).sum(1) % PRIME


def _det_mod(M):
    a = M
    return (a[0, 0] * (a[1, 1] * a[2, 2] % PRIME - a[1, 2] * a[2, 1] % PRIME)
            - a[0, 1] * (a[1, 0] * a[2, 2] % PRIME - a[1, 2] * a[2, 0] % PRIME)
            + a[0, 2] * (a[1, 0] * a[2, 1] % PRIME - a[1, 1] * a[2, 0] % PRIME)) % PRIME


def generators_mod(q, p):
    """Candidate generators of the (0,0) ring at an integer point, mod PRIME, with their bidegrees.
    A = q q^T and B = p p^T are the colour Gram matrices of the two grains."""
    A, B = _matmul_mod(q, q.T % PRIME), _matmul_mod(p, p.T % PRIME)
    A2, B2 = _matmul_mod(A, A), _matmul_mod(B, B)
    tr = lambda M: int(np.trace(M) % PRIME)
    return {
        "T_q": ((2, 0), tr(A)), "P_q": ((4, 0), tr(A2)), "d_q": ((3, 0), int(_det_mod(q))),
        "T_p": ((0, 2), tr(B)), "P_p": ((0, 4), tr(B2)), "d_p": ((0, 3), int(_det_mod(p))),
        "X11": ((2, 2), tr(_matmul_mod(A, B))), "X21": ((4, 2), tr(_matmul_mod(A2, B))),
        "X12": ((2, 4), tr(_matmul_mod(A, B2))), "X22": ((4, 4), tr(_matmul_mod(A2, B2))),
        "Y": ((6, 6), tr(_matmul_mod(_matmul_mod(A2, B2), _matmul_mod(A, B)))),
    }


def generator_monomials(names, degs, k, l):
    """All exponent vectors over the generators with total bidegree (k, l)."""
    out = []

    def rec(i, kk, ll, e):
        if i == len(names):
            if kk == 0 and ll == 0:
                out.append(tuple(e))
            return
        dk, dl = degs[i]
        n = 0
        while n * dk <= kk and n * dl <= ll:
            rec(i + 1, kk - n * dk, ll - n * dl, e + [n])
            n += 1
            if dk == 0 and dl == 0:
                break

    rec(0, k, l, [])
    return out


def rank_mod_p(M):
    M = M.copy() % PRIME
    r = 0
    rows, cols = M.shape
    for c in range(cols):
        piv = next((i for i in range(r, rows) if M[i, c] != 0), None)
        if piv is None:
            continue
        M[[r, piv]] = M[[piv, r]]
        inv = pow(int(M[r, c]), PRIME - 2, PRIME)
        M[r] = M[r] * inv % PRIME
        for i in range(rows):
            if i != r and M[i, c] != 0:
                M[i] = (M[i] - M[i, c] * M[r]) % PRIME
        r += 1
        if r == rows:
            break
    return r


def verify_generators(Dmax, H, seed=0):
    """For each bidegree with k + l <= Dmax: Molien count, number of generator monomials, and their exact rank mod p.
    rank == Molien proves the generator monomials span that bidegree (rank mod p <= rank over Q <= Molien)."""
    rng = np.random.default_rng(seed)
    probe = generators_mod(np.ones((3, 3), dtype=np.int64), np.ones((3, 3), dtype=np.int64))
    names = list(probe)
    degs = [probe[n][0] for n in names]
    out = {}
    for k in range(Dmax + 1):
        for l in range(Dmax + 1 - k):
            mons = generator_monomials(names, degs, k, l)
            want = int(H[k, l])
            if not mons:
                out[(k, l)] = {"molien": want, "monomials": 0, "rank": 0, "spans": want == 0}
                continue
            npts = len(mons) + 8
            vals = np.zeros((npts, len(mons)), dtype=np.int64)
            for i in range(npts):
                q = rng.integers(0, PRIME, size=(3, 3), dtype=np.int64)
                p = rng.integers(0, PRIME, size=(3, 3), dtype=np.int64)
                g = generators_mod(q, p)
                gv = [g[n][1] for n in names]
                for j, e in enumerate(mons):
                    v = 1
                    for base, ex in zip(gv, e):
                        if ex:
                            v = v * pow(base, ex, PRIME) % PRIME
                    vals[i, j] = v
            r = rank_mod_p(vals)
            out[(k, l)] = {"molien": want, "monomials": len(mons), "rank": r, "spans": r == want,
                           "relations": len(mons) - r}
    return out
