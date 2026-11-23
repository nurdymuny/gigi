/-
YANG–MILLS REQUIREMENTS AS GEOMETRIC OBJECTS, Section 1: conditioning expressed as geometry.

In *Reading the Curve* each cell K recovers a closed linear curvature P(z) = c + B z (tr B = 0) from face fluxes. The
flux map X_K sends the coefficients to the fluxes; the recovery R_K satisfies R_K X_K = I. Two exact facts carry the
note's geometric reading; `scripts/ym_geometry_rtc_cells.py` checks both on all 4,320 cells of the paper's meshes.

* `amplification_lower_bound`: if R ∘ X = id, every coefficient vector satisfies ‖x‖ ≤ ‖R‖·‖X x‖. So the image of the
  unit ball (the flux ellipsoid) has every semiaxis at least 1/‖R‖, i.e. Λ_K = ‖R_K‖ ≥ 1/σ_min(X_K): a short axis of
  the ellipsoid forces a large amplification.
* `triangle_flux_scale`: the flux of P through the straight triangle (z₁, z₂, z₃) is P(centroid)·½(z₂−z₁)×(z₃−z₁)
  (exact for linear P). In units scaled by λ it is λ² times the flux of (c, λB) in the old units. So the flux map in
  physical units is λ²·X·diag(1, λ): with the coefficient metric transported (B weighted by λ) the ellipsoid only
  scales, by λ²; read in raw coefficient length it is distorted. This is the note's statement that the chart's
  diameter normalisation is part of the claim.
-/
import Mathlib

namespace YMGeometry

open Matrix

/-- **Section 1: the amplification is at least the reciprocal of the shortest semiaxis.** -/
theorem amplification_lower_bound {E F : Type*} [NormedAddCommGroup E] [NormedSpace ℝ E]
    [NormedAddCommGroup F] [NormedSpace ℝ F] (X : E →L[ℝ] F) (R : F →L[ℝ] E) (hRX : R.comp X = ContinuousLinearMap.id ℝ E)
    (x : E) : ‖x‖ ≤ ‖R‖ * ‖X x‖ := by
  have h : R (X x) = x := by simpa using congrArg (fun T => T x) hRX
  calc ‖x‖ = ‖R (X x)‖ := by rw [h]
    _ ≤ ‖R‖ * ‖X x‖ := R.le_opNorm (X x)

/-- Exact recovery: a left inverse returns the coefficients from their own fluxes, and so recovers nothing from
fluxes that vanish (pure gauge has zero flux through every face). -/
theorem exact_recovery {E F : Type*} [AddCommGroup E] [Module ℝ E] [AddCommGroup F] [Module ℝ F]
    (X : E →ₗ[ℝ] F) (R : F →ₗ[ℝ] E) (hRX : R ∘ₗ X = LinearMap.id) (x : E) : R (X x) = x ∧ R 0 = 0 := by
  exact ⟨by simpa using congrArg (fun T => T x) hRX, map_zero R⟩

/-- The value of the linear field (c, B) at a point. -/
def fieldAt (c : Fin 3 → ℝ) (B : Matrix (Fin 3) (Fin 3) ℝ) (z : Fin 3 → ℝ) : Fin 3 → ℝ := c + B *ᵥ z

/-- The flux of (c, B) through the straight triangle (z₁, z₂, z₃): the field at the centroid against the area
vector. -/
noncomputable def triangleFlux (c : Fin 3 → ℝ) (B : Matrix (Fin 3) (Fin 3) ℝ) (z₁ z₂ z₃ : Fin 3 → ℝ) : ℝ :=
  fieldAt c B ((1 / 3 : ℝ) • (z₁ + z₂ + z₃)) ⬝ᵥ ((1 / 2 : ℝ) • crossProduct (z₂ - z₁) (z₃ - z₁))

/-- **Section 1, the scaling law behind metric fidelity.** Scaling the chart by λ multiplies every flux by λ² and
weights the gradient part B by λ. -/
theorem triangle_flux_scale (c : Fin 3 → ℝ) (B : Matrix (Fin 3) (Fin 3) ℝ) (z₁ z₂ z₃ : Fin 3 → ℝ) (s : ℝ) :
    triangleFlux c B (s • z₁) (s • z₂) (s • z₃) = s ^ 2 * triangleFlux c (s • B) z₁ z₂ z₃ := by
  simp only [triangleFlux, fieldAt]
  have hc : crossProduct (s • z₂ - s • z₁) (s • z₃ - s • z₁)
      = s ^ 2 • crossProduct (z₂ - z₁) (z₃ - z₁) := by
    rw [← smul_sub, ← smul_sub, LinearMap.map_smul, LinearMap.map_smul₂, smul_smul, sq]
  have hm : B *ᵥ ((1 / 3 : ℝ) • (s • z₁ + s • z₂ + s • z₃)) = (s • B) *ᵥ ((1 / 3 : ℝ) • (z₁ + z₂ + z₃)) := by
    rw [← smul_add, ← smul_add]
    simp only [mulVec_smul, smul_mulVec]
  rw [hc, hm, smul_comm (1 / 2 : ℝ), dotProduct_smul, smul_eq_mul]

end YMGeometry
