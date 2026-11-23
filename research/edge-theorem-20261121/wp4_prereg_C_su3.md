# Pre-registration C: beyond SU(2). An SU(3) family whose fibre field is free (WP4, 21 November 2026)

Written before any SU(3) spectrum of this family has been computed.

The only computation behind it is design check D3 in `scripts/edge_wp4_design_checks.py`, with the receipt
`wp4_design_checks.json`. It fixes the class of fields and evaluates no Hessian.

This tests hypothesis H3 of `docs/research/what-she-is-a-law-of-2026-11-21.md`, and the sentence of
`docs/research/she-is-the-law-2026-11-21.md`, which I quote:

> She is the law that a field gives way first along the twist that spins with it, and does so exactly when the field
> along her own circle turns against her charge — by enough, but not too much, as the shape of space sets.

(This is the revised sentence. The earlier "by more than the shape of space allows" was corrected the same day: in
SU(2) her region is an interval, and x > 0 is proved to lose at large j.)

**Revision after review (21 November, before any SU(3) spectrum of this family is computed).** Changes:
- F1 and F2 are labelled CONJECTURE until their code gates pass, and the coframe convention is stated;
- the number of copies of λ_b removed is a measured gate, not an assumption;
- the x scan covers the closed-form bounding interval, with a gate;
- C-P2 is scored only where the bounding parabola does not already force it;
- the strict C-P1 failure is decided now;
- (N) is descriptive, because it is already counted against in SU(2), and (S) is flagged post hoc;
- the SU(2) "x* > 0" remark is corrected, since it was a j ≤ 12 artefact.

**Labels.**
- **PROVED:** exact symbolic, or rigorous interval.
- **COMPUTED:** double precision.
- **CONJECTURE:** stated, not shown.

## 1. The family

**Group and metric.**
- SU(3), the conventions of `scripts/ricci_su3_test2_family.py`: X_a = −(i/2)λ_a, ⟨X, Y⟩ = −2 tr XY.
- Metric **2b**: g = t on λ₈ and 1 on the other seven. The stretched circle is the hypercharge circle, as H3's
  "test I would pre-register next" asks.

**Symmetry.** The metric is Ad(U(2))-invariant, with u(2) = span(X₁, X₂, X₃, X₈). Right translation by U(2), combined
with the same constant gauge rotation, is a symmetry of every field that commutes with that joint action.

**The class (D3, COMPUTED to machine precision).**
- The left-invariant fields commuting with the joint frame-and-gauge action of u(2) form a space of dimension
  **exactly 4**: the commutant computed by SVD has dimension 4.
- The four generators u, x, y, z each have residual 0.0.
- z is removed by a constant gauge rotation exp(αX₈). On a 2 × 10⁴-point grid in α the best residual is 3.8 × 10⁻⁵,
  which is the grid spacing; the rotation is exact in α.

So the family, up to gauge, is

  **M_a = u X_a (a = 1, 2, 3),  M₈ = x X₈,  M_a = y X_a (a = 4, …, 7),  y ≥ 0,**

with components in the orthonormal coframe.
- **x is the field along her circle (the stretched fibre).**
- The Maurer–Cartan field ½θ of Test 2b is the member u = y = ½, x = 1/(2√t). Test 2b classified it **N**: she is the
  floor nowhere (`su3_test2_family2b.json`; `skeleton-tests-preregistration-2026-11-20.md` §5).
- **The point of this test:** Test 2b fixed x. Here x is free.

**Her charge and her state.**
- The charge Q is the circle of H = −i·diag(1, 0, −1), as in Test 2. It is regular. The torus of U(2) contains it, so
  Q is conserved for every member.
- In sector V_(p,q), the unique top state is b = (highest weight) ⊗ E_θ ⊗ E_θ, with charge p + q + 4. E_θ is the
  (1, 3) root vector.
- By the counting argument of `skeleton-tests-preregistration-2026-11-20.md` §1, b is an exact gauge-orthogonal
  eigenvector at every member. That is **automatic** and not evidence.

