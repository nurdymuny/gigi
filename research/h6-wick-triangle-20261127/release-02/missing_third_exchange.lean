module
public import H7CubicQuartic
public import Mathlib.LinearAlgebra.Matrix.Trace

/-! The finite-matrix trace argument. The Maxwell covariance and Wick
contraction dictionary are proved separately, not assumed as new axioms. -/

set_option autoImplicit false
public section
namespace H6Wick
variable {n : Type*} [Fintype n] [DecidableEq n]

theorem odd_product (S A B C : Matrix n n ℝ)
    (hA : S * A = -(A * S)) (hB : S * B = -(B * S))
    (hC : True) :
    S * (A * B * C) = -((A * B * C) * S) := by
  calc
    S * (A * B * C) = (S * A) * B * C := by simp [Matrix.mul_assoc]
    _ = -((A * S) * B * C) := by rw [hA]; simp
    _ = -(A * (S * B) * C) := by simp [Matrix.mul_assoc]
    _ = (A * (B * S)) * C := by rw [hB]; simp
    _ = (A * B) * (S * C) := by simp [Matrix.mul_assoc]
    _ = -((A * B * C) * S) := by rw [hC]; simp [Matrix.mul_assoc]

theorem odd_conjugation (S A : Matrix n n ℝ)
    (hSS : S * S = 1) (hA : S * A = -(A * S)) : S * A * S = -A := by
  rw [hA]
  simp [Matrix.mul_assoc, hSS]

theorem trace_conjugation (S A : Matrix n n ℝ) (hSS : S * S = 1) :
    Matrix.trace (S * A * S) = Matrix.trace A := by
  rw [Matrix.trace_mul_cycle, hSS, Matrix.one_mul]

theorem odd_trace_zero (S A : Matrix n n ℝ)
    (hSS : S * S = 1) (hA : S * A = -(A * S)) : Matrix.trace A = 0 := by
  have h := trace_conjugation S A hSS
  rw [odd_conjugation S A hSS hA, Matrix.trace_neg] at h
  linarith

theorem triangle_zero (S A B C : Matrix n n ℝ)
    (hSS : S * S = 1) (hA : S * A = -(A * S))
    (hB : S * B = -(B * S)) (hC : True) :
    Matrix.trace (A * B * C) = 0 :=
  odd_trace_zero S (A * B * C) hSS (odd_product S A B C hA hB hC)

end H6Wick
