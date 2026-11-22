/-
The Plateau Tangent Proposition of the plasma-confinement paper (Prop. `prop:plateau`), and the algebra behind its
uniqueness remark.

* `tangent_condition`: if `α` is differentiable at `s > 0`, `α s > 0`, and `C(t) = t / α t` has a local maximum at
  `s`, then `α' s = α s / s`.
* `deriv_C`: `C' = (α − s α') / α²` (the paper's derivative).
* `stationary_upward`: in `u = log s`, write `g(u) = log α(eᵘ)`, so `g' = s·(log α)'` and `g'' = s²(log α)'' + s(log α)'`.
  At a stationary point of `C` (`g' = 1`), `g'' = s²(log α)'' + 1`, which is positive when `(log α)'' > 0`. So every
  stationary point is a strict upward crossing of `g' = 1`, and there is at most one. This is why the paper's
  log-convexity remark is correct; the paper does not give the reason.
-/
import Mathlib

namespace Chihiro

/-- The derivative of `C(t) = t / α t` at `s`, when `α` is differentiable there with `α s ≠ 0`. -/
theorem deriv_C {α : ℝ → ℝ} {s a' : ℝ} (hα : HasDerivAt α a' s) (h0 : α s ≠ 0) :
    HasDerivAt (fun t => t / α t) ((α s - s * a') / α s ^ 2) s := by
  have h := (hasDerivAt_id' s).div hα h0
  convert h using 1
  ring

/-- **The tangent condition.** At an interior local maximum of `C(t) = t / α t`, `α' = α / s`. -/
theorem tangent_condition {α : ℝ → ℝ} {s a' : ℝ} (hs : 0 < s) (hα : HasDerivAt α a' s) (hpos : 0 < α s)
    (hmax : IsLocalMax (fun t => t / α t) s) : a' = α s / s := by
  have hd := deriv_C hα hpos.ne'
  have h0 : (α s - s * a') / α s ^ 2 = 0 := hmax.hasDerivAt_eq_zero hd
  have hsq : α s ^ 2 ≠ 0 := by positivity
  rw [div_eq_zero_iff] at h0
  rcases h0 with h0 | h0
  · field_simp; linarith
  · exact absurd h0 hsq

/-- **Every stationary point crosses upward.** With `p = (log α)'(s)` and `q = (log α)''(s)`, the second derivative
of `g(u) = log α(eᵘ)` is `s² q + s p`. At a stationary point of `C`, `s p = 1`, so it equals `s² q + 1`, positive
whenever `q > 0` (log-convexity). -/
theorem stationary_upward {s p q : ℝ} (hstat : s * p = 1) (hq : 0 < q) : 0 < s ^ 2 * q + s * p := by
  rw [hstat]; positivity

/-- **The condition that actually holds.** Log-convexity (`q > 0`) is not needed: at a stationary point (`s p = 1`)
the log-curvature `s² q + s p` is positive as soon as `s² q > −1`. On the Exp 12 solver, `α_crit` is log-*concave*
on s ∈ [0.9, 4.9] in all four classes, yet `s² q ≈ −0.67` to `−0.74` at the peaks, so every stationary point still
crosses upward and the peak is unique there (receipt `tmp/v3_draft/plateau_check/` in the CHIHIRO repository). -/
theorem stationary_upward_weak {s p q : ℝ} (hstat : s * p = 1) (hq : -1 < s ^ 2 * q) :
    0 < s ^ 2 * q + s * p := by
  rw [hstat]; linarith

end Chihiro
