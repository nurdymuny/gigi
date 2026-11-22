/-
The Lift Unification (March 2026, Zenodo 19344768), reread: Lean targets from
`docs/research/lift-unification-review-2026-11-22.md`.

* the mammogram: Lift(Sick, Pos) = 25/3, Lift(Healthy, Pos) = 25/27, normalized covariance 22/3;
* the dice couplings: Cov(A, C) = −1/72, Lift 2/3; Cov(A, D) = 0;
* the exact capacity law: with prior odds o = e^{−K} and likelihood ratio r = e^{τ}, posterior odds = o·r = e^{τ−K},
  so the posterior passes ½ exactly when τ > K (C = τ/K > 1);
* Mossel: P(all even) = 1/4, P(k | all even) = 2·(1/3)^k sums to 1, E[k | all even] = 3/2, and the Lifts 4, 8/5, 16/25;
* the corrected explanatory step: P(all even | length k) = (2/5)^(k−1), so it is 1 (not ½) at k = 1.
-/
import Mathlib

namespace Lift

/-- The mammogram (1000 women, base rate 1%, sensitivity 90%, false-positive rate 10%). -/
theorem mammogram :
    (9 : ℚ) / 10 / (108 / 1000) = 25 / 3 ∧
    (1 : ℚ) / 10 / (108 / 1000) = 25 / 27 ∧
    ((9 : ℚ) / 1000 - 1 / 100 * (108 / 1000)) / (1 / 100 * (108 / 1000)) = 22 / 3 ∧
    (25 : ℚ) / 3 * (1 / 100) = 1 / 12 := by
  norm_num

/-- The dice couplings on {1,…,6}²: `A` = first die odd, `C` = difference 3, `D` = difference 4. -/
theorem dice_couplings :
    (1 : ℚ) / 36 - 1 / 2 * (3 / 36) = -1 / 72 ∧
    ((1 : ℚ) / 36) / (1 / 2 * (3 / 36)) = 2 / 3 ∧
    (1 : ℚ) / 36 - 1 / 2 * (2 / 36) = 0 := by
  norm_num

/-- **The exact capacity law.** With prior odds `e^{−K}` (K > 0) and likelihood ratio `e^{τ}`, the posterior odds are
`e^{τ−K}`, which exceed `1` (posterior above ½) exactly when `τ > K`, i.e. when `C = τ/K > 1`. -/
theorem capacity_law {τ K : ℝ} (hK : 0 < K) :
    1 < Real.exp (-K) * Real.exp τ ↔ 1 < τ / K := by
  rw [← Real.exp_add, Real.one_lt_exp_iff, one_lt_div hK]
  constructor <;> intro h <;> linarith

/-- At `C = τ/K = 1` the posterior odds are exactly `1`: the point of maximum uncertainty, not the benign case. -/
theorem capacity_one_is_fifty_fifty (K : ℝ) : Real.exp (-K) * Real.exp K = 1 := by
  rw [← Real.exp_add]; simp

/-- Mossel's paradox: `P(all even) = 1/4`. -/
theorem mossel_all_even : ∑' k : ℕ, (2 : ℝ) ^ k / 6 ^ (k + 1) = 1 / 4 := by
  have h : ∀ k : ℕ, (2 : ℝ) ^ k / 6 ^ (k + 1) = (1 / 6) * (1 / 3) ^ k := by
    intro k
    have h6 : (6 : ℝ) ^ k = 2 ^ k * 3 ^ k := by rw [← mul_pow]; norm_num
    rw [div_pow, one_pow, pow_succ, h6]
    field_simp
  simp_rw [h]
  rw [tsum_mul_left, tsum_geometric_of_lt_one (by norm_num) (by norm_num)]
  norm_num

