/-
The Davis Non-Decoupling Theorem (2026), reread line by line: Lean targets from
`docs/research/non-decoupling-review-2026-11-22.md`.

* N1  a bounded error cannot dominate an unbounded energy linearly: if `𝓔 ≤ D` always and `S_YM` takes arbitrarily
      large values, no fixed `c > 0` gives `𝓔 ≥ c · S_YM` (why Part II fails);
* N2  the monopole flux `∫₀^{2π} ∫₀^π ½ sin θ dθ dφ = 2π` (first Chern number one on S²);
* N3  the dimension-2 topological floor: `(2πk)² ≤ 4π · I` forces `I ≥ π k²`;
* N4  the high-frequency ratio is unbounded: for every `C` there is `N` with `N² > C` (why Lemma 4 fails).
-/
import Mathlib

namespace NonDecoupling

open Real intervalIntegral

/-- **N1.** Suppose the modeling error is bounded, `𝓔 x ≤ D` for every connection `x`, while the Yang–Mills energy
`S` is unbounded above. Then for every `c > 0` some connection violates `c · S x ≤ 𝓔 x`. -/
theorem no_linear_bound {X : Type*} (E S : X → ℝ) {D : ℝ} (hE : ∀ x, E x ≤ D)
    (hS : ∀ M : ℝ, ∃ x, M < S x) {c : ℝ} (hc : 0 < c) :
    ∃ x, E x < c * S x := by
  obtain ⟨x, hx⟩ := hS (D / c)
  refine ⟨x, lt_of_le_of_lt (hE x) ?_⟩
  rwa [div_lt_iff₀ hc, mul_comm] at hx

/-- **N2, the polar integral.** `∫₀^π sin θ dθ = 2`. -/
theorem integral_sin_zero_pi : ∫ θ in (0 : ℝ)..π, Real.sin θ = 2 := by
  rw [integral_sin]
  simp [Real.cos_pi]
  norm_num

/-- **N2.** The monopole curvature `F = ½ sin θ dθ ∧ dφ` on the unit sphere has total flux `2π`, i.e. first Chern
number `(1/2π) ∫ F = 1`: the bundle is non-trivial, so by Chern–Weil no connection on it is flat. -/
theorem monopole_flux :
    ∫ _φ in (0 : ℝ)..(2 * π), ∫ θ in (0 : ℝ)..π, (1 / 2 : ℝ) * Real.sin θ = 2 * π := by
  have h : ∫ θ in (0 : ℝ)..π, (1 / 2 : ℝ) * Real.sin θ = 1 := by
    rw [intervalIntegral.integral_const_mul, integral_sin_zero_pi]; norm_num
  rw [h]
  simp

/-- **N3.** In dimension 2: if the total flux is `2πk` and Cauchy–Schwarz gives `(∫F)² ≤ Area · ∫F²` with
`Area = 4π` (the unit sphere), then `∫F² ≥ π k²`, so `S_YM = ½ ∫F² ≥ π k² / 2`. -/
theorem dim2_floor {k I : ℝ} (hCS : (2 * π * k) ^ 2 ≤ 4 * π * I) : π * k ^ 2 ≤ I := by
  have hpi : 0 < π := Real.pi_pos
  have : 4 * π * (π * k ^ 2) ≤ 4 * π * I := by nlinarith
  exact le_of_mul_le_mul_left this (by positivity)

/-- **N4.** The ratio `S_YM / ‖α‖²` for `α = ε sin(Nx)` grows like `N²`; for any proposed constant there is a
frequency that beats it. -/
theorem ratio_unbounded (C : ℝ) : ∃ N : ℕ, C < (N : ℝ) ^ 2 := by
  obtain ⟨N, hN⟩ := exists_nat_gt (max C 1)
  refine ⟨N, ?_⟩
  have h1 : (1 : ℝ) < N := lt_of_le_of_lt (le_max_right _ _) hN
  have hC : C < N := lt_of_le_of_lt (le_max_left _ _) hN
  nlinarith

end NonDecoupling
