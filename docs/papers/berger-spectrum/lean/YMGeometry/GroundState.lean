/-
YANG–MILLS REQUIREMENTS AS GEOMETRIC OBJECTS, Section 8: when the gap becomes a weighted geometric inequality.

The note states the ground-state transform for a Schrödinger operator −κΔ_g + V on a configuration manifold, which
needs self-adjointness, boundary and domain conditions before integrating by parts. On a finite weighted graph the
same transform is an exact identity with no analytic hypothesis at all:

    H x (i) = κ Σ_j w_ij (x_i − x_j) + V_i x_i,      H ψ = E₀ ψ,
    ⟨ψ f, (H − E₀)(ψ f)⟩ = (κ/2) Σ_i Σ_j w_ij ψ_i ψ_j (f_i − f_j)².

The right side is a Dirichlet form for the measure ν_i = ψ_i² once ψ > 0 and w ≥ 0, which is what turns the energy
above the ground state into "weighted squared gradient" — the configuration-space geometry of Section 8. A lattice
gauge theory's configuration space, discretised, is such a graph; `scripts/ym_geometry_tests.py` (T8) checks the
identity on the Kogut–Susskind SU(2) plaquette to 10⁻¹⁵.

Note: the identity uses only `H ψ = E₀ ψ`; it holds for every eigenvector. What the ground state adds is positivity
of the weights ψ_i ψ_j (`dirichlet_nonneg`). A sign-changing eigenvector gives negative weights: no measure, no
Poincaré inequality (gate F3 of T8).
-/
import Mathlib

namespace YMGeometry

open Finset

variable {n : ℕ}

/-- The graph Schrödinger operator `κ·(graph Laplacian) + V`. -/
def graphH (κ : ℝ) (w : Fin n → Fin n → ℝ) (V x : Fin n → ℝ) (i : Fin n) : ℝ :=
  κ * ∑ j, w i j * (x i - x j) + V i * x i

/-- Pointwise: above an eigenvalue, the operator on `ψ f` only sees differences of `f`. -/
lemma graphH_shift (κ E₀ : ℝ) (w : Fin n → Fin n → ℝ) (V ψ f : Fin n → ℝ)
    (hψ : ∀ i, graphH κ w V ψ i = E₀ * ψ i) (i : Fin n) :
    graphH κ w V (fun k => ψ k * f k) i - E₀ * (ψ i * f i) = κ * ∑ j, w i j * ψ j * (f i - f j) := by
  have h := hψ i
  simp only [graphH] at h ⊢
  have hs : ∑ j, w i j * (ψ i * f i - ψ j * f j) - f i * ∑ j, w i j * (ψ i - ψ j)
      = ∑ j, w i j * ψ j * (f i - f j) := by
    rw [mul_sum, ← sum_sub_distrib]
    exact sum_congr rfl fun j _ => by ring
  linear_combination κ * hs + f i * h

/-- **Section 8, the ground-state transform, exactly, on a finite weighted graph.** -/
theorem ground_state_transform (κ E₀ : ℝ) (w : Fin n → Fin n → ℝ) (hw : ∀ i j, w i j = w j i)
    (V ψ f : Fin n → ℝ) (hψ : ∀ i, graphH κ w V ψ i = E₀ * ψ i) :
    ∑ i, (ψ i * f i) * (graphH κ w V (fun k => ψ k * f k) i - E₀ * (ψ i * f i))
      = κ / 2 * ∑ i, ∑ j, w i j * ψ i * ψ j * (f i - f j) ^ 2 := by
  simp_rw [graphH_shift κ E₀ w V ψ f hψ]
  set S1 := ∑ i, ∑ j, w i j * ψ i * ψ j * (f i * (f i - f j)) with hS1
  set S2 := ∑ i, ∑ j, w i j * ψ i * ψ j * (f j * (f j - f i)) with hS2
  have hL : ∑ i, ψ i * f i * (κ * ∑ j, w i j * ψ j * (f i - f j)) = κ * S1 := by
    rw [hS1, mul_sum]
    refine sum_congr rfl fun i _ => ?_
    rw [mul_sum, mul_sum, mul_sum]
    exact sum_congr rfl fun j _ => by ring
  have hswap : S1 = S2 := by
    rw [hS1, hS2, sum_comm]
    exact sum_congr rfl fun i _ => sum_congr rfl fun j _ => by rw [hw j i]; ring
  have hsum : S1 + S2 = ∑ i, ∑ j, w i j * ψ i * ψ j * (f i - f j) ^ 2 := by
    rw [hS1, hS2, ← sum_add_distrib]
    refine sum_congr rfl fun i _ => ?_
    rw [← sum_add_distrib]
    exact sum_congr rfl fun j _ => by ring
  rw [hL, ← hsum, ← hswap]
  ring

/-- With a positive ground state and nonnegative weights the right side is a Dirichlet form: nonnegative, so `E₀`
sits at the bottom on every vector of the form `ψ f`. -/
theorem dirichlet_nonneg {κ : ℝ} (hκ : 0 ≤ κ) {w : Fin n → Fin n → ℝ} (hw0 : ∀ i j, 0 ≤ w i j) {ψ : Fin n → ℝ}
    (hψ0 : ∀ i, 0 < ψ i) (f : Fin n → ℝ) :
    0 ≤ κ / 2 * ∑ i, ∑ j, w i j * ψ i * ψ j * (f i - f j) ^ 2 := by
  apply mul_nonneg (by linarith)
  exact sum_nonneg fun i _ => sum_nonneg fun j _ => by
    have := hψ0 i; have := hψ0 j; have := hw0 i j; positivity

/-- **Gate F4: a wrong `E₀` breaks the identity by exactly the shift times the weighted norm.** -/
theorem ground_state_transform_shift (κ E₀ δ : ℝ) (w : Fin n → Fin n → ℝ) (hw : ∀ i j, w i j = w j i)
    (V ψ f : Fin n → ℝ) (hψ : ∀ i, graphH κ w V ψ i = E₀ * ψ i) :
    ∑ i, (ψ i * f i) * (graphH κ w V (fun k => ψ k * f k) i - (E₀ + δ) * (ψ i * f i))
      = κ / 2 * ∑ i, ∑ j, w i j * ψ i * ψ j * (f i - f j) ^ 2 - δ * ∑ i, (ψ i * f i) ^ 2 := by
  rw [← ground_state_transform κ E₀ w hw V ψ f hψ, mul_sum, ← sum_sub_distrib]
  exact sum_congr rfl fun i _ => by ring

end YMGeometry
