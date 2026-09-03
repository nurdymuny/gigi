"""Stress-test P2 (System III lock) with everything that could break it.

Three attacks on the -0.0141 deg/day drift:

  1. ALIAS LANDSCAPE. Phase is only known mod 36 deg, so any drift d that
     satisfies d*dt = 0 (mod 36) for the inter-epoch gaps is indistinguishable
     from zero. I asserted only the smallest alias fits both gaps. Verify it
     numerically: sweep d over a wide grid and show every minimum.

  2. BOOTSTRAP. Resample the (filter, visit) measurements within each epoch,
     refit, and get a real error bar on the drift rather than an estimate.

  3. LEAVE-ONE-OUT. 2023 had poor southern geometry and the two-lobe confusion.
     Refit on 2024->2025 alone, and on 2023->2025 alone. If the drift moves
     outside its error bar, the fit is being carried by a fragile epoch.

Then the prediction is re-issued with whichever uncertainty is honest.
"""
import glob
import os
import re

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
exec(open(os.path.join(HERE, "phase.py")).read().split("pat = re.compile")[0])

M, PER = 10, 36.0
WAVE = ["f467m", "f502n", "f631n", "f763m", "fq727n"]
RNG = np.random.default_rng(20260903)

pat = re.compile(r"(\d{4})([ab])_(f[a-z0-9]+|fq[0-9]+n)\.fits$")
meas = []   # (year, mjd, phi0)
for p in sorted(glob.glob(os.path.join(HERE, "*_f*.fits"))):
    if os.path.getsize(p) < 1_000_000:
        continue
    mt = pat.search(os.path.basename(p))
    if not mt or int(mt.group(1)) < 2023 or mt.group(3) not in WAVE:
        continue
    hdr, img = read_fits(p)
    r = zcoef(ring(img, -63.0)[0], M)
    if r and r[0] >= 0.002:
        meas.append((mt.group(1), mjd(hdr), r[1]))

years = sorted({m[0] for m in meas})


def circ_mean(phis):
    a = np.asarray(phis) * 2 * np.pi / PER
    return (np.angle(np.mean(np.exp(1j * a))) * PER / (2 * np.pi)) % PER


def epoch_means(rows):
    out = {}
    for y in years:
        sub = [r for r in rows if r[0] == y]
        if sub:
            out[y] = (circ_mean([r[2] for r in sub]), np.mean([r[1] for r in sub]))
    return out


def fit_drift(em, use_years=None):
    ys = [y for y in sorted(em) if (use_years is None or y in use_years)]
    if len(ys) < 2:
        return np.nan, np.nan
    t0 = em[ys[0]][1]
    p0 = em[ys[0]][0]
    dt = np.array([em[y][1] - t0 for y in ys])
    dp = np.array([wrap(em[y][0] - p0, PER) for y in ys])
    slope = np.sum(dp * dt) / np.sum(dt * dt)
    resid = dp - slope * dt
    return slope, np.sqrt(np.mean(resid ** 2))


em = epoch_means(meas)
base_slope, base_rms = fit_drift(em)
print("BASELINE")
for y in years:
    print(f"  {y}: phi0 {em[y][0]:6.1f}  MJD {em[y][1]:.1f}  n={sum(1 for m in meas if m[0]==y)}")
print(f"  drift {base_slope:+.4f} deg/day   residual {base_rms:.2f} deg\n")

# ---- 1. alias landscape ------------------------------------------------------
print("1. ALIAS LANDSCAPE   (residual rms vs assumed drift; minima are the aliases)")
ys = sorted(em)
t0, p0 = em[ys[0]][1], em[ys[0]][0]
dts = np.array([em[y][1] - t0 for y in ys])
dps = np.array([em[y][0] for y in ys])
# Outside review (2026-09-03) found two faults in the first version of this
# block: the sweep stopped at +-0.6 deg/day and could not see a minimum at the
# grid edge (there is one at -0.5999), and the printed verdict used a 2.0 deg
# cut while the text claimed 1.5, letting +0.355 (1.71 deg) through. Fixed:
# +-1 deg/day, endpoints included, criterion = 1.5 deg = the a/b noise floor
# plus margin.
CRIT = 1.5
grid = np.arange(-1.00, 1.00 + 1e-9, 0.0001)
land = np.array([np.sqrt(np.mean([wrap(dps[i] - (p0 + d * dts[i]), PER) ** 2
                                  for i in range(len(ys))])) for d in grid])
