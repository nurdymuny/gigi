# Registered predictions — Saturn's south-polar decagon

Five falsifiable claims about the next public OPAL Saturn epoch (expected
Aug–Sep 2026, HST Cycle 33, DOI 10.17909/T9G593). Every number below was
measured from public 2023–2025 maps before the 2026 data existed; the scripts
that produce them are in this directory. Anyone with `curl` can check them.

Registered 2026-09-03. B. R. Davis, Davis Geometric.

> **Status after outside review, same day — see the final section.** None of
> the five survives as registered. The review's decisive input was the
> discovery paper itself (Sánchez-Lavega et al., *Sci. Adv.*, Sept 2026, PMID
> 42685223), which I had flagged as a ten-minute check at the start of the day
> and never read. The sections below are left as written so the record shows
> what was claimed and when; the dispositions are at the end.

---

## P1. The jet is still narrowing, or has settled narrow

**Measured.** Latitudinal FWHM of the m=10 envelope, five wave-carrying filters:
2.80° in 2024 (cross-filter sd 0.65°) → 1.70° in 2025 (sd 0.16°).

**Predict.** 2026 FWHM ≤ 1.8°, cross-filter sd ≤ 0.25°.

**Falsified if** FWHM ≥ 2.4° or sd ≥ 0.5° — the wave is oscillating, not settling.

---

## P2. Is the decagon locked to System III? A four-way test

**Retraction first.** The first version of this file stated as a finding that
the decagon drifts at −0.014°/day, "the hexagon's rate", and that the smaller
aliases were excluded. A brute-force sweep of the residual against assumed
drift (`stress_p2.py`) shows that is false. Three annual snapshots against a
36° period cannot pin a drift; the hexagon's rate was measured with Voyager
and Cassini sampling days apart. Four drifts fit within the noise floor, and
two fit better than the one I claimed:

| drift (°/day) | residual | 2026 φ₀ it predicts |
|---|---|---|
| +0.4635 | 0.18° | 2.7° |
| −0.1225 | 0.67° | 7.2° |
| **−0.0140** | 1.21° | **10.3°** |
| +0.3550 | 1.72° | 35.6° |

All four are physically plausible — at 63°S even +0.46°/day is 2.75 m/s.
"Locked to System III" is one of four hypotheses the data allows, not a result.

**Method (unchanged, and it is sound).** The complex zonal coefficient
z = Σ(x−x̄)e^(−imφ) gives the longitude of a brightness maximum,
φ₀ = −arg(z)/m, mod 36°. Calibrated on the hexagon in 2018–19: same-day scatter
1.5°, year-over-year change −1.7° — consistent with its published near-zero
drift, though the same alias caveat applies to that calibration.

**Measured.** Circular mean of φ₀ over 5 filters × 2 visits:

| epoch | MJD | φ₀ (deg, mod 36) | n |
|---|---|---|---|
| 2023 | 60239 | 25.8 | 7 |
| 2024 | 60544 | 23.4 | 9 |
| 2025 | 60916 | 15.4 | 10 |

Same-day noise floor 1.09° rms (11 a/b pairs). Leave-one-out shows the two
segments differ (−0.008 then −0.022°/day, a 2.6σ difference), so the honest
error on any branch is about ±0.007°/day, not the ±0.003 a within-epoch
bootstrap gives. Over 361 days that is ±2.5°, so **±3° on each prediction**.

**Predict.** The four aliases send the 2026 phase to four different places,
each ±3°: 2.7°, 7.2°, 10.3°, 35.6° (mod 36). The 2026 epoch discriminates.
The locked-to-System-III hypothesis specifically predicts **10.3° ± 3°**.

**Falsified if** — for the lock hypothesis — the 2026 phase lands outside
10.3 ± 4°. A 6° window on a 36° circle is hit by chance 17% of the time; with
the three competitors predicting elsewhere, a hit at 10.3 favors the lock but
does not prove it. Four epochs will cut the alias set sharply; five will
likely close it. This is a multi-year measurement and the ledger should say so.

---

## P3. Vertical coherence — RETRACTED, folded into P1

