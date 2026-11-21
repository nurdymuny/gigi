# The exact edge of her region (21 November 2026)

**Status.** This is a derivation with exact ingredients, plus root-finding to 10⁻¹² and validation against known
numbers. It is not a pre-registered test. Sectors j ≤ 12, double precision.
- Script: `scripts/ricci_su2_exact_edge.py`.
- Receipt: `research/su2-floor-family-20261119/exact_edge.json`.

## The derivation

- **Family.** SU(2) on (t, 1, 1), circle-symmetric field m₀ = x e₀, m₁ = y e₁, m₂ = y e₂.
- **Charge blocks.** The charge q = m + c_f + c_l is conserved for every member, so each sector of H + GG* splits
  into blocks of at most nine states. Their rows are now closed-form in (x, y, t):
  - diagonal: (w_a − 2m/√t)² + 4(l(l + 1) − m²) + C_aa, with C = −(Ω₁² + Ω₂²) + Ric ⊗ I − 2[F, ·];
  - ladder couplings: 2i √((l − m)(l + m + 1)) Ω₋ and 2i √((l + m)(l − m + 1)) Ω₊.
- **Her block.** Her own block is 1 × 1, with entry λ = (x + (j + 2)/√t)² + 2√t·x − y².
- **The edge.** She is the floor (of H + GG*) exactly when λ is at most the lowest eigenvalue of every other block.
  Moving x toward her edge, the first block to undercut her fixes it: **the edge is a root of
  det(B_{j,q}(x, y, t) − λ_j) = 0 for that competitor block.**
- **Gauge-orthogonal version** (the definition the floor tests use): the same, with the slice spectrum in place of
  the blocks.

## Validation against numbers that were already known

| check | result |
| --- | --- |
| blocks vs the full H + GG* sector spectra (12 random points) | agree to 1.5 × 10⁻¹⁵ |
| Berger path x = (2 − t)/√t, y = √t crossing the H + GG* edge | **t = 2.7311222782381144** vs t_A = 2.7311222782381135 (root of the degree-16 polynomial): difference 9 × 10⁻¹⁶; competitor block (j = 4, q = 0), the same block |
| Berger path crossing the gauge-orthogonal edge | **t = 2.6980269405211** vs t* = 2.6980269405200: difference 1.1 × 10⁻¹²; competitor sector j = 4 |
| gauge-orthogonal edges at Test 6's 21 points vs its bisection | agree to ≤ 3 × 10⁻⁵ (bisection tolerance 10⁻⁴) |

**So the Berger threshold is, exactly, where the Berger path crosses her edge.** This holds for both definitions: t_A
for H + GG*, and t* for the gauge-orthogonal spectrum. The edge is the general law, and the Berger threshold is one
point on it.

## The edge

The gauge-orthogonal edge x*_S and the H + GG* edge x*_A, with the competitor that sets each:

| t | y = 0.5 | y = 1.5 | y = √t |
| --- | --- | --- | --- |
| 2 | −0.605 (j = 3) | −0.658 (j = 4) | −0.655 (j = 4) |
| 2.5 | −0.431 (j = 3) | −0.477 (j = 4) | −0.479 (j = 4) |
| 3 | −0.318 (j = 2) | −0.352 (j = 4) | −0.355 (j = 4) |
| 4 | −0.192 (j = 2) | −0.192 (j = 4) | −0.189 (j = 4) |
| 5 | −0.121 (j = 2) | −0.096 (j = 4) | −0.077 (j = 4) |
| 6 | −0.078 (j = 2) | −0.033 (j = 4) | +0.008 (j = 4) |
| 8 | −0.032 (j = 2) | +0.040 (j = 4) | +0.138 (j = 4) |
| 10 | −0.009 (j = 2) | +0.079 (j = 4) | +0.241 (j = 4) |
| 12 | +0.003 (j = 2) | +0.102 (j = 4) | +0.329 (j = 4) |

(Gauge-orthogonal edge; the competitor sector is in parentheses. The H + GG* edges are slightly lower; see the
receipt.)

- **One competitor governs most of the family:** the charge-0 block of sector j = 4, for y around 1 and above. That
  is the same block that sets the Berger threshold. At small cross field, small blocks of sectors j = 2 and 3 (charge
  ±1, ±½) take over.
- **The edge rises with the stretch and crosses zero.** At y = 0.5 it crosses near t ≈ 12; at y = 1.5 between t = 6
  and 8; at y = √t near t ≈ 6. Past that, she is the floor even with the field along her circle turning slightly
  *with* her, provided the cross field is strong.
- **Her region is a half-line in x at every point computed.** The edge is a single crossing (Test 6 checked this on a
  0.05 grid at its 21 points).

