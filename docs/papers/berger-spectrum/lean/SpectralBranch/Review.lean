/-
Branch IX (the spectral branch, February 2026), reread line by line: the Lean targets from
`docs/research/spectral-branch-review-2026-11-22.md`.

* S1  spectral polarity: on a semiregular bipartite graph the class-signed vector `(|B| on A, −|A| on B)` satisfies
      the Laplacian eigen-equation with eigenvalue `d_A + d_B` on both classes;
* S2  the Shidoku top eigenvector: `(−1/24, 1/12)` is normalized with ratio `−2`; the paper's stated values
      `(−1/(8√3), 1/(4√6))` have squared norm `2`;
* S3  the finite Poincaré inequality `∑ c_k² ≤ (1/λ₁) ∑ λ_k c_k²` when every `λ_k ≥ λ₁ > 0`;
* S6  `λ ≥ π²/D²` and `λ ≤ c h²` force `h² D² ≥ π²/c`: under Ric ≥ 0 a bottleneck cannot close at fixed diameter;
* S9  the class completion counts at removal level 4;
* S10 the row-change counting step of the intercalate lemma.
-/
import Mathlib

namespace SpectralBranch

open Finset

/-! ## S1. Spectral polarity -/

/-- **S1, class A.** For `v ∈ A` with `d_A` neighbours, all in `B`: `(Lf)(v) = d_A f(v) − d_A f_B`, and with
`f_A = |B|`, `f_B = −|A|` and `d_A |A| = d_B |B|` this equals `(d_A + d_B) f_A`. -/
theorem polarity_A {dA dB nA nB : ℝ} (h : dA * nA = dB * nB) :
    dA * nB - dA * (-nA) = (dA + dB) * nB := by
  linear_combination h

/-- **S1, class B.** For `v ∈ B`: `(Lf)(v) = d_B f_B − d_B f_A = (d_A + d_B) f_B`. -/
theorem polarity_B {dA dB nA nB : ℝ} (h : dA * nA = dB * nB) :
    dB * (-nA) - dB * nB = (dA + dB) * (-nA) := by
  linear_combination h

/-- **S1, Shidoku instance.** `(d_A, d_B) = (4, 8)`, `(|A|, |B|) = (192, 96)`: the condition holds and the eigenvalue
is `12`. -/
theorem polarity_shidoku : (4 : ℝ) * 192 = 8 * 96 ∧ (4 : ℝ) + 8 = 12 := by norm_num

/-- **S1, Latin square instance.** `(d_A, d_B) = (4, 12)`, `(|A|, |B|) = (432, 144)`; eigenvalue `16`. -/
theorem polarity_latin : (4 : ℝ) * 432 = 12 * 144 ∧ (4 : ℝ) + 12 = 16 := by norm_num

/-! ## S2. The Shidoku top eigenvector -/

/-- **S2.** The normalized class-signed top eigenvector of the Shidoku graph takes the values `−1/24` on the 192
low-degree vertices and `1/12` on the 96 high-degree ones: unit norm, orthogonal to the constants, ratio `−2`
(as the polarity lemma requires, `−|A|/|B| = −2` up to sign). -/
theorem shidoku_top_eigvec :
    (192 : ℝ) * (1 / 24) ^ 2 + 96 * (1 / 12) ^ 2 = 1 ∧
    (192 : ℝ) * (-1 / 24) + 96 * (1 / 12) = 0 ∧
    ((1 : ℝ) / 12) / (-1 / 24) = -2 := by
  norm_num

/-- **S2, the paper's values.** `(−1/(8√3), 1/(4√6))` have squared norm `192·(1/192) + 96·(1/96) = 2`: not a unit
vector, and their ratio is `−√2`, not `−2`. -/
theorem shidoku_paper_values_not_unit :
    (192 : ℝ) * (1 / (8 * Real.sqrt 3)) ^ 2 + 96 * (1 / (4 * Real.sqrt 6)) ^ 2 = 2 := by
  have h3 : Real.sqrt 3 ^ 2 = 3 := Real.sq_sqrt (by norm_num)
  have h6 : Real.sqrt 6 ^ 2 = 6 := Real.sq_sqrt (by norm_num)
  have h3' : Real.sqrt 3 ≠ 0 := by positivity
  have h6' : Real.sqrt 6 ≠ 0 := by positivity
  field_simp
  rw [h3, h6]
  norm_num

/-! ## S3. The finite Poincaré inequality -/

