"""The floor at the bottom of the basin (22 November 2026), for docs/research/the-basin-2026-11-22.md, open question 1.

The Yang-Mills flow from the Berger field lands on the flat field A = theta (x = 2/sqrt t, y = 2), gauge-equivalent to
A = 0 by a gauge transformation of degree one. At A = 0, H + GG^* is the Hodge Laplacian on su(2)-valued 1-forms and the
gauge slice is the coexact 1-forms. This script computes the slice (gauge-orthogonal) spectrum per sector j <= 12 at A = 0
and A = theta on (t,1,1), checks that the two L2 spectra agree as multisets (each sector-j eigenvalue counted j+1 times) below a safe cutoff, records the floor, the sector holding
it and its multiplicity, Helike's value there, and compares with Henkel-Lauret (arXiv:2605.05406): their metric makes
{aE1, bE2, cE3} orthonormal with [E1,E2] = 2E3 (cyclic), so (t,1,1) is a = 1/sqrt t, b = c = 1; their Prop. 3.6 at b = c
gives the k = 0 eigenvalues {4a^2, 4a^2, 4b^4/a^2} = {4/t, 4/t, 4t}; their Theorem 4.1 coexact branch at k = 2, j = 1 is
mu^- = nu + 2b^4/a^2 - 2(b^2/a^2) sqrt(a^2 nu + b^4) with nu = 8.
usage: OMP_NUM_THREADS=1 python scripts/basin_floor_at_bottom.py OUT.json
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su3_test2_family as T2        # noqa: E402

S, C2, eps = T2.su2_data()
rows = []
for t in (0.5, 0.8, 0.9, 1.0, 1.2, 1.5, 2.0, 2.75, 3.0, 3.55, 4.0, 6.0, 8.0, 10.0):
    sp = {}
    for name, (x, y) in (("A0", (0.0, 0.0)), ("theta", (2 / np.sqrt(t), 2.0))):
        sp[name] = []
        for j in range(13):
            H, G = T2.hessian_and_gauge(C2, [t, 1, 1], eps, [x * np.eye(3)[0], y * np.eye(3)[1], y * np.eye(3)[2]], S.spin_rep(j))
            sp[name].append(np.sort(T2.slice_spectrum(H, G)))
    # gauge-equivalent fields have the same L2 spectrum: compare the multisets (sector-j eigenvalue counted j+1 times)
    # below a cutoff that sectors j > 12 cannot reach (their lowest value grows; cutoff = smallest sector-12 minimum)
    cut = min(sp["A0"][12][0], sp["theta"][12][0]) - 1e-6
    ms = {k: np.sort(np.concatenate([np.repeat(v[v < cut], j + 1) for j, v in enumerate(sp[k])])) for k in sp}
    shift = float(np.abs(ms["A0"] - ms["theta"]).max()) if len(ms["A0"]) == len(ms["theta"]) else float("inf")
    mins = [float(v[0]) for v in sp["A0"]]; fl = min(mins)
    secs = [j for j in range(13) if abs(mins[j] - fl) < 1e-9 * max(1, fl)]
    mult = sum(int((np.abs(sp["A0"][j] - fl) < 1e-9 * max(1, fl)).sum()) for j in secs)
    a2 = 1 / t; nu = 8.0
    mu_minus_k2 = nu + 2 / a2 - 2 * (1 / a2) * np.sqrt(a2 * nu + 1)
    rows.append({"t": t, "floor": fl, "sectors_A0": secs, "multiplicity": mult, "min(4/t,4t)": min(4 / t, 4 * t),
                 "helike_A0_sector0": 4 / t, "helike_theta_min": 16 / t, "theta_vs_A0_L2_multiset_maxdiff_below_cutoff": shift, "n_below_cutoff": int(len(ms["A0"])),
                 "A0_sector2_min": float(sp["A0"][2][0]), "HL_mu_minus_k2_j1": float(mu_minus_k2),
                 "A0_sector1_min": float(sp["A0"][1][0]), "HL_9a2": 9 / t})
    print(rows[-1], flush=True)
Path(sys.argv[1]).write_text(json.dumps({"status": "COMPUTED (double precision, sectors j <= 12)", "rows": rows}, indent=1) + "\n")
