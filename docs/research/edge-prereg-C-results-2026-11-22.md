# Pre-registration C: SU(3) with a free fibre field: results (22 November 2026)

**What this is.** The pre-registration is `research/edge-theorem-20261121/wp4_prereg_C_su3.md`, registered at
232ab8d. It has two amendments, C1 (4287efc) and C2 (9f73efd), both made before any floor, edge or competitor was
computed.
- Script: `scripts/edge_wp4_su3_free_fibre.py`.
- Receipt: `research/edge-theorem-20261121/wp4_su3_free_fibre.json`.
- Post-hoc diagnostic, labelled as such: `scripts/edge_wp4_su3_diagnostic.py` and `wp4_su3_diagnostic_posthoc.json`.

**The short version.** I freed the field along her stretched circle in SU(3), to see whether some x makes her the
floor, the way it does in SU(2). **No x does, at any of the 20 shapes.** The existence prediction, the weakest thing
the sentence could say here, **failed**. In this family she does not give way first, and she misses by a lot.

## The family, in one line

- SU(3) with the metric stretched along λ₈ (t on the hypercharge circle).
- Every left-invariant field that commutes with U(2) (dimension 4, design check D3), up to gauge:
  M = u on X₁, X₂, X₃; x on X₈; y on X₄…X₇.
- **x is the field along her stretched circle**, and here it is free. Test 2b had pinned it at the Maurer–Cartan
  value and found class N.
- The grid: t ∈ {2, 3, 4, 6, 8} × (u, y) ∈ {(½, ½), (½, 1), (1, 1), (1, 2)}, with the nine sectors of Test 2. The x
  scan covers the closed-form bounding interval, plus 0.5 on each side.

## Two things I had to fix before the run, both in the open

1. **Amendment C1: her value comes in multiplets.**
   - The family commutes with a **non-abelian** U(2), so she is the top of an SU(2)₁₂₃ multiplet of dimension p + 3,
     doubled when p = q. All of its members share her value.
   - My gate expected 1 or 2 copies and saw 4 to 10. I derived the multiplet count, checked that it matched all nine
     counts, and registered it before computing any floor.
   - I derived the formula after seeing the counts, and the amendment says so.
2. **Amendment C2: other states can tie with her.**
   - At (u, y) = (½, 1), a *second* doublet has exactly her value for every x and t.
   - Those states are competitors that tie her, not copies of her. Under the registered non-strict floor rule a tie
     leaves her the floor, so I remove only her own multiplet.

**Implementation.** All spectra are computed charge block by charge block, in the eigenbasis of the field-independent
charge Q. This is a unitary change of basis, and it agrees with the dense computation to 2 × 10⁻¹³. It made the run
about five times faster.

## Gates (all passed)

| gate | result |
| --- | --- |
| C-K1, K2, K3 (Test 2 controls rerun) | pass |
| C-K4 (SU(2) analogue, same scan code) | x² coefficient 1 to 3 × 10⁻¹⁴; three known edges reproduced to 3 × 10⁻¹⁰ |
| C-K5 (Maurer–Cartan member vs Test 2b) | 2.6 × 10⁻¹⁴ |
| C-sym | ‖[Q, H]‖ 1.1 × 10⁻¹⁴; b residual 8 × 10⁻¹⁶; ‖G\*b‖ 7 × 10⁻¹⁶ |
| C-quad (her x² coefficient is 3/4) | at all 20 shapes and nine sectors, to ≤ 9 × 10⁻¹⁵ |
| C-lin (the bounding state is affine in x) | exact (0.0) |
| C-sigma (one direction of "against her charge") | σ = +1 at every shape |
| C-mult (C2) | pass; the extra ties are recorded at the five (½, 1) shapes |
| C-bound | pass (vacuous, because the floor set is empty) |

## The arms

| arm | prediction | result |
| --- | --- | --- |
| **C-P1 weak** | at some shape, some x makes her the floor of all nine sectors | **FAILED**: her floor set is empty at all 20 shapes, for both the gauge-orthogonal spectrum and H + GG\* |
| **C-P1 strict** | nonempty at every shape with t ≥ 3 | **FAILED** at all 16 |
| C-P2 | the bulk of her region is on the side where x turns against her | not informative (there is no region) |
| C-P3 | one interval | holds only vacuously |
| C-P4 (S) | the competitor is nearly blind to x | holds only vacuously (there is no edge) |
| C-P5 | the upper edge against t | none exists |

By the verdict rule fixed in advance: **C-P1 weak failing counts against the sentence and H3 beyond SU(2).** Freeing
the field along her circle does not make her the floor in this SU(3) family.

## How badly she loses (post-hoc diagnostic; not part of the protocol)

After the verdict, I looked at t = 4. For each shape I took the x where her worst margin over the nine sectors is
largest, her best chance:

| (u, y) | best x | (0,0) | (1,0) | (1,1) | (2,0) | (3,0) | (4,0) |
| --- | --- | --- | --- | --- | --- | --- | --- |
| (½, ½) | −0.32 | −1.09 | −1.89 | −2.18 | −3.16 | −4.96 | −6.82 |
| (½, 1) | −0.59 | −0.28 | −1.50 | −1.62 | −3.00 | −4.34 | −6.09 |
| (1, 1) | −0.67 | −1.00 | −2.55 | −2.85 | −4.50 | −6.12 | −8.17 |
| (1, 2) | −1.57 | +2.76 | −2.28 | −2.41 | −4.50 | −6.46 | −8.28 |

(Margin = the lowest eigenvalue other than her copies, minus hers. Negative means she loses.)

- She loses in every nonzero sector, even at her best x, and the loss **grows with the sector**. Sector (4, 0) is
  worst everywhere, and it blocks her at 121 of 127 scanned x values.
- That is the opposite of SU(2). There the field along her circle gives her a lift **discount**, 2xj/√t, which carries
  her below her competitors in the higher sectors once x is negative enough.
- Here x enters her value with coefficient 3/4 (proved by C-quad). Her lift into higher sectors still costs more than
  her competitors' lifts, by more than any allowed x can repay.
- Some margins come out exactly −1.5, −3.0 and −4.5 at the (½, 1) and (1, ·) shapes. That suggests exact structure,
  which I have not examined.

## What I take from it

- **The sentence, as I wrote it, is about SU(2), or at least not about this SU(3) family.** "She gives way first when
  the field along her own circle turns against her charge" does not hold when "her own circle" is the stretched
  hypercharge circle of SU(3), even with that field free.
- **What did carry over is structural:**
  - her x² coefficient is her charge squared (3/4), as minimal coupling predicts;
  - the bounding state is linear in x;
  - she is an exact gauge-orthogonal eigenvector.
- **What did not carry over is the reason she wins.** In SU(2) the field along her circle buys her a discount in
  every higher sector. In SU(3) the sectors climb against her.
- **The test was fair to the sentence.** I registered x as "her circle" because H3 asked for the stretched fibre. She
  is also charged under λ₃, which couples through u, and u was only sampled at ½ and 1. A family that stretches
  the circle of her own charge H, and not λ₈, is the obvious next test. **That would be a new hypothesis, chosen after
  this failure, and would need its own pre-registration.**

## Status

- **COMPUTED** (double precision; nine sectors; x grid 0.25, so a region narrower than 0.25 cannot be seen).
- The margins in the diagnostic, often −1.5 or worse, are far above that resolution.
- The failure is registered as a failure.
