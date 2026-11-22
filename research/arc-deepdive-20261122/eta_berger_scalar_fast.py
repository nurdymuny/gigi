"""Eta invariant of the untwisted odd signature operator (= eta of curl on its non-zero spectrum) on the Berger
sphere (t,1,1), and a control on Hitchin's (1974) explicit Dirac spectrum. COMPUTED. See eta_berger_scalar.py for
the method; this version uses the banded charge-block solver (eta_fastcurl.py) so the cutoff can be much higher.
E(eps) = sum sign(mu) exp(-eps mu^2) is fitted on eps in [EMIN, EMAX] with powers -3/2..3/2; the constant is eta(0).
usage: OMP_NUM_THREADS=1 python eta_berger_scalar_fast.py
"""
import json
import math
from pathlib import Path

import numpy as np

import eta_fastcurl as F

HERE = Path(__file__).resolve().parent
POW = [-1.5, -0.5, 0.0, 0.5, 1.0, 1.5, 2.0]


def eps_grid(scale):
    return np.geomspace(2e-4, 6e-3, 16) * scale


def fit(EPS, E):
    A = np.array([[e ** p for p in POW] for e in EPS])
    coef, *_ = np.linalg.lstsq(A, E, rcond=None)
    return float(coef[POW.index(0.0)]), float(np.max(np.abs(A @ coef - E)))


def dirac(lam2, pmax=2500):
    lam = math.sqrt(lam2)
    EPS = eps_grid(1.0)
    p = np.arange(1, pmax + 1, dtype=float)
    E = np.array([np.sum(2 * p * np.exp(-e * (p / lam + lam / 2) ** 2)) for e in EPS])
    P, Q = np.meshgrid(p, p, indexing="ij")
    X = np.sqrt(4 * P * Q * lam2 + (P - Q) ** 2) / lam
    vp, vm = lam / 2 + X, lam / 2 - X
    for k, e in enumerate(EPS):
        E[k] += np.sum((P + Q) * np.exp(-e * vp * vp)) + np.sum((P + Q) * np.sign(vm) * np.exp(-e * vm * vm))
    return fit(EPS, E)


def curl_eta(t, jmax):
    EPS = eps_grid(1.0)
    E = np.zeros(len(EPS))
    for j in range(jmax + 1):
        mu, _ = F.curl_spectrum(t, j)
        mu = mu[np.abs(mu) > 1e-9]
        sg = np.sign(mu)
        m2 = mu * mu
        E += (j + 1) * np.array([np.sum(sg * np.exp(-e * m2)) for e in EPS])
    return fit(EPS, E)


def main():
    out = {"dirac_control": [], "curl": []}
    for lam2 in (0.5, 2.0, 4.0, 9.0):
        eta, res = dirac(lam2)
        out["dirac_control"].append({"lambda2": lam2, "eta": eta, "hitchin": -(lam2 - 1) ** 2 / 6, "resid": res})
        print("dirac", out["dirac_control"][-1], flush=True)
    for t in (1.0, 0.5, 2.0, 3.0, 4.0, 6.0):
        jmax = 900
        eta, res = curl_eta(t, jmax)
        out["curl"].append({"t": t, "jmax": jmax, "eta": eta, "resid": res,
                            "-(2/3)(t-1)^2": -2 * (t - 1) ** 2 / 3})
        print("curl", out["curl"][-1], flush=True)
        (HERE / "eta_berger_scalar_receipt.json").write_text(json.dumps(out, indent=1) + "\n")


if __name__ == "__main__":
    main()
