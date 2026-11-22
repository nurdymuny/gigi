# Pre-registration D: SU(3) along her own charge circle: results (22 November 2026)

**What this is.** The pre-registration is `research/edge-theorem-20261122-d/wp5_prereg_D_su3_own_circle.md`. It was
revised after an adversarial review that found one fatal flaw and four major ones, all before any data, and registered
at 8adea29. The interim note (`edge-prereg-D-interim-2026-11-22.md`) reported 11 of 20 shapes, unscored. This note
scores all 20, exactly as registered.
- Script: `scripts/edge_wp5_su3_own_circle.py`.
- Receipt: `research/edge-theorem-20261122-d/wp5_su3_own_circle.json`.

**The short version.** In Test C I stretched SU(3) along hypercharge, and she was the floor nowhere. In D I stretched
it along the circle of her own charge, and **she came back**:
- at 19 of 20 shapes, and at every one of the 16 with t ≥ 3;
- always in one interval;
- always on the side where the field along her circle turns against her charge.

The sentence holds in a second group, along the right circle.

The one clause that failed is the one that failed in SU(2) too: her rival is not blind. The charged rival rides the
field almost as the SU(2) charged rivals did.

## Gates (all passed)

| gate | result |
| --- | --- |
| D-K1–K3 (Test 2 controls) | pass |
| D-K4 (SU(2) analogue) | 2.5 × 10⁻⁹ |
| D-K5 (Maurer–Cartan known answer) | 2.6 × 10⁻¹⁴ |
| D-K6 (rotated basis) | 7.7 × 10⁻¹⁵ |
| D-K7 (wrong-axis control) | 3.0 × 10⁻¹⁴ |
| D-quad: her x² coefficient is 1 | 5.3 × 10⁻¹⁵ |
| D-kappa: lift slope (p + q)/√t | 7.1 × 10⁻¹⁵ |
| D-sym | ‖[Q, H]‖ 6.4 × 10⁻¹⁵; residual 8 × 10⁻¹⁶; ‖G\*b‖ 4 × 10⁻¹⁶ |
| D-lin, D-mult, D-bound | pass |

## The arms

| arm | prediction | result |
| --- | --- | --- |
| **D-P1 weak** | her floor set is nonempty at one or more shapes | **held** |
| **D-P1 strict** | nonempty at every shape with t ≥ 3 | **held, 16 of 16** |
| **D-ext** | the pass survives the two extra sectors (5,0) and (6,0) | **held overall** (not truncation-sensitive). At one t = 2 shape, (1, 1, 3/2), no grid point survives; every other nonempty shape keeps 2 to 8 grid points |
| **D-P2** (side, "only when") | σ·x_hi ≤ 0 where scorable (bound not forcing it, SU(2) calibration edge ≤ 0) | **held, 15 of 15** scorable shapes. The other 4 nonempty shapes (t = 6 and 8, calibration edge > 0) are descriptive, and there too her region lies at x < 0 |
| **D-P3** | one interval | **held** at every nonempty shape |
| **D-P4 (S)** | the rival's slope ratio < 0.5 at every edge | **failed**. The charged rival, sector (4,0) with charge −7, has ratios 0.61 to 0.76. The neutral rival, sector (3,0), has 0.15 to 0.19 |

By the verdict rule fixed in advance:
- D-P1 (both forms), D-P2 and D-P3 held. D-P4 (S) failed.
- So **the sentence's existence, side and interval clauses are supported in this SU(3) slice, along her own circle**
  (COMPUTED, nine sectors, t from 2 to 8).
- The "nearly blind" clause is counted against, as it already was in SU(2) (Test B).
- The rule's own reading says this points to **her own circle** as what carries over between groups, and reads C's
  failure as "the wrong circle". That reading is itself a hypothesis, and Test F is testing it now.

## Her region, shape by shape

