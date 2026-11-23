/-
YANG–MILLS REQUIREMENTS AS GEOMETRIC OBJECTS, Section 7: the gap as the shape of fixed-energy sets.

The note's equivalence "a positive gap ⇔ the fixed-energy sublevel set is bounded, with radius √(E*/Δ)" needs no
spectral theorem. It is a fact about any energy that scales quadratically: q(c•x) = c²·q(x), q ≥ 0, and q(x) > 0
off the vacuum. Here `E` stands for the vacuum-orthogonal form domain; the vacuum has already been removed, which is
exactly the hypothesis `pos` (without it, a zero-energy direction makes the sublevel set unbounded: see `vacuum_kept`).
-/
import Mathlib

namespace YMGeometry

variable {E : Type*} [NormedAddCommGroup E] [NormedSpace ℝ E]

/-- An energy form, as the note uses it: nonnegative, quadratic under scaling, positive off the vacuum. -/
structure EnergyForm (q : E → ℝ) : Prop where
  nonneg : ∀ x, 0 ≤ q x
  hom : ∀ (c : ℝ) (x : E), q (c • x) = c ^ 2 * q x
  pos : ∀ x, x ≠ 0 → 0 < q x

/-- Scaling a nonzero vector onto the level set `q = E*`. -/
lemma scale_to_level {q : E → ℝ} (hq : EnergyForm q) {Es : ℝ} (hE : 0 < Es) {x : E} (hx : x ≠ 0) :
    q (Real.sqrt (Es / q x) • x) = Es ∧ ‖Real.sqrt (Es / q x) • x‖ = Real.sqrt (Es / q x) * ‖x‖ := by
  have hqx := hq.pos x hx
  refine ⟨?_, ?_⟩
  · rw [hq.hom, Real.sq_sqrt (div_nonneg hE.le hqx.le)]
    field_simp
  · rw [norm_smul, Real.norm_of_nonneg (Real.sqrt_nonneg _)]

omit [NormedSpace ℝ E] in
/-- **Section 7, (2) ⇒ (3).** A gap bounds every fixed-energy sublevel set, by √(E*/Δ). -/
theorem sublevel_bounded_of_gap {q : E → ℝ} {Δ Es : ℝ} (hΔ : 0 < Δ) (hgap : ∀ x, Δ * ‖x‖ ^ 2 ≤ q x)
    {x : E} (hx : q x ≤ Es) : ‖x‖ ≤ Real.sqrt (Es / Δ) := by
  apply Real.le_sqrt_of_sq_le
  rw [le_div_iff₀ hΔ]
  nlinarith [hgap x]