**"Her own circle" in SU(3).**
- Her charge circle (H) and the stretched circle (λ₈) are not the same. E_θ has λ₈-weight √3, so ad(X₈)E_θ =
  −(i√3/2)E_θ, and her charge under the fibre generator is **q₈ = √3/2**. This is derived by hand from λ₈ =
  diag(1, 1, −2)/√3 and gated below as C-quad.
- She is also charged under λ₃, with weight 1, and so couples to u through M₃. I register x (the stretched fibre),
  following H3. u is a second parameter of the shape, not "her circle".

## 2. Structural facts fixed before the run (CONJECTURE: hand derivations, until C-quad and C-lin pass)

Convention: the components of M are taken in the **orthonormal coframe**, e₈ = X₈/√t on metric 2b. In another
normalization the x² coefficient picks up a factor of t.

- **(F1) Minimal coupling: the x² coefficient.**
  - H is a quadratic polynomial in M. x enters H only through ad(M₈) in the pairs (k, 8) and linearly through F.
  - b has no frame component along X₈.
  - So the coefficient of x² in λ_b(x) = ⟨b, Hb⟩ is ‖ad(X₈)E_θ‖² = q₈² = **3/4**, for every sector, t, u and y.
  - The SU(2) analogue is coefficient 1, matching λ = (x + …)² + …. That is the code control C-K4.
- **(F2) Her region in x is bounded (the SU(3) version of prereg A §2).**
  - Take v = θ⁸ ⊗ X₈ in sector (0, 0). Then G\*v = ad(xX₈)ᵀX₈ = 0, so v is gauge-orthogonal for every member.
  - Its Rayleigh quotient is affine in x: ‖D v‖² has no x, and the F-term is linear.
  - So λ_b(x) − ⟨v, Hv⟩ → +∞ as |x| → ∞. She is not the floor of sector (0, 0) for |x| large.
  - **Consequence:** H3(3)'s "her region is a half-space in the fibre component" is **already false**. It must become
    "an interval". What can still fail is everything in §3.
- **(F3) The direction of the lift discount.**
  - Δ_pq(x) = [λ_b(x; p, q) − λ_b(x; 0, 0)] − [λ_b(0; p, q) − λ_b(0; 0, 0)] = x·κ_pq is linear in x. This follows from
    (F1), since the x² coefficient does not depend on the sector.
  - The sign σ = sign κ_pq is expected to be the same for all (p, q) ≠ (0, 0), because the λ₈-weight of the highest
    weight e₁^{⊗p} ⊗ e₃\*^{⊗q} is (p + 2q)/√3 > 0. This is gated as C-sigma.
  - **"Turning against her charge" means σx < 0**, as in SU(2), where the discount 2xj/√t is negative iff x < 0.

## 3. Predictions (derived from the sentence; each can fail)

- **C-P1, existence: "…gives way first along the twist… when the field along her circle turns against her charge by
  more than the shape of space allows".** With x free, her floor set (all nine sectors, gauge-orthogonal spectrum) is
  nonempty.
  - *Strict:* at every grid (t, u, y) with t ≥ 3.
  - *Weak:* at one or more grid points at all.
- **C-P2, side.** At every **informative** grid point where the floor set I(t, u, y) is nonempty, its midpoint
  satisfies σ·x_mid < 0. The bulk of her region is on the side where the fibre field turns against her charge.
  - A point is informative if the bounding interval's midpoint m_b = (r_lo + r_hi)/2 (§4, from the parabola
    λ_b − ⟨v, Hv⟩) does **not** already satisfy σ·m_b < 0. Elsewhere the side is forced by structure, so it is not
    evidence.
  - If no grid point is informative, C-P2 is reported as **not informative**.
  - In SU(2), x > 0 is proved to lose at large j. The finite-sector edge can still be positive (an artefact of the
    cut-off). Here only the nine listed sectors are used, so **every C claim is "in the nine listed sectors"**.