| t | shape (w, y₁₂ = y₂₃, y₁₃) | her floor region (x) | rival above (sector, charge) | slope ratio |
| --- | --- | --- | --- | --- |
| 2 | (½, ½, ¼) | [−2.03, −0.98] | (4,0), −7 | 0.67 |
| 2 | (½, ½, ¾) | [−1.68, −0.74] | (3,0), 0 | 0.19 |
| 2 | (½, 1, 1) | empty (best −0.10) | n/a | n/a |
| 2 | (1, 1, 3/2) | [−1.65, −1.19], truncation-sensitive | (4,0), −7 | 0.74 |
| 3 | (½, ½, ¼) | [−2.07, −0.63] | (3,0), 0 | 0.15 |
| 3 | (½, ½, ¾) | [−2.07, −0.58] | (3,0), 0 | 0.17 |
| 3 | (½, 1, 1) | [−1.67, −1.12] | (4,0), −7 | 0.75 |
| 3 | (1, 1, 3/2) | [−1.95, −1.07] | (4,0), −7 | 0.75 |
| 4 | (½, ½, ¼) | [−2.15, −0.59] | (4,0), −7 | 0.61 |
| 4 | (½, ½, ¾) | [−2.32, −0.49] | (3,0), 0 | 0.15 |
| 4 | (½, 1, 1) | [−1.96, −1.07] | (4,0), −7 | 0.75 |
| 4 | (1, 1, 3/2) | [−2.17, −1.01] | (4,0), −7 | 0.75 |
| 6 | (½, ½, ¼) | [−2.32, −0.59] | (4,0), −7 | 0.62 |
| 6 | (½, ½, ¾) | [−2.48, −0.44] | (4,0), −7 | 0.64 |
| 6 | (½, 1, 1) | [−2.42, −1.00] | (4,0), −7 | 0.76 |
| 6 | (1, 1, 3/2) | [−2.55, −0.94] | (4,0), −7 | 0.76 |
| 8 | (½, ½, ¼) | [−2.50, −0.57] | (4,0), −7 | 0.62 |
| 8 | (½, ½, ¾) | [−2.65, −0.42] | (4,0), −7 | 0.63 |
| 8 | (½, 1, 1) | [−2.66, −0.94] | (4,0), −7 | 0.76 |
| 8 | (1, 1, 3/2) | [−2.79, −0.88] | (4,0), −7 | 0.76 |

**Descriptive** (no gate):
- **Her region widens as the sphere stretches.** At t = 8 it runs from about −2.8 to −0.4.
- **Its upper edge stays clearly negative**, while the SU(2) calibration edge drifts to zero and slightly past it at
  t = 6 and 8. In the bigger group the field has to turn against her by more.
- **The rival pattern is SU(2)'s.** The neutral rival is nearly blind; the charged rival rides; and as the stretch
  grows, the charged one takes over.

## What I read from it

- **The circle matters, and her own circle carries the law.** Hypercharge (C): nowhere. Her charge circle (D): 19 of
  20, every t ≥ 3, one interval, the right side. Same group, same nine sectors, same method. The only difference is
  which circle is stretched.
- **What travelled into SU(3) is the whole edge law except the blind rival.** That includes:
  - her exactness;
  - her minimal coupling (x² coefficient 1);
  - the lift discount (κ = (p + q)/√t);
  - her bounded region in one interval;
  - the "against her charge" side.

  The rival that sets her edge is, as in SU(2), whichever reaches her first. The charged one rides the field nearly as
  hard as she does.
- **In SU(3) "her own circle" is three things at once:** the principal circle, the coroot of her highest root, and a
  regular circle. D cannot tell which one she needs. Test F, running now in SU(4) and Sp(2), sweeps the circle across
  the chamber to find out.

## Status

- **COMPUTED.** Nine sectors (plus two for the truncation check), a 0.25 grid with golden-section search,
  t ∈ {2, 3, 4, 6, 8}, 20 shapes.
- Nothing here is a proof.
- The sub-family is a 4-parameter slice of an 8-parameter class: diagonal Cartan block, zero phases, y₁₂ = y₂₃.
