/-
YANG–MILLS REQUIREMENTS AS GEOMETRIC OBJECTS, Section 7 (counterexamples) and Section 9 (limits).

The note's second counterexample: energies 0, 1, 1/2, 1/3, … . Every finite truncation has a positive gap, yet no
gap survives the limit, because the fixed-energy sublevel radius grows like √N. This is the failure a lattice
programme must rule out: "every truncation is gapped" proves nothing about the limit.

Truncation N lives on `Fin (N+1)`: coordinate 0 is the vacuum, coordinate k ≥ 1 has energy 1/k.
-/
import Mathlib

namespace YMGeometry

open Finset

/-- The energies of the countermodel: 0 on the vacuum, 1/k on the k-th excitation. -/
noncomputable def lam (k : ℕ) : ℝ := if k = 0 then 0 else 1 / k

/-- The energy form of truncation N. -/
noncomputable def qTrunc (N : ℕ) (x : Fin (N + 1) → ℝ) : ℝ := ∑ k : Fin (N + 1), lam (k : ℕ) * x k ^ 2

/-- **Every truncation is gapped:** off the vacuum, the energy is at least (1/N)·‖x‖². -/
theorem trunc_gap (N : ℕ) (x : Fin (N + 1) → ℝ) (hx : x 0 = 0) :
    (1 / (N : ℝ)) * ∑ k, x k ^ 2 ≤ qTrunc N x := by
  rw [qTrunc, mul_sum]
  refine sum_le_sum fun k _ => ?_
  by_cases hk : (k : ℕ) = 0
  · have : k = 0 := Fin.ext hk
    subst this; simp [hx]
  · apply mul_le_mul_of_nonneg_right _ (sq_nonneg _)
    simp only [lam, hk, ite_false]
    have hk1 : (1 : ℝ) ≤ (k : ℕ) := by exact_mod_cast Nat.one_le_iff_ne_zero.mpr hk
    have hkN : ((k : ℕ) : ℝ) ≤ N := by exact_mod_cast Nat.lt_succ_iff.mp k.isLt
    exact one_div_le_one_div_of_le (by linarith) hkN

/-- **But the limit is gapless:** in truncation N the vector √N·e_N has energy exactly 1 and squared norm N. -/
theorem trunc_escape (N : ℕ) (hN : 0 < N) :
    qTrunc N (Pi.single (Fin.last N) (Real.sqrt N)) = 1 ∧
      ∑ k, (Pi.single (Fin.last N) (Real.sqrt N) : Fin (N + 1) → ℝ) k ^ 2 = N := by
  have hsq : Real.sqrt N ^ 2 = N := Real.sq_sqrt (Nat.cast_nonneg N)
  constructor
  · rw [qTrunc, sum_eq_single (Fin.last N)]
    · simp only [Pi.single_eq_same, Fin.val_last, lam, hN.ne', ite_false, hsq]
      field_simp
    · intro b _ hb; simp [Pi.single_eq_of_ne hb]
    · simp
  · rw [sum_eq_single (Fin.last N)]
    · simp [hsq]
    · intro b _ hb; simp [Pi.single_eq_of_ne hb]
    · simp

/-- **No uniform bound:** for every radius R some truncation has a unit-energy, vacuum-orthogonal vector of
squared norm above R. -/
theorem no_uniform_gap (R : ℝ) : ∃ N : ℕ, 0 < N ∧ ∃ x : Fin (N + 1) → ℝ,
    x 0 = 0 ∧ qTrunc N x = 1 ∧ R < ∑ k, x k ^ 2 := by
  obtain ⟨N, hN⟩ := exists_nat_gt (max R 1)
  have hN0 : 0 < N := by
    have : (1 : ℝ) < N := lt_of_le_of_lt (le_max_right _ _) hN
    exact_mod_cast (show (0 : ℝ) < N by linarith)
  obtain ⟨h1, h2⟩ := trunc_escape N hN0
  refine ⟨N, hN0, _, ?_, h1, ?_⟩
  · rw [Pi.single_eq_of_ne]
    intro h
    have := congrArg Fin.val h
    simp at this
    omega
  · rw [h2]; exact lt_of_le_of_lt (le_max_left _ _) hN

end YMGeometry
