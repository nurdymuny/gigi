# Saturn's south-polar decagon from public OPAL maps — technical summary

B. R. Davis, Davis Geometric. Analysis date 2026-09-03. All scripts in this
directory; all data public.

---

## 1. Data

**Source.** HST WFC3/UVIS, Outer Planet Atmospheres Legacy (OPAL), PI A.
Simon, DOI 10.17909/T9G593. Legacy program: no proprietary period.

**Products.** Per epoch and filter, one global cylindrical map:
`hlsp_opal_hst_wfc3-uvis_saturn-<YYYY><a|b>_<filter>_v1_globalmap.fits`
at `https://archive.stsci.edu/hlsps/opal/cycle<NN>/saturn/`. 1800×900 px,
0.2°/px, equirectangular, full longitude at every latitude, BITPIX −32
big-endian, ~6.5 MB. Header is plain 80-char ASCII cards in 2880-byte blocks;
parsed without astropy. Row 0 = +90° (verified by hemispheric coverage
asymmetry).

**Epochs used.** Saturn 2018a–2025b (cycles 25–32), two visits per year. The
`a`/`b` visits are the same calendar day, 9.4–11.1 h apart (≈0.9 Saturn
rotation), used by OPAL to close longitude coverage. Their difference is the
same-day noise floor.

| epoch | DATE-OBS (a) | filters used | note |
|---|---|---|---|
| 2018–2022 | Jun–Sep | F631N only | time series / hexagon calibration |
| 2023a/b | 2023-10-22 | six | southern geometry poor |
| 2024a/b | 2024-08-22 | six (F467M b missing, 404) | |
| 2025a/b | 2025-08-29 | six | |

Six filters: F467M, F502N, F631N, F763M (continuum, deep), FQ727N (weak CH₄,
upper troposphere), FQ889N (strong CH₄, tropopause/lower stratosphere).

**Viewing geometry.** Sub-Earth planetodetic latitude B from JPL Horizons
(quantity 14, geocentric observer): **+12.79° (2023-10-22), +3.83°
(2024-08-22), −3.26° (2025-08-29)**. The first draft used values derived from
each map's coverage boundary (+15.1, +8.5, −7.3), biased by +2.3, +4.7, −4.0
by the 50% threshold on a mosaic. Emission angle at latitude φ taken as
|φ − B| at the central meridian; OPAL projects each input over sub-Earth
longitude ±90° and mosaics six, so the nearest CM is within ~30° and the
true per-pixel emission angle is a range (e.g. −63°: 75.8–79.3° in 2023,
59.7–63.7° in 2025). The released FITS carry no per-pixel emission or weight
plane.

## 2. Methods

**Ring profile.** Mean brightness vs longitude over rows within ±1.0° of the
target latitude (±0.5° for fine scans). Require ≥98% finite columns.

**Zonal coefficient.** For profile x_j at longitudes φ_j = 2πj/n,
z_m = Σ_j (x_j − x̄) e^(−imφ_j).

**Fractional amplitude.** A_m = |z_m| / (n x̄). Scale-free. **Caveat found
in-session:** inflates in dark rings (limb, FQ889N) because x̄ is small. Not
a wave criterion on its own.

**Spectral criterion (the wave test).** P_m = |z_m| / Σ_{k=5}^{13} |z_k|,
compared to the flat level 1/9 = 0.111. A wave is a single m standing well
above flat (≥2× used throughout). Note the null here is white across m=5–13;
real image noise is red (§6), which makes high-m peaks *more* significant
than this test credits and low-m peaks less.

**Phase.** φ₀ = −arg(z_m)/m mod 360/m, the longitude of a brightness maximum.
Same-day a/b difference = noise floor. Circular mean/SD over filters.

**Envelope width.** Scan A_m(latitude) at 0.2° steps over a window; local
background = 25th percentile; FWHM by walking to half-max from the peak;
reject if the half-max crossing hits the window edge.

