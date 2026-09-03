"""Is the decagon's ceiling a wall, or a front that is rising?

Each OPAL filter samples a different pressure level, because methane absorbs
more strongly at longer wavelengths and so the photons come from higher up:

    f467m   blue continuum        deep      (~ several hundred mbar)
    f502n   green continuum       deep
    f631n   red continuum         deep
    f763m   red/NIR continuum     deep-mid
    fq727n  weak CH4 band         upper troposphere (~100-200 mbar)
    fq889n  strong CH4 band       tropopause / lower stratosphere (~ <100 mbar)

Earlier: the decagon is strong in the first five and essentially absent in
FQ889N in 2025. Two questions this settles, per year:

  1. the amplitude profile with altitude - does the wave reach FQ889N, and is
     its FQ889N amplitude rising year over year (a front) or flat (a wall)?
  2. the peak LATITUDE per filter - the published team noted the position
     "shifts slightly depending on wavelength". That is a vertical tilt. Is
     the tilt shrinking as the wave organises (becoming barotropic) or not?

The hexagon developed a stratospheric expression only as northern summer
approached (Fletcher et al. 2018, Cassini CIRS). The south is now entering
spring. So the FQ889N trend is the seasonal-forcing test, and it is one the
next public epoch can check.
"""
import glob
import os
import re

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
FILTERS = ["f467m", "f502n", "f631n", "f763m", "fq727n", "fq889n"]
LEVEL = {"f467m": "deep", "f502n": "deep", "f631n": "deep", "f763m": "deep-mid",
         "fq727n": "upper trop", "fq889n": "strat"}


def read_fits(path):
    raw = open(path, "rb").read()
    hdr, i = {}, 0
    while True:
        c = raw[i:i + 80].decode("ascii", "replace")
        i += 80
        if c.startswith("END"):
            break
        if "=" in c:
            hdr[c[:8].strip()] = c[9:].split("/")[0].split("/")[0].strip().strip("' ")
    st = ((i + 2879) // 2880) * 2880
    nx, ny = int(hdr["NAXIS1"]), int(hdr["NAXIS2"])
    return hdr, np.frombuffer(raw[st:st + nx * ny * 4], dtype=">f4").astype(float).reshape(ny, nx)


def amp_row(img, row, m):
    s = img[row].copy()
    good = np.isfinite(s) & (s != 0)
    if good.mean() < 0.98:
        return np.nan
    s[~good] = np.nanmean(s[good])
    xbar = s.mean()
    if xbar <= 0:
        return np.nan
    n = len(s)
    phi = np.arange(n) * 2 * np.pi / n
    return float(abs(np.sum((s - xbar) * np.exp(-1j * m * phi))) / (n * xbar))


def scan(img, m, lo, hi):
    ny = img.shape[0]
    dl = 180.0 / ny
    out = []
    for row in range(ny):
        lat = 90.0 - (row + 0.5) * dl
        if lo <= lat <= hi:
            a = amp_row(img, row, m)
            if np.isfinite(a):
                out.append((lat, a))
    return np.array(out)


def peak_and_bg(prof):
    """Peak amplitude and latitude, and the scan-window background (25th pct)."""
    if len(prof) < 12:
        return None
    amps = prof[:, 1]
    bg = np.percentile(amps, 25)
    i = int(np.argmax(amps))
    return prof[i, 0], amps[i], bg


pat = re.compile(r"(\d{4})([ab])_(f[a-z0-9]+|fq[0-9]+n)\.fits$")
data = {}   # (year, visit, filter) -> (lat, peak, bg)
for p in sorted(glob.glob(os.path.join(HERE, "*_f*.fits"))):
    if os.path.getsize(p) < 1_000_000:
        continue
    mt = pat.search(os.path.basename(p))
    if not mt or int(mt.group(1)) < 2023:
        continue
    _, img = read_fits(p)
    r = peak_and_bg(scan(img, 10, -70.0, -56.0))
    if r:
        data[(mt.group(1), mt.group(2), mt.group(3))] = r

years = sorted({k[0] for k in data})

print("m=10 PEAK AMPLITUDE by altitude (filter) and year   [fractional, x1000]")
print(f"{'filter':>7} {'level':>11}  " + "  ".join(f"{y:>12}" for y in years) + "   contrast vs bg (2025)")
print("-" * 80)
for f in FILTERS:
    cells = []
    for y in years:
        v = [data[k][1] for k in data if k[0] == y and k[2] == f]
        cells.append(f"{1000*np.mean(v):6.2f} (n{len(v)})" if v else f"{'--':>12}")
    v25 = [data[k] for k in data if k[0] == years[-1] and k[2] == f]
    con = f"{np.mean([x[1]/max(x[2],1e-9) for x in v25]):5.1f}x" if v25 else "  --"
    print(f"{f:>7} {LEVEL[f]:>11}  " + "  ".join(cells) + f"   {con}")

print("\nPEAK LATITUDE by altitude and year  (the vertical tilt)")
print(f"{'filter':>7} {'level':>11}  " + "  ".join(f"{y:>12}" for y in years))
print("-" * 70)
tilt = {}
for f in FILTERS:
    cells = []
    for y in years:
        v = [data[k][0] for k in data if k[0] == y and k[2] == f]
        if v:
            cells.append(f"{np.mean(v):+8.1f}    ")
            tilt.setdefault(y, {})[f] = np.mean(v)
        else:
            cells.append(f"{'--':>12}")
    print(f"{f:>7} {LEVEL[f]:>11}  " + "  ".join(cells))

print("\nTILT: peak-latitude spread across the five filters that carry the wave")
for y in years:
    lat5 = [tilt[y][f] for f in FILTERS[:5] if f in tilt.get(y, {})]
    if len(lat5) >= 4:
        print(f"  {y}: deep {tilt[y].get('f631n', np.nan):+.1f}  ->  upper-trop "
              f"{tilt[y].get('fq727n', np.nan):+.1f}     spread sd {np.std(lat5):.2f} deg  "
              f"range {max(lat5)-min(lat5):.1f} deg")

print("\nTHE CEILING: FQ889N (strat) m=10 amplitude as a fraction of F631N (deep)")
for y in years:
    s = [data[k][1] for k in data if k[0] == y and k[2] == "fq889n"]
    d = [data[k][1] for k in data if k[0] == y and k[2] == "f631n"]
    sb = [data[k][2] for k in data if k[0] == y and k[2] == "fq889n"]
    if s and d:
        print(f"  {y}: FQ889N/F631N = {np.mean(s)/np.mean(d):.3f}    "
              f"FQ889N peak/bg = {np.mean(s)/max(np.mean(sb),1e-9):.2f}x  "
              f"(a real wave would sit well above ~1.5x)")
