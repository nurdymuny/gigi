"""Checks behind 'her shadow from two sectors down' (22 November 2026).
At A = theta the floor of sector j (j = 12..40 surviving on S^3/2I) equals the floor at A = 0 of sector j - 2, which
for t >= 2.75 is her own price there, (j-2+2)^2/t = j^2/t. At A = 0 the floor of every surviving sector is her price
for t in 2.75..10. Uses the Floor paper's charge-block builder (H + GG^*).
usage (repository root): python docs/research/basin-2026-11-22/poincare_ghost_check.py
"""
import sys
from pathlib import Path
import numpy as np
ROOT = Path(__file__).resolve().parents[3]; sys.path.insert(0, str(ROOT / "scripts"))
_a, sys.argv = sys.argv, [sys.argv[0]]
import ricci_su2_exact_edge as EE  # noqa: E402
sys.argv = _a
B = EE.Blocks(); worst = 0
for tv in [2.75, 3.0, 3.55, 5.0, 7.0, 10.0]:
    for j in [12, 20, 24, 30, 32, 36, 40]:
        th = B.sector_spectrum(2 / np.sqrt(tv), 2.0, tv, j).min(); z = B.sector_spectrum(0.0, 0.0, tv, j - 2).min()
        worst = max(worst, abs(th - j * j / tv), abs(th - z))
print("max |floor(theta, j) - j^2/t|, |floor(theta, j) - floor(0, j-2)|, t in 2.75..10, j = 12..40:", worst)
print("t=1: max |floor(theta, j) - floor(0, j-2)| =",
      max(abs(B.sector_spectrum(2.0, 2.0, 1.0, j).min() - B.sector_spectrum(0, 0, 1.0, j - 2).min()) for j in [12, 20, 24, 30]))
w = max(abs(B.sector_spectrum(0, 0, tv, j).min() - EE.lam_tw(0, 0, tv, j))
        for tv in [2.75, 3.0, 3.55, 5.0, 7.0, 10.0] for j in [0, 12, 20, 24, 30, 32, 36, 40])
print("max |floor(0, j) - her price| over surviving j <= 40, t in 2.75..10:", w)
for tv in [1.0, 1.5, 2.0, 2.25, 2.5, 2.75]:
    print(f" t={tv}: A=0 sector 12 floor {B.sector_spectrum(0, 0, tv, 12).min():.4f}, her price {EE.lam_tw(0, 0, tv, 12):.4f}")
print("sector 0 floor at theta:", [(tv, round(float(B.sector_spectrum(2/np.sqrt(tv), 2.0, tv, 0).min()), 6)) for tv in [2.75, 3.0, 3.55, 5.0, 7.0, 10.0]])
