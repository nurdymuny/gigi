# Pre-registration F: which circle does she live on? A sweep across the chamber (22 November 2026)

Written before any floor set, edge or competitor of this test has been computed.

**Revision after review, before any data.** The first draft was taken apart by an adversarial review: two fatal
problems and seven major ones. This version replaces it. It keeps the question and fixes the design.
- **Fatal 1:** θ^∨ is singular in **both** groups, not only in Sp(2).
- **Fatal 2:** one control 8° from ρ^∨ could not test "nowhere else". By continuity, a region on ρ^∨ spills onto
  nearby circles, so "absent on the control" was really a prediction of a discontinuity.
- **The design is now a sweep** of the stretched circle across the Weyl chamber, wall to wall, with four
  hypotheses registered.

The review's structural checks used an off-grid member only.

**Labels.**
- **PROVED:** exact symbolic, or rigorous interval.
- **COMPUTED:** double precision.
- **CONJECTURE:** stated, not shown.

## 0. What I knew when I wrote this (disclosed in full)

- **Test D is not finished.** When this was committed, 15 of D's 20 shapes were complete and I had looked at 14 of
  them. Along her own charge circle in SU(3), her floor region was nonempty at 13 of those 14, in one interval, at
  x < 0; the one empty shape was at t = 2. **These predictions are not blind to D.**
- **Test C** (SU(3) along hypercharge, a wall): she was the floor nowhere.
- **The idea** comes from `docs/research/the-arc-2026-11-22.md`, Part II.1: she lives on the principal circle.
- **Why SU(3) cannot decide it.** In SU(3) her circle H is at once:
  - the principal circle ρ^∨;
  - the coroot θ^∨ of her highest root;
  - a regular (interior) circle.

  In SU(4) and Sp(2) these come apart.

**The geometry, computed** (`scripts/edge_wp7_groups.py`, structural only):
- In both groups, the angle from θ^∨ to ρ^∨ is 18.43°.
- Rotating e_K from θ^∨ through ρ^∨ reaches the far wall at 45°.

