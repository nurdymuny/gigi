"""Numerical sanity check (not a proof) of the coercivity inequality behind condition (A):

    V_src(q) >= (kappa/2)|q|^2 - S^4/(2 g^2),   kappa = min_k(omega_k^2 - src_k),  S = sum(omega),

and of the closed forms kappa_0 = s(2+t-2s), kappa_1 = s[(2+t) - s(1+t)]/t. Adversarial BFGS from 200 starts per
point at the controls and at the extreme registered grid points. Light; runs in seconds.
usage: python scripts/check_helike_discrete_spectrum.py
"""
import math
import numpy as np
from scipy.optimize import minimize
import helike_berger_model as bm

POINTS = [(1.15, 3, .004), (1.15, 3, .002), (1.24, 3, .07), (1.12, 3, .03), (1.18, 4, .05), (1.09, 4, .03)]


def V(q, g, t, a, b):
    om = np.array([math.sqrt(t), 1 / math.sqrt(t), 1 / math.sqrt(t)])
    src = np.array([a, b, b])
    cof = np.array([np.cross(q[1], q[2]), np.cross(q[2], q[0]), np.cross(q[0], q[1])])
    return 0.5 * np.sum((om[:, None] * q - g * cof) ** 2) - 0.5 * np.sum(src[:, None] * q ** 2)


def main():
    rng = np.random.default_rng(1)
    ok = True
    for s, t, g2 in POINTS:
        g = math.sqrt(g2)
        a, b = bm.mc_sources(s, t)
        om = [math.sqrt(t), 1 / math.sqrt(t), 1 / math.sqrt(t)]
        k0, k1 = om[0] ** 2 - a, om[1] ** 2 - b
        closed = abs(k0 - s * (2 + t - 2 * s)) < 1e-12 and abs(k1 - s * ((2 + t) - s * (1 + t)) / t) < 1e-12
        kap, S = min(k0, k1), sum(om)
        C = S ** 4 / (2 * g2)
        slack = lambda x: (V(x.reshape(3, 3), g, t, a, b) - 0.5 * kap * np.sum(x ** 2) + C) / C
        best = min(minimize(slack, rng.normal(size=9) * rng.choice([1, S / g, 3 * S / g, 10 * S / g]), method='BFGS',
                            options={'gtol': 1e-12, 'maxiter': 2000}).fun for _ in range(200))
        ok &= closed and kap > 0 and best >= 0
        print(f"s={s} t={t} g2={g2}: kappa={kap:.5f} closed forms={closed} min relative slack={best:.4f}")
    print('PASS' if ok else 'FAIL')
    return 0 if ok else 1


if __name__ == '__main__':
    raise SystemExit(main())