/-- Mossel's paradox: the conditional law `P(k | all even) = 2·(1/3)^k` for `k ≥ 1` sums to one. -/
theorem mossel_conditional_sums_to_one : ∑' k : ℕ, 2 * (1 / 3 : ℝ) ^ (k + 1) = 1 := by
  have h : ∀ k : ℕ, 2 * (1 / 3 : ℝ) ^ (k + 1) = (2 / 3) * (1 / 3) ^ k := by intro k; ring
  simp_rw [h]
  rw [tsum_mul_left, tsum_geometric_of_lt_one (by norm_num) (by norm_num)]
  norm_num

/-- Mossel's paradox: `E[k | all even] = 3/2`, not `3`. -/
theorem mossel_expectation : ∑' k : ℕ, ((k : ℝ) + 1) * (2 * (1 / 3 : ℝ) ^ (k + 1)) = 3 / 2 := by
  have h : ∀ k : ℕ, ((k : ℝ) + 1) * (2 * (1 / 3 : ℝ) ^ (k + 1)) =
      2 * ((k : ℝ) * (1 / 3) ^ (k + 1)) + 2 * (1 / 3 : ℝ) ^ (k + 1) := by intro k; ring
  have hs1 : Summable (fun k : ℕ => (k : ℝ) * (1 / 3 : ℝ) ^ k) :=
    (summable_pow_mul_geometric_of_norm_lt_one 1 (r := (1 / 3 : ℝ)) (by norm_num)).congr (fun k => by simp)
  have hA : ∑' k : ℕ, (k : ℝ) * (1 / 3 : ℝ) ^ (k + 1) = 1 / 4 := by
    have e : ∀ k : ℕ, (k : ℝ) * (1 / 3 : ℝ) ^ (k + 1) = (1 / 3) * ((k : ℝ) * (1 / 3) ^ k) := by
      intro k; ring
    simp_rw [e]
    rw [tsum_mul_left, tsum_coe_mul_geometric_of_norm_lt_one (by norm_num)]
    norm_num
  have hB : ∑' k : ℕ, 2 * (1 / 3 : ℝ) ^ (k + 1) = 1 := mossel_conditional_sums_to_one
  have hs2 : Summable (fun k : ℕ => 2 * (1 / 3 : ℝ) ^ (k + 1)) := by
    have := (summable_geometric_of_lt_one (r := (1 / 3 : ℝ)) (by norm_num) (by norm_num)).mul_left (2 / 3)
    exact this.congr (by intro k; ring)
  have hs3 : Summable (fun k : ℕ => (k : ℝ) * (1 / 3 : ℝ) ^ (k + 1)) :=
    (hs1.mul_left (1 / 3)).congr (by intro k; ring)
  simp_rw [h]
  rw [(hs3.mul_left 2).tsum_add hs2, tsum_mul_left, hA, hB]
  norm_num

/-- Mossel's Lifts for lengths 1, 2, 3: `4`, `8/5`, `16/25`. -/
theorem mossel_lifts :
    ((2 : ℚ) / 3) / (1 / 6) = 4 ∧ ((2 : ℚ) / 9) / (5 / 36) = 8 / 5 ∧ ((2 : ℚ) / 27) / (25 / 216) = 16 / 25 := by
  norm_num

/-- **The corrected step.** Given that the first `6` comes at roll `k`, each earlier roll is uniform on `{1,…,5}`, so
`P(all even | length k) = (2/5)^(k−1)`. At `k = 1` this is `1`, not `½` as the paper says, and the decay ratio is
`2/5`, not `½`. Consistency check: `P(length k)·P(all even | length k) = (5/6)^(k−1)(1/6)(2/5)^(k−1) = 2^(k−1)/6^k`. -/
theorem all_even_given_length (k : ℕ) :
    ((5 : ℚ) / 6) ^ k * (1 / 6) * (2 / 5) ^ k = 2 ^ k / 6 ^ (k + 1) := by
  have h53 : (5 / 6 : ℚ) * (2 / 5) = 1 / 3 := by norm_num
  have h6 : (6 : ℚ) ^ k = 2 ^ k * 3 ^ k := by rw [← mul_pow]; norm_num
  rw [mul_right_comm, ← mul_pow, h53, div_pow, one_pow, pow_succ, h6]
  field_simp

end Lift
