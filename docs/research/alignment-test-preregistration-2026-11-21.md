# Pre-registration: the alignment test (Test 4), 21 November 2026

Written before any code or data for it.

## The claim under test

"A field yields along its twist when its own field strength lines up with the twist's spin." In my own words: the
direction is what governs her.

## Why this test can fail

The earlier tests could not test the direction of this claim that carries the risk:
- Test 2: the squashed meron, and SU(3) families 2a and 2b.
- Prediction G, a necessary condition, untested because those families had no floor points.

Every background where the field strength aligns with the twist's spin so far is the Berger field itself. This test
uses backgrounds that are aligned but are not the Berger field. If the twist is never the floor there, alignment is
not sufficient, and the sentence, read as a law, fails.

## Families, fixed now

The background is A = sθ, with θ the Maurer–Cartan form, on the unstretched group (bi-invariant metric). The grid is
s = 1.05, 1.10, …, 3.00 (40 points).

- **4a, SU(3).** The basis, gauge algebra, nine sectors (p, q), charge circle diag(1, 0, −1) and extremal state are
  those of Test 2 family 2, with the metric g = 1 on all eight directions. M_a = s X_a.
- **4b, SU(2).** The frame brackets are 2ε and the gauge algebra uses ε. Since φ(e_a) = 2τ_a identifies the frame
  with the gauge algebra, θ has components 2e_a and M_a = 2s e_a. (At s = ½ this is the meron.) The sectors are
  j = 0, …, 12. The charge circle is the one about e₀, with frame part ad(e₀), gauge part 2·ad(τ₀) and V part ρ₀.
  The extremal state is the unique state of charge j + 4, the twist.

For A = sθ the field-strength share of the twist's frame is s_F = 2s(1 − s) on SU(3) (computed for SU(3) before this
note, at every t of families 2a and 2b; exploratory). It is negative for every s > 1, so both families are **aligned
throughout the grid**. That is checked for SU(2) as precondition A2.

## Measured, exactly as in Test 2

- The gauge-orthogonal spectrum, H compressed to ker G*.
- The extremal-state checks: the twist is an exact eigenvector to 10⁻⁹‖H‖, ‖G*b‖ ≤ 10⁻⁹‖G‖, [Q, H] = 0 to
  10⁻¹⁰, and the top-charge eigenspace is one-dimensional. A failure stops the run.
- The floor test ⟨b, Hb⟩ ≤ μ_min + 10⁻⁹ max(1, |μ_min|).
- T_all is the set of grid points where the twist is the floor of every listed sector.
- Classes: E (all of the grid), P (a run of at least two consecutive points, not all), N (otherwise).

## The prediction and the verdict

- **Prediction (from the claim):** in each family, T_all contains a run of at least two consecutive grid points
  (class E or P). The Berger field needed strength before the twist became the floor (t ≥ 2.70), so a threshold is
  allowed, but it must be crossed inside s ≤ 3.
- **Supported** (Numerical) if both families are E or P.
- **Counted against** "alignment suffices" if either family is N.
- **What a supported result would not show:** that alignment is necessary (Test 2 already found no counterexample
  there), or anything about magnitude thresholds beyond what the data show.

## Controls (must pass before any family result is read)

- **C1.** At s = 1 the field is flat: energy ≤ 10⁻²⁰ on both groups. At s = ½ it is a Yang–Mills field: gradient
  ≤ 10⁻⁸ on both groups.
- **C2.** On SU(2) at s = ½ (the meron), the lowest gauge-orthogonal eigenvalue of sectors j = 0, …, 4 is −2, 1, 6,
  7, 10 to 10⁻⁹, the published spectrum (Kumar–Lechtenfeld–Picanço Costa). This fixes the SU(2) identification
  φ(e_a) = 2τ_a.
- **C3.** K1–K3 of Test 2 family 2 already validate the general builder, the SU(3) representations and the charge.
  They are rerun unchanged.
- **Precondition A2.** s_F < 0 at every grid point of both families. Otherwise the point is not aligned and is
  reported separately.

## Reported (no gate)

- The per-sector floor sets.
- s_F(s) and the Ricci share.
- The gradient norm (these backgrounds are not at rest for s ≠ 1, ½, 0; like the Berger field).
- Where the twist sits in each sector's spectrum.

Script `scripts/ricci_alignment_test.py`, to be written after this commit. Receipts in
`research/su2-floor-family-20261119/alignment_test_*.json`.
