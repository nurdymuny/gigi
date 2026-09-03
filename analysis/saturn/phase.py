"""Where is the decagon, in longitude, and does it move?

The hexagon's defining property (Godfrey 1988; Sanchez-Lavega et al. 2014) is
that it is nearly stationary in System III longitude - it drifts at roughly
-0.01 deg/day, which is to say it is locked to the planet's deep rotation. That
locking is the strongest evidence it is a Rossby wave standing in a jet rather
than a feature advected by one.

Nobody has reported whether the decagon shares that property. This measures it.

For brightness x(phi) ~ A cos(m (phi - phi0)), the complex zonal coefficient is
    z = sum_j (x_j - xbar) exp(-i m phi_j)  ~  (n/2) A exp(-i m phi0)
so phi0 = -arg(z)/m is the longitude of a brightness maximum, defined modulo
360/m degrees (36 deg for the decagon, 60 deg for the hexagon).

Calibration: the hexagon in 2018-2019, when the north was well presented. If
the method and the map longitude system are right, its phase should be stable
across the a/b same-day visits AND across the year, to within a few degrees.
Then the same method is applied to the decagon.
"""
import glob
import os
import re

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))


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


def mjd(hdr):
    return float(hdr.get("EXPSTART", "nan"))


def ring(img, lat, halfwidth=1.0):
    ny, nx = img.shape
    dl = 180.0 / ny
    r0 = int((90 - (lat + halfwidth)) / dl)
    r1 = int((90 - (lat - halfwidth)) / dl)
    blk = img[max(0, min(r0, r1)):max(r0, r1) + 1]
    prof = np.where(np.isfinite(blk) & (blk != 0), blk, np.nan)
    with np.errstate(all="ignore"):
        col = np.nanmean(prof, axis=0)
    return col, float(np.isfinite(col).mean())


def zcoef(col, m):
    good = np.isfinite(col)
    if good.mean() < 0.98:
        return None
    s = col.copy()
    s[~good] = np.nanmean(col[good])
    xbar = s.mean()
    n = len(s)
    phi = np.arange(n) * 2 * np.pi / n            # map column -> longitude
    z = np.sum((s - xbar) * np.exp(-1j * m * phi))
    amp = abs(z) / (n * xbar)
    phi0 = (-np.angle(z) / m) * 180.0 / np.pi     # deg, longitude of a max
    return amp, phi0 % (360.0 / m)


def wrap(d, period):
    """Signed difference on a circle of the given period."""
    return (d + period / 2) % period - period / 2


pat = re.compile(r"(\d{4})([ab])(?:_(f[a-z0-9]+|fq[0-9]+n))?\.fits$")
files = []
for p in sorted(glob.glob(os.path.join(HERE, "*.fits"))):
    if os.path.getsize(p) < 1_000_000:
        continue
    mt = pat.search(os.path.basename(p))
    if not mt:
        continue
    filt = mt.group(3) or "f631n"
    if mt.group(3) is None and os.path.exists(os.path.join(HERE, f"{mt.group(1)}{mt.group(2)}_f631n.fits")):
        continue      # duplicate of the renamed copy
    files.append((p, mt.group(1), mt.group(2), filt))


