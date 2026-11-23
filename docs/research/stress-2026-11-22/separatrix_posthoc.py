"""POST HOC (after the S run): how far the Berger field sits from the basin boundary.
At x = x_B, bisect in y for the boundary between landing on A = 0 (below) and A = theta (above).
usage (repository root): python docs/research/stress-2026-11-22/separatrix_posthoc.py
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import stress_tests as S  # noqa: E402

for t in (2.75, 3.0, 3.55, 5.0, 7.0, 10.0):
    xb, yb = S.berger(t)
    lo, hi = 1e-3, 3 * yb
    assert S.landing(S.flow(xb, lo, t), t) == "A=0" and S.landing(S.flow(xb, hi, t), t) == "A=theta"
    for _ in range(40):
        mid = 0.5 * (lo + hi)
        if S.landing(S.flow(xb, mid, t), t) == "A=0":
            lo = mid
        else:
            hi = mid
    yc = 0.5 * (lo + hi)
    print(f"t={t:5}: Berger y = {yb:.6f}, boundary at y = {yc:.6f}, margin {yb - yc:+.6f} ({(yb - yc) / yb:+.2%} of y)")
