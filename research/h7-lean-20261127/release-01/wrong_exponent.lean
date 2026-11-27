module

public import Mathlib.Analysis.SpecialFunctions.Pow.Real
public import Mathlib.Tactic.FieldSimp
public import Mathlib.Tactic.Linarith
public import Mathlib.Tactic.NormNum
public import Mathlib.Tactic.Positivity
public import Mathlib.Tactic.Ring

/-! Real-algebraic core of the H7 cubic-quartic deduction.
The global bound and homogeneity are explicit hypotheses, not axioms.
The Lie-algebra instantiation and compact-resolvent step are outside this file.
-/

set_option autoImplicit false

public section

namespace H7

/-- A uniform bound under arbitrary positive cubic rescaling forces zero. -/
theorem zero_of_scaled_bound {x B : ℝ} (hx : 0 ≤ x)
    (h : ∀ r : ℝ, 0 < r → r ^ 3 * x ≤ B) : x = 0 := by
  by_cases hz : x = 0
  · exact hz
  have hp : 0 < x := lt_of_le_of_ne hx (Ne.symm hz)
  have hB : 0 ≤ B := le_trans hx (by simpa using h 1 zero_lt_one)
  let r : ℝ := B / x + 1
  have hr : 1 ≤ r := by
    dsimp [r]
    linarith [div_nonneg hB hx]
  have hrp : 0 < r := lt_of_lt_of_le zero_lt_one hr
  have hrx : r * x = B + x := by
    dsimp [r]
    field_simp [ne_of_gt hp]
  have hc : r ≤ r ^ 3 := le_self_pow₀ hr (by decide)
  have hm := mul_le_mul_of_nonneg_right hc hx
  have hb := h r hrp
  exfalso
  linarith

theorem fourth_root_fourth (q : ℝ) (hq : 0 ≤ q) :
    (q ^ (1 / 4 : ℝ)) ^ (4 : ℕ) = q := by
  rw [← Real.rpow_natCast, ← Real.rpow_mul hq]
  norm_num

theorem fourth_root_cube (q : ℝ) (hq : 0 ≤ q) :
    (q ^ (1 / 4 : ℝ)) ^ (3 : ℕ) = q ^ (3 / 4 : ℝ) := by
  rw [← Real.rpow_natCast, ← Real.rpow_mul hq]
  norm_num

/-- Conditional on a global inhomogeneous bound, not on its desired conclusion. -/
theorem cubic_quartic_bound {E : Type*} [SMul ℝ E]
    (C Q : E → ℝ) (A B : ℝ)
    (hQ : ∀ q, 0 ≤ Q q)
    (hCscale : ∀ r, 0 < r → ∀ q, C (r • q) = r ^ 3 * C q)
    (hQscale : ∀ r, 0 < r → ∀ q, Q (r • q) = r ^ 4 * Q q)
    (hglobal : ∀ q, |C q| ≤ A * Q q + B) :
    ∀ q, |C q| ≤ (A + B) * Q q ^ (2 / 3 : ℝ) := by
  intro q
  have hscaled : ∀ r : ℝ, 0 < r → r ^ 3 * |C q| ≤ A * (r ^ 4 * Q q) + B := by
    intro r hr
    have hb := hglobal (r • q)
    rw [hCscale r hr q, hQscale r hr q, abs_mul,
      abs_of_pos (pow_pos hr 3)] at hb
    exact hb
  by_cases hz : Q q = 0
  · have hc : |C q| = 0 := zero_of_scaled_bound (abs_nonneg _) (by
      intro r hr
      simpa [hz] using hscaled r hr)
    rw [hc, hz]
    norm_num
  · have hp : 0 < Q q := lt_of_le_of_ne (hQ q) (Ne.symm hz)
    let u : ℝ := Q q ^ (1 / 4 : ℝ)
    have hu : 0 < u := Real.rpow_pos_of_pos hp _
    have hu4 : u ^ 4 = Q q := fourth_root_fourth _ (hQ q)
    have hu3 : u ^ 3 = Q q ^ (3 / 4 : ℝ) := fourth_root_cube _ (hQ q)
    have hcancel : (u⁻¹) ^ 4 * Q q = 1 := by
      rw [← hu4, ← mul_pow]
      simp [ne_of_gt hu]
    have hb := hscaled u⁻¹ (inv_pos.mpr hu)
    rw [hcancel, mul_one] at hb
    have hm := mul_le_mul_of_nonneg_left hb (le_of_lt (pow_pos hu 3))
    have hcancel3 : u ^ 3 * ((u⁻¹) ^ 3 * |C q|) = |C q| := by
      rw [← mul_assoc, ← mul_pow]
      simp [ne_of_gt hu]
    rw [hcancel3, hu3] at hm
    simpa [mul_comm] using hm

/-- Positive cited constants give a positive homogeneous-bound constant. -/
theorem positive_constant_bound {E : Type*} [SMul ℝ E]
    (C Q : E → ℝ) (A B : ℝ) (hA : 0 < A) (hB : 0 < B)
    (hQ : ∀ q, 0 ≤ Q q)
    (hCscale : ∀ r, 0 < r → ∀ q, C (r • q) = r ^ 3 * C q)
    (hQscale : ∀ r, 0 < r → ∀ q, Q (r • q) = r ^ 4 * Q q)
    (hglobal : ∀ q, |C q| ≤ A * Q q + B) :
    ∃ c : ℝ, 0 < c ∧ ∀ q, |C q| ≤ c * Q q ^ (3 / 4 : ℝ) := by
  exact ⟨A + B, add_pos hA hB, cubic_quartic_bound C Q A B hQ hCscale hQscale hglobal⟩