The first version of this file claimed "three independent measures of
cross-filter agreement all tightened 2024→2025" and called it a measured
barotropization. `stress_p3.py` tested each word. The table it was built on
mixed in low-contrast filters whose envelope fits were unreliable. Gated to
filters whose m=10 peak is at least 6× the local background:

| measure (m=10, gated) | 2024 | 2025 | change |
|---|---|---|---|
| FWHM scatter across filters, sd | 0.33° | 0.13° | ×0.40 — tightened |
| peak-latitude scatter (tilt), sd | 0.14° | 0.17° | ×1.21 — flat |
| phase scatter within a visit, sd | 0.31° | 0.58° | ×1.83 — **loosened** |

Tilt never tightened; the 0.45° I reported for 2024 was two low-contrast blue
filters. Phase went the other way. 2023 has fewer than three filters that pass
the gate at all, so the 2.9°→0.6° phase "tightening" I reported for 2023→2024
is not assessable and should not have been in the table.

Nor were the measures independent: within 2024, per-filter FWHM and per-filter
peak latitude correlate at r = −0.75 (n=6). And the ride-along m=11 mode does
the same thing as m=10 on every measure, so the width tightening is not
specific to the decagon.

**What survives.** Cross-filter width scatter genuinely tightened, and it
survives SNR-normalisation (scatter × amplitude falls to 0.44 of its 2024
value while amplitude rose only 1.10×). That is one observable, and it is the
second clause of P1. No separate P3 claim remains.

**Predict.** Nothing beyond P1.

---

## P4. The poleward lobe — RETRACTED. There was no lobe.

The first version of this file said the m=10 signal had two lobes in Oct 2023,
a strong one at −69° and a weak one at −63°, and that the decagon was
therefore *selected* at 63°S from a two-lobe precursor. `stress_p4.py` ran
three geometric tests. The third killed it outright.

**Spectrum at −69°, 2023b F631N** (flat noise = 0.111):

| m | 5 | 6 | 7 | 8 | 9 | **10** | 11 | 12 | 13 |
|---|---|---|---|---|---|---|---|---|---|
| −69° | 0.15 | 0.14 | 0.13 | 0.12 | 0.11 | **0.10** | 0.09 | 0.08 | 0.07 |
| −63° | 0.09 | 0.10 | 0.08 | 0.05 | 0.07 | **0.29** | 0.23 | 0.08 | 0.01 |

At −69° the spectrum is a smooth 1/f slope; m=10 sits *below* the flat level.
There is no wave there. At −63° m=10 stands 2.6× above flat. One ring has a
decagon; the other has red noise.

**Why it fooled me.** Two compounding errors:

1. The sub-Earth latitude in Oct 2023, derived from the maps' own coverage
   boundary, was **+15°**, not the +9° I had assumed. That put −69° at **84°
   emission angle** — essentially the limb — and even the real decagon at −63°
   was at 78°.
2. The fractional-amplitude estimator A = |z|/(n·x̄) divides by the ring's
   mean brightness. In a dark limb ring every mode's fractional amplitude is
   inflated. The "21×10⁻³ at −69°" was large because x̄ was small, not because
   m=10 was.

The emission-angle scan confirms the mechanism: in 2024 the same large,
marginally coherent m=10 (18–29×10⁻³, phase sd 4–5°) appears at −78° and −80°,
where the emission angle is 86–88°. It tracks the limb across years, not a
fixed latitude. In 2025, with the south tilted toward Earth, it is gone from
the cap entirely and only the −63° decagon remains.

**A method error to carry forward.** I used cross-filter phase coherence as a
fingerprint for "wave". It is not. It proves only that the same longitudinal
structure is present in every filter, which any real image feature satisfies —
haze banding, a limb-darkening residual, a seam. The wave test is the
*spectrum*: a peak at one m standing well above the flat level. Every claim in
this directory that rested on coherence alone has been re-checked against that
criterion; P5 survives because its reference ring (−63°) has a spectral peak.

**The decagon was born at 63°S, as reported.** Nothing in the 2023 maps
supports a precursor elsewhere.

**Predict.** Nothing. P4 is withdrawn.

---

## P5. No stratospheric expression yet (and when to expect one)

