"""Bertrand's question on SU(2) itself: under the Haar measure (a uniformly random
unit quaternion), the geodesic angle a has density (2/pi) sin^2 a on [0, pi], so the
sameness S = cos^2(a/2) has the semicircle density (8/pi) sqrt(S (1 - S)) on [0, 1],
and P(chord > sqrt 3) = P(S < 1/4) = 1/3 - sqrt(3)/(4 pi).
Research note: docs/research/unit-curvature-2026-11-18.md, section 7.

usage: python scripts/haar_sameness.py
"""
import numpy as np
import sympy as sp

S = sp.symbols("S", positive=True)
dens = 8 / sp.pi * sp.sqrt(S * (1 - S))
norm = sp.simplify(sp.integrate(dens, (S, 0, 1)))
P = sp.simplify(sp.integrate(dens, (S, 0, sp.Rational(1, 4))))
target = sp.Rational(1, 3) - sp.sqrt(3) / (4 * sp.pi)
print("normalisation:", norm)
print("P(S < 1/4) =", P, "=", float(P), "| equals 1/3 - sqrt3/(4 pi):", sp.simplify(P - target) == 0)

rng = np.random.default_rng(3)
q = rng.normal(size=(4_000_000, 4))
q /= np.linalg.norm(q, axis=1)[:, None]
a = np.arccos(np.clip(q[:, 0], -1, 1))
Sv = np.cos(a / 2) ** 2
h, _ = np.histogram(Sv, bins=20, range=(0, 1), density=True)
c = (np.arange(20) + 0.5) / 20
print("Monte Carlo P(S < 1/4) (4 million Haar turns):", float((Sv < 0.25).mean()))
print("max |histogram - semicircle| over 20 bins:", float(np.abs(h - 8 / np.pi * np.sqrt(c * (1 - c))).max()))
