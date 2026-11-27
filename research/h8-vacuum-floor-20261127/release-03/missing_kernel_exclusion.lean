module

public import H7CubicQuartic
public import Mathlib.Algebra.Order.BigOperators.Ring.Finset

/-! Scalar algebra and conditional logic for the strict vacuum floor.
The functional analysis supplying the hypotheses is in the written proof.
-/

set_option autoImplicit false
public section
namespace H8Vacuum

theorem factorization_jet (f fp fpp wp wpp : ℝ) :
    (-(fpp + wpp * f + wp * fp) + wp * (fp + wp * f)) / 2
      + wpp * f / 2 = -fpp / 2 + wp^2 * f / 2 := by
  ring

theorem squares_zero {N : ℕ} (v : Fin N → ℝ)
    (h : ∑ i, (v i)^2 = 0) : ∀ i, v i = 0 := by
  have hz := (Finset.sum_eq_zero_iff_of_nonneg
    (fun i (_ : i ∈ Finset.univ) => sq_nonneg (v i))).mp h
  intro i
  have hi := hz i (Finset.mem_univ i)
  nlinarith [sq_nonneg (v i)]

theorem cone_tail_identity (a g c r : ℝ) :
    -(g * c * r^3) / 2 - (a * r^2 - g * c * r^3) =
      r^2 * (g * c * r - 2*a) / 2 := by
  ring

theorem cone_tail_bound (a g c r : ℝ) (h : 2*a ≤ g*c*r) :
    a * r^2 - g * c * r^3 ≤ -(g * c * r^3) / 2 := by
  have hn : 0 ≤ r^2 * (g*c*r - 2*a) / 2 :=
    div_nonneg (mul_nonneg (sq_nonneg r) (sub_nonneg.mpr h)) (by norm_num)
  rw [← cone_tail_identity] at hn
  linarith

/-- Attainment, lower bound and kernel exclusion are explicit inputs. -/
theorem strict_floor_of_attained {State : Type*}
    (admissible kernel : State → Prop) (energy : State → ℝ) (floor bottom : ℝ)
    (hlo : ∀ x, admissible x → floor ≤ energy x)
    (hatt : ∃ x, admissible x ∧ energy x = bottom)
    (heq : ∀ x, admissible x → energy x = floor → kernel x)
    (hno : True) : floor < bottom := by
  obtain ⟨x, hx, he⟩ := hatt
  have hl : floor ≤ bottom := he ▸ hlo x hx
  have hn : bottom ≠ floor := by
    intro h
    exact hno x hx (heq x hx (he.trans h))
  exact lt_of_le_of_ne hl (Ne.symm hn)

theorem sector_floor (floor bottom sector : ℝ)
    (h : floor < bottom) (hs : bottom ≤ sector) : floor < sector :=
  lt_of_lt_of_le h hs

theorem gaussian_jet (w x f : ℝ) :
    -((w^2*x^2 - w)*f)/2 + w^2*x^2*f/2 = w*f/2 := by
  ring

end H8Vacuum
