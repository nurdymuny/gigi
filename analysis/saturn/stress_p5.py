"""Stress-test P5: is "not detected in FQ889N" a measurement, or an inability
to measure?

P5 rests on FQ889N's m=10 phase being random relative to the five wave filters
(rms 12.1 deg vs 10.4 for noise). That conflates two different situations:

  (a) FQ889N sees no m=10 structure         -> phase random, claim stands
  (b) FQ889N cannot measure ANY phase       -> phase random regardless; the
                                               test says nothing about the wave

Distinguish them with the one thing the P4 failure taught: the SPECTRUM, and
the filter's own same-day self-consistency.

  1. SELF-CONSISTENCY. FQ889N phase at -63.3, visit a vs b, each year. A filter
     that can measure phase repeats it to ~1-3 deg across 10 h. If a-vs-b is
     ~10 deg, the filter is blind and P5 is uninformative.
  2. SPECTRUM. m=5..13 in FQ889N at -63.3 each epoch. A wave is a peak at m=10
     standing well above the flat level, whatever the fractional amplitude.
  3. LATITUDE SCAN. The discovery team said the wave's position "shifts with
     wavelength". Look for an m=10 spectral peak anywhere in -70..-56 in
     FQ889N, not only at the deep-filter latitude.
  4. REFERENCE. Same three tests on F631N so the FQ889N numbers mean something.
"""
import glob
import os
import re

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
exec(open(os.path.join(HERE, "phase.py")).read().split("pat = re.compile")[0])

M, PER = 10, 36.0
FLAT = 1.0 / 9.0
WAVE = ["f467m", "f502n", "f631n", "f763m", "fq727n"]


def spectrum(img, lat, halfwidth=1.0):
    col, cov = ring(img, lat, halfwidth)
    good = np.isfinite(col)
    if good.mean() < 0.98:
        return None
    s = col.copy()
    s[~good] = np.nanmean(col[good])
    amp = np.abs(np.fft.rfft(s - s.mean()))[5:14]
    tot = amp.sum()
    return amp / tot if tot > 0 else None


pat = re.compile(r"(\d{4})([ab])_(f[a-z0-9]+|fq[0-9]+n)\.fits$")
maps = {}
for p in sorted(glob.glob(os.path.join(HERE, "*_f*.fits"))):
    if os.path.getsize(p) < 1_000_000:
        continue
    mt = pat.search(os.path.basename(p))
    if mt and int(mt.group(1)) >= 2023:
        maps[(mt.group(1), mt.group(2), mt.group(3))] = read_fits(p)[1]

years = sorted({k[0] for k in maps})

# ---- 1. self-consistency -------------------------------------------------------
print("1. SELF-CONSISTENCY  - phase at -63.3, visit a vs b, same filter")
print(f"  {'year':>5} {'filter':>7} {'phi a':>7} {'phi b':>7} {'b-a':>7}   amp a   amp b")
self_889, self_631 = [], []
for y in years:
    for f, store in (("fq889n", self_889), ("f631n", self_631)):
        ka, kb = (y, "a", f), (y, "b", f)
        if ka in maps and kb in maps:
            ra = zcoef(ring(maps[ka], -63.3)[0], M)
            rb = zcoef(ring(maps[kb], -63.3)[0], M)
            if ra and rb:
                d = wrap(rb[1] - ra[1], PER)
                store.append(d)
                print(f"  {y:>5} {f:>7} {ra[1]:7.1f} {rb[1]:7.1f} {d:+7.1f}   {ra[0]:.4f}  {rb[0]:.4f}")
if self_889:
    print(f"  FQ889N a-vs-b rms: {np.sqrt(np.mean(np.square(self_889))):.1f} deg   "
          f"F631N a-vs-b rms: {np.sqrt(np.mean(np.square(self_631))):.1f} deg   (random ~10.4)")

# ---- 2. spectrum at -63.3 -------------------------------------------------------
print("\n2. SPECTRUM at -63.3  (flat = 0.111; a wave is m=10 well above it)")
print(f"  {'epoch':>6} {'filter':>7}  " + " ".join(f"m{m:<4}" for m in range(5, 14)) + "  m10/flat  peak")
for y in years:
    for v in "ab":
        for f in ("f631n", "fq889n"):
            k = (y, v, f)
            if k not in maps:
                continue
            sp = spectrum(maps[k], -63.3)
            if sp is None:
                continue
            pk = int(np.argmax(sp)) + 5
            print(f"  {y+v:>6} {f:>7}  " + " ".join(f"{x:.2f} " for x in sp)
                  + f"  {sp[5]/FLAT:7.2f}x   m={pk}")

# ---- 3. latitude scan in FQ889N ----------------------------------------------
print("\n3. LATITUDE SCAN  - m10/flat in FQ889N across the cap; any latitude > 2x?")
print(f"  {'epoch':>6}  " + " ".join(f"{lat:>5.0f}" for lat in np.arange(-70, -55, 1.0)))
for y in years:
    for v in "ab":
        k = (y, v, "fq889n")
        if k not in maps:
            continue
        vals = []
        for lat in np.arange(-70, -55, 1.0):
            sp = spectrum(maps[k], lat, halfwidth=0.5)
            vals.append(sp[5] / FLAT if sp is not None else np.nan)
        best = np.nanmax(vals)
        blat = np.arange(-70, -55, 1.0)[int(np.nanargmax(vals))]
        print(f"  {y+v:>6}  " + " ".join(f"{x:5.1f}" if np.isfinite(x) else "   --" for x in vals)
              + f"   best {best:.1f}x at {blat:+.0f}")

print("\n   same scan, F631N, for reference:")
for y in years:
    k = (y, "b", "f631n")
    if k not in maps:
        continue
    vals = []
    for lat in np.arange(-70, -55, 1.0):
        sp = spectrum(maps[k], lat, halfwidth=0.5)
        vals.append(sp[5] / FLAT if sp is not None else np.nan)
    best = np.nanmax(vals)
    blat = np.arange(-70, -55, 1.0)[int(np.nanargmax(vals))]
    print(f"  {y+'b':>6}  " + " ".join(f"{x:5.1f}" if np.isfinite(x) else "   --" for x in vals)
          + f"   best {best:.1f}x at {blat:+.0f}")

# ---- 4. the original P5 test, rerun, for the record -------------------------------
print("\n4. ORIGINAL TEST rerun - FQ889N phase vs wave-filter mean phase at -63.3")
diffs = []
for y in years:
    for v in "ab":
        phs = []
        for f in WAVE:
            k = (y, v, f)
            if k in maps:
                r = zcoef(ring(maps[k], -63.3)[0], M)
                if r:
                    phs.append(r[1])
        k9 = (y, v, "fq889n")
        if len(phs) < 3 or k9 not in maps:
            continue
        ang = np.array(phs) * 2 * np.pi / PER
        ref = np.angle(np.mean(np.exp(1j * ang))) * PER / (2 * np.pi) % PER
        r9 = zcoef(ring(maps[k9], -63.3)[0], M)
        if r9:
            d = wrap(r9[1] - ref, PER)
            diffs.append(d)
            print(f"  {y+v}: wave {ref:5.1f}  FQ889N {r9[1]:5.1f}  diff {d:+6.1f}")
if diffs:
    print(f"  rms {np.sqrt(np.mean(np.square(diffs))):.1f} deg  (random ~10.4)")
