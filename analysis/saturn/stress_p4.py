"""Stress-test P4: was the -69 deg lobe of October 2023 a wave, or the limb?

P4 says the m=10 signal had two lobes in 2023 (-69 strong, -63 weak), the
poleward one collapsed within a year, and the decagon was therefore SELECTED
at 63S rather than born there. The caveat: in Oct 2023 the sub-Earth latitude
was near +9, so -69 sat at ~78 deg emission angle, near the limb.

Three geometric attacks:

  1. MOSAIC. Each map is stitched from six HST visits. The a and b maps are
     ~10 h apart at different central meridians, so their seams fall in
     different places. A seam artifact changes phase between a and b; a wave
     on the planet does not. Compare phi0 at -69 per filter, a vs b, 2023.

  2. EMISSION ANGLE. If the pipeline manufactures m=10 at high emission
     angle, it should do so at whatever latitude sits at that angle in every
     epoch - roughly -73 in 2024 and -79 in 2025 - not at a fixed -69. Scan
     the whole southern cap each year for coherent m=10 and tag each hit with
     its emission angle. Sub-Earth latitude B is derived from each map's own
     coverage boundary, not assumed.

  3. SPECTRUM. A limb or seam artifact is not wavenumber-specific. Compute the
     full m=5..13 spectrum at -69 in 2023 and see whether m=10 stands out.
"""
import glob
import os
import re

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
exec(open(os.path.join(HERE, "phase.py")).read().split("pat = re.compile")[0])

M, PER = 10, 36.0
WAVE = ["f467m", "f502n", "f631n", "f763m", "fq727n"]


def sub_earth_lat(img):
    """B from the coverage boundary: the last latitude with >50% longitude coverage."""
    ny, nx = img.shape
    cov = (np.isfinite(img) & (img != 0)).sum(axis=1) / nx
    rows = np.where(cov > 0.5)[0]
    dl = 180.0 / ny
    lat_n = 90.0 - (rows.min() + 0.5) * dl
    lat_s = 90.0 - (rows.max() + 0.5) * dl
    # the pole tilted toward Earth is the one seen deeper past 90-|B|
    b_from_n = 90.0 - lat_n      # if north tilted away, lat_n = 90 - B  -> B = 90 - lat_n (B>0 means... )
    b_from_s = -(90.0 + lat_s)   # lat_s = -(90 - |B|) when south tilted away
    # whichever pole is cut off tells us the sign: visible down to -87 => B ~ -3
    if abs(lat_n) < abs(lat_s):  # north cut off more -> north tilted away -> B < 0
        return -(90.0 - abs(lat_n))
    return 90.0 - abs(lat_s)


def emission_deg(lat, B):
    """Emission angle at the central meridian for planetographic latitude lat."""
    return abs(lat - B)


def cap_scan(img, m, lo=-86.0, hi=-54.0, step=1.0):
    out = []
    for lat in np.arange(lo, hi + 0.01, step):
        r = zcoef(ring(img, lat, halfwidth=0.5)[0], m)
        if r:
            out.append((lat, r[0], r[1]))
    return out


pat = re.compile(r"(\d{4})([ab])_(f[a-z0-9]+|fq[0-9]+n)\.fits$")
maps = {}
for p in sorted(glob.glob(os.path.join(HERE, "*_f*.fits"))):
    if os.path.getsize(p) < 1_000_000:
        continue
    mt = pat.search(os.path.basename(p))
    if mt and int(mt.group(1)) >= 2023 and mt.group(3) in WAVE:
        maps[(mt.group(1) + mt.group(2), mt.group(3))] = read_fits(p)[1]

epochs = sorted({k[0] for k in maps})

print("SUB-EARTH LATITUDE B, derived from each map's coverage boundary")
B = {}
for ep in epochs:
    vals = [sub_earth_lat(maps[(ep, f)]) for f in WAVE if (ep, f) in maps]
    B[ep] = float(np.median(vals))
    print(f"  {ep}: B = {B[ep]:+.1f}   ->  emission angle at -63: {emission_deg(-63, B[ep]):.0f}   at -69: {emission_deg(-69, B[ep]):.0f}")