**Stress-tested (`stress_p5.py`) and it stands — on better evidence than it
was first given.** The original argument was that FQ889N's m=10 phase is
random relative to the wave filters (rms 12.1° vs 10.4° for noise). After P4
that argument is not enough on its own: a filter too noisy to measure any
phase would also read "random". So two further tests.

*Can FQ889N measure a phase?* Same-day a-vs-b at −63.3°: **5.3° rms**
(F631N: 1.1°; random: 10.4°). Noisier than the deep filters, but not blind.
The null is informative.

*Is there a spectral peak?* This is the wave test, and it is the one that
should have carried P5 from the start.

| epoch | F631N m10/flat | FQ889N m10/flat | FQ889N peak mode |
|---|---|---|---|
| 2023a | 2.41× | 0.55× | m=6 |
| 2024a | 2.74× | 0.93× | m=7 |
| 2024b | 2.31× | 1.79× | m=7 |
| 2025a | 3.13× | 0.81× | m=11 |
| 2025b | 3.26× | 0.42× | m=7 |

A latitude scan of FQ889N over −70…−56 finds no m=10 above 2× flat at any
latitude, any epoch, either visit (best: 1.8× at −62, 2024b). The "position
shifts with wavelength" possibility does not rescue a detection. **Not
detected above noise** is confirmed by the spectrum, not just the phase.

**The one epoch that came close.** 2024b: 1.79× flat, phase locked to the
wave filters within 3.3°, self-consistent with 2024a to 1.7°. Then 2025
returned to 0.4–0.8× with phase off by 11–18°. Below threshold and not
monotonic — either a fluctuation or a transient stratospheric touch. Noted,
not claimed.

**Context.** The hexagon developed its stratospheric expression only as
northern summer approached (Fletcher et al. 2018, Cassini CIRS). Southern
insolation at 60°S peaks in 2032.

**Predict.** 2026 FQ889N shows no m=10 spectral peak: m10/flat < 2.0× at
every latitude in −70…−56, in both visits.

**Falsified if** m10/flat ≥ 2.0× in both visits at a common latitude, with
a-vs-b phase agreement < 3° and phase within 4° of the wave filters — in which
case the wave has reached the stratosphere six years ahead of the hexagon's
seasonal template, and P5 becomes the most interesting result here.

---

## Systematics I know about

- **Same-day a/b visits differ by a consistent +0.9° in phase**, at both poles,
  every year, every filter. That is not drift (it would imply +2°/day, which the
  year-to-year data rules out). It is a pipeline systematic between visits —
  most likely the assumed rotation period over ~10 h. It cancels in the yearly
  means used above.
- **The north cannot be tracked.** Its mode spectrum is flat noise by 2025 as
  the hemisphere goes into shadow. Every hexagon number here is 2018–19 or
  2023, not a trend.
- **The 2023 epoch had worse southern geometry than I assumed.** Sub-Earth
  latitude derived from the maps' coverage boundaries: +15.1° (2023), +8.5°
  (2024), −7.3° (2025). I had guessed +9, +4, −2. Every emission-angle
  statement in the first draft was too optimistic by about 6°.
- **Fractional amplitude is unreliable in dark rings.** A = |z|/(n·x̄) inflates
  every mode when x̄ is small — at the limb, and in the strong-methane band
  FQ889N. Two "signals" in the first draft (the −69° lobe; FQ889N's "high"
  amplitude) were this. The wave criterion is the spectral peak over the flat
  level, never fractional amplitude alone, and never cross-filter phase
  coherence alone.

## After outside review — 2026-09-03, same day

An outside model was given `SUMMARY_TECHNICAL.md` and `REVIEW_PROMPT.md` and
told to break what was left. It did. Its load-bearing claims were then
checked here: JPL Horizons (sub-Earth latitudes) and the paper's abstract via
Europe PMC (drift, oscillation, latitudes, jet speed, vortex) confirm it on
every point I could reach; the full paper and the 2014 GRL hexagon paper are
paywalled from this machine and those specifics are carried as *reported by
the reviewer*.

### P1 — WITHDRAWN as a jet claim; kept as an uncalibrated envelope observation

