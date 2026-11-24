# Adversarial review of the Yang Mills geometric identities

Bee Rosa Davis | Davis Geometric

23 November 2026 | Research note | Adversarial review, not a closed acceptance gate

## What I am keeping and what I am testing

I want to read the object, not only the number attached to it. That does not make
the number less important. It makes the identity of the object, the metric used
to measure it, and the test used to accept it more important.

The additions to [*Yang-Mills Requirements as Geometric Objects*](yang-mills-geometric-identities-2026-11-23.md) now reach beyond
the single plaquette: shared-link lattices, two constructions of the Hamiltonian,
a transfer-operator failure that must be visible, and a pre-registered study of
the lowest excitation. Those are concrete steps. This review asks whether the
claims, the measurements, and the acceptance machinery agree.

**My conclusion is to keep the finite geometric programme, but not to close its
verification gate yet.** The principal numerical controls reproduce. Several
acceptance rules still accept evidence that does not establish the claim beside
it. The proposed ring control also changes the available topological sectors;
that distinction must be measured before its gap is used to classify an
excitation as an edge or a band.

This review does not replace the original note or amend its pre-registration.
It records the corrections and additional gates that those documents need.

## The exact version and the scope of this review

The reviewed source is
[`b418ef79789769c9ea0ce2ae51906de0bcbc3141`](https://github.com/nurdymuny/gigi/tree/b418ef79789769c9ea0ce2ae51906de0bcbc3141),
including the ladder results, claims ledger, and excitation pre-registration.
The Lean sources inspected are
[`c06bed173d937263eae4b26c32644b11c0c59789`](https://github.com/nurdymuny/gigi/tree/c06bed173d937263eae4b26c32644b11c0c59789/docs/papers/berger-spectrum/lean/YMGeometry).
They were inspected, not independently rebuilt in this review: the required
local dependency cache was unavailable.

The new replay runs the complete single-plaquette script, lattice validations
V1--V5, the three-plaquette reflection test, and ten bounded ladder cases at
one to three plaquettes. It does not rerun the expensive four- and five-plaquette
production table. Those larger results are identified as the committed results,
not as a fresh independent replay.

The earlier cell replay covered all 4,320 cells with both ring constructions at
`63588263`. The cell script and receipt helper are unchanged between that revision
and this one. That earlier replay is retained separately; its results are not
presented as a new run. The frozen mesh inputs are not distributed by this note.

No new finite-coupling excitation profiles, ring-gap classifications, or scores
from the pre-registered study were computed here. A small electric-only ring
control tests the proposed comparison's premises without answering its physical
prediction. No Rust engine suite or live GIGI deployment was tested.

## What reproduced

The [new machine-readable review](../../research/ym-adversarial-20261123/review.json)
records source hashes, environment, numerical results, and the deliberately
corrupted fixtures. Its status is `ADVERSARIAL_FINDINGS_REPRODUCED_NOT_SCIENTIFIC_GATE_PASS`.

| Check | Fresh replay | What this establishes |
| --- | --- | --- |
| One plaquette, `g^2 = 1` | Gap `3.26909918552091`; radius `0.5530774493113728` at energy budget 1 | The submitted finite model and its radius conversion reproduce |
| V1, sampled link identities | Maximum gauge-generator discrepancy about `2.23e-14`; accepted tolerance `1e-12` | Agreement on the sampled configurations, not a universal roundoff bound |
| V2, single-plaquette spectra | Maximum difference about `7.11e-15` | Spin-network and tridiagonal spectra agree in the tested cases |
| V3, product-space comparison | 4, 11, and 23 physical levels; maximum spectral difference about `1.64e-12` | The three tested spectra agree within their `1e-9` tolerance |
| V4, gauge transformation samples | Maximum change about `2.70e-15` | The sampled ground-state amplitudes are gauge invariant to numerical precision |
| V5, interior plaquette commutators | Maximum about `6.67e-16` | The tested interior restrictions commute numerically |
| P5, Wilson coefficients | Bessel-ratio relative discrepancy about `4.22e-15` | Two calculations of the retained coefficients agree numerically |
| P5, negative-adjoint action | 290 of 590 kinetic weights negative; reflected minimum about `-1.14804e-7` | This preparation family detects the planted negative direction |

The quadrature emitted a roundoff warning at the requested tolerance. The Bessel
comparison is useful evidence, but neither it nor a small displayed discrepancy
turns that warning into a rigorous end-to-end error bound.

The earlier cell replay had a maximum recovery residual of approximately
`4.64e-14`. It also refused all 40 tested nonclosed-flux inputs. Its
[receipt](../../research/ym-adversarial-20261123/prior-rtc-replay-63588263.json)
is prior-run evidence, distinct from the new replay.

## Findings that prevent gate closure

### A1 The wrong energy shift still passes

In `scripts/ym_geometry_tests.py`, T8 stores an expected residual for changing
the ground energy by `1e-6`, but accepts the mutation whenever the measured
residual is noticeably nonzero. It does not compare the residual with the
expected signed value.

The audit changes only the applied shift to `1e-3`, leaving the stated expected
answer at `1e-6`. The submitted script still reports `ok: true`. The ratios of
measured to expected residual are approximately `1000.000007` and `999.999963`.
This is a false PASS for the note's claim that the break has exactly the
predicted amount, not a failure of the ground-state identity itself.

For \(x=\psi_0 f\), the necessary comparison is

\[
\langle x,(H-E_0-\delta)x\rangle-\mathcal D(f)
=-\delta\|x\|^2.
\]

**Repair gate:** compare both sign and magnitude with that prediction using a
declared cancellation/error allowance. An incorrect shift, an incorrect sign,
and a missing shift must each fail as separate fixtures.

### A2 An interrupted run leaves the preceding PASS in place

A planted exception before publication exits with code 1, correctly. However,
the preceding JSON remains unchanged with `ok: true`. The helper's timestamp
check is reached only if execution reaches the helper; it does not invalidate
an old receipt when the calculation fails earlier.

**Repair gate:** give each attempt a run identity and a durable current status.
Publish the completed receipt atomically. Preserve old successful results as
historical evidence, but never let them describe the interrupted attempt.
Test exceptions, process termination, and publication failure, including a
consumer that reads only the saved status. The audit records RUNNING before
its calculations and FAILED for caught exceptions; it is not an implementation
or a complete crash test of that proposed durability contract.

### A3 Numerical compatibility with positivity is not certification

The single-plaquette and lattice tests call a matrix `psd_certified` when its
least computed eigenvalue is above a small negative threshold. The threshold
is a multiple of machine epsilon, matrix size, and the largest matrix entry.
It is not a derived bound covering quadrature, operator construction, matrix
exponentials, powers, inner products, and eigensolution.

The exact matrix \(\operatorname{diag}(1,-10^{-15})\) is indefinite, but passes
the single-plaquette rule: its allowance is approximately `4.44e-15`. Even a
rigorous uncertainty interval crossing zero would not certify positivity.

**Repair gate:** distinguish `numerically_consistent`, `negative_witness`, and
`unresolved`. Reserve `certified` for an actual certificate with a stated exact
target and justified errors. For the intended operator, an exact factorization
can prove positivity independently of the floating-point implementation; do not
confuse that proof with certification of a computed matrix.

### A4 The volume check accepts an arbitrarily small positive gap

P2 tests only whether the minimum gap in the supplied finite table is positive.
In a replay of the committed fixtures through the actual result-aggregation
code, replacing every five-plaquette gap by `1e-12`, with its radius changed to
`1e6`, still produces an overall PASS. No new physics was simulated for this
witness; it isolates the acceptance rule with deliberately corrupted fixtures.

The rule correctly answers the narrow question "are these listed gaps positive?"
It does not test a useful lower bound, halving decrements, or uniformity over
unbounded sizes. The countermodel already present in the note, with gaps `1/N`,
explains why finite positivity cannot do that work.

P1 has a related scope issue: it accepts adjacent-cutoff changes below 1% for
`g^2 >= 2`. It does not enforce a `3e-4` claim. The committed five-plaquette
`g^2 = 2` change is `3.272356e-4`, and adjacent-cutoff agreement is not itself an
error bound to the infinite-cutoff value. The `g^2 = 1` results are excluded
from P1's acceptance predicate, including the disclosed 2.83% case.

**Repair gate:** label the present results finite-size and adjacent-cutoff
observations. If a lower-bound diagnostic is desired, predeclare it and require
a collapsing-gap fixture to fail. For a mathematical uniformity claim, prove
a bound independent of size with the required comparison maps; no finite list
or fitted decay ratio substitutes for that estimate.

### A5 Validation does not stop the physics stage

The lattice script prints V1--V5 statuses, then calls `physics` even if a
validation is false. The audit supplies a false V1 and verifies that execution
still enters the physics stage. The eventual final status would be false; the
defect is the promised fail-fast behavior, not a hidden successful final verdict.

**Repair gate:** require all validations before expensive physics or physical
result publication. Plant each validation failure separately and assert zero
physics calls and a failed current receipt. The current lattice receipt has
one `all_gates_caught` input, the negative-adjoint gate; it does not establish
the claim that every finite test has its own planted failure.

## Mathematical statements that need their hypotheses restored

### B1 The graph identity and its measure

For real symmetric weights, real potential, and a real eigenvector \(\psi\),
the complex-vector identity is

\[
\langle\psi f,(H-E)\psi f\rangle
=\frac{\kappa}{2}\sum_{i,j}w_{ij}\psi_i\psi_j|f_i-f_j|^2.
\]

If the right side instead uses \((f_i-f_j)^2\), restrict \(f\) to real vectors,
as the current Lean theorem does. For the two-site graph
\(H=\left[\begin{smallmatrix}1&-1\\-1&1\end{smallmatrix}\right]\),
\(\psi=(1,1)/\sqrt2\), and \(f=(1,i)\), the left side is 1, the square
without a modulus gives \(-i\), and the modulus-squared expression gives 1.

A sign-changing real eigenvector still defines the nonnegative measure
\(\nu_i=\psi_i^2\). For \((1,-1)/\sqrt2\), it is the probability measure
\((1/2,1/2)\). What can fail is the nonnegativity of the edge form.
To obtain the usual positive Dirichlet form and weighted Poincare statement,
state \(\kappa\ge0\), nonnegative symmetric weights, and a strictly positive
ground state; account for the kernel or disconnected components. Do not say
there is "no measure."

The radius statement also needs its conditions: a nonnegative homogeneous
energy, a positive budget, the specified vacuum-orthogonal space, and the sharp
lower-bound constant for the exact supremum radius. In infinite dimension that
supremum need not be attained. Quadratic scaling alone is not the whole theorem.

### B2 Reflection is about a central operator, not the parity of every exponent

The integer \(t_i+t_j-1\) is not always odd. For self-adjoint \(T\), the
useful identity is

\[
\langle x_i,T^{t_i+t_j-1}x_j\rangle
=\langle T^{t_i-1}x_i,T T^{t_j-1}x_j\rangle.
\]

There is one central, unsquared \(T\). That is why negative directions can be
detected. Site reflection, in the already-constructed positive Hilbert space
used by this finite test, produces a Gram matrix and cannot detect the sign
of \(T\). This is not a claim that site reflection positivity is generally
vacuous as a condition on Euclidean correlations.

The expanded P5 family includes every basis vector at time 1, so it does reach
the whole retained finite space. That is a real improvement. The Wilson
coefficient test and the Kogut--Susskind spectral calculation nevertheless use
different transfer constructions; agreement with a Hamiltonian-limit relation
is not tested merely by running both.

### B3 The formal and numerical coverage must be listed claim by claim

The inspected Lean source contains 17 `theorem` declarations, not 14. More
importantly, the finite-graph identity, sublevel-form equivalence, and
left-inverse bound have hypotheses that must not disappear in the claims ledger.
`exact_recovery` assumes \(R\circ X=I\); it does not establish that property for
the actual constrained least-squares construction. The spectral-theorem
connection and min-max result are still listed as future work in the note.

V3 compares spectra and state counts. It does not construct an identification
of the two Hilbert spaces and compare their eigenprojectors or observables.
"Spectra agree" is supported; "state for state" needs an explicit isometry,
Gauss residuals, and transported operator or projector comparisons. The two
implementations also share representation-theoretic helper code, so their
independence is partial and should be described accordingly.

**Repair gate:** keep one row per claim with its precise theorem, assumptions,
implementation check, planted failure, source revision, and remaining obligation.
The phrase "every EXACT statement has two independent checks" is not currently
supported by that inventory.

### B4 Keep the ruler beside the cell measurement

The reported `1.4e-12` gauge-change quantity is
\(\max|x_1-x_0|/\max(1,\max|x_0|)\), not an absolute coefficient bound.
The earlier replay reached about `1.435e-12`. The committed rival-curl ratio
reaches about `2.084e-12`, not a uniform `1e-13` bound. "Reproduced exactly"
should be "reproduced to numerical precision" across environments.

The 98% gradient share is the squared norm of the selected singular vector's
gradient coefficients in the chosen basis and coefficient metric. It is not
automatically a basis-independent physical-energy fraction. The exact metric
and its transport belong beside that claim.

The cell gauge test is the Abelian control. Raw non-Abelian curvature transforms
covariantly, \(F\mapsto gFg^{-1}\); gauge-invariant contractions or a declared
transport/frame comparison are required for a gauge-invariant reading.

## Amendments needed before the excitation study is scored

### C1 A ring removes ends and adds winding sectors

The periodic ladder contains a noncontractible cycle. In the spin-network basis,
choose a cross-section through one upper and one lower horizontal link and set

\[
z(s)=(-1)^{2j_{\mathrm{upper}}+2j_{\mathrm{lower}}}.
\]

Local plaquette operations preserve this parity. The small control on the
three-plaquette ring at `jmax = 1/2` has 16 physical basis states: eight in each
sector. Every cross-sector plaquette matrix entry is zero. In the electric-only
Hamiltonian the least odd-sector energy is \(9g^2/8\), whereas the least
nonzero even-sector energy is \(3g^2/2\). These are exact Casimir sums for the
represented states, not predictions of the full finite-coupling ring gap.

This exposes a confound in the proposed comparison. A lowest ring state in a
different center-flux sector cannot be created from the vacuum by the local
plaquette operators. Every corresponding transition amplitude is then zero.
An open-versus-ring gap ordering alone would not distinguish the proposed edge
and band mechanisms.

**Amendment:** label winding, momentum, reflection, and vacuum sectors. Report
the absolute gap separately from the lowest excitation accessible to the chosen
plaquette probes. Compare matched sectors and state which question is being
answered. Plant a lower, inaccessible winding state and require the analysis
to identify it, not classify it as a local edge or band. A zero-overlap family
must return "probe does not see this level," not a normalized profile.

This is not an objection invented to stop the ring test. Electric-flux sectors
are part of its geometry. Plaquette-chain work explicitly distinguishes the
sector without winding flux; see Ebner et al., Appendix A of
[the 2024 study](https://arxiv.org/html/2401.15184v4). The control above derives
the selection rule directly in the submitted basis.

### C2 A projector intensity is not an amplitude with its sign retained

For an excited eigenspace with projector \(P\), define the transition kernel

\[
C_{kl}=\langle O_k\Omega,P O_l\Omega\rangle,
\qquad O_k=\operatorname{tr}U_k.
\]

Its diagonal is the invariant intensity \(I_k=C_{kk}\). To compare an amplitude
envelope with the registered sine profile, use \(\sqrt{I_k}\). To compute the
edge weight, use \((I_1+I_p)/\sum_k I_k\), not squares of the intensities.
For a three-site sine amplitude the correct edge weight is `1/2`; treating its
intensities as amplitudes and squaring again gives `1/3`.

Diagonal intensities also discard sign and relative phase. The even profile
\((1,0,1)\) and odd profile \((1,0,-1)\) have identical intensities. The
pre-registration cannot recover parity or "all amplitudes have one sign"
from that diagonal alone. Keep the transition kernel and the action of the
reflection operator on the excited subspace; resolve parity there. For a
rank-one real state, relative signs survive in the off-diagonal kernel.

M3 needs a degeneracy rule too. One basis-independent choice for an exactly
degenerate level of rank \(r\) is
\(e_k=\operatorname{Tr}(P h_k)/r-\langle\Omega,h_k\Omega\rangle\).
Its sum is the gap if \(\sum_k h_k=H\). For a numerically grouped but split
cluster, the same sum is the cluster's mean excitation energy, not necessarily
the lowest gap. Report the distinction and the energy spread.

**Amendment gates:** plant a degenerate doublet and rotate its basis even if the
real experiment has no degeneracy. Check the full kernel, edge-weight formula,
parity resolution, local-energy sum, and zero-overlap refusal. Require at least
one valid, sector-matched ring comparison before any verdict whose definition
depends on rings; "every ring passes" must not be true merely because none was
measured. Compare gaps at matched, demonstrably adequate cutoffs, not only
profiles. A stable profile can coexist with a poorly converged energy.

### C3 State which field the proposed measurements actually read

The function \(\psi_1(U)/\psi_0(U)\), where the denominator is defined, lives on
the gauge configuration space. The list of plaquette transition amplitudes is
a spatial response profile extracted from that function, not the entire
function. It is useful, but generally not informationally complete.

For a positive ground state, an orthogonal real excitation has zero weighted
mean in the ground-state measure. Its configuration-space ratio must change
sign unless it vanishes. A node-free spatial transition envelope is not a
contradiction; it is a different object. Keep that distinction when describing
the experiment as reading the eigenfield.

The proposed Helike comparison also needs the same *geometric* space, not only
the same graph. The paper's meshes discretise \(S^3/2I\), the Poincare homology
sphere, not the full cover \(S^3\). A uniform combinatorial ladder Hamiltonian
does not automatically encode a Berger metric. Specify the quotient/cover,
metric weights, gauge sector, normalization, and a map between classical
tangent fields and the proposed quantum observables before comparing their
charges or shapes. This is a requirement for that future experiment, not a
claim that the present ladder code is defective.

## Literature and the strength of the claim

The Kogut--Susskind Hamiltonian is established lattice-gauge machinery; its
original construction is [Kogut and Susskind, 1975](https://doi.org/10.1103/PhysRevD.11.395).
Strong-coupling lattice results are relevant context, but the note itself says
it has not established that its chosen moderate couplings lie in the cited
theorem's range. The later statement that these numerical points occupy a
regime "where a gap is already proved" should not silently erase that condition.
The [Osterwalder--Seiler paper](https://doi.org/10.1016/0003-4916(78)90039-8)
provides the lattice positivity and strong-coupling context; it is not a
parameter-specific certificate for this calculation.

Nor does the literature stop at eigenvalues. Studies of SU(2) plaquette chains
already investigate eigenstates, entanglement, and local-operator matrix
elements: [Ebner et al., 2023](https://arxiv.org/abs/2303.14264) and
[Ebner et al., 2024](https://arxiv.org/abs/2401.15184). This does not settle whether
my particular geometric question has been answered. It does mean that "what
is not measured" and "the reading is not known" require a narrower comparison
with prior work. I can state what I intend to measure without claiming that
other researchers have only looked at numbers.

## The next gate

I would close the current review in this order:

1. Correct the graph, reflection, normalization, coverage, and literature wording.
2. Repair the shift check, current-receipt lifecycle, positivity statuses, and
   validation short circuit; demonstrate each negative fixture actually fails.
3. Separate finite-size observations, cutoff diagnostics, and uniform estimates.
   Publish a claim-to-test ledger rather than one omnibus PASS.
4. Amend the excitation pre-registration before measuring its new observables:
   sector matching, projector kernel, parity, degeneracy, zero overlap, and
   adequate ring/cutoff coverage. Preserve the original registration and list
   exactly what information was already known when each amendment was made.
5. Only then run and score the excitation study. Report the observed geometric
   structure even if none of the registered readings survives.

I am keeping the question open because it is a good question to test, not
because an affirmative result is owed to it. A smaller statement that survives
an adversarial reading is something I can build on.

## Reproduction and evidence

Run from a checkout containing this review and the unchanged reviewed source:

```powershell
$env:OPENBLAS_NUM_THREADS = '1'
$env:OMP_NUM_THREADS = '1'
$env:MKL_NUM_THREADS = '1'
python scripts/ym_geometry_adversarial_review.py output/ym-adversarial-replay
```

The script refuses source files that differ from its pinned revision. It writes
to the supplied output directory and never patches the submitted calculations
on disk. Deliberate replacements occur only in isolated in-process calls or
child processes. The replay uses Python 3.12.10, NumPy 2.4.0, SciPy 1.16.3, and
SymPy 1.14.0. Numerical last digits can vary across libraries and machines.
Text hashes normalize Windows CRLF to LF, matching the committed Git blobs.

- [Audit implementation](../../scripts/ym_geometry_adversarial_review.py).
- [Review receipt and source hashes](../../research/ym-adversarial-20261123/review.json).
- [Fresh unmodified plaquette receipt](../../research/ym-adversarial-20261123/plaquette-baseline.json).
- [Execution log](../../research/ym-adversarial-20261123/replay.log).
- [Deliberately wrong shift accepted by the submitted script](../../research/ym-adversarial-20261123/wrong-shift-submitted-receipt.json).
- [Deliberately collapsed gap accepted by the submitted reducer](../../research/ym-adversarial-20261123/collapsed-gap-reducer-receipt.json).
- [Earlier cell replay, explicitly historical](../../research/ym-adversarial-20261123/prior-rtc-replay-63588263.json).

The corrupted receipts are counterexample artifacts, not scientific results.
Their embedded PASS fields are precisely the behavior being challenged. The
review uses automated computation and adversarial probes; it does not represent
independent human peer review. Authorship and responsibility for this research
note are mine.
