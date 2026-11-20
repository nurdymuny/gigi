# Magnet M1: the moment-corrected magnet (specification)

**Status:** specification, written 19 November 2026, after M0 ([`MAGNET_M0_RESULTS.md`](MAGNET_M0_RESULTS.md)) and
before any M1 code or audit run. Research code, not an engine run. Abelian control, q6, the M0 meshes and data.

## Why

M0 kept everything the magnet rule asks for and it worked:
- seamless bumps (P-seam 0.0094%);
- a seat spring that removes every wobble mode (P-wobble 0);
- a force that is an energy gradient to 6 × 10⁻¹⁶.

Its error now splits cleanly:

| | n = 2 | n = 3 |
| --- | --- | --- |
| Reconstruction term | 7.65% | 6.46% |
| Poke term | 51.15% | 51.06% |

The poke term stopped growing when the seams went (the seamed removal arm grows 147% → 170%). It did not start
falling.

**The hypothesis (Lemma B).** For a seamless reading, Lemma A turns the poke term into the *moment term*
⟨δF, (ψ_e − W_e)⟩ for each edge e, where ψ_e = R¹δ_e is the reading's bump for a single poke and W_e is the Whitney
tent. Lemma B says this term is first order if each bump has its tent's **zeroth moment**:

  z_e^a := ∫_Ω (ψ_e − W_e)(e_a) dvol = 0,  a = 0, 1, 2,

taken componentwise in the left-invariant frame e_a. M0 never imposed this. A k = 1 MLS reading matches Whitney's
moments only *in sum*, through reproducing constants and linear fields, not edge by edge.

**This control is special.** Its field is left-invariant: A = θ⁰ and F = dA, so δF = 4θ⁰ is *constant* in the
left-invariant frame. For it, the moment term has no Taylor remainder:

  poke_e = 4 z_e⁰ exactly (up to quadrature).

So the audit below is a sharp test of the hypothesis, not a fit.

## Part 1: the moment audit (runs first; n = 1 and n = 2)

**Script:** `scripts/ricci_magnet_m0_moment_audit.py`. Same Cell, reach, weights (κ = 1, ρ = h_max) and
quadrature as M0.

**Measured, per edge:**

- **z_e^a** for a = 0, 1, 2: the reading's bump and the Whitney tent integrated in the left-invariant frame, with
  volume element √det g. The reading is converted from K's chart to reference coordinates by the affine chart map,
  then evaluated on the dual frame.
- The relative defect **|z_e| / ‖W_e‖_L¹**.
- M0's poke vector **t_e**, recomputed exactly as M0 does it: ⟨F, curl ψ_e⟩ − ⟨F, W² Dδ_e⟩.
- The prediction **p_e = 4 z_e⁰**.

**Reported:**

- the poke term from t, as an M0 reproduction check: 51.15% at n = 2;
- the poke term from p;
- the dual-mass-norm cosine between t and p;
- the relative residual ‖t − p‖ / ‖t‖;
- the distribution of |z_e| / ‖W_e‖_L¹;
- z for all three frame components, saved for M1 as `moments_L{n}.npz`.

**Predictions, recorded before the run:**

- **A1.** The M0 reproduction agrees with the receipt to 1e-6 relative.
- **A2.** p accounts for the poke term: cosine ≥ 0.99 and ‖t − p‖ / ‖t‖ ≤ 0.05 at n = 1 and n = 2.
- **A3.** The zeroth-moment defect is O(1), not O(h): the median of |z_e| / ‖W_e‖_L¹ does not fall from n = 1 to
  n = 2 by more than a factor 1.5.

**If A2 fails**, the moment term is not what holds the poke term up. Then M1 is not built, and the next step is
to locate the term that does: the seat spring, quadrature, or the chart-to-frame step.

## Part 2: M1, built only if A2 holds

**The reading.** It is M0's reading with every bump corrected to its tent's zeroth moment:

  R̃¹a = R¹a − Σ_e a_e Σ_a z_e^a β_e^a,  β_e^a := φ(|X − m_e| / ρ) θ^a / ∫ φ(|X − m_e| / ρ) dvol.

Here φ is M0's Wendland C² function, m_e the edge midpoint on S³, and θ^a the left-invariant coframe. The
correction fields have these properties:

