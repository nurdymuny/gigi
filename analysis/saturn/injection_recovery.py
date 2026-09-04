"""Calibrate the ridge tracker: what can it see, what does it invent?

Until this exists, no null from the tracker means anything - the outside
review's point 14. Three experiments on real maps:

  NULL     phase-randomise every latitude row's longitude structure
           independently. Per-row red spectrum and the zonal-mean template
           are preserved (DC untouched); any coherent meander is destroyed.
           Run the tracker. Fraction with P10/flat >= 2 and peak m=10 is the
           false-positive rate of the detector on this map's noise.

  INJECT   build a synthetic window: the map's own zonal-mean template T(phi)
           displaced column by column by dphi(lambda) = A cos(10 lambda + psi),
           plus the phase-randomised residual as noise. Sweep A. Record
           detection probability, recovered/injected amplitude, phase error.

  REMOVE   take the real 2025 window, shift every column back by the
           tracker's own recovered m=10 meander, re-run. If the detection
           survives its own removal, the tracker is reading something other
           than the meander.

Noise is never simulated as independent white pixels; it is the map's own
residual, phase-scrambled along longitude only.
"""
import os
import sys

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from ridge_tracker import (FLAT, MODES, decompose, read_fits, track_ridge,
                           window_rows)

RNG = np.random.default_rng(20260903)
LO, HI = -70.0, -52.0
AMPS = [0.0, 0.10, 0.15, 0.20, 0.30, 0.50, 0.80, 1.20]
NREAL = 24


def window(img):
    rows, lats = window_rows(img.shape[0], LO, HI)
    sub = img[rows].copy()
    good = np.isfinite(sub) & (sub != 0)
    sub = np.where(good, sub, np.nan)
    sub = sub / np.nanmean(sub, axis=0)          # per-column gain, as the tracker does
    sub = np.where(np.isfinite(sub), sub, np.nanmean(sub))
    return sub, lats, rows


BLOCK_DEG = 9.0   # a quarter of the m=10 wavelength


def phase_randomise_rows(sub):
    """Blocked longitude resampling - the reviewer's null.

    The first version randomised each row's Fourier phases independently.
    That preserves every row's m=10 POWER (the wave put it there) and only
    scrambles its phase; summing rows, the tracker recovers a random-walk
    m=10 meander and the 'null' fired 25-54% of the time. Not a null.

    This version cuts the residual into longitude blocks a quarter-wavelength
    wide and permutes them with ONE permutation applied to every row. Local
    red noise, seams and limb structure keep their cross-row coherence; any
    m=10 meander is destroyed because its blocks land in random order.
    """
    nx = sub.shape[1]
    bl = int(round(BLOCK_DEG / (360.0 / nx)))
    nb = nx // bl
    perm = RNG.permutation(nb)
    idx = np.concatenate([np.arange(b * bl, (b + 1) * bl) for b in perm])
    if len(idx) < nx:                       # tail that did not fit a block
        idx = np.concatenate([idx, np.arange(nb * bl, nx)])
    return sub[:, idx]


def shift_columns(template, lats, dphi):
    """Image whose every column is the template displaced by dphi[j] degrees (+north)."""
    nx = len(dphi)
    out = np.empty((len(lats), nx))
    for j in range(nx):
        # template is sampled at lats; we want value at lat - dphi (structure moved north by dphi)
        out[:, j] = np.interp(lats - dphi[j], lats[::-1], template[::-1])
    return out


def embed(img, sub_new, rows):
    im = img.copy()
    im[rows] = sub_new
    return im


def run_tracker(img):
    r = track_ridge(img, LO, HI)
    if r is None:
        return None
    lon, dphi, corr, peak_lat = r
    amp, ph, P = decompose(lon, dphi)
    pk = int(MODES[np.argmax([P[m] for m in MODES])])
    return dict(A10=amp[10], phi10=ph[10], P10f=P[10] / FLAT, peak=pk, lon=lon, dphi=dphi)


def detected(r):
    return r is not None and r["P10f"] >= 2.0 and r["peak"] == 10


def wrap(d, per=36.0):
    return (d + per / 2) % per - per / 2


