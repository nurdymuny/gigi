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
a spectral point nearby; calling it a particular eigenvalue also needs discreteness and identification.
The cost of the cutoff shows up honestly inside that residual, so nothing about the
interface has to be assumed.

The matching rule Bee derived still matters. It is why no wall may be assumed, and it is what a future
boundary-complete solver must implement. This route simply never touches the interface.

## 1. Trial states

Let χ be a Ritz vector of `ConeRitz`, and ψ = χ/√ν the physical function on the positive
sheet. Apply the charge projector in §3 to this physical function. A naive scalar rotation of χ is not
equivalent unless the density transformation is included in its pulled-back action. Let η be a smooth cutoff
built only from SO(2) and colour invariants:

\[
\eta(q)=h\!\left(\frac{\det G}{\delta\,\det G_0}\right)\,\big(1-h(|G|/R)\big),
\]

Here |G| means the Frobenius norm. Use h(x) = a(x-1)/(a(x-1)+a(2-x)), where a(u) is
exp(-1/u) for u > 0 and zero otherwise. This fixes the smooth profile and all its derivatives before measurement.
The finite outer radius R, its units, the quadrature partition and orders, and the rigorous integration-error
budget remain **UNREGISTERED: no certification run may start until they are fixed in a dated amendment**.
No adaptive choice may use the desired ordering as its stopping rule. Set ψ_c = ηψ on det q > 0 and
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
The certificate uses the full-state residual with error bounds, never these estimates. In particular the
determinant transition is δ < det G/det G₀ < 2δ; mass below δ does not control derivatives on that shell.
The outer shell R < |G| < 2R must also be included. Excited states carry polynomial prefactors,
and the true eigenfunction's tail is set by the Agmon distance, not by the Ritz Gaussian.

## 2. Residuals of the full operator

With ψ_c normalized in the physical measure, let E = ⟨ψ_c, Hψ_c⟩ and
σ² = ‖(H − E)ψ_c‖². Inner products conjugate the first factor. The identity
σ² = ‖Hψ_c‖² − E² is an independent cross-check, not the preferred subtraction of two nearly equal numbers.
Negative computed variance, nonfinite values, or an unresolved imaginary part of E must refuse, not clamp.
The operator H is the physical one: −½Δ_q plus
the sourced potential, written in the signed chart, with the reduced Laplacian from the boundary note (density
μ = 8r₀³r₁², cometric B = KKᵀ). On the support of η, that chart and the positive log chart are the same patch.

- **Spectral proximity (no separation hypothesis).** For a self-adjoint H, a unit vector in D(H), and its
  exact E and σ, [E − σ, E + σ] intersects spec H. Enclosed arithmetic must enlarge this interval for errors
  in E, residual and normalization. Floating-point estimates alone are not a rigorous enclosure. Proximity
  does not identify the ground state, distinguish a doublet, or establish a particular level's ordering.
- **Cutoff accounting.** Hψ_c = η(H − E_R)ψ + E_Rηψ − ∇η·B∇ψ − ½(Δ_red η)ψ. The commutator terms are supported in
  the transition shells. They are computed, not dropped, and their norm is reported separately as the cutoff
  leakage. Leakage exceeding a tenth of σ is a diagnostic warning, not permission to waive the absolute
  control gate below. If σ = 0 this ratio is undefined and both absolute norms must be reported.
- **How residuals are evaluated.** By direct quadrature of |(H − E)ψ_c|², with |Hψ_c|² as a cross-check,
  over the support. Complex charge states require absolute squares, never (Hψ_c)². Two quadrature orders
  and Cartesian samples test numerical agreement only. Smooth cutoffs and log-chart functions are not
  polynomials: increasing a Hermite order by one is not a quadrature exactness theorem. Certified mode
  must bound integration and derivative errors across the interior and both shells. Do not use the
  Galerkin matrix squared: H² restricted to the trial space
  misses the out-of-space part, which is exactly the residual.
- **Independent oracle.** At sampled points, recompute Hψ_c by direct nine-coordinate Cartesian differentiation,
  reusing the boundary note's oracle. Require agreement ≤ 1e-9·(1 + |Hψ_c|).

## 3. SO(2) charge sectors: exact construction or bounded projection error

