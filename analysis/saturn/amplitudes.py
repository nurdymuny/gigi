"""Extract per-mode zonal amplitudes for the GIGI observation-space bundle.

For every (year, filter, ring) this computes the UNNORMALISED fractional
amplitude of each wavenumber:

    A_m = |sum_j (x_j - xbar) exp(-i m phi_j)| / (n * xbar)

Unnormalised on purpose. Dividing each mode by the band sum (as the first pass
did) produces simplex coordinates, where any mode rising mechanically pushes the
others down - which manufactures exactly the "modes compete" result we are
trying to test. Fractional amplitude is scale-free without being compositional.

The two same-day visits (a/b, ~half a Saturn rotation apart) are kept as SEPARATE
records rather than averaged: their scatter is the instrument noise floor, and
leaving them in lets GIGI see it.
"""
import glob
import json
import os
import re

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
RINGS = {"jet-63S": -63.0, "ctrl-45S": -45.0, "north78N": 78.0}
MODES = list(range(5, 14))
MIN_COVER = 0.98


def read_fits(path):
    raw = open(path, "rb").read()
    hdr, i = {}, 0
    while True:
        c = raw[i:i + 80].decode("ascii", "replace")
        i += 80
        if c.startswith("END"):
            break
        if "=" in c:
            hdr[c[:8].strip()] = c[9:].split("/")[0].strip().strip("' ")
    st = ((i + 2879) // 2880) * 2880
    nx, ny = int(hdr["NAXIS1"]), int(hdr["NAXIS2"])
    arr = np.frombuffer(raw[st:st + nx * ny * 4], dtype=">f4").astype(float)
    return hdr, arr.reshape(ny, nx)


def ring_profile(img, lat, halfwidth=1.0):
    ny, nx = img.shape
    dl = 180.0 / ny
    r0 = int((90 - (lat + halfwidth)) / dl)
    r1 = int((90 - (lat - halfwidth)) / dl)
    blk = img[max(0, min(r0, r1)):max(r0, r1) + 1]
    if blk.size == 0:
        return None, 0.0
    prof = np.where(np.isfinite(blk) & (blk != 0), blk, np.nan)
    with np.errstate(all="ignore"):
        col = np.nanmean(prof, axis=0)
    return col, float(np.isfinite(col).mean())


def amplitudes(col):
    good = np.isfinite(col)
    s = col.copy()
    s[~good] = np.nanmean(col[good])
    xbar = s.mean()
    if xbar <= 0:
        return None
    n = len(s)
    phi = np.arange(n) * 2 * np.pi / n
    dev = s - xbar
    return {m: float(abs(np.sum(dev * np.exp(-1j * m * phi))) / (n * xbar)) for m in MODES}


rows = []
pat = re.compile(r"(\d{4})([ab])_(f[a-z0-9]+|fq[0-9]+n)\.fits$")
for path in sorted(glob.glob(os.path.join(HERE, "*_f*.fits"))):
    mt = pat.search(os.path.basename(path))
    if not mt:
        continue
    # OPAL does not have every filter in every epoch; a miss returns a 404 page.
    # Guard on size and magic bytes so one absent bandpass cannot abort the sweep.
    if os.path.getsize(path) < 1_000_000:
        print(f"  skip (not a FITS, {os.path.getsize(path)} bytes): {os.path.basename(path)}")
        continue
    year, visit, filt = mt.group(1), mt.group(2), mt.group(3)
    hdr, img = read_fits(path)
    for ring_name, lat in RINGS.items():
        col, cov = ring_profile(img, lat)
        if col is None or cov < MIN_COVER:
            continue
        amps = amplitudes(col)
        if amps is None:
            continue
        rows.append({
            "ring": ring_name,
            "year": float(year),
            "visit": visit,
            "filter": filt,
            "date": hdr.get("DATE-OBS", "")[:10],
            "coverage": round(cov, 4),
            **{f"a{m}": amps[m] for m in MODES},
        })

out = os.path.join(HERE, "mode_amplitudes.json")
json.dump(rows, open(out, "w"), indent=1)

print(f"wrote {out}: {len(rows)} sections")
for ring in RINGS:
    sub = [r for r in rows if r["ring"] == ring]
    if not sub:
        print(f"  {ring:>10}: none above coverage {MIN_COVER}")
        continue
    yrs = sorted({int(r['year']) for r in sub})
    fs = sorted({r['filter'] for r in sub})
    print(f"  {ring:>10}: {len(sub):3d} sections  years {yrs}  filters {len(fs)}")
