"""Split of the Weitzenbock curvature term into Ricci and field strength, and the twist's share of each (exploratory).

R0 is the constant term of H + G G^* = sum_i X_i^* X_i + R0. Checks numerically that R0 = Ric (x) I - 2 [F, .], where
(Ric (x) I) is R0 at the zero background and ([F, .] b)_i = sum_j [F_ij, b_j]: the Ricci term plus the field-strength
term with the factor 2 of the Yang-Mills Hessian (the gyromagnetic ratio 2 of a spin-1 gauge excitation, as in
Nielsen-Olesen). Then splits <beta, R0 beta> for the twist's internal frame beta into its Ricci and field-strength
parts, on the Berger field and on the squashed meron (resting field), at several stretches t.
usage: python scripts/ricci_su2_gyromagnetic_split.py OUT.txt
"""
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_floor_vs_ceiling as FC          # noqa: E402
import ricci_su2_meron_family_soft as MF         # noqa: E402
import ricci_su2_soft_mode_flow as F             # noqa: E402
import ricci_su2_spin_sector as S                # noqa: E402


def main():
    beta = np.zeros(9, complex); beta[[4, 5, 7, 8]] = [-1, -1j, -1j, 1]; beta /= np.linalg.norm(beta)
    lines = []
    for t in (1.0, 2.0, 3.0, 4.0, 6.0, 8.0):
        bgs = {"Berger field": np.asarray(F.lc_background(t)),
               "squashed meron": MF.follow(np.round(np.arange(1.0, t + 1e-9, 0.05), 4))[-1].reshape(3, 3)}
        for name, m in bgs.items():
            _, _, _, R0, _ = FC.weitzenbock_parts(t, m, 0)
            _, _, _, R0z, _ = FC.weitzenbock_parts(t, np.zeros((3, 3)), 0)
            s, c = F.structure(t)
            T = np.zeros((9, 9))
            for i in range(3):
                for j in range(3):
                    Fij = np.cross(m[i], m[j]) - sum(c[i, j, k] * m[k] for k in range(3))
                    T[3 * i:3 * i + 3, 3 * j:3 * j + 3] = S.cross_mat(Fij)
            dev = float(np.abs(R0 - R0z + 2 * T).max())
            ric = float((beta.conj() @ R0z @ beta).real); spin = float((beta.conj() @ (R0 - R0z) @ beta).real)
            lines.append(f"t={t:4.1f} {name:15s}: |R0 - (Ric - 2[F,.])| = {dev:.1e};  <beta, Ric beta> = {ric:+8.4f}, "
                         f"<beta, -2[F,.] beta> = {spin:+8.4f}, total = {ric + spin:+8.4f}")
    lines.append("Berger field: Ricci share 4 - 2t, field-strength share 8 - 6t, total 12 - 8t (fits every row above).")
    txt = "\n".join(lines); print(txt)
    Path(sys.argv[1]).write_text(txt + "\n")


if __name__ == "__main__":
    main()
