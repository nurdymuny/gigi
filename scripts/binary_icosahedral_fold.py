"""The double-cover fold on the binary icosahedral group 2I (the group of the
Poincare homology sphere S^3/2I). Research note: docs/research/unit-curvature-2026-11-18.md.

Builds 2I (120 unit quaternions) by closure, checks that its centre is {+1, -1},
builds the Cayley graph for a generating set closed under inverses, folds each g
with -g (the cosets of the centre), and checks Bee's fold identity AP = PB
(an equitable partition, as in her Good Will Hunting graph-fold article). It then
splits the spectrum into the folded (even) part and the twin (odd, spinor) part,
reports graph distances between twins, the spectral gaps (Fiedler), and the
Ollivier-Ricci curvature of every edge (reported, not gated).

usage: python scripts/binary_icosahedral_fold.py [RECEIPT.json]
"""
import itertools
import json
import sys
from collections import deque
from pathlib import Path

import numpy as np
from scipy.optimize import linprog

PHI = (1 + 5 ** 0.5) / 2


def qmul(p, q):
    a1, b1 = p[0], p[1:]
    a2, b2 = q[0], q[1:]
    return np.r_[a1 * a2 - b1 @ b2, a1 * b2 + a2 * b1 + np.cross(b1, b2)]


def key(q):
    return tuple(np.round(q, 9) + 0.0)


def closure(gens):
    elems = {key(np.r_[1.0, 0, 0, 0]): np.r_[1.0, 0, 0, 0]}
    queue = deque(elems.values())
    while queue:
        g = queue.popleft()
        for s in gens:
            h = qmul(g, s)
            k = key(h)
            if k not in elems:
                elems[k] = h
                queue.append(h)
    return list(elems.values())


def ollivier(A, x, y):
    """kappa(x,y) = 1 - W1(mu_x, mu_y), mu = uniform on neighbours (non-lazy), W1 by LP."""
    nx, ny = np.flatnonzero(A[x]), np.flatnonzero(A[y])
    D = DIST[np.ix_(nx, ny)].astype(float)
    m, n = len(nx), len(ny)
    c = D.ravel()
    Aeq, beq = [], []
    for i in range(m):
        row = np.zeros(m * n); row[i * n:(i + 1) * n] = 1; Aeq.append(row); beq.append(1 / m)
    for j in range(n):
        row = np.zeros(m * n); row[j::n] = 1; Aeq.append(row); beq.append(1 / n)
    res = linprog(c, A_eq=np.array(Aeq), b_eq=np.array(beq), bounds=(0, None), method="highs")
    return 1 - res.fun


def main():
    receipt = Path(sys.argv[1]) if len(sys.argv) > 1 else None
    a = np.r_[1, 1, 1, 1] / 2.0                       # order 6
    b = np.r_[PHI, 1 / PHI, 1, 0] / 2.0               # order 10
    G = closure([a, b])
    n = len(G)
    idx = {key(g): i for i, g in enumerate(G)}
    out = {"order": n}
    # centre: elements commuting with the generators
    centre = [g for g in G if np.allclose(qmul(g, a), qmul(a, g)) and np.allclose(qmul(g, b), qmul(b, g))]
    out["centre"] = sorted([list(np.round(g, 12)) for g in centre])
    # Cayley graph, S = {a, a^-1, b, b^-1}
    inv = lambda q: np.r_[q[0], -q[1:]]
    S = [a, inv(a), b, inv(b)]
    A = np.zeros((n, n))
    for i, g in enumerate(G):
        for s in S:
            A[i, idx[key(qmul(g, s))]] += 1
    out["symmetric"] = bool(np.allclose(A, A.T))
    # fold: cells {g, -g}
    cell_of, cells = {}, []
    for i, g in enumerate(G):
        j = idx[key(-g)]
        if i not in cell_of:
            cell_of[i] = cell_of[j] = len(cells)
            cells.append((i, j))
    m = len(cells)
    P = np.zeros((n, m))
    for i, c in cell_of.items():
        P[i, c] = 1
    # B from one representative per cell, then check AP = PB for every member
    B = np.array([A[cells[c][0]] @ P for c in range(m)])
    out["fold_cells"] = m
    out["AP_equals_PB"] = bool(np.array_equal(A @ P, P @ B))
    # walk counts: (A^k)_{u, v} + (A^k)_{u, -v} = (B^k)_{[u],[v]}
    Ak, Bk, worst = np.eye(n), np.eye(m), 0.0
    for _ in range(12):
        Ak, Bk = Ak @ A, Bk @ B
        worst = max(worst, float(np.abs(Ak @ P - P @ Bk).max()))
    out["walk_identity_max_error_k_le_12"] = worst
    # spectrum split: even (functions with f(-g) = f(g)) = folded; odd = twin
    T = np.zeros((n, n))
    for i, g in enumerate(G):
        T[i, idx[key(-g)]] = 1
    ev, V = np.linalg.eigh(A)
    # project onto even/odd subspaces and diagonalise each
    Pe, Po = (np.eye(n) + T) / 2, (np.eye(n) - T) / 2
    def sub_eigs(Proj):
        U, s, _ = np.linalg.svd(Proj)
        Q = U[:, s > 0.5]
        return np.sort(np.linalg.eigvalsh(Q.T @ A @ Q))
    even, odd = sub_eigs(Pe), sub_eigs(Po)
    Bsym_eigs = np.sort(np.linalg.eigvals(B).real)
    out["even_count"], out["odd_count"] = int(len(even)), int(len(odd))
    out["even_equals_quotient_spectrum"] = bool(np.allclose(even, Bsym_eigs, atol=1e-9))
    out["full_equals_even_plus_odd"] = bool(np.allclose(np.sort(ev), np.sort(np.r_[even, odd]), atol=1e-9))
    deg = 4.0
    lap_even = np.sort(deg - even)
    lap_odd = np.sort(deg - odd)
    out["laplacian_gap_full"] = float(np.sort(deg - ev)[1])
    out["laplacian_gap_even_folded"] = float(lap_even[1])
    out["laplacian_lowest_odd_twin"] = float(lap_odd[0])
    # distances between twins g and -g
    global DIST
    DIST = np.full((n, n), -1, int)
    for s0 in range(n):
        DIST[s0, s0] = 0
        q = deque([s0])
        while q:
            u = q.popleft()
            for v in np.flatnonzero(A[u]):
                if DIST[s0, v] < 0:
                    DIST[s0, v] = DIST[s0, u] + 1
                    q.append(v)
    twin_d = [int(DIST[i, j]) for i, j in cells]
    out["twin_distance_min_max"] = [min(twin_d), max(twin_d)]
    out["diameter"] = int(DIST.max())
    # Ollivier-Ricci curvature on every edge (reported)
    kap = []
    for i in range(n):
        for j in np.flatnonzero(A[i]):
            if j > i:
                kap.append(ollivier(A, i, j))
    kap = np.array(kap)
    out["ollivier_edge_curvature_min_max"] = [float(kap.min()), float(kap.max())]
    # random-walk gap: 1 - second largest |eigenvalue| of P = A/deg (for Ollivier comparison)
    pe = np.sort(ev / deg)
    out["random_walk_gap_1_minus_lambda2"] = float(1 - pe[-2])
    print(json.dumps(out, indent=1))
    if receipt:
        receipt.parent.mkdir(parents=True, exist_ok=True)
        receipt.write_text(json.dumps(out, indent=1) + "\n")


DIST = None
if __name__ == "__main__":
    main()
