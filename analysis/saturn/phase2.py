"""Two puzzles, one test: phase is the fingerprint of a single wave.

PUZZLE 1. In 2023 the m=10 envelope peaked at -69 deg in all five wave-carrying
filters; in 2024 and 2025 it peaked at -63. Did the wave migrate 6 deg toward
the equator, or were there two features and one won? If the -69 and -63 signals
in 2023 share a phase, they are one broad wave. If not, they are two.

PUZZLE 2. FQ889N (strong methane, stratosphere) shows a high fractional m=10
amplitude but at a wandering latitude with low contrast. Earlier I called the
wave "absent" there. If the FQ889N m=10 phase matches the other filters, the
wave reaches the stratosphere and that claim is wrong. If it is random, the
signal there is noise and "not detected" is the honest statement.
"""
import glob
import os
import re

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
exec(open(os.path.join(HERE, "phase.py")).read().split("pat = re.compile")[0])

M = 10
PER = 36.0

pat = re.compile(r"(\d{4})([ab])_(f[a-z0-9]+|fq[0-9]+n)\.fits$")
files = {}
for p in sorted(glob.glob(os.path.join(HERE, "*_f*.fits"))):
    if os.path.getsize(p) < 1_000_000:
        continue
    mt = pat.search(os.path.basename(p))
    if mt and int(mt.group(1)) >= 2023:
        files[(mt.group(1) + mt.group(2), mt.group(3))] = p

WAVE = ["f467m", "f502n", "f631n", "f763m", "fq727n"]

print("PUZZLE 1 - the 2023 latitude question")
print(f"{'epoch':>6} {'filter':>7}   {'phi0 @ -69':>11} {'amp':>8}   {'phi0 @ -63':>11} {'amp':>8}   diff")
print("-" * 76)
for ep in ["2023a", "2023b", "2024a", "2025a"]:
    for f in WAVE:
        p = files.get((ep, f))
        if not p:
            continue
        _, img = read_fits(p)
        r69 = zcoef(ring(img, -69.0)[0], M)
        r63 = zcoef(ring(img, -63.0)[0], M)
        if r69 and r63:
            d = wrap(r63[1] - r69[1], PER)
            print(f"{ep:>6} {f:>7}   {r69[1]:11.1f} {r69[0]:8.4f}   {r63[1]:11.1f} {r63[0]:8.4f}   {d:+6.1f}")

print("\n  latitude profile of amplitude AND phase, 2023b F631N (the discovery band):")
_, img = read_fits(files[("2023b", "f631n")])
print(f"  {'lat':>6} {'amp x1000':>10} {'phi0':>7}")
prev = None
for lat in np.arange(-72, -55, 1.0):
    r = zcoef(ring(img, lat, halfwidth=0.5)[0], M)
    if r:
        tag = ""
        if prev is not None:
            tag = f"   ({wrap(r[1]-prev, PER):+5.1f} from row above)"
        print(f"  {lat:+6.0f} {1000*r[0]:10.2f} {r[1]:7.1f}{tag}")
        prev = r[1]

print("\n\nPUZZLE 2 - does the wave reach FQ889N (stratosphere)?")
print(f"{'epoch':>6}   {'wave-filter mean phi0':>22}   {'FQ889N phi0 @ same lat':>22}   diff    FQ889N amp")
print("-" * 90)
diffs = []
for ep in sorted({k[0] for k in files}):
    phs = []
    for f in WAVE:
        p = files.get((ep, f))
        if p:
            r = zcoef(ring(read_fits(p)[1], -63.3)[0], M)
            if r:
                phs.append(r[1])
    p9 = files.get((ep, "fq889n"))
    if not phs or not p9:
        continue
    ang = np.array(phs) * 2 * np.pi / PER
    ref = np.angle(np.mean(np.exp(1j * ang))) * PER / (2 * np.pi) % PER
    r9 = zcoef(ring(read_fits(p9)[1], -63.3)[0], M)
    if r9:
        d = wrap(r9[1] - ref, PER)
        diffs.append(d)
        print(f"{ep:>6}   {ref:22.1f}   {r9[1]:22.1f}   {d:+6.1f}    {r9[0]:.4f}")
if diffs:
    rms = np.sqrt(np.mean(np.square(diffs)))
    # for a random phase on a 36-deg circle, rms of the wrapped difference ~ 36/sqrt(12) = 10.4
    print(f"\n  rms phase difference FQ889N vs wave filters: {rms:.1f} deg")
    print(f"  same-wave expectation: ~1-2 deg.   random expectation: ~{PER/np.sqrt(12):.1f} deg.")
