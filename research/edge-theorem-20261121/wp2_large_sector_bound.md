# WP2: a sharper large-sector bound (21 November 2026)

Scripts: `scripts/edge_wp2_symbolic.py`, `scripts/edge_wp2_bounds.py`, `scripts/edge_wp2_interval_proof.py`.
Receipts in this folder: `wp2_symbolic.txt`, `wp2_bounds.json`, `wp2_interval_proof_*.json` with their `.log.txt`.
Labels: PROVED (exact symbolic or rigorous interval), COMPUTED (double precision), CONJECTURE.

## 0. A correction to the edge first (PROVED, then COMPUTED)

The j ≤ 12 edge x*(t, y) is positive for t ≳ 7 at y ≥ 1. The grid in `wp2_bounds.json` (`edge_grid`) gives, for
example, x*(10, 3) = 0.16 and x*(8, 1.5) = 0.0315. But **for x > 0 she is not the floor of every sector.**

- PROVED (`wp2_symbolic.txt`, S3–S5). At m = l, the diagonal of type (1, −1) in block q = l, minus λ_{2l}, is exactly
  l·(−8x/√t) + (−4√t x + 4y² − 8x/√t). Type (1, 0) in block q = l + 1 gives l·(−4x/√t) + (−2√t x − x² + 3y² − 4x/√t).
  A Rayleigh quotient with one basis vector bounds λ_min(block) above by its diagonal. So for x > 0 the twist is not the
  floor of H + GG* in any sector j = 2l with l > (4y² − 4√t x − 8x/√t)·√t/(8x).
  - Example (x, y, t) = (0.1, √10, 10): this holds for every l > 152.11.
- COMPUTED at the same point:
  - with `Blocks.block`, λ_min − λ_j = −34.1 at j = 400;
  - with the independent full builder (`ricci_su3_test2_family.hessian_and_gauge`), −0.583 (H + GG*) and −0.581
    (gauge-orthogonal) at j = 120, and −8.95 at j = 200. The value at j = 60 is 0, from the twist itself.
- COMPUTED: at x = 0 (t = 10, y = √10), the margin rises to 7.78 by j = 20000. At x = −0.01 and x = −0.03 it stays
  positive and grows. At x = +0.01 it turns negative between j = 1000 and j = 4000.
- So the all-sector edge is **min(x*(t, y), 0)**, not x*(t, y). This is a CONJECTURE for intermediate j at points
  not sampled. The "≤ 0" half is PROVED by the Rayleigh bound above.
- The j ≤ 12 table answers a different question once x* > 0. The domains "x ≤ x* − δ" in the task reach x > 0 when
  δ < x*. There, no cutoff L0 exists: the truth fails at large l.

## 1. Why plain Gershgorin is slow

Exact (S3): the row (m = l − ξ, a) has

    D − λ_{2l} = 4ξ(2l − ξ)(1 − 1/t) + l·K_a + E_a(ξ) + R_a,
    K_a = 4 − 8/t − 4(w_a + x)/√t.

- The ladder couplings are 2√(ξ(2l − ξ + 1))·|Ω₋| and 2√((2l − ξ)(ξ + 1))·|Ω₊|.
- At ξ = 0 the gap is l·K_a, with K_(1,0) = −4x/√t and K_(1,−1) = −8x/√t; these are small near x = 0. The coupling is
  O(√l). So Gershgorin needs √l·K ≳ const, which gives L0 ~ 1/K².
- The true margin is l·K + O(1): second order, coupling² / gap = O(l)/O(l). So the right L0 is ~ 1/K, or O(1) when
  the O(1) term is positive.

## 2. Two sharper bounds

**(GW) weighted Gershgorin with an l-dependent diagonal similarity.**

- Weights: s(ξ) = ∏_{k < min(ξ, X1)} κ_k/√l, mirrored as s(2l − ξ) = s(ξ).
- Row bound:

      G_w = g − R_a − 2√(ξ(2l − ξ + 1))·P₋_a·√l/κ_{ξ−1} − 2√((2l − ξ)(ξ + 1))·P₊_a·κ_ξ/√l.

