"""WP5 design checks for pre-registration D: SU(3) stretched along the twist's own charge circle (22 November 2026).

Structural only: no floor, edge or competitor is computed.
  D5a  basis: e_H = X_H/|X_H| with H = -i diag(1,0,-1) (the twist's charge), e_K its orthogonal complement in the
       Cartan, the six root vectors; structure constants in the new orthonormal basis Y.
  D5b  the joint (frame + gauge) commutant of t^2 = span(e_H, e_K) on left-invariant fields M[i, a]: dimension
       (expected 10: 2x2 Cartan block + aI + bJ on each of the three root planes) and the constant-gauge reduction by
       T^2 (two phases removable, so 8 parameters up to gauge).
  D5c  the reduced family M(x, w, y12, y23, y13): x on e_H (the field along her circle), w on e_K, y_k * identity on
       root plane k; checked to lie in the commutant.
  D5d  at one random member: Q (circle of H, joint frame + gauge + V) commutes with H and GG*, the charge-(top)
       eigenspace is one-dimensional in each of the nine sectors, b is an exact gauge-orthogonal eigenvector; the
       second x-difference of lambda_b (step 1) in each sector (her x^2 coefficient times 2); copy counts n_b.
  D5e  the Maurer-Cartan member in this basis (M_a = Y_a / (2 sqrt g_a)) has the same energy as Test 2's.
usage: OMP_NUM_THREADS=1 python scripts/edge_wp5_design_checks.py OUT.json
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su3_test2_family as T2        # noqa: E402

X, C = T2.su3_data()
out = {}
# ---- D5a: rotated orthonormal basis Y = R X (rows of R)
XH = -1j * np.diag([1.0, 0, -1.0])
h = np.array([float((-2 * np.trace(XH @ X[a])).real) for a in range(8)])
nh = np.linalg.norm(h); eH = h / nh
eK = np.zeros(8); eK[2], eK[7] = eH[7], -eH[2]                       # orthogonal in span(X3, X8)
R = np.eye(8); R[2], R[7] = eH, eK                                    # Y_2 = e_H, Y_7 = e_K, roots unchanged
assert np.allclose(R @ R.T, np.eye(8))
Y = np.einsum("ab,bij->aij", R, X)
Cy = np.einsum("ai,bj,ijk,ck->abc", R, R, C, R)
for a in range(8):
    for b in range(8):
        Z = Y[a] @ Y[b] - Y[b] @ Y[a]
        assert np.allclose(Z, np.einsum("k,kij->ij", Cy[a, b], Y), atol=1e-13)
out["D5a"] = {"|X_H| in the orthonormal basis": float(nh), "e_H (X coords)": eH.tolist(), "e_K (X coords)": eK.tolist()}
H_idx, K_idx = 2, 7
roots = {"12": (0, 1), "13": (3, 4), "23": (5, 6)}                    # (X1,X2), (X4,X5), (X6,X7)
# ---- D5b: commutant of the joint t^2 action
eqs = []
for gi in (H_idx, K_idx):
    A = Cy[gi].T
    E = np.zeros((64, 64))
    for i in range(8):
        for a_ in range(8):
            Zm = np.zeros((8, 8)); Zm[i, a_] = 1
            E[:, i * 8 + a_] = (A @ Zm + Zm @ A.T).ravel()
    eqs.append(E)
E = np.vstack(eqs); _, sv, Vt = np.linalg.svd(E)
out["D5b_commutant_dim"] = int((sv <= 1e-10).sum() + (64 - len(sv)))
# ---- D5c: reduced family
def field(x, w, y12, y23, y13):
    M = np.zeros((8, 8)); M[H_idx, H_idx] = x; M[K_idx, K_idx] = w
    for k, v in (("12", y12), ("23", y23), ("13", y13)):
        for i in roots[k]:
            M[i, i] = v
    return M
Mt = field(0.3, 0.7, 0.4, 0.9, 1.3)
out["D5c_reduced_in_commutant_residual"] = float(np.abs(E @ Mt.ravel()).max())
# H-weights of the root planes (her charge under ad(H) on each plane): eigenvalues of i ad(X_H) there
adH = np.einsum("c,cab->ba", h @ R.T, Cy)                              # ad(X_H) in the Y basis (matrix acting on coords)
wts = {k: sorted(np.round(np.linalg.eigvals(1j * adH[np.ix_(v, v)]).real, 10).tolist()) for k, v in roots.items()}
out["D5c_H_weights_of_root_planes"] = wts
# ---- D5d: one random member, nine sectors
rho = {pq: [sum(R[a, b] * r_ for b, r_ in enumerate(T2.su3_irrep(*pq, X))) for a in range(8)] for pq in T2.SECTORS}
def metric(t):
    g = np.ones(8); g[H_idx] = t; return g
rng = np.random.default_rng(522)
xv, wv, y12, y23, y13 = rng.uniform(-1, 1), rng.uniform(0.3, 1.5), rng.uniform(0.3, 1.5), rng.uniform(0.3, 1.5), rng.uniform(0.3, 1.5)
tv = 3.7
d5d = {"member": [xv, wv, y12, y23, y13, tv]}
for pq in T2.SECTORS:
    r_ = rho[pq]; d = r_[0].shape[0]
    Hs, lam = [], []
    for xx in (xv - 1, xv, xv + 1):
        M = field(xx, wv, y12, y23, y13)
        Hm, G = T2.hessian_and_gauge(Cy, metric(tv), Cy, [M[i] for i in range(8)], r_)
        if xx == xv:
            parts = (1j * adH, 1j * adH, 1j * sum(h[a] * T2.su3_irrep(*pq, X)[a] for a in range(8)))
            signs = T2.fit_signs(parts, Hm, G @ G.conj().T, 8, 8, d)
            Q = T2.assemble_Q(parts, signs[0], 8, 8, d) if len(signs) == 1 else None
            if Q is None:
                d5d[str(pq)] = {"signs": str(signs)}; break
            top = pq[0] + pq[1] + 4
            if abs(np.linalg.eigvalsh(Q).max() - top) > 1e-8:
                Q = -Q
            row = T2.classify_point(Hm, G, Q, top)
            mu = T2.slice_spectrum(Hm, G)
            nb = int((np.abs(mu - row.get("extremal", np.nan)) <= 1e-9 * max(1, abs(row.get("extremal", 1)))).sum())
            d5d[str(pq)] = {"signs": signs[0], "top_unique_and_exact": "error" not in row, "row_error": row.get("error"),
                            "n_b": nb, "is_floor_here (not scored)": None}
        q, U = np.linalg.eigh(Q) if xx != xv - 1 else (None, None)
    if Q is None:
        continue
    q, U = np.linalg.eigh(Q); b = U[:, np.argmax(q)]
    for xx in (xv - 1, xv, xv + 1):
        M = field(xx, wv, y12, y23, y13)
        Hm, _ = T2.hessian_and_gauge(Cy, metric(tv), Cy, [M[i] for i in range(8)], r_)
        lam.append(float((b.conj() @ Hm @ b).real))
    d5d[str(pq)]["second_x_difference"] = lam[0] - 2 * lam[1] + lam[2]
    print(pq, d5d[str(pq)], flush=True)
out["D5d"] = d5d
# ---- D5e: Maurer-Cartan member energy equals Test 2's at t = 1 (bi-invariant: same field)
g1 = np.ones(8)
Emc = T2.energy(Cy, g1, Cy, [0.5 * np.eye(8)[a] for a in range(8)]); Emc0 = T2.energy(C, g1, C, T2.su3_background(g1))
out["D5e_MC_energy_rotated_vs_test2"] = [Emc, Emc0]
Path(sys.argv[1]).write_text(json.dumps(out, indent=1, default=str) + "\n")
print(json.dumps({k: v for k, v in out.items() if k != "D5d"}, indent=1, default=str))
