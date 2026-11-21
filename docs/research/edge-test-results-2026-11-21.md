# The edge test (Test 6): results (21 November 2026)

The protocol was fixed before any code:
[`edge-test-preregistration-2026-11-21.md`](edge-test-preregistration-2026-11-21.md). The predictions were committed
before phase 2 (`edge_test_predictions.json`, commit 580a170).
- Results: `edge_test_results.json`.
- Script: `scripts/ricci_su2_edge_test.py`.
- Sectors j ≤ 12, double precision.

## Gated results: E1 fails, E2 fails

**Phase 1** (training stretches t = 2, 3, 4, 6). For each height, the model x*(t) = c₁/t + c₂/t² was fitted to the
measured edges. Its training residuals were up to 0.02 at y = 0.5, 0.05 at y = 1.5 and 0.08 at y = √t: the form was
already loose before the test.

**E1, held-out edges** (|measured − predicted| ≤ 0.05):

| t | height | predicted | measured | error | pass |
| --- | --- | --- | --- | --- | --- |
| 2.5 | 0.5 | −0.413 | −0.431 | −0.018 | yes |
| 2.5 | 1.5 | −0.439 | −0.477 | −0.038 | yes |
| 2.5 | √t | −0.432 | −0.479 | −0.047 | yes |
| 5 | 0.5 | −0.131 | −0.121 | +0.010 | yes |
| 5 | 1.5 | −0.120 | −0.096 | +0.024 | yes |
| 5 | √t | −0.106 | −0.077 | +0.029 | yes |
| 8 | 0.5 | −0.064 | −0.032 | +0.033 | yes |
| 8 | 1.5 | −0.052 | **+0.040** | +0.092 | **no** |
| 8 | √t | −0.041 | **+0.138** | +0.179 | **no** |

**E1 fails.** Interpolation inside the training range held (6 of 6). Extrapolation to t = 8 failed at two heights: the
edge crossed to positive x, which the model, built to approach 0 from below, cannot produce.

**E2, the Berger crossing, fails.**
- The model's crossing of the Berger path x = (2 − t)/√t is t_c = 2.6315, against t* = 2.6980, a distance of
  0.067 > 0.05.
- The Berger field is not the floor at t_c + 0.05 = 2.68, which is consistent with t* = 2.698 but not with the
  model.

## What held at every point tested (gated as reported, not as a prediction)

- **Her region is a half-line in x.** At all 21 (t, height) pairs, the set where she is the floor of all sectors
  j ≤ 12 is exactly [−3, x*] on a 0.05 grid: no holes and no second piece. So "the field along her circle must turn
  against her by at least so much" is the right shape of statement.
- **The edge rises with the stretch.** At y = 0.5 it goes −0.75 (t = 2), −0.43 (2.5), −0.38 (3), −0.12 (5),
  −0.03 (8). It is negative but approaching zero at small y.
- **Height matters more as the stretch grows.** At t = 8 the edge is −0.03 at y = 0.5, +0.04 at y = 1.5 and +0.14 at
  y = √t. So for strongly stretched spheres with a strong cross field, she can be the floor even with the field along
  her circle turning slightly *with* her.

## What this changes

- **"x < 0" is not the exact boundary.** It is a good first description at small and moderate stretch: every floor
  cell of the exploratory map at t ≤ 4 has x < 0. With enough stretch and cross field, the edge passes zero.
- **The edge is a function x*(t, y) of both variables.** The two-term 1/t model is too simple, and the Berger
  threshold is not yet explained by it.
- **Next (to be pre-registered, not fitted to these data alone).** The edge is where her diagonal entry λ meets the
  lowest eigenvalue of a competing charge block. Both have closed-form rows in this family. Deriving the competitor
  block's lowest eigenvalue exactly would give x*(t, y) as an equation rather than a fit. The Berger crossing could
  then be computed rather than predicted.

## The record of readings, updated

| reading | test | outcome |
| --- | --- | --- |
| symmetry makes her the floor | Test 2 | counted against |
| the field strength aligned with her spin makes her the floor | Test 4 | counted against |
| aligned and singled out makes her the floor | Test 5 | counted against |
| her region is x < x*(t) with x* = c₁/t + c₂/t² | Test 6 | E1 interpolation held, extrapolation failed; E2 failed |
| her region is a half-line in x at fixed (t, y) | Test 6 (reported) | held at all 21 points tested |
| floor of every sector on the Berger field, t ∈ [2.75, 10] | Floor paper | **proved** |
