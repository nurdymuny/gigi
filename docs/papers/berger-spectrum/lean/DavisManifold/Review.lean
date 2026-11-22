/-
The Davis Manifold (2025), reread line by line: the Lean targets L1 and L3–L8 from
`docs/research/davis-manifold-review-2026-11-22.md`.

What is proved here:
* L1  the Cantelli posterior lower bound is monotone in the two tails (Prop. 3);
* L3  `∏ (1 − Eᵢ) ≥ 1 − ∑ Eᵢ` on `[0,1]`, and the counterexample to the vacuity corollary (Cor. 1);
* L4  the single non-vacuity condition `(1 − ε) κ_soft > (1 + ε) κ_hard` separates a hard-region point from a far
      point in ambient distance, and is equivalent to `κ_soft − κ_hard > ε (κ_soft + κ_hard)`;
* L5  with one reference set the margin gives `d(z₀, z₁) ≥ κ_soft − κ_hard` (the repaired Prop. 4), and with two
      different reference points it does not (a counterexample on ℝ);
* L6  the InfoNCE score margin is a gap in mean *squared* distance, and the counterexample to Lemma 1;
* L7  `log ∑ e^{uₖ} ≥ log K + mean u` (Lemma 1, Step 1; Jensen, not AM–GM);
* L8  `1 − sin x / x < x² / 6` for `x > 0`, so chord-versus-arc distortion is at most `(kL)²/24` (quadratic order).

Nothing here is about the floor itself; these are the 2025 paper's own lemmas.
-/
import Mathlib

namespace DavisManifold

open Real Finset

/-! ## L1. The Cantelli posterior bound is monotone in the tails -/

/-- **L1.** If the false-alarm tail is at most `c₀` and the hit rate is at least `1 − c₁`, the posterior
`π₁ p₁ / (π₀ p₀ + π₁ p₁)` is at least the Cantelli bound `π₁ (1 − c₁) / (π₀ c₀ + π₁ (1 − c₁))`. -/
theorem cantelli_posterior_bound {π₀ π₁ p₀ p₁ c₀ c₁ : ℝ}
    (hπ₀ : 0 ≤ π₀) (hπ₁ : 0 < π₁) (hp₀ : 0 ≤ p₀) (hc₁ : c₁ < 1)
    (h₀ : p₀ ≤ c₀) (h₁ : 1 - c₁ ≤ p₁) :
    π₁ * (1 - c₁) / (π₀ * c₀ + π₁ * (1 - c₁)) ≤ π₁ * p₁ / (π₀ * p₀ + π₁ * p₁) := by
  have hc₀ : 0 ≤ c₀ := hp₀.trans h₀
  have hq : 0 < 1 - c₁ := by linarith
  have hp₁ : 0 < p₁ := hq.trans_le h₁
  have hd₁ : 0 < π₀ * c₀ + π₁ * (1 - c₁) := by positivity
  have hd₂ : 0 < π₀ * p₀ + π₁ * p₁ := by positivity
  rw [div_le_div_iff₀ hd₁ hd₂]
  -- reduces to `(1 − c₁) p₀ ≤ p₁ c₀`, which holds since `1 − c₁ ≤ p₁` and `p₀ ≤ c₀`
  have key : (1 - c₁) * p₀ ≤ p₁ * c₀ :=
    mul_le_mul h₁ h₀ hp₀ hp₁.le
  nlinarith [mul_le_mul_of_nonneg_left key (mul_nonneg hπ₀ hπ₁.le)]

/-! ## L3. The product of survivals beats the union bound -/

