"""Photometric m-mode ENVELOPE width at both of Saturn's polygonal poles.

CORRECTION AFTER OUTSIDE REVIEW (2026-09-03). This script does NOT measure jet
width, and its first docstring said it did. It measures the FWHM of the m=10
(or m=6) brightness-amplitude envelope as a function of latitude - a
wave-contrast width. Jet width requires a zonal wind profile u(phi) from cloud
tracking, which reflectivity maps cannot give. The discovery paper measures
the 2025 wind jet at about 2.8 deg FWHM and 116 m/s, unchanged in shape since
1981; the "1.7 deg" this script returns for 2025 is a different quantity.

Two further limits. Half-max crossings are taken on 0.2 deg rows without
interpolation, so any cross-filter scatter below ~0.2 deg is quantisation and
must not be reported. And the lambda/W = constant test this script was built
to run is not standard theory - mode selection depends on the full profile,
beta, L_D, stratification and shear - so its output should be read as a
description of the envelope, not a test of anything.

Original rationale, kept for the record: the idea was that if the wavenumber
a jet selects were set by circumference over some multiple of a
characteristic width, then

    lambda / W  =  the same constant, north and south

with lambda = C(phi) / m. Even had W been a jet width, the south was not in
steady state across the epochs used, so the test had no fixed quantity.

Method: scan latitude finely, compute the UNNORMALISED fractional amplitude of
the pole's dominant mode at each latitude, and take the full width at half
maximum of that envelope. Repeat per filter and per year to get a spread rather
than a single number.

Saturn is oblate (f = 0.098), so degrees of latitude are NOT the same distance
north and south, and the latitude-circle radius is not a*cos(phi). Both are done
properly below.
"""
import glob
import os
import re

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))

A_EQ = 60268.0   # km, equatorial radius (1 bar)
B_POL = 54364.0  # km, polar radius
E2 = 1.0 - (B_POL / A_EQ) ** 2


def axis_radius_km(phi_deg):
    """Distance from the rotation axis to the surface at planetographic latitude."""
    p = np.radians(abs(phi_deg))
    return A_EQ * np.cos(p) / np.sqrt(1.0 - E2 * np.sin(p) ** 2)


def deg_lat_km(phi_deg):
    """Kilometres per degree of latitude (meridional radius of curvature)."""
    p = np.radians(abs(phi_deg))
    M = A_EQ * (1.0 - E2) / (1.0 - E2 * np.sin(p) ** 2) ** 1.5
    return M * np.pi / 180.0


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
    return hdr, np.frombuffer(raw[st:st + nx * ny * 4], dtype=">f4").astype(float).reshape(ny, nx)


def amp_at_row(img, row, m):
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


def envelope_fwhm(img, m, lat_lo, lat_hi):
    """FWHM in degrees of the A_m(latitude) envelope, plus its peak latitude."""
    ny = img.shape[0]
    dl = 180.0 / ny
    lats, amps = [], []
    for row in range(ny):
        lat = 90.0 - (row + 0.5) * dl
        if not (lat_lo <= lat <= lat_hi):
            continue
        a = amp_at_row(img, row, m)
        if np.isfinite(a):
            lats.append(lat)
            amps.append(a)
    if len(amps) < 12:
        return None
    lats, amps = np.array(lats), np.array(amps)
    # local background = the 25th percentile of the scan window
    base = np.percentile(amps, 25)
    pk_i = int(np.argmax(amps))
    peak = amps[pk_i]
    if peak <= base:
        return None
    half = base + 0.5 * (peak - base)
    # walk out from the peak to the half-max crossings
    lo = pk_i
    while lo > 0 and amps[lo] > half:
        lo -= 1
    hi = pk_i
    while hi < len(amps) - 1 and amps[hi] > half:
        hi += 1
    if lo == 0 or hi == len(amps) - 1:
        return None          # envelope not enclosed by the window - reject
    return abs(lats[hi] - lats[lo]), lats[pk_i], peak, base


POLES = [
    ("NORTH hexagon", 6, 70.0, 86.0),
    ("SOUTH decagon", 10, -72.0, -54.0),
]

pat = re.compile(r"(\d{4})([ab])_(f[a-z0-9]+|fq[0-9]+n)\.fits$")
files = []
for p in sorted(glob.glob(os.path.join(HERE, "*_f*.fits"))):
    if os.path.getsize(p) < 1_000_000:
        continue
    mt = pat.search(os.path.basename(p))
    if mt and int(mt.group(1)) >= 2023:      # clean epochs only
        files.append((p, mt.group(1), mt.group(2), mt.group(3)))

results = {}
for label, m, lo, hi in POLES:
    print(f"\n=== {label}  (m={m}, scanning {lo:+.0f} to {hi:+.0f}) ===")
    print(f"{'epoch':>10} {'filter':>8} {'peak lat':>9} {'FWHM deg':>9} {'FWHM km':>9} {'contrast':>9}")
    rows = []
    for path, yr, vis, filt in files:
        _, img = read_fits(path)
        r = envelope_fwhm(img, m, lo, hi)
        if r is None:
            continue
        w_deg, peak_lat, peak, base = r
        w_km = w_deg * deg_lat_km(peak_lat)
        rows.append((w_deg, w_km, peak_lat, peak / max(base, 1e-12)))
        print(f"{yr+vis:>10} {filt:>8} {peak_lat:9.1f} {w_deg:9.2f} {w_km:9.0f} {peak/max(base,1e-12):9.2f}")
    if rows:
        wd = np.array([r[0] for r in rows])
        wk = np.array([r[1] for r in rows])
        pl = np.array([r[2] for r in rows])
        print(f"{'':>10} {'MEDIAN':>8} {np.median(pl):9.1f} {np.median(wd):9.2f} {np.median(wk):9.0f}"
              f"   n={len(rows)}  sd(deg)={wd.std():.2f}")
        results[label] = dict(m=m, w_km=float(np.median(wk)), w_deg=float(np.median(wd)),
                              lat=float(np.median(pl)), n=len(rows), sd=float(wd.std()))

print("\n" + "=" * 66)
print("THE TEST:  is lambda / W the same constant at both poles?")
print("=" * 66)
for label, r in results.items():
    circ = 2 * np.pi * axis_radius_km(r["lat"])
    lam = circ / r["m"]
    print(f"  {label}")
    print(f"    peak latitude        {r['lat']:+.1f} deg   (n={r['n']} measurements, sd {r['sd']:.2f} deg)")
    print(f"    latitude circle      {circ:,.0f} km")
    print(f"    wavelength lambda    {lam:,.0f} km   (= circumference / m={r['m']})")
    print(f"    jet width W          {r['w_km']:,.0f} km   ({r['w_deg']:.2f} deg)")
    print(f"    lambda / W           {lam / r['w_km']:.2f}")
if len(results) == 2:
    (ln, rn), (ls, rs) = list(results.items())
    cn = 2 * np.pi * axis_radius_km(rn["lat"]) / rn["m"] / rn["w_km"]
    cs = 2 * np.pi * axis_radius_km(rs["lat"]) / rs["m"] / rs["w_km"]
    print(f"\n  ratio (lambda/W)_south / (lambda/W)_north = {cs / cn:.2f}")
    print("  PASS if within ~1.0 +- 0.25 : one constant sets the mode at both poles.")
    print("  FAIL otherwise             : the poles do not share a mode-selection rule,")
    print("                               and the wavenumber difference needs another cause.")