**Oblate geometry.** a = 60 268 km, b = 54 364 km, e² = 1 − b²/a².
Axis radius r(φ) = a cos φ / √(1 − e² sin²φ); km per degree of latitude from
the meridional radius of curvature M(φ) = a(1−e²)/(1−e² sin²φ)^{3/2}.
Circumference C = 2πr; wavelength λ = C/m.

**Quality gate (applied where stated).** Envelope peak ≥ 6× local background;
FQ889N excluded from wave-property measurements (§5.5); envelope peak within
4° of the known jet latitude.

**GIGI.** `examples/saturn_decagon.rs` (base = (lat, lon), fiber = six
filters) and `examples/saturn_mode_bundle.rs` (base = (year, filter), fiber =
A₅…A₁₃). See §7.

## 3. Detection and controls

**Latitude.** Envelope peak over all filters and 2023–2025 epochs before
cuts: median −63.3°, n=29, sd 0.65°. Published: 63°S.

**Control ring.** m=10 mean A over 2023–2025, all filters: jet ring (−63)
0.00915; control ring (−45) 0.00254. Ratio 3.6.

**Visibility control (F631N, normalised P₁₀ at −63).**

| epoch | coverage | P₁₀ | P₆ |
|---|---|---|---|
| 2021b | 0.99 | 0.022 | 0.285* |
| 2023a | 1.00 | 0.271 | 0.106 |
| 2023b | 1.00 | 0.291 | 0.102 |
| 2024a | 1.00 | 0.310 | 0.095 |
| 2024b | 1.00 | 0.261 | 0.105 |
| 2025a | 1.00 | 0.348 | 0.067 |
| 2025b | 1.00 | 0.363 | 0.029 |

\*2021b P₆ and its fractional A₆ = 0.113 (40× other years) are a limb artifact
(B ≈ +18° then); see §6.

**Growth, unnormalised (jet ring, F631N, a/b averaged).** A₁₀: 0.00689
(2023) → 0.00756 (2024) → 0.01012 (2025), +47%; same-day half-difference
≈ 0.0003. Control ring A₆ and A₁₀ fall together over the same years
(corr +0.997), the common-mode signature of changing geometry — the jet ring
does not share it.

**Northern hexagon.** P₆ at +78°: 0.500 (2018a), 0.511 (2019a), 4.5–4.6×
flat. By 2025b the +78° spectrum is flat (all modes 0.076–0.146, peak 1.31×).
The north cannot be tracked from Earth over this window; no claim is made
about hexagon evolution.

## 4. Jet width and λ/W

Gated (drop FQ889N; peak within 4° of jet).

| | W (deg) | W (km) | n | λ (km) | λ/W |
|---|---|---|---|---|---|
| north m=6, 2023 | 2.60 | 2987 | 7 | 14 475 | 4.85 |
| south m=10, 2024 | 2.80 | 3047 | 9 | 18 624 | 6.11 |
| south m=10, 2025 | 1.70 | 1850 | 10 | 18 624 | 10.07 |

The test "λ/W = one constant at both poles" cannot be run: the south is not
in steady state. Converging on the northern value would require W_south to
*grow* to ~3.5°; it narrowed 39% instead. Cross-filter W scatter, gated:
0.33° (2024) → 0.13° (2025); scatter × amplitude falls to 0.44 of its 2024
value while amplitude rose 1.10×, so the tightening exceeds SNR. m=11 at the
same ring shows the same tightening (0.47 → 0.17), so it is a property of the
ring, not the mode.

## 5. Predictions: status after same-day stress tests

### P1 — narrowing. STANDS.
2026: FWHM ≤ 1.8°, cross-filter sd ≤ 0.25°. Falsified by FWHM ≥ 2.4° or
sd ≥ 0.5°.

### P2 — System III drift. DEMOTED to a four-way multi-year test.
Circular-mean φ₀ (5 filters × 2 visits): 25.8° (MJD 60239), 23.4° (60544),
15.4° (60916). Same-day floor 1.09° rms (11 pairs). Hexagon calibration
2018–19: a/b 1.8°, 1.1°; year change −1.7°.