/-- **Section 7, (3) ⇒ (2).** If one fixed-energy sublevel set is bounded (radius `R`), there is a gap,
`Δ = E*/R²`. -/
theorem gap_of_sublevel_bounded {q : E → ℝ} (hq : EnergyForm q) {Es R : ℝ} (hE : 0 < Es) (hR : 0 < R)
    (hbd : ∀ x, q x ≤ Es → ‖x‖ ≤ R) : ∀ x, Es / R ^ 2 * ‖x‖ ^ 2 ≤ q x := by
  intro x
  by_cases hx : x = 0
  · subst hx
    have : q (0 : E) = 0 := by simpa using hq.hom 0 (0 : E)
    simp [this]
  have hqx := hq.pos x hx
  obtain ⟨hlev, hnorm⟩ := scale_to_level hq hE hx
  have hy := hbd _ hlev.le
  rw [hnorm] at hy
  -- √(E*/q x)·‖x‖ ≤ R, so (E*/q x)·‖x‖² ≤ R², so E*·‖x‖² ≤ R²·q x
  have hs : 0 ≤ Real.sqrt (Es / q x) * ‖x‖ := by positivity
  have hsq : (Real.sqrt (Es / q x) * ‖x‖) ^ 2 ≤ R ^ 2 := pow_le_pow_left₀ hs hy 2
  rw [mul_pow, Real.sq_sqrt (div_nonneg hE.le hqx.le)] at hsq
  rw [div_mul_eq_mul_div, div_le_iff₀ (by positivity)]
  have : Es * ‖x‖ ^ 2 ≤ R ^ 2 * q x := by
    have h := mul_le_mul_of_nonneg_right hsq hqx.le
    rwa [div_mul_eq_mul_div, div_mul_cancel₀ _ hqx.ne'] at h
  linarith [mul_comm (q x) (R ^ 2)]

/-- **Section 7, the equivalence.** For any one budget `E* > 0`: a positive gap exists exactly when the
fixed-energy sublevel set is bounded. -/
theorem gap_iff_sublevel_bounded {q : E → ℝ} (hq : EnergyForm q) {Es : ℝ} (hE : 0 < Es) :
    (∃ Δ > 0, ∀ x, Δ * ‖x‖ ^ 2 ≤ q x) ↔ (∃ R, ∀ x, q x ≤ Es → ‖x‖ ≤ R) := by
  constructor
  · rintro ⟨Δ, hΔ, hgap⟩
    exact ⟨Real.sqrt (Es / Δ), fun x hx => sublevel_bounded_of_gap hΔ hgap hx⟩
  · rintro ⟨R, hbd⟩
    refine ⟨Es / (max R 1) ^ 2, by positivity, ?_⟩
    exact gap_of_sublevel_bounded hq hE (by positivity) (fun x hx => (hbd x hx).trans (le_max_left _ _))

/-- **Section 7, the radius is exactly √(E*/Δ).** If `Δ` is the sharp gap (every larger constant fails on some
vector), the sublevel set reaches beyond every radius below √(E*/Δ). With `sublevel_bounded_of_gap` this pins the
supremum of the norms on the set to √(E*/Δ). The supremum need not be attained. -/
theorem sublevel_radius_sharp {q : E → ℝ} (hq : EnergyForm q) {Δ Es : ℝ} (hΔ : 0 < Δ) (hE : 0 < Es)
    (hsharp : ∀ ε > 0, ∃ x, x ≠ 0 ∧ q x < (Δ + ε) * ‖x‖ ^ 2) {r : ℝ} (hr : r < Real.sqrt (Es / Δ)) :
    ∃ y, q y ≤ Es ∧ r < ‖y‖ := by
  by_cases hr0 : r < 0
  · exact ⟨0, by simpa using (show q (0 : E) ≤ Es by simpa [show q (0 : E) = 0 by simpa using hq.hom 0 (0 : E)]
      using hE.le), by simpa using hr0⟩
  replace hr0 := not_lt.mp hr0
  -- choose ε with r² < E*/(Δ+ε)
  have hr2 : r ^ 2 < Es / Δ := by
    have := (Real.lt_sqrt hr0).mp hr; exact this
  obtain ⟨ε, hε, hεr⟩ : ∃ ε > 0, r ^ 2 * (Δ + ε) < Es := by
    have h1 : r ^ 2 * Δ < Es := by rwa [lt_div_iff₀ hΔ] at hr2
    by_cases hr00 : r = 0
    · exact ⟨1, one_pos, by simp [hr00, hE]⟩
    have hr2pos : 0 < r ^ 2 := by positivity
    refine ⟨(Es - r ^ 2 * Δ) / (2 * r ^ 2), by apply div_pos <;> linarith, ?_⟩
    have : r ^ 2 * ((Es - r ^ 2 * Δ) / (2 * r ^ 2)) = (Es - r ^ 2 * Δ) / 2 := by field_simp
    nlinarith [this]
  obtain ⟨x, hx, hqx⟩ := hsharp ε hε
  obtain ⟨hlev, hnorm⟩ := scale_to_level hq hE hx
  refine ⟨_, hlev.le, ?_⟩
  rw [hnorm]
  have hqpos := hq.pos x hx
  have hxn : 0 < ‖x‖ := norm_pos_iff.mpr hx
  -- r < √(E*/q x)·‖x‖  ⇐  r²·q x < E*·‖x‖²  ⇐  r²(Δ+ε)‖x‖² < E*‖x‖²  and  q x < (Δ+ε)‖x‖²
  have key : r ^ 2 * q x < Es * ‖x‖ ^ 2 := by
    have h1 : r ^ 2 * q x ≤ r ^ 2 * ((Δ + ε) * ‖x‖ ^ 2) := mul_le_mul_of_nonneg_left hqx.le (sq_nonneg r)
    have h2 : r ^ 2 * ((Δ + ε) * ‖x‖ ^ 2) < Es * ‖x‖ ^ 2 := by
      rw [← mul_assoc]; exact mul_lt_mul_of_pos_right hεr (by positivity)
    linarith
  have hs : 0 ≤ Real.sqrt (Es / q x) * ‖x‖ := by positivity
  apply lt_of_pow_lt_pow_left₀ 2 hs
  rw [mul_pow, Real.sq_sqrt (div_nonneg hE.le hqpos.le), div_mul_eq_mul_div, lt_div_iff₀ hqpos]
  linarith

/-- **The gate the note asks for: keep the vacuum and the set is unbounded.** If some nonzero direction costs no
energy, the sublevel set contains its every multiple. -/
theorem vacuum_kept {q : E → ℝ} (hhom : ∀ (c : ℝ) (x : E), q (c • x) = c ^ 2 * q x) {Ω : E} (hΩ : Ω ≠ 0)
    (hq0 : q Ω = 0) {Es : ℝ} (hE : 0 ≤ Es) (R : ℝ) : ∃ y, q y ≤ Es ∧ R < ‖y‖ := by
  have hn : 0 < ‖Ω‖ := norm_pos_iff.mpr hΩ
  refine ⟨((|R| + 1) / ‖Ω‖) • Ω, by rw [hhom, hq0]; simpa using hE, ?_⟩
  rw [norm_smul, Real.norm_of_nonneg (by positivity), div_mul_cancel₀ _ hn.ne']
  linarith [le_abs_self R]

end YMGeometry
