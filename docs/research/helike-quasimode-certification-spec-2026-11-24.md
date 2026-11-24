# Quasimode certification for the sourced condensate: spec

24 November 2026. **SPEC, before implementation. No certificate has been computed.** It answers the open item in
the [boundary results](helike-qspace-boundary-results-2026-11-24.md): how to get statements about the *physical*
operator (the regular Cartesian Friedrichs realization, colour singlets, both determinant signs matched at rank
two) without first building a boundary-complete solver.

## The idea in plain English

I do not need a basis that reaches the rank-two interface if I can prove that the states I care about never get
there. Take the positive-cone Ritz vector, which lives deep inside det q > 0. Multiply it by a smooth cutoff that
is exactly one where the state lives and exactly zero well before det q reaches zero. The result is an ordinary,
compactly supported, smooth function of q. It sits in the domain of the physical operator, whatever the interface
rule is. Then measure how far it is from being an eigenfunction (its residual). A small residual proves there is
a true eigenvalue nearby. The cost of the cutoff shows up honestly inside that residual, so nothing about the
interface has to be assumed.

The matching rule Bee derived still matters. It is why no wall may be assumed, and it is what a future
boundary-complete solver must implement. This route simply never touches the interface.

## 1. Trial states

Let χ be a Ritz vector of `ConeRitz` (sector-projected, §3), and ψ = χ/√ν the physical function on the positive
sheet. Let η be a smooth cutoff built only from SO(2) and colour invariants:

\[
\eta(q)=h\!\left(\frac{\det G}{\delta\,\det G_0}\right)\,\big(1-h(|G|/R)\big),
\]

with h a fixed C^∞ step (0 below 1, 1 above 2; registered profile, not tuned). Set ψ_c = ηψ on det q > 0 and
zero elsewhere. Then ψ_c ∈ C_c^∞ of the open positive sheet and lies in the Cartesian operator domain. Because η
is a function of invariants, it commutes with the colour action and with the residual SO(2), so it does not mix
charge sectors.

**Cutoff placement.** The log-chart envelope puts the following weight below det G = δ det G₀ (one-dimensional
Gaussian tail along the linear direction c·z = log(det G/det G₀); computed, light):

| Point (s, t, g²) | sd of log det G | δ = 0.1 | δ = 0.01 |
| --- | --- | --- | --- |
| 1.15, 3, 0.004 (control) | 0.159 | 5e-48 | 2e-185 |
| 1.15, 3, 0.002 (control) | 0.112 | 7e-94 | below double range |
| 1.12, 3, 0.03 | 0.458 | 2.5e-7 | 5e-24 |
| 1.15, 3, 0.05 | 0.561 | 2.0e-5 | 1e-16 |
| 1.24, 3, 0.07 | 0.574 | 3.0e-5 | 5e-16 |
| 1.09, 4, 0.03 | 0.531 | 7e-6 | 2e-18 |
| 1.18, 4, 0.05 | 0.583 | 4e-5 | 1e-15 |

So δ = 0.01 is the registered choice. These numbers are for the trial envelope and are only a placement guide.
The certificate uses the measured residual, never these estimates. Excited states carry polynomial prefactors,
and the true eigenfunction's tail is set by the Agmon distance, not by the Ritz Gaussian.

## 2. Residuals of the full operator

With ψ_c normalized, let E = ⟨ψ_c, Hψ_c⟩ and σ² = ‖Hψ_c‖² − E². The operator H is the physical one: −½Δ_q plus
the sourced potential, written in the signed chart, with the reduced Laplacian from the boundary note (density
μ = 8r₀³r₁², cometric B = KKᵀ). On the support of η, that chart and the positive log chart are the same patch.

- **Weinstein (unconditional).** The interval [E − σ, E + σ] contains a point of spec H.
- **Cutoff accounting.** Hψ_c = η(H − E_R)ψ + E_Rηψ − ∇η·B∇ψ − ½(Δ_red η)ψ. The commutator terms are supported in
  the transition shells. They are computed, not dropped, and their norm is reported separately as the cutoff
  leakage. A certificate whose leakage exceeds a tenth of σ is reported but flagged: the cutoff is badly placed.
- **How ‖Hψ_c‖² is evaluated.** By direct quadrature of (Hψ_c)² over the support, at two quadrature orders
  (Gauss–Hermite in the log chart, inflated by one order beyond the polynomial degree, with η handled by
  composite rules on the shells). Do not use the Galerkin matrix squared: H² restricted to the trial space
  misses the out-of-space part, which is exactly the residual.
- **Independent oracle.** At sampled points, recompute Hψ_c by direct nine-coordinate Cartesian differentiation,
  reusing the boundary note's oracle. Require agreement ≤ 1e-9·(1 + |Hψ_c|).

## 3. Exact SO(2) charge sectors

The background diag(x, y, y) leaves the SO(2) rotating space indices 1, 2 (locked to colour) as a symmetry of H.
On G = qqᵀ the invariants are G₀₀, G₁₁ + G₂₂, |v|² with v = (G₀₁, G₀₂), the 2×2 block determinant, vᵀSv, and the
oriented d = det q. The charged covariants are v± = G₀₁ ± iG₀₂ (charge ±1) and
w± = ½(G₁₁ − G₂₂) ± iG₁₂ (charge ±2). Helike's doublet is charge ±2. The competitor and the ground state are
charge 0.

