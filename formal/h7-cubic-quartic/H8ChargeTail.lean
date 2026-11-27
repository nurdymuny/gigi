module

public import H7CubicQuartic
public import Mathlib.Analysis.SpecialFunctions.Pow.Real

/-! The algebra behind the H8 charge tail: the gauge choice that proves the horizontal barrier, the AM-GM
ruler, the constrained zero-point floor in both regimes, and its closed form at t = 3. The projection
definition, the integration by parts and the operator-domain arguments are in the written proof, not here.
-/

set_option autoImplicit false
public section
namespace H8ChargeTail

/-- With `Y = c • X₁`, the circle vector minus the gauge vector has rows `0`, `-(X₃ - c[X₁,X₂])`,
`X₂ - c[X₃,X₁]`; written with the gauge rows as `[X_k, Y]`. -/
theorem gauge_rows {V : Type*} [AddCommGroup V] [Module ℝ V] (br : V → V → V)
    (hsmul : ∀ (c : ℝ) x y, br x (c • y) = c • br x y)
    (hanti : ∀ x y, br x y = -br y x) (c : ℝ) (X1 X2 X3 : V) :
    br X1 (c • X1) = 0 ∧
    -X3 - br X2 (c • X1) = -(X3 - c • br X1 X2) ∧
    X2 - br X3 (c • X1) = X2 - c • br X3 X1 := by
  have hself : br X1 X1 = 0 := by
    have h := hanti X1 X1
    have h2 : (2 : ℝ) • br X1 X1 = 0 := by rw [two_smul]; nth_rewrite 1 [h]; exact neg_add_cancel _
    exact (smul_eq_zero.mp h2).resolve_left (by norm_num)
  refine ⟨?_, ?_, ?_⟩
  · rw [hsmul, hself, smul_zero]
  · rw [hsmul, hanti X2 X1, smul_neg]
    abel
  · rw [hsmul]

/-- The ruler: `V ≥ s²/(2t)` and kinetic cost `m²/(2s²)` together cost at least `|m|/√t`. -/
theorem ruler (t s m : ℝ) (ht : 0 < t) (hs : 0 < s) :
    |m| / Real.sqrt t ≤ s ^ 2 / (2 * t) + m ^ 2 / (2 * s ^ 2) := by
  have hr : 0 < Real.sqrt t := Real.sqrt_pos.mpr ht
  have hrt : Real.sqrt t ^ 2 = t := Real.sq_sqrt ht.le
  generalize Real.sqrt t = r at hr hrt ⊢
  subst hrt
  rw [← sq_abs m]
  have key : 0 ≤ (s ^ 2 / r - |m|) ^ 2 / (2 * s ^ 2) := by positivity
  have expand : (s ^ 2 / r - |m|) ^ 2 / (2 * s ^ 2)
      = s ^ 2 / (2 * r ^ 2) + |m| ^ 2 / (2 * s ^ 2) - |m| / r := by
    field_simp
    ring
  linarith

/-- The two charge inequalities `c ≤ P(E-P)` (plain) and `c ≤ P(E-σ)` (twisted), with `σ ≤ E`,
force `E ≥ σ + c/σ` whenever `c ≤ σ²`. -/
theorem zero_point_floor (σ c E P : ℝ) (hσ : 0 < σ)
    (hE : σ ≤ E) (hA : c ≤ P * (E - P)) (hB : c ≤ P * (E - σ)) (hsmall : c ≤ σ ^ 2) :
    σ + c / σ ≤ E := by
  rcases le_total P σ with h | h
  · -- twisted bound: c ≤ P(E-σ) ≤ σ(E-σ)
    have h1 : c ≤ σ * (E - σ) := le_trans hB (mul_le_mul_of_nonneg_right h (by linarith))
    have h2 : c / σ ≤ E - σ := by rw [div_le_iff₀ hσ]; linarith
    linarith
  · -- plain bound: E ≥ P + c/P ≥ σ + c/σ since P ≥ σ ≥ √c
    have hPpos : 0 < P := lt_of_lt_of_le hσ h
    have h1 : c / P ≤ E - P := by rw [div_le_iff₀ hPpos]; linarith
    have h2 : σ + c / σ ≤ P + c / P := by
      have hd : P + c / P - (σ + c / σ) = (P - σ) * (P * σ - c) / (P * σ) := by
        field_simp
        ring
      have hnum : 0 ≤ (P - σ) * (P * σ - c) / (P * σ) := by
        apply div_nonneg
        · apply mul_nonneg (by linarith)
          nlinarith
        · positivity
      linarith
    linarith

/-- Beyond the small-charge range only the plain bound is used: `c ≤ P(E-P)` gives `2√c ≤ E`. -/
theorem large_charge_floor (c E P : ℝ) (hE : 0 ≤ E) (hA : c ≤ P * (E - P)) :
    2 * Real.sqrt c ≤ E := by
  have h4 : 4 * c ≤ E ^ 2 := by nlinarith [sq_nonneg (E - 2 * P)]
  have hs : Real.sqrt (4 * c) ≤ E := Real.sqrt_le_iff.mpr ⟨hE, h4⟩
  have h2 : Real.sqrt (4 * c) = 2 * Real.sqrt c := by
    rw [Real.sqrt_mul (by norm_num : (0 : ℝ) ≤ 4)]
    norm_num [show Real.sqrt 4 = 2 by rw [show (4 : ℝ) = 2 ^ 2 by norm_num, Real.sqrt_sq (by norm_num)]]
  linarith

/-- At `t = 3` with `n = 3`: `σ = 3S/2 = 5√3/2` and `c = m²/12`, so `σ + c/σ = (225 + m²)/(30√3)`. -/
theorem t3_floor_closed_form (m : ℝ) :
    5 * Real.sqrt 3 / 2 + (m ^ 2 / 12) / (5 * Real.sqrt 3 / 2) = (225 + m ^ 2) / (30 * Real.sqrt 3) := by
  have h3 : Real.sqrt 3 ^ 2 = 3 := Real.sq_sqrt (by norm_num)
  have hp : Real.sqrt 3 ≠ 0 := by positivity
  field_simp
  nlinarith [h3]

end H8ChargeTail
