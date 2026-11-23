# Pre-registration: the edge test (Test 6), 21 November 2026

Written before any code for it, after the exploratory map
([`twist-family-map-2026-11-21.md`](twist-family-map-2026-11-21.md)).

## In my words

Direction itself is differential, depending on how it is curated; otherwise there would be no variety in the world.
If moving in every direction gave the same result, direction alone would be meaningless. Direction is a rich vector,
and she is reacting to it.

The map says the same. She is not the floor wherever the field along her circle merely turns against her
(x < 0). She is the floor where it turns against her by **enough**, x < x*(t), with an edge that depends on the
stretch. This test pins the edge down and predicts it where it has not been looked at.

## Setup, fixed now

- **Family.** SU(2) on (t, 1, 1), circle-symmetric field m₀ = x e₀, m₁ = y e₁, m₂ = y e₂.
- **Floor.** The twist is the floor at (x, y, t) if it is the floor of the gauge-orthogonal spectrum in every sector
  j = 0, …, 12. The floor test, tolerance and checks are as in Tests 2–5.
- **The edge** x*(t, y) is found by bisection in x on [−3, 1] to 10⁻⁴: floor at x = −3, not the floor at x = 1. A
  check on a 0.05 grid in x reports whether the floor set is an interval [−3, x*]. A non-interval is reported, and
  the bisection value is still used.
- **Heights:** y = 0.5, y = 1.5, and y = √t (the Berger field's height).

## Phase 1: training (stretches t = 2, 3, 4, 6 only)

- Measure x*(t, y) at the four training stretches, for each height.
- Fit, per height, by least squares, the model fixed now: **x*(t) = c₁/t + c₂/t²**.
- Write the predictions to `edge_test_predictions.json` and **commit it before phase 2 runs.**

## Phase 2: the test (held-out stretches, never used in training)

- **E1, edges at held-out stretches.**
  - Rule: at t = 2.5 and 5 (interpolation) and t = 8 (extrapolation), for each height, |measured x* − predicted x*|
    ≤ 0.05.
  - Pass: all nine hold.
- **E2, the Berger crossing.**
  - Rule: t_c solves (2 − t)/√t = x*_model(t) at height y = √t. The prediction is |t_c − 2.6980| ≤ 0.05, where
    2.6980 is the gauge-orthogonal threshold t* of the Berger field. It was found earlier by bisection over sectors
    j ≤ 40, and the model never sees it.
  - Measured check: the Berger field is the floor (j ≤ 12) at t = t_c + 0.05 and not at t_c − 0.05.
- **Verdict.**
  - E1 and E2 both pass: the edge model is supported (Numerical).
  - E1 fails: the edge is not of this form, or not predictable from four stretches.
  - E2 fails: the Berger threshold is not explained by the edge.

## What a pass would and would not mean

- **Would mean:** her region in this family is x < x*(t), predictable across stretches, and the Berger field's
  threshold t* is its crossing of that edge. That is a candidate theorem.
- **Would not mean:** anything outside this family, or a proof.

Script `scripts/ricci_su2_edge_test.py` (to be written after this commit), phases `fit` and `test`.