## What this means

- What decides whether she is the floor is **a single comparison**: her own value against the lowest value of one
  competitor, usually the charge-0 block of sector 4. The comparison is governed by the field along her circle (x),
  the stretch (t) and the cross field (y) **together**. That is direction as a rich vector, as Bee put it, made
  exact.
- The Floor paper's thresholds t_A and t* are this law evaluated on the Berger path, reproduced to 10⁻¹⁵ and 10⁻¹².

## Next

- **A closed form for the edge.** det(B_{4,0}(x, y, t) − λ₄) = 0 is a polynomial equation in (x, y, √t). The
  Berger restriction alone gave the degree-16 polynomial, so the general one is heavy but finite. Its largest real
  root in x is the edge.
- **All sectors.** Here j ≤ 12. On the Berger path all sectors are proved in [2.75, 10]. For the family, the
  large-j Gershgorin argument should carry over once x and y enter the rows, which they now do in closed form.
- **A theorem for the family:** "she is the floor iff x < x*(t, y)", with x* defined by the competitor block. The
  Floor Theorem would be its first case.

## Addendum (same day): the closed-form edge polynomial, and the large-j proof on a domain

**The edge polynomial.** Script `scripts/ricci_su2_edge_polynomial.py`, receipt `edge_polynomial.txt`.

- With s = √t, the numerator of det(B_{4,0}(x, y, s²) − λ₄ I) factors over ℚ(√2, √3) into two polynomials. Their
  degrees in (x, y, s):
  - **P₁: (5, 6, 12)**, 61 terms;
  - **P₂: (7, 8, 16)**, 113 terms.
- **P₂ vanishes on the edge** wherever the (4, 0) block is the competitor, which is y ≳ 1. It is within 10⁻⁸ at the
  numerically located edges, against 10³ to 10⁶ elsewhere.
- **Restricted to the Berger path** x = (2 − s²)/s, y = s, P₂ contains the degree-16 polynomial of t_A exactly
  (polynomial division, zero remainder).
- So t_A is, algebraically, a root of the edge polynomial on the Berger path. The edge is the largest real root in x
  of P₂(x, y, √t).
- At small cross field (y = 0.5) the competitors are blocks of sectors 2 and 3, which have their own polynomials (not
  derived here).

**The large-j half of an all-sector proof on a domain.** Script `scripts/ricci_su2_family_large_j.py`, receipt
`family_large_j.json`.

- On D = {x ∈ [−3, −1.5], y ∈ [0.5, 3], t ∈ [3, 10]}, which lies inside her region with margin, every non-twist row
  of every sector j ≥ 80 has a positive Gershgorin margin. This is **proved** by interval bisection: region A
  ξ ≤ 20, region B ξ ≥ 20 relaxed, smallest lower bounds 0.72 to 1.59.
- The closed-form G is a lower bound on the actual Gershgorin quantity: checked on random rows, excess
  ≤ 2.8 × 10⁻¹³. Its ingredients (w_a, C, Ω±) are exact symbolic expressions.
- Plain Gershgorin needs large j unless the domain stays well away from the edge. Sampled cutoffs: l ≈ 640 for
  x ∈ [−2, −0.5]; l ≈ 160 for [−2, −1]; l ≈ 40 for [−3, −1.5]. That is why D stops at −1.5.
- The small-j half (j < 80 on D) is in progress, with a mean-value interval certificate.


## Correction (21 November, after the edge-theorem workflow; verified independently)

The positive edges in the table above (t ≳ 6 at y ≥ 1.5) are **artefacts of the cut-off j ≤ 12**. For x > 0 the
twist is the floor of no field in every sector. This is **proved**: in the block of charge q = l, the state of weight
m = l and type (1, −1) has Rayleigh quotient minus λ equal to −8lx/√t + O(1), which is negative for large l. Two
independent builders reproduced it; for example, at (x, y, t) = (0.1, √10, 10), sector j = 120 lies 0.58 below her.

- **Bound.** The all-sector edge is at most min(x*₁₂, 0).
- **Equality is a conjecture for t ≥ 3 only.** It is refuted near t = 2.01 at y = 0.25, where a q = l block undercuts
  her first in sector j = 16.
- **Her region is also bounded below.** λ grows like x², while the gauge-orthogonal state θ̂⁰ ⊗ τ₀ of sector 0 grows
  linearly in x. So her region in x is an **interval**, not a half-line. Every earlier scan stopped at x = −3, which
  hid the lower end.

The Berger crossings t_A and t* and the polynomial P₂ are unaffected.

Receipts: `research/edge-theorem-20261121/` (WP1, WP2, WP4 and their verifiers, `workflow_reports.json`).