if __name__ == "__main__":
    bases = [("2025b_f763m.fits", "F763M deep"), ("2025b_fq889n.fits", "FQ889N strat"),
             ("2025b_f467m.fits", "F467M blue"), ("2024a_f631n.fits", "F631N 2024")]
    summary = {}
    for fname, label in bases:
        path = os.path.join(HERE, fname)
        if not os.path.exists(path):
            continue
        _, img = read_fits(path)
        sub, lats, rows = window(img)
        template = sub.mean(axis=1)
        resid = sub - template[:, None]
        real = run_tracker(img)
        print(f"\n=== {label}   real map: A10 {real['A10']:.3f} deg, P10/flat {real['P10f']:.2f}, "
              f"peak m={real['peak']}, phi0 {real['phi10']:.1f} ===")

        # ---- NULL ---------------------------------------------------------
        fp = 0
        maxP = 0.0
        for _ in range(NREAL):
            noise = phase_randomise_rows(resid)
            r = run_tracker(embed(img, template[:, None] + noise, rows))
            if r:
                maxP = max(maxP, r["P10f"])
                fp += detected(r)
        print(f"  NULL   ({NREAL} realisations): false m=10 detections {fp}/{NREAL} = {fp/NREAL:.2f}   max P10/flat {maxP:.2f}")

        # ---- INJECT --------------------------------------------------------
        print(f"  INJECT  {'A in':>5} {'det':>5} {'A out/A in':>10} {'phase err rms':>13} {'P10/flat':>9}")
        lon = np.arange(sub.shape[1]) * 360.0 / sub.shape[1]
        rows_out = {}
        for A in AMPS:
            det, ratio, perr, p10 = 0, [], [], []
            for _ in range(NREAL):
                psi = RNG.uniform(0, 360)
                dphi_in = A * np.cos(np.radians(10 * lon + psi))
                synth = shift_columns(template, lats, dphi_in) + phase_randomise_rows(resid)
                r = run_tracker(embed(img, synth, rows))
                if r is None:
                    continue
                det += detected(r)
                p10.append(r["P10f"])
                if A > 0:
                    ratio.append(r["A10"] / A)
                    # injected maximum longitude: dphi max where 10*lon + psi = 0 -> lon0 = -psi/10 mod 36
                    perr.append(wrap(r["phi10"] - ((-psi / 10.0) % 36.0)))
            rows_out[A] = dict(det=det / NREAL, ratio=np.mean(ratio) if ratio else np.nan,
                               perr=np.sqrt(np.mean(np.square(perr))) if perr else np.nan,
                               p10=np.mean(p10))
            print(f"         {A:5.2f} {det/NREAL:5.2f} {rows_out[A]['ratio']:10.2f} {rows_out[A]['perr']:13.1f} {rows_out[A]['p10']:9.2f}")

        # ---- REMOVE ----------------------------------------------------------
        back = shift_columns(np.ones(len(lats)), lats, np.zeros(sub.shape[1]))  # placeholder shape
        dphi_rec = real["dphi"]
        amp10, ph10 = real["A10"], real["phi10"]
        meander = amp10 * np.cos(np.radians(10 * lon - 10 * ph10))
        # shift each real column south by its recovered m=10 meander
        unshifted = np.empty_like(sub)
        for j in range(sub.shape[1]):
            unshifted[:, j] = np.interp(lats + meander[j], lats[::-1], sub[::-1, j])
        r = run_tracker(embed(img, unshifted, rows))
        print(f"  REMOVE  after subtracting recovered m=10 meander: P10/flat {r['P10f']:.2f}, peak m={r['peak']}"
              f"  ->  {'PASS (detection dies)' if not detected(r) else 'FAIL (detection survives its own removal)'}")

        summary[label] = dict(real=dict(A10=real["A10"], P10f=real["P10f"], peak=real["peak"]),
                              null_fpr=fp / NREAL, null_maxP=maxP,
                              inject={str(k): v for k, v in rows_out.items()},
                              remove_P10f=r["P10f"], remove_peak=r["peak"])

    import json
    json.dump(summary, open(os.path.join(HERE, "injection_results.json"), "w"), indent=1, default=float)
