# Helike as a probe of a published quantum Yang-Mills model

**Bee Rosa Davis | Davis Geometric**  
23 November 2026. Research specification, version 1.

**Status:** experiment specified; interacting quantum spectrum **NOT RUN**.
The exact coordinate and probe identities below have been checked symbolically.
Those checks are not measurements of a quantum gap.

## 1. The experiment I want

I want to take the field I found and use its geometry to ask an existing
Yang-Mills model what its lowest excitation is. I am not going to build the
answer into the model. Its Hamiltonian comes from a published source. Helike
supplies the probe; an unrestricted calculation within the same finite quantum
basis supplies the comparison.

There are three questions, with three separate answers:

1. Does the translation preserve her actual geometric direction?
2. Does a probe built from that direction couple to the model's lowest physical
   excitation, or only to a higher one?
3. Does starting from her geometry help recover that excitation efficiently?

The first is an exact algebra question. The second and third are experiments.
They are not answered by putting her classical Hessian eigenvalue into a
quantum energy column.

## 2. The external model

Use the SU(2) specialization of Balachandran, Vaidya and de Queiroz,
[*A Matrix Model for QCD*, arXiv:1412.7900v1](https://arxiv.org/abs/1412.7900v1),
equations II.6--II.11; journal DOI
[10.1142/S0217732315500807](https://doi.org/10.1142/S0217732315500807).
It is a published homogeneous quantum reduction on the round three-sphere,
not a proposed Hamiltonian of my own. Its nine real coordinates are the entries
of a matrix M: spatial direction indexes rows; colour direction indexes columns.

Write S = M M^T and freeze the Hamiltonian as

\[
 H_{g,R}=\frac1R\left[-\frac{g^2}{2}\Delta_M+V_g(M)\right],\qquad
 V_g(M)=\frac{\operatorname{tr}S-6\det M+
 \tfrac12((\operatorname{tr}S)^2-\operatorname{tr}(S^2))}{2g^2}.
\]

The overall 1/R multiplies **both** terms. This follows II.8 and the explicit
Hamiltonian in Acharyya and Balachandran,
[*Matrix Model of QCD: Edge Localized Glue Balls and Phase Transitions*,
arXiv:1702.06430v2](https://arxiv.org/abs/1702.06430v2), equation 2.14.
Do not infer an inconsistent radius convention from II.9 in the first paper.

This is a named finite-mode Yang-Mills model, not the full four-dimensional
theory and not a claim of universal agreement about a reduction. It is chosen
because its homogeneous sphere connections admit an explicit map from Helike.

### Our frozen experimental choices

- R = 1, hbar = 1; report energies in units of 1/R. Couplings are g^2 = 1
  (primary), then g^2 = 1/2 and 2 (replications); g is positive.
- Use the cover S^3, not S^3/2I. Keep the sphere round: t = 1. No Berger
  deformation is inserted into the external Hamiltonian.
- Use the regular Cartesian realization on L^2(R^9, dM), with the Friedrichs
  closure of the nonnegative energy quadratic form initially on smooth compactly
  supported functions, restricted to colour singlets.
- Colour acts as M -> M R_c^T, R_c in SO(3). A physical wavefunction satisfies
  psi(M R_c^T) = psi(M); equivalently all three Gauss generators annihilate it.
- Keep every spatial rotation sector present in the finite basis. Colour
  singlet does **not** mean spatial scalar.
- No theta term, added mass, fitted energy offset, or extra boundary interaction.
  No identification of distinct matrices by an unimplemented large gauge map.
  In particular M = 0 and the orbit of M = I remain in the Cartesian domain.

These domain choices define the experiment. The second paper studies alternative
boundary extensions after reduction. We do not use those extensions or their
edge-state spectra as benchmarks. The first paper's colour-carrying variational
excitation is not a singlet reference answer either.

Do not replace det M by its absolute value, or identify SO(3) gauge invariance
with O(3) invariance. That would change the cubic term and the Hilbert space.

## 3. Exact map from the existing classical work

Local sources are the [Floor paper](../papers/berger-spectrum/paper.tex),
[the family identity](what-she-is-a-law-of-2026-11-21.md), and
[`ricci_su2_resting_fields.py`](../../scripts/ricci_su2_resting_fields.py).
The source snapshot for this specification is
`ed5d3cf5209bb8cecd6d97c4f0df3065af90363e`.

In the local round-sphere convention, frame brackets contain 2 epsilon while
Lie-algebra brackets contain epsilon. If m has the local connection coefficients,

\[
 E_{\rm local}(m)=\frac12\sum_{i<j}
 |m_i\mathbin\times m_j-2\epsilon_{ijk}m_k|^2,
 \qquad m=2M,
 \qquad E_{\rm local}(2M)=16g^2 V_g(M).
\]

This last equality is a polynomial identity in all nine entries, not a fit along
one path. The sign of the curvature vector relative to the paper's anti-Hermitian
matrix convention is recorded through II.6; the squared energies agree.
For the Hessians, the same change of coordinates gives

\[
 D_m^2 E_{\rm local}(2M)=4g^2 D_M^2 V_g(M).
\]

This is a map of classical coefficients and energies, not an asserted equality
of two quantum Hamiltonians. Quantum energies use the external kinetic term and
its R convention throughout.

The homogeneous member j = 0 of Helike is the complex tensor
(theta^1 + i theta^2) tensor (tau_1 + i tau_2). Its two orthonormal real directions
in row-spatial, column-colour convention are

\[
 B_c=\frac1{\sqrt2}
 \begin{pmatrix}0&0&0\\0&1&0\\0&0&-1\end{pmatrix},\qquad
 B_s=\frac1{\sqrt2}
 \begin{pmatrix}0&0&0\\0&0&1\\0&1&0\end{pmatrix}.
\]

The unit complex tensor is (B_c + i B_s)/sqrt(2). The plus sign follows the
paper's stated coframe and internal basis. Both quadratures are retained; a
change to the conjugate convention must transform both, not silently flip one.

At the canonical round connection m_* = I, hence M_* = I/2, the local formula
gives the classical eigenvalue 10. In the external potential convention this is
5/(2g^2), on both displayed directions. This is a calibration at a classical
saddle, **not** a claim that M_* is the quantum vacuum. The vacuum will be solved
for over the whole declared configuration space.

Only j = 0 is represented here. The exact eigenfield extends to t = 1, but the
Berger floor theorem on t in [2.75,10] is not a theorem about this round-sphere
quantum experiment. No numerical floor or spin label is transferred by name.

## 4. A probe which actually preserves the direction

A fixed colour-labelled linear coordinate is not a physical singlet observable.
Instead use the gauge-invariant spatial Gram matrix S = M M^T and define

\[
 O_c(M)=\operatorname{tr}(B_c S),\qquad
 O_s(M)=\operatorname{tr}(B_s S).
\]

Their gradients at M_* = I/2 are exactly B_c and B_s:

\[
 dO_a|_{M_*}(\delta M)=\langle B_a,\delta M\rangle_F,
 \quad a=c,s.
\]

Thus their first response is Helike's direction, not a visually similar
substitute. In local m coordinates the corresponding observable is
one-half tr(B_a m m^T); its gradient at m_* is B_a too. The constant difference
between the two observable normalizations cancels in normalized spectral weights.

Rotating the spatial frame completes B_c and B_s to the five-dimensional space
of symmetric trace-free 3 by 3 matrices. Fix the full orthonormal basis as

\[
 B_1=B_c,\ B_2=B_s,\quad
 B_3=(E_{01}+E_{10})/\sqrt2,\quad
 B_4=(E_{02}+E_{20})/\sqrt2,\quad
 B_5=\operatorname{diag}(-2,1,1)/\sqrt6.
\]

Here E_ij has a single 1 in row i, column j. Define O_A = tr(B_A S) for all five.
No direction is fitted to the independently computed excited states.

### Two channels, declared before the spectrum is seen

**H2: directional Helike channel.** The five O_A form a spatial spin-2 tensor.
From a spin-0 vacuum they can reach spin 2, not a spin-0 excitation. This exact
selection rule will be checked rather than discovered after a disappointing run.
The quantum spin J = 2 is not the classical spatial-harmonic index j = 0.

**H0: scalar Helike-derived channel.** Independently test the strength of this
tensor, without choosing a direction:

\[
 O_0(M)=\sum_{A=1}^5 O_A(M)^2
       =\operatorname{tr}(S^2)-\tfrac13(\operatorname{tr}S)^2.
\]

This is a quartic scalar composite of her rotational orbit, **not** the original
linear eigenfield. It can reach spin 0. Its result is labelled separately even
if it succeeds where H2 does not. Neither channel can be advertised as detecting
every symmetry sector. The independent spectrum decides what lies lowest.

For computation use q = M/g. Divide O_A(M) by g^2 and O_0(M) by g^4, so their
polynomials are simply the same formulas in q. This is a declared amplitude
normalization, not a change to the states or to the Hamiltonian.

## 5. The quantum comparison

Let Omega be the independently computed normalized ground state, E_0 its energy,
and P_n the projector onto a complete excited energy level. Let Q remove the
entire ground-state subspace. If the ground-state multiplicity is unresolved,
stop scoring rather than choosing a convenient vacuum vector.

For each observable set phi_A = Q O_A Omega. Record its norm before normalizing.
For either channel define

\[
 W_n=\frac{\sum_A\|P_n\phi_A\|^2}{\sum_A\|\phi_A\|^2},\qquad
 \Delta_{\rm abs}=E_1-E_0.
\]

The H0 sum has one member. The H2 sum has five. Also retain the full transition
kernel K_AB(n) = <phi_A, P_n phi_B>, not just its diagonal. This preserves phase
and orientation information without depending on a chosen eigenvector inside
a degenerate level.

The probe-accessible gap is the least E_n - E_0 with nonzero weight. Report it
beside the absolute gap, never instead of it. Exact symmetry zeros, numerically
unresolved overlaps, and robustly measured overlaps are different outcomes.

An independent propagator calculates

\[
 C_{AB}(s)=\langle\phi_A,e^{-s(H-E_0)}\phi_B\rangle
         =\sum_{n>0}e^{-s(E_n-E_0)}K_{AB}(n).
\]

Use s = 0, 1/4, 1/2, 1, 2, 4. Spectral sums and matrix-exponential actions must
agree. Effective decay rates are reported as functions of s; one short-time
slope is not declared to be the gap. A Rayleigh quotient is an expectation,
not automatically the smallest energy.

## 6. Independent reference and bounded numerical construction

In q coordinates the frozen Hamiltonian at R = 1 is

\[
 H=-\tfrac12\Delta_q+\tfrac12\operatorname{tr}(q q^T)
       -3g\det q+\tfrac{g^2}{4}
       [(\operatorname{tr}(q q^T))^2-\operatorname{tr}((q q^T)^2)].
\]

Both the determinant and quartic terms remain in every interacting run.

Use all gauge-invariant polynomials of degree at most N times
exp(-tr(q q^T)/2), not a Helike-only trial space. One candidate spanning set is
monomials in the six entries of q q^T, multiplied by det(q)^epsilon with epsilon
equal to 0 or 1 and weighted degree 2 sum(n_i) + 3 epsilon <= N.
The identity (det q)^2 = det(q q^T) removes duplicate generators.

N = 4, 6, 8, 10 gives predicted dimensions 29, 91, 238, 546. These are
combinatorial predictions, not measured spectrum sizes. Retaining all six Gram
entries is essential: diagonal or spatial-invariant coordinates alone would
remove the spin-2 response we are testing.

**Reference construction:** exact Gaussian moments, analytic polynomial
derivatives, the full overlap and Hamiltonian matrices, then generalized
diagonalization. Keep the determinant-odd basis states and their cubic couplings.
Check conditioning explicitly; do not discard small Gram eigenvalues without
an exact dependence proof. Use higher precision if orthogonalization requires it.

**Independent small-basis check:** construct the three Gauss generators on the
nine-coordinate oscillator space at N = 4, find their common kernel, and assemble
the Hamiltonian by oscillator raising/lowering operations. Its singlet dimension
and entire projected spectrum must agree with the invariant-polynomial route.
This route must not import the first route's Hamiltonian or Gram entries.
Prove the spanning rule, and check its level-by-level counts, before calling
the higher-cutoff reference complete within its truncation.

**Probe construction:** build the probes and their Krylov spaces without reading
the reference excited eigenvectors. The reference vacuum is permitted input and
its computational cost is reported separately. This first experiment tests
excitation detection given a vacuum; it does not yet supply a cheaper vacuum solver.

Every finite-basis observable is P_N O P_N. For H0 assemble the quartic polynomial
first and then project. In general P_N(sum O_A^2)P_N differs from
sum(P_N O_A P_N)^2; the latter is forbidden here.

### Numerical policy fixed before the runs

- Exact identities must simplify to zero; no floating tolerance substitutes.
- Independent finite matrices and spectra: relative/absolute hybrid error 1e-10.
  Hermiticity, orthogonality, Gauss and finite-matrix eigen-residuals: 1e-10.
- Group energy levels using 1e-8 max(1, |E|), retain their spread and residuals,
  and refuse ambiguous grouping. Projectors, not arbitrary basis vectors, are scored.
- Report all finite-cutoff gaps. A model-level numerical estimate additionally
  requires two consecutive cutoff increments with E_0, E_1 and each quoted gap
  changing by <= 1e-3 max(1, |quantity|), and channel weights changing by <= 0.01.
- Measure the norm of H psi outside the basis using polynomial moments through
  degree N + 4. Require <= 1e-2 max(1, gap) for quoted low states. Measure
  discarded probe norm too; its squared fraction must be <= 1e-3.
- These convergence diagnostics are not rigorous error enclosures or a proof
  against every possible state beyond the cutoff. Use the label
  `numerically_converged_model_estimate`, never `certified_gap`.
- Weight >= 1e-4, stable under the above checks, counts as a robust practical
  detection. Smaller measured weight is reported, not rounded to zero; only
  an exact selection rule establishes invisibility. This threshold is an
  experimental usefulness criterion, not a new physical constant.
- A probe gap matches the independent gap when their difference is <=
  1e-3 max(1, |gap|). This is scored only after the construction and convergence
  gates pass, never by accepting a plausible plot.

If N = 6, 8, 10 do not settle the comparison, N = 12 (predicted dimension 1134)
is the only pre-authorized extension, subject to the same resource ceiling.
Failure to converge is `UNRESOLVED`, not evidence against Helike.

## 7. Controls that must fail in the right way

The noninteracting oscillator is a solver control, not a replacement model.
Switch off the cubic and quartic terms only in that explicitly labelled control.
Then the nine-coordinate oscillator has E_0 = 9/2. Within colour singlets its
first excitation has gap 2, with one spatial scalar and a spin-2 quintet.
Both proposed channels have nonzero overlap with that level. Derive these
statements by oscillator algebra in the test, and check the full multiplicity.
Do not assert that this control is the global weak-coupling limit of the
multi-well interacting model.

| Gate | Required positive check | Deliberate defect that must be rejected |
| --- | --- | --- |
| HP00 Source | Versioned Hamiltonian, domain, units and source hashes fixed | Changed cubic sign; missing R on potential; unsigned determinant |
| HP01 Translation | Exact all-entry energy identity, Hessian scaling, both quadratures and observable gradients | m = M; omitted cubic term; flipped imaginary quadrature |
| HP02 Gauge | Right-colour invariance and Gauss annihilation; correct spatial covariance | Use fixed coloured linear coordinate; omit one Gauss generator |
| HP03 Physical basis | N = 4 independent kernel, spectrum and counts agree; spanning rule established | Drop determinant sector, off-diagonal Gram entries, or a low spin sector |
| HP04 Oscillator | E_0 = 9/2, physical gap 2, complete six-state level and both probe overlaps | Keep a colour-carrying one-quantum state; count only one member of a multiplet |
| HP05 Honest probe | Absolute versus accessible gaps, vacuum subtraction, H2 spin rule | Plant a lower scalar state invisible to a tensor; leave vacuum component; normalize zero norm |
| HP06 Geometric response | Complete-level kernels invariant under basis change; both quadratures retained | Rotate a planted degenerate doublet; inspect one eigenvector only; conjugate one component |
| HP07 Projection | Direct quartic projection and finite-spectrum/propagator agreement | Square already projected quadratic operators; use one decay slope as an eigenvalue |
| HP08 Numerical reliability | Cutoff, conditioning, residual, tail and grouping rules all met | Stable gap with drifting E_0 and E_1; omitted low state; ill-conditioned Gram solve |
| HP09 Utility | Fixed-budget probe versus matched controls, all work counted | Hide vacuum cost; use exact excited vector as seed; report only best seed or coupling |
| HP10 Evidence | Unique run manifest, RUNNING first, atomic terminal receipt, verifier consumes this run only | Old PASS beside a new failed run; corrupted hash; missing gate; partial run; timeout |

Each negative fixture has exactly one required verdict. A scientific outcome
such as `TENSOR_MISSES_LOWEST_LEVEL` is distinct from a failed construction gate.
Removing the mechanism under test must turn its gate red. Passing a test which
accepts either a detection or a miss is not evidence for either.

At registration all HP00--HP10 gates remain **NOT RUN** as complete gates.
The checked symbolic identities are preliminary components of HP01/HP02;
they do not close the quantum experiment's gates.

## 8. What would count as useful

For each coupling publish separate H2 and H0 rows containing the absolute gap,
lowest-level spin and multiplicity, accessible gap, lowest-level weight,
transition kernel, cutoff changes and residuals.

Then test block Krylov spaces generated by QHQ from the declared probe states,
using total Hamiltonian-vector product budgets 8, 16, 32 and 64. Count each
column action, not each block call. A recovered level must meet the gap and
residual thresholds without access to the reference excited eigenvectors.

Use a scalar trace probe tr(q q^T) as a plain baseline, and fixed random physical
seeds matched to each channel's spin, number of independent starts and allowed
oscillator degrees. Seeds are 11, 23, 47, 89 and 173. The deterministic probes
are fixed before diagonalization; random controls are generated independently
of the excited spectrum. All seeds are reported.

An efficiency advantage requires at least a factor of two fewer vector products
to the same accuracy against the median matched random control, at every
coupling with converged reference data. A failed or unfinished comparator cannot
supply an advantage. Ordinary successful detection remains a separate useful
result even if there is no speed advantage.

The possible findings are concrete:

- **H2 detects the lowest level:** her directional channel reaches the cheapest
  excitation of this model at the tested coupling.
- **H0 detects it while H2 does not:** the scalar composite works; the original
  directional claim does not. The selection rule identifies what changed.
- **Only a higher level is reached:** identify its spin, geometry and energy;
  do not rename it the floor.
- **No robust detection:** report small or forbidden overlaps and the independent
  lowest level. No post-hoc change of Hamiltonian, probe or coupling.
- **UNRESOLVED:** the numerical or resource gate did not permit a conclusion.

These labels describe numerical findings in this model. A positive result is a
reason to extend the experiment to more spatial modes, not a claim that those
modes have already been tested.

## 9. Work order and receipts

1. Preserve this registration, source versions and the preliminary algebra check.
2. Implement the gates with planted failures, then the independent N = 4
   constructions and oscillator control. Do not run the interacting survey if
   either construction disagrees.
3. Run the primary coupling; only then the two predeclared replications. Keep
   all cases, including misses and unfinished cutoffs, in the report.
4. Score the two channels, then the computational comparison. Publish the raw
   evidence and the geometry of the detected level, not just its eigenvalue.

Resource ceiling for the first interacting survey: 45 minutes total, 8 GiB peak
resident memory, one process, no full GIGI suite. Record wall time and memory
for each stage. Exhausting the ceiling ends the survey with a partial receipt;
it does not silently trigger another method or a longer job.

Proposed evidence root: `research/helike-quantum-probe-20261123/`.
Each future run gets an immutable subdirectory containing source hashes,
Hamiltonian/domain fingerprint, basis counts, condition diagnostics, gates,
matrices or reconstruction instructions, spectra, kernels, convergence history
and command transcript. Numerical arrays carry shape and units. Source changes
after registration require a dated amendment naming any results already seen.

No new GQL verb or deployed endpoint is claimed here. GIGI can later retain and
query the evidence, but storage does not replace the independently verified
quantum calculation.

### Preliminary algebra receipt

The reproducible check is
[`ym_helike_probe_contract.py`](../../scripts/ym_helike_probe_contract.py).
It checks 15 exact identities, without constructing a quantum spectrum:

```text
python scripts/ym_helike_probe_contract.py --out research/helike-quantum-probe-20261123/algebra-contract.json
python scripts/ym_helike_probe_contract.py --inject-defect missing_frame_scale
python scripts/ym_helike_probe_contract.py --inject-defect drop_cubic
python scripts/ym_helike_probe_contract.py --inject-defect flipped_imaginary
python scripts/ym_helike_probe_contract.py --inject-defect coloured_probe
```

The baseline returns exit 0; each defect returns exit 1. The baseline receipt is
[here](../../research/helike-quantum-probe-20261123/algebra-contract.json), and
the four rejected variants are beside it as `rejected-<defect>.json`.
All 15 baseline checks passed and all four defects were rejected on 23 November
2026, using SymPy 1.14.0. Receipts record the script and specification hashes.
These are exact algebra and fault-injection checks, not the complete HP gates
or evidence that the interacting probe reaches the lowest excitation.

## 10. In plain English

The published model supplies the energy rules. We leave those rules alone.

Helike supplies a particular way the field can change. We have an exact
translation that turns that shape into a permitted observable, rather than
merely drawing her next to an unrelated calculation.

One calculation finds the model's lowest excitation without being told to
look like her. Another asks which excitations her shape actually reaches.
We compare the answers. If they match, we will know precisely what worked.
If they differ, we will have the missing level and its geometry in front of us.
That is the experiment.