- The O(√l) coupling becomes O(1) in the ξ = 0 row, and O(l) against the O(l) gap in the ξ = 1 row.
- Valid for any positive weights: a similar matrix has the same eigenvalues, and Gershgorin applies on the real line.
- COMPUTED: on 60 random (x, y, t, l), G0, GW and the comparison-matrix bound all lie below the exact block minimum
  (0 violations).

**(LDL) exact treatment of the near-end blocks, plain Gershgorin in the bulk.**

- Take the blocks with n0 = l − q ≤ N1 or n0' = l + q ≤ N1; each has at most 9 states. M = (B − λ_j)/l is a matrix in
  U = 1/l:
  - pivots K_a + 8ξ(1 − 1/t) + O(U);
  - ladder entries O(√U), exact (S2: real symmetric).
- It is proved positive definite by interval LDLᵀ. Every other row has min(ξ, 2l − ξ) ≥ N1 − 1. There the diagonal
  gap 8lξ(1 − 1/t) beats 4√(2lξ)·P already at small l, and plain G is proved positive (FA/FB of family_large_j).
- This loses nothing on the near-end blocks: it proves the exact eigenvalue statement.
- Checks in every run:
  - M·l + λ equals `Blocks.block` to 4.5e−16 (relative);
  - G(l, 2l − ξ, a) = G(l, ξ, −a), symbolically;
  - G_exact ≤ the actual Gershgorin quantity, to 2.8e−13.
- The block q = −l − 2 (type (−1, −1) at m = −l) has entry exactly λ_j. It is the twist's mirror copy, and it is
  excluded, as in `phi_A`.

## 3. Cutoffs (COMPUTED, sampled; see the table below)

- L0 is the smallest grid l beyond which the bound is positive at every grid l.
- Sampling: the (t, y) grid t ∈ {3, …, 10}, y ∈ {0.5, …, 3}; x at the top of the range, at the top − 0.5, and at −3.
- GW uses the best (θ, X1) per point, with κ_k = θ/√(k + 1).
- "EX" is the cutoff of the exact block minima. EX = 1 means the exact margin was positive at every sampled l ≥ 1 at
  those points.
- "literal": x ≤ x* − δ. "corrected": x ≤ min(x*, 0) − δ.

| domain | max L0 plain G0 | max L0 weighted GW | max L0 exact EX | grid points with x_top >= 0 |
|---|---|---|---|---|
| delta=0.5 literal | 763.5 | 103.0 | 1.0 | 0 of 48 |
| delta=0.5 corrected | 353.5 | 68.5 | 1.0 | 0 of 48 |
| delta=0.25 literal | 11138.5 | 388.0 | 1.0 | 0 of 48 |
| delta=0.25 corrected | 1414.0 | 140.5 | 1.0 | 0 of 48 |
| delta=0.1 literal | inf | inf | inf | 3 of 48 |
| delta=0.1 corrected | 8978.0 | 353.5 | 1.0 | 0 of 48 |

Worst sampled points for δ = 0.1 corrected (G0 / GW): (t=10, y=3.0, x_top=-0.100): 8978.0 / 353.5; (t=9, y=3.0, x_top=-0.100): 8185.5 / 322.5; (t=8, y=3.0, x_top=-0.100): 7236.5 / 303.0

## 4. Interval proofs (PROVED; mpmath iv, 80 bits, bisection)

Statement: on each box, for every sector j ≥ 2·L0, every non-twist block has λ_min > λ_j.

