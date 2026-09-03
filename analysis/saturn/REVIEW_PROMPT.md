# Prompt for an outside review (GPT or any capable model)

Paste everything below the line, and attach or paste `SUMMARY_TECHNICAL.md`
from the same directory. If the model can run code, also attach the six
Python scripts; the FITS files are public at the URL in the summary.

---

You are reviewing a one-day analysis of Saturn's newly discovered south-polar
decagon, done from public Hubble/OPAL maps. The attached technical summary
has the data, methods, numbers, five registered predictions, and the same-day
stress tests that already retracted two of them and demoted a third.

**Your job is to break what's left.** Do not summarise, do not praise, do not
soften. The author has already retracted three claims today and would rather
lose two more now than publish them. Default to "this is wrong" and make the
analysis earn every survivor. If you cannot find a flaw in something, say so
in one sentence and move on.

Work through these in order. For each, give a verdict (holds / fails /
cannot tell), the reason, and what test or data would settle it.

### A. The two survivors

1. **P1 (jet narrowing 2.8° → 1.7°, cross-filter scatter 0.33° → 0.13°).**
   The FWHM finder uses the 25th percentile of the scan window as background
   and walks to half-max. Could a change in the *background* (limb darkening
   as B moved from +8.5° to −7.3°) narrow the measured FWHM without the jet
   narrowing? Could the m=11 companion, which shows the same tightening,
   indicate the whole ring's SNR improved rather than any jet property?
   Propose a null test on a latitude with no wave.

2. **P5 (no m=10 in FQ889N).** The null is "flat = 1/9 across m=5–13" but the
   author notes image noise is red. Re-derive the significance of the 2024b
   value (1.79× flat, phase-locked within 3.3°) against a red-noise null. Is
   the 2.0× threshold justified, and what is the false-positive rate of the
   15-latitude × 6-visit scan under that null? Could FQ889N's low a/b
   self-consistency (5.3°) mean the *wave* is there but the filter's phase is
   scrambled by haze, making the null uninformative after all?

### B. The demoted one

3. **P2 (System III drift, four aliases).** Check the alias arithmetic: with
   gaps of 305 and 372 days and period 36°, list every drift in ±1°/day that
   fits all three epochs within 1.5°. The author found four in ±0.6°/day; is
   the count right, and how many more appear out to ±1°? Is any of them
   physically preferred by known zonal wind speeds at 63°S (cite the profile
   you use)? Is the hexagon calibration (2018–19, one year gap, ±1.5°)
   subject to the same alias and therefore not a calibration at all?

### C. Geometry and pipeline

4. **Sub-Earth latitude B** was derived from each map's coverage boundary:
   +15.1° (2023-10-22), +8.5° (2024-08-22), −7.3° (2025-08-29). Check these
   against JPL Horizons or an ephemeris. If they are off by more than 2°,
   which conclusions change? (The −69° retraction in P4 leaned on B = +15°.)

5. **Emission angle** was taken as |φ − B|, the central-meridian value, on
   the argument that a mosaic images each longitude near its CM passage. Is
   that argument sound for OPAL's six-visit mosaics? What is the actual
   emission-angle range across a ring in these maps?

6. **The +0.9° a/b phase offset** appears at both poles, every year, every
   filter, over 9–11 hours. The author calls it a pipeline systematic
   (rotation period over ~10 h) and notes it cancels in yearly means. Compute
   what rotation-period error would produce it. Is that within the known
   uncertainty of Saturn's System III period? If it is *physical*, does it
   change P2?

7. **OPAL's limb-darkening correction and mosaic seams.** Do you know the
   pipeline well enough to say whether it can imprint azimuthal structure at
   m ≈ 6–12 near the limb? The P4 retraction attributes the −69° signal to red
   noise in a dark ring; could it instead be seam structure from six visits?

### D. Estimators

8. **Fractional amplitude** A = |z|/(n·x̄). The author found it inflates in
   dark rings and switched to the spectral peak-over-flat for detection. Is
   there a better estimator — e.g. amplitude relative to the local RMS of the
   ring, or a matched filter for a ten-fold pattern? Would it change any
   survivor?

9. **Spectral peak-over-flat with a white null.** Fit a red-noise slope to the
   −69° spectrum given in the summary (P₅…₁₃ = .15 .14 .13 .12 .11 .10 .09 .08
   .07) and re-express every "×flat" number in the summary against that
   slope. Which claims strengthen, which weaken?

10. **The m=11 companion** (P₁₁ ≈ 0.85 × P₁₀ in 2025). A decagon with
    perfectly even vertices gives no m=11. Non-uniform spacing gives
    symmetric sidebands at 9 and 11; the data show 11 ≫ 9. What produces an
    asymmetric sideband? Ring-averaging a meridionally tilted wave? A drift
    during the ~10 h mosaic? A genuine second mode? Give a discriminating test.

### E. Physics

11. **Width narrowing 39% in one year with amplitude rising 47%.** What does
    barotropic-instability theory predict for envelope width vs amplitude
    during growth and at saturation? Is narrowing-with-growth the expected
    signature, the opposite, or uninformative without winds?

12. **The mode-number question the author started with** — why m=6 north and
    m=10 south — is untouched by everything that survived. Given jet
    latitude, width, and the λ/W values in §4 (4.85 north, 6.1→10.1 south),
    what does the standard deformation-radius / jet-width scaling actually
    predict, and does the south's instability make the question unanswerable
    from imagery alone?

### F. Missing

13. What public or near-public data could break the P2 alias before 2029?
    List specific instruments and programs.

14. What did the author not think to check? Name the single test most likely
    to retract P1 or P5.

15. The two literature values quoted from memory — hexagon drift ≈
    −0.013°/day (Sánchez-Lavega et al. 2014) and stratospheric hexagon onset
    with northern summer (Fletcher et al. 2018) — verify or correct them.

### Format

Numbered to match. Verdict first, then reasoning, then the settling test.
No preamble, no closing summary. If you ran code, show the code.
