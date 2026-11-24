/-
YANG–MILLS REQUIREMENTS AS GEOMETRIC OBJECTS, Section 5: reflection positivity has a specific geometric role.

On a lattice with a symmetric transfer matrix T, a preparation x at Euclidean time site t ≥ 1 propagates as T^t x.
Where the reflection plane sits decides what the test can see:

* **Through a site** (t ↦ −t): the reflected pairing ⟨x_i, T^(t_i + t_j) x_j⟩ is always the Gram matrix of the
  vectors T^(t_i) x_i. It is nonnegative whatever T is, so in this already-constructed positive Hilbert space it
  cannot detect the sign of T (`site_reflection_always_positive`). This is not a claim that site reflection
  positivity is vacuous as a condition on Euclidean correlations in general.
* **Through a link** (t ↦ 1 − t): the pairing ⟨x_i, T^(t_i + t_j − 1) x_j⟩ = ⟨T^(t_i − 1) x_i, T · T^(t_j − 1) x_j⟩
  has one central, unsquared T (the exponent itself need not be odd). It is nonnegative when T = SᵀS is a positive
  operator (`link_reflection_positive`), and a negative eigenvalue of T that the preparations reach makes it fail
  (`link_reflection_detects_negative`).

So the lattice form of the note's test is positivity of the transfer matrix, read through link reflection. The
numerical companion is T5 of `scripts/ym_geometry_tests.py` on the Kogut–Susskind SU(2) plaquette.
-/
import Mathlib

namespace YMGeometry

open Matrix Finset

variable {m : Type*} [Fintype m] [DecidableEq m] {ι : Type*} [Fintype ι]

/-- For a symmetric `T`, powers split across the dot product. -/
lemma dot_pow_split (T : Matrix m m ℝ) (hT : Tᵀ = T) (a b : ℕ) (x y : m → ℝ) :
    x ⬝ᵥ ((T ^ (a + b)) *ᵥ y) = ((T ^ a) *ᵥ x) ⬝ᵥ ((T ^ b) *ᵥ y) := by
  rw [pow_add, ← mulVec_mulVec, dotProduct_mulVec, ← mulVec_transpose, transpose_pow, hT]

omit [DecidableEq m] in
/-- A positive operator `SᵀS` moves across the dot product as `S` on both sides. -/
lemma gram_factor (S : Matrix m m ℝ) (w z : m → ℝ) : ((Sᵀ * S) *ᵥ w) ⬝ᵥ z = (S *ᵥ w) ⬝ᵥ (S *ᵥ z) := by
  rw [← mulVec_mulVec, mulVec_transpose, ← dotProduct_mulVec]

omit [DecidableEq m] in
/-- A double sum of pairings of fixed vectors is the squared length of their combination. -/
lemma double_sum_dot (v : ι → ℝ) (y : ι → m → ℝ) :
    ∑ i, ∑ j, v i * v j * (y i ⬝ᵥ y j) = (∑ i, v i • y i) ⬝ᵥ (∑ j, v j • y j) := by
  rw [sum_dotProduct]
  refine sum_congr rfl fun i _ => ?_
  rw [dotProduct_sum]
  refine sum_congr rfl fun j _ => ?_
  rw [smul_dotProduct, dotProduct_smul, smul_eq_mul, smul_eq_mul]
  ring

omit [DecidableEq m] in
lemma dot_self_nonneg (z : m → ℝ) : 0 ≤ z ⬝ᵥ z := by
  rw [dotProduct]; exact sum_nonneg fun i _ => mul_self_nonneg (z i)

/-- **Site reflection tests nothing:** for symmetric `T` the pairing is a Gram matrix, whatever `T`'s sign. -/
theorem site_reflection_always_positive (T : Matrix m m ℝ) (hT : Tᵀ = T) (x : ι → m → ℝ) (t : ι → ℕ)
    (v : ι → ℝ) : 0 ≤ ∑ i, ∑ j, v i * v j * (x i ⬝ᵥ ((T ^ (t i + t j)) *ᵥ x j)) := by
  simp_rw [dot_pow_split T hT]
  rw [double_sum_dot v (fun i => (T ^ t i) *ᵥ x i)]
  exact dot_self_nonneg _

/-- **Link reflection is positive for a positive transfer matrix** `T = SᵀS` (symmetric): the odd power
`T^(a + 1 + b)` splits as `(S Tᵃ x) · (S Tᵇ y)`. -/
theorem link_reflection_positive (S : Matrix m m ℝ) (T : Matrix m m ℝ) (hT : Tᵀ = T) (hS : T = Sᵀ * S)
    (x : ι → m → ℝ) (t : ι → ℕ) (v : ι → ℝ) :
    0 ≤ ∑ i, ∑ j, v i * v j * (x i ⬝ᵥ ((T ^ (t i + 1 + t j)) *ᵥ x j)) := by
  have split : ∀ i j, x i ⬝ᵥ ((T ^ (t i + 1 + t j)) *ᵥ x j)
      = (S *ᵥ ((T ^ t i) *ᵥ x i)) ⬝ᵥ (S *ᵥ ((T ^ t j) *ᵥ x j)) := by
    intro i j
    rw [dot_pow_split T hT (t i + 1) (t j), pow_succ', ← mulVec_mulVec, hS, gram_factor]
  simp_rw [split]
  rw [double_sum_dot v (fun i => S *ᵥ ((T ^ t i) *ᵥ x i))]
  exact dot_self_nonneg _

/-- **Link reflection detects a negative transfer direction:** if `T u = −μ u` with `μ > 0` and `u ≠ 0`, the single
preparation `u` at the first site (one link) has negative reflected norm. -/
theorem link_reflection_detects_negative (T : Matrix m m ℝ) {u : m → ℝ} {μ : ℝ} (hμ : 0 < μ) (hu : u ≠ 0)
    (hTu : T *ᵥ u = -μ • u) : u ⬝ᵥ ((T ^ (0 + 1 + 0)) *ᵥ u) < 0 := by
  simp only [zero_add, add_zero, pow_one, hTu, dotProduct_smul, smul_eq_mul]
  have : 0 < u ⬝ᵥ u := by
    obtain ⟨k, hk⟩ := Function.ne_iff.mp hu
    rw [dotProduct]
    exact lt_of_lt_of_le (mul_self_pos.mpr hk)
      (single_le_sum (f := fun i => u i * u i) (fun i _ => mul_self_nonneg (u i)) (mem_univ k))
  nlinarith

end YMGeometry
