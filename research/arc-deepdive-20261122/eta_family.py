"""The two hands and the curl: first-order (odd signature / Chern-Simons-Hessian) operator for the circle-symmetric
family on the Berger sphere (t,1,1), sector by sector. Research script for lit_eta.md.

Conventions as in the edge paper (docs/papers/by-enough/paper.tex, Section 2) and scripts/ricci_su2_spin_sector.py:
frame [e_a,e_b] = 2 eps e_c, metric (t,1,1), su(2) as R^3 with the cross product, M_0 = x tau_0, M_1 = y tau_1,
M_2 = y tau_2. Orientation: theta^0 ^ theta^1 ^ theta^2 > 0.

Checks (all COMPUTED):
  C1  the family builder at the Berger point reproduces sector_matrix((t,1,1), j) (the Levi-Civita field);
  C2  curl_A = *d_A on ad-valued 1-forms is Hermitian, and H = curl_A^2 + S_F exactly (S_F = the 2<F,[b,b]> term);
  C3  the twist value lambda(x,y,t,j) of the closed form is an H-eigenvalue of multiplicity 2 (twist + mirror);
  C4  curl_A restricted to that eigenspace: is it mu*I (twist and mirror share a curl eigenvalue) or +-mu?
  C5  at y = 0, the sister eigenspace (value lambda(-x,0,t,j)) and its curl values;
  C6  the twist's curl eigenvalue mu against candidate closed forms.
usage: OMP_NUM_THREADS=1 python eta_family.py
"""
import json
import math
import sys
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[1] / "scripts"))
import ricci_su2_spin_sector as SS  # noqa: E402

PAIRS = SS.PAIRS                     # (0,1),(1,2),(0,2)
STAR = {(0, 1): (2, 1.0), (1, 2): (0, 1.0), (0, 2): (1, -1.0)}   # *(theta^i ^ theta^j) = sign * theta^k


def frame_data(t):
    s = np.sqrt(np.array([t, 1.0, 1.0]))
    c = np.zeros((3, 3, 3))
    for (i, j, k), v in SS.BR.items():
        c[i, j, k] = v * s[k] / (s[i] * s[j])
    return s, c


def family_m(x, y):
    return [np.array([x, 0.0, 0.0]), np.array([0.0, y, 0.0]), np.array([0.0, 0.0, y])]


def operators(t, j, m, abelian=False):
    """Return H (9d x 9d), curl_A (9d x 9d), G (9d x 3d) in sector j for constant field m (list of 3 R^3 vectors)."""
    s, c = frame_data(t)
    rho = SS.spin_rep(j)
    d = j + 1
    I3, Id = np.eye(3), np.eye(d)
    N = 9 * d
    idx = lambda i: slice(i * 3 * d, (i + 1) * 3 * d)  # noqa: E731
    deriv = lambda i: np.kron(I3, rho[i] / s[i])  # noqa: E731
    ad = lambda v: np.kron(np.zeros((3, 3)) if abelian else SS.cross_mat(v), Id)  # noqa: E731
    f = {}
    for (i, jj) in PAIRS:
        comm = np.zeros(3) if abelian else np.cross(m[i], m[jj])
        f[(i, jj)] = comm - sum(c[i, jj, k] * m[k] for k in range(3))
    Dblk = {}
    for (i, jj) in PAIRS:
        D = np.zeros((3 * d, N), complex)
        D[:, idx(jj)] += deriv(i) + ad(m[i])
        D[:, idx(i)] -= deriv(jj) + ad(m[jj])
        for k in range(3):
            D[:, idx(k)] -= c[i, jj, k] * np.eye(3 * d)
        Dblk[(i, jj)] = D
    Dall = np.vstack([Dblk[p] for p in PAIRS])
    H = Dall.conj().T @ Dall
    SF = np.zeros((N, N), complex)
    if not abelian:
        for (i, jj) in PAIRS:
            P = -SS.cross_mat(f[(i, jj)])
            blk = np.kron(P, Id)
            SF[idx(i), idx(jj)] += blk
            SF[idx(jj), idx(i)] += blk.T
    H = H + SF
    curl = np.zeros((N, N), complex)
    for p, D in Dblk.items():
        k, sg = STAR[p]
        curl[idx(k), :] += sg * D
    G = np.zeros((N, 3 * d), complex)
    for i in range(3):
        G[idx(i), :] = deriv(i) + ad(m[i])
    return H, curl, G, SF, f