def run(title, lat, m, years, filters, min_amp):
    period = 360.0 / m
    print(f"\n{'=' * 74}\n{title}   lat {lat:+.0f}, m={m}, phase period {period:.0f} deg\n{'=' * 74}")
    print(f"{'epoch':>7} {'filter':>7} {'MJD':>10} {'amp':>8} {'phi0 deg':>9}")
    rows = []
    for p, yr, vis, filt in files:
        if yr not in years or filt not in filters:
            continue
        hdr, img = read_fits(p)
        col, cov = ring(img, lat)
        r = zcoef(col, m)
        if r is None or r[0] < min_amp:
            continue
        amp, phi0 = r
        rows.append((yr + vis, filt, mjd(hdr), amp, phi0))
        print(f"{yr+vis:>7} {filt:>7} {mjd(hdr):10.2f} {amp:8.5f} {phi0:9.1f}")

    # --- same-day scatter: a vs b, filter by filter -> the noise floor
    print("\n  same-day (a vs b) phase differences, by filter:")
    sd_pairs = []
    for filt in filters:
        by = {r[0]: r for r in rows if r[1] == filt}
        for yr in years:
            a, b = by.get(yr + "a"), by.get(yr + "b")
            if a and b:
                d = wrap(b[4] - a[4], period)
                sd_pairs.append(d)
                print(f"    {yr} {filt:>7}:  {d:+6.1f} deg   over {(b[2]-a[2])*24:.1f} h")
    if sd_pairs:
        noise = np.sqrt(np.mean(np.square(sd_pairs)))
        print(f"    rms same-day scatter = {noise:.2f} deg   (n={len(sd_pairs)})")
    else:
        noise = np.nan

    # --- cross-filter agreement within one visit -> is it one pattern?
    print("\n  cross-filter phase spread within a visit:")
    for ep in sorted({r[0] for r in rows}):
        ph = [r[4] for r in rows if r[0] == ep]
        if len(ph) >= 3:
            ref = ph[0]
            devs = [wrap(x - ref, period) for x in ph]
            mean_dev = np.mean(devs)
            spread = np.std(devs)
            print(f"    {ep}: mean {((ref + mean_dev) % period):6.1f}  sd {spread:5.2f} deg  (n={len(ph)})")

    # --- year to year: drift
    print("\n  year-to-year phase change (mean over filters):")
    yearly = {}
    for yr in years:
        sub = [r for r in rows if r[0].startswith(yr)]
        if not sub:
            continue
        # circular mean of phases
        ang = np.array([r[4] for r in sub]) * 2 * np.pi / period
        cm = np.angle(np.mean(np.exp(1j * ang))) * period / (2 * np.pi) % period
        yearly[yr] = (cm, np.mean([r[2] for r in sub]), len(sub))
        print(f"    {yr}: phi0 = {cm:6.1f} deg   (n={len(sub)}, MJD {yearly[yr][1]:.0f})")
    yrs = sorted(yearly)
    for a, b in zip(yrs, yrs[1:]):
        d = wrap(yearly[b][0] - yearly[a][0], period)
        dt = yearly[b][1] - yearly[a][1]
        print(f"    {a} -> {b}:  {d:+6.1f} deg over {dt:.0f} d  =  {d/dt:+.4f} deg/day  "
              f"(aliases at multiples of {period/dt:+.4f} deg/day)")
    return yearly, noise, period


# ---- calibration on the hexagon, when the north was well seen ---------------
hx, hx_noise, _ = run("CALIBRATION - the hexagon", 78.0, 6,
                      ["2018", "2019"], ["f631n"], min_amp=0.002)

# ---- the decagon, six filters, three clean years -----------------------------
dc, dc_noise, per = run("THE DECAGON", -63.0, 10,
                        ["2023", "2024", "2025"],
                        ["f467m", "f502n", "f631n", "f763m", "fq727n"], min_amp=0.002)

print("\n" + "=" * 74)
print("READ")
print("=" * 74)
if len(dc) >= 2:
    yrs = sorted(dc)
    phis = np.array([dc[y][0] for y in yrs])
    ts = np.array([dc[y][1] for y in yrs])
    # least-squares drift, unwrapping relative to first epoch
    dphi = np.array([wrap(p - phis[0], per) for p in phis])
    dt = ts - ts[0]
    slope = np.sum(dphi * dt) / np.sum(dt * dt) if np.sum(dt * dt) > 0 else 0.0
    resid = dphi - slope * dt
    print(f"  decagon System III drift (smallest alias): {slope:+.4f} deg/day")
    print(f"  residual after removing it: rms {np.sqrt(np.mean(resid**2)):.2f} deg  "
          f"vs same-day noise {dc_noise:.2f} deg")
    print(f"  hexagon literature value: about -0.013 deg/day (Sanchez-Lavega 2014)")
    # forward prediction to the next OPAL epoch, taken as ~2026-08-25 (MJD 61277)
    t_next = 61277.0
    pred = (phis[0] + slope * (t_next - ts[0])) % per
    print(f"\n  PREDICTION  2026 epoch (~MJD {t_next:.0f}): phi0 = {pred:.1f} deg  (mod {per:.0f})")
    print(f"              +- {max(dc_noise, np.sqrt(np.mean(resid**2))):.1f} deg if locked; ")
    print(f"              any other value = the decagon is not stationary in System III.")