The background diag(x, y, y) leaves the SO(2) rotating space indices 1, 2 (locked to colour) as a symmetry of H.
On G = qqᵀ the invariants are G₀₀, G₁₁ + G₂₂, |v|² with v = (G₀₁, G₀₂), the 2×2 block determinant, vᵀSv, and the
oriented d = det q. The charged covariants are v± = G₀₁ ± iG₀₂ (charge ±1) and
w± = ½(G₁₁ − G₂₂) ± iG₁₂ (charge ±2). Helike's doublet is charge ±2. The competitor and the ground state are
charge 0.

The exact projector is the continuous group integral
P_m ψ(G) = (1/2π)∫ e^{−imθ} ψ(R_θ·G) dθ. A finite angular sum is exact only after a
finite Fourier bandwidth is proved for the **whole function** under this group action. Polynomial degree in
log-Cholesky or normal coordinates is not such a proof; neither is agreement between N and 2N samples at 1e-12.
For example, mode 34 aliases to mode 2 on both 16- and 32-point grids, although its true mode-2 coefficient is zero.

Two admissible routes are kept distinct:
- Construct charged covariants in G with a proved transformation law, multiplied by invariant envelopes and
  cutoff. Algebraic charge is then exact; this is a new trial construction, not a relabeling of the current basis.
- Project the existing trial functions by angular quadrature and **bound** the omitted Fourier coefficients
  in the norm required by H (not only pointwise values). Carry the resulting normalization, charge and residual
  errors into every bound. Without that tail bound, report approximate-sector diagnostics only; no sector
  certificate or Temple separation claim is allowed.

Fix the action/sign convention by testing v± and w± under an explicit rotation before using any label.
Because η is invariant, it commutes with the continuous projector. Report ‖(L − m)ψ_c‖/‖ψ_c‖,
the tail enclosure, and the original harmonic weights. N/2N agreement remains a useful diagnostic, not proof.

All Rayleigh quotients, residuals and Temple bounds below are taken inside one charge sector. Since H commutes with
L, a sector statement is a statement about the full spectrum restricted to that sector.

## 4. Upper bounds, and Temple lower bounds under a stated separation

- **Upper bounds (no separation hypothesis).** In each exact sector, Rayleigh–Ritz on the cutoff physical
  functions span{ηP_mψ_k} gives
  rigorous upper bounds on the k-th physical eigenvalue in that sector (min–max). The cutoff space lies in the form
  domain, so no boundary assumption enters. This assumes the ordered discrete eigenvalues exist and the trial
  functions are independent. Numerical matrix, mass, quadrature and generalized-eigensolve errors must be enclosed
  before computed numbers are called rigorous upper bounds.
- **Lower bounds (conditional).** Require a self-adjoint sector realization, a normalized domain vector,
  a correctly identified ordered discrete level λ_k, λ_k < ρ ≤ λ_{k+1}, and **E < ρ**. Then
  λ_k ≥ E − σ²/(ρ − E). If an enclosure cannot establish a strictly positive denominator, refuse.
  The bound alone does not establish that k is the ground or competitor index. Lower-level admixture does not
  automatically invalidate this one-sided inequality, but level identity and upper separation must be justified. The
  number ρ is not computable from the trial space. It enters as an explicit **separation hypothesis S**:
  - **S1 (sector-local).** In each charge sector, the level after the one being bounded lies no lower than
    E₀ + (its harmonic value in that sector) − 0.25·ω_min. The harmonic values come from the six normal
    frequencies and the locked charges. In charge 0, that is the second harmonic level. In charge ±2, it is the
    next harmonic level after ω_H.
    E₀ here denotes the exact ground energy: a computable separation based on it needs an independent lower
    bound (or remains an explicit hypothesis). Substituting a Ritz upper bound is not valid.
  - **S2 (global, not yet operational).** The proposed statement that no eigenvalue below E₀ + 2ω_max comes
    from outside the background well: not from the origin
    region, and not from the det q < 0 sheet. Supporting evidence, which is not proof, is a classical global
    minimization of V_src on both sheets with many starts. It must show V_src ≥ V_bg + 3ω_max outside
    the cutoff region. The mirror sheet differs by V(w) − V(−w) = −2g(Σω)r₀r₁w, which is large at these depths,
    so the check is expected to pass. It must still be run. Eigenfunctions do not partition into inside/outside
    labels automatically: a localization partition, auxiliary domains, boundary conditions and localization
    error must be specified before S2 is a precise test. Many-start minimization is evidence, not a global
    lower bound. Until S2 is made precise, no lower-bound certificate citing it is acceptable.