idx = [i for i in range(1, len(grid) - 1) if land[i] < land[i - 1] and land[i] < land[i + 1]]
if land[0] < land[1]:
    idx.append(0)
if land[-1] < land[-2]:
    idx.append(len(grid) - 1)
mins = sorted(((grid[i], land[i]) for i in idx), key=lambda x: x[1])
print(f"  {'drift deg/day':>14} {'residual deg':>13}   verdict")
for d, r in mins[:12]:
    tag = f"  <-- fits within {CRIT} deg" if r <= CRIT else ""
    if abs(d - 0.42) < 0.06:
        tag += "   [published 2.5 m/s eastward = +0.39..+0.42 in this convention]"
    print(f"  {d:+14.4f} {r:13.3f}{tag}")
n_fit = sum(1 for _, r in mins if r <= CRIT)
print(f"  aliases fitting within {CRIT} deg: {n_fit}  over |d| <= 1.0 deg/day")
print(f"  (at 63S the latitude circle is ~184,000 km, so 0.46 deg/day is only 2.7 m/s;\n"
      f"   Rossby phase speeds of several m/s are ordinary. Every alias above is\n"
      f"   physically allowed, and a wider sweep would find more. The data cannot\n"
      f"   choose among them.)\n")

# ---- 2. bootstrap -------------------------------------------------------------
print("2. BOOTSTRAP   (resample (filter,visit) within each epoch, 4000 draws)")
slopes = []
for _ in range(4000):
    rows = []
    for y in years:
        sub = [m for m in meas if m[0] == y]
        idx = RNG.integers(0, len(sub), len(sub))
        rows.extend(sub[i] for i in idx)
    s, _ = fit_drift(epoch_means(rows))
    if np.isfinite(s):
        slopes.append(s)
slopes = np.array(slopes)
lo, hi = np.percentile(slopes, [2.5, 97.5])
print(f"  drift = {np.median(slopes):+.4f}  95% CI [{lo:+.4f}, {hi:+.4f}] deg/day")
print(f"  hexagon literature -0.013 inside CI: {lo <= -0.013 <= hi}")
print(f"  zero inside CI:                       {lo <= 0.0 <= hi}\n")

# ---- 3. leave-one-out ---------------------------------------------------------
print("3. LEAVE-ONE-OUT")
for drop in years:
    keep = [y for y in years if y != drop]
    s, r = fit_drift(em, keep)
    flag = "" if lo <= s <= hi else "   <-- OUTSIDE bootstrap CI"
    print(f"  drop {drop}: fit on {'+'.join(keep)}  ->  {s:+.4f} deg/day  (resid {r:.2f}){flag}")

# ---- re-issued prediction ------------------------------------------------------
print("\nRE-ISSUED P2")
t_next = 61277.0
t_last = em[ys[-1]][1]
p_last = em[ys[-1]][0]
pred = (p_last + np.median(slopes) * (t_next - t_last)) % PER
half = 1.96 * np.std(slopes) * (t_next - t_last)
noise = 1.09
err = np.sqrt(half ** 2 + noise ** 2)
print(f"  phi0(MJD {t_next:.0f}) = {pred:.1f} deg  +- {err:.1f}  (mod 36)")
print(f"     drift term  +-{half:.1f}   measurement floor +-{noise:.1f}")
print(f"  if instead the wave were freely drifting, a 2*{err:.1f} = {2*err:.1f} deg window")
print(f"  is hit by chance with probability {2*err/PER:.0%}.")
