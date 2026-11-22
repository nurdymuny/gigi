/-
HELIKE, Chapter 4, Section 4.3: overload (stress test S3).

Scale the whole field by `k`: `(x, y) → (k·x, k·y)`. Her price in family `j` is
`(k·x + (j+2)/√t)² + 2√t·k·x − k²y²`. The square is the walkway's `(stride + belt)²`.

Proved here:
* in every family her price is at least `2√t·k·x − k²y²`, and the excess is exactly the square of her net mismatch;
* the bound is reached in the family whose stride cancels the belt, `j + 2 = −k·x·√t`;
* if a family is within half a step of that, her price is within `1/(4t)` of the bound. So her lowest price
  divided by `k²` lies between `−y² + 2√t·x/k` and `−y² + 2√t·x/k + 1/(4t·k²)`, and tends to `−y²`, not
  `x² − y²` (the registered prediction S3a, which failed).
-/
import Helike.Price

namespace Helike

/-- Her price under overload: the field scaled by `k`. -/
noncomputable def overloadPrice (x y s k j : ℝ) : ℝ :=
  price (k * x) (k * y) s j

/-- The excess over the bound is exactly the square of her net mismatch. -/
theorem overload_excess (x y s k j : ℝ) :
    overloadPrice x y s k j - (2 * s * k * x - k ^ 2 * y ^ 2) = (k * x + (j + 2) / s) ^ 2 := by
  unfold overloadPrice price
  ring

/-- **The floor under overload.** In every family her price is at least `2√t·k·x − k²y²`. -/
theorem overload_lower (x y s k j : ℝ) : 2 * s * k * x - k ^ 2 * y ^ 2 ≤ overloadPrice x y s k j := by
  have := overload_excess x y s k j
  nlinarith [sq_nonneg (k * x + (j + 2) / s)]

/-- **She climbs to the family whose stride cancels the belt.** If `j + 2 = −k·x·√t`, the bound is reached. -/
theorem overload_attained {x y s k j : ℝ} (hs : s ≠ 0) (hj : j + 2 = -(k * x * s)) :
    overloadPrice x y s k j = 2 * s * k * x - k ^ 2 * y ^ 2 := by
  have h := overload_excess x y s k j
  have hz : k * x + (j + 2) / s = 0 := by
    rw [hj]
    field_simp
    ring
  rw [hz] at h
  linarith

/-- **The nearest whole family is within `1/(4t)` of the bound.** -/
theorem overload_near {x y s k j : ℝ} (hs : s ≠ 0) (hj : |j + 2 + k * x * s| ≤ 1 / 2) :
    overloadPrice x y s k j ≤ 2 * s * k * x - k ^ 2 * y ^ 2 + 1 / (4 * s ^ 2) := by
  have h := overload_excess x y s k j
  have hm : k * x + (j + 2) / s = (j + 2 + k * x * s) / s := by
    field_simp
    ring
  have hsq : (k * x + (j + 2) / s) ^ 2 ≤ 1 / (4 * s ^ 2) := by
    rw [hm, div_pow]
    have hs2 : 0 < s ^ 2 := by positivity
    have hnum : (j + 2 + k * x * s) ^ 2 ≤ 1 / 4 := by
      have := sq_abs (j + 2 + k * x * s)
      have habs : 0 ≤ |j + 2 + k * x * s| := abs_nonneg _
      nlinarith
    rw [div_le_div_iff₀ hs2 (by positivity)]
    nlinarith
  linarith

end Helike