The projection is exact, not inferred from overlaps. P_m ψ(G) = (1/N)Σ_k e^{−2πimk/N} ψ(R_{2πk/N}·G), with
N > 2·(maximum charge present in ψ). The maximum charge is bounded by the polynomial degree, so trapezoid in the
angle is exact for the polynomial part. For the log-chart envelope, which is not SO(2)-invariant, take N and 2N
and require agreement at 1e-12. Because η is invariant, P_m(ηψ) = ηP_mψ. Report the charge residual
‖(L − m)ψ_c‖/‖ψ_c‖ as a check, not as the definition.

All Rayleigh quotients, residuals and Temple bounds below are taken inside one charge sector. Since H commutes with
L, a sector statement is a statement about the full spectrum restricted to that sector.

## 4. Upper bounds, and Temple lower bounds under a stated separation

- **Upper bounds (unconditional).** In each sector, Rayleigh–Ritz on the cutoff space span{ηP_mχ_k} gives
  rigorous upper bounds on the k-th physical eigenvalue in that sector (min–max). The cutoff space lies in the form
  domain, so no boundary assumption enters.
- **Lower bounds (conditional).** Temple: if λ_k < ρ ≤ λ_{k+1} in the sector, then λ_k ≥ E − σ²/(ρ − E). The
  number ρ is not computable from the trial space. It enters as an explicit **separation hypothesis S**:
  - **S1 (sector-local).** In each charge sector, the level after the one being bounded lies no lower than
    E₀ + (its harmonic value in that sector) − 0.25·ω_min. The harmonic values come from the six normal
    frequencies and the locked charges. In charge 0, that is the second harmonic level. In charge ±2, it is the
    next harmonic level after ω_H.
  - **S2 (global).** No eigenvalue below E₀ + 2ω_max comes from outside the background well: not from the origin
    region, and not from the det q < 0 sheet. Supporting evidence, which is not proof, is a classical global
    minimization of V_src on both sheets with many starts. It must show V_src ≥ V_bg + 3ω_max outside
    the cutoff region. The mirror sheet differs by V(w) − V(−w) = −2g(Σω)r₀r₁w, which is large at these depths,
    so the check is expected to pass. It must still be run.
- A future upgrade replaces S by an IMS-localization lower bound (H ≥ Σ JᵢHJᵢ − ½Σ|∇Jᵢ|²). That is out of scope
  here.

**What the switch needs.** The ordering "competitor above Helike" needs only (Helike's upper bound in charge ±2) <
(competitor's Temple lower bound in charge 0). E₀ cancels from the ordering. Gap *values* also need E₀ enclosed:
the Ritz upper bound plus Temple lower bound in charge 0.

## 5. Arithmetic honesty

Double-precision quadrature is not an interval enclosure. Two certificate levels are recorded separately:
- `PASS_CERTIFIED_FP`: residuals evaluated in floating point, two quadrature orders agree, and the oracle agrees.
- `PASS_CERTIFIED_INTERVAL`: the same, with residual integrals enclosed in interval arithmetic (mpmath `iv`, or
  Arb).

Either one remains conditional on S for lower bounds. Receipts state the level and list S verbatim.

## 6. Gates, in order

1. **Controls first** (s = 1.15, t = 3, g² = 0.004, 0.002, at Mq = 6 and 8). Weinstein intervals must contain the
   Ritz gaps' constituents. The cutoff leakage must be below 1e-10. The O(g²) coefficients must stay within the
   existing 0.03 of the exact values.
2. **Convergence** (`run_helike_cone_convergence.py`): CONVERGED at 0.003, Mq 6 → 8.
3. **Mutants**, each must be caught by an assertion:
   - drop the commutator terms;
   - a non-invariant cutoff (η in G₁₁ alone), which must break the charge check;
   - Galerkin H² in place of the true ‖Hψ‖²;
   - a wrong sector phase;
   - S omitted from a Temple bound, which must refuse rather than produce a number;
   - flat Laplacian in the chart.
4. **Method amendment**, recorded in its own document before any registered point: the registered method changes
   from LocalFock (not semibounded, retired) to cutoff-certified cone Ritz. Physical formula, grid, predictions,
   tolerance 0.015 and decision rule are unchanged. The convergence ladder becomes Mq = 6, 8 with the same 0.003
   rule. Identification by exact charge sector replaces the 0.9 harmonic-weight rule; the weight is still
   reported.
5. **Only then** the registered runner may release, and only on the large machine. The runner already refuses
   without all three conditions (`unmet_release_conditions`).

## 7. What this does not do

It does not build the two-sheet solver or decide anything at the interface. It does not certify states whose
weight reaches the cutoff shells. Near the top of the s window, or at larger g², the leakage gate will say so
rather than pass. It does not lift S. A switch certified under S is reported as "conditional on S" in every
summary, the paper included.
