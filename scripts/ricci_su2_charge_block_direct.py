"""Direct assembly of the charge blocks of H + G G^* on the Berger sphere (t, 1, 1), without the full sector matrix.

Sector of spin l = j/2. Representation rho'_0 = -2i J_z, rho'_1 = -2i J_x, rho'_2 = -2i J_y (a cyclic relabelling of
ricci_su2_spin_sector.spin_rep: same brackets [rho_a, rho_b] = 2 eps_abc rho_c, unitarily equivalent, so the same
spectrum), so the long-axis weight m is diagonal. Frame and Lie indices are taken in the eigenbasis of the rotation
about axis 0 (charges c_f, c_l in {+1, 0, -1}). The fibre charge q = m + c_f + c_l is conserved, and the block of
charge q has the states (m, c_f, c_l) with m = q - c_f - c_l, |m| <= l: at most 9.
  A = sum_i X_i^* X_i + R0 (Weitzenbock, exact),  X_i = rho'_i / sqrt(g_i) (x) I_9 + I (x) Omega_i.
Each X_i P_q is computed on the at most 27 states it reaches (m' in {m-1, m, m+1}); the block is
P_q^* (sum_i X_i^* X_i + R0) P_q. Cost O(1) per block, independent of j.
usage: python scripts/ricci_su2_charge_block_direct.py   (validation against the projected sector matrices)
"""
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_sector_minimality as Mn     # noqa: E402


def frame_data(t):
    """Omega_i (9x9) and R0 (9x9) in the charge eigenbasis of the axis-0 rotation, charges of the 9 basis states."""
    Mn.G_BERGER = [t, 1, 1]
    Om, s_ = Mn.omegas()
    R0, _, _, _ = Mn.weitzenbock(js=(0, 1, 2))
    R = np.zeros((3, 3), complex); R[1, 2] = -1j; R[2, 1] = 1j          # rotation about axis 0 on a 3-vector index
    c3, U3 = np.linalg.eigh(R)                                           # charges -1, 0, +1
    c3 = np.round(c3).astype(int)
    U = np.kron(U3, U3)                                                  # frame (x) Lie
    ch = np.array([[a, b] for a in c3 for b in c3])                      # (c_f, c_l) for each of the 9 states
    Omc = [U.conj().T @ O @ U for O in Om]
    R0c = U.conj().T @ R0 @ U
    return s_, Omc, R0c, ch


def ladder(l, m, sgn):
    v = l * (l + 1) - m * (m + sgn)
    return np.sqrt(max(v, 0.0))


def rho_action(i, l, m):
    """rho'_i |m> as a dict {m': coefficient}."""
    if i == 0:
        return {m: -2j * m}
    up, dn = ladder(l, m, 1), ladder(l, m, -1)          # J+|m> = up |m+1>, J-|m> = dn |m-1>
    out = {}
    if i == 1:                                           # -2i J_x = -i (J+ + J-)
        if up: out[m + 1] = -1j * up
        if dn: out[m - 1] = -1j * dn
    else:                                                # -2i J_y = -(J+ - J-)
        if up: out[m + 1] = -up
        if dn: out[m - 1] = dn
    return out


def block(t, j, q, data=None):
    """The charge-q block of H + G G^* in sector j (states and matrix)."""
    s_, Omc, R0c, ch = data if data is not None else frame_data(t)
    l = j / 2
    states = [(q - ch[k][0] - ch[k][1], k) for k in range(9) if abs(q - ch[k][0] - ch[k][1]) <= l + 1e-9]
    if not states:
        return [], np.zeros((0, 0))
    targets = {}

    def tidx(key):
        if key not in targets:
            targets[key] = len(targets)
        return targets[key]
    cols = []
    for i in range(3):
        Xi = {}
        for col, (m, k) in enumerate(states):
            for mp, cf in rho_action(i, l, m).items():                       # rho' part on V_j
                Xi[(tidx((mp, k)), col)] = Xi.get((tidx((mp, k)), col), 0) + cf / s_[i]
            for kp in range(9):                                               # Omega part on frame (x) Lie
                v = Omc[i][kp, k]
                if abs(v) > 1e-15:
                    Xi[(tidx((m, kp)), col)] = Xi.get((tidx((m, kp)), col), 0) + v
        cols.append(Xi)
    T = len(targets); n = len(states)
    B = np.zeros((n, n), complex)
    for Xi in cols:
        M = np.zeros((T, n), complex)
        for (r, c), v in Xi.items():
            M[r, c] = v
        B += M.conj().T @ M
    for a, (m, k) in enumerate(states):
        for b, (m2, k2) in enumerate(states):
            if m == m2:
                B[a, b] += R0c[k, k2]
    return states, 0.5 * (B + B.conj().T)


def sector_from_blocks(t, j):
    data = frame_data(t); l = j / 2
    qs = np.arange(-l - 2, l + 2 + 1e-9, 1.0)
    ev = []
    for q in qs:
        _, B = block(t, j, q, data)
        if B.size:
            ev.extend(np.linalg.eigvalsh(B))
    return np.sort(np.array(ev))


def main():
    import ricci_su2_sector_gauge_check as C
    worst = 0.0
    for t in (2.0, 3.0, 4.0, 6.5):
        for j in (0, 1, 2, 3, 5, 8, 12):
            H, G = C.hessian_and_gauge([t, 1, 1], j)
            ref = np.linalg.eigvalsh(H + G @ G.conj().T)
            got = sector_from_blocks(t, j)
            d = float(np.max(np.abs(ref - got))) if len(ref) == len(got) else float("inf")
            worst = max(worst, d / max(1.0, np.abs(ref).max()))
            print(f"t={t} j={j}: {len(got)} eigenvalues from blocks vs {len(ref)} projected, max diff {d:.2e}", flush=True)
    print("worst relative difference", worst)


if __name__ == "__main__":
    main()
