/-
HELIKE, Chapter 4, Section 4.3: the fall.

The circle-symmetric family `M₀ = x τ₀, M₁ = y τ₁, M₂ = y τ₂` on the Berger sphere `(t, 1, 1)`, with `t = s²`.
Its two curvatures are `B = y² − 2√t·x` (along the fibre, `fieldCurv` in `Price.lean`) and `c = y(x − 2/√t)`
(across it), and its Yang–Mills energy density is `E = ½(B² + 2c²)`.

Proved here:
* the Berger field's energy is `½((3t − 4)² + 2t²)` (34.71 at t = 3.55, the Floor paper's 34.6);
* the two flat fields of the family, `A = 0` and `A = θ`, have both curvatures zero and energy zero;
* her price at the bottom of the fall: `(j + 2)²/t` at `A = 0` and `(j + 4)²/t` at `A = θ`, both positive.

That the fall stays inside the family, and where it lands, are computed, not proved here.
-/
import Helike.Price

namespace Helike

/-- The curvature across the fibre, `c = y(x − 2/√t)`. -/
noncomputable def crossCurv (x y s : ℝ) : ℝ :=
  y * (x - 2 / s)

/-- The Yang–Mills energy density of a member of the family, `½(B² + 2c²)`. -/
noncomputable def energy (x y s : ℝ) : ℝ :=
  (fieldCurv x y s ^ 2 + 2 * crossCurv x y s ^ 2) / 2

/-- On the Berger field the cross curvature is `−t`. -/
theorem crossCurv_berger {s : ℝ} (hs : s ≠ 0) :
    crossCurv ((2 - s ^ 2) / s) s s = -s ^ 2 := by
  unfold crossCurv
  field_simp
  ring

/-- **The Berger field's energy** is `½((3t − 4)² + 2t²)`. At `t = 3.55` this is `34.71…`, the start of the fall. -/
theorem energy_berger {s : ℝ} (hs : s ≠ 0) :
    energy ((2 - s ^ 2) / s) s s = ((3 * s ^ 2 - 4) ^ 2 + 2 * (s ^ 2) ^ 2) / 2 := by
  unfold energy
  rw [fieldCurv_berger hs, crossCurv_berger hs]
  ring

/-- `A = 0` is flat: both curvatures vanish. -/
theorem flat_zero (s : ℝ) : fieldCurv 0 0 s = 0 ∧ crossCurv 0 0 s = 0 := by
  unfold fieldCurv crossCurv
  constructor <;> ring

/-- `A = θ`, the Maurer–Cartan form, sits in the family at `x = 2/√t`, `y = 2`, and is flat. -/
theorem flat_theta {s : ℝ} (hs : s ≠ 0) : fieldCurv (2 / s) 2 s = 0 ∧ crossCurv (2 / s) 2 s = 0 := by
  unfold fieldCurv crossCurv
  constructor
  · field_simp
    ring
  · ring

theorem energy_zero (s : ℝ) : energy 0 0 s = 0 := by
  unfold energy
  rw [(flat_zero s).1, (flat_zero s).2]
  ring

theorem energy_theta {s : ℝ} (hs : s ≠ 0) : energy (2 / s) 2 s = 0 := by
  unfold energy
  rw [(flat_theta hs).1, (flat_theta hs).2]
  ring

/-- **Her price at the bottom of the trivial basin**, `A = 0`: `(j + 2)²/t`. -/
theorem price_zero {s : ℝ} (hs : s ≠ 0) (j : ℝ) : price 0 0 s j = (j + 2) ^ 2 / s ^ 2 := by
  unfold price
  field_simp
  ring

/-- **Her price at the bottom of the fall**, `A = θ`: `(j + 4)²/t`. -/
theorem price_theta {s : ℝ} (hs : s ≠ 0) (j : ℝ) : price (2 / s) 2 s j = (j + 4) ^ 2 / s ^ 2 := by
  unfold price
  field_simp
  ring

/-- At both bottoms she is a wall: her price is positive in every family `j ≥ 0`. -/
theorem price_zero_pos {s : ℝ} (hs : s ≠ 0) {j : ℝ} (hj : 0 ≤ j) : 0 < price 0 0 s j := by
  rw [price_zero hs]
  have h1 : 0 < (j + 2) ^ 2 := by positivity
  have h2 : 0 < s ^ 2 := by positivity
  exact div_pos h1 h2

theorem price_theta_pos {s : ℝ} (hs : s ≠ 0) {j : ℝ} (hj : 0 ≤ j) : 0 < price (2 / s) 2 s j := by
  rw [price_theta hs]
  have h1 : 0 < (j + 4) ^ 2 := by positivity
  have h2 : 0 < s ^ 2 := by positivity
  exact div_pos h1 h2

end Helike
