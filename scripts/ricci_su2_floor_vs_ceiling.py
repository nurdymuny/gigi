"""What puts the extremal state at the floor on the Berger field and not on the squashed meron? (exploratory)

For a left-invariant background A = sum m_i theta^i on (t, 1, 1), the Weitzenbock form H + G G^* = sum X_i^* X_i + R0
with X_i = rho_i / s_i (x) I + I (x) Omega_i, Omega_i = I (x) [m_i]x + (-Gamma_i) (x) I. Checked numerically: R(j) =
(H + G G^*) - sum X_i^* X_i equals R0 (x) I in every sector tested, at both backgrounds.
For the extremal state b = f_top (x) beta, the energy splits into fibre |X_0 b|^2, base |X_1 b|^2 + |X_2 b|^2 and
curvature <beta, R0 beta>; reported with beta's place in the spectrum of R0 and the X_+ b residual.
usage: python scripts/ricci_su2_floor_vs_ceiling.py
"""
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_spin_sector as S                 # noqa: E402
import ricci_su2_soft_mode_flow as F              # noqa: E402
import ricci_su2_meron_family_soft as MF          # noqa: E402


def gamma_mats(t):
    s, c = F.structure(t)
    Gam = lambda i, a, b: 0.5 * (c[i, a, b] - c[a, b, i] + c[b, i, a])       # noqa: E731
    return s, c, [np.array([[Gam(i, a, b) for b in range(3)] for a in range(3)]) for i in range(3)]


def weitzenbock_parts(t, m, j):
    s, c, G = gamma_mats(t)
    Om = [np.kron(np.eye(3), S.cross_mat(m[i])) + np.kron(-G[i], np.eye(3)) for i in range(3)]
    d = j + 1; rho = S.spin_rep(j)
    X = [np.kron(np.eye(9), rho[i] / s[i]) + np.kron(Om[i], np.eye(d)) for i in range(3)]
    A = F.operator(np.asarray(m), s, c, j)
    R = A - sum(x.conj().T @ x for x in X)
    R0 = R[::d, ::d]
    return A, X, R, R0, Om


def main():
    for t in (1.0, 2.0, 3.0, 4.0, 6.0):
        backgrounds = {"Berger field": F.lc_background(t)}
        xm = MF.follow(np.round(np.arange(1.0, t + 1e-9, 0.05), 4))[-1] if t >= 1 else None
        backgrounds["squashed meron"] = xm.reshape(3, 3)
        for name, m in backgrounds.items():
            devs = []
            for j in (1, 2, 4):
                A, X, R, R0, Om = weitzenbock_parts(t, m, j)
                devs.append(float(np.abs(R - np.kron(R0, np.eye(j + 1))).max()))
            _, _, _, R0, Om = weitzenbock_parts(t, m, 0)
            beta = np.zeros(9, complex); beta[[4, 5, 7, 8]] = [-1, -1j, -1j, 1]; beta /= np.linalg.norm(beta)
            w = np.linalg.eigvalsh(0.5 * (R0 + R0.T))
            rb = float((beta.conj() @ R0 @ beta).real)
            rank = int((w < rb - 1e-9).sum()) + 1
            xplus = float(np.linalg.norm((Om[1] + 1j * Om[2]) @ beta))
            print(f"t={t:4.1f} {name:15s}: Weitzenbock R(j) = R0 (x) I to {max(devs):.1e}; R0 spectrum "
                  f"[{w.min():+.2f} .. {w.max():+.2f}], <beta,R0 beta> = {rb:+.3f} (rank {rank} of 9); "
                  f"|(Omega_1 + i Omega_2) beta| = {xplus:.2e}", flush=True)


if __name__ == "__main__":
    main()