- **Smooth and global** (Wendland × coframe), so the reading stays seamless (M1 of the rule).
- **Moment-exact.** Each β_e^a has unit a-moment and zero b-moment for b ≠ a, so R̃¹ has every bump's zeroth moment
  equal to its tent's.
- **Bounded:** supported in the same ball as the reading's weights.

**Rows.** Constants are still reproduced exactly: for left-invariant data, Σ_e (I¹c)_e z_e = 0 because both R¹ and
W¹ reproduce constants. Linear fields are reproduced only up to O(h): Lemma C's first-order conflict, which Lemma B
says is all that consistency needs.

**The force** is ∇E for E = ½ Σ_K w_K ∫_K |curl R̃¹a|² plus the seat spring on R̃¹'s own-wall flux. It is variational
by construction.

**The arms:**

- **seated:** the M1 force;
- **unseated:** no spring; reported only;
- **removal:** M0 itself, already measured.

**Gates.** Levels n = 2 and n = 3 decide; n = 1 is reported.

- **M1-poke.** The poke term at n = 2 is at most a tenth of M0's (≤ 5.1%), and it is no larger at n = 3.
  - For this control the moment term vanishes identically once the moments match. What remains is quadrature and
    the spring.
- **M1-recon (V2's failure mode).** The reconstruction term is at most 2× M0's at each level (≤ 15.3% at n = 2,
  ≤ 12.9% at n = 3), and it does not grow from n = 2 to n = 3.
- **M1-moment.** max_e |z̃_e| / ‖W_e‖_L¹ ≤ 1e-8 (the correction does what it says).
- **M1-variational.** K is symmetric to 1e-12.
- **M1-wobble.** Seated arm: no physical eigenvalue below 1, and none negative.
- **M1-seam.** Wall jump ≤ 1% at n = 2, smaller at n = 3.

**Reported, not gated:**

- BW05 relative error (Whitney: 26.0% at n = 2);
- the curvature error;
- the spring's force share;
- the bump amplification max_e ‖ψ̃_e‖_L¹ / ‖W_e‖_L¹.

**Decision rule:**

- **All gates pass.** BW05 is then set by the reconstruction term, near 6–8% at k = 1.
  - The next spec is k = 2 with the moment correction (M2), aimed at BW05 itself.
  - A second, non-left-invariant test field is added there. For general smooth fields Lemma B promises O(h), not
    zero, and this control cannot show the difference.
- **M1-poke passes, M1-recon fails.** The correction fields are too rough.
  - Try the reach-wide normalisation (β over 2ρ) once. Then stop and rethink.
- **M1-poke fails although A2 held.** The spring or quadrature is coupling into the poke. Measure the spring's
  share before anything else.

## What does not change

- The data, meshes, mass, BW05 measure and target.
- The vote rule and reach.
- Wendland weights, κ = 1.
- No criterion is loosened after a result.
- BW05 is reported unchanged. Any weak-consistency or trajectory measure added later is a separate, pre-registered
  gate, never a replacement.

## Part 1 results (run after this specification was committed, at 4bc4477)

Receipts: `research/magnet-m1-20261119/audit_L0.json`, `audit_L1.json`. Per-edge moments and poke vectors are in
`moments_L{0,1}.npz`.

| | n = 1 | n = 2 |
| --- | --- | --- |
| Poke term recomputed from t (M0 receipt at n = 2: 51.15045%) | 46.53% | 51.15045% |
| Poke term predicted from p = 4 z⁰ | 46.37% | 51.35% |
| Cosine between t and p, dual mass norm | 0.9978 | 0.9993 |
| Best scale t/p | 1.0014 | 0.9955 |
| Residual ‖t − p‖ / ‖t‖ | **6.6%** | 3.8% |
| Median \|z_e\| / ‖W_e‖_L¹ | 19.5% | 31.1% |
| Median per frame component | 6.0% | 12.7% |

- **A1: pass.** The n = 2 recomputation equals M0's receipt to 1e-15.
- **A2: passes at n = 2, fails at n = 1 as written.**
  - At n = 2: cosine 0.9993 and residual 3.8%, both inside the lines.
  - At n = 1: cosine 0.998 and best scale 1.001, but the residual is 6.6%, over the 5% line written for both
    levels.
  - The residual falls with the seam jumps (0.46% at n = 1, 0.034% at n = 2), consistent with the interface terms
    that Lemma A omits for a reading that is seamless only to that accuracy. This is an explanation, not a pass.
    The decision rule ties building M1 to A2, so the n = 1 miss is reported to Bee for the decision.
- **A3: pass.** The defect is O(1) and grows from n = 1 to n = 2. It does not fall like h.
- **Not a check.** The "constants_check_sum_z" field in the receipts (4e-17 at n = 1, 0.011 at n = 2) sums z over
  all edges. That is the reading of data equal to one on every edge, not of a left-invariant field. It is not the
  constants identity of Part 2 and is ignored here.

**Reading.** At n = 2 the poke term that M0 left flat is, to within 4%, its bumps' zeroth-moment defect contracted
with the constant δF. That is Lemma B's mechanism, and it is what M1 corrects.

**Decision (Bee, 19 November 2026, after Part 1).** Build and run M1 despite A2's miss at n = 1 (residual 6.6%
against 5%). The decisive level, n = 2, passes cleanly, and the mechanism is unambiguous at both levels (cosine
0.998 and 0.9993). This is an exception to the decision rule as written, recorded here before any M1 code. Part 2's
gates are unchanged.

