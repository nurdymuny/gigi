module

public import H7CubicQuartic
public import Mathlib.Analysis.Calculus.Deriv.Pow
public import Mathlib.Analysis.Calculus.Deriv.Mul
public import Mathlib.Analysis.Calculus.Deriv.Add

/-! Local geometry of the inter-well path. Global orbit existence and the
path-infimum argument are written separately; they are not asserted here. -/

set_option autoImplicit false
public section

namespace H8

@[expose] noncomputable def W (u x y : ℝ) : ℝ := x ^ 2 / (2 * u) + u * y ^ 2 - x * y ^ 2
@[expose] noncomputable def fx (u x y : ℝ) : ℝ := y ^ 2 - x / u
@[expose] noncomputable def fy (u x y : ℝ) : ℝ := (u - x) * y
@[expose] noncomputable def unstableRate (u : ℝ) : ℝ :=
  (Real.sqrt ((1 / u) ^ 2 + 8) - 1 / u) / 2

theorem metric_pairing (u x y vx vy : ℝ) :
    (x / u - y ^ 2) * vx + 2 * ((u - x) * y) * vy =
      -(fx u x y * vx + 2 * fy u x y * vy) := by
  unfold fx fy
  ring

theorem dissipation (u x y : ℝ) :
    (x / u - y ^ 2) * fx u x y + 2 * ((u - x) * y) * fy u x y =
      -(fx u x y ^ 2 + 2 * fy u x y ^ 2) := by
  unfold fx fy
  ring

/-- This is a genuine chain-rule statement for any curve satisfying the local ODE. -/
theorem path_derivative {u τ : ℝ} (hu : 0 < u) {x y : ℝ → ℝ}
    (hx : HasDerivAt x (fx u (x τ) (y τ)) τ)
    (hy : HasDerivAt y (fy u (x τ) (y τ)) τ) :
    HasDerivAt (fun s => W u (x s) (y s))
      (-(fx u (x τ) (y τ) ^ 2 + 2 * fy u (x τ) (y τ) ^ 2)) τ := by
  have hd := (((hx.fun_pow 2).div_const (2 * u)).fun_add
    ((hy.fun_pow 2).const_mul u)).fun_sub (hx.fun_mul (hy.fun_pow 2))
  have he : (2 * x τ ^ (2 - 1) * fx u (x τ) (y τ)) / (2 * u) +
      u * (2 * y τ ^ (2 - 1) * fy u (x τ) (y τ)) -
      (fx u (x τ) (y τ) * y τ ^ 2 + x τ * (2 * y τ ^ (2 - 1) * fy u (x τ) (y τ))) =
      -(fx u (x τ) (y τ) ^ 2 + 2 * fy u (x τ) (y τ) ^ 2) := by
    unfold fx fy
    norm_num
    field_simp [ne_of_gt hu]; ring
  simpa only [W, Nat.cast_ofNat, he] using hd

theorem rectangle_boundaries {u x y : ℝ} (hu : 0 < u)
    (hxu : x ≤ u) (hy : 0 ≤ y) (hy1 : y ≤ 1) :
    0 ≤ fx u 0 y ∧ fx u u y ≤ 0 ∧ fy u x 0 = 0 ∧ fy u x 1 ≤ 0 := by
  have hs : y ^ 2 ≤ 1 := by nlinarith
  simp only [fx, fy, zero_div, sub_zero, div_self (ne_of_gt hu), mul_zero, mul_one]
  exact ⟨sq_nonneg y, by linarith, trivial, by linarith⟩

theorem interior_descent {u x y : ℝ} (hxu : x < u) (hy : 0 < y) :
    fy u x y < 0 := by
  exact mul_neg_of_neg_of_pos (sub_neg.mpr hxu) hy

theorem equilibria {u x y : ℝ} (hu : 0 < u) (hy : 0 ≤ y)
    (hf : fx u x y = 0) (hh : fy u x y = 0) :
    (x = 0 ∧ y = 0) ∨ (x = u ∧ y = 1) := by
  by_cases hz : y = 0
  · left
    have hxzero : x = 0 := by simpa [fx, hz, ne_of_gt hu] using hf
    exact ⟨hxzero, hz⟩
  · have hxu : x = u := sub_eq_zero.mp ((mul_eq_zero.mp hh).resolve_right hz)
    right
    refine ⟨hxu, ?_⟩
    have hs : y ^ 2 - 1 = 0 := by simpa [fx, hxu, ne_of_gt hu] using hf
    nlinarith

theorem saddle_height {u : ℝ} (hu : 0 < u) :
    W u u 1 - W u 0 0 = u / 2 := by
  unfold W
  field_simp [ne_of_gt hu]; ring

theorem unstable_root {u : ℝ} (hu : 0 < u) :
    0 < unstableRate u ∧ unstableRate u ^ 2 + (1 / u) * unstableRate u - 2 = 0 := by
  have hs := Real.sq_sqrt (by positivity : 0 ≤ (1 / u) ^ 2 + 8)
  have hp := Real.sqrt_nonneg ((1 / u) ^ 2 + 8)
  have hi : 0 < 1 / u := one_div_pos.mpr hu
  unfold unstableRate
  constructor <;> nlinarith

theorem unstable_direction {u : ℝ} (hu : 0 < u) :
    -unstableRate u / u + 2 = unstableRate u ^ 2 ∧ -unstableRate u < 0 := by
  obtain ⟨hp, he⟩ := unstable_root hu
  constructor
  · rw [div_eq_mul_inv] at *
    nlinarith
  · linarith

theorem rectangle_ceiling {u x y : ℝ} (hu : 0 < u)
    (hx : x < u) (hy : 0 ≤ y) (hy1 : y < 1) :
    max x (u * y ^ 2) < u := by
  apply max_lt_iff.mpr
  refine ⟨hx, ?_⟩
  have hs : y ^ 2 < 1 := by nlinarith
  simpa using mul_lt_mul_of_pos_left hs hu

theorem decay_direction {u x y M : ℝ} (hx : x ≤ M) (hy : 0 ≤ y) :
    fy u x y ≤ -(u - M) * y := by
  unfold fy
  nlinarith

theorem weighted_cauchy (a b c d : ℝ) :
    (a * c + 2 * b * d) ^ 2 ≤ (a ^ 2 + 2 * b ^ 2) * (c ^ 2 + 2 * d ^ 2) := by
  nlinarith [sq_nonneg (a * d - b * c)]

theorem straight_excess (u : ℝ) :
    (u ^ 2 + 2) * (1 + 2 * u ^ 2) / 36 - (u / 2) ^ 2 = (u ^ 2 - 1) ^ 2 / 18 := by
  ring

theorem coupling_scaling {g : ℝ} (hg : 0 < g) (u x y : ℝ) :
    (x / g) ^ 2 / (2 * u) + u * (y / g) ^ 2 - g * (x / g) * (y / g) ^ 2 =
      W u x y / g ^ 2 := by
  unfold W
  field_simp [ne_of_gt hg]

end H8