- A future upgrade replaces S by an IMS-localization lower bound (H ≥ Σ JᵢHJᵢ − ½Σ|∇Jᵢ|²). That is out of scope
  here.

The one-sided inequality can be checked directly: no spectrum in (λ_k, ρ) implies
⟨(H − λ_k)(H − ρ)⟩ = σ² + (E − λ_k)(E − ρ) ≥ 0. Division by the positive
ρ − E gives the stated lower bound. If E ≥ ρ that division does not give it; the algorithm must refuse.

**What the switch needs.** The ordering "competitor above Helike" needs only (Helike's upper bound in charge ±2) <
(competitor's Temple lower bound in charge 0). E₀ cancels from the ordering. Gap *values* also need E₀ enclosed:
the Ritz upper bound plus Temple lower bound in charge 0.

## 5. Arithmetic honesty

Double-precision quadrature is not an interval enclosure. Two evidence levels are recorded separately:
- `PASS_NUMERICAL`: numerical agreement only; not a spectral certificate and cannot clear the release gate.
- `PASS_CERTIFIED_INTERVAL`: enclosures for normalization, Rayleigh energy, full residual and cutoff derivatives,
  charge projection/tails, integration truncation, mass matrix and eigensolve errors. Interval-valued samples
  alone do not enclose an integral. The proof obligations and their evidence must all be present.

Lower bounds remain conditional on S until S is proved. Receipts separate arithmetic status, target identification,
and assumptions; list S verbatim and label every dependent bound conditional. `PASS_CERTIFIED_FP` and the bare
`PASS_CERTIFIED` are retired ambiguous labels. No certificate schema/verifier is implemented yet; no status
string, including `PASS_CERTIFIED_INTERVAL`, can by itself release a run.

## 6. Gates, in order

1. **Controls first** (s = 1.15, t = 3, g² = 0.004, 0.002, at Mq = 6 and 8). Measure proximity intervals for
   absolute energies, then form gap enclosures using an independently enclosed ground energy. A raw Ritz value
   is not a known physical eigenvalue and is not an oracle for interval containment. Include exact harmonic
   controls with known spectra and residuals. The cutoff leakage must be below 1e-10 (a proposed extra gate,
   not a relaxation of an old one). O(g²) coefficients stay within the existing 0.03 of the exact values.
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
   rule. Exact/bounded charge identification is additional: it does **not** replace the existing 0.9 grid
   harmonic-weight requirement. The convergence controls retain weight ≥ 0.95, reference-weight quadrature
   error ≤ 1e-6, and gap change ≤ 0.003. The earlier perturbative control gate stated weight > 0.95;
   preserve that strict comparison when replaying that gate. These are distinct gates, not adjustable tolerances.
5. **Only then** the registered runner may release, and only on the large machine. The runner already refuses
   without all three conditions (`unmet_release_conditions`).

## 7. What this does not do

It does not build the two-sheet solver or decide anything at the interface. It does not certify states whose
weight reaches the cutoff shells. Near the top of the s window, or at larger g², the leakage gate will say so
rather than pass. It does not lift S. A switch certified under S is reported as "conditional on S" in every
summary, the paper included.

## 8. Evidence repair before any larger run

The [evidence amendment](helike-quasimode-evidence-amendment-2026-11-24.md) freezes the RED/GREEN and
mechanism-removal checks. Neither a convergence receipt nor these small software checks certify the physical
spectrum. The Mq 4 to 6 discrepancy remains on record, and no Mq 8 calculation is part of this repair.

Mathematical references: [Tadmor on Fourier aliasing](https://math.umd.edu/~tadmor/lecture-notes/hyperbolic/node15.html)
and [Kuroda on Kato-Temple estimates](https://www.kurims.kyoto-u.ac.jp/~prims/pdf/43-2/43-2-24.pdf).