/-- **S3.** If every eigenvalue `λ_k` (k ≥ 1) is at least `λ₁ > 0`, then `∑ c_k² ≤ (1/λ₁) ∑ λ_k c_k²`. This is the
spectral form of the Poincaré inequality, exact on a finite graph. -/
theorem finite_poincare {ι : Type*} (s : Finset ι) (lam c : ι → ℝ) {lam1 : ℝ} (h1 : 0 < lam1)
    (hge : ∀ k ∈ s, lam1 ≤ lam k) :
    ∑ k ∈ s, c k ^ 2 ≤ (1 / lam1) * ∑ k ∈ s, lam k * c k ^ 2 := by
  rw [mul_sum]
  apply sum_le_sum
  intro k hk
  have hk' := hge k hk
  have hq : 1 <= lam k / lam1 := (one_le_div h1).mpr hk'
  have hc : 0 <= c k ^ 2 := sq_nonneg _
  have e : 1 / lam1 * (lam k * c k ^ 2) = lam k / lam1 * c k ^ 2 := by ring
  rw [e]
  nlinarith

/-! ## S6. Under Ric ≥ 0 the isoperimetric capacity is bounded below -/

/-- **S6.** If `λ D² ≥ π²` (Zhong–Yang) and `λ ≤ c h²` (Buser), with `c > 0`, then `h² D² ≥ π² / c`. So under
Ric ≥ 0 the Cheeger constant cannot go to zero at fixed diameter: the paper's "bottleneck with Ric ≥ 0" example is
excluded by its own Theorem SCE. -/
theorem isoperimetric_capacity_lower {lam D h c : ℝ} (hc : 0 < c) (hD : 0 ≤ D ^ 2)
    (hLY : Real.pi ^ 2 ≤ lam * D ^ 2) (hB : lam ≤ c * h ^ 2) :
    Real.pi ^ 2 / c ≤ h ^ 2 * D ^ 2 := by
  rw [div_le_iff₀ hc]
  have : lam * D ^ 2 ≤ c * h ^ 2 * D ^ 2 := mul_le_mul_of_nonneg_right hB hD
  nlinarith

/-! ## S9. Class completion counts at removal level 4 -/

/-- **S9.** At `k = 4` only the distance-4 neighbours can match (`p(4,4) = 1/C(16,4) = 1/1820`), so
`N₄ = 1 + deg/1820`: `1.0021978…` for degree 4, `1.0043956…` for degree 8, `1.0065934…` for degree 12. -/
theorem n4_counts :
    Nat.choose 16 4 = 1820 ∧
    (1 : ℚ) + 4 / 1820 = 456 / 455 ∧
    (1 : ℚ) + 8 / 1820 = 457 / 455 ∧
    (1 : ℚ) + 12 / 1820 = 458 / 455 := by
  refine ⟨by decide, by norm_num, by norm_num, by norm_num⟩

/-! ## S10. The row-change counting step of the intercalate lemma -/

/-- **S10.** If four rows change `t₀, t₁, t₂, t₃` cells, each in `{0, 2, 3, 4}` (a Latin row cannot change in exactly
one cell; it can change in three, by a 3-cycle), and the total is 4, then the changes are one row with 4 or two rows
with 2. The paper's reason ("three forces a fourth") is not needed and is false; counting is enough. -/
theorem row_change_counting (t : Fin 4 → ℕ) (ht : ∀ i, t i = 0 ∨ t i = 2 ∨ t i = 3 ∨ t i = 4)
    (hsum : t 0 + t 1 + t 2 + t 3 = 4) :
    (∀ i, t i ≠ 3) ∧ ((∃ i, t i = 4) ∨ (∃ i j, i ≠ j ∧ t i = 2 ∧ t j = 2)) := by
  have h0 := ht 0; have h1 := ht 1; have h2 := ht 2; have h3 := ht 3
  constructor
  · intro i
    match i with
    | 0 => omega
    | 1 => omega
    | 2 => omega
    | 3 => omega
  · rcases h0 with h0 | h0 | h0 | h0 <;> rcases h1 with h1 | h1 | h1 | h1 <;>
      rcases h2 with h2 | h2 | h2 | h2 <;> rcases h3 with h3 | h3 | h3 | h3 <;>
      first
      | omega
      | (left; exact ⟨0, h0⟩) | (left; exact ⟨1, h1⟩) | (left; exact ⟨2, h2⟩) | (left; exact ⟨3, h3⟩)
      | (right; exact ⟨0, 1, by decide, h0, h1⟩) | (right; exact ⟨0, 2, by decide, h0, h2⟩)
      | (right; exact ⟨0, 3, by decide, h0, h3⟩) | (right; exact ⟨1, 2, by decide, h1, h2⟩)
      | (right; exact ⟨1, 3, by decide, h1, h3⟩) | (right; exact ⟨2, 3, by decide, h2, h3⟩)

end SpectralBranch
