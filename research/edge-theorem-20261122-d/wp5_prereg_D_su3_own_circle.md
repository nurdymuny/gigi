# Pre-registration D: SU(3) stretched along her own charge circle (WP5, 22 November 2026)

Written before any floor, edge or competitor of this family has been computed.

The only computations behind it are the design checks in `scripts/edge_wp5_design_checks.py` (receipt
`wp5_design_checks.json`). They are structural: no floor, edge or competitor.

**Why this test.** Test C (`research/edge-theorem-20261121/wp4_prereg_C_su3.md`, results in
`docs/research/edge-prereg-C-results-2026-11-22.md`) stretched SU(3) along the hypercharge circle λ₈ and freed the
field there. She was the floor nowhere. C's own write-up named the obvious follow-up: stretch along the circle of
**her own charge** H = −i diag(1, 0, −1). **This test was chosen after C failed.** It is a new hypothesis, registered
here before any of its data, and it does not rescue C: C's failure stands.


**Revision after review (22 November, before any floor, edge or competitor of this family was computed).** An
adversarial review found one fatal problem and four major ones. Every fix below supersedes the text it names. The
reviewer's structural checks used random off-grid members only; none was a floor or an edge.

1. **D-P2 replaced (the old version was fatal).**
   - ⟨v, Hv⟩ is **constant** in x, not just affine (see F2 below), and λ_b in sector (0,0) is
     x² + (2/√t + √t)x + c. So the bounding midpoint m_b = −(1/√t + √t/2) < 0 at every shape.
   - The old D-P2 was therefore "not informative" by construction. Its informative rule was also wrong: m_b < 0 does
     not force x_mid < 0.
   - **New D-P2 ("only when", read from "exactly when"):** σ·x_hi ≤ 0, i.e. her floor set lies in {σx ≤ 0}.
   - It is scored only where σ·r_hi > 0, so that the bound does not force it, **and** where the SU(2) calibration
     below has x_hi ≤ 0.
   - **SU(2) calibration.** span(e_H, Y₄, Y₅) is an su(2) with [Y₄, Y₅] = e_H. It corresponds to the SU(2) family at
     (x_SU2, y_SU2) = (2x, 2y₁₃), with eigenvalues × 4.
     - At each shape the same scan code is run on SU(2), sectors j ≤ 4, at y_SU2 = 2y₁₃ and the same t. Its upper
       edge x_hi,SU2 / 2 is reported.
     - D-P2 is scored only at shapes where that is ≤ 0, because a positive finite-sector edge is a known truncation
       artefact in SU(2).
     - The rest are descriptive.
   - **If no shape is scorable, D-P2 is "not informative", and no verdict may claim the direction clause.**
2. **D-lin no longer stops on "no real root".**
   - If λ_b − ⟨v, Hv⟩ has no real root, she is never the floor of sector (0,0) at that shape. Then I = ∅ by the
     (0,0) bound, it is counted in D-P1, and the scan is skipped there.
   - Only the second difference (≤ 10⁻⁹) and ‖G\*v‖ (≤ 10⁻¹²) remain stop gates.
   - When r_hi − r_lo < 0.6, the three copy-check points are r_lo, the midpoint and r_hi.