`jetwidth.py` never measured a jet. It measured the FWHM of the photometric
m=10 amplitude vs latitude — a wave-contrast envelope. Jet width needs a wind
profile u(φ) from cloud tracking, which reflectivity maps do not give. The
paper (per reviewer) measures the 2025 wind jet at ≈2.8° FWHM, 116 m/s peak,
shape and strength unchanged since 1981. My "1.7°" is a different quantity.

Two further faults. The 0.13° cross-filter scatter is below the 0.2° pixel and
came from non-interpolated half-max crossings; it is not reportable. And a
quick F631N check on the −45° control ring narrows in the same direction
(2.2° → 1.6–1.8°, too few gated samples to conclude) — the shared-morphology /
SNR explanation the m=11 ride-along already suggested is not excluded.

Status: "the photometric m=10 envelope narrowed 2024→2025" is an observation
with an unexcluded confound. No prediction is registered from it until an
injection/recovery calibration (§ Next, item 2) says what the estimator can
and cannot see.

### P2 — SETTLED by the literature, and the lock is falsified

The paper reports the decagon **moves eastward at 2.5 m/s**. In this map's
east-increasing column convention that is +0.39 to +0.42°/day (60.5°S to
63.3°S). Re-running the alias sweep with the endpoint bug fixed and the 1.5°
criterion actually applied gives five solutions in ±1°/day:

| drift (°/day) | rms | |
|---|---|---|
| **+0.4634** | **0.167** | nearest branch to the published drift, and the best fit of all |
| −0.5999 | 0.363 | endpoint minimum the old script could not see |
| −0.1224 | 0.672 | |
| +0.9409 | 0.868 | |
| −0.0141 | 1.203 | the "locked" branch I claimed — the worst of the five |

The +0.355 branch I listed fails the criterion (1.71°). My phase data
reproduce the published drift once the alias is broken by dense 2025 ground
tracking (PVOL/ALPO, June–October). The decagon is **not** locked to System
III at the hexagon's rate; it drifts at ~2.5 m/s where the hexagon drifts at
~0. Vertices also oscillate 4.6°–8.4° with a 32-day period, so the ±3°
windows I proposed were meaningless regardless of branch.

Sign note: the reviewer reports the 2014 GRL hexagon drift as +0.0129 ±
0.0020°/day in *west*-positive System III. My −0.013 was a misquote; the
magnitude stands, the sign flips with convention.

### P3 — withdrawn earlier; unchanged.

### P4 — withdrawn earlier; the retraction holds under corrected geometry

Sub-Earth latitudes from Horizons: **+12.79° (2023), +3.83° (2024), −3.26°
(2025)**. My coverage-boundary estimates (+15.1, +8.5, −7.3) were off by
+2.3, +4.7, −4.0 — biased by the 50% threshold on a mosaic. The −69° ring in
2023 was at 81.8° central-meridian emission, not 84°; still extreme, and the
−69° spectrum still has no m=10 peak. The 2024 limb artifacts at −78/−80° were
at ~82–84°, not 86–88°. The 2024→2025 geometry improvement at −63° was 7°, not
16° — geometry remains a confounder for P1 but was overstated as one.

### P5 — WITHDRAWN as a demonstrated false negative

The paper places the decagon at planetographic 58°–63°S and (per reviewer)
identifies it in FQ889N near 58.8°–60.5°S in 2025, as part of a vertically
layered structure from ~10 mbar to ~2 bar. My fixed-latitude spectral
statistic returns 0.4–0.6× flat at −59° in both 2025 maps. **The detector
missed a known positive.** "Not detected above noise" was a statement about
my estimator's sensitivity to a latitude-displaced meander, not about Saturn.
Red-noise refit (P_m ∝ m^−0.778) raises every m=10 "×flat" by 1.1565 —
baselines, not p-values — and under an illustrative independent-Rayleigh red
null the 15-latitude × 6-visit scan's best value (1.79×) has a family-wise
false-positive rate near 0.95. Neither rescues nor condemns; the point is the
statistic was never calibrated on a positive.

### Cross-cutting corrections

