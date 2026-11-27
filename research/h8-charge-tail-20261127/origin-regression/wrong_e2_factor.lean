module

public import H7CubicQuartic
public import Mathlib.Analysis.SpecialFunctions.Pow.Real

/-! Scalar algebra, ad-invariance identities and the conditional ordering for the O(g^2) splitting of the
two-quantum colour singlets at the origin. Polynomial differentiation, Gaussian moments, Schur's lemma, the
quasimode estimate and spectral localization are in the written proof, not here.
-/

set_option autoImplicit false
public section
namespace H8Origin

/-- The second-order coefficient: `-(2κ/ω₁)(1/u + 1/ω₁)` with `ω₁ = √t`, `u = 1/√t`. -/
theorem e2_closed_form (κ t w1 u : ℝ) (ht : 0 < t) (hw : w1 = Real.sqrt t) (hu : u = 1 / Real.sqrt t) :
    -(2 * κ / w1) * (1 / u + 1 / w1) = -4 * κ * (1 + 1 / t) := by
  have hs : Real.sqrt t ^ 2 = t := Real.sq_sqrt ht.le
  have hp : 0 < Real.sqrt t := Real.sqrt_pos.mpr ht
  subst hw hu
  generalize Real.sqrt t = r at hs hp ⊢
  subst hs
  have hr : r ≠ 0 := hp.ne'
  field_simp

theorem e2_negative (κ t : ℝ) (hk : 0 < κ) (ht : 0 < t) : -2 * κ * (1 + 1 / t) < 0 := by
  have h : 0 < 1 + 1 / t := by positivity
  nlinarith

/-- `∇C·∇(‖X₂‖² − ‖X₃‖²) = 2⟨X₂,[X₃,X₁]⟩ − 2⟨X₃,[X₁,X₂]⟩ = 0`. -/
theorem charge_two_annihilated {V : Type*} (B : V → V → ℝ) (br : V → V → V)
    (hinv : ∀ x y z, B (br x y) z = B x (br y z)) (hsym : ∀ x y, B x y = B y x)
    (X1 X2 X3 : V) :
    2 * B X2 (br X3 X1) - 2 * B X3 (br X1 X2) = 0 := by
  have h : B X2 (br X3 X1) = B X3 (br X1 X2) := by
    rw [hsym X2, hinv]
  rw [h]
  ring

/-- `∇C·∇⟨X₂,X₃⟩ = ⟨[X₃,X₁],X₃⟩ + ⟨X₂,[X₁,X₂]⟩ = 0`. -/
theorem charge_two_cross_annihilated {V : Type*} (B : V → V → ℝ) (br : V → V → V)
    (hinv : ∀ x y z, B (br x y) z = B x (br y z)) (hsym : ∀ x y, B x y = B y x)
    (hanti : ∀ x y z, B (br x y) z = -B (br y x) z) (X1 X2 X3 : V) :
    B (br X3 X1) X3 + B X2 (br X1 X2) = 0 := by
  have h1 : B (br X3 X1) X3 = B (br X1 X3) X3 := by
    rw [hinv, hsym]
  have h2 : B (br X3 X1) X3 = -B (br X1 X3) X3 := hanti X3 X1 X3
  have h3 : B X2 (br X1 X2) = B (br X2 X2) X1 := by
    rw [hsym X2, hinv, hsym]
  have h4 : B (br X2 X2) X1 = -B (br X2 X2) X1 := hanti X2 X2 X1
  linarith

/-- `−∇C·∇(‖X₂‖² + ‖X₃‖²) = −4C`, with `C = ⟨X₁,[X₂,X₃]⟩`. -/
theorem charge_zero_image {V : Type*} (B : V → V → ℝ) (br : V → V → V)
    (hinv : ∀ x y z, B (br x y) z = B x (br y z)) (hsym : ∀ x y, B x y = B y x)
    (X1 X2 X3 : V) :
    -(2 * B X2 (br X3 X1) + 2 * B X3 (br X1 X2)) = -4 * B X1 (br X2 X3) := by
  have h2 : B X2 (br X3 X1) = B X1 (br X2 X3) := by
    rw [hsym X1, hinv]
  have h3 : B X3 (br X1 X2) = B X1 (br X2 X3) := by
    rw [hsym X3, ← hinv]
  rw [h2, h3]
  ring

/-- Two quasimode energies with the stated remainders: the charge-0 one is lower in the window `K g < −e₂/2`. -/
theorem ordering_of_quasimodes (c e2 K g E0 E2 : ℝ) (hgpos : 0 < g)
    (h0 : |E0 - (c + e2 * g^2)| ≤ K * g^3) (h2 : |E2 - c| ≤ K * g^3) (hg : K * g < -e2 / 2) :
    E0 < E2 := by
  have a0 := (abs_le.mp h0).2
  have a2 := (abs_le.mp h2).1
  have hg2 : 0 < g^2 := by positivity
  have key : g^2 * (e2 + 2 * (K * g)) < 0 := mul_neg_of_pos_of_neg hg2 (by linarith)
  nlinarith

end H8Origin
