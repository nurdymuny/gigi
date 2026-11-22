# The Davis Manifold, reread line by line: what holds, what breaks, and what the revision needs

*22 November 2026. A research note, not a result. This is the first step toward a revised edition of my 2025 paper.*

**The paper.** *The Davis Manifold: Geometry-First Detection with Compositional Error Budgets* (2025, 61 pp).
- Source: `davis-wilson-lattice/docs/theory/the_davis_manifold.tex`, 3,500 lines. The copy in `curvature aware gpu/` is
  byte-identical. Neither copy is committed to any repository.
- PDF: `Downloads/DAVIS_Manifold__Geometry_First_Detection_with_Compositional_Error_Budgets-2.pdf`.

**How it relates to this morning's note.** *The phone in the house* (§4) mapped the paper's objects onto the floor
certificate and tagged each bridge. This note does the other half: it checks the 2025 paper's own definitions,
lemmas and theorems as mathematics, line by line. The floor gives the paper its first exact specimen. Before it can
carry that specimen, its own proofs have to stand.

**Receipts.** The numerical checks are `davis-manifold-2026-11-22/checks.py`, with output in `checks_output.txt` in the
same folder. Everything else here is algebra, done by hand. The items marked *Lean target* are to be machine-checked
next.

## 1. What holds as written

- **The Cantelli step, Prop. 3 / App. A.3. Correct.**
  - Cantelli bounds both tails at the midpoint.
  - The posterior bound follows because π₁p₁/(π₀p₀ + π₁p₁) increases in p₁ and decreases in p₀. *Lean target L1.*
- **InfoNCE controls the score margin, Lemma 1 Step 1 / A.1. Correct.**
  - log(1 + Σₖ e^{Δₖ}) ≥ log K + (1/K) Σₖ Δₖ gives m_eff ≥ log K − L_NCE.
  - The paper calls this AM–GM. It is Jensen's inequality for the convex function log-sum-exp. *Lean target L7.*
- **The independence slack, Def. in §4.3. Correct.**
  - As defined, P(G) = Π(1 − Eᵢ) − δ_indep holds with equality, not just as a lower bound. That is because
    1 − Π(1 − Eᵢ) is never negative, so the (·)₊ never bites.
- **The union-bound fallback, App. A.4. Correct.**
- **Theorem 1, existence. Correct as a construction.**
  - κ_hard = Δ/4 and κ_soft = Δ/2 are definitions. The proof then picks λ so that ε(L*) < κ_soft / 2R.
  - It inherits whatever Lemma 1 and Prop. 2 actually deliver (§2 below).
- **The toy horizon example, §5.3. Correct.**
  - The surrogate peaks at L* = 6.03.
  - The four printed values are 0.165, 0.208, 0.162 and 0.054. The paper prints 0.16, 0.21, 0.16 and 0.054.

## 2. What breaks, and the repair for each

### 2.1 Lemma 1's conclusion does not follow: a margin in score is not a gap in mean distance

The lemma claims Δ_cfg ≥ m_eff / L_eff, where:
- Δ_cfg is the gap in *mean* distance between different-configuration and same-configuration pairs;
- L_eff = D/τ² is the Lipschitz constant of the score.

**Why it fails.** The score is s(δ) = −δ²/(2τ²), so the margin is exactly

  m_eff = (E[δ₋²] − E[δ₊²]) / (2τ²).

That is a gap in mean *squared* distance. A gap in second moments can exist with no gap in means.

**Counterexample** (in `checks.py`). Take τ = 1, D = 2, δ₊ ≡ 1, and δ₋ equal to 0 or 2 with probability ½ each.
Then m_eff = 0.5 and Δ_cfg = 0, while the lemma's bound requires Δ_cfg ≥ 0.25.

**Where the proof breaks.** Step 3 applies |s(u) − s(v)| ≤ L|u − v| but needs the one-sided inequality
s(u) − s(v) ≤ L(v − u). That inequality fails whenever v < u.

**Repair.** State the lemma in the quantity it actually controls:

  Δ⁽²⁾_cfg := E[δ₋²] − E[δ₊²] = 2τ² m_eff ≥ 2τ² (log K − L̂_NCE − slack).

Then define the margins on root-mean-square distances. Alternatively, keep Δ_cfg and add an explicit assumption
that delivers it, for example a coupling in which δ₋ ≥ δ₊ almost surely. *Lean target L6:* the identity and the
counterexample, both exact rational arithmetic.

