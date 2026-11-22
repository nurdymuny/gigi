/-
The ballooning operator's anatomy (CHT form used by the plasma-confinement paper's Exps 12 and 18b):
f = 1 + Λ², Λ = sθ/κ − α sin θ, on the Gaussian trial ξ = exp(−γθ²/2).

To fourth order in θ (sin θ ≈ θ, so Λ ≈ (s/κ − α)θ), the stabilizing Rayleigh term is
  R_stab = γ² ( ⟨θ²⟩ + (s/κ − α)² ⟨θ⁴⟩ ),  ⟨θ²⟩ = 1/(2γ), ⟨θ⁴⟩ = 3/(4γ²),
and equals γ/2 + ¾ (s/κ − α)²: a perfect square of the net turning (stride minus belt), the same shape as the
floor's price. Proved here as an identity in the Gaussian moments.

Scope: this is the fourth-order algebra. The sixth-order truncation was registered (B1–B3) and FAILED, because it
is unbounded below for wide Gaussians. The exact Gaussian quotient (B4–B5, held) is computed by quadrature, not here.
-/
import Mathlib

namespace Chihiro

/-- Gaussian moments under the normalized weight `exp(−γθ²)`. -/
noncomputable def m2 (γ : ℝ) : ℝ := 1 / (2 * γ)
noncomputable def m4 (γ : ℝ) : ℝ := 3 / (4 * γ ^ 2)

/-- **The stabilizing term is a perfect square of the net turning, plus the trial's own cost.** -/
theorem rstab_fourth_order {γ s κ α : ℝ} (hγ : γ ≠ 0) :
    γ ^ 2 * (m2 γ + (s / κ - α) ^ 2 * m4 γ) = γ / 2 + 3 / 4 * (s / κ - α) ^ 2 := by
  unfold m2 m4
  field_simp

/-- **The square vanishes exactly where stride and belt cancel,** `s = κα`, and is never negative: the bending
cost is least when the field-line turning is cancelled. -/
theorem net_turning_zero {s κ α : ℝ} (hκ : κ ≠ 0) (h : s = κ * α) : (s / κ - α) ^ 2 = 0 := by
  subst h; field_simp; ring

theorem net_turning_nonneg (s κ α : ℝ) : 0 ≤ 3 / 4 * (s / κ - α) ^ 2 := by positivity

end Chihiro
