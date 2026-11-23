"""Gauge content of the exact spin sectors at the Berger field (check of docs/research/su2-exact-sector-2026-11-19.md).

The Berger field is not at rest, so gauge directions are not zero modes of the Hessian; they are tilted, and they
can mix with physical directions inside a sector. In sector j the gauge directions are b_i = e^_i xi + [M_i, xi]
(xi in V_j (x) su(2), 3(j+1) of them). For each sector this reports the full Hessian's negative eigenvalues with
their gauge overlap, the Hessian restricted to the gauge-orthogonal (Coulomb) slice, and its Rayleigh range on the
gauge subspace. Uses scripts/ricci_su2_spin_sector.py's construction unchanged. Research script.

usage: python scripts/ricci_su2_sector_gauge_check.py [OUT.json]
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_spin_sector as S   # noqa: E402


def hessian_and_gauge(g, j, abelian=False):
    s, c, m, f = S.background(g, abelian)
    rho = S.spin_rep(j); d = j + 1; I3, Id = np.eye(3), np.eye(d); N = 9 * d
    idx = lambda i: slice(i * 3 * d, (i + 1) * 3 * d)                             # noqa: E731
    deriv = lambda i: np.kron(I3, rho[i] / s[i])                                  # noqa: E731
    ad = lambda v: np.kron(np.zeros((3, 3)) if abelian else S.cross_mat(v), Id)   # noqa: E731
    blocks = []
    for (i, jj) in S.PAIRS:
        D = np.zeros((3 * d, N), complex)
        D[:, idx(jj)] += deriv(i) + ad(m[i]); D[:, idx(i)] -= deriv(jj) + ad(m[jj])
        for k in range(3):
            D[:, idx(k)] -= c[i, jj, k] * np.eye(3 * d)
        blocks.append(D)
    Dall = np.vstack(blocks); H = Dall.conj().T @ Dall
    if not abelian:
        for (i, jj) in S.PAIRS:
            blk = np.kron(-S.cross_mat(f[(i, jj)]), Id)
            H[idx(i), idx(jj)] += blk; H[idx(jj), idx(i)] += blk.T
    Gm = np.vstack([deriv(i) + ad(m[i]) for i in range(3)])                       # xi -> gauge direction
    return H, Gm


def main():
    out = {}
    for j in (0, 12, 20, 24):
        H, Gm = hessian_and_gauge([3, 1, 1], j)
        lam, V = np.linalg.eigh(H)
        U, sv, _ = np.linalg.svd(Gm, full_matrices=True)
        r = int((sv > 1e-10 * sv[0]).sum())
        Qg, Qp = U[:, :r], U[:, r:]
        ov = np.linalg.norm(Qg.conj().T @ V, axis=0) ** 2
        neg = lam < -1e-9
        slice_ev = np.linalg.eigvalsh(Qp.conj().T @ H @ Qp)
        gauge_ev = np.linalg.eigvalsh(Qg.conj().T @ H @ Qg)
        out[f"j{j}"] = {"dim": int(H.shape[0]), "gauge_rank": r,
                        "full_negatives": [float(x) for x in lam[neg]],
                        "full_negatives_gauge_overlap": [float(x) for x in ov[neg]],
                        "full_lowest_12_with_gauge_overlap": [[float(a), float(b)] for a, b in zip(lam[:12], ov[:12])],
                        "slice_lowest_10": [float(x) for x in slice_ev[:10]],
                        "slice_negative_count": int((slice_ev < -1e-9).sum()),
                        "gauge_rayleigh_min_max": [float(gauge_ev.min()), float(gauge_ev.max())]}
        print(f"j={j}: slice lowest {np.round(slice_ev[:6], 10)}; slice negatives {out[f'j{j}']['slice_negative_count']}; "
              f"full negatives {int(neg.sum())} with gauge overlap {np.round(ov[neg].min(), 3)}..{np.round(ov[neg].max(), 3)}")
    if len(sys.argv) > 1:
        Path(sys.argv[1]).write_text(json.dumps(out, indent=1) + "\n")


if __name__ == "__main__":
    main()