### 2.2 Prop. 4 (margin ⇒ statistic separation): the two reference points give no lower bound

**What the proposition says.** Hypothesis (a) says:
- h = 0 implies d(z, z⁰) ≤ κ_hard;
- h = 1 implies d(z, z¹) ≥ κ_soft.

The proof then asserts d(z₀, z₁) ≥ κ_soft − κ_hard.

**Why it fails.** With two *different* reference points, the triangle inequality gives nothing. A point far from z¹
can sit right next to a point near z⁰.

**Repair.** Use one reference set R₀ for both conditions:
- h = 0 implies d(z, R₀) ≤ κ_hard;
- h = 1 implies d(z, R₀) ≥ κ_soft.

Then d(z₀, z₁) ≥ d(z₁, R₀) − d(z₀, R₀) ≥ κ_soft − κ_hard. This also matches Definition 3, which measures both margins
from the same reference sets. *Lean target L5.*

### 2.3 Prop. 2 (smoothness ⇒ distortion): one hypothesis is empty, one conclusion is trivial, and the order is wrong

- **Hypothesis C2 is empty.** It bounds ‖∇_γ̇ g‖. For the Levi-Civita connection this is identically zero, because
  the metric is parallel, so C2 constrains nothing.
- **The first conclusion is trivial.** It bounds |ℓ_g(γ) − ℓ_δ(γ)| ≤ C′K L². For a pullback metric g = JᵀJ, the
  length of any path under g *equals* its ambient Euclidean length, so this is identically zero.
- **The real quantity is chord against geodesic,** and the geometry that governs it is the *extrinsic* curvature of
  the embedded image (its second fundamental form), not the intrinsic metric.
- **The order is wrong.** For an arc of curvature k and length L,

    1 − chord/arc = 1 − sin(kL/2)/(kL/2) = (kL)²/24 + O((kL)⁴).

  `checks.py` matches (kL)²/24 to four digits at kL = 0.1 and 0.3. So the distortion is **quadratic** in curvature
  times length. The paper's linear form ε(L) = C″KL is a valid upper bound once kL ≤ 1, but it is not the true
  order.

**Repair.** Restate Prop. 2 with a bound k on the second fundamental form along the path family, so that
ε(L) ≤ (kL)²/24 + higher-order terms. Keep the linear bound as a coarser corollary.

**Why this repair matters beyond tidiness.** The quadratic order is the order of my 2026 Davis Duality:
flat-approximation error is sandwiched as c₁‖Ω‖h² ≤ error ≤ c₂‖Ω‖h². With the repair, the 2025 seed and the 2026
theorem agree on the exponent. Without it they disagree, and *The phone in the house* §4.5 draws its line from
2025 to the floor through a linear law that is not the true order.
- Tag: RESONANCE → this makes it consistent. It does not make it a derivation.
- *Lean target L8:* 1 − sin x / x ≤ x²/6 for 0 < x ≤ 1, which gives the quadratic upper bound.

### 2.4 Three different "non-vacuity" conditions

| where | condition |
| --- | --- |
| Def. 6, A5, Thm 1, all of §6 | κ_soft − 2Rε(L*) > 0 |
| Prop. 4 and its proof | κ_soft > κ_hard |
| Thm 2 | κ_soft − κ_hard − ε* L* > 0 |

These are not equivalent.
- The second is enough for the pairwise gap in Prop. 4, since (1 − ε)(κ_soft − κ_hard) > 0 needs only that and
  ε < 1.
- None of the three is the condition a *detector* needs. A detector classifies a single point by its ambient
  distance to the reference.

**What the detector needs.** Under bounded distortion:
- a hard-region point is at ambient distance at most (1 + ε)κ_hard from the reference;
- a far point is at ambient distance at least (1 − ε)κ_soft.

The two are separable in ambient distance exactly when

  (1 − ε) κ_soft > (1 + ε) κ_hard, equivalently κ_soft − κ_hard > ε (κ_soft + κ_hard).

**Repair.** Make this the single non-vacuity condition, everywhere. *Lean target L4.*

### 2.5 Theorem 2 (the main Davis bound): one step needs an extra assumption

**The gap.** The proof passes from P(Y=1 | S > s*, G) to P(Y=1 | H, G) using H ⊆ {S > s*}. Conditioning on a
*subset* can lower a probability, so this step is not justified as written.

