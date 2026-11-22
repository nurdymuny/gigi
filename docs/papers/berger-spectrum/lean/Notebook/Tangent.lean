/-
Bee's notebook sketch (22 November 2026): a tangent line δ = 2 + x/2, a curve under it, and a remainder that is
quadratic, ½·0.3² < ½·0.1. Read as Vygodsky's square-root approximation √(a² + b) ≈ a + b/(2a) with a = 2, b = 2x,
so the curve is s(x) = √(4 + 2x), tangent to δ at D = (0, 2).

Proved here:
* the tangent lies above the curve for x ≥ 0, and the gap is at most x²/16 (quadratic, the curvature term);
* the sketch's two bounds, ½x² and ½x, cross exactly at x = 1 (the dashed line through C): below it the quadratic
  bound is the smaller one, above it the linear one is;
* the sketch's numbers, ½·0.3² < ½·0.1.

The same shape is the second edition of the Davis Manifold: chord against arc is quadratic, (kL)²/24, inside the
linear bound kL (`DavisManifold.chord_arc_distortion`).
-/
import Mathlib

namespace Notebook

/-- The tangent `2 + x/2` lies above the curve `√(4 + 2x)` for `x ≥ 0`. -/
theorem tangent_above {x : ℝ} (hx : 0 ≤ x) : Real.sqrt (4 + 2 * x) ≤ 2 + x / 2 := by
  rw [Real.sqrt_le_left (by linarith)]
  nlinarith [sq_nonneg x]

/-- The gap between tangent and curve is at most `x²/16`: `(2 + x/2)² − (4 + 2x) = x²/4`, and the sum of the two is
at least `4`. -/
theorem tangent_gap {x : ℝ} (hx : 0 ≤ x) : (2 + x / 2) - Real.sqrt (4 + 2 * x) ≤ x ^ 2 / 16 := by
  set r := Real.sqrt (4 + 2 * x) with hr
  have hr0 : 0 ≤ r := Real.sqrt_nonneg _
  have hr2 : r ^ 2 = 4 + 2 * x := Real.sq_sqrt (by linarith)
  have hr_ge : 2 ≤ r := by nlinarith [hr2, hr0]
  have hsum : 4 ≤ (2 + x / 2) + r := by linarith
  have hprod : ((2 + x / 2) - r) * ((2 + x / 2) + r) = x ^ 2 / 4 := by nlinarith [hr2]
  have hd0 : 0 ≤ (2 + x / 2) - r := by
    have := tangent_above hx; rw [← hr] at this; linarith
  nlinarith

/-- The sketch's two bounds cross at `x = 1` (point C): for `0 < x < 1`, `½x² < ½x`. -/
theorem quadratic_below_linear {x : ℝ} (h0 : 0 < x) (h1 : x < 1) : x ^ 2 / 2 < x / 2 := by
  nlinarith

/-- At `x = 1` the two bounds are equal: the dashed line through C. -/
theorem bounds_meet_at_one : (1 : ℝ) ^ 2 / 2 = 1 / 2 := by norm_num

/-- Past `x = 1` the order reverses: the linear bound is the smaller one there. -/
theorem linear_below_quadratic {x : ℝ} (h1 : 1 < x) : x / 2 < x ^ 2 / 2 := by
  nlinarith

/-- The numbers in the sketch: `½ · 0.3² < ½ · 0.1`. -/
theorem sketch_numbers : (1 / 2 : ℝ) * (3 / 10) ^ 2 < (1 / 2) * (1 / 10) := by norm_num

end Notebook
