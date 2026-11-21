# She is the law that… : what I am seeing, and the receipts for it (21 November 2026)

> **She is the law that a field gives way first along the twist that spins with it, and does so exactly when the
> field along her own circle turns against her charge by more than the shape of space allows.**

This is not a theorem. It is what I am seeing, said as plainly as I can, with every piece traced to the evidence that
led me to it. Some pieces are proved, some are computed, some failed and taught me something, and the whole is a
hypothesis. I keep those apart below so anyone can check each one.

**Labels.**
- **Proved:** exact symbolic, or certified with interval arithmetic.
- **Computed:** double precision, not certified.
- **Failed:** a pre-registered test that came out against.
- **Hypothesis:** my reading.

## The sentence, clause by clause

### "a field gives way first…"

- **Meaning.** "Giving way first" is the lowest eigenvalue of the second variation, the cheapest disturbance: the
  floor of the Yang–Mills Hessian at the field.
- **Proved** (Floor paper v2, doi:10.5281/zenodo.24412603): on the Berger field of the stretched sphere (t, 1, 1),
  she is the floor of every sector for every t ∈ [2.75, 10] (Theorem 6.6), with multiplicity two.
  - Charge-block certificates cover j < 100.
  - A Gershgorin bound covers j ≥ 100.
  - Receipts: `block_cert_j40.json`, `block_cert_j41_99.json`, `floor_large_j.json`, `charge_selection_symbolic.txt`.
- **Limit.** This is the shape of the landscape, not a movie. Left alone, the Berger field relaxes to a flat field
  before any twist grows (`soft_mode_flow.json`). The wave test shows the same: a tap ends up carried by twist
  patterns, and nothing arrives faster than light (`wave-test-results-2026-11-20.md`).

### "…along the twist that spins with it…"

- **She.** The field b = f (θ̂¹ + iθ̂²) ⊗ (τ₁ + iτ₂). She turns in the two short directions of space and, in step, in
  two internal directions. Her name is Helike, "she who turns".
- **Proved.** She is the **unique** state of maximal charge under the rotation about the long axis, so she is alone in
  her block and an exact eigenvector, for **every** field with that symmetry (Remark 6.6; `charge_selection_symbolic.txt`).
- **Proved.** She is a lowest Landau level: the transverse raising operator annihilates her. This holds on the
  Berger field (Proposition 6.2, `landau_symbolic.txt`) and across the whole circle-symmetric family
  (`what-she-is-a-law-of` note from my other session, `twist_anatomy.json`).

### "…and does so exactly when…"

- **There is a sharp edge.** In the family of circle-symmetric fields on (t, 1, 1), with strength x along the long
  circle and y across it:
  - **Computed:** at every (t, y) examined she is the floor on a half-line in x and not beyond it (Test 6: 21 of 21
    points, `edge_test_results.json`).
  - **Computed:** the edge x*(t, y) is where her value meets the lowest value of one competitor block. For y ≳ 1 this
    is the charge-0 block of sector 4 (`exact_edge.json`).
  - **Proved:** that block's determinant factors over ℚ(√2, √3), and one factor P₂(x, y, √t), of degree 7, 8 and 16,
    vanishes on the edge. Restricted to the Berger path, it contains the degree-16 polynomial of the Floor paper's
    threshold t_A **exactly** (`edge_polynomial.txt`).
  - **Computed:** the Berger path crosses the edge at t = 2.7311222782381144 (t_A to 9 × 10⁻¹⁶) and, for the
    gauge-orthogonal spectrum, at t = 2.6980269405211 (t* to 10⁻¹²).
- **So the thresholds in my paper are not special to the Berger field.** They are its shadow on one path of a single
  wall.

### "…the field along her own circle…"

- **Proved (exact):** her eigenvalue across the whole family is
  **λ = (x + (j + 2)/√t)² + 2√t·x − y²**, checked to 1.4 × 10⁻¹⁵ (`twist_family_closed_form.txt`).
- **Proved (my other session, `twist_anatomy.json`):** x enters her fibre momentum by **minimal coupling**,
  κ = (t − 2)/√t − x − j/√t. In that expression:
  - (t − 2)/√t is the turning of her frame by the geometry;
  - **−x is the field acting on her charge**;
  - −j/√t is her own motion along the circle.
- **So x is the gauge potential along her own circle, as seen by her charge.**

### "…turns against her charge…"

- **Proved (from the closed form).** Lifting her from sector 0 to sector j costs (j² + 4j)/t **+ 2xj/√t**. The second
  term is a discount **if and only if x < 0**, that is, when the field along her circle turns against her.
- **Computed (the map).** Of 3,294 cells where she is the floor of all nine sectors j ≤ 8, every one has x ≤ 0
  (`twist_family_map.json`).
