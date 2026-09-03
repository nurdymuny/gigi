# Registered predictions — Saturn's south-polar decagon

Five falsifiable claims about the next public OPAL Saturn epoch (expected
Aug–Sep 2026, HST Cycle 33, DOI 10.17909/T9G593). Every number below was
measured from public 2023–2025 maps before the 2026 data existed; the scripts
that produce them are in this directory. Anyone with `curl` can check them.

Registered 2026-09-03. B. R. Davis, Davis Geometric.

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

**Correction first.** Earlier notes in this directory said the wave was
"absent" in FQ889N and had a "ceiling". That overstated it. FQ889N shows m=10
fractional amplitude, but its phase relative to the wave filters is random —
rms 12.1° against 10.4° expected for pure noise (n=5 epochs; two of five
agree, three do not). The honest statement is **not detected above noise**.

**Context.** The hexagon developed its stratospheric expression only as
northern summer approached (Fletcher et al. 2018, Cassini CIRS). Southern
insolation at 60°S peaks in 2032.

**Predict.** 2026 FQ889N phase remains random relative to the wave filters:
rms phase difference > 6°.

**Falsified if** FQ889N phase locks to within 3° rms across both visits — in
which case the wave has reached the stratosphere seven years ahead of the
hexagon's seasonal template, and P5 becomes the most interesting result here.

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

## What would make this a paper

P1 stands: the jet narrowed 2.9°→1.7° on contrast-matched filters, and its
cross-filter width scatter fell beyond what SNR explains. P3 does not stand;
it was one observable dressed as three. P4 does not stand; the "second lobe"
was red noise in a limb ring, and the decagon was born where the discovery
team said it was. P5 is a clean binary with a date, and has not yet been
stress-tested.

Of five predictions registered on 2026-09-03, two survive a same-day attempt
to break them, one is demoted to a multi-year test, two are withdrawn. The
stress-test scripts are in this directory so the failures are reproducible
alongside the survivors.

P2 is not yet a result. It is the most valuable *question* here — a second
polar polygon locked to the deep rotation would constrain the interior, not
just the atmosphere — but three annual snapshots cannot answer it, and the
first draft of this ledger said they could. The alias sweep in `stress_p2.py`
is the check that caught it. Each further OPAL epoch removes aliases; the
question likely closes at five.
