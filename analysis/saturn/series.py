"""Eight-year history of both of Saturn's polar wave modes, from OPAL F631N.

The control that matters: Saturn's sub-Earth latitude swings from ~+26 deg (2018,
north tilted toward Earth) through 0 at equinox (May 2025) and on toward the south.
So the south pole becomes progressively BETTER RESOLVED across exactly the window
in which the decagon "appears". Coverage is therefore reported next to every
measurement - without it, an emerging wave cannot be told apart from an emerging
view of one.
"""
import glob
import os

import numpy as np

MLO, MHI = 5, 14


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


def ring(img, lat_target, halfwidth=1.0):
    """Mean brightness vs longitude in a latitude band. Returns (profile, coverage)."""
    ny, nx = img.shape
    dl = 180.0 / ny
    r0 = int((90 - (lat_target + halfwidth)) / dl)
    r1 = int((90 - (lat_target - halfwidth)) / dl)
    blk = img[max(0, min(r0, r1)):max(r0, r1) + 1]
    if blk.size == 0:
        return None, 0.0
    good = np.isfinite(blk) & (blk != 0)
    prof = np.where(good, blk, np.nan)
    with np.errstate(all="ignore"):
        col = np.nanmean(prof, axis=0)
    cov = np.isfinite(col).mean()
    return col, cov


def mode_power(col, m):
    good = np.isfinite(col)
    if good.mean() < 0.95:
        return np.nan
    s = col.copy()
    s[~good] = np.nanmean(col[good])
    amp = np.abs(np.fft.rfft(s - s.mean()))
    band = amp[MLO:MHI].sum()
    return float(amp[m] / band) if band > 0 else np.nan


files = sorted(glob.glob(os.path.join(os.path.dirname(__file__), "*.fits")))
print(f"{'epoch':>7} {'obs date':>12} | "
      f"{'NORTH 78N':>22} | {'SOUTH 63S':>22}")
print(f"{'':>7} {'':>12} | {'cover':>6} {'m=6':>7} {'m=10':>7} | "
      f"{'cover':>6} {'m=10':>7} {'m=6':>7}")
print("-" * 78)

rows = []
for f in files:
    hdr, img = read_fits(f)
    ep = os.path.basename(f).replace(".fits", "")
    date = hdr.get("DATE-OBS", "?")[:10]
    cn, covn = ring(img, 78.0)
    cs, covs = ring(img, -63.0)
    n6 = mode_power(cn, 6) if cn is not None else np.nan
    n10 = mode_power(cn, 10) if cn is not None else np.nan
    s10 = mode_power(cs, 10) if cs is not None else np.nan
    s6 = mode_power(cs, 6) if cs is not None else np.nan
    rows.append((ep, date, covn, n6, n10, covs, s10, s6))
    fmt = lambda v: "  --  " if not np.isfinite(v) else f"{v:6.3f}"
    print(f"{ep:>7} {date:>12} | {covn:6.2f} {fmt(n6)} {fmt(n10)} | "
          f"{covs:6.2f} {fmt(s10)} {fmt(s6)}")

print("\nREAD:")
n6s = [r[3] for r in rows if np.isfinite(r[3])]
s10s = [(r[0], r[6]) for r in rows if np.isfinite(r[6])]
if n6s:
    print(f"  hexagon m=6 at 78N over {len(n6s)} epochs: "
          f"mean {np.mean(n6s):.3f}  sd {np.std(n6s):.3f}  "
          f"range {min(n6s):.3f}-{max(n6s):.3f}")
if s10s:
    print("  decagon m=10 at 63S by epoch: " +
          ", ".join(f"{e}={v:.3f}" for e, v in s10s))
    early = [v for e, v in s10s if e < "2023"]
    late = [v for e, v in s10s if e >= "2023"]
    if early and late:
        print(f"  pre-2023 mean {np.mean(early):.3f}  vs  2023+ mean {np.mean(late):.3f}"
              f"   ratio {np.mean(late)/max(np.mean(early),1e-9):.2f}x")
covs_by_ep = [(r[0], r[5]) for r in rows]
print("  south-pole coverage trend: " +
      ", ".join(f"{e}={c:.2f}" for e, c in covs_by_ep))