Least-squares drift −0.0141°/day, residual 1.21°. **Alias sweep**
(`stress_p2.py`) over |d| < 0.6°/day: four minima within noise —
+0.4635 (0.18°), −0.1225 (0.67°), −0.0140 (1.21°), +0.3550 (1.72°). All
physically ordinary (0.46°/day = 2.7 m/s at 63°S). Leave-one-out: −0.0079
(2023→24), −0.0215 (2024→25), 2.6σ apart → per-branch error ≈ ±0.007°/day,
±3° on a 2026 phase. The four branches predict 2026 φ₀ = 2.7°, 7.2°,
**10.3°** (lock), 35.6°, each ±3°. "Locked at the hexagon's rate" is one of
four and was wrongly stated as a finding in the first draft.

### P3 — barotropization. WITHDRAWN.
Gated m=10, 2024→2025: FWHM sd 0.33→0.13 (×0.40); tilt sd 0.14→0.17 (flat);
phase sd 0.31→0.58 (loosened). Within 2024, per-filter FWHM vs peak latitude
r = −0.75 (n=6): not independent. 2023 has <3 gated filters, so the reported
2.9°→0.6° phase tightening was never assessable. Surviving observable is P1's
second clause.

### P4 — two-lobe selection. WITHDRAWN.
Claimed lobes at −69° (A₁₀ = 0.021) and −63° (0.008) in 2023. Spectrum at
−69°, 2023b F631N: P₅…₁₃ = .15 .14 .13 .12 .11 **.10** .09 .08 .07 — a smooth
red slope, m=10 *below* flat. At −63° same frame: P₁₀ = 0.29 (2.6×). The −69°
ring sat at 84° emission angle (B = +15°, not the +9° assumed); fractional
amplitude inflated by small x̄. In 2024 the same large marginally-coherent
m=10 (A 18–29×10⁻³, phase sd 4–5°) appears at −78/−80° (emission 86–88°): it
tracks the limb, not a latitude. Cross-filter phase coherence is **not** a
wave fingerprint — it shows only shared image structure.

### P5 — no stratospheric expression. STANDS, restated.
FQ889N a/b self-consistency 5.3° rms (F631N 1.1°; random 10.4°): noisy, not
blind. P₁₀/flat at −63.3°: 0.55, 0.93, 1.79, 0.81, 0.42 (2023a, 2024a, 2024b,
2025a, 2025b) vs F631N 2.41–3.26. Latitude scan −70…−56: no P₁₀ > 2× flat at
any latitude, any epoch, either visit; best 1.8× at −62°, 2024b, which also
had phase locked to the wave filters within 3.3° and a/b within 1.7° — then
2025 fell to 0.4–0.8× with phase off 11–18°. Noted, not claimed. 2026:
P₁₀/flat < 2.0× at every latitude, both visits. Falsified by ≥2.0× in both
visits at a common latitude with a/b < 3° and phase within 4° of the wave
filters.

## 6. Systematics and method errors found

- **a/b same-day phase offset +0.9°**, both poles, all years, all filters.
  Not drift (would be +2°/day, excluded by yearly data). Pipeline, likely the
  assumed rotation period over ~10 h. Cancels in yearly means.
- **B assumed vs derived**: first-draft emission angles were ~6° too
  optimistic.
- **Fractional amplitude in dark rings** inflates every mode. Produced the
  −69° "lobe" and FQ889N's "high amplitude". Use the spectral criterion.
- **Cross-filter phase coherence** proves shared structure, not a wave.
- **Simplex normalisation** (P_m over the band) makes modes compositional:
  one rising mechanically depresses others. Used only for detection, never
  for competition claims.
- **Image noise is red**, not white, across m=5–13 (the −69° spectrum is the
  clean example). The 1/9 flat null is conservative for m=10 and lenient for
  m=5–6.
