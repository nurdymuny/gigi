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

## P3. Vertical coherence — the wave has become barotropic and stays so

Three independent measures of agreement across the five filters (≈ five
altitudes) all tightened between 2024 and 2025:

| measure | 2023 | 2024 | 2025 |
|---|---|---|---|
| FWHM scatter across filters, sd | — | 0.65° | 0.16° |
| peak-latitude scatter (tilt), sd | 0.41° | 0.45° | 0.15° |
| phase scatter within a visit, sd | 2.9° / 1.5° | 0.62° / 0.43° | 0.71° / 0.51° |

Every altitude the wave occupies now shares one width, one latitude, one phase.

**Predict.** 2026: tilt sd ≤ 0.25°, phase sd ≤ 0.8°, FWHM sd ≤ 0.25°.

**Falsified if** any one of the three re-broadens past twice its 2025 value.

---

## P4. The poleward lobe stays dead

**Measured.** In Oct 2023 the m=10 signal had two lobes with an amplitude node
at −65°: a strong one at −69/−70° (F631N amp 21×10⁻³) and a weaker one at
−62/−63° (8×10⁻³). Their phases differ by ~5° (mod 36) and each is coherent
across filters. By Aug 2024 the −69° lobe had collapsed > 10× (amp < 2×10⁻³)
and gone phase-incoherent across filters; the −63° lobe grew and won.

This reframes the discovery. The decagon did not appear at 63°S; it was
*selected* there from a two-lobe precursor. Whether the −69° lobe was a second
wave or the poleward half of one meridionally-tilted wave is open — but either
way, it lost.

**Caveat, stated plainly.** In Oct 2023 the sub-Earth latitude was about +9°,
so −69° sat near the limb at high emission angle. A limb artifact would be
low-m (one bright edge), not a phase-coherent m=10 across five bandpasses — but
I have not ruled out the OPAL limb-darkening correction imprinting structure.
Confidence: medium.

**Predict.** 2026 m=10 amplitude at −69° < 3×10⁻³ with cross-filter phase sd > 5°.

**Falsified if** a coherent m=10 (phase sd < 2°) reappears at −69°.

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
- **The 2023 epoch had poor southern geometry.** P4 inherits that.

## What would make this a paper

P3 + P1 together are a measured barotropization, on three independent
observables, and they stand. P4 is the story nobody has told, with its limb
caveat carried. P5 is a clean binary with a date.

P2 is not yet a result. It is the most valuable *question* here — a second
polar polygon locked to the deep rotation would constrain the interior, not
just the atmosphere — but three annual snapshots cannot answer it, and the
first draft of this ledger said they could. The alias sweep in `stress_p2.py`
is the check that caught it. Each further OPAL epoch removes aliases; the
question likely closes at five.
