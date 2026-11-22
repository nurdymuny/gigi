/-
HELIKE, Chapter 4, Section 4.1: what she costs, and why.

Every statement here is exact algebra over the reals. The stretch t enters through s = √t, so we write t = s²
and nothing below needs a square root except the cutoff.

Her price on the circle-symmetric family (x, y, t) is
    λ = (x + (j+2)/√t)² + 2√t·x − y²,
and the book's claim is that it splits, exactly, into three pieces that each mean something:
    λ = κ²  +  (2j + t + y²)  +  (4 − 2t − 2B),
    κ = (t−2)/√t − x − j/√t      (her net turning: stride plus two belts),
    B = y² − 2√t·x               (the field's own curvature).
-/
import Mathlib

namespace Helike

/-- Her price in sector `j`, on the background `(x, y)`, at stretch `t = s²`. -/
noncomputable def price (x y s j : ℝ) : ℝ :=
  (x + (j + 2) / s) ^ 2 + 2 * s * x - y ^ 2

/-- Her net turning around her long circle: the sphere's turning `(t−2)/√t`, the field's push `−x`, and her own
turning `−j/√t`. -/
noncomputable def kappa (x s j : ℝ) : ℝ :=
  (s ^ 2 - 2) / s - x - j / s

/-- The field's curvature in the plane she spreads across. -/
def fieldCurv (x y s : ℝ) : ℝ :=
  y ^ 2 - 2 * s * x

/-- **The anatomy (Long Form 4.1, Rung 10).** Her price is exactly the walkway term `κ²`, plus the lowest-Landau-level
crossing cost `2j + t + y²`, plus the curvature term `(4 − 2t) − 2B`. Nothing is approximated. -/
theorem price_anatomy (x y j : ℝ) {s : ℝ} (hs : s ≠ 0) :
    price x y s j =
      kappa x s j ^ 2 + (2 * j + s ^ 2 + y ^ 2) + ((4 - 2 * s ^ 2) - 2 * fieldCurv x y s) := by
  unfold price kappa fieldCurv
  field_simp
  ring

/-! ### The Berger field: `x = (2 − t)/√t`, `y = √t` -/

/-- The floor law across the Berger spheres `(t, 1, 1)`. -/
noncomputable def floorLaw (t j : ℝ) : ℝ :=
  (j ^ 2 + (8 - 2 * t) * j - 2 * t ^ 2 - 4 * t + 16) / t

/-- The same law in the form the Floor paper prints it. -/
theorem floorLaw_paper_form (t j : ℝ) :
    floorLaw t j = (j * (j + 2) - 2 * (t - 3) * j - 2 * (t - 2) * (t + 4)) / t := by
  unfold floorLaw
  ring

/-- On the Berger field, her net turning is `(2t − 4 − j)/√t`. -/
theorem kappa_berger (j : ℝ) {s : ℝ} (hs : s ≠ 0) :
    kappa ((2 - s ^ 2) / s) s j = (2 * s ^ 2 - 4 - j) / s := by
  unfold kappa
  field_simp
  ring

/-- On the Berger field, the field's curvature is `3t − 4`. -/
theorem fieldCurv_berger {s : ℝ} (hs : s ≠ 0) :
    fieldCurv ((2 - s ^ 2) / s) s s = 3 * s ^ 2 - 4 := by
  unfold fieldCurv
  field_simp
  ring

/-- **Her price on the Berger field is the floor law.** -/
theorem price_berger (j : ℝ) {s : ℝ} (hs : s ≠ 0) :
    price ((2 - s ^ 2) / s) s s j = floorLaw (s ^ 2) j := by
  unfold price floorLaw
  field_simp
  ring

/-- Long Form 4.1, Rung 11: at stretch `t = 3` the law is `(j(j+2) − 14)/3`. -/
theorem floorLaw_three (j : ℝ) : floorLaw 3 j = (j * (j + 2) - 14) / 3 := by
  unfold floorLaw
  ring

theorem floorLaw_three_sector0 : floorLaw 3 0 = -14 / 3 := by
  norm_num [floorLaw]

theorem floorLaw_three_sector12 : floorLaw 3 12 = 154 / 3 := by
  norm_num [floorLaw]

/-! ### The nomograph (Figure 4.1): vertices walk down a ruler -/

/-- Each parabola in vertex form: bottom at `j = t − 4`, height `4 − 3t`. -/
theorem floorLaw_vertex_form (j : ℝ) {t : ℝ} (ht : t ≠ 0) :
    floorLaw t j = (j - (t - 4)) ^ 2 / t + (4 - 3 * t) := by
  unfold floorLaw
  field_simp
  ring

/-- For a genuine stretch `t > 0`, no sector is cheaper than the vertex value `4 − 3t`. -/
theorem floorLaw_ge_vertex (j : ℝ) {t : ℝ} (ht : 0 < t) :
    4 - 3 * t ≤ floorLaw t j := by
  rw [floorLaw_vertex_form j ht.ne']
  have h : 0 ≤ (j - (t - 4)) ^ 2 / t := div_nonneg (sq_nonneg _) ht.le
  linarith

/-- **The solid dots lie on one straight line.** Every stretch's cheapest point `(t − 4, 4 − 3t)` satisfies
`λ = −3j − 8`. -/
theorem vertex_on_line {t : ℝ} (ht : t ≠ 0) :
    floorLaw t (t - 4) = -3 * (t - 4) - 8 := by
  rw [floorLaw_vertex_form (t - 4) ht]
  ring

/-- **The cutoff.** For `t ≥ 4/3` her price crosses zero at `j_c = t − 4 + √(t(3t − 4))`. -/
theorem floorLaw_cutoff {t : ℝ} (ht : 4 / 3 ≤ t) :
    floorLaw t (t - 4 + Real.sqrt (t * (3 * t - 4))) = 0 := by
  have htpos : 0 < t := by linarith
  have hsq : Real.sqrt (t * (3 * t - 4)) ^ 2 = t * (3 * t - 4) :=
    Real.sq_sqrt (by nlinarith)
  rw [floorLaw_vertex_form _ htpos.ne']
  have hshift : t - 4 + Real.sqrt (t * (3 * t - 4)) - (t - 4) = Real.sqrt (t * (3 * t - 4)) := by ring
  rw [hshift, hsq]
  field_simp
  ring

end Helike
