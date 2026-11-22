# Pre-registration E: the two hands (WP6, 22 November 2026; revised after adversarial review, before any data)

Written before any sector floor, floor eigenvector or onset of this test has been computed.

**What this rests on.**
- The structural design check `wp6_design_check_w_table.txt`: the diagonal data w_a and C_aa of the closed-form rows.
  It computes no eigenvalue.
- The adversarial review of the first draft. It used row algebra, a second-order perturbation estimate built from the
  rows (no eigenvalues), and isospectrality and exactness checks at off-grid points.
  - **That estimate forecasts some outcomes.** I disclose it below and use it openly to set the predictions. The
    predictions are therefore informed by row algebra, not blind.

**The idea being tested** (`docs/research/the-two-hands-2026-11-22.md`, Part II, corrected here):

> The cheap lift reaches across: the linear discount of a state at orbital weight m is +4·c_l·x·m/√t. It favours the
> states whose internal turning, relative to their orbital turning, is opposite to the field along the circle:
> **c_l·m·x < 0**. For x < 0 that is Helike (m = l, type (1, 1)). For x > 0 it is the counter-rotating states, above
> all the sister (m = l, type (1, −1)).

**Labels.**
- **PROVED:** exact symbolic, or rigorous interval.
- **COMPUTED:** double precision.
- **CONJECTURE:** stated, not shown.

## 1. Already known, disclosed in full

1. **The x > 0 theorem (PROVED, for H + GG\*).** The sister's Rayleigh quotient is below λ_j by −8lx/√t + O(1).
   - **WP2 §0** also proves that type (1, 0) in block l + 1 undercuts her (K = −4x/√t).
   - WP2 already computed λ_min − λ_j = −34.1 at j = 400 at (0.1, √10, 10), next to this grid. It checked the
     gauge-orthogonal value at j = 120 and 200 with the full builder.
2. **The tail check** (`wp1_tail_check.json`, near-top-charge heuristic, no eigenvectors).
   - Of 131 j ≤ 12 edges, **57 are positive**, and every edge with t ≤ 6 is negative.
   - The minimum over sectors 13 to 200 was at:
     - (13, ±(l + 1)): 72 times;
     - (200, ±l): 54 times;
     - (200, ±(l + 1)): 5 times.
   - At t = 2, block −l also undercut her at x ≈ −0.6 for y ≤ 0.5.
3. **WP2 §1:** the true margin at large l is l·K + O(1) at second order. **wp1_top_family** has the polynomial families
   of blocks q = l and q = l + 1. **wp1_family_members.json (key `mirror`):** blocks q and −q are isospectral
   (algebra).
4. **y = 0.** (x, 0) and (−x, 0) are gauge-equivalent (prereg B, D1b). At y = 0 there is an extra conserved charge
   q′ = m + c_f − c_l. The sister is its unique top state, so it is exact, with value λ(−x, 0, t, j). **So every
   y = 0 point is the mirror of a point (−x, 0)**, outside D, and is not independent evidence.
5. **The design check and the review's row algebra (exact or second-order, no eigenvalues).**
   - At m = l, the linear-in-l diagonal term is −4w_a·l/√t, and the largest w_a wins. For x > 0 that is the sister,
     **except at t = 2**, where the sister ties type (0, −1) on w. Sister minus (0, −1) at m = l is
     −4√2·x + 2y² − 6, the same at every l. So t = 2 is degenerate.
   - **The sister's nearest diagonal rival (her copies excluded) is type (1, 0) at m = l, in block l + 1.** Exactly:
     sister − (1, 0) = [−4lx + √t(x² + y² − 2√t·x) − 4x]/√t. It is zero at l₁₀ = (x² + y²)√t/(4x) − t/2 − 1.
   - **Second order.** In block l the sister couples to (1, 0) at m = l − 1 with modulus 4y√l, across a gap of about
     8l(1 − 1/t). Its level drops by about 2y²t/(t − 1), independent of l.
   - **So the onset is not the diagonal crossing.** Its leading scaling is still **∝ y²/x** at fixed t, which is what
     E2 tests.
   - The first-order amplitude puts 1 − S at about y²t²/(4l(t − 1)²) from that channel alone, so a high purity S is
     nearly forced. E3 therefore tests the **size** of the impurity against a registered formula, not that S is
     large.

## 2. Points, sectors, operator

- **Grid:** x ∈ {0.05, 0.2, 0.5, 1.0, 2.0} × y ∈ {0, 0.5, 1.5, 3.0} × t ∈ {2, 3, 4, 6, 10}, which is 100 points.
  - **Scored:** y > 0 and t ≥ 3. That is 5 × 3 × 4 = **60 points**.
  - **Descriptive only:** the 20 points with y = 0 (mirrors) and the 20 with t = 2 (degenerate).
- **Sectors:** j = 0 … 400, all charge blocks, H + GG\*.
  - The closed-form blocks come from `scripts/ricci_su2_exact_edge.py`.
  - Their Weitzenböck identity is verified symbolically for the family (edge-paper review, `wz_family.py`). The
    charge-basis step is being written up.
