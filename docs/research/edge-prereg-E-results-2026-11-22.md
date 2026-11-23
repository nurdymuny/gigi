# Pre-registration E: the two hands: results (22 November 2026)

**What this is.** The pre-registration is `research/edge-theorem-20261122-e/wp6_prereg_E_two_hands.md`. It was revised
after an adversarial review, before any data, and registered at 8be0713.
- Script: `scripts/edge_wp6_two_hands.py`.
- Receipt: `research/edge-theorem-20261122-e/wp6_two_hands.json`.
- The idea comes from *the two hands* (Part II).

**The short version.** For x > 0, the floor of every sector is held by a hand that reaches across. That is the state
whose internal turning is opposite to the field along the circle, relative to its own orbital turning (c_l·m·x < 0).
- It held at **all 60 scored points**, from a sector j₀ ≤ 272 on, and usually from j = 1.
- The sister's impurity is exactly what first-order mixing in her own block predicts, to within 14%.
- The onset-scaling test could not be scored: too few points had an onset late enough to measure.

## Gates (all passed)

| gate | result |
| --- | --- |
| E-code-1 (blocks vs the full builder, including j = 120 and 200) | 6.2 × 10⁻¹⁵ |
| E-code-2 (her block = λ_j, relative) | 3.1 × 10⁻¹⁵ |
| E-y0 (at y = 0 the sister is exact, with value λ(−x, 0, t, j)) | 3.6 × 10⁻¹⁵ |
| E-mirror (blocks q and −q isospectral) | 2.2 × 10⁻¹⁵ |
| E-ctrl (inside D she is the floor of every j ≤ 400) | smallest margin +3.51 |

## The arms

| arm | prediction | result |
| --- | --- | --- |
| **E1** | at every one of 60 scored points (y > 0, t ≥ 3), the floor eigenspace is at least half across-reaching from some j₀ ≤ 300 on | **held, 60 of 60.** j₀ is 1 at most points and at most 272 |
| **E2-y** | j₀(y = 3)/j₀(y = 1.5) ∈ [2, 8] (predicted 4) at eligible pairs, with at least 5 pairs | **not scored**: only 4 eligible pairs. Descriptively they give **4.41, 4.45, 4.47 and 4.61** |
| **E2-x** | j₀(x = 0.2)/j₀(x = 1.0) ∈ [2.5, 10] | **not scored**: no eligible pairs (the onset is immediate at x = 1) |
| **E3** | the measured impurity 1 − S₄₀₀ is within a factor 1.5 of the registered first-order formula, at ≥ 80% of eligible points | **held, 60 of 60.** The ratio of measured to predicted is between 0.86 and 1.00 |

## What it says

1. **The cross-reach is the structure of the floor at x > 0** (COMPUTED, j ≤ 400, t ≥ 3).
   - Whichever way the field along the circle turns, the cheapest state in each sector is one whose internal
     turning opposes it.
   - At j = 400 the floor sits in block ±l, the sister's block and its mirror, at all 60 scored points.
2. **The cross field sets her sister's impurity, quantitatively.** Her impurity is the first-order mixing in her own
   block, to within 14%, from 4 × 10⁻⁴ at y = 0.5 up to a few per cent at y = 3. So y decides how clean the reaching
   hand is: exact at y = 0, and mixed by an amount the rows predict.
3. **The onset scaling looks right but is not tested.** The four pairs that could be measured all give a ratio near
   4.5, where y² scaling predicts 4. That is suggestive, and below the registered bar for a verdict.

## Descriptive: t = 2 and y = 0

- **At t = 2, a different hand reaches across.** At 15 of the 20 points with t = 2, the j = 400 floor sits in block
  ±(l − 2), not ±l.
  - Its across-reaching weight is still 0.96 to 1.00, so it reaches across.
  - But it is not the sister: her weight there is 0.
  - The review predicted that t = 2 is degenerate, because the sister ties another across-reaching type on w there.
  - So **"a hand always reaches across" survives at t = 2; "her sister" does not.**
- **y = 0** behaves as proved: the sister is exact (E-y0).

## Status

- **COMPUTED.**
- The cross-reach at large j is nearly forced by the row diagonal (the design check). What this test adds:
  - it holds at the level of the actual floor, with no block restriction, from small j on;
  - the sister's impurity is first-order and quantitatively predicted.
- **A proof** of the asymptotic statement, for all l beyond an explicit l₀, looks within reach of the same exact
  Gershgorin machinery that closed the large-j theorems. That would make the two hands a theorem.