- **I did not read the paper.** Named as a ten-minute check at the start;
  skipped. P2's drift, P5's FQ889N latitude, the 2.8° jet width, the 32-day
  oscillation, and the northern vortex were all in it.
- **a/b +0.9° offset** is not a rotation-period error (would need 92–109 s,
  absurd against 10 h 39 m 22.4 s); it is navigation / registration / mosaic
  phase. Cancels in yearly means; cannot pick an alias.
- **λ/W = constant is not standard theory.** Mode selection depends on the
  full profile, β, L_D, stratification and shear. The paper's simulations
  were *seeded* with m=10 (or ten perturbations); they do not predict it.
  Antuñano et al. (2015) found both polar jets satisfy similar instability
  criteria without explaining why only one had a polygon.
- **Fletcher et al. 2018** detected a stratospheric hexagonal thermal boundary
  in 2014–2017 during late northern spring; the paper says earlier data lacked
  the signal to tell whether it was always there. "Only grew a stratospheric
  layer as summer approached" was too strong.
- **Additional data already exist**: PVOL/ALPO vertex tracking June–Oct 2025;
  Calar Alto PlanetCam 29 Aug–1 Sep 2025; **HST GO-18102, 16–17 Sept 2025** —
  a non-annual OPAL-quality epoch that may already be in MAST; VLT/VISIR
  thermal (did not resolve vertices).

### What actually survives the day, as observations

- The decagon is at 63.3°S in continuum filters, n=29, sd 0.65° — consistent
  with the published 58–63°S.
- It was absent at 63°S in 2021 at 0.99 coverage and present from 2023 at
  1.00 — real, not a viewing artifact.
- Its continuum m=10 amplitude rose 2023→2025, consistent with the paper's
  "evolving phenomenon".
- Its annual F631N phases are consistent with the published eastward drift
  once the alias is broken.
- GIGI's curvature verb is blind to longitude order by construction; in the
  restructured (year, filter) bundle it correctly flagged the 2021 limb
  artifact from distribution shape. That is QA, not detection.

Zero of five registered predictions stand. Five stress-test scripts and one
outside review are in this directory so the failures are as reproducible as
anything else here.

## Next — reviewer's replacement methods, in the order I would run them

Not started. Each is a scope decision.

1. `ridge_tracker.py` — detect the wave as a meandering ridge φ(λ) in a polar
   map, fit A_m as geometric displacement in km, hierarchically across
   filters. Gate: must recover the published F763M and FQ889N positives and
   return null at −45°.
2. `injection_recovery.py` — plant m=10 ridges into phase-randomised maps,
   run the full search, report detection probability, look-elsewhere FPR,
   width bias vs emission angle. Until this exists no null here means
   anything.
3. `raw_exposure_check.py` — fit the six raw exposures jointly; planetary
   structure is fixed in System III, seams follow exposure boundaries, limb
   residuals follow emission angle.
4. `wind_profile.py` — cloud-tracking u(φ) from same-filter pairs one rotation
   apart; validate on the published 116 m/s / 2.8° before using it.
5. `vertex_motion.py` — fit drift + 32-day oscillation + per-vertex
   amplitude + Red-Spot-relative phase on PVOL/ALPO/PlanetCam/HST vertices.
6. `stability_modes.py` — Rayleigh–Kuo eigenproblem on the measured u(φ) over
   m=2…20, L_D=500…4000 km; the only route to *why ten and why six* that does
   not insert ten disturbances by hand.
7. GIGI bundle over (epoch, visit, filter, latitude, sector) with explicit
   ordering fields and the mode vectors in the fiber — for outlier QA and
   CADENCE on sampling limits, never for detection.

The reviewer's closing line is the right one: the result worth having is not
an independent re-detection. It is a calibrated vertical structure, a test of
vortex forcing, or an eigenanalysis that predicts m=10 unprompted.

P2 is not yet a result. It is the most valuable *question* here — a second
polar polygon locked to the deep rotation would constrain the interior, not
just the atmosphere — but three annual snapshots cannot answer it, and the
first draft of this ledger said they could. The alias sweep in `stress_p2.py`
is the check that caught it. Each further OPAL epoch removes aliases; the
question likely closes at five.
