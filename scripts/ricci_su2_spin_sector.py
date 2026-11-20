"""The linearised SU(2) Yang-Mills operator at a left-invariant metric's Levi-Civita field, restricted to one
spin sector j (functions in the V_j isotypic component, on which left-invariant vector fields act through the
spin-j/2 representation). Research script.

Sector space: b = sum_i b_i theta^i, with b_i in V_j (x) su(2): 9 (j+1) complex unknowns. Quadratic form of the
Hessian of E = 1/2 |F|^2 (orthonormal left frame, brackets [e_i, e_j] = 2 e_k as e2e/probes/ricci_oracle.py):

    Q(b) = sum_{i<j} |D_ij b|^2 + 2 <F_ij, [b_i, b_j]>,
    D_ij b = e^_i b_j - e^_j b_i + [M_i, b_j] - [M_j, b_i] - sum_k c_ij^k b_k,

against the L2 mass sum_i |b_i|^2. On S^3/2I (quotient by left translations) sector j appears with multiplicity
dim V_j^{2I}: 1 for j = 0, 12, 20, 24, 30, ...; 0 for j = 1..11.

usage: python scripts/ricci_su2_spin_sector.py [OUT.json]
"""
import json
import math
import sys
from pathlib import Path

import numpy as np

BR = {}
for i, j, k in [(0, 1, 2), (1, 2, 0), (2, 0, 1)]:
    BR[(i, j, k)] = 2.0
    BR[(j, i, k)] = -2.0
PAIRS = [(0, 1), (1, 2), (0, 2)]


def spin_rep(j):
    """rho(e_a) = -2i J_a on V_j (dim j+1), so that [rho_0, rho_1] = 2 rho_2 (cyclic)."""
    s = j / 2
    m = np.arange(s, -s - 1, -1)
    d = len(m)
    Jz = np.diag(m).astype(complex)
    Jp = np.zeros((d, d), complex)
    for k in range(1, d):
        Jp[k - 1, k] = math.sqrt(s * (s + 1) - m[k] * (m[k] + 1))
    Jx, Jy = (Jp + Jp.conj().T) / 2, (Jp - Jp.conj().T) / (2j)
    rho = [-2j * Jx, -2j * Jy, -2j * Jz]
    for a, b, c in [(0, 1, 2), (1, 2, 0), (2, 0, 1)]:
        assert np.allclose(rho[a] @ rho[b] - rho[b] @ rho[a], 2 * rho[c])
    return rho


def cross_mat(v):
    a, b, c = v
    return np.array([[0, -c, b], [c, 0, -a], [-b, a, 0]], float)


def background(g, abelian=False):
    s = np.sqrt(np.asarray(g, float))
    c = np.zeros((3, 3, 3))
    for (i, j, k), v in BR.items():
        c[i, j, k] = v * s[k] / (s[i] * s[j])
    Gam = lambda i, j, k: 0.5 * (c[i, j, k] - c[j, k, i] + c[k, i, j])   # noqa: E731
    M = [np.array([[Gam(i, jj, k) for jj in range(3)] for k in range(3)]) for i in range(3)]
    m = [np.array([M[i][2, 1], M[i][0, 2], M[i][1, 0]]) for i in range(3)]      # so(3) matrix -> vector
    f = {}
    for (i, j) in PAIRS:
        comm = np.zeros(3) if abelian else np.cross(m[i], m[j])
        f[(i, j)] = comm - sum(c[i, j, k] * m[k] for k in range(3))
    return s, c, m, f


def sector_matrix(g, j, abelian=False):
    s, c, m, f = background(g, abelian)
    rho = spin_rep(j)
    d = j + 1
    I3, Id = np.eye(3), np.eye(d)
    N = 9 * d                                         # index (i, a, w): frame i, su(2) a, V_j w
    idx = lambda i: slice(i * 3 * d, (i + 1) * 3 * d)  # noqa: E731

    def deriv(i):       # e^_i acting on b_k: identity on su(2), rho/sqrt(g) on V_j
        return np.kron(I3, rho[i] / s[i])

    def ad(v):          # [v, .] on su(2) (cross product), identity on V_j
        return np.kron(np.zeros((3, 3)) if abelian else cross_mat(v), Id)
    blocks = []
    for (i, jj) in PAIRS:
        D = np.zeros((3 * d, N), complex)
        D[:, idx(jj)] += deriv(i) + ad(m[i])
        D[:, idx(i)] -= deriv(jj) + ad(m[jj])
        for k in range(3):
            D[:, idx(k)] -= c[i, jj, k] * np.eye(3 * d)
        blocks.append(D)
    Dall = np.vstack(blocks)
    H = Dall.conj().T @ Dall
    if not abelian:     # 2 <F_ij, [b_i, b_j]>: real bilinear B(b,b') = f.(b_i x b'_j + b'_i x b_j); Hermitian form B(conj b, b)
        for (i, jj) in PAIRS:
            # f.(x cross y) = x . (y cross f) = x^T C y with C = cross_mat(...)?  f.(x × y) = x^T [ (y × f) ] -> matrix P with x^T P y, P = -cross_mat(f)
            P = -cross_mat(f[(i, jj)])                 # x^T P y = f . (x × y)
            blk = np.kron(P, Id)
            H[idx(i), idx(jj)] += blk
            H[idx(jj), idx(i)] += blk.T
    assert np.allclose(H, H.conj().T, atol=1e-10)
    return np.linalg.eigvalsh(H)


def main():
    out = {}
    # controls
    e0 = np.sort(sector_matrix([3, 1, 1], 0))
    exact0 = sorted([-14 / 3] * 2 + [(29 - math.sqrt(1345)) / 3] * 2 + [16 - 2 * math.sqrt(73), 8.0] +
                    [(29 + math.sqrt(1345)) / 3] * 2 + [16 + 2 * math.sqrt(73)])
    out["control_j0_vs_exact_9x9_maxabs"] = float(np.max(np.abs(e0 - exact0)))
    ab = {}
    for jj in (1, 2, 12):
        ev = np.round(np.sort(sector_matrix([1, 1, 1], jj, abelian=True)), 8)
        vals, counts = np.unique(ev, return_counts=True)
        ab[jj] = {float(v): int(n) for v, n in zip(vals, counts)}
    out["control_round_abelian"] = ab
    # spin-6 sector (j = 12), Berger, full and abelianised; round full for reference
    for name, g, abel in (("berger_full", [3, 1, 1], False), ("berger_abelian", [3, 1, 1], True),
                          ("round_full", [1, 1, 1], False)):
        ev = np.sort(sector_matrix(g, 12, abel))
        out[f"j12_{name}"] = [float(x) for x in ev]
    print("control j=0 vs exact 9x9:", out["control_j0_vs_exact_9x9_maxabs"])
    for jj, d in ab.items():
        print(f"round abelian j={jj}:", d)
    for name in ("berger_full", "berger_abelian", "round_full"):
        ev = np.array(out[f"j12_{name}"])
        print(f"j=12 {name}: lowest 12:", np.round(ev[:12], 3), " count < 0:", int((ev < -1e-9).sum()),
              " |ev|<1e-9:", int((np.abs(ev) < 1e-9).sum()))
    if len(sys.argv) > 1:
        Path(sys.argv[1]).write_text(json.dumps(out, indent=1) + "\n")


if __name__ == "__main__":
    main()
