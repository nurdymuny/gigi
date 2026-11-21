# Pre-registration: the stretch test (Test 5), 21 November 2026

Written before any code or data for it, after the alignment test (Test 4; family 4b class N).

## The hypothesis

From the exploratory reading of Test 4: **the field has to point at her, and only at her.** The twist is the floor
when her internal frame β, together with its conjugate, spans the lowest eigenspace of the curvature term
R₀ = Ric − 2[F, ·] **on its own**.

- On the aligned round backgrounds of Test 4, β was in the lowest eigenspace, but that eigenspace was
  five-dimensional, and the twist was not the floor for j ≥ 1.
- On the Berger field the lowest eigenspace is β and its conjugate alone, and the twist is the floor (proved on
  t ∈ [2.75, 10]).

This is one positive example. This test tries to break the hypothesis.

## Setup, fixed now

- **Group and background.** SU(2) on the Berger metric (t, 1, 1), with A = sθ, θ the Maurer–Cartan form. In the
  orthonormal coframe M_a = 2s e_a/√g_a, with φ(e_a) = 2τ_a as in Test 4; at t = 1, s = ½ this is the meron. These
  backgrounds are not the Berger field (its Levi-Civita field is not a multiple of θ).
- **Grid.** s ∈ {0.25, 0.5, 0.75, 1.25, 1.5, 2.0, 2.5, 3.0} × t ∈ {0.5, 1, 1.5, 2, 2.5, 3, 4, 6, 8}: 72 points. The flat
  field s = 1 is excluded.
- **Sectors.** j = 0, …, 12.
- **Floor test, extremal-state checks, stopping rules.** Exactly as in Tests 2 and 4: the twist is the unique state
  of charge j + 4; T_all is the set of grid points where it is the floor of every sector j ≤ 12.
- **Classification of each point from R₀ alone, done before the floor test.** w₀ is the lowest eigenvalue of R₀ (a
  9 × 9 matrix) and m the number of eigenvalues below w₀ + 10⁻⁹.
  - **L1, singled out:** ⟨β, R₀β⟩ ≤ w₀ + 10⁻⁹ and m = 2.
  - **L2, shared:** ⟨β, R₀β⟩ ≤ w₀ + 10⁻⁹ and m > 2.
  - **L0, not lowest:** otherwise.

## Predictions and verdict

- **P-nec (necessity).** No point of L0 or L2 is in T_all.
  - Violated: the twist is the floor somewhere the field does not single her out. That counts against "only at
    her".
- **P-suf (sufficiency, somewhere).** T_all ∩ L1 contains at least two grid points that are neighbours (adjacent in
  s at the same t, or in t at the same s).
  - Violated: being singled out never makes her the floor on a connected patch, so it is not the missing part.
- **Supported** (Numerical) if both hold.
- **Not predicted:** that every L1 point is in T_all. The Berger field is singled out from t = 12/7 but is the floor
  only from t ≈ 2.70, so a margin evidently matters; its size is not predicted. The fraction of L1 in T_all is
  reported.

## Controls (must pass before any result is read)

- **D1.** At t = 1, s = ½ (the meron), the lowest gauge-orthogonal eigenvalues of j = 0, …, 4 are −2, 1, 6, 7, 10
  to 10⁻⁹.
- **D2.** The Weitzenböck form holds with R₀ = Ric ⊗ I − 2[F, ·]: max |R(j) − R₀ ⊗ I| ≤ 10⁻¹⁰ for j = 1, 2, 4 at
  four grid points (s, t) = (0.5, 3), (1.5, 1), (2, 4), (3, 8). It is computed with
  `scripts/ricci_su2_floor_vs_ceiling.py`'s weitzenbock_parts.
- **D3.** At t = 1 the results coincide with Test 4b at s = 1.5, 2.0, 2.5 and 3.0: same is_floor per sector.

## Reported

- The (s, t) map of L-class and floor-ness, with per-sector floor sets.
- The R₀ gap between the β-pair and the next eigenvalue.
- s_F and the Ricci share.
- The agreement table: L-class against floor.

Script `scripts/ricci_stretch_test.py`, to be written after this commit. Receipt
`research/su2-floor-family-20261119/stretch_test.json`.
