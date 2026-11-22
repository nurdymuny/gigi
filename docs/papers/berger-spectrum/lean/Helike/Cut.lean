/-
HELIKE, Chapter 4, Section 4.3: the cut (stress test S1).

Switch the cross field off, `y → 0`. The fall is the gradient flow of `E` in the metric `x² + 2y²`:
`dx/dτ = −∂E/∂x`, `dy/dτ = −½ ∂E/∂y`.

Proved here:
* `∂E/∂y = 2y·(B + (x − 2/√t)²)`, which vanishes on `y = 0`: the cut line is never left;
* on the cut line `E = 2t·x²` and `∂E/∂x = 4t·x`, so the cut fall is `x(τ) = x₀·e^{−4tτ}`, which solves the flow
  and tends to `0`: the cut field falls to `A = 0`;
* along the cut, between the Berger value of `x` and anything above it, her price in every family `j ≥ 0` only
  rises (stress test S1c).

That this explicit solution is the only one (uniqueness for the flow) is standard ODE theory and not proved here.
-/
import Helike.Fall

namespace Helike

open Filter Topology

/-- **The slope of the energy across the cut line.** `∂E/∂y = 2y·(B + (x − 2/√t)²)`. -/
theorem hasDerivAt_energy_y (x s y : ℝ) :
    HasDerivAt (fun y => energy x y s) (2 * y * (fieldCurv x y s + (x - 2 / s) ^ 2)) y := by
  unfold energy fieldCurv crossCurv
  have hB : HasDerivAt (fun y : ℝ => y ^ 2 - 2 * s * x) (2 * y) y := by
    simpa using (hasDerivAt_pow 2 y).sub_const (2 * s * x)
  have hc : HasDerivAt (fun y : ℝ => y * (x - 2 / s)) (x - 2 / s) y := by
    simpa using (hasDerivAt_id y).mul_const (x - 2 / s)
  have h := ((hB.pow 2).add ((hc.pow 2).const_mul 2)).div_const 2
  convert h using 1
  ring

/-- **The cut line is never left.** On `y = 0` the slope across it is zero. -/
theorem cut_line_invariant (x s : ℝ) :
    HasDerivAt (fun y => energy x y s) 0 0 := by
  have h := hasDerivAt_energy_y x s 0
  simpa using h

/-- On the cut line the energy is `2t·x²`. -/
theorem energy_cut (x s : ℝ) : energy x 0 s = 2 * s ^ 2 * x ^ 2 := by
  unfold energy fieldCurv crossCurv
  ring

/-- On the cut line the slope along it is `4t·x`. -/
theorem hasDerivAt_energy_cut (s x : ℝ) : HasDerivAt (fun x => energy x 0 s) (4 * s ^ 2 * x) x := by
  have h : (fun x => energy x 0 s) = fun x => 2 * s ^ 2 * x ^ 2 := by
    funext x
    exact energy_cut x s
  rw [h]
  have := ((hasDerivAt_pow 2 x).const_mul (2 * s ^ 2))
  convert this using 1
  ring

/-- **The cut fall solves the flow.** `x(τ) = x₀·e^{−4tτ}` satisfies `dx/dτ = −∂E/∂x = −4t·x`. -/
theorem cut_fall_solves (x₀ s τ : ℝ) :
    HasDerivAt (fun τ => x₀ * Real.exp (-4 * s ^ 2 * τ))
      (-(4 * s ^ 2 * (x₀ * Real.exp (-4 * s ^ 2 * τ)))) τ := by
  have h1 : HasDerivAt (fun τ : ℝ => -4 * s ^ 2 * τ) (-4 * s ^ 2 * 1) τ :=
    (hasDerivAt_id τ).const_mul (-4 * s ^ 2)
  have h2 := (h1.exp).const_mul x₀
  convert h2 using 1
  ring

/-- **The cut field falls to `A = 0`.** For `t > 0`, `x₀·e^{−4tτ} → 0` as `τ → ∞`. -/
theorem cut_fall_tendsto_zero (x₀ : ℝ) {s : ℝ} (hs : s ≠ 0) :
    Tendsto (fun τ => x₀ * Real.exp (-4 * s ^ 2 * τ)) atTop (𝓝 0) := by
  have hneg : -4 * s ^ 2 < 0 := by
    have : 0 < s ^ 2 := by positivity
    linarith
  have hlin : Tendsto (fun τ : ℝ => -4 * s ^ 2 * τ) atTop atBot :=
    tendsto_id.const_mul_atTop_of_neg hneg
  have hexp : Tendsto (fun τ : ℝ => Real.exp (-4 * s ^ 2 * τ)) atTop (𝓝 0) :=
    Real.tendsto_exp_atBot.comp hlin
  simpa using hexp.const_mul x₀

/-- **Her price only rises along the cut (S1c).** On `y = 0`, for `x` between the Berger value `(2 − t)/√t` and
anything above it, her price in every family `j ≥ 0` is nondecreasing in `x`. -/
theorem price_cut_mono {s : ℝ} (hs : 0 < s) {j x₁ x₂ : ℝ} (hj : 0 ≤ j)
    (h₁ : (2 - s ^ 2) / s ≤ x₁) (h₁₂ : x₁ ≤ x₂) : price x₁ 0 s j ≤ price x₂ 0 s j := by
  have key : price x₂ 0 s j - price x₁ 0 s j = (x₂ - x₁) * (x₁ + x₂ + 2 * (j + 2) / s + 2 * s) := by
    unfold price
    field_simp
    ring
  have hsum : 0 ≤ x₁ + x₂ + 2 * (j + 2) / s + 2 * s := by
    have hx : 2 * ((2 - s ^ 2) / s) ≤ x₁ + x₂ := by linarith
    have hb : 2 * ((2 - s ^ 2) / s) + 2 * (j + 2) / s + 2 * s = (2 * j + 8) / s := by
      field_simp
      ring
    have hpos : 0 ≤ (2 * j + 8) / s := by positivity
    linarith
  have : 0 ≤ price x₂ 0 s j - price x₁ 0 s j := by
    rw [key]
    exact mul_nonneg (by linarith) hsum
  linarith

end Helike