def lam_closed(x, y, t, j):
    return (x + (j + 2) / math.sqrt(t)) ** 2 + 2 * math.sqrt(t) * x - y * y


def eigspace(Hm, val, tol=1e-8):
    w, V = np.linalg.eigh(Hm)
    sel = np.abs(w - val) < tol * max(1.0, abs(val))
    return V[:, sel], int(sel.sum())


def main():
    out = {}
    rng = np.random.default_rng(20261122)
    # C1: Berger point vs Levi-Civita builder
    c1 = 0.0
    for t in (0.7, 3.0, 6.5):
        x, y = (2 - t) / math.sqrt(t), math.sqrt(t)
        for j in (0, 1, 4):
            H, *_ = operators(t, j, family_m(x, y))
            e1 = np.sort(np.linalg.eigvalsh(H))
            e2 = np.sort(SS.sector_matrix([t, 1, 1], j))
            c1 = max(c1, float(np.max(np.abs(e1 - e2))))
    out["C1_berger_point_vs_levi_civita_builder_maxabs"] = c1
    # C2: curl Hermitian; H = curl^2 + S_F
    c2h = c2s = 0.0
    for _ in range(6):
        x, y, t = rng.uniform(-3, 2), rng.uniform(0, 3), rng.uniform(0.5, 8)
        for j in (0, 2, 5):
            H, curl, G, SF, _ = operators(t, j, family_m(x, y))
            c2h = max(c2h, float(np.max(np.abs(curl - curl.conj().T))))
            c2s = max(c2s, float(np.max(np.abs(H - (curl @ curl + SF)))))
    out["C2_curl_hermitian_maxabs"] = c2h
    out["C2_H_minus_curl2_minus_SF_maxabs"] = c2s
    # C3-C6
    rows = []
    for _ in range(12):
        x, y, t = rng.uniform(-3, 1.5), rng.uniform(0.2, 3), rng.uniform(1.5, 9)
        for j in (1, 3, 6):
            H, curl, G, SF, f = operators(t, j, family_m(x, y))
            lam = lam_closed(x, y, t, j)
            V, mult = eigspace(H, lam)
            row = {"x": x, "y": y, "t": t, "j": j, "lam": lam, "mult": mult}
            if mult == 2:
                C = V.conj().T @ curl @ V
                mus = np.linalg.eigvalsh((C + C.conj().T) / 2)
                row["curl_on_twist_space"] = [float(v) for v in mus]
                row["curl_offdiag_leak"] = float(np.linalg.norm(curl @ V - V @ C))
                # S_F on the twist space
                S = V.conj().T @ SF @ V
                row["SF_on_twist_space"] = [float(v) for v in np.linalg.eigvalsh((S + S.conj().T) / 2)]
                row["GstarV_norm"] = float(np.linalg.norm(G.conj().T @ V))
            rows.append(row)
    out["twist_rows"] = rows
    # C5: y = 0, sister space at lambda(-x,0)
    srows = []
    for _ in range(8):
        x, t = rng.uniform(-3, 3), rng.uniform(1.5, 9)
        for j in (1, 4):
            H, curl, G, SF, f = operators(t, j, family_m(x, 0.0))
            for lab, val in (("twist", lam_closed(x, 0, t, j)), ("sister", lam_closed(-x, 0, t, j))):
                V, mult = eigspace(H, val)
                C = V.conj().T @ curl @ V
                mus = np.linalg.eigvalsh((C + C.conj().T) / 2) if mult else []
                srows.append({"x": x, "t": t, "j": j, "which": lab, "val": val, "mult": mult,
                              "curl": [float(v) for v in mus],
                              "leak": float(np.linalg.norm(curl @ V - V @ C)) if mult else None})
    out["y0_rows"] = srows
    path = HERE / "eta_family_receipt.json"
    path.write_text(json.dumps(out, indent=1) + "\n")
    print(json.dumps({k: v for k, v in out.items() if k.startswith("C")}, indent=1))
    for r in rows[:12]:
        print(r)
    for r in srows[:8]:
        print(r)


if __name__ == "__main__":
    main()
