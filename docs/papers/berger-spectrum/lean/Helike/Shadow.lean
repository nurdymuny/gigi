/-
HELIKE, Chapter 4, Section 4.3: her shadow.

The wrap (the gauge change that carries `A = 0` to `A = θ` on the round three-sphere) moves a state two families up.
Her own prices already know this: her price at `A = θ` in family `j` is exactly her price at `A = 0` in family
`j + 2`. On the Poincaré sphere the lowest wall of the icosahedral basin in family `j ≥ 12` costs `j²/t`, which is her
price at `A = 0` two families down.

Proved here: the price identities. That `j²/t` is the lowest wall at `A = θ`, and that the state is her eigenvector
carried by the wrap, are computed, not proved here.
-/
import Helike.Fall

namespace Helike

/-- **The wrap moves her two families.** Her price at `A = θ` in family `j` equals her price at `A = 0` in family
`j + 2`. -/
theorem price_theta_eq_shift {s : ℝ} (hs : s ≠ 0) (j : ℝ) : price (2 / s) 2 s j = price 0 0 s (j + 2) := by
  rw [price_theta hs, price_zero hs]
  ring

/-- **The shadow's price.** Her price at `A = 0`, two families down, is `j²/t`. -/
theorem shadow_price {s : ℝ} (hs : s ≠ 0) (j : ℝ) : price 0 0 s (j - 2) = j ^ 2 / s ^ 2 := by
  rw [price_zero hs]
  ring

end Helike