- **Look-elsewhere**: the P5 latitude scan searches 15 latitudes × 2 visits ×
  3 years; its "best 1.8×" is inflated. The detection at −63°/m=10 is
  confirmatory (published latitude and wavenumber), not a search.
- **North** unmeasurable after ~2023.
- **2021b** limb artifact (A₆ = 0.113).
- **Literature values, corrected after review.** Hexagon drift: reviewer
  reports +0.0129 ± 0.0020°/day in *west*-positive System III (Sánchez-Lavega
  et al. 2014, GRL; paywalled here) with period 10 h 39 m 23.01 ± 0.01 s; my
  "−0.013" was a sign misquote across conventions. Fletcher et al. 2018
  detected a stratospheric hexagonal thermal boundary in 2014–2017 during
  late northern spring and state earlier data could not tell whether it had
  always been present; "only grew a stratospheric layer as summer approached"
  overstated it.

## 7. GIGI's role, precisely

`curvature::scalar_curvature` = mean over fields of variance/range², with
`variance()` = m2/count and `range()` = max−min: both symmetric in the record
multiset. Longitude order never enters. A 1000-shuffle test moved K by
5.6×10⁻¹⁷ while A₁₀ collapsed. K is blind to wavenumber by construction.

Restructure: base = (year, filter), fiber = A₅…A₁₃, so m is carried by field
identity. Then var/range² per field is a shape statistic bounded by Popoviciu
(≤ 0.25; bimodal → 0.25, uniform → 1/12, cluster + outlier → 0). Hypothesis:
a regime transition would drive the m=10 field toward 0.25. Result: 0.054
(jet), 0.031 (control); nothing on any ring above 0.11. Null, and
under-powered — three post-transition time points cannot show bimodality.
The "cluster + outlier" readings on m=5–7 at the jet ring were GIGI correctly
flagging the 2021 artifact from distribution shape alone.

The spectral gap correctly returned `Undefined { components: 1296 }` on the
unindexed (lat, lon) bundle rather than a confident 0.0 — the F-6 refusal
from the same day's durability work, firing on real data.

The Davis capacity C = τ/K on the (lat, lon) bundle is ≥ 2π by algebra
(K ≤ 0.25 ⇒ C ≥ 4τ… with the bridge K_dc = 2√K ≤ 1 ⇒ C ≥ 2π > C_crit = π),
so "C above critical" there is an identity, not a measurement.

## 8. Open questions

1. What breaks the P2 alias before 2029? Any Saturn imaging between OPAL
   epochs at ≥0.5° longitude resolution on the 63°S ring — Cassini is gone;
   JWST/NIRCam or ground-based AO could.
2. Is the m=11 companion (P₁₁ ≈ 0.85 P₁₀ in 2025) a sideband of non-uniform
   vertex spacing, a ring-average artifact of the finite meridional width, or
   a second mode?
3. Is 2024b's FQ889N touch (1.79×, phase-locked) a fluctuation or a transient?
4. Why did the width narrow 39% in one year — barotropic saturation, or a
   change in the jet itself? Requires winds, which reflectivity maps lack.
5. Verify B against Horizons; verify the two literature values.

## 9. Reproduce

```
curl -O https://archive.stsci.edu/hlsps/opal/cycle32/saturn/hlsp_opal_hst_wfc3-uvis_saturn-2025b_f631n_v1_globalmap.fits
python series.py      # §3 time series
python amplitudes.py  # bundle input
python jetwidth.py    # §4
python phase.py       # §5 P2 baseline
python stress_p2.py … stress_p5.py
cargo run --release --example saturn_mode_bundle -- mode_amplitudes.json
```

## 10. After outside review — dispositions (2026-09-03, same day)

An outside model was given §1–9 and `REVIEW_PROMPT.md`. Its claims were
checked against Horizons and the paper's abstract (Europe PMC, PMID
42685223); everything reachable confirmed. Full paper and 2014 GRL paywalled
here; those specifics are *as reported by reviewer*.

