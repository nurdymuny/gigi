"""Detect the decagon as a meandering RIDGE, not as brightness on a fixed ring.

The fixed-latitude Fourier transform failed a known positive (FQ889N, where
the wave sits a few degrees equatorward of the deep filters). The decagon is
a band whose LATITUDE varies with longitude. So measure that directly.

Method
  1. Take the south polar window (LAT_LO..LAT_HI). Build the zonal-mean
     latitude profile T(phi) over all longitudes: the template.
  2. At every longitude column, cross-correlate that column's latitude
     profile against T over shifts of +-MAXSHIFT degrees. The shift that
     maximises correlation is the band's local latitude displacement
     dphi(lambda). Correlation is scale-free, so unequal vertex contrast does
     not break it. Parabolic interpolation gives sub-pixel shifts.
  3. Fourier-decompose dphi(lambda). The primary measurement is now a
     GEOMETRIC amplitude A_m in degrees (and km), not brightness over mean
     brightness. Report the spectrum P_m = |z_m| / sum|z_5..13| against the
     flat level too, so this can be compared with the old detector.
  4. Same procedure on a control window around -45 where there is no wave.

Gates the outside review set:
  * must recover the published F763M positive (deep, ~63 S)
  * must recover the published FQ889N positive (~59-60.5 S)
  * must return null on the -45 control
  * (injection_recovery.py then tests: must fail when a planted ridge is
    removed, must recover planted amplitude and phase)

No astropy, numpy only.
"""
import glob
import json
import os
import re
import sys

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))

A_EQ, B_POL = 60268.0, 54364.0
E2 = 1.0 - (B_POL / A_EQ) ** 2
MODES = np.arange(5, 14)
FLAT = 1.0 / 9.0
MAXSHIFT_DEG = 3.0

WINDOWS = {
    "south": (-70.0, -52.0),
    "ctrl45": (-52.0, -38.0),
}


def deg_lat_km(phi):
    p = np.radians(abs(phi))
    return A_EQ * (1 - E2) / (1 - E2 * np.sin(p) ** 2) ** 1.5 * np.pi / 180


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


def window_rows(ny, lo, hi):
    dl = 180.0 / ny
    lats = 90.0 - (np.arange(ny) + 0.5) * dl
    sel = np.where((lats >= lo) & (lats <= hi))[0]
    return sel, lats[sel]


def track_ridge(img, lo, hi, maxshift_deg=MAXSHIFT_DEG):
    """Return (lon_deg, dphi_deg, corr, template_peak_lat) or None."""
    ny, nx = img.shape
    dl = 180.0 / ny
    rows, lats = window_rows(ny, lo, hi)
    sub = img[rows]                              # (nlat, nlon)
    good = np.isfinite(sub) & (sub != 0)
    if good.mean() < 0.95:
        return None
    sub = np.where(good, sub, np.nan)
    # per-column normalisation removes limb-darkening gain differences
    col_mean = np.nanmean(sub, axis=0)
    sub = sub / col_mean
    template = np.nanmean(sub, axis=1)           # zonal-mean latitude profile
    template = template - np.nanmean(template)
    ms = int(round(maxshift_deg / dl))
    inner = slice(ms, len(template) - ms)        # template interior
    T = template[inner]
    T = T - T.mean()
    Tn = np.sqrt(np.sum(T * T)) + 1e-12
    lon = np.arange(nx) * 360.0 / nx
    dphi = np.full(nx, np.nan)
    corr = np.full(nx, np.nan)
    for j in range(nx):
        col = sub[:, j]
        if not np.all(np.isfinite(col)):
            col = np.where(np.isfinite(col), col, np.nanmean(col))
        col = col - col.mean()
        cc = np.empty(2 * ms + 1)
        for k, s in enumerate(range(-ms, ms + 1)):
            seg = col[ms + s: len(col) - ms + s]
            seg = seg - seg.mean()
            cc[k] = np.sum(seg * T) / (np.sqrt(np.sum(seg * seg)) * Tn + 1e-12)
        k = int(np.argmax(cc))
        # parabolic sub-pixel refinement
        if 0 < k < len(cc) - 1:
            y0, y1, y2 = cc[k - 1], cc[k], cc[k + 1]
            den = (y0 - 2 * y1 + y2)
            off = 0.5 * (y0 - y2) / den if abs(den) > 1e-12 else 0.0
            off = float(np.clip(off, -1, 1))
        else:
            off = 0.0
        shift_px = (k - ms) + off
        # positive shift means the column's structure sits at a LOWER row index
        # than the template, i.e. further north. Convert to +north degrees.
        dphi[j] = -shift_px * dl
        corr[j] = cc[k]
    peak_lat = lats[np.argmax(template)]
    return lon, dphi, corr, float(peak_lat)


