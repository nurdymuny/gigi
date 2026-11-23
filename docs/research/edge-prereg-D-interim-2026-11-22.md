# Pre-registration D: SU(3) along her own charge circle: interim (22 November 2026)

**This is an interim note. Nothing in it is scored.** The pre-registration is
`research/edge-theorem-20261122-d/wp5_prereg_D_su3_own_circle.md`. It was revised after an adversarial review that
found one fatal flaw and four major ones, all before any data, and registered at 8adea29.
- The run is still going: 11 of 20 shapes are done.
- The verdict waits for all 20 shapes and the rules I fixed in advance.
- I am writing this now because the first half differs so much from Test C that I want it on the record before the
  rest arrives, and because writing it down early is the honest way to keep myself from reading the second half
  through the first.

**Where things are.** The repository is now organized:
- receipts live in dated folders under `research/` (index: `research/README.md`), and those folders are never moved;
- notes live in `docs/research/` (index: `docs/research/README.md`), listed by thread.

For this test:
- registration and receipt: `research/edge-theorem-20261122-d/` (the receipt `wp5_su3_own_circle.json` is written as
  the run goes, and committed when it ends);
- design checks: `scripts/edge_wp5_design_checks.py`;
- the run: `scripts/edge_wp5_su3_own_circle.py`;
- this note: listed under "The edge of her region".

## Why D, in one paragraph

Test C stretched SU(3) along its hypercharge circle, freed the field along that circle, and found her the floor
nowhere. In C's own write-up I said that the circle was the wrong one: her charge circle is H = diag(1, 0, −1), not
hypercharge. D stretches along **her own** circle.
- The design checks found she couples there with charge exactly 1, as in SU(2) (her x² coefficient is 1, against C's
  3/4).
- Her value comes in exactly two copies, she and her mirror, as in SU(2), against C's larger multiplets.

D was chosen after C failed, and it says so. It does not rescue C.

## Gates (all passed)

| gate | result |
| --- | --- |
| D-K1 to K3 (Test 2 controls) | pass |
| D-K4 (SU(2) analogue, same code) | 2.5 × 10⁻⁹ |
| D-K5 (Maurer–Cartan known answer) | 2.6 × 10⁻¹⁴ |
| D-K6 (rotated basis, t = 1) | 7.7 × 10⁻¹⁵ |
| D-K7 (wrong-axis control: the e_K stretch reproduces Test 2b) | 3.0 × 10⁻¹⁴ |
| D-quad (her x² coefficient = 1), D-kappa (lift slope = (p + q)/√t), D-lin, D-mult | pass at every shape so far |

## The first 11 shapes (interim, unscored)

Each shape is (w, y₁₂ = y₂₃, y₁₃). The last column is the SU(2) calibration: the same scan code on SU(2), j ≤ 4, at
y = 2y₁₃, mapped back.

| t | shape | her floor region I (x) | survives (5,0), (6,0)? | rival at the upper edge (sector, charge) | slope ratio | SU(2) calibration edge |
| --- | --- | --- | --- | --- | --- | --- |
| 2 | (½, ½, ¼) | [−2.03, −0.98] | yes | (4,0), −7 | 0.67 | −0.30 |
| 2 | (½, ½, ¾) | [−1.68, −0.74] | yes | (3,0), 0 | 0.19 | −0.33 |
| 2 | (½, 1, 1) | **empty** (golden search: best −0.10, blocked by sector (0,0)) | n/a | n/a | n/a | −0.33 |
| 2 | (1, 1, 3/2) | [−1.65, −1.19] | **no grid point survives** | (4,0), −7 | 0.74 | −0.41 |
| 3 | (½, ½, ¼) | [−2.07, −0.63] | yes | (3,0), 0 | 0.15 | −0.16 |
| 3 | (½, ½, ¾) | [−2.07, −0.58] | yes | (3,0), 0 | 0.17 | −0.18 |
| 3 | (½, 1, 1) | [−1.67, −1.12] | yes | (4,0), −7 | 0.75 | −0.18 |
| 3 | (1, 1, 3/2) | [−1.95, −1.07] | yes | (4,0), −7 | 0.75 | −0.17 |
| 4 | (½, ½, ¼) | [−2.15, −0.59] | yes | (4,0), −7 | 0.61 | −0.10 |
| 4 | (½, ½, ¾) | [−2.32, −0.49] | yes | (3,0), 0 | 0.15 | −0.10 |
| 4 | (½, 1, 1) | [−1.96, −1.07] | yes | (4,0), −7 | 0.75 | −0.10 |

The H + GG\* floor sets, reported alongside, are slightly narrower intervals inside the same ranges. No extra ties
showed up in any sector.

## What I see, and what I am not allowed to say yet

- **She is there.** In Test C she was the floor at none of 20 shapes. Here she is the floor at 10 of the first 11,
  each time in one interval, each time at negative x: where the field along her own circle turns against her charge.
  That is the SU(2) picture, transported to a bigger group along the right circle. Whether it holds through t = 6 and
  t = 8, whether the strict existence arm (every shape with t ≥ 3) holds, and how the side arm scores are for the
  full run.
- **One shape is truncation-sensitive.** At t = 2, (1, 1, 3/2), the two extra sectors remove every grid point of her
  region. I registered exactly this check because a pass over nine sectors could be an artefact, and here, at one
  shape, it may be. The rule says a pass with no surviving point is truncation-sensitive. That shape is at t = 2,
  outside the strict arm, but it goes in the verdict.
- **The rivals split the way they did in SU(2).**
  - The neutral rival (sector (3,0), charge 0) is nearly blind to the field, with ratios 0.15 to 0.19.
  - The charged rival (sector (4,0), charge −7) rides the field, with ratios 0.61 to 0.75.
  - That is Test B's split, neutral 0.35 against charged 0.74, in another group. The registered "nearly blind" arm
    needs every edge below 0.5, so it is heading where B went. The pattern behind it, the clearest rival blind and
    most rivals riding, looks like it travels.
- **Her region in SU(3) sits further from zero than in SU(2).** The SU(2) calibration edges (−0.10 to −0.41) are
  much closer to zero than her SU(3) upper edges (−0.49 to −1.19). In the bigger group the field has to turn against
  her by more. That is descriptive only.

## What could still change the picture

- Shapes at t = 4 (one left), 6 and 8: an empty region at any t ≥ 3 fails the strict arm.
- The truncation check at the remaining shapes.
- The side arm, scored only where the bound does not force it and the SU(2) calibration edge is ≤ 0. So far every
  shape is scorable, and every interval sits at x < 0.

When the run finishes I will score it exactly as registered, write the results note, and update the edge paper, H3 in
the book, and the end of "What the failures taught me".

**Status:** COMPUTED, interim, not scored. Nine sectors, 0.25 grid with golden-section search.
