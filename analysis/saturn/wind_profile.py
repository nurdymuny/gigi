"""Zonal wind profile u(phi) from OPAL a/b visit pairs by cloud tracking.

The a and b maps of one epoch are 9.4-11.1 h apart - about 0.9 Saturn
rotation - and both are in System III. A cloud feature moving eastward at u
relative to System III shifts to larger longitude by u * dt between them.
Cross-correlating the two ring profiles at each latitude gives that shift.

Two things have to be removed first. The decagon itself is a nearly
stationary pattern (2.5 m/s, i.e. ~0.2 deg over the pair) and would pull the
correlation toward zero shift, and large-scale albedo banding does the same.
So both profiles are high-passed (modes m < HP_CUT removed) before
correlating, leaving small-scale cloud texture that moves with the wind.

Timing: each mosaic longitude was imaged near its own central-meridian
passage; visits a and b follow the same rotation schedule offset by the visit
gap, so dt at any longitude ~ the difference in visit start times. Taken from
EXPSTART. This is an approximation the raw-exposure fit would remove.

Sign: map columns increase eastward (per outside review), so a positive
column shift from a to b is eastward motion.

Validation targets (discovery paper, 2025): jet peak ~116 m/s at 60.5 S,
FWHM ~2.8 deg. Northern hexagon jet (literature) ~100-125 m/s near 76-78 N.

Bootstrap: the ring is split into NWIN overlapping longitude windows, each
correlated separately; the spread across windows is the uncertainty.
"""
import glob
import json
import os
import re

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
A_EQ, B_POL = 60268.0, 54364.0
E2 = 1.0 - (B_POL / A_EQ) ** 2
HP_CUT = 15
NWIN = 8
MAXSHIFT_DEG = 15.0
MIN_CORR = 0.25


def axis_radius_km(phi):
    p = np.radians(abs(phi))
    return A_EQ * np.cos(p) / np.sqrt(1 - E2 * np.sin(p) ** 2)


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


def ring(img, lat, halfwidth=0.5):
    ny, nx = img.shape
    dl = 180.0 / ny
    r0 = int((90 - (lat + halfwidth)) / dl)
    r1 = int((90 - (lat - halfwidth)) / dl)
    blk = img[max(0, min(r0, r1)):max(r0, r1) + 1]
    prof = np.where(np.isfinite(blk) & (blk != 0), blk, np.nan)
    with np.errstate(all="ignore"):
        col = np.nanmean(prof, axis=0)
    return col


def highpass(x, cut=HP_CUT, comb=10, comb_max=80, comb_halfwidth=1):
    """Remove large scales AND the stationary wave's harmonic comb.

    First run (cut=15, no comb) failed validation: the profile went to
    +-100-200 m/s exactly in the decagon band. A polygon with sharp corners
    has power at every multiple of its wavenumber, and a stationary pattern
    leaking through pulls the cross-correlation toward zero shift. So notch
    m = 10, 20, ..., 80 (+-1) as well.
    """
    good = np.isfinite(x)
    if good.mean() < 0.95:
        return None
    y = np.where(good, x, np.nanmean(x))
    y = y / y.mean() - 1.0                    # fractional contrast
    F = np.fft.rfft(y)
    F[:cut] = 0
    if comb:
        for k in range(comb, min(comb_max, len(F) - 1) + 1, comb):
            F[max(0, k - comb_halfwidth): k + comb_halfwidth + 1] = 0
    return np.fft.irfft(F, n=len(y))


def shift_deg(a, b, maxshift_deg, nx):
    """Circular cross-correlation: shift of b relative to a, degrees east."""
    ms = int(round(maxshift_deg / (360.0 / nx)))
    an = (a - a.mean()) / (a.std() + 1e-12)
    bn = (b - b.mean()) / (b.std() + 1e-12)
    shifts = np.arange(-ms, ms + 1)
    cc = np.array([np.mean(an * np.roll(bn, -s)) for s in shifts])
    k = int(np.argmax(cc))
    off = 0.0
    if 0 < k < len(cc) - 1:
        y0, y1, y2 = cc[k - 1], cc[k], cc[k + 1]
        den = y0 - 2 * y1 + y2
        off = float(np.clip(0.5 * (y0 - y2) / den, -1, 1)) if abs(den) > 1e-12 else 0.0
    return (shifts[k] + off) * 360.0 / nx, float(cc[k])