**Repair, either of:**
- define H = {S > s*} ∩ {no abstention} exactly;
- or assume a monotone likelihood ratio in S, so that raising the threshold cannot lower the posterior.

**A related point.** The Cantelli moments used under G must be the moments *conditional on G*, and those are the
ones to estimate. The paper estimates the unconditional moments.

### 2.6 The vacuity corollary is false as stated

**The claim.** If E_sum > τ_vac ≈ 0.7, then the lower bound is ≈ 0 "for any reasonable δ_indep".

**Counterexample** (in `checks.py`). Four equal errors summing to just over 0.7 give a product
(1 − 0.175)⁴ = 0.463. The union bound gives 0.30. Neither is ≈ 0.

The reason is general: Π(1 − Eᵢ) ≥ 1 − ΣEᵢ always holds (*Lean target L3*), so the multiplicative form never vacates
before the union form does.

**Repair.** Define vacuity by the bound itself: declare it vacuous when the right-hand side of (13) falls below a
pre-registered floor. Keep E_sum as a monitoring trigger, not as the definition.

### 2.7 Small things

- **The "AM–GM" label** in Lemma 1. It is Jensen's inequality.
- **Appendix A.5, Step 3:** "back off slightly from the limiting value of λ" should say "increase λ further". This
  also needs K(λ) to *strictly* decrease toward 0, which is stated nowhere.
- **Lemma 1's "O(ρ)" and "with high probability"** carry no explicit constants, even though Appendix A promises
  them.
- **Every number in the HERALD/VIDAR tables and in the VIDAR worked example is illustrative**, as the paper says. A
  revision should keep that label on each one.

## 3. Why these repairs come first

The revision's new section gives the framework its first exact instance: the floor certificate as a fully
certified Davis system. *The phone in the house* §4.3 has the mapping. In that instance:
- the margin is her rival's price minus hers;
- the enclosure is the width of each interval box;
- abstention happens exactly where a box straddles the edge;
- the budget is 1,640,935 interval boxes below family 80, plus Gershgorin at family 80 and above.

**It is the degenerate case of Theorem 2.** Every error term is zero by proof:
- E_geom and E_link vanish under interval arithmetic;
- ζ = 0, because abstention *is* the set of undecided boxes.

So the bound reduces to the certificate itself. That is clean, but it only holds if the deterministic skeleton of
the 2025 paper is sound: §2.2's separation step, §2.4's non-vacuity condition, and §2.5's subset step. Those three
repairs are the load-bearing ones for the new section. §2.1 and §2.3 are load-bearing for the paper's own ML
claims.

**Altitude** (unchanged from the morning note): the floor certificate has the *shape* of a Davis system, which is
STRUCTURAL. There is still no single theorem of which both are instances. The revision must say that in the same
sentence that introduces the instance.

## 4. Lean plan (in the HELIKE Lean project, next)

| target | statement | source |
| --- | --- | --- |
| L1 | posterior lower bound is monotone: p₀ ≤ c₀ and p₁ ≥ 1 − c₁ imply the Cantelli posterior bound | Prop. 3 |
| L3 | Π(1 − Eᵢ) ≥ 1 − ΣEᵢ for Eᵢ ∈ [0, 1], and the four-equal-errors counterexample to Cor. 1 | §2.6 |
| L4 | (1 − ε)κ_soft > (1 + ε)κ_hard implies ambient separation under (1 ± ε) distortion | §2.4 |
| L5 | single-reference triangle step: d(z₀, z₁) ≥ κ_soft − κ_hard | §2.2 |
| L6 | m_eff = (E δ₋² − E δ₊²)/(2τ²), and the counterexample to Lemma 1 | §2.1 |
| L7 | log Σₖ e^{uₖ} ≥ log K + mean(u) | Lemma 1, Step 1 |
| L8 | 1 − sin x / x ≤ x²/6 on (0, 1] | §2.3 |

## In my words

I wrote this paper before I had a single exact case to test it against. Rereading it with her in hand, the shape
holds: margins, an enclosure that eats them, refusal where the enclosure wins, and a budget made of pieces. Some of
the steps I used to get there do not. Two of them, the reference points and the linear distortion, break in a way
that is better for me than the original: fixed, the paper agrees with the Davis Duality on the exponent. I would
rather have a 2025 paper that is right than one that is fast.