/-- **L3.** For error probabilities in `[0,1]`, `∏ (1 − Eᵢ) ≥ 1 − ∑ Eᵢ` (Weierstrass product inequality). So the
multiplicative form of the Davis bound is never below the union-bound form. -/
theorem prod_one_sub_ge {ι : Type*} (s : Finset ι) (E : ι → ℝ)
    (h0 : ∀ i ∈ s, 0 ≤ E i) (h1 : ∀ i ∈ s, E i ≤ 1) :
    1 - ∑ i ∈ s, E i ≤ ∏ i ∈ s, (1 - E i) := by
  classical
  induction s using Finset.induction_on with
  | empty => simp
  | insert a s ha ih =>
    rw [sum_insert ha, prod_insert ha]
    have ih' := ih (fun i hi => h0 i (mem_insert_of_mem hi)) (fun i hi => h1 i (mem_insert_of_mem hi))
    have hEa0 := h0 a (mem_insert_self a s)
    have hEa1 := h1 a (mem_insert_self a s)
    have hP : 0 ≤ ∏ i ∈ s, (1 - E i) :=
      prod_nonneg fun i hi => by linarith [h1 i (mem_insert_of_mem hi)]
    have hsum : 0 ≤ ∑ i ∈ s, E i := sum_nonneg fun i hi => h0 i (mem_insert_of_mem hi)
    nlinarith [mul_le_mul_of_nonneg_left ih' (by linarith : (0:ℝ) ≤ 1 - E a)]

/-- **L3, the counterexample to the vacuity corollary.** Four equal errors of `7/40` sum to `0.7`, yet the
multiplicative factor is `(33/40)⁴ > 0.46`: nowhere near zero. -/
theorem vacuity_counterexample :
    (4 : ℝ) * (7 / 40) = 7 / 10 ∧ (46 : ℝ) / 100 < (1 - 7 / 40) ^ 4 := by
  constructor <;> norm_num

/-! ## L4. The single non-vacuity condition -/

/-- **L4.** Under `(1 ± ε)` distortion, a point of the hard region (geodesic distance `≤ κ_hard` from the reference)
and a far point (geodesic distance `≥ κ_soft`) are separated in *ambient* distance whenever
`(1 − ε) κ_soft > (1 + ε) κ_hard`. -/
theorem ambient_separation {ε κh κs dh df gh gf : ℝ}
    (hε : 0 ≤ ε) (hε1 : ε ≤ 1)
    (hgh : gh ≤ κh) (hgf : κs ≤ gf)
    (hdh : dh ≤ (1 + ε) * gh) (hdf : (1 - ε) * gf ≤ df)
    (hcond : (1 + ε) * κh < (1 - ε) * κs) :
    dh < df := by
  have h1 : (1 + ε) * gh ≤ (1 + ε) * κh := mul_le_mul_of_nonneg_left hgh (by linarith)
  have h2 : (1 - ε) * κs ≤ (1 - ε) * gf := mul_le_mul_of_nonneg_left hgf (by linarith)
  linarith

/-- **L4, the same condition rewritten.** `(1 − ε) κ_soft > (1 + ε) κ_hard ↔ κ_soft − κ_hard > ε (κ_soft + κ_hard)`. -/
theorem nonvacuity_iff (ε κh κs : ℝ) :
    (1 + ε) * κh < (1 - ε) * κs ↔ ε * (κs + κh) < κs - κh := by
  constructor <;> intro h <;> linarith

/-- **L4, in the existence theorem.** With `κ_hard = Δ/4` and `κ_soft = Δ/2` (`Δ > 0`), the non-vacuity condition
is exactly `ε < 1/3`, independent of `Δ`. -/
theorem nonvacuity_quarter_half {Δ ε : ℝ} (hΔ : 0 < Δ) :
    (1 + ε) * (Δ / 4) < (1 - ε) * (Δ / 2) ↔ ε < 1 / 3 := by
  constructor
  · intro h; nlinarith
  · intro h; nlinarith

/-! ## L5. One reference set, not two reference points -/

/-- **L5 (the repaired Prop. 4).** If the benign point is within `κ_hard` of a reference set `R` and the event point
is at least `κ_soft` from the same set, they are at least `κ_soft − κ_hard` apart. -/
theorem margin_gap_single_reference {X : Type*} [PseudoMetricSpace X] (R : Set X) (z₀ z₁ : X)
    {κh κs : ℝ} (h₀ : Metric.infDist z₀ R ≤ κh) (h₁ : κs ≤ Metric.infDist z₁ R) :
    κs - κh ≤ dist z₀ z₁ := by
  have := Metric.infDist_le_infDist_add_dist (x := z₁) (y := z₀) (s := R)
  rw [dist_comm] at this
  linarith

/-- **L5, why the original Prop. 4 fails.** With two *different* reference points `z⁰ = 0` and `z¹ = 10` on `ℝ`, the
benign point `z₀ = 0` is within `κ_hard = 1/2` of `z⁰`, the event point `z₁ = 0` is at least `κ_soft = 1` from `z¹`,
and yet `d(z₀, z₁) = 0 < κ_soft − κ_hard`. -/
theorem margin_gap_two_references_fails :
    dist (0 : ℝ) 0 ≤ 1 / 2 ∧ (1 : ℝ) ≤ dist (0 : ℝ) 10 ∧ dist (0 : ℝ) (0 : ℝ) < 1 - 1 / 2 := by
  simp only [Real.dist_eq]
  norm_num

/-! ## L6. The InfoNCE margin is a gap in mean squared distance -/

/-- The score of the paper: `s(δ) = −δ² / (2τ²)`. -/
noncomputable def score (τ δ : ℝ) : ℝ := -δ ^ 2 / (2 * τ ^ 2)

/-- **L6.** The score difference is a difference of *squared* distances: `s(u) − s(v) = (v² − u²) / (2τ²)`. -/
theorem score_sub (τ u v : ℝ) : score τ u - score τ v = (v ^ 2 - u ^ 2) / (2 * τ ^ 2) := by
  unfold score; ring

/-- **L6, the counterexample to Lemma 1.** Take `τ = 1`, `D = 2` (so `L_eff = D/τ² = 2`), the positive distance
`δ₊ = 1` always, and the negative distance `δ₋ ∈ {0, 2}` with probability ½ each. The score margin is
`m_eff = E s(δ₊) − E s(δ₋) = 1/2 > 0`, but the mean-distance gap is `Δ_cfg = E δ₋ − E δ₊ = 0`, which violates the
claimed `Δ_cfg ≥ m_eff / L_eff = 1/4`. -/
theorem lemma1_counterexample :
    let mEff : ℝ := score 1 1 - ((1/2) * score 1 0 + (1/2) * score 1 2)
    let Δcfg : ℝ := ((1/2) * 0 + (1/2) * 2) - 1
    let Leff : ℝ := 2 / 1 ^ 2
    mEff = 1 / 2 ∧ Δcfg = 0 ∧ Δcfg < mEff / Leff := by
  simp only [score]
  norm_num

/-! ## L7. log-sum-exp is at least log K plus the mean (Jensen) -/

/-- **L7.** For `K ≥ 1` values `u`, `log ∑ e^{uₖ} ≥ log K + (∑ uₖ)/K`. This is Jensen's inequality for `exp`; the
2025 paper calls it AM–GM. -/
theorem log_sum_exp_ge {K : ℕ} (hK : 0 < K) (u : Fin K → ℝ) :
    Real.log K + (∑ k, u k) / K ≤ Real.log (∑ k, Real.exp (u k)) := by
  have hKr : (0 : ℝ) < K := by exact_mod_cast hK
  -- Jensen with uniform weights 1/K
  have hJ := (convexOn_exp).map_sum_le (t := (Finset.univ : Finset (Fin K)))
    (w := fun _ => (1 : ℝ) / K) (p := u)
    (fun _ _ => by positivity)
    (by simp [Finset.card_univ]; field_simp)
    (fun _ _ => Set.mem_univ _)
  simp only [smul_eq_mul] at hJ
  rw [← Finset.mul_sum, ← Finset.mul_sum] at hJ
  have hS : 0 < ∑ k, Real.exp (u k) :=
    Finset.sum_pos (fun k _ => Real.exp_pos _) ⟨⟨0, hK⟩, Finset.mem_univ _⟩
  have h2 : (1 / K) * ∑ k, u k ≤ Real.log ((1 / K) * ∑ k, Real.exp (u k)) := by
    rw [← Real.exp_le_exp, Real.exp_log (by positivity)]
    exact hJ
  rw [Real.log_mul (by positivity) hS.ne', one_div, Real.log_inv] at h2
  have : (∑ k, u k) / K = (1 / K) * ∑ k, u k := by ring
  rw [this, one_div]
  linarith

/-! ## L8. Chord against arc is quadratic in curvature times length -/

/-- **L8.** For `x > 0`, `1 − sin x / x < x² / 6`. -/
theorem one_sub_sin_div_lt {x : ℝ} (hx : 0 < x) : 1 - Real.sin x / x < x ^ 2 / 6 := by
  have h := Real.sin_gt_sub_cube hx
  have h2 : 1 - x ^ 2 / 6 < Real.sin x / x := by
    rw [lt_div_iff₀ hx]; nlinarith [h]
  linarith

/-- **L8, chord against arc.** On an arc of curvature `k > 0` and length `L > 0`, the chord is `2 sin(kL/2)/k`, and
`1 − chord/arc < (kL)² / 24`. So the 2025 paper's linear `ε(L) = C″KL` is an upper bound but not the true order. -/
theorem chord_arc_distortion {k L : ℝ} (hk : 0 < k) (hL : 0 < L) :
    1 - (2 * Real.sin (k * L / 2) / k) / L < (k * L) ^ 2 / 24 := by
  have hx : 0 < k * L / 2 := by positivity
  have h := one_sub_sin_div_lt hx
  have e : (2 * Real.sin (k * L / 2) / k) / L = Real.sin (k * L / 2) / (k * L / 2) := by
    field_simp
  rw [e]
  calc 1 - Real.sin (k * L / 2) / (k * L / 2) < (k * L / 2) ^ 2 / 6 := h
    _ = (k * L) ^ 2 / 24 := by ring

end DavisManifold