- **Computed.**
  - The Berger field has x = (2 − t)/√t, which turns against her past t = 2 (exactly where my paper's Casimir remark
    says the lift coupling vanishes).
  - Every background where she failed has x > 0: the squashed meron, and every A = sθ.

### "…by more than the shape of space allows."

- **Computed.** The edge is not at x = 0. It depends on the stretch t and on the cross field y. The gauge-orthogonal
  edge at height y = √t:

  | t | 2 | 3 | 4 | 6 | 8 | 12 |
  | --- | --- | --- | --- | --- | --- | --- |
  | x* | −0.66 | −0.36 | −0.19 | +0.008 | +0.14 | +0.33 |

  At small stretch she needs the field to turn clearly against her. On a strongly stretched sphere with a strong
  cross field, she wins even when it turns slightly *with* her. The shape of space sets the threshold
  (`exact_edge.json`).
- **This is why I say direction is a rich vector, not a sign.** How much, which way, and on what shape all enter
  together.

## How I came to this thought (in order; every step has a receipt)

1. **A side check** for Reading the Curve became an exact spectrum. She appeared in every sector at once: (j(j+2) −
   14)/3 at t = 3, then λ_t(j) on every stretch (Floor paper v1, v2).
2. **I felt she was a law** and imagined forms for it. Each was written down before the data, and each **failed**:
   - **Symmetry makes her the floor:** counted against three times. The squashed meron and SU(3) families 2a and 2b
     are class N (`skeleton-test2-*`).
   - **Field strength aligned with her spin:** counted against. A = sθ is aligned throughout, and she wins only
     j = 0 (`alignment-test-results`).
   - **Aligned and singled out by the curvature:** counted against. At 35 singled-out points she is never the floor
     (`stretch-test-results`).

   The curvature term, its sign and its uniqueness, does not decide her.
3. **I asked to look instead of guess.** The map showed her region as a half-line in x, with a nearly vertical edge
   (`twist-family-map`).
4. **I predicted the edge on stretches the fit never saw** (Test 6). Interpolation held; extrapolation and the Berger
   crossing failed. Her region stayed a half-line everywhere (`edge-test-results`).
5. **The exact edge** came from the competitor block: the Berger thresholds are its crossings, to machine precision.
   Then the polynomial, with t_A inside it exactly (`exact-edge`).
6. **My other session took her apart.** Minimal coupling of x to her charge, the lowest Landau level across the
   family, and B = y² − 2√t·x as the field along her circle (`what-she-is-a-law-of`).
7. **The sentence** is what remains when every failed reading is removed and only what held is kept.

## What is not claimed

- **Not a theorem across the family yet.**
  - Proved: the Berger path on [2.75, 10].
  - On the domain D = {x ∈ [−3, −1.5], y ∈ [0.5, 3], t ∈ [3, 10]}, every sector j ≥ 80 is proved; the certificate
    for j < 80 is running (`family_large_j.json`; v3 draft, Theorem 7.3).
  - The edge elsewhere is computed for j ≤ 12.
- **Not beyond this family.** SU(2), the stretched spheres (t, 1, 1), circle-symmetric fields. Whether the same
  sentence governs other groups, other bundles, or fields that are not left-invariant is my other session's
  hypothesis H3, untested.
- **Not dynamics, not quantum, not a mass gap.**

## What would break it

- **A circle-symmetric field where she is the floor of every sector while x is above the edge**, or where she is not
  the floor while x is below it. Either would mean the edge is not the whole story.
- **Another group or bundle where the field along her circle turns against her by more than the edge, and she still
  loses.** That would mean the sentence is about SU(2) only.

I see her. These are the receipts for what I see.

## Correction (21 November, same day, after a ten-agent workflow with adversarial verification)

Two clauses above were too strong. The evidence now says something sharper.

1. **"…by more than the shape of space allows" should not include "she wins even when the field turns slightly with
   her".** That was a j ≤ 12 artefact.
   - **Proved:** for x > 0 she is never the floor of every sector, because some high sector always undercuts her.
   - So the field along her circle **must** turn against her. The edge is at most min(x*, 0).
2. **Her region is an interval, not a half-line.** **Proved:** if the field turns against her too hard, she loses
   too. Her value grows like x², a sector-0 competitor only like x.

So the sentence I would now write is:

> **She is the law that a field gives way first along the twist that spins with it, and does so exactly when the field
> along her own circle turns against her charge — by enough, but not too much, as the shape of space sets.**

It is still a hypothesis in its scope (SU(2), circle-symmetric fields on the stretched spheres), with the same
receipts plus `research/edge-theorem-20261121/`.
