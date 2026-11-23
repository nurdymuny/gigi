"""The squashed meron's extremal state is the TOP of the gauge-orthogonal spectrum for t <= 1 (Floor paper v2, Section 6,
'Further observations': symmetry does not make the floor). Every grid point t = 0.05..1.00 (step 0.05) and every
sector j <= 12, with the construction of scripts/ricci_su2_test2_family.py (meron continued down from t = 1).
usage: python scripts/ricci_su2_meron_ceiling_check.py OUT.txt
"""
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_meron_family_soft as MF     # noqa: E402
import ricci_su2_soft_mode_flow as F         # noqa: E402
import ricci_su2_test2_family as T2          # noqa: E402


def main():
    down = [t for t in T2.GRID[::-1] if t <= 1.0]
    bg = {float(t): x.reshape(3, 3) for t, x in zip(down, MF.follow(down))}
    Qs = {j: np.linalg.eigh(T2.charge(j)) for j in range(13)}
    notop, worst, n = [], np.inf, 0
    for t in sorted(bg):
        s, c = F.structure(t); m = bg[t]
        for j in range(13):
            H, G = T2.h_and_g(m, s, c, j); q, U = Qs[j]
            b = U[:, np.abs(q - (j / 2 + 2)) < 1e-8][:, 0]
            lb = float((b.conj() @ H @ b).real)
            Uf, sv, _ = np.linalg.svd(G, full_matrices=True); r = int((sv > 1e-10 * sv.max()).sum())
            mu = np.linalg.eigvalsh(Uf[:, r:].conj().T @ H @ Uf[:, r:])
            gap = lb - mu[-1]; worst = min(worst, gap); n += 1
            if gap < -1e-9 * max(1.0, abs(mu[-1])):
                notop.append((t, j, gap))
    txt = (f"squashed meron, t = 0.05..1.00, j = 0..12: {n} points; extremal state below the top of the "
           f"gauge-orthogonal spectrum at {len(notop)} of them {notop[:10]}; min (extremal - top) = {worst:.2e}")
    print(txt); Path(sys.argv[1]).write_text(txt + "\n")


if __name__ == "__main__":
    main()
