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
