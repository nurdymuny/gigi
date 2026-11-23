"""The threshold of H + G G^* (as opposed to the gauge-orthogonal threshold t* ~ 2.698), double precision.

scripts/ricci_su2_tstar_exact.py finds that lambda_t(4) is an eigenvalue of the charge-0 block of the sector j = 4 at
t_A = 2.7311222782381136231 (largest real root of the degree-16 factor in s = sqrt t). This scan checks, with the
validated blocks of scripts/ricci_su2_charge_block_direct.py, that between t_A and 2.75 no non-twist block of any
sector j <= 40 lies below lambda_t(j), and which sectors have a block below it just under t_A.
usage: python scripts/ricci_su2_tA_scan.py OUT.txt
"""
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_charge_block_direct as D    # noqa: E402

T_A = 2.7311222782381136231


def lam(t, j):
    return (j * (j + 2) - 2 * (t - 3) * j - 2 * (t - 2) * (t + 4)) / t


def margin(t, j, fd):
    """min over non-twist blocks of (lowest eigenvalue - lambda_t(j)), and the number of eigenvalues below lambda."""
    l = j / 2; m = np.inf; nb = 0
    for q in np.arange(-l - 2, l + 2 + 1e-9, 1.0):
        if abs(abs(q) - (l + 2)) < 1e-9:
            continue
        _, B = D.block(t, j, q, fd)
        if B.size:
            e = np.linalg.eigvalsh(B)
            m = min(m, e.min() - lam(t, j)); nb += int((e < lam(t, j) - 1e-9).sum())
    return m, nb


def main():
    lines = []
    above = np.linspace(T_A + 1e-6, 2.75, 60)
    worst, viol = (np.inf, None), []
    for t in above:
        fd = D.frame_data(t)
        for j in range(41):
            m, nb = margin(t, j, fd)
            worst = min(worst, (m, (round(float(t), 6), j)))
            if nb:
                viol.append((float(t), j, m))
    lines.append(f"t in (t_A, 2.75] (60 points), j <= 40: violations {len(viol)}; smallest margin {worst[0]:.3e} at (t, j) = {worst[1]}")
    for t in (T_A - 1e-4, T_A - 1e-3, 2.72, 2.70):
        fd = D.frame_data(t)
        bad = [j for j in range(41) if margin(t, j, fd)[1]]
        lines.append(f"t = {t:.6f}: sectors with an eigenvalue of H + GG* below lambda_t(j): {bad}")
    txt = "\n".join(lines); print(txt)
    Path(sys.argv[1]).write_text(txt + "\n")


if __name__ == "__main__":
    main()
