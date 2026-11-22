/-
HELIKE, Chapter 5: the two hands.

A row of a charge block has diagonal entry `(w − 2m/√t)²` with `w = f − c_l·x`, where `c_l = ±1` is the state's
internal turning, `x` the field along the long circle and `m` its weight along the circle. At `m = l`:
`(f − c_l·x − 2l/√t)²`.

Proved here, by isolating the bilinear part (the mixed second difference in `x` and `l`):
* the cross term of every such row is `+4·c_l·x·l/√t`;
* hers (`c_l = +1`) minus her sister's (`c_l = −1`) is `+8·x·l/√t`, so her sister undercuts her by `8xl/√t` when
  `x > 0`;
* in her own closed-form price the matching term is `2xj/√t`, which is `4xl/√t` at `j = 2l`.

That these rows are the operator's rows is the symbolic Weitzenböck identity of the edge paper, not proved here.
-/
import Helike.Price

namespace Helike

/-- The diagonal entry of a row at weight `m = l`. -/
noncomputable def rowEntry (f c x l s : ℝ) : ℝ :=
  (f - c * x - 2 * l / s) ^ 2

/-- The mixed second difference in `x` and `l`: the part of a quantity that is bilinear in `x` and `l`. -/
noncomputable def mixed (g : ℝ → ℝ → ℝ) (x l : ℝ) : ℝ :=
  g x l - g 0 l - g x 0 + g 0 0

/-- **The cross term.** In every row, the bilinear part in `x` and `l` is `4·c_l·x·l/√t`. -/
theorem rowEntry_cross (f c s x l : ℝ) (hs : s ≠ 0) :
    mixed (fun x l => rowEntry f c x l s) x l = 4 * c * x * l / s := by
  unfold mixed rowEntry
  field_simp
  ring

/-- **The cheap lift reaches across.** Her cross term minus her sister's is `8xl/√t`. -/
theorem her_minus_sister (f s x l : ℝ) (hs : s ≠ 0) :
    mixed (fun x l => rowEntry f 1 x l s) x l - mixed (fun x l => rowEntry f (-1) x l s) x l = 8 * x * l / s := by
  rw [rowEntry_cross f 1 s x l hs, rowEntry_cross f (-1) s x l hs]
  field_simp
  ring

/-- In her own price the bilinear part in `x` and `j` is `2xj/√t`. -/
theorem price_cross (y s x j : ℝ) (hs : s ≠ 0) :
    mixed (fun x j => price x y s j) x j = 2 * x * j / s := by
  unfold mixed price
  field_simp
  ring

/-- **The two agree.** With `j = 2l`, her price's cross term is her row's cross term `4xl/√t`. -/
theorem price_cross_matches_row (f y s x l : ℝ) (hs : s ≠ 0) :
    mixed (fun x j => price x y s j) x (2 * l) = mixed (fun x l => rowEntry f 1 x l s) x l := by
  rw [price_cross y s x (2 * l) hs, rowEntry_cross f 1 s x l hs]
  ring

end Helike