## Amendment 1: M1b (written after M1's n = 1 run and before any M1b run or any gated M1 result)

**What n = 1 showed (a reported level).**
- The moment correction removes the poke term: 46.5% → 3.07%.
- It also disturbs smooth fields:
  - the reconstruction term rises to 35.5%;
  - the seat spring, fighting the correction, carries 280% of the force.
- The correction Z a = Σ_e a_e z_e·β_e cancels on smooth data only globally (Σ_e a_e z_e^b = 0 for left-invariant
  a), not pointwise.

**M1b** applies the correction only to what the reading cannot reproduce:

  R̃¹a = R¹a − Z(a − G a),  G = I¹R¹ (line integrals of M0's reading along every edge).

- **Rows.** Data the reading reproduces (Ga = a, in particular every linear field in the cell's chart) is untouched.
  The rows are exact again.
- **Moments.** The moment left over on each edge is (Gᵀz)_e instead of 0. That is Lemma C's first-order
  conflict, made explicit.
- **Pre-check, run before this amendment, n = 1 only.** The median leftover is 1.7% (M0's defect: 19.5%), and
  ‖Gᵀz‖/‖z‖ = 0.066. z is rough from edge to edge, so G smooths it away.

**When it runs.** M1b runs if M1 fails M1-recon at n = 2. Otherwise M1 proceeds to n = 3 as specified.

**Gates for M1b.** As in Part 2, with M1-moment replaced by:

- **M1b-moment.** The median leftover |z̃_e|/‖W_e‖_L¹ is at most a fifth of M0's at n = 2 (≤ 6.2%), and smaller at
  n = 3 than at n = 2. The receipt also reports how far the leftover differs from the prediction Gᵀz.
- **M1-poke** (≤ 5.1% at n = 2, no larger at n = 3), **M1-recon** (≤ 2 × M0's, not growing), **M1-variational**,
  **M1-wobble** and **M1-seam** are unchanged.

**Script:** `scripts/ricci_magnet_m1b.py`, with the same reading, reach, quadrature and arms as M1.

## Amendment 2: M1c (written after M1b's gated n = 2 result and before any M1c run)

**What M1b did at n = 2.**

- Passes: M1b-moment (leftover median 5.02%), M1-recon (8.23%), variational, wobble and seam.
- Fails: M1-poke (10.42% > 5.1%).
- Reported BW05: 11.2% unseated, 14.0% seated. That is below Whitney's 26.0%, the first variational force to get
  there.
- The leftover moment grew from n = 1 to n = 2 (1.7% → 5.0%), and the poke term with it.

**Pre-check** (`scripts/ricci_magnet_m1_neumann_precheck.py`, run before this amendment). Applying the correction
again leaves (Gᵀ)^k z. The median leftover after k rounds:

| k | 0 (M0) | 1 (M1b) | 2 | 3 | 4 | 5 | 6 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| n = 1 | 19.5% | 1.7% | 1.9% | 1.7% | 1.6% | 1.5% | 1.4% |
| n = 2 | 31.1% | 5.0% | 2.7% | 1.7% | 0.98% | 0.74% | 0.58% |

- What does not decay lies along G's top three eigenvectors: eigenvalue 0.966 at n = 1, 0.990 at n = 2. That is the
  near-left-invariant data, which G nearly reproduces: Lemma C's barrier, made explicit.

**M1c.** M1b with the correction coefficients summed over K + 1 rounds:

  c_e = z'_e / N_e,  z' = Σ_{k=0}^{K} (Gᵀ)^k z,  **K = 8, fixed here.**

- The reading is still R̃¹a = R¹a − Z′(a − Ga). Rows stay exact, and the force stays an energy gradient.
- The leftover moment is (Gᵀ)^{K+1} z. The receipt checks this against the assembled moments.

**Gates.** Unchanged from Amendment 1:

- M1b-moment: median leftover ≤ 6.2% at n = 2, and smaller at n = 3;
- M1-poke: ≤ 5.1% at n = 2, no larger at n = 3;
- M1-recon: ≤ 2 × M0's, not growing;
- M1-variational, M1-wobble, M1-seam.

**Script.** `scripts/ricci_magnet_m1b.py LEVEL OUTDIR --neumann 8` writes `M1c_L{n}.json`.

## Amendment 3: M1d, the seat spring scaled with h² (Bee's instruction; written before any M1d run)

**Why.** The seat spring compares the reading's own-wall fluxes with the measured ones under a Whitney-stiff
penalty. For smooth data the misfit is O(h²). The penalty's stiffness grows like 1/h², so the spring's force error
stays O(1). Its measured force share does not fall with refinement:

- M1b: 13.5% at n = 1, 14.0% at n = 2;
- M1c: 18.5% at n = 1, 17.0% at n = 2.

**M1d.** M1c with σ_n = (h_n/h₁)², so σ = 1 at n = 1 and the spring scales like h². Nothing else changes: the same
reading, correction, K = 8 and arms. The gates are those of Amendment 2, judged on the seated arm.
- M1-wobble checks that the scaled spring still removes the wobble modes. The scaled spring adds stiffness
  σ·O(1/h²) = O(1) on them, so it should.

**Implementation.**
- `scripts/ricci_magnet_m1c_fast.py` assembles M1c without widening each cell to the columns of Q = I − G:
  K = K₀ − QᵀX − XᵀQ + QᵀBQ, and the same pattern for the spring and the force vectors.
- Validated before use: at n = 1 it reproduces `M1c_L0.json` to 2 × 10⁻¹³ on every reported quantity, four times
  faster.
- M1d is the same script with `M1_SIGMA_H2=1` and `--tag M1d`.

**M1c at n = 3** (Amendment 2, σ = 1) also runs with the fast assembly, so that Amendment 2 is judged as specified.
The original-assembly n = 3 runs of M1b and M1c were stopped (too slow: about a day). M1b had already failed
M1-poke at n = 2.

### M1d at n = 2 (result, 20 November)

Receipt `research/magnet-m1-20261119/M1d_L1.json` (σ = 0.270 at n = 2). Fast assembly, so reading, correction and
moments are those of M1c; only the seated arm changes.

| | M1c n = 2 (σ = 1) | M1d n = 2 (σ ∝ h²) |
| --- | --- | --- |
| leftover moment, median | 0.38% | 0.38% |
| poke term | 2.49% | 2.49% |
| reconstruction term | 11.48% | 11.48% |
| seam (RMS jump, relative) | 0.034% | 0.034% |
| seated: spring force share | 17.0% | **4.6%** |
| seated: wobble modes below 1 | 0 | 0 |
| seated: smallest physical eigenvalue | 3.91 | 3.91 |
| seated: BW05 (reported, not gated) | 15.0% | **9.9%** |
| unseated: BW05 | 11.3% | 11.3% |

- **Gates at n = 2: all pass**, as for M1c: moment ≤ 6.2%, poke ≤ 5.1%, reconstruction ≤ 2 × M0's, symmetric to
  2 × 10⁻¹⁶, wobble-free, seam.
- Scaling the spring with h² cut its force share from 17% to 4.6%. The seated BW05 fell from 15.0% to 9.9%, below
  the unseated arm and well below Whitney's 26.0%.
- The scaled spring still removes every wobble mode at n = 2.
- **The trend gates are decided at n = 3** (M1c and M1d at n = 3, both running).
  - Pass 1 already predicts a leftover-moment median of 0.70% at n = 3, larger than 0.38% at n = 2.
  - So **the M1b-moment trend gate ("smaller at n = 3") will fail** for both, unless the cell pass contradicts its
    own prediction. It has matched the prediction to 10⁻¹³ at every level so far.
  - The poke and reconstruction trend gates are still open.
