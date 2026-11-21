"""The map (step 2, 21 November 2026; exploratory, double precision, not pre-registered).

Circle-symmetric family on (t, 1, 1): m_0 = x e_0, m_1 = y e_1, m_2 = y e_2. For each (x, y, t) on a grid: is the twist
the floor of the gauge-orthogonal spectrum in every sector j <= JMAX, how many sectors does it win, and which sector
fails first. The twist's value is the closed form lambda = (x + (j+2)/sqrt t)^2 + 2 sqrt(t) x - y^2
(scripts/ricci_su2_twist_family_closed_form.py); the floor test is as in Tests 2-5.
Marked points: the Berger field (x = (2-t)/sqrt t, y = sqrt t), the squashed meron (numerical continuation), and the
line A = s theta (x = y/sqrt t).
usage: python scripts/ricci_su2_twist_family_map.py OUT.json [NPROC]
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))

TS = [1.0, 2.0, 3.0, 4.0, 6.0]
XS = np.round(np.linspace(-3, 3, 49), 4)
YS = np.round(np.linspace(0, 3, 37), 4)
JMAX = 8
_C = {}


def lam(x, y, t, j):
    return (x + (j + 2) / np.sqrt(t)) ** 2 + 2 * np.sqrt(t) * x - y ** 2


def cell(args):
    x, y, t = args
    import ricci_su3_test2_family as T2
    if not _C:
        S, C2, eps = T2.su2_data(); _C.update(S=S, C2=C2, eps=eps, rho={j: S.spin_rep(j) for j in range(JMAX + 1)})
    C2, eps = _C["C2"], _C["eps"]
    g = [t, 1.0, 1.0]; M = [x * np.eye(3)[0], y * np.eye(3)[1], y * np.eye(3)[2]]
    won, first_fail, gaps = 0, None, {}
    for j in range(JMAX + 1):
        H, G = T2.hessian_and_gauge(C2, g, eps, M, _C["rho"][j])
        mu = T2.slice_spectrum(H, G)
        lt = lam(x, y, t, j)
        gaps[j] = float(lt - mu[0])
        if lt <= mu[0] + 1e-9 * max(1.0, abs(mu[0])):
            won += 1
        elif first_fail is None:
            first_fail = j
    return {"x": float(x), "y": float(y), "t": t, "floor_all": won == JMAX + 1, "sectors_won": won,
            "first_fail": first_fail, "gap_j1": gaps[1], "gaps": gaps}


def main():
    import multiprocessing as mp
    out = Path(sys.argv[1]); nproc = int(sys.argv[2]) if len(sys.argv) > 2 else 6
    args = [(float(x), float(y), t) for t in TS for x in XS for y in YS]
    with mp.Pool(nproc) as pool:
        cells = pool.map(cell, args, chunksize=20)
    # marked points
    import ricci_su2_meron_family_soft as MF
    marks = {}
    for t in TS:
        ts = np.round(np.arange(1.0, t + 1e-9, 0.05), 4) if t >= 1 else None
        mer = MF.follow(ts)[-1].reshape(3, 3)
        marks[str(t)] = {"berger": [(2 - t) / np.sqrt(t), np.sqrt(t)], "meron": [float(mer[0, 0]), float(mer[1, 1])]}
    summary = {}
    for t in TS:
        cs = [c for c in cells if c["t"] == t]
        fl = [c for c in cs if c["floor_all"]]
        summary[str(t)] = {"cells": len(cs), "floor_all": len(fl),
                           "floor_with_x_neg": sum(1 for c in fl if c["x"] < 0),
                           "floor_with_x_nonneg": sum(1 for c in fl if c["x"] >= 0),
                           "x_range_of_floor": [min((c["x"] for c in fl), default=None), max((c["x"] for c in fl), default=None)]}
    out.write_text(json.dumps({"summary": summary, "marks": marks, "cells": cells}, indent=1) + "\n")
    print(json.dumps({"summary": summary, "marks": marks}, indent=1))


if __name__ == "__main__":
    main()
