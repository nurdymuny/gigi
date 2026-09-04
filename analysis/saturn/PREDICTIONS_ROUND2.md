# Round 2 — predictions from the rebuilt methods

Registered 2026-09-03, evening. B. R. Davis, Davis Geometric. Follows the
outside review that retired all five round-1 predictions
([PREDICTIONS.md](PREDICTIONS.md)). Every number here comes from a method
that was gated on a known positive and calibrated by injection before it was
allowed to say anything.

---

## What was built

| method | status | gate / calibration |
|---|---|---|
| `ridge_tracker.py` | built | recovers F763M (3.54×) **and the FQ889N positive the old detector missed** (2.53×, ridge at −58.7°); zero false m=10 on 33 control maps |
| `injection_recovery.py` | built, null rebuilt once | FPR 3/96 at 2.0× (null max 1.93–2.11); 50% completeness at A≈0.12°, 100% at ≥0.20° deep / ≥0.30° FQ889N; amplitude bias 0.89–1.06; phase 1–1.5° rms; every real detection dies under removal |
| `wind_profile.py` | built, **partial** | jet found at 108–137 m/s near −61° in 3 of 6 recent pairs; errors 10–60 m/s; FWHM 1.0–4.0° vs published 2.8° — mosaic-limited, not usable for U″ |
| `stability_modes.py` | built, sign bug fixed once | parametric published jet; north reproduces m=6; see P-R2-4/5 |
| GIGI bundle over ridge A₅…A₁₃ | run | K=0.040; m=10 var/range² 0.026 (tight cluster, no bimodality) — QA only |
| `vertex_motion.py` | not built | needs the paper's PVOL/ALPO vertex table |
| `raw_exposure_check.py` | not built | needs MAST raw frames; required for P-R2-3 |

Bugs found and fixed while building, recorded because they would have
produced wrong predictions: the first injection null randomised each row's
Fourier phases independently, which preserves every row's m=10 power and gave
a 25–54% "false-positive rate" — replaced with blocked longitude resampling;
the stability operator had the PV-gradient sign flipped, giving e-folding
times of years for a 116 m/s jet; multithreaded BLAS on 180×180 matrices ran
at 6.5 cores for 40 minutes to reach 15% — pinned to one thread; and three
zombie extraction jobs from an earlier 404 crash were still alive.

## Measurements the calibration made trustworthy

**The meander's geometric amplitude has not grown.** Ridge A₁₀ in degrees of
latitude displacement, deep filters:

| epoch | F631N | F763M | F467M | F502N | FQ727N |
|---|---|---|---|---|---|
| 2023 | 0.198 | 0.238 | 0.174 | 0.207 / 0.162 | 0.218 |
| 2024 | 0.193 / 0.193 | 0.192 / 0.184 | 0.193 | 0.188 / 0.225 | 0.172 / 0.202 |
| 2025 | 0.205 / 0.208 | 0.197 / 0.200 | 0.183 / 0.181 | 0.196 / 0.203 | 0.201 / 0.198 |

≈ 0.20° ≈ 200 km, flat to ±0.02° across three years. Injection says the
estimator is unbiased at this amplitude. What grew 47% (round 1) was albedo
contrast, not displacement — the paper's "weaker albedo contrasts" in
2023–24, quantified.

**FQ889N: absent-to-present.** Ridge P₁₀/flat in FQ889N: 1.14 (2023a),
1.33 (2024a), 0.09 (2024b) → **2.53 / 2.27 (2025a/b)**, phase within 1° of
the deep filters. But per-visit completeness at A=0.18° in FQ889N is only
~65–75%, so two consecutive misses have probability ~6–12% even if the wave
was there. Suggestive of a 2025 onset; not established.

**Phase drift, both estimators agree.** Ridge phases 8.5° → 6.0° → 33.5°
(2023→24→25; cross-filter sd < 1°); year-to-year changes −2.5°, −8.5°, the
same as the fixed-ring estimator (−2.4°, −8.0°). Both fit the published
+0.4634°/day branch (rms 0.17°) and not the "locked" branch.

## Predictions

### P-R2-1. The meander amplitude stays at 0.20°