| angle | SU(4) e_K ∝ diag(…) | Sp(2) e_K ∝ (b, a) | regular? | her top root unique? | q_K |
| --- | --- | --- | --- | --- | --- |
| 0° | (1, 0, 0, −1) = θ^∨ | (1, 0) = θ^∨ | **wall** | yes | 1 |
| 9.22° | (1, 0.162, −0.162, −1) | (1, 0.162)* | yes | yes | 0.987 |
| 18.43° | (1, ⅓, −⅓, −1) = ρ^∨ | (1, ⅓) = ρ^∨ | yes | yes | 0.949 |
| 27.65° | (1, 0.524, −0.524, −1) | (1, 0.524)* | yes | yes | 0.886 |
| 36.87° | (1, 0.75, −0.75, −1) | (1, 0.75)* | yes | yes | 0.800 |
| 45° | (1, 1, −1, −1) | (1, 1) | **wall** | **no** (her frame top is degenerate, as on C's hypercharge circle) | 0.707 |

\* The same angles in both groups. The Sp(2) coordinates are given in the order the code uses.

- Sp(2)'s **ρ** (the Weyl vector, as opposed to ρ^∨) lies at 26.57°. It does **not** count as "principal" for H_P.
- The 0° wall is a wall where her frame top is still unique; at 45° it is not. That is the distinction the review
  found between the θ^∨ wall and C's wall.

## 1. The families

**Groups:** G ∈ {SU(4), Sp(2)}. The conventions are those of `scripts/ricci_su3_test2_family.py`, with the
structure constants computed, not typed.

**Circles:** e_K at the six angles above, as unit generators.

**Metric:** g = t on e_K, 1 elsewhere, with **t = 3**.

**Fields (the diagonal sub-family, as in D):**
- x on e_K, **free**;
- w·I on the Cartan complement;
- y_θ·I on the plane of her root (θ, highest for every K in the table; at 45°, one of the tied top roots, fixed by the
  code's ordering and recorded);
- y·I on every other root plane.

**Shapes (w, y, y_θ)**, held fixed across the angles, so that differences come from the circle alone:

  (½, ½, ¼), (½, ½, ¾), (½, 1, 1).

The first two are D's most robust shapes, **chosen knowing D**. The third has w ≠ y.

**Her state:** the top state of Q_K in each sector. Where the top is degenerate (the walls, the sectors measured in
§2), her copies are the whole tied top space, by the C1 rule, together with the mirror (bottom) space when it is
tied.

**Main sectors:**
- SU(4): 1, 4, 4̄, 6, 10, 10̄;
- Sp(2): 1, 4, 5, 10, 14, 16.

**Truncation check** (the Sym^k tower, which extends her charge range):
- SU(4): Sym³ = (3,0,0), dimension 20;
- Sp(2): (3,0) and (4,0), dimensions 20 and 35, built as Sym³ and Sym⁴ of the fundamental.

## 2. Structure (CONJECTURE until gated)

- **S1 (exactness).** Her copies are exact gauge-orthogonal eigenvectors, and her top space is the same at every x.
- **S2 (minimal coupling).** The coefficient of x² in λ_b is q_K², with the values in the table: 2q_K² = 2, 1.974,
  1.8, 1.570, 1.28, 1.0 from 0° to 45°.
- **S3 (bounded region).** v = θ^{e_K} ⊗ e_K in the trivial sector has G\*v = 0 and ⟨v, Hv⟩ constant in x. No real
  root of λ_b − ⟨v, Hv⟩ means the floor set is empty at that point, which is data, not a stop.
- **S4 (σ).** σ = +1 **automatically**: the V-top charge under e_K is ≥ 0 in every sector. This is not evidence.
- **S5 (quadratic in x).** H(x) = H₀ + xH₁ + x²H₂ and G(x) = G₀ + xG₁ exactly. This is used for speed and gated.

## 3. Measurements, per (group, angle, shape)

- **The floor set I:** the gauge-orthogonal spectrum over the main sectors, with her copies removed.
  - A 0.25 grid on the bounding interval, with brentq to 10⁻⁸ at each sign change.
  - If I is empty on the grid, a golden-section maximization of φ_S around the best grid point.
- **The truncation check:** at **every grid x in I, and at the golden point** if that is the only point, the
  extra sectors as well.
- **"Survives":** at least one checked x has her below everything in the extra sectors too.
- **φ_best:** the best margin, reported.
- **The upper-edge rival:** its sector, its charge, its slope ratio, and which sector blocks her. Descriptive only.

## 4. Hypotheses and predictions (fixed now)

**Score per (group, angle):** the number of shapes, out of 3, where I is nonempty **and** survives the truncation
check.
- **Lives:** ≥ 2 of 3.
- **Does not live:** 0 of 3.
- **Marginal:** 1 of 3.

| hypothesis | prediction along the arc (per group) |
| --- | --- |
| **H_P (principal)** | lives at 18.43°; the set of angles where she lives is one contiguous run that contains 18.43° and contains neither wall (0°, 45°) |
| **H_θ (her own SU(2))** | lives at 0° |
| **H_reg (regular)** | lives at every interior angle (9.22°, 18.43°, 27.65°, 36.87°) and at neither wall |
| **H_C (C-type wall)** | does not live at 45° |

- **Each hypothesis is supported only if its prediction holds in both groups.** Any marginal score in a predicted
  cell makes that hypothesis "not decided".
- The hypotheses are not exclusive. H_reg implies H_C; H_P and H_reg can both hold; H_θ conflicts with H_P and H_reg
  at 0°.
- **None holds:** if she lives nowhere, the SU(3) result was special to SU(3).
- **Each group is also reported on its own.** A split between groups is a finding, not a verdict.
- **Asymmetry, stated now.** "Does not live" is robust, because extra sectors only shrink the floor set. "Lives" is
  only as good as the truncation check.

**F-side** (D's rule, with σ automatic):
- At every scored point with I nonempty, σ·x_hi ≤ 0.
- It is scored only where σ·r_hi > 0 **and** the SU(2) calibration edge is ≤ 0. Elsewhere it is "not informative".
- **The calibration map.** Her SU(2) circle is θ^∨ = cos φ·e_K + sin φ·e_⊥, at angle φ from e_K, with e_⊥ in the
  Cartan complement. The SU(2) family at
  - x_SU2 = 2(x cos φ + w sin φ),
  - y_SU2 = 2y_θ,
  - t_SU2 = t cos²φ + sin²φ,
  - with j ≤ 4,

  gives an upper edge, mapped back to x. At φ = 0 this is D's map.
- **Not evidence:** S1–S5 and σ.

## 5. Gates (all stop the run before any prediction is read)

| gate | what must hold |
| --- | --- |
| **F-K0 (known answer)** | the new general-G code, run on SU(3) with K = H at D's first shape (t = 2, (½, ½, ¼)) and x = −1.5, reproduces D's code (`scripts/edge_wp5_su3_own_circle.py`) for λ_b and the slice floor in every sector, to 10⁻⁹ relative |
| **F-K1 (wrong-axis)** | the same SU(3) run with K moved to the hypercharge direction reproduces Test C's code at the same field, to 10⁻⁹ relative |
| **F-K2 (group data)** | brackets close to 10⁻¹³; every representation satisfies ρ([X, Y]) = [ρX, ρY] to 10⁻¹²; dimensions as listed |
| **F-K3 (geometry)** | the angles, regularity, top-root uniqueness and q_K match §0's table |
| **F-S5** | H(x) and G(x) rebuilt from the quadratic and linear pieces equal the direct builds at 3 random x, to 10⁻¹² relative |
| **F-sym** | ‖[Q_K, H]‖ ≤ 10⁻¹⁰‖H‖ at every evaluated point |
| **F-quad** | the second x-difference of λ_b equals 2q_K², to 10⁻⁹ |
| **F-lin** | ⟨v, Hv⟩ has second x-difference ≤ 10⁻⁹, and ‖G\*v‖ ≤ 10⁻¹² |
| **F-mult** | her copy spaces are exact and tied at three x; other ties are reported |
| **F-bound** | I ⊂ the bounding interval |

## 6. Cost and runtime

- One process, OMP_NUM_THREADS=1.
- H, G and Q_K are built once per (sector, shape) in three pieces and rotated into Q_K's eigenbasis. Each x then needs
  only small block eigensolves.
- 2 groups × 6 angles × 3 shapes = **36 points**, all at t = 3.
- One point is timed first. There is no grid fallback: if the run is slow, it runs longer, and the verdict thresholds
  do not change.
- **D is scored as registered** when it finishes, independently of this test.
- Script: `scripts/edge_wp7_principal_circle.py`, with `scripts/edge_wp7_groups.py`.
- Receipt: `research/edge-theorem-20261122-f/wp7_principal_circle.json`.