- **The sector floor includes her.** At x > 0 she is the co-rotating hand, and the question is which hand holds the
  floor.
- **Degeneracy.** The sector floor's eigenspace is every eigenvector, over all blocks, whose eigenvalue is within
  10⁻¹⁰·max(1, |μ|) of the lowest. Blocks ±l and ±(l + 2) come in exact mirror pairs, so the eigenspace is at least
  two-dimensional. All weights below are **projector weights**: the trace of the projector onto the relevant states,
  divided by the dimension of the eigenspace.

## 3. Measurements

- **A_j:** the across-reaching weight of the floor eigenspace, on states with c_l·m·x < 0.
- **S_j:** the sister weight, on the states (m = l, (1, −1)) and (m = −l, (−1, 1)).
- **j₀:** the smallest j such that A_j′ ≥ 0.5 for every j′ in [j, 400]. There is **no block restriction**. If no such j
  exists, the point has no onset.
- **The registered first-order impurity** at j = 400, from the rows alone:
  - (1 − S)_pred = Σ_k |B_{s,k}|² / (D_k − D_s)².
  - The sum runs over every state k ≠ s of the sister's block q = l, with s the sister.
  - B is the closed-form block and D its diagonal.

## 4. Predictions (each can fail; the aggregation is fixed now)

- **E1 (a hand reaches across and holds the floor).** At **every one of the 60 scored points**, j₀ exists and
  j₀ ≤ 300.
  - The fraction is reported as well. A single failure fails E1.
- **E2 (the two fields set the onset, ∝ y²/x).**
  - **E2-y:** at each scored (x, t) where j₀ exists at both y = 1.5 and y = 3.0, with j₀(1.5) ≥ 10, the ratio
    j₀(3.0)/j₀(1.5) ∈ [2, 8]. The leading prediction is 4.
  - **E2-x:** at each scored (y, t) where j₀ exists at both x = 0.2 and x = 1.0, with j₀(1.0) ≥ 10, the ratio
    j₀(0.2)/j₀(1.0) ∈ [2.5, 10]. The leading prediction is 5.
  - **Each holds if at least 80% of its eligible pairs pass**, with at least 5 eligible pairs. Fewer than 5 makes it
    "not scored".
- **E3 (the size of the cross field's impurity).** At each scored point where the floor eigenspace at j = 400 lies in
  blocks ±l, the measured 1 − S₄₀₀ is within a factor 1.5 of (1 − S)_pred.
  - It holds if at least 80% of the eligible points pass, with at least 10 eligible.
  - This tests whether first-order mixing within the sister's block accounts for her impurity. That is the precise
    form of "the cross field decides which hand is exact".

## 5. Gates (all stop the run before any prediction is read)

| gate | what must hold |
| --- | --- |
| **E-code-1** | closed-form sector spectra = the general builder's full H + GG\* (`T2.hessian_and_gauge`) to 10⁻¹² relative: at 10 random x > 0 points with j ≤ 12, **and** at 2 points with j ∈ {120, 200} |
| **E-code-2** | her block's entry = λ_j to 10⁻¹² **relative**, at every point and sector |
| **E-y0** | at y = 0, the sister is an exact eigenvector of block q = l (residual ≤ 10⁻¹² relative to ‖B‖), with value λ(−x, 0, t, j) to 10⁻¹² relative, for every j ≤ 400 (exactness from q′; the value from D1b) |
| **E-mirror** | blocks q and −q have equal spectra to 10⁻¹² relative, at every point for j ∈ {50, 200, 400} (formerly E4; algebra, not evidence) |
| **E-ctrl** | at (−2, 1, 4), (−1.75, 2.5, 8) and (−2.5, 0.75, 6), inside D, her value is the floor of every sector j ≤ 400 (Theorem on D) |

## 6. Verdict rule (fixed now)

- **E1 fails:** at some scored point no across-reaching state holds the floor from some j ≤ 300 on. **That counts
  against the cross-reach reading** there.
- **E2 fails** (E2-y or E2-x): the onset does not scale with the two fields as the row algebra says. The "y decides,
  x reaches" story is incomplete.
- **E3 fails:** her sister's impurity is not first-order mixing within her block. Something else, another block or a
  higher order, shapes the floor.
- **All hold:** the cross-reach is supported as the structure of the floor at x > 0 for j ≤ 400 and t ≥ 3, with the
  onset scaling y²/x and first-order purity (COMPUTED). The asymptotic statement remains to be proved.
- **Not evidence:** the gates (E-y0, E-mirror and E-ctrl are proved facts or algebra), the y = 0 points and the t = 2
  points, which are descriptive.

## 7. Runtime rules

- One process, OMP_NUM_THREADS=1.
- Script: `scripts/edge_wp6_two_hands.py`.
- Receipt: `research/edge-theorem-20261122-e/wp6_two_hands.json`.
- The review's scratch scripts are kept in `research/edge-theorem-20261122-e/review_scripts/`.