| box (x × y × t) | L0 (sectors j ≥ 2·L0) | δ below grid edge (COMPUTED) | result | worst (−1)-block pieces | seconds | receipt |
|---|---|---|---|---|---|---|
| [-3.0, -0.575] × [0.5, 3.0] × [3.0, 4.0] | 20 (N1=8, XA=20) | 0.1006 | PROVED | 498 | 23.1 | `wp2_interval_proof_slab_t3-4_x-0.575_L20.json` |
| [-3.0, -0.495] × [0.5, 3.0] × [3.0, 4.0] | 20 (N1=8, XA=20) | 0.0206 | PROVED | 551 | 84.7 | `wp2_interval_proof_slab_t3-4_y0.5-3_x-0.495_L20.json` |
| [-3.0, -0.355] × [0.5, 3.0] × [4.0, 5.0] | 20 (N1=8, XA=20) | 0.1010 | PROVED | 359 | 19.1 | `wp2_interval_proof_slab_t4-5_x-0.355_L20.json` |
| [-3.0, -0.274] × [0.5, 3.0] × [4.0, 5.0] | 20 (N1=8, XA=20) | 0.0200 | PROVED | 384 | 63.3 | `wp2_interval_proof_slab_t4-5_y0.5-3_x-0.274_L20.json` |
| [-3.0, -0.229] × [0.5, 3.0] × [5.0, 6.0] | 20 (N1=8, XA=20) | 0.1007 | PROVED | 404 | 21.0 | `wp2_interval_proof_slab_t5-6_x-0.229_L20.json` |
| [-3.0, -0.149] × [0.5, 3.0] × [5.0, 6.0] | 20 (N1=8, XA=20) | 0.0207 | PROVED | 954 | 21.1 | `wp2_interval_proof_slab_t5-6_y0.5-3_x-0.149_L20.json` |
| [-3.0, -0.183] × [0.5, 3.0] × [6.0, 7.0] | 20 (N1=8, XA=20) | 0.1002 | PROVED | 590 | 24.4 | `wp2_interval_proof_slab_t6-7_x-0.183_L20.json` |
| [-3.0, -0.103] × [0.5, 3.0] × [6.0, 7.0] | 20 (N1=8, XA=20) | 0.0202 | PROVED | 1579 | 31.6 | `wp2_interval_proof_slab_t6-7_y0.5-3_x-0.103_L20.json` |
| [-3.0, -0.154] × [0.5, 3.0] × [7.0, 10.0] | 20 (N1=8, XA=20) | 0.1005 | PROVED | 2892 | 55.7 | `wp2_interval_proof_slab_t7-10_x-0.154_L20.json` |
| [-3.0, -0.074] × [0.5, 3.0] × [7.0, 10.0] | 20 (N1=8, XA=20) | 0.0205 | PROVED | 12445 | 212.0 | `wp2_interval_proof_slab_t7-10_y0.5-3_x-0.074_L20.json` |
| [-3.0, -0.02] × [1.0, 3.0] × [7.0, 10.0] | 20 (N1=8, XA=20) | 0.0196 | PROVED | 126082 | 4445.8 | `wp2_interval_proof_slab_t7-10_y1-3_x-0.02_L20.json` |
| [-3.0, -0.6] × [0.5, 3.0] × [3.0, 10.0] | 10 (N1=10, XA=10) | 0.1256 | PROVED | 4005 | 109.5 | `wp2_interval_proof_x-0.6_L10.json` |
| [-3.0, -0.6] × [0.5, 3.0] × [3.0, 10.0] | 20 (N1=8, XA=20) | 0.1256 | PROVED | 1653 | 40.5 | `wp2_interval_proof_x-0.6_L20.json` |

- The δ column is COMPUTED, not proved. It is how far each box's x-top lies below the minimum of min(x*, 0) over the
  grid points of its slab; the slabs follow the edge. x* increases with t at every grid y (COMPUTED). It is not
  monotone in y: at t = 3, x*(3, 0.5) = −0.3355 and x*(3, 1.0) = −0.3294. Between grid points the margin to the edge
  is not proved.
- Earlier result: `family_large_j.json` proved j ≥ 80 on x ∈ [−3, −1.5]. These runs prove j ≥ 40 (and j ≥ 20 on
  x ≤ −0.6) up to about 0.02 below the edge. On t ∈ [7, 10], y ∈ [1, 3] they reach x ≤ −0.02, against an all-sector
  edge of at most 0.
- Cost grows as the top of x approaches 0. The (−1) blocks needed 126082 pieces at x ≤ −0.02, against ~500 at
  x ≤ −0.6.

## 5. Open

- The small-j half (j < 2·L0) on these boxes is not done here.
- The edge itself: the intermediate-j claim for min(x*, 0) is a CONJECTURE.
- x ∈ (−0.02, 0) is not covered. The degenerate point x = 0 has K_(1,0) = K_(1,−1) = 0. There the near-end margin is
  O(1): 7.78 at (10, √10), COMPUTED. A proof there needs a second-order argument, not U → 0 positivity.