def wind_at(img_a, img_b, lat, dt_days):
    ha, hb = highpass(ring(img_a, lat)), highpass(ring(img_b, lat))
    if ha is None or hb is None:
        return None
    nx = len(ha)
    wl = nx // NWIN * 2                      # window length, 50% overlap
    starts = np.arange(0, nx, nx // NWIN)
    us, cs = [], []
    for s in starts:
        idx = (np.arange(wl) + s) % nx
        # window the pair with a taper so edges do not alias
        w = np.hanning(wl)
        sh, c = shift_deg(ha[idx] * w, hb[idx] * w, MAXSHIFT_DEG, nx)
        if c >= MIN_CORR:
            us.append(sh)
            cs.append(c)
    if len(us) < NWIN // 2:
        return None
    us = np.array(us)
    C = 2 * np.pi * axis_radius_km(lat) * 1000.0     # m
    u = np.median(us) / 360.0 * C / (dt_days * 86400.0)
    u_sd = (np.percentile(us, 75) - np.percentile(us, 25)) / 1.349 / 360.0 * C / (dt_days * 86400.0)
    return dict(u=float(u), u_sd=float(u_sd), n_win=len(us), corr=float(np.mean(cs)))


def profile(img_a, img_b, dt_days, lats):
    out = []
    for lat in lats:
        r = wind_at(img_a, img_b, lat, dt_days)
        if r:
            r["lat"] = float(lat)
            out.append(r)
    return out


def jet_stats(prof):
    """Peak, and FWHM of u(phi) above the local minimum."""
    if len(prof) < 6:
        return None
    lats = np.array([p["lat"] for p in prof])
    u = np.array([p["u"] for p in prof])
    i = int(np.argmax(u))
    base = np.percentile(u, 20)
    half = base + 0.5 * (u[i] - base)
    lo, hi = i, i
    while lo > 0 and u[lo] > half:
        lo -= 1
    while hi < len(u) - 1 and u[hi] > half:
        hi += 1
    fwhm = abs(lats[hi] - lats[lo]) if (lo > 0 and hi < len(u) - 1) else np.nan
    return dict(peak_lat=float(lats[i]), peak_u=float(u[i]), peak_sd=float(prof[i]["u_sd"]), fwhm=float(fwhm))


if __name__ == "__main__":
    pat = re.compile(r"(\d{4})([ab])_(f[a-z0-9]+|fq[0-9]+n)\.fits$")
    pairs = {}
    for p in sorted(glob.glob(os.path.join(HERE, "*_f*.fits"))):
        if os.path.getsize(p) < 1_000_000:
            continue
        mt = pat.search(os.path.basename(p))
        if mt:
            pairs.setdefault((mt.group(1), mt.group(3)), {})[mt.group(2)] = p
    # the F631N-only years use the bare YYYYa/YYYYb names
    for p in sorted(glob.glob(os.path.join(HERE, "20??[ab].fits"))):
        m = re.search(r"(\d{4})([ab])\.fits$", p)
        if m and (m.group(1), "f631n") not in pairs:
            pairs.setdefault((m.group(1), "f631n"), {})[m.group(2)] = p

    south_lats = np.arange(-72.0, -50.0, 0.5)
    north_lats = np.arange(70.0, 86.0, 0.5)
    results = []
    print("ZONAL WIND FROM a/b CLOUD TRACKING")
    print(f"{'year':>5} {'filter':>7} {'dt h':>5} | {'SOUTH peak lat':>14} {'u m/s':>7} {'+-':>5} {'FWHM':>5} | "
          f"{'NORTH peak lat':>14} {'u m/s':>7} {'+-':>5} {'FWHM':>5}")
    print("-" * 100)
    for (yr, f), ab in sorted(pairs.items()):
        if "a" not in ab or "b" not in ab:
            continue
        if f not in ("f631n", "f763m", "f502n"):
            continue
        ha, ia = read_fits(ab["a"])
        hb, ib = read_fits(ab["b"])
        dt = float(hb["EXPSTART"]) - float(ha["EXPSTART"])
        ps = profile(ia, ib, dt, south_lats)
        pn = profile(ia, ib, dt, north_lats)
        js, jn = jet_stats(ps), jet_stats(pn)
        results.append(dict(year=yr, filter=f, dt_days=dt, south=ps, north=pn, south_jet=js, north_jet=jn))
        fs = f"{js['peak_lat']:14.1f} {js['peak_u']:7.0f} {js['peak_sd']:5.0f} {js['fwhm']:5.1f}" if js else f"{'--':>34}"
        fn = f"{jn['peak_lat']:14.1f} {jn['peak_u']:7.0f} {jn['peak_sd']:5.0f} {jn['fwhm']:5.1f}" if jn else f"{'--':>34}"
        print(f"{yr:>5} {f:>7} {dt*24:5.1f} | {fs} | {fn}")

    json.dump(results, open(os.path.join(HERE, "wind_results.json"), "w"), indent=1)

    # validation against the published 2025 jet
    v = [r for r in results if r["year"] == "2025" and r["south_jet"]]
    if v:
        print("\nVALIDATION - published 2025 southern jet: ~116 m/s at 60.5 S, FWHM ~2.8 deg")
        for r in v:
            j = r["south_jet"]
            print(f"  {r['filter']}: peak {j['peak_u']:.0f} +- {j['peak_sd']:.0f} m/s at {j['peak_lat']:+.1f}, FWHM {j['fwhm']:.1f}"
                  f"   ->  {'PASS' if abs(j['peak_u']-116) < 30 and abs(j['peak_lat']+60.5) < 1.5 else 'FAIL'}")
    # print one full profile for the record
    r25 = next((r for r in results if r["year"] == "2025" and r["filter"] == "f631n"), None)
    if r25:
        print("\n  2025 F631N southern profile:")
        for p in r25["south"]:
            bar = "#" * int(max(0, p["u"]) / 5)
            print(f"    {p['lat']:+6.1f}  {p['u']:7.1f} +- {p['u_sd']:5.1f}  corr {p['corr']:.2f}  {bar}")
