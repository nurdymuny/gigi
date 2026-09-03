"""Stress-test P3 (barotropization): are the three tightening measures real,
independent, and physical - or is it SNR?

P3 says the cross-filter scatter in width, peak latitude and phase all shrank
2024->2025, so the wave became vertically coherent. Attacks:

  1. CONTRAST GATE. Low-contrast measurements are unreliable and inflate
     scatter. Recompute every scatter measure using only filters whose m=10
     envelope peak exceeds 6x its local background. If 2024's scatter was
     carried by low-contrast filters, it collapses toward 2025's and the
     "tightening" was SNR.

  2. SNR-NORMALISED SCATTER. For a signal of amplitude A against noise sigma,
     phase scatter goes like sigma/A. If scatter * A is flat across years, the
     tightening is entirely amplitude growth. If scatter * A also falls, there
     is coherence gain beyond SNR.

  3. INDEPENDENCE. Within each year, correlate per-filter FWHM with per-filter
     peak latitude. Strongly correlated -> not independent measures.

  4. YEAR-BY-YEAR. Which measure tightened in which year. No smoothing over it.

  5. RIDE-ALONG CONTROL. Apply the same measures to m=11 at the same ring. It
     rides with m=10 (0.31 vs 0.36 normalised in 2025b). If it tightens the
     same way it is either the same wave or the same SNR; if m=10 tightens and
     m=11 does not, the coherence is specific to the decagon mode.
"""
import glob
import os
import re

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
exec(open(os.path.join(HERE, "phase.py")).read().split("pat = re.compile")[0])

WAVE = ["f467m", "f502n", "f631n", "f763m", "fq727n"]
PER10, PER11 = 36.0, 360.0 / 11


def envelope(img, m, lo=-70.0, hi=-56.0):
    ny = img.shape[0]
    dl = 180.0 / ny
    lats, amps = [], []
    for row in range(ny):
        lat = 90.0 - (row + 0.5) * dl
        if lo <= lat <= hi:
            s = img[row].copy()
            good = np.isfinite(s) & (s != 0)
            if good.mean() < 0.98:
                continue
            s[~good] = np.nanmean(s[good])
            xbar = s.mean()
            if xbar <= 0:
                continue
            n = len(s)
            phi = np.arange(n) * 2 * np.pi / n
            amps.append(abs(np.sum((s - xbar) * np.exp(-1j * m * phi))) / (n * xbar))
            lats.append(lat)
    lats, amps = np.array(lats), np.array(amps)
    if len(amps) < 12:
        return None
    base = np.percentile(amps, 25)
    i = int(np.argmax(amps))
    pk = amps[i]
    if pk <= base:
        return None
    half = base + 0.5 * (pk - base)
    lo_i, hi_i = i, i
    while lo_i > 0 and amps[lo_i] > half:
        lo_i -= 1
    while hi_i < len(amps) - 1 and amps[hi_i] > half:
        hi_i += 1
    if lo_i == 0 or hi_i == len(amps) - 1:
        return None
    return dict(fwhm=abs(lats[hi_i] - lats[lo_i]), lat=lats[i], amp=pk, contrast=pk / max(base, 1e-12))


pat = re.compile(r"(\d{4})([ab])_(f[a-z0-9]+|fq[0-9]+n)\.fits$")
rows = []   # dict per (epoch, filter, mode)
for p in sorted(glob.glob(os.path.join(HERE, "*_f*.fits"))):
    if os.path.getsize(p) < 1_000_000:
        continue
    mt = pat.search(os.path.basename(p))
    if not mt or int(mt.group(1)) < 2023 or mt.group(3) not in WAVE:
        continue
    _, img = read_fits(p)
    for m, per in ((10, PER10), (11, PER11)):
        e = envelope(img, m)
        z = zcoef(ring(img, -63.3)[0], m)
        if e and z:
            rows.append(dict(year=mt.group(1), visit=mt.group(2), filt=mt.group(3), m=m,
                             phase=z[1], per=per, **e))

years = ["2023", "2024", "2025"]


def circ_sd(phis, per):
    a = np.asarray(phis) * 2 * np.pi / per
    R = abs(np.mean(np.exp(1j * a)))
    return np.sqrt(-2 * np.log(max(R, 1e-12))) * per / (2 * np.pi)