# ---- 1. mosaic test ----------------------------------------------------------
print("\n1. MOSAIC TEST  - phi0 at -69 in 2023, visit a vs visit b, per filter")
print(f"  {'filter':>7} {'a':>7} {'b':>7} {'b-a':>7}   amp a    amp b")
diffs = []
for f in WAVE:
    ra = zcoef(ring(maps[("2023a", f)], -69.0)[0], M) if ("2023a", f) in maps else None
    rb = zcoef(ring(maps[("2023b", f)], -69.0)[0], M) if ("2023b", f) in maps else None
    if ra and rb:
        d = wrap(rb[1] - ra[1], PER)
        diffs.append(d)
        print(f"  {f:>7} {ra[1]:7.1f} {rb[1]:7.1f} {d:+7.1f}   {ra[0]:.4f}   {rb[0]:.4f}")
if diffs:
    print(f"  rms a-b difference at -69: {np.sqrt(np.mean(np.square(diffs))):.1f} deg")
    print(f"  same test at -63 (the known wave) gave 1.09 rms; random would give ~10.4.")

# ---- 2. emission-angle test --------------------------------------------------
print("\n2. EMISSION-ANGLE TEST - every coherent m=10 in the southern cap, by epoch")
print("   coherent = cross-filter phase sd < 5 deg AND amplitude > 2x cap median")
print(f"  {'epoch':>6} {'lat':>6} {'emis':>5} {'amp x1e3':>9} {'phase sd':>9}   note")
for ep in epochs:
    scans = {f: cap_scan(maps[(ep, f)], M) for f in WAVE if (ep, f) in maps}
    if len(scans) < 3:
        continue
    lats = sorted(set.intersection(*[set(l for l, _, _ in s) for s in scans.values()]))
    med_amp = np.median([a for s in scans.values() for _, a, _ in s])
    hits = []
    for lat in lats:
        amps = [next(a for l, a, _ in scans[f] if l == lat) for f in scans]
        phs = [next(p for l, _, p in scans[f] if l == lat) for f in scans]
        ang = np.array(phs) * 2 * np.pi / PER
        R = abs(np.mean(np.exp(1j * ang)))
        sd = np.sqrt(-2 * np.log(max(R, 1e-12))) * PER / (2 * np.pi)
        if sd < 5.0 and np.mean(amps) > 2 * med_amp:
            hits.append((lat, emission_deg(lat, B[ep]), 1000 * np.mean(amps), sd))
    if not hits:
        print(f"  {ep:>6}   (no coherent m=10 anywhere in the cap)")
        continue
    # merge adjacent latitudes into bands
    bands, cur = [], [hits[0]]
    for h in hits[1:]:
        if h[0] - cur[-1][0] <= 1.01:
            cur.append(h)
        else:
            bands.append(cur)
            cur = [h]
    bands.append(cur)
    for bnd in bands:
        lat_c = np.mean([h[0] for h in bnd])
        e = emission_deg(lat_c, B[ep])
        amp = max(h[2] for h in bnd)
        sd = np.mean([h[3] for h in bnd])
        note = ""
        if e >= 75:
            note = "<- high emission angle"
        if abs(lat_c + 69) <= 1.5:
            note += "  [the -69 lobe]"
        if abs(lat_c + 63) <= 1.5:
            note += "  [the decagon]"
        print(f"  {ep:>6} {lat_c:+6.1f} {e:5.0f} {amp:9.2f} {sd:9.2f}   {note}")

print("\n   If the -69 signal were a high-emission-angle artifact, 2024 and 2025 should")
print("   show a coherent m=10 at whatever latitude sits at ~78 deg emission angle")
print(f"   (about {B['2024a']-78:+.0f} in 2024, {B['2025a']-78:+.0f} in 2025), and nothing at a fixed -69.")

# ---- 3. spectrum test --------------------------------------------------------
print("\n3. SPECTRUM TEST - m=5..13 at -69 vs -63, 2023b F631N (flat noise = 0.111)")
img = maps[("2023b", "f631n")]
for lat in (-69.0, -63.0):
    col, _ = ring(img, lat)
    good = np.isfinite(col)
    s = col.copy()
    s[~good] = np.nanmean(col[good])
    amp = np.abs(np.fft.rfft(s - s.mean()))[5:14]
    spec = amp / amp.sum()
    pk = int(np.argmax(spec)) + 5
    print(f"  lat {lat:+.0f}: " + " ".join(f"m{m}={v:.2f}" for m, v in zip(range(5, 14), spec))
          + f"   peak m={pk} at {spec.max()/0.111:.1f}x flat")