3. **New code gates, so that a wrong-axis stretch is caught.**
   - **D-K7:** the same rotated code with g = t on **e_K** (which is Weyl-conjugate to C's λ₈ stretch), at the
     Maurer–Cartan field, must reproduce `su3_test2_family2b.json` (⟨b, Hb⟩ and the slice floor per sector) at
     t ∈ {2, 4, 8} to 10⁻⁹ relative.
   - **D-kappa:** the linear coefficient of λ_b(x; p, q) − λ_b(x; 0, 0) equals (p + q)/√t to 10⁻⁹ at every shape.
   - Both are checks, not evidence.
4. **Truncation asymmetry, stated now.**
   - Adding sectors can only shrink I. So a D-P1 **fail** is robust, but a **pass** may be a truncation artefact (in
     SU(2), finite-j truncation enlarged her region).
   - **New registered check D-ext:** at every grid x in I, the sectors (5, 0) and (6, 0) are also evaluated, and
     I_ext ⊂ I is reported.
   - A D-P1 pass with I_ext = ∅ everywhere is reported as a **truncation-sensitive pass**, not a pass.
5. **Scope of the verdict.**
   - The grid uses y₁₂ = y₂₃. That is a **forking-path choice** (it halves the shape space). So is the choice of the
     four shapes below.
   - The fail verdict is rescoped to "in this 4-parameter slice (diagonal Cartan block, zero phases, y₁₂ = y₂₃), at
     these shapes, in these sectors". The phrase "every SU(3) family tried" is withdrawn.
6. **The grid is mapped onto SU(2), and the resolution is improved.**
   - y₁₃ ∈ {¼, ¾, 1, 3/2} gives SU(2) heights y_SU2 ∈ {0.5, 1.5, 2, 3}, inside the SU(2) domain where her region is
     known.
   - **The shapes are now (w, y₁₂ = y₂₃, y₁₃) ∈ {(½, ½, ¼), (½, ½, ¾), (½, 1, 1), (1, 1, 3/2)}** (a choice, labelled
     as such; it replaces the old four).
   - The step stays 0.25 (0.5 in SU(2) units). Wherever I is empty on the grid, a **golden-section maximisation of φ_S**
     on the bracket around its best grid point is run (to 10⁻⁶ in x). A positive maximum makes I nonempty, and it is
     scored. This is so that a narrow region cannot be missed and read as a P1 failure.
7. **F2's detail corrected.** v has only the (e_H, e_H) frame–gauge component. The ad(x e_H) terms act only on frame
   components k ≠ e_H, and the F terms fill only frame blocks i ≠ j. So ⟨v, Hv⟩ is constant in x. The conclusion
   (a bounded region) stands.
8. **D-mult made precise.**
   - At each of the three x values, b and the state of charge −(p + q + 4) must both be exact eigenvectors and tied
     to 10⁻⁹. Otherwise stop.
   - Exactly these two values are removed.
   - The total count n_b may vary (accidental ties) and is reported only.
9. **D-sigma is automatic.** κ_pq = (p + q)/√t, so σ = +1 is structural and **not evidence**. "Against her charge"
   means x < 0.
10. **Normalisation, stated together.** The root plane (1,3) has weight 2 under X_H, which is charge 1 under the unit
    generator e_H. "Exactly as in SU(2)" holds after the factor-2 map in item 1.
11. **D5e is trivial.** Energies are basis-independent at t = 1. It is kept only as a sanity line, not a check.

**Labels.**
- **PROVED:** exact symbolic, or rigorous interval.
- **COMPUTED:** double precision.
- **CONJECTURE:** stated, not shown.

## 1. The family

**Group, basis and metric.**
- SU(3) with X_a = −(i/2)λ_a and ⟨X, Y⟩ = −2 tr XY (Test 2 conventions).
- New orthonormal Cartan basis:
  - e_H = X_H/|X_H|, with X_H = −i diag(1, 0, −1) and |X_H| = 2. So e_H = ½X₃ + (√3/2)X₈.
  - e_K = (√3/2)X₃ − ½X₈.
- The six root vectors are unchanged.
- **Metric:** g = t on e_H and 1 on the other seven. The stretched circle is **the circle of her own charge**.
- The metric is Ad(T²)-invariant, with T² the maximal torus (span of e_H and e_K). H is regular, so T² is the whole
  centralizer.

**Symmetric fields (D5b, COMPUTED):**
- The left-invariant fields commuting with the joint frame-and-gauge action of t² form a space of dimension **10**.
  It splits into:
  - a 2 × 2 Cartan block (4 parameters);
  - aI + bJ on each of the three root planes (6 parameters).
- Constant gauge rotations by T² remove two of the three root-plane phases. That leaves **8 parameters up to gauge**.
  I have not proved this count beyond that argument.

**The registered sub-family (D5c), with 5 parameters:**

  M_{e_H} = x·e_H,  M_{e_K} = w·e_K,  M = y₁₂·I on root plane (1,2),  y₂₃·I on (2,3),  y₁₃·I on (1,3),

with components in the orthonormal coframe.
- **x is the field along her own circle.**
- **y₁₃ is the field across her.** Her frame and gauge plane is the root plane (1,3), which has H-weight ±2. The
  planes (1,2) and (2,3) have H-weight ±1 (D5c).
- **This is not the whole symmetric class.** The Cartan block is taken diagonal, and every root-plane phase is set to
  zero (including the one gauge-invariant phase). Nothing here claims generality.
- The Maurer–Cartan field ½θ is the member x = 1/(2√t), w = y = ½. Its energy at t = 1 equals Test 2's, 0.375 (D5e).

**Her state.** In sector V_(p,q), b = (highest weight) ⊗ E_θ ⊗ E_θ, with charge p + q + 4 under Q (the joint circle
of H). At a random member (D5d, COMPUTED) the following hold in all nine sectors:
- the sign fit of Q is unique;
- the top charge is unique;
- b is an exact gauge-orthogonal eigenvector;
- **her value occurs exactly twice in the slice spectrum** (n_b = 2: she and her mirror, as in SU(2));
- **the second x-difference of λ_b is 2**, so her x² coefficient is **1**. Her charge under the unit generator e_H is
  2/|X_H| = 1, exactly as in SU(2).

## 2. Structural facts (CONJECTURE until their gates pass)

- **F1 (minimal coupling, as in SU(2)).** The x² coefficient of λ_b is 1, in every sector at every member. Gated as
  D-quad.
- **F2 (her region is bounded).** The state v = θ^{e_H} ⊗ e_H in sector (0,0) is gauge-orthogonal for every member:
  G\*v = −[x e_H, e_H] = 0. Its Rayleigh quotient is affine in x, because x enters H only through ad(M_{e_H}) in the
  pairs (e_H, k) and linearly through F. So λ_b − ⟨v, Hv⟩ → +∞ as |x| → ∞, and her region in x is bounded. Gated as
  D-lin, with the bounding interval [r_lo, r_hi] from the quadratic and affine fits.
- **F3 (one direction of "against her charge").**
  - κ_pq = [λ_b(1; p, q) − λ_b(1; 0, 0)] − [λ_b(0; p, q) − λ_b(0; 0, 0)] is the lift discount coefficient.
  - **"Turning against her charge" means σx < 0**, with σ the common sign of κ_pq over the eight sectors (p, q) ≠
    (0, 0).
  - Gated as D-sigma. If the sign is not common, D-P2 is untestable.

## 3. Predictions (from the sentence, read in this family; each can fail)

The sentence, as revised on 21 November:

> She is the law that a field gives way first along the twist that spins with it, and does so exactly when the field
> along her own circle turns against her charge — by enough, but not too much, as the shape of space sets.

- **D-P1, existence.** With x free, her floor set I (all nine sectors, gauge-orthogonal spectrum) is nonempty.
  - *Strict:* at every grid shape with t ≥ 3.
  - *Weak:* at one or more grid shapes.
- **D-P2, side.** At every **informative** shape with I nonempty, the midpoint x_mid of I satisfies σ·x_mid < 0.
  Informative means the bounding midpoint (r_lo + r_hi)/2 does **not** already satisfy σ·m_b < 0.
- **D-P3, one interval.** I is a single interval on the scan.
- **D-P4 (S).** At the upper edge (the side of larger σx) the competitor's slope ratio is < 0.5.
  - Post hoc, as in B and C, so a pass is weak evidence.
  - (N), the competitor's charge, is descriptive only.
- **D-P5 (descriptive).** The upper edge against t, at fixed shape.

**Every D claim is "in the nine listed sectors, on the 0.25 grid".** Sectors (p, q) ≠ those nine are not checked,
and a region narrower than 0.25 cannot be seen.

## 4. Grid and measurement

**Grid.**
- t ∈ {2, 3, 4, 6, 8}.
- (w, y₁₂ = y₂₃, y₁₃) ∈ {(½, ½, ½), (½, ½, 1), (½, 1, 1), (1, 1, 2)}.
- That is 20 shapes, with the nine sectors of Test 2.

**Copies of λ_b.** n_b is measured at x ∈ {r_lo + 0.3, (r_lo + r_hi)/2, r_hi − 0.3} as the number of slice
eigenvalues within 10⁻⁹·max(1, |λ_b|).
- Her **copies** are b and the state of charge −(p + q + 4), her mirror, when its value equals hers. **Exactly those
  two are removed.**
- Anything else at her value is a **tied competitor**, kept in the spectrum and reported. This is C's Amendment C2,
  adopted here from the start.

**Scan and measurements.** The rest is exactly as in Test C, using the same code with the basis and metric changed:
- **Scan:** 0.25 grid on [r_lo − 0.5, r_hi + 0.5], with brentq to 10⁻⁸ at each sign change of
  φ_S = min over sectors [the lowest non-copy slice eigenvalue − λ_b + 10⁻⁹·max(1, |μ|)].
- **H + GG\*:** the same, reported alongside.
- **Competitor at x_hi ± 10⁻⁴:** its sector and Q-charge. Its slope is computed two ways: Hellmann–Feynman on H
  (scored) and the direct slice-eigenvalue derivative (reported).
- **Implementation:** spectra block by block in the eigenbasis of Q.

## 5. Gates (all stop the run before any prediction is read)

| gate | what must hold |
| --- | --- |
| **D-K1–K5** | Test C's controls K1–K5 rerun unchanged and pass. That is the same code, so these check that nothing broke |
| **D-K6** (rotated basis) | the Maurer–Cartan member at t = 1 in the new basis has the same nine-sector slice spectra as Test 2's at t = 1, to 10⁻¹² relative. The metric is bi-invariant at t = 1, so the change of basis must not matter |
| **D-sym** | ‖[Q, H]‖_F ≤ 10⁻¹⁰‖H‖_F; the top charge is unique; b's residual ≤ 10⁻⁹‖H‖₂; ‖G\*b‖ ≤ 10⁻⁹‖G‖₂; at every evaluated point and sector |
| **D-quad** (F1) | the second x-difference of λ_b at step 1 about x = 0 equals **2** to 10⁻⁹, in every sector at every shape |
| **D-lin** (F2) | ⟨v, Hv⟩ has second x-difference ≤ 10⁻⁹, and ‖G\*v‖ ≤ 10⁻¹², at every shape; λ_b − ⟨v, Hv⟩ has two real roots |
| **D-sigma** | a common sign σ over the eight sectors at every shape (if not, D-P2 is untestable; not a stop) |
| **D-mult** | at every shape and sector, the copies (b, and her mirror if tied) are found, the same at the three x values; the extra ties are reported |
| **D-bound** | I ⊂ [r_lo, r_hi] at every shape |

## 6. Verdict rule (fixed now)

- **D-P1 weak fails** (no x, no shape): counts against the sentence in this SU(3) slice, **even along her own circle**
  (scope as in revision item 5).
- **D-P1 weak holds, strict fails at some t ≥ 3:** counts against the universal reading (that for every shape some x
  works), not against the direction clause.
- **D-P2 fails** at an informative shape: counts against "turns against her charge" in SU(3).
- **D-P3 fails:** counts against the interval form.
- **D-P4 (S) fails:** counts against "nearly blind" (already failing in SU(2); see B).
- **All of D-P1 (both), D-P2 (where informative), D-P3 and D-P4 (S) hold:** the sentence is supported in one SU(3)
  family, stretched along her own circle, in nine sectors (COMPUTED). That would point to **her own circle** as what
  carries over between groups, and C's failure would read as "the wrong circle". That reading would itself be a
  hypothesis for a further test.
- **Not evidence either way:** b's exactness (automatic), F1 and F2 (structural), and the D5 design numbers.

## 7. Runtime rules

- One process, OMP_NUM_THREADS=1.
- Script: `scripts/edge_wp5_su3_own_circle.py`.
- Receipt: `research/edge-theorem-20261122-d/wp5_su3_own_circle.json`.
- Expected cost is about that of Test C (about 5 h).
