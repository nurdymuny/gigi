# Which winding does she fall with? (C1, 22 November 2026)

Open question 2 of `docs/research/the-basin-2026-11-22.md`. Script: `scripts/arc_chern_simons_fall.py`.
Receipt: `cs_fall.json` (beside this note). Run: `OMP_NUM_THREADS=1 python scripts/arc_chern_simons_fall.py research/arc-deepdive-20261122`.

## Conventions

- Frame: [e_i, e_j] = 2 ε_ijk e_k on the unit S³, so dθ⁰ = −2 θ¹∧θ² (cyclic). The Berger metric is (t,1,1), with θ̂⁰ = √t θ⁰.
- su(2): ℝ³ with the cross product, realised as τ_a = −(i/2)σ_a. The family is A = x τ₀ θ̂⁰ + y τ₁ θ̂¹ + y τ₂ θ̂², and F = dA + ½[A,A].
- A = θ = g⁻¹dg is x = 2/√t, y = 2.
- **CS(A) = (1/8π²) ∫ tr(A∧dA + ⅔ A∧A∧A)**, where tr is the 2×2 matrix trace of the anti-Hermitian τ's (tr τ_aτ_b = −½δ_ab).
- Orientation: θ⁰∧θ¹∧θ² (the Berger volume form has the same orientation).
- Volumes: Vol(unit S³) = 2π², and Vol(Berger) = 2π²√t.

## Results

**1. The functional on her family.** PROVED (sympy; exterior algebra on 2×2 matrices).

  **CS(x, y, t) = ( t x² + 2 y² − √t · x y² ) / 4**

- The Berger volume factor √t cancels against the coframe factor θ̂⁰ = √t θ⁰.
- So CS is a polynomial in x, y and √t. It is not a polynomial in t itself.
- The same derivation gives F₁₂ = (y² − 2√t x) τ₀, which is the basin note's B. So the conventions agree.

**2. Normalisation check.** PROVED, with an independent COMPUTED check.

- CS(θ) = **1**. This equals −(1/24π²)∫tr(g⁻¹dg)³ for the identity map S³ → SU(2).
- The quadrature check uses an Euler chart and integrates numerically. It gives ∫θ⁰θ¹θ² = 2π² and CS(θ) = 1.0000000000.
- CS(0) = 0.
- CS(meron: t = 1, x = y = 1) = **1/2**.
- CS(Berger Levi-Civita field, x = (2−t)/√t, y = √t) = **(t² − 2t + 2)/2**. Its minimum is 1/2, at t = 1 only.

**3. The pass.** PROVED (sympy; all real critical points of E = ½(B² + 2c²) in the family).

- The flat fields are A = 0 (CS 0) and A = θ (CS 1, at y = ±2). Nothing else in the family is flat.
- There is exactly one more critical pair: **the deformed meron**. Write u = (3√t − √(9t+16))/2. Then the pair is
  x = 2/√t + u and y² = −√t·u.
  - In the metric x² + 2y² it has exactly one negative Hessian direction (index 1), at every t sampled.
  - Its CS is 9t²/4 + 7t/2 + 1 − (3t^{3/2}/4 + √t/2)√(9t+16).
  - That CS equals 1/2 at t = 1 and rises slowly toward 5/9 as t → ∞. Examples: 0.5076 at t = 2, 0.5197 at t = 3.55, 0.5389 at t = 10.
- So the pass between the two basins sits at CS ≈ ½. It is exactly ½ only on the round sphere.

**4. The fall.** COMPUTED (the `family_flow.py` ODE, LSODA, rtol 1e-11, to τ = 200).

| t | CS at start | CS at landing | landing | min CS on the fall (τ) | deformed meron CS | closest approach (L² dist, τ) | CS when her price turns positive |
|---|---|---|---|---|---|---|---|
| 2 | 1 | 1.000000 | A = θ | 0.5624 (0.2) | 0.5076 | 0.120, 0.20 | 0.9997 (τ ≈ 2e-5; her start price is 0 at j = 0) |
| 3 | 5/2 | 1.000000 | A = θ | 0.6569 | 0.5160 | 0.287, 0.15 | 0.918 (τ = 0.041) |
| 3.55 | 3.7512 | 1.000000 | A = θ | 0.7068 | 0.5197 | 0.377, 0.13 | 0.922 (τ = 0.050) |
| 5 | 17/2 | 1.000000 | A = θ | 0.8209 | 0.5269 | 0.604, 0.10 | 0.972 (τ = 0.057) |
| 10 | 41 | 1.000000 | A = θ | 0.9993 | 0.5389 | 1.251, 0.10 | 1.087 (τ = 0.051) |
| extras: 0.5, 1.5 | 0.625 | 1.000000 | A = θ | 0.5493, 0.5210 | 0.5111, 0.5029 | 0.121, 0.045 | (price never negative) |
| 1 | 1/2 | 1/2 | stays on the meron | 1/2 | 1/2 | 0 | |

## Answers

- **What winding does she fall with?** **Winding one.** From every stretch t ≠ 1 the fall lands on A = θ, with CS = 1 to six or more digits. Read against A = 0, the landing basin is the degree-1 vacuum.
  - The starting CS, (t²−2t+2)/2, is not an integer, so the start does not carry a winding.
  - What decides the basin is which side of the pass the fall ends on, and it always ends on the winding-1 side.
  - So in the basin note's terms: **the floor at the bottom is her shadow** (the winding-1 reading, sector 2 at 4/t), not herself.
- **Does CS cross a half-integer?**
  - **It never crosses ½, the pass between winding 0 and winding 1.** The minimum CS on the fall stays above ½ at every t run, and also above the deformed meron's own CS. The fall comes down toward the pass, lingers near it (τ ≈ 0.1–0.3, closest for t near 1), and then climbs back to CS = 1.
  - For t > 2 the start lies above 3/2, so CS sweeps **down** through 3/2, 5/2, … (all of the half-integers below the start). For example, t = 3.55 crosses 3.5, 2.5 and 1.5 at τ = 0.0014, 0.0091 and 0.0242.
    - These are not passes between flat basins inside the family. The family has no flat field at CS ≥ 2.
    - (At t = 5 the start is exactly 17/2, so the "8.5 crossing at τ = 0" in the receipt is the starting point, not a crossing.)
- **CS is not monotone along the fall.** It overshoots down toward the pass and then rises back to 1. That is possible because dCS/dτ = ⟨∇CS, −∇E⟩ has no fixed sign.
- **Where she becomes a wall.** At t = 3 to 10, her price turns positive (door → wall) when CS ≈ 0.92–1.09. That is already within about 0.1 of the landing value.

## Labels and limits

- PROVED: the CS formula; the special values; the critical points and the deformed meron closed form.
- COMPUTED:
  - the quadrature check;
  - the flow, the crossings and the closest approaches;
  - the Hessian index at the deformed meron (sampled t values only).
- STRUCTURAL / not shown here:
  - that the flow stays in the family (basin note);
  - that ending on the θ side means "same side of the pass" in the full infinite-dimensional sense. Within the 2-dimensional family it is a fact of the ODE.
- Standard but not fetched this session, so it is UNVERIFIED as a literature quote: CS shifts by the integer degree under a large gauge transformation, so only CS mod 1 is gauge-invariant on the full space. Inside the family only constant gauge acts, so the real-valued CS above is well defined there.
- Also UNVERIFIED as literature: calling the deformed meron a "sphaleron". Here it is an index-1 critical point at CS ≈ ½, and that is all I claim.
