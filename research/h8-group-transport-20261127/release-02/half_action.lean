module

public import H8Interwell
public import Mathlib.Algebra.Module.LinearMap.Defs

/-! Conditional transport algebra. Pairings and brackets are explicit parameters.
Their geometric interpretation, the existence of embeddings and the global
path argument are established in the written proof, not in this library. -/

set_option autoImplicit false
public section

namespace H8Group

open H8

variable {E F : Type*} [AddCommGroup E] [Module ℝ E] [AddCommGroup F] [Module ℝ F]

theorem gradient_intertwines (L : E →ₗ[ℝ] F) (b : E → E → E) (c : F → F → F)
    (x y z : E) (ω : ℝ) (hbr : L (b x y) = c (L x) (L y)) :
    L (ω • z - b x y) = ω • L z - c (L x) (L y) := by
  rw [map_sub, map_smul, hbr]

theorem cubic_transfer (L : E →ₗ[ℝ] F) (B : E → E → ℝ) (C : F → F → ℝ)
    (b : E → E → E) (c : F → F → F) (I : ℝ)
    (hbr : ∀ x y, L (b x y) = c (L x) (L y))
    (hmetric : ∀ x y, C (L x) (L y) = I * B x y) (x y z : E) :
    C (L x) (c (L y) (L z)) = I * B x (b y z) := by
  rw [← hbr, hmetric]

@[expose] noncomputable def superpotential (B : E → E → ℝ) (b : E → E → E)
    (w1 w2 w3 : ℝ) (x y z : E) : ℝ :=
  (w1 * B x x + w2 * B y y + w3 * B z z) / 2 - B x (b y z)

theorem superpotential_transfer (L : E →ₗ[ℝ] F) (B : E → E → ℝ) (C : F → F → ℝ)
    (b : E → E → E) (c : F → F → F) (I : ℝ)
    (hbr : ∀ x y, L (b x y) = c (L x) (L y))
    (hmetric : ∀ x y, C (L x) (L y) = I * B x y)
    (w1 w2 w3 : ℝ) (x y z : E) :
    superpotential C c w1 w2 w3 (L x) (L y) (L z)
      = I * superpotential B b w1 w2 w3 x y z := by
  simp only [superpotential, ← hbr, hmetric]
  ring

theorem plane_pairing (I u x y vx vy : ℝ) :
    I * (x / u - y ^ 2) * vx + 2 * I * ((u - x) * y) * vy =
      -(I * fx u x y * vx + 2 * I * fy u x y * vy) := by
  unfold fx fy
  ring

theorem plane_dissipation (I u x y : ℝ) :
    I * ((x / u - y ^ 2) * fx u x y + 2 * ((u - x) * y) * fy u x y) =
      -I * (fx u x y ^ 2 + 2 * fy u x y ^ 2) := by
  rw [H8.dissipation]
  ring

theorem scaled_saddle_height {u : ℝ} (hu : 0 < u) (I : ℝ) :
    I * (W u u 1 - W u 0 0) = I * u / 4 := by
  rw [H8.saddle_height hu]
  ring

theorem action_density_scale {I : ℝ} (hI : 0 < I) (a b : ℝ) :
    Real.sqrt (I * a) * Real.sqrt (I * b) = I * (Real.sqrt a * Real.sqrt b) := by
  rw [Real.sqrt_mul (le_of_lt hI), Real.sqrt_mul (le_of_lt hI)]
  calc
    Real.sqrt I * Real.sqrt a * (Real.sqrt I * Real.sqrt b) =
        (Real.sqrt I) ^ 2 * (Real.sqrt a * Real.sqrt b) := by ring
    _ = I * (Real.sqrt a * Real.sqrt b) := by rw [Real.sq_sqrt (le_of_lt hI)]

/-- The two global distance estimates are hypotheses, not conclusions of this lemma. -/
theorem distance_squeeze (d I u : ℝ) (lower : I * u / 2 ≤ d)
    (upper : ∀ ε : ℝ, 0 < ε → d ≤ I * u / 2 + ε) : d = I * u / 2 := by
  apply le_antisymm _ lower
  by_contra h
  have hd : I * u / 2 < d := lt_of_not_ge h
  have hs := upper ((d - I * u / 2) / 2) (by linarith)
  linarith

theorem coupling_action {g : ℝ} (hg : 0 < g) (I u : ℝ) :
    (I * (u / 2)) / g ^ 2 = I * u / (2 * g ^ 2) := by
  field_simp [ne_of_gt hg]

end H8Group