def measures(sub):
    """cross-filter scatter, pooled over the two visits of one year, by visit."""
    out = {}
    f = [r["fwhm"] for r in sub]
    l = [r["lat"] for r in sub]
    out["fwhm_sd"] = np.std(f) if len(f) > 1 else np.nan
    out["tilt_sd"] = np.std(l) if len(l) > 1 else np.nan
    ph = []
    for v in "ab":
        sv = [r for r in sub if r["visit"] == v]
        if len(sv) >= 3:
            ph.append(circ_sd([r["phase"] for r in sv], sv[0]["per"]))
    out["phase_sd"] = np.mean(ph) if ph else np.nan
    out["amp"] = np.mean([r["amp"] for r in sub])
    out["contrast"] = np.mean([r["contrast"] for r in sub])
    out["n"] = len(sub)
    return out


def table(title, mode, gate):
    print(f"\n{title}")
    print(f"  {'year':>5} {'n':>3} {'fwhm sd':>8} {'tilt sd':>8} {'phase sd':>9} {'mean amp':>9} {'contrast':>9}")
    res = {}
    for y in years:
        sub = [r for r in rows if r["year"] == y and r["m"] == mode and r["contrast"] >= gate]
        if len(sub) < 3:
            print(f"  {y:>5}   (fewer than 3 measurements pass the gate)")
            continue
        mm = measures(sub)
        res[y] = mm
        print(f"  {y:>5} {mm['n']:3d} {mm['fwhm_sd']:8.2f} {mm['tilt_sd']:8.2f} {mm['phase_sd']:9.2f} "
              f"{1000*mm['amp']:9.2f} {mm['contrast']:9.1f}")
    return res


print("STRESS-TEST P3")
raw10 = table("m=10, ALL filters (as reported in PREDICTIONS.md)", 10, 0.0)
gate10 = table("1. m=10, CONTRAST-GATED (peak >= 6x local background)", 10, 6.0)

print("\n2. SNR-NORMALISED: scatter x mean amplitude (x1000). Flat => pure SNR.")
print(f"  {'year':>5} {'fwhm_sd*A':>10} {'tilt_sd*A':>10} {'phase_sd*A':>11}")
for y in years:
    if y in gate10:
        mm = gate10[y]
        print(f"  {y:>5} {1000*mm['fwhm_sd']*mm['amp']:10.3f} {1000*mm['tilt_sd']*mm['amp']:10.3f} "
              f"{1000*mm['phase_sd']*mm['amp']:11.3f}")

print("\n3. INDEPENDENCE: within-year correlation of per-filter FWHM vs peak latitude (m=10, gated)")
for y in years:
    sub = [r for r in rows if r["year"] == y and r["m"] == 10 and r["contrast"] >= 6.0]
    if len(sub) >= 4:
        f = [r["fwhm"] for r in sub]
        l = [r["lat"] for r in sub]
        c = np.corrcoef(f, l)[0, 1] if np.std(f) > 0 and np.std(l) > 0 else np.nan
        print(f"  {y}: r = {c:+.2f}   (n={len(sub)})")

print("\n4. YEAR-BY-YEAR: in which year did each measure tighten? (gated m=10)")
ys = [y for y in years if y in gate10]
for a, b in zip(ys, ys[1:]):
    for k in ("fwhm_sd", "tilt_sd", "phase_sd"):
        va, vb = gate10[a][k], gate10[b][k]
        if np.isfinite(va) and np.isfinite(vb):
            ratio = vb / va if va > 0 else np.nan
            verdict = "tightened" if ratio < 0.67 else ("loosened" if ratio > 1.5 else "flat")
            print(f"  {a}->{b}  {k:>9}: {va:5.2f} -> {vb:5.2f}   x{ratio:4.2f}  {verdict}")

ctl11 = table("5. RIDE-ALONG CONTROL: m=11 at the same ring, same gate", 11, 6.0)

print("\nREAD")
if "2024" in gate10 and "2025" in gate10:
    a, b = gate10["2024"], gate10["2025"]
    print(f"  gated m=10 2024->2025:  fwhm sd {a['fwhm_sd']:.2f}->{b['fwhm_sd']:.2f}   "
          f"tilt sd {a['tilt_sd']:.2f}->{b['tilt_sd']:.2f}   phase sd {a['phase_sd']:.2f}->{b['phase_sd']:.2f}")
    print(f"  amplitude rose x{b['amp']/a['amp']:.2f}, contrast rose x{b['contrast']/a['contrast']:.2f}")
    for k in ("fwhm_sd", "tilt_sd", "phase_sd"):
        norm_a, norm_b = a[k] * a["amp"], b[k] * b["amp"]
        if np.isfinite(norm_a) and np.isfinite(norm_b) and norm_a > 0:
            print(f"    {k:>9}: SNR-normalised ratio {norm_b/norm_a:.2f}  "
                  f"({'coherence gain beyond SNR' if norm_b/norm_a < 0.7 else 'consistent with SNR alone'})")
