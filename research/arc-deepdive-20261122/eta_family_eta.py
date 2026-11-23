"""Eta invariant of the ad-twisted odd signature operator L_A for the circle-symmetric family (x, y) on the Berger
sphere (t,1,1). COMPUTED (heat-regularised, fitted; see eta_berger_scalar_fast.py). Also records, per point, the
twist's curl eigenvalue sign pattern (mu_j = -(x + (j+2)/sqrt t), from eta_family.py) and the Chern-Simons
polynomial W = -t x^2 - 2 y^2 + sqrt(t) x y^2 (CS up to a convention constant; derivation in lit_eta.md).
usage: OMP_NUM_THREADS=1 python eta_family_eta.py
"""
import json
import math
import sys
import time
from pathlib import Path

import numpy as np

import eta_fastcurl as F
from eta_berger_scalar_fast import eps_grid, fit

HERE = Path(__file__).resolve().parent
JMAX = 700


def eta_point(x, y, t):
    m = [np.array([x, 0.0, 0.0]), np.array([0.0, y, 0.0]), np.array([0.0, 0.0, y])]
    EPS = eps_grid(1.0)
    E = np.zeros(len(EPS))
    nzero = 0
    smallest = np.inf
    for j in range(JMAX + 1):
        mu, _ = F.odd_signature_spectrum(t, j, m)
        z = np.abs(mu) < 1e-9
        nzero += int(z.sum()) * (j + 1)
        mu = mu[~z]
        smallest = min(smallest, float(np.min(np.abs(mu))))
        E += (j + 1) * np.array([np.sum(np.sign(mu) * np.exp(-e * mu * mu)) for e in EPS])
    eta, res = fit(EPS, E)
    return eta, res, nzero, smallest


def main():
    pts = []
    t = 3.0
    for y in (0.0, 0.6, 1.2):
        for x in (-1.5, -0.6, 0.6, 1.5):
            pts.append((x, y, t))
    pts += [(2 / math.sqrt(t) * s, 2 * s, t) for s in (0.0, 0.5, 1.0)]   # A = s theta; s=0 trivial, s=1 flat
    out = {"jmax": JMAX, "points": []}
    for (x, y, t) in pts:
        t0 = time.time()
        eta, res, nz, sm = eta_point(x, y, t)
        W = -t * x * x - 2 * y * y + math.sqrt(t) * x * y * y
        row = {"x": x, "y": y, "t": t, "eta": eta, "fit_resid": res, "zero_modes": nz, "min_abs_nonzero": sm,
               "W": W, "secs": time.time() - t0}
        out["points"].append(row)
        print(row, flush=True)
        (HERE / "eta_family_eta_receipt.json").write_text(json.dumps(out, indent=1) + "\n")


if __name__ == "__main__":
    main()