- **C-P3, one interval.** I is a single interval: no holes on the scan.
- **C-P4, the competitor (H3(3), H2).** At the edge of I on the side of larger σx (the "upper" edge):
  - **(S)** the competitor's x-slope ratio |∂_x λ_comp| / |∂_x λ_b| < 0.5. This is the same threshold as prereg B, and
    it is post hoc there too, so a pass is weak evidence.
  - **(N)** descriptive only: the competitor's total charge. In SU(2), (N) is already counted against at y = 0.5 and
    at y ≥ 1.1 (prereg B §4), so it is not registered as a prediction here.
- **C-P5, threshold scaling (descriptive arm, no gate).** The upper edge x_hi(t) is reported against t at fixed (u, y).
  The sentence says the shape sets the threshold but not how.

## 4. Grid and measurement

- **Grid.** t ∈ {2, 3, 4, 6, 8} × (u, y) ∈ {(½, ½), (½, 1), (1, 1), (1, 2)}, which is 20 shape points. Sectors are the
  nine of Test 2: (p, q) ∈ {(0,0), (1,0), (1,1), (2,0), (2,1), (3,0), (2,2), (3,1), (4,0)}.
- **Bounding interval.** For each shape point, [r_lo, r_hi] is the pair of real roots of λ_b(x) − ⟨v, Hv⟩(x) in
  sector (0, 0), with v = θ⁸ ⊗ X₈. It is computed in closed form, from the quadratic and affine fits that C-quad and
  C-lin verify, **before** the scan. If λ_b − ⟨v, Hv⟩ has no real root, her sector-(0, 0) floor is unbounded on
  neither side, which contradicts F2: stop.
