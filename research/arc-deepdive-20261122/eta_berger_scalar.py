"""Numerical eta invariant of the (untwisted) odd signature operator on the Berger sphere (t,1,1), to pin the
normalisation of the quoted formulas (Hitchin 1974 for Dirac; Bakas-Lust 2011 / Hitchin 1997 for the signature
operator, where a factor-2 discrepancy is reported). Research script for lit_eta.md. COMPUTED.

eta(B_ev) = eta(curl restricted to its non-zero spectrum) on 1-forms (exact forms and functions pair into +-
eigenvalues). Sector j (spin j/2) contributes its 3(j+1) curl eigenvalues with right multiplicity (j+1).
Regularisation: E(eps) = sum sign(mu) exp(-eps mu^2) -> eta(0) as eps -> 0; fitted with the terms
eps^{-3/2}, eps^{-1/2}, 1, eps^{1/2}, eps, eps^{3/2} on a grid of eps (the local terms are fitted, not assumed zero).
Control: the same machinery on Hitchin's explicit Dirac spectrum (Proposition 3.2, metric (lambda^2,1,1)),
compared with eta_D = -(lambda^2-1)^2/6 (lambda^2 < 16).
usage: OMP_NUM_THREADS=1 python eta_berger_scalar.py
"""
import json
import math
import sys
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[1] / "scripts"))
import ricci_su2_spin_sector as SS  # noqa: E402

STAR = {(0, 1): (2, 1.0), (1, 2): (0, 1.0), (0, 2): (1, -1.0)}
EPS = np.geomspace(0.004, 0.08, 14)
POW = [-1.5, -0.5, 0.0, 0.5, 1.0, 1.5]


def scalar_curl(t, j):
    s = np.sqrt(np.array([t, 1.0, 1.0]))
    c = np.zeros((3, 3, 3))
    for (i, jj, k), v in SS.BR.items():
        c[i, jj, k] = v * s[k] / (s[i] * s[jj])
    rho = SS.spin_rep(j)
    d = j + 1
    idx = lambda i: slice(i * d, (i + 1) * d)  # noqa: E731
    curl = np.zeros((3 * d, 3 * d), complex)
    for (i, jj), (k, sg) in STAR.items():
        D = np.zeros((d, 3 * d), complex)
        D[:, idx(jj)] += rho[i] / s[i]
        D[:, idx(i)] -= rho[jj] / s[jj]
        for kk in range(3):
            D[:, idx(kk)] -= c[i, jj, kk] * np.eye(d)
        curl[idx(k), :] += sg * D
    assert np.allclose(curl, curl.conj().T)
    return np.linalg.eigvalsh(curl)


def fit_eta(E):
    A = np.array([[e ** p for p in POW] for e in EPS])
    coef, *_ = np.linalg.lstsq(A, E, rcond=None)
    resid = float(np.max(np.abs(A @ coef - E)))
    return float(coef[POW.index(0.0)]), {str(p): float(cf) for p, cf in zip(POW, coef)}, resid


def eta_curl(t, jmax):
    E = np.zeros(len(EPS))
    for j in range(jmax + 1):
        mu = scalar_curl(t, j)
        mu = mu[np.abs(mu) > 1e-9]
        E += (j + 1) * np.array([np.sum(np.sign(mu) * np.exp(-e * mu * mu)) for e in EPS])
    return fit_eta(E)


def eta_dirac_hitchin(lam, pmax):
    E = np.zeros(len(EPS))
    for p in range(1, pmax + 1):
        v = p / lam + lam / 2
        E += 2 * p * np.exp(-EPS * v * v)
        for q in range(1, pmax + 1):
            X = math.sqrt(4 * p * q * lam * lam + (p - q) ** 2) / lam
            vp, vm = lam / 2 + X, lam / 2 - X
            E += (p + q) * np.exp(-EPS * vp * vp)
            if abs(vm) > 1e-12:
                E += (p + q) * np.sign(vm) * np.exp(-EPS * vm * vm)
    return fit_eta(E)


def main():
    out = {"eps": list(EPS), "powers": POW, "dirac_control": [], "curl": []}
    for lam2 in (0.5, 2.0, 4.0, 9.0):
        lam = math.sqrt(lam2)
        eta, coef, res = eta_dirac_hitchin(lam, 700)
        ref = -(lam2 - 1) ** 2 / 6
        out["dirac_control"].append({"lambda2": lam2, "eta_fit": eta, "hitchin": ref, "fit_resid": res})
        print(f"Dirac lambda^2={lam2}: fit {eta:.6f}  Hitchin {ref:.6f}  resid {res:.1e}", flush=True)
    for t in (1.0, 0.5, 2.0, 3.0, 4.0):
        jmax = int(min(420, 60 * math.sqrt(max(t, 1)) + 140))
        eta, coef, res = eta_curl(t, jmax)
        out["curl"].append({"t": t, "jmax": jmax, "eta_fit": eta, "coef": coef, "fit_resid": res,
                            "minus_2_3_(t-1)^2": -2 * (t - 1) ** 2 / 3, "minus_1_3_(t-1)^2": -(t - 1) ** 2 / 3})
        print(f"curl t={t} jmax={jmax}: eta {eta:.6f}; -(2/3)(t-1)^2={-2*(t-1)**2/3:.6f}; "
              f"-(1/3)(t-1)^2={-(t-1)**2/3:.6f}; resid {res:.1e}", flush=True)
        (HERE / "eta_berger_scalar_receipt.json").write_text(json.dumps(out, indent=1) + "\n")


if __name__ == "__main__":
    main()