**Predict.** 2026 OPAL, both visits, F631N and F763M: ridge A₁₀ = 0.20 ±
0.05° (0.15–0.25°).

**Falsified if** A₁₀ < 0.12° in both deep filters (the wave is decaying) or
> 0.30° (still growing). Calibration: 100% completeness and < 5% bias at
0.20°, so a miss is physics, not the detector.

### P-R2-2. The stratospheric expression persists

**Predict.** 2026 FQ889N ridge P₁₀/flat ≥ 2.0 in at least one visit, phase
within 3° of the deep filters.

**Falsified if** both visits < 1.6× while the deep filters remain ≥ 2.5×.
P(both miss | present at 2025 amplitude) ≈ 6–12%, so a double miss is
~90% evidence the stratospheric touch was transient.

### P-R2-3. GO-18102 splits the drift branches — conditional

HST program GO-18102 observed Saturn 16–17 September 2025, ≈18.5 days after
OPAL 2025b. Ridge phase at 2025b: 33.5°.

| branch | predicted ridge phase, 2025-09-16 (mod 36°) |
|---|---|
| +0.4634°/day (published, best fit) | 33.5 + 8.6 = **6.1°** |
| −0.0141°/day ("locked", worst fit) | **33.2°** |

Separation 27°. Uncertainty ≈ ±6° (32-day vertex oscillation of 4.6–8.4°
over 18 days, plus ±0.6° from drift error). **Requires the raw-exposure
pipeline** — GO-18102 is not an OPAL product. Registered so the test exists
before the data are processed.

### P-R2-4. The northern jet is 2.6–3.4° wide

The barotropic model gives m=6 as the fastest-growing mode at 77.5°N for a
Gaussian jet of FWHM 2.8–3.2° and L_D 1500–3000 km (sensitivity mode m=6 at
35–42%). A 2.0° jet gives m=10; a 4.0° jet gives m=5.

**Predict.** The measured northern polar jet FWHM at the hexagon's level is
2.6–3.4°. Checkable now against Cassini-era wind profiles (Antuñano et al.
2015, not read here) or any future measurement.

**Falsified if** the measured FWHM is ≤ 2.2° or ≥ 3.8°, in which case the
barotropic model does not explain the hexagon's wavenumber either.

### P-R2-5. Free instability makes a decagon only for L_D ≤ 1500 km

With the published southern jet (116 m/s, 2.8° FWHM, 60.5°S), the fastest
barotropic mode is m=15–17 for L_D ≥ 2000 km and m=10–11 only for
L_D = 1000–1500 km, where growth slows to 0.02–0.4/day (e-fold 3–50 days —
compatible with a two-year assembly). At L_D ≈ 1500 km **one parameterisation
gives m=6 north and m≈11 south**, on a growth curve too flat to discriminate
±1. Circumference alone would have said 13.

**Predict.** Either the deformation radius at the wave's level is ≤ 1500 km,
or the decagon is forced (the dark anticyclonic vortex the paper names).

**Falsified as a free-instability account if** an independent L_D ≥ 2000 km
is established at that level *and* the vortex is shown not to drive it. The
paper's own L_D range (850–2700 km, per reviewer) straddles the boundary, so
this is live.

## What this round did not do

- Measure the jet. The mosaics carry too little small-scale texture; the
  raw frames are needed.
- Fit the 32-day vertex oscillation. No vertex table.
- Analyse the m=11 companion's complex phase relationship to m=10
  (reviewer's method 8). The ridge tracker returns z₉, z₁₀, z₁₁; the
  analysis is a follow-on.
- Read the paper. Still paywalled from this machine; the abstract and the
  reviewer's report are what was used.

## Honest scope

P-R2-1 and P-R2-2 are the two that stand on a calibrated instrument and
public data with a public date. P-R2-3 needs a pipeline that does not yet
exist here. P-R2-4 and P-R2-5 are model predictions about quantities other
people can already measure — they are the first things in this directory that
address *why ten and why six* rather than *whether*, and they are a barotropic
β-plane model of a stratified spherical atmosphere, which is the least a
model can be and still be one.