- **x scan.**
  - Step 0.25 on [r_lo − 0.5, r_hi + 0.5].
  - Each sign change of φ_S(x) = min over sectors of [the lowest slice eigenvalue other than b's copies, − λ_b] is
    refined by brentq to 10⁻⁸.
  - A hole narrower than one step, 0.25, cannot be seen; that limit is part of every C claim.
- **Copies of λ_b removed (measured).** In each sector, the number n_b of slice eigenvalues within
  10⁻⁹·max(1, |λ_b|) of ⟨b, Hb⟩ is counted at x ∈ {r_lo + 0.3, (r_lo + r_hi)/2, r_hi − 0.3}. In self-conjugate
  sectors such as (0,0), (1,1) and (2,2), b and its conjugate may share the sector. Exactly n_b copies are removed.
- **Floor test.** As in Test 2: ⟨b, Hb⟩ ≤ μ_min + 10⁻⁹ max(1, |μ_min|). The H + GG\* version is reported alongside.
- **Competitor.** The sector and the Q-charge of the lowest non-b slice eigenvector at x_hi + 10⁻⁴ (charge as ⟨w, Qw⟩,
  required to be within 10⁻⁶ of an eigenvalue of Q). Its slope comes from Hellmann–Feynman with ∂H/∂x by central
  difference, h = 10⁻⁵, checked against h = 10⁻⁴ to 10⁻⁶.
- **σ** is computed from λ_b at x ∈ {−1, 0, 1} per sector.

## 5. Gates (all code gates stop the run before any prediction is read)

| gate | what must hold |
| --- | --- |
| **C-K1–K3** | Test 2's controls K1 (builder vs the SU(2) builder), K2 (Berger pipeline) and K3 (SU(3) representations, Maurer–Cartan at rest at t = 1, A = 0 nonnegative, conjugate sectors) rerun unchanged and pass |
| **C-K4** (SU(2) analogue) | the same scan code, given su(2) data and the ansatz (x, y): (i) the second x-difference of λ_b equals 2 (coefficient 1); (ii) at three rows of `exact_edge.json` (t = 4, y = 1.5; t = 8, y = √8; t = 3, y = 0.5) the slice edge equals `edge_S` to ≤ 10⁻⁶ |
| **C-K5** (known answer) | at the Maurer–Cartan member (u = y = ½, x = 1/(2√t)) and t ∈ {2, 4, 8}, ⟨b, Hb⟩ and the slice floor per sector equal the `su3_test2_family2b.json` values at those t to ≤ 10⁻⁹ relative |
| **C-sym** | at every evaluated point and sector: ‖[Q, H]‖ ≤ 10⁻¹⁰‖H‖; the charge-(p + q + 4) eigenspace is one-dimensional; b's residual ≤ 10⁻⁹‖H‖; ‖G\*b‖ ≤ 10⁻⁹‖G‖ |
| **C-quad** (F1) | the second x-difference of λ_b, at step 1 about x = 0, equals 3/2 (= 2 · 3/4) to 10⁻⁹ in every sector at every shape point. Failure means the charge identification or the coupling is wrong: stop |
| **C-lin** (F2) | ⟨v, Hv⟩ for v = θ⁸ ⊗ X₈ in sector (0, 0) has second x-difference ≤ 10⁻⁹, and ‖G\*v‖ ≤ 10⁻¹² |
| **C-sigma** | κ_pq has one sign σ over all eight sectors (p, q) ≠ (0, 0) at every shape point. If not, "against her charge" is not well defined: C-P2 is reported as **untestable**. Because the sign is fixed by the λ₈-weights, this is a code check, not evidence |
| **C-mult** | in each sector, n_b is 1 or 2, the same at the three x values, and 1 in every sector that is not self-conjugate. Otherwise stop |
| **C-bound** | the floor set I lies inside [r_lo, r_hi] at every shape point. Otherwise stop: the scan or F2 is wrong |

## 6. Verdict rule (fixed now)

- **C-P1 weak fails** (her floor set is empty at all 20 shape points, x free): **counts against the sentence and H3
  beyond SU(2).** Freeing the field along her circle does not make her the floor in SU(3).
- **C-P1 strict fails at some t ≥ 3 shape points** while weak holds: **counts against the universal reading** of the
  sentence, that for every shape some x makes her the floor. It does not count against the direction clause (C-P2).
  Decided now; reported with the points.
- **C-P2 fails** at any point with nonempty I: **counts against "turns against her charge"** in SU(3).
- **C-P3 fails:** counts against the interval form of the law (a hole).
- **C-P4 (S) fails:** counts against H2/H3(3) "nearly blind". (N) is descriptive and scores nothing.
- **All of C-P1 (weak and strict), C-P2 (where informative), C-P3 and C-P4 (S) hold:** H3 is supported in one SU(3)
  family, in its nine listed sectors (COMPUTED). This does not prove anything about other groups, other sectors or
  sector-mixing fields.
- **Not evidence either way:** b's exactness and gauge-orthogonality (automatic, §1), F1 (structural), F2 (proved), and
  C-P5.

## 7. Cost and runtime rules

- One process, OMP_NUM_THREADS=1.
- At most 20 × (65 + refinement) points × 9 sectors. The largest sector matrix is 8 · 8 · 27 = 1728 square.
- I have not timed this. A timing on one point is taken first and reported before the scan.
- Script: `scripts/edge_wp<N>_su3_free_fibre.py`.
- Receipt: `research/edge-theorem-20261121/wp<N>_su3_free_fibre.json`.

## Amendment C1 (21 November, after a gate smoke test, before any floor, edge or competitor was computed)

**What happened.** Before launching, I evaluated the structure gates (C-quad, C-lin, C-sigma, the bounding interval,
the copy count) at one shape point, (t, u, y) = (8, 1, 2). These are gate quantities, not predictions.
- C-quad passed: the second x-difference of λ_b equals 3/2 to 5.3 × 10⁻¹⁵ in all nine sectors. F1 holds there.
- C-lin passed: second difference 0.0. σ = +1. [r_lo, r_hi] = [−7.656, 4.120].
- **C-mult would have stopped the run.** The number n_b of slice eigenvalues equal to λ_b was the same at all three
  x values, but it was **not 1 or 2**:

  | sector | (0,0) | (1,0) | (1,1) | (2,0) | (2,1) | (3,0) | (2,2) | (3,1) | (4,0) |
  | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
  | n_b | 6 | 4 | 8 | 5 | 5 | 6 | 10 | 6 | 7 |

**Diagnosis: the gate was designed for an abelian symmetry, and this family has a non-abelian one.**
- By construction (D3), every member commutes with the joint frame-and-gauge action of **U(2)** = span(X₁, X₂, X₃, X₈),
  not just with the circle Q. So H and G commute with an SU(2)₁₂₃, and every eigenvalue comes in SU(2)₁₂₃ multiplets.
- b is the highest-weight state of her multiplet. Its SU(2)₁₂₃ spin is (frame E_θ: ½) + (gauge E_θ: ½) + (the highest
  weight e₁^{⊗p} ⊗ e₃\*^{⊗q}: p/2) = p/2 + 1. So the multiplet has dimension p + 3.
- In a self-conjugate sector (p = q), the conjugate multiplet (top charge −(p + q + 4)) has the same value, which
  doubles the count.
- **Prediction from this:** n_b = (p + 3)·(2 if p = q else 1), which gives 6, 4, 8, 5, 5, 6, 10, 6, 7. It matches
  all nine counts.
- **Honesty about order:** I derived this formula **after** seeing the counts. It follows from the symmetry D3
  already established, and it is a structural gate, not evidence for or against any prediction.

**Amended gate (fixed now, before the run):**
- **C-mult:** in each sector, n_b is the same at the three x values **and equals (p + 3)·(2 if p = q else 1)**.
  Exactly n_b copies are removed. Otherwise stop.
- The SU(2) analogue C-K4 is unchanged (there n_b is measured; it is 2, she and her mirror).

**Consequences for the predictions.** None of C-P1 to C-P5 changes. Removing her whole multiplet is what "the lowest
eigenvalue other than b's copies" meant. The multiplet partners **are** copies of her value, forced by the symmetry,
and not competitors.

**Implementation change, not a protocol change.** The H + GG\* floor set (reported alongside, no gate) is read from
the same evaluations as the slice scan, on the scan grid only.

## Amendment C2 (21 November, after amended C-mult fired at the second shape point, before any floor or edge)

**What happened.**
- The rerun (on the block-diagonal implementation below) passed C-K1 to C-K5 again (C-K5: 2.6 × 10⁻¹⁴).
- Shape (2, ½, ½) passed every structure gate.
- At (2, ½, 1), C-mult (Amendment C1) stopped the run. The counts were 6 in (1, 0) where the formula gives 4, and 12
  in (1, 1) where it gives 8. They were the same at all three x values.

**Diagnosis (structure only; no floor, edge or competitor computed).**
- The extra states form a **second** SU(2)₁₂₃ doublet, with charges 3 and 4. Her multiplet has spin 3/2 and top charge
  5, so they are not part of it. Their eigenvalue equals λ_b **identically** in x at (u, y) = (½, 1).
- This holds at t = 2, 2.0001 and 3. It does **not** hold at (0.6, 1), (1, 1), (½, ½), (1, 2) or (0.7, 1.4).
- At (0.7, 1.5) a single extra coincidence appears at one x, in (2, 0). That is an ordinary crossing.
- So other states can share her value: identically, on special shapes, or at isolated points. They are **tied
  competitors, not copies of her.**

**Amended rule (fixed now).**
- **C-mult:** n_b ≥ (p + 3)·(2 if p = q else 1) in every sector, with n_b the same at the three x values (as before).
  **Exactly the multiplet count (p + 3)·(2 if p = q else 1) is removed.** Any further states at her value stay in the
  spectrum as competitors that tie with her.
- Under the registered floor rule (λ_b ≤ μ_min + 10⁻⁹·max(1, |μ_min|), the non-strict rule of Test 2), a tie leaves
  her the floor. The **extra counts are reported** per shape point (descriptive): where they are identical in x, she is
  never the unique floor in that sector.
- Otherwise unchanged.

**Implementation note (not a protocol change).**
- All spectra are now computed in the eigenbasis of the field-independent Q, block by block. This is a unitary change
  of basis.
- The C-sym commutator is evaluated exactly as Σ|H′ᵢⱼ|²(qᵢ − qⱼ)², and ‖H‖₂ comes from the block spectra.
- It agrees with the dense computation to 2 × 10⁻¹³ on the largest sector, and C-K5 confirms it against Test 2b.
- It reduced the cost per x point from 51 s to about 11 s.