| item | before | after |
|---|---|---|
| P1 | jet narrowed 2.8→1.7°, sd 0.13° | **withdrawn as jet claim.** `jetwidth.py` measures the m=10 photometric envelope, not u(φ). Paper: wind jet ≈2.8° FWHM, 116 m/s, unchanged since 1981. 0.13° is sub-pixel, not reportable. −45° control narrows too (few samples). Envelope narrowing kept as an uncalibrated observation. |
| P2 | four aliases, lock predicts 10.3° | **settled by literature; lock falsified.** Paper: eastward 2.5 m/s = +0.39…+0.42°/day in map convention. Corrected sweep (±1°/day, endpoints, 1.5° crit): five solutions; +0.4634 (rms 0.167) is nearest to published *and the best fit*; −0.0141 (rms 1.203) is the worst. +0.355 fails at 1.71°. Vertices oscillate 4.6–8.4°, 32 d; ±3° windows meaningless. Hexagon drift sign misquoted (+0.0129 west-positive). |
| P3 | withdrawn | unchanged |
| P4 | withdrawn | holds under Horizons B (−69° at 81.8° emission in 2023; 2024 limb artifacts at ~82–84°). |
| P5 | no m=10 in FQ889N | **withdrawn — demonstrated false negative.** Paper (per reviewer): decagon in FQ889N near 58.8–60.5°S, layered ~10 mbar–2 bar. Pipeline gives 0.4–0.6× flat at −59° in 2025. Detector missed a known positive; fixed-latitude spectral statistic is insensitive to a latitude-displaced meander. Red refit m^−0.778 scales m=10 by 1.1565 (baselines only). Illustrative red-Rayleigh null: 90-trial FPR ≈ 0.95 for ≥1.79×. |
| B | +15.1, +8.5, −7.3 (coverage) | +12.79, +3.83, −3.26 (Horizons) |
| a/b +0.9° | rotation period | not (needs 92–109 s error); navigation/registration |
| λ/W | test of mode selection | not standard theory; paper's sims seeded with m=10; Antuñano et al. 2015: both polar jets meet similar criteria |
| Fletcher 2018 | "only grew … as summer approached" | detected 2014–2017; earlier data inconclusive |

**Zero of five stand.** Surviving as observations: continuum detection at
63.3°S; 2021 null at 0.99 coverage vs 2023+ presence; continuum m=10 growth
2023→2025; annual phases consistent with the published drift on the correct
branch; GIGI curvature as outlier QA in the restructured bundle.

**Root cause.** The paper was flagged as a ten-minute check at the start and
not read. Five of fifteen review findings (P2 drift, P5 latitude, jet FWHM,
32-day oscillation, northern vortex) were in it.

**Additional data (reviewer):** PVOL/ALPO vertex tracking Jun–Oct 2025;
Calar Alto PlanetCam 29 Aug–1 Sep 2025; **HST GO-18102, 16–17 Sep 2025**
(non-annual epoch, possibly in MAST); VLT/VISIR 115.283U.001 (thermal, no
vertex resolution); HST Cycle 33 OPAL GO-17995 next.

**Next (not started; see `PREDICTIONS.md` § Next):** ridge tracker →
injection/recovery → raw-exposure fit → wind profile → vertex-motion fit with
vortex coupling → Rayleigh–Kuo eigenanalysis → GIGI bundle for QA/CADENCE.

## 11. Round 2 — methods built, calibrated, and fired (same evening)

Full ledger: `PREDICTIONS_ROUND2.md`. Outputs: `ridge_output.txt`,
`injection_output.txt`, `wind_output.txt`, `stability_output.txt`, and the
JSON files.

