/-
The Davis Duality (2026), reread: Lean targets D1–D3 from `docs/research/davis-duality-review-2026-11-22.md`.

* D1  the *pointwise* lower bound is false: for f = x² and the affine interpolant f̂ = h·x on [0, h], the error at
      x = 0 is zero while the Hessian is 2;
* D2  the per-cell lower bound, by second differences: for any affine ℓ, if e = f − ℓ has second difference D at a
      point, then the largest |e| among the three points is at least |D|/4;
* D3  the constant 1/16 is sharp: on [0, h] the affine ℓ(x) = h·x − h²/8 keeps |x² − ℓ(x)| ≤ h²/8, which is
      λh²/16 with λ = 2.
-/
import Mathlib

namespace DavisManifold

/-- **D1.** The affine interpolant `h·x` of `x²` on `[0, h]` has zero error at `x = 0`, where the Hessian of `x²` is `2`.
So no bound `c₁ · 2 · h² ≤ |error(x)|` with `c₁ > 0` can hold at every point. -/
theorem pointwise_lower_bound_fails {h c₁ : ℝ} (hh : 0 < h) (hc : 0 < c₁) :
    |((0 : ℝ) ^ 2 - h * 0)| < c₁ * 2 * h ^ 2 := by
  simp; positivity

/-- **D2.** Second differences kill affine functions: if `ℓ` is affine along the line (`ℓ(p+v) + ℓ(p−v) = 2ℓ(p)`) and
the errors `e₊ = f(p+v) − ℓ(p+v)`, `e₋`, `e₀` satisfy `e₊ + e₋ − 2e₀ = D`, then one of them has `|e| ≥ |D|/4`. -/
theorem second_difference_lower {ep em e0 D : ℝ} (h : ep + em - 2 * e0 = D) :
    |D| / 4 ≤ max (max |ep| |em|) |e0| := by
  have h1 : |D| ≤ |ep| + |em| + 2 * |e0| := by
    rw [← h]
    calc |ep + em - 2 * e0| ≤ |ep + em| + |2 * e0| := abs_sub _ _
      _ ≤ |ep| + |em| + |2 * e0| := by gcongr; exact abs_add_le _ _
      _ = |ep| + |em| + 2 * |e0| := by rw [abs_mul]; norm_num
  have hm1 : |ep| ≤ max (max |ep| |em|) |e0| := le_trans (le_max_left _ _) (le_max_left _ _)
  have hm2 : |em| ≤ max (max |ep| |em|) |e0| := le_trans (le_max_right _ _) (le_max_left _ _)
  have hm3 : |e0| ≤ max (max |ep| |em|) |e0| := le_max_right _ _
  linarith

/-- **D2, applied.** For `f = ½λx²` the second difference at step `v` is exactly `λv²`; with `v = h/2` this gives
the per-cell lower bound `λh²/16`. -/
theorem quadratic_second_difference (lam p v : ℝ) :
    lam / 2 * (p + v) ^ 2 + lam / 2 * (p - v) ^ 2 - 2 * (lam / 2 * p ^ 2) = lam * v ^ 2 := by ring

theorem per_cell_constant (lam h : ℝ) : lam * (h / 2) ^ 2 / 4 = lam * h ^ 2 / 16 := by ring

/-- **D3.** The constant `1/16` is attained: on `[0, h]`, `x² − (h·x − h²/8) = (x − h/2)² − h²/8` lies in
`[−h²/8, h²/8]`, and `h²/8 = λh²/16` for `λ = 2`. -/
theorem sharp_sixteenth {h x : ℝ} (h0 : 0 ≤ x) (h1 : x ≤ h) :
    |x ^ 2 - (h * x - h ^ 2 / 8)| ≤ 2 * h ^ 2 / 16 := by
  rw [abs_le]
  constructor <;> nlinarith [sq_nonneg (x - h / 2)]

end DavisManifold
