"""zeta-regularised zero-point differences for M3 (bundle-framing §14), from slice spectra written by `helike zero-point`.

For two fields A (reference) and B, with fast spectra mu (sectors j >= jmin, multiplicity j+1):
    dK(tau) = K_B(tau) - K_A(tau),   K(tau) = sum mult exp(-tau mu)
small-tau expansion dK ~ sum_k c_k tau^k, k in {-3/2, -1/2, 0, 1/2, 1, 3/2, 2, 5/2}. Facts used and checked:
    c_{-3/2} = 0 (same number of modes per volume), c_{1/2} = 2 sqrt(pi) dC with dC the coefficient of log eps in
    1/2 sum omega exp(-eps omega): the running coupling, dC = (b0 / 8 pi^2) Vol d(density), b0 = 22/3.
zeta finite part at mu R = 1 (R the sphere radius):
    dE0 = 1/2 FP_{s=-1/2} dzeta_H(s),
    Gamma(s) dzeta_H(s) = sum_k c_k tau0^{s+k}/(s+k) + int_0^tau0 tau^{s-1} (dK - fit) + sum_modes (+-) mu^{-s} Gamma(s, tau0 mu).
The pole c_{1/2}/(s+1/2) is dropped (it is the log divergence, absorbed in 1/g^2); the scheme freedom is dE0 -> dE0 + kappa dC."""
import math

import numpy as np
from scipy.special import digamma, erfc

POWERS = [-1.5, -0.5, 0.0, 0.5, 1.0, 1.5, 2.0, 2.5]
TAU_FIT = np.geomspace(0.003, 0.03, 48)


def modes(point, jmin=1):
    mu, mult = [], []
    for s in point["sectors"]:
        if s["j"] >= jmin:
            m = np.array(s["slice"]); mu.append(m); mult.append(np.full(len(m), s["mult"], dtype=float))
    return np.concatenate(mu), np.concatenate(mult)


def heat(mu, mult, tau):
    return (mult[None, :] * np.exp(-np.outer(tau, mu))).sum(axis=1)


def gamma_m_half(x):
    """Gamma(-1/2, x) = 2 e^-x / sqrt x - 2 sqrt(pi) erfc(sqrt x)."""
    r = np.sqrt(x)
    return 2 * np.exp(-x) / r - 2 * math.sqrt(math.pi) * erfc(r)


def coeffs(ref, pt, jmin=1, tau=None):
    tau = TAU_FIT if tau is None else tau
    (ma, wa), (mb, wb) = modes(ref, jmin), modes(pt, jmin)
    dK = heat(mb, wb, tau) - heat(ma, wa, tau)
    X = np.stack([tau ** k for k in POWERS], axis=1)
    c, *_ = np.linalg.lstsq(X, dK, rcond=None)
    return dict(zip(POWERS, map(float, c))), float(np.max(np.abs(X @ c - dK))), float(np.max(np.abs(dK)))


def zeta_diff(ref, pt, jmin=1, tau0=0.02, n_quad=400):
    """dE0 (zeta, mu R = 1) and the log coefficient dC, for B = pt relative to A = ref."""
    (ma, wa), (mb, wb) = modes(ref, jmin), modes(pt, jmin)
    if min(ma.min(), mb.min()) <= 0:
        raise ValueError("non-positive fast mode: the fast sector is not a harmonic fibre here")
    c, resid, scale = coeffs(ref, pt, jmin)
    s = -0.5
    reg = sum(ck * tau0 ** (s + k) / (s + k) for k, ck in c.items() if k != 0.5) + c[0.5] * math.log(tau0)
    # remainder on [TAU_FIT[0], tau0] from exact sums; below TAU_FIT[0] the fit is used (remainder ~ fit residual)
    tq = np.geomspace(TAU_FIT[0], tau0, n_quad)
    rem = heat(mb, wb, tq) - heat(ma, wa, tq) - sum(ck * tq ** k for k, ck in c.items())
    integrand = tq ** (s - 1) * rem
    reg += float(np.trapz(integrand, tq))
    reg += float((wb * mb ** 0.5 * gamma_m_half(tau0 * mb)).sum() - (wa * ma ** 0.5 * gamma_m_half(tau0 * ma)).sum())
    g = math.gamma(s)
    fp = reg / g - c[0.5] * digamma(s) / g
    return {"dE0": 0.5 * fp, "dC": c[0.5] / (2 * math.sqrt(math.pi)), "c": c, "fit_resid": resid, "dK_scale": scale}


def lower_cont(mu, tau0, s=-0.5, nmax=60):
    """mu^-s gamma(s, tau0 mu) continued in s: tau0^s sum_n (-tau0 mu)^n / (n! (s+n)); entire in mu (negative mu fine)."""
    x = -tau0 * np.asarray(mu, dtype=float)
    term = np.ones_like(x); tot = term / s
    for n in range(1, nmax):
        term = term * x / n
        tot = tot + term / (s + n)
    return tau0 ** s * tot


def zeta_fast(ref, pt, tau0=0.02, n_quad=400):
    """dE0 of the fast fibre (sectors j >= 1) at mu R = 1, from a fit of the TOTAL heat-trace difference (whose
    expansion is clean) minus the finite slow set (j = 0) removed exactly by the continued lower gamma.
    Returns dE0_fast, dE0_slow_harmonic (1/2 sum sqrt mu over positive slow modes, for reference) and dC."""
    (ma, wa), (mb, wb) = modes(ref, 0), modes(pt, 0)
    (fa, va), (fb, vb) = modes(ref, 1), modes(pt, 1)
    sa = np.concatenate([np.array(s["slice"]) for s in ref["sectors"] if s["j"] == 0])
    sb = np.concatenate([np.array(s["slice"]) for s in pt["sectors"] if s["j"] == 0])
    if min(fa.min(), fb.min()) <= 0:
        raise ValueError("non-positive fast mode: the fast sector is not a harmonic fibre here")
    c, resid, scale = coeffs(ref, pt, 0)
    s = -0.5
    reg = sum(ck * tau0 ** (s + k) / (s + k) for k, ck in c.items() if k != 0.5) + c[0.5] * math.log(tau0)
    reg -= float(lower_cont(sb, tau0).sum() - lower_cont(sa, tau0).sum())
    tq = np.geomspace(TAU_FIT[0], tau0, n_quad)
    rem = heat(mb, wb, tq) - heat(ma, wa, tq) - sum(ck * tq ** k for k, ck in c.items())
    reg += float(np.trapz(tq ** (s - 1) * rem, tq))
    reg += float((vb * fb ** 0.5 * gamma_m_half(tau0 * fb)).sum() - (va * fa ** 0.5 * gamma_m_half(tau0 * fa)).sum())
    g = math.gamma(s)
    fp = reg / g - c[0.5] * digamma(s) / g
    slow = lambda m: 0.5 * float(np.sqrt(m[m > 0]).sum())
    return {"dE0_fast": 0.5 * fp, "dC": c[0.5] / (2 * math.sqrt(math.pi)), "c": c, "fit_resid": resid, "dK_scale": scale,
            "slow_harmonic_diff": slow(sb) - slow(sa), "slow_negative": [float(m) for m in sb if m < 0]}