def decompose(lon, dphi):
    good = np.isfinite(dphi)
    d = np.where(good, dphi, np.nanmean(dphi))
    d = d - d.mean()
    n = len(d)
    lam = np.radians(lon)
    z = {m: np.sum(d * np.exp(-1j * m * lam)) for m in MODES}
    amp_deg = {m: 2 * abs(z[m]) / n for m in MODES}     # geometric amplitude, deg
    phase = {m: (-np.angle(z[m]) / m * 180 / np.pi) % (360.0 / m) for m in MODES}
    tot = sum(abs(z[m]) for m in MODES)
    P = {m: abs(z[m]) / tot for m in MODES}
    return amp_deg, phase, P


def run_one(path, win, lo, hi):
    hdr, img = read_fits(path)
    r = track_ridge(img, lo, hi)
    if r is None:
        return None
    lon, dphi, corr, peak_lat = r
    amp, ph, P = decompose(lon, dphi)
    pk = int(MODES[np.argmax([P[m] for m in MODES])])
    return dict(window=win, peak_lat=peak_lat,
                mean_corr=float(np.nanmean(corr)),
                rms_dphi=float(np.nanstd(dphi)),
                amp10_deg=amp[10], amp10_km=amp[10] * deg_lat_km(peak_lat),
                phase10=ph[10], P10=P[10], P10_flat=P[10] / FLAT, peak_mode=pk,
                P={int(m): P[m] for m in MODES}, amp={int(m): amp[m] for m in MODES},
                phase={int(m): ph[m] for m in MODES})


if __name__ == "__main__":
    pat = re.compile(r"(\d{4})([ab])_(f[a-z0-9]+|fq[0-9]+n)\.fits$")
    files = []
    for p in sorted(glob.glob(os.path.join(HERE, "*_f*.fits"))):
        if os.path.getsize(p) < 1_000_000:
            continue
        mt = pat.search(os.path.basename(p))
        if mt and int(mt.group(1)) >= 2023:
            files.append((p, mt.group(1) + mt.group(2), mt.group(3)))

    out = []
    print("RIDGE TRACKER - geometric m=10 amplitude of the band's latitude displacement")
    print(f"{'epoch':>6} {'filter':>7} {'window':>7} {'ridge lat':>9} {'corr':>5} "
          f"{'rms dphi':>8} {'A10 deg':>8} {'A10 km':>7} {'P10/flat':>8} {'peak m':>6} {'phi0':>6}")
    print("-" * 96)
    for p, ep, f in files:
        for win, (lo, hi) in WINDOWS.items():
            r = run_one(p, win, lo, hi)
            if r is None:
                continue
            r.update(epoch=ep, filter=f)
            out.append(r)
            flag = "  <-- WAVE" if (r["P10_flat"] >= 2.0 and r["peak_mode"] == 10) else ""
            print(f"{ep:>6} {f:>7} {win:>7} {r['peak_lat']:9.1f} {r['mean_corr']:5.2f} "
                  f"{r['rms_dphi']:8.3f} {r['amp10_deg']:8.3f} {r['amp10_km']:7.0f} "
                  f"{r['P10_flat']:8.2f} {r['peak_mode']:6d} {r['phase10']:6.1f}{flag}")

    json.dump(out, open(os.path.join(HERE, "ridge_results.json"), "w"), indent=1)

    print("\nGATES")
    def best(ep_prefix, filt, win):
        c = [r for r in out if r["epoch"].startswith(ep_prefix) and r["filter"] == filt and r["window"] == win]
        return max(c, key=lambda r: r["P10_flat"]) if c else None
    g1 = best("2025", "f763m", "south")
    g2 = best("2025", "fq889n", "south")
    g3 = [r for r in out if r["window"] == "ctrl45"]
    print(f"  F763M 2025 south:  P10/flat {g1['P10_flat']:.2f}, peak m={g1['peak_mode']}, ridge {g1['peak_lat']:+.1f}"
          f"  -> {'PASS' if g1['P10_flat'] >= 2 and g1['peak_mode'] == 10 else 'FAIL'}" if g1 else "  F763M: no result")
    print(f"  FQ889N 2025 south: P10/flat {g2['P10_flat']:.2f}, peak m={g2['peak_mode']}, ridge {g2['peak_lat']:+.1f}"
          f"  -> {'PASS' if g2['P10_flat'] >= 2 and g2['peak_mode'] == 10 else 'FAIL'}" if g2 else "  FQ889N: no result")
    if g3:
        mx = max(r["P10_flat"] for r in g3)
        n10 = sum(1 for r in g3 if r["peak_mode"] == 10 and r["P10_flat"] >= 2)
        print(f"  -45 control ({len(g3)} maps): max P10/flat {mx:.2f}, false m=10 detections {n10}"
              f"  -> {'PASS' if n10 == 0 else 'FAIL'}")