theorem coercivity_factorization (z : ℝ) :
    z ^ 4 / 2 - z ^ 3 + 27 / 32 =
      (2 * z - 3) ^ 2 * (4 * z ^ 2 + 4 * z + 3) / 32 := by
  ring

/-- The scalar coercivity estimate, with fractional powers interpreted over the reals. -/
theorem scaled_coercivity (q g a : ℝ) (hq : 0 ≤ q) (hg : 0 < g) (ha : 0 ≤ a) :
    -(27 * a ^ 4 / (32 * g ^ 2)) ≤ g ^ 2 / 2 * q - g * a * q ^ (3 / 4 : ℝ) := by
  let u : ℝ := q ^ (1 / 4 : ℝ)
  have hu : 0 ≤ u := Real.rpow_nonneg hq _
  have hu4 : u ^ 4 = q := fourth_root_fourth q hq
  have hu3 : u ^ 3 = q ^ (3 / 4 : ℝ) := fourth_root_cube q hq
  have hf : g ^ 2 / 2 * u ^ 4 - g * a * u ^ 3 + 27 * a ^ 4 / (32 * g ^ 2) =
      (2 * g * u - 3 * a) ^ 2 *
        (4 * g ^ 2 * u ^ 2 + 4 * g * a * u + 3 * a ^ 2) / (32 * g ^ 2) := by
    field_simp [ne_of_gt hg]; ring
  have hn : 0 ≤ (2 * g * u - 3 * a) ^ 2 *
      (4 * g ^ 2 * u ^ 2 + 4 * g * a * u + 3 * a ^ 2) / (32 * g ^ 2) := by
    positivity
  rw [hu4, hu3] at hf
  linarith

/-- The spectral consequence is not asserted here; this is its potential inequality. -/
theorem potential_lower_bound (R T κ g S c C Q : ℝ)
    (hQ : 0 ≤ Q) (hg : 0 < g) (hS : 0 ≤ S) (hc : 0 ≤ c)
    (hquadratic : κ / 2 * R ≤ T) (hcubic : |C| ≤ c * Q ^ (3 / 4 : ℝ)) :
    κ / 2 * R - 27 * S ^ 4 * c ^ 4 / (32 * g ^ 2) ≤
      T - g * S * C + g ^ 2 / 2 * Q := by
  have hs := scaled_coercivity Q g (S * c) hQ hg (mul_nonneg hS hc)
  have hl := mul_le_mul_of_nonneg_left (le_trans (le_abs_self C) hcubic)
    (mul_nonneg (le_of_lt hg) hS)
  calc
    κ / 2 * R - 27 * S ^ 4 * c ^ 4 / (32 * g ^ 2) =
        κ / 2 * R + -(27 * (S * c) ^ 4 / (32 * g ^ 2)) := by ring
    _ ≤ T + (g ^ 2 / 2 * Q - g * (S * c) * Q ^ (3 / 4 : ℝ)) :=
      add_le_add hquadratic hs
    _ ≤ T - g * S * C + g ^ 2 / 2 * Q := by nlinarith only [hl]

noncomputable def kappa (t s : ℝ) : ℝ := s * ((t + 2) - s * (t + 1)) / t

theorem kappa_pos {t s : ℝ} (ht : 1 ≤ t) (hs : 0 < s)
    (hwindow : s < (t + 2) / (t + 1)) : 0 < kappa t s := by
  have ht0 : 0 < t := by linarith
  have ht1 : 0 < t + 1 := by linarith
  have hw := (lt_div_iff₀ ht1).mp hwindow
  unfold kappa
  exact div_pos (mul_pos hs (by linarith)) ht0

/-- Both original source coefficients lie above the same positive kappa. -/
theorem quadratic_coefficients {t s : ℝ} (ht : 1 ≤ t) (hs : 0 < s)
    (hwindow : s < (t + 2) / (t + 1)) :
    1 / t - (1 - s) * ((1 - s) / t - s) = kappa t s ∧
      kappa t s ≤ t - (1 - s) * (t - 2 * s) := by
  have ht0 : 0 < t := by linarith
  have hidentity : 1 / t - (1 - s) * ((1 - s) / t - s) = kappa t s := by
    unfold kappa
    field_simp [ne_of_gt ht0]; ring
  refine ⟨hidentity, ?_⟩
  have hdifference : t - (1 - s) * (t - 2 * s) - kappa t s =
      s * (t - 1) * (t + 2 - s) / t := by
    unfold kappa
    field_simp [ne_of_gt ht0]; ring
  have hw := (lt_div_iff₀ (by linarith : 0 < t + 1)).mp hwindow
  have hst : 0 ≤ t + 2 - s := by
    nlinarith [mul_nonneg (le_of_lt hs) (le_of_lt ht0)]
  have hn : 0 ≤ s * (t - 1) * (t + 2 - s) / t := by
    exact div_nonneg (mul_nonneg (mul_nonneg (le_of_lt hs) (by linarith)) hst) (le_of_lt ht0)
  linarith

end H7
