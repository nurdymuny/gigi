"""The floor formula across the Berger family g = (t, 1, 1) (double precision; exploration, not a certificate).

lambda_t(j) = [j(j+2) - 2(t-3) j - 2(t-2)(t+4)] / t is an exact gauge-orthogonal eigenvalue for every t > 0
(symbolic: scripts/ricci_su2_sector_formula_squashed.py). This scan checks (1) that it appears in the computed
slice spectrum, (2) for which t it is the bottom of every sector j <= JMAX, (3) the threshold t* below which a
lower eigenvalue appears (bisection on the first offending sector).
usage: python scripts/ricci_su2_floor_family_scan.py OUT.json [JMAX]
"""
import json
import sys
from pathlib import Path

import numpy as np
from scipy.linalg import null_space

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_sector_gauge_check as C   # noqa: E402

lam = lambda t, j: (j * (j + 2) - 2 * (t - 3) * j - 2 * (t - 2) * (t + 4)) / t   # noqa: E731


def slice_ev(t, j):
    H, G = C.hessian_and_gauge([t, 1, 1], j)
    N = null_space(G.conj().T)
    return np.linalg.eigvalsh(N.conj().T @ H @ N)


def main():
    out, jmax = Path(sys.argv[1]), int(sys.argv[2]) if len(sys.argv) > 2 else 40
    res = {"formula": "lambda_t(j) = [j(j+2) - 2(t-3) j - 2(t-2)(t+4)] / t", "jmax": jmax}
    res["branch_present_worst_distance"] = max(float(np.min(np.abs(slice_ev(t, j) - lam(t, j))))
                                               for t in (1.0, 1.5, 2.0, 2.5, 2.8, 3.0, 3.3, 4.0, 6.0) for j in range(13))
    table = {}
    for t in [1.0, 1.5, 2.0] + [round(x, 2) for x in np.arange(2.5, 3.01, 0.05)] + [3.5, 4.0, 5.0, 6.0, 8.0, 10.0]:
        table[str(t)] = [j for j in range(jmax + 1) if slice_ev(t, j)[0] < lam(t, j) - 1e-9]
    res["sectors_where_branch_is_not_the_floor"] = table
    lo, hi = 2.65, 2.7
    for _ in range(40):
        mid = (lo + hi) / 2
        lo, hi = (mid, hi) if slice_ev(mid, 4)[0] < lam(mid, 4) - 1e-11 else (lo, mid)
    res["threshold_t_star_first_sector_j4"] = (lo + hi) / 2
    out.write_text(json.dumps(res, indent=1) + "\n")
    print(json.dumps(res, indent=1))


if __name__ == "__main__":
    main()
