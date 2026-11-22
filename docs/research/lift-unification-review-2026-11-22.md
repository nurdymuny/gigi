# The Lift Unification, reread line by line: what holds, what breaks, and what the revision needs

*22 November 2026. A research note, not a result. Fifth in the series, after the Davis Manifold, the spectral
branch, Non-Decoupling and the Duality.*

**The paper.** *The Lift Unification: Why Six Pillars of Probability Fail for the Same Geometric Reason*
(March 2026; Zenodo 19344768).
- Published file: `lift.pdf`, 21 pp., md5 a1152066…, identical to the Zenodo copy.
- Of the two `.tex` files in Downloads, `lift_unification (1).tex` matches the published text: it is the one with
  "K projects differently onto each operation". `lift_unification.tex` is an earlier draft.

**Why now.** §10 rests on the Non-Decoupling Theorem's Part II, which this morning's review found false. The
mammogram in this paper is also where *The phone in the house* (§4.4) put the floor's skeleton: the floor changes
hands at C = 1.

**Receipts.** Every number below is checked in Lean (`Lift/Review.lean` on `feat/helike-lean`, 9 theorems), most as
exact rational arithmetic. Mossel's paradox is checked as real infinite sums.

## 1. What holds: all six falsifications, as mathematics

- **Mammogram.**
  - Λ(Sick, Pos) = 0.9/0.108 = 25/3 and Λ(Healthy, Pos) = 25/27.
  - The normalized covariance is Λ − 1 = 22/3.
  - The posterior is 25/3 × 1/100 = 1/12 ≈ 8.3%.
- **Dice.**
  - Cov(A, C) = −1/72 with Λ = 2/3; Cov(A, D) = 0.
  - The knife-edge point (difference 3 against difference 4) is exact.
- **Monty Hall, the prosecutor, and the violence predictor** P(A|B)/P(B|A) = P(A)/P(B): correct.
- **The capacity law is exact, not approximate** (§7). Posterior log-odds = −K + τ, so the posterior passes ½ **if and
  only if** τ > K, that is C = τ/K > 1. *Lean: `capacity_law`.*
  - Base rate 1%: τ = ln 9 ≈ 2.20, K = ln 99 ≈ 4.60, C = 0.478, posterior 8.3%.
  - Base rate 20%: K = ln 4, C = 1.58, posterior ≈ 69%.
- **Mossel's dice.**
  - P(all even) = 1/4.
  - P(k | all even) = 2·(1/3)^k sums to 1, and E[k | all even] = **3/2**.
  - The Lifts are 4, 8/5, 16/25.

  All four are exact; the expectation is an infinite sum. *Lean: `mossel_*`.*
- **The sharpest sentence in the paper survives intact,** and it is the floor's skeleton. Evidence (the belt) moves
  the log-odds by τ; the prior resists with K; the floor (the more probable hypothesis) changes hands **exactly at
  C = 1**. This is the STRUCTURAL bridge recorded in *The phone in the house* §4.4, and it is now Lean-checked.

## 2. What breaks, and the repair for each

### 2.1 One explanatory step in Mossel's paradox is wrong (§8.3)

The paper says: "A length-1 sequence has probability 1/2 of being all even … length 2 has probability 1/4. Length 3
has probability 1/8."
- Given that the first 6 comes at roll k, the earlier k − 1 rolls are uniform on {1, …, 5}, two of which are even.
- So P(all even | length k) = (2/5)^(k−1): it is 1 at k = 1 (a single 6 is even), then 2/5, then 4/25.
- The conclusion is unaffected. The penalty is still exponential in k, with ratio 2/5, and the computed answer 3/2
  is correct. *Lean: `all_even_given_length`*, which checks consistency with the paper's own P(length k ∧ all even).

### 2.2 Proposition 5 contradicts §7 about C = 1

Proposition 5 says the tools "behave as the standard pedagogy suggests" when "all three [Λ, the prior ratio and
τ/K] are near 1". But at τ/K = 1 the posterior odds are exactly 1. That is the 50/50 point of *maximum* uncertainty,
as §7.3 itself computes ("The posterior would be 50%"). *Lean: `capacity_one_is_fifty_fifty`.*

**Repair.** The benign regime for the capacity is C ≫ 1 (evidence dominates) or C ≪ 1 (prior dominates, and
intuition is warned). C ≈ 1 is the edge, where the floor changes hands and intuition is least reliable. That reading
is also the one that matches the floor.

### 2.3 §10 restates the false part of Non-Decoupling

Theorem 6, item 1: "any flat (decoupled) model incurs irreducible structural error bounded below by the Yang–Mills
action ∫‖F‖²." The conclusion repeats it: "irreducible error bounded below by the curvature".
- This is Part II of the Non-Decoupling Theorem, which is false as stated (bounded holonomy error against unbounded
  energy; see the [Non-Decoupling review](non-decoupling-review-2026-11-22.md) §2.2).

**Repair.** Cite the parts that hold:
- flat ⇔ trivial holonomy;
- c_k ≠ 0 forbids flatness;
- for the quantitative step, the small-loop holonomy law and the Duality's per-cell bound for functions.

### 2.4 §10.3's dictionary is interpretation, and should say so

Each item is a reading, not a theorem:
- "Conditioning is parallel transport";
- "the covariance is the curvature tensor";
- "Bayes' rule is a round trip whose holonomy debt is the prior ratio";
- "the tree is an atlas of charts".

No connection on a bundle is constructed in which conditioning *is* transport and covariance *is* curvature. The
paper's own closing ("the geometric structure is … the natural completion of a pattern") is the right register.
§10.3 should match it: tag each item RESONANCE, or construct the bundle.

### 2.5 The factor of 2 and the Troyon limit (§8.4)

"Whether the factor of 2 is precisely the covering degree of S¹ → ℝP¹ — as it is in plasma confinement (Troyon
limit)…" The CHIHIRO audit today found the Troyon/1/(2π) identification unsupported: no committed code reproduces
0.163, and the natural-to-engineering conversion appears to be 40π, not 8π, pending Chihiro's check. Remove "as it
is in plasma confinement (Troyon limit)". The rest of the sentence already says this is an open question, which is
honest.

### 2.6 Smaller things

- **Count.** The introduction says the Lift "appears identically in all four frameworks"; the paper has six.
- **Unreceipted claim.** §10.5's "validated across eleven branches" has no receipt. Cite the branches, or say "applied
  in".
- **Pedagogy register.** Phrases like "The democratic picture is a lie" and "at least one unnamed MIT faculty member"
  are a matter of tone, not correctness. The acknowledgment is gracious; the body could match it.

## 3. What this means for the arc

The mammogram bridge is **strengthened**, not weakened. The capacity law is exact and Lean-checked: the more probable
hypothesis changes at precisely C = 1, and Proposition 5's mistake is exactly that it treated that edge as the safe
place. The skeleton (a carried level, a nearly blind rival, and a floor that changes hands where they meet) is here
in exact form. The anatomy (the net-turning square, the cross term, curvature against spin) is not, as *The phone in
the house* already said.

## In my words

This paper is right where it stays inside probability. Every number checks, Mossel's 3/2 included, and the line I
liked best turns out to be exact: the evidence overturns the prior precisely when τ beats K. Where it went wrong is
the same place the other papers did: a borrowed theorem that was false (the Yang–Mills lower bound), a dictionary
stated as identity, and one sentence that called the edge the safe place. The edge is never the safe place. That is
what the floor taught me.