**Ridge tracker** (`ridge_tracker.py`). Window −70…−52°. Per-column gain
normalisation; template = zonal-mean latitude profile; per-column
cross-correlation against the template over ±3° with parabolic sub-pixel
refinement gives δφ(λ); A_m = 2|Σ δφ e^(−imλ)|/n in degrees; P_m over m=5–13
vs 1/9. Gates: F763M 2025 3.54×; **FQ889N 2025 2.53/2.27× at ridge −58.7°**
(the round-1 false negative, recovered at the paper's latitude); 33 control
maps at −45°: max 1.94×, zero false m=10.

**Injection/recovery** (`injection_recovery.py`). Null = blocked longitude
resampling (9° blocks, one permutation for all rows). First version
phase-randomised rows independently, which preserves per-row m=10 power;
the tracker summed a random-walk meander and "fired" 25–54% — not a null.
Corrected: FPR 3/96 at 2.0× (null max 1.93–2.11); completeness 50% at
A≈0.10–0.15° deep / ≈0.15° FQ889N, 100% at ≥0.20° deep / ≥0.30° FQ889N;
recovered/injected 0.89–1.06; phase rms 1.1–1.5° at 0.20°; removal of the
recovered m=10 meander kills every real detection (0.00–0.02×).

**Geometric amplitude is flat.** A₁₀ ≈ 0.20 ± 0.02° (~200 km) in every deep
filter, 2023–2025. The round-1 "+47%" was albedo contrast at fixed latitude.

**Wind profile** (`wind_profile.py`). a/b pairs (9.4–11.1 h ≈ 0.9 rotation),
high-pass m<15 plus a comb notch at multiples of 10 (the stationary
polygon's harmonics pulled the first run to zero shift in the wave band),
8 overlapping Hanning-windowed longitude segments, circular cross-correlation
with parabolic sub-pixel. Result: jet at 108–137 m/s near −61° in 3 of 6
2024–25 pairs, errors 10–60 m/s, FWHM 1.0–4.0°. Partial recovery of the
published 116 m/s / 60.5°S; unusable for U″. Raw frames required.

**Stability** (`stability_modes.py`). Barotropic β-plane, Dirichlet ends at
±9°, 0.1° grid, k = m/r(φ₀), K² = k² + L_D⁻², β = 2Ω cos φ₀ / M(φ₀),
(U−c)(ψ″−K²ψ) + (β−U″)ψ = 0 → [diag(U)L + diag(q)]ψ = cLψ, growth = k·Im(c).
First run had the PV term with the wrong sign (growth ~0.002/day); fixed.
Second run thrashed BLAS threads on 180×180 matrices (6.5 cores, 40 min,
15% done); pinned to one thread, 60 sensitivity draws.

South (116 m/s, 2.8°, −60.5°): fastest m=17/17/16/15/14 for L_D = ∞/4000/
3000/2000/1500 km (0.96→0.38 /day); L_D=1000: m=5 at 0.02/day; ≤700 stable.
Sensitivity mode at L_D=1500: **m=11**; at 1000: m=11 (m=10 18%).
North (120 m/s, 2.8°, +77.5°): fastest m=7/7/7/6/6 for L_D = ∞/4000/3000/
2000/1500 (0.97→0.32 /day); ≤1000 stable. Sensitivity mode **m=6** at
L_D=1500–3000 (35–42%). Width sweep at 77.5°N: FWHM 2.0°→m=10, 2.4→8,
2.8→7, 3.2→6, 3.6→5–6, 4.0→5. r_south/r_north = 2.23; β_north = 0.44 β_south.

**GIGI over ridge amplitudes** (`ridge_amplitudes.json` →
`saturn_mode_bundle`): 31 sections, K=0.040; m=10 mean 0.189°, var/range²
0.026 (tight cluster + outlier); no bimodality. QA only.

**Predictions registered:** P-R2-1 A₁₀(2026) = 0.20 ± 0.05°; P-R2-2 FQ889N
P₁₀/flat ≥ 2.0 in ≥1 visit; P-R2-3 GO-18102 (2025-09-16) ridge phase 6.1°
(published branch) vs 33.2° (locked), ±6°, conditional on a raw-frame
pipeline; P-R2-4 northern jet FWHM 2.6–3.4°; P-R2-5 L_D ≤ 1500 km at the
wave level or the decagon is forced.
