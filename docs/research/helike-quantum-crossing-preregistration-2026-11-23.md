# Confirmatory crossing experiment: follow the states, not just the gap

23 November 2026, Pacific time. Specification v1. **REVIEW HOLD. NOT IMPLEMENTED. NOT RUN.**

This commit specifies the experiment and its tests. It does not authorize a run. Review this commit on the other
machine before implementation or execution. All gates below are NOT RUN; a test definition is not a test receipt.

## 1. The question

I want to know whether Helike's directional channel becomes the lowest excitation because two distinct families
of states exchange order. The energy tells me which is cheaper. The state tells me what that excitation is.
I want both, measured together, without mistaking a change of numerical basis for a change in the field.

The exploration suggests a scalar-to-spin-2 switch near coupling-squared 1.76. This experiment asks three
separate questions:

1. **Crossing:** do the lowest scalar excitation and the lowest spin-2 excitation exchange order at held-out
   couplings, with converged numerical evidence under the existing policy?
2. **Geometry:** how do their size, oriented volume, anisotropy, fluctuations and subspaces change? Does the
   scalar retain a larger signed-volume expectation than the tensor across the crossing?
3. **Numerical obstruction:** which polynomial shells generate the residual, including their interference?
   A large coefficient in one shell must not be mistaken for a measured cause of the error.

The geometry measurements are required outputs, not decorative illustrations of the eigenvalues. A failed
geometry hypothesis is useful evidence; it is not grounds to relabel or discard the measured states.

## 2. What is already known and what is held out

Exploration source: `ba8bf3efd86d2513258c4f4bb0dad401e46cf19e`.
The [exploratory note](helike-quantum-edge-exploration-2026-11-24.md) and its historical filename are retained.
The saved scan contains 22 cases and 176 reported levels at N=10,12. Its inspected couplings were
`0.5, 0.75, 1, 1.25, 1.5, 1.75, 2, 2.5, 3, 4, 6`.

| Historical artifact | SHA-256 at the exploration commit |
| --- | --- |
| `scripts/explore_ym_helike_edge.py` | `bd3a45915a153e3e3dd5330ce962a1fc77af11fc67aa721f78572cd455281d7a` |
| `research/helike-quantum-probe-20261123/exploration-edge-20261124/edge_scan.json` | `03d471f38e01946dd7ce39365da9fccb100097853ec2424a35784c35c17c4c1f` |
| `research/helike-quantum-probe-20261123/exploration-edge-20261124/levels.csv` | `f70604e0af1b8e7a363d3187c734f833eace71b07acca2727917cae6a7e6720f` |

The registration gate independently hashes these historical artifacts; this document is not a substitute for
that check. A later documentation-only change to the exploration script does not rewrite its original hash.

Recomputed linear interpolation gives 1.7699786411 at N=10 and 1.7609417618 at N=12: a shift of 0.0090368794.
These values informed this design. They are not independent predictions, measured roots, or confidence limits.
The earlier residual-versus-width assertion lacks a retained table and is not used to select the next method.

**Held out means absent from the saved exploration and known earlier surveys, not statistically independent
data.** This is a deterministic numerical replication within the same model. If any proposed point has already
been inspected elsewhere, record that exposure before launch and amend the registration before viewing new
results. Do not silently rename exposed data as held out.

## 3. Fixed model and numerical design

Inherit the [original registration](helike-quantum-probe-preregistration-2026-11-23.md),
[GPU amendment](helike-quantum-gpu-amendment-2026-11-23.md) and
[convergence amendment](helike-quantum-convergence-amendment-2026-11-23.md), except for the new coupling set and
explicit width grid below. This is a new confirmatory experiment, not a retroactive change to earlier runs.

The external model remains the homogeneous SU(2) sphere matrix model of
[Balachandran, Vaidya and de Queiroz](https://arxiv.org/abs/1412.7900v1), with the overall radius convention checked
against [Acharyya and Balachandran, equation 2.14](https://arxiv.org/abs/1702.06430v2). Keep the registered Cartesian
Friedrichs domain, colour singlets, every available spatial spin sector, R=1, hbar=1, and no boundary extension.
Do not import the second paper's boundary-localized states or a Berger deformation into this experiment.

In computational coordinates q=M/g, let T=tr(qq^T). The fixed operator is

\[
 H=-\tfrac12\Delta_q+\tfrac12 T-3g\det q
   +\tfrac{g^2}{4}\big(T^2-\operatorname{tr}[(qq^T)^2]\big).
\]

No terms, physical sectors, coordinate orientation or probe normalization may change. Coupling varies; radius
does not. No running-coupling relation between g and sphere size is inferred. Quantum J=2 remains distinct from
the classical spatial-harmonic index j=0. Increasing this basis does not add the omitted spatial field modes.

| Item | Frozen choice |
| --- | --- |
| Held-out g squared | `1.60, 1.70, 1.74, 1.78, 1.82, 1.90` |
| Confirmatory cutoffs | `12, 14, 16`, all six couplings: 18 cases |
| Width-selection cutoff | `10`, ground energy only |
| Width grid | `0.8, 1.0, 1.2, 1.35, 1.5, 1.7, 2.0, 2.3, 2.7, 3.2, 4.0, 5.0` |
| Width rule | Minimum computed E0; exact ties choose smaller alpha; publish all energies and the runner-up difference |
| Width reuse | Freeze one alpha per coupling across all three cutoffs and all states |
| Grid boundary | Record it; refuse confirmatory launch for that coupling, no automatic grid extension |
| Prediction bracket | Crossing within `[1.70, 1.82]` |
| Outer checks | Scalar lower at `1.60`; tensor lower at `1.90` |
| Fine-bracket checks | `1.74` and `1.78`; secondary resolution, not permission to move the primary bracket |
| Main solver | CUDA float64/complex128, no automatic CPU fallback |
| Budgets | 45 minutes per complete registered attempt, 8 GiB worker host RSS, 4 GiB CUDA allocation |

The width grid comes from exploration; the new coupling measurements do not. Freeze the full ground-energy
selection receipt before inspecting excited-state outputs. Publish the width margin and flag a near tie under
`1e-10 * max(1, abs(E0))`; the deterministic minimum rule still applies. No excited energy, response, residual
or desired crossing location can choose alpha. No additional bisection points or extended cutoffs in this run.

Keep the original criteria: Hermiticity, metric orthogonality and finite-solver checks at 1e-10; energy grouping
at 1e-8 times max(1, absolute energy); Casimir and probe-spin residuals at 1e-6; full-space residual at
`0.01 * max(1, absolute_gap)`; discarded squared probe fraction at most 0.001; energy/gap changes at most
`0.001 * max(1, abs(new_value))` and normalized channel-weight changes at most 0.01 for BOTH 12->14 and 14->16.
Robust detection still requires weight at least 1e-4. These are numerical policies, not physical constants.

## 4. Identify the branches before asking which is lower

Solve the complete finite singlet problem. Do not restrict the computation to H0/H2-responsive vectors or to
the first eight levels. Save every finite eigenvalue, all spin labels and complete low-level projectors.

Define E_s as the lowest excited spin-0 energy and E_t as the lowest spin-2 energy, excluding the vacuum.
Track these sector branches, not fixed sorted-eigenvalue indices. Define

\[
 \Delta_s=E_s-E_\Omega,\qquad \Delta_t=E_t-E_\Omega,\qquad D=E_t-E_s.
\]

D positive means scalar lower; D negative means tensor lower. The ground energy cancels in D. A planted fixture
must verify this sign convention. Check the unrestricted first excitation independently: another spin sector
below both branches invalidates the claim that this is a switch of the absolute excitation floor.
The two branches exchange order; this does not mean one state turns from scalar into tensor.

At a coincident or nearly grouped scalar/tensor level, diagonalize J squared within the full energy block and
retain both spin subspaces. Then resolve the restricted Hamiltonian within each spin subspace. Never discard a
mixed-spin block, average its spin, or force a five-dimensional tensor plus scalar into one label. Preserve
energy spread and sector projectors. Repeated unresolved copies of the same spin are an explicit ambiguity;
do not pick one arbitrary copy. A non-scalar or unresolved vacuum blocks this experiment's probe scoring.

For each selected subspace U, in orthonormal oscillator coordinates, U-dagger U=I. Store its projected energy
matrix B=U-dagger H U and full residual R_U=H U-U B. Use the operator norm of R_U for a multiplet, not one chosen
member's norm. Include the subspace energy spread in the reported residual radius. Save both quantities.

## 5. What counts as a resolved crossing

Let epsilon_s and epsilon_t be the full residual norms plus their respective energy-spread allowances.
Define the numerical separation allowance

\[
 u_D=\epsilon_s+\epsilon_t+10^{-8}\max(1,|E_s|,|E_t|).
\]

At each point, D > u_D is `SCALAR_LOWER`, D < -u_D is `TENSOR_LOWER`, otherwise `ORDER_UNRESOLVED`.
These floating residual radii are not certified error bounds or a proof that all exact levels are enclosed.
Their purpose here is to refuse an order claim when the measured equation mismatch can obscure it.
The sign test is additional; it cannot waive the old residual or convergence gates.

**Primary numerical support requires all of the following:**

- All 18 cases and all prerequisite gates complete, with both successive convergence checks at every coupling.
- At 1.60 and 1.70, `SCALAR_LOWER` at all three cutoffs; at 1.82 and 1.90, `TENSOR_LOWER` at all three cutoffs.
- The full finite spectrum has no lower competing excitation at these points. Its audit is not replaced by
  inspecting only the two channels.
- The ground, scalar and tensor state-geometry measurements pass the consistency and stability gates below.
- The exact planted positive and negative controls produce their independently specified outcomes.

The interior points refine a bracket only if they separately resolve. Their unresolved order is reported as
`fine_bracket_status: ORDER_UNRESOLVED`; it does not alter the predefined primary test. No scalar/tensor spin
assignment or geometry measurement may be omitted because these two points are close to degeneracy.

Report a primary bracket, not a unique critical coupling or phase transition. Linear interpolation, if displayed,
is secondary and labelled as such, with both endpoint data and cutoff dependence. Sampled signs do not exclude
multiple crossings inside the bracket or an upper crossing elsewhere. No confidence percentage is manufactured
from three deterministic cutoffs. Continuity and uniqueness of exact sector branches are not proved here.

## 6. Geometry of a quantum state

A state here is a wavefunction on nine-dimensional matrix configuration space, not one classical arrow or one
matrix. Use the complete-level density rho=P/r, where P is its orthogonal projector and r its rank. For a
nondegenerate state this is the usual pure-state expectation. For a multiplet this avoids choosing a direction
that exists only because the eigensolver picked a basis.

Use physical M=gq and G=M M^T. Colour acts on M's right; spatial rotations on its left. Define three scalar
measurements of each configuration:

\[
 F_R=\tfrac13\operatorname{tr}G,\qquad F_V=\det M,\qquad
 F_A=\operatorname{tr}(G^2)-\tfrac13(\operatorname{tr}G)^2.
\]

They measure squared size, oriented volume and shape anisotropy. Retain the determinant's sign: SO(3) is not
O(3). The classical orbit of M=I has (F_R,F_V,F_A)=(1,1,0), and M=0 has (0,0,0). These are reference points,
not identified quantum vacua. Their locations in M coordinates do not merge as g changes.

For ground, scalar and tensor, retain all three means, the full 3-by-3 covariance and raw second moments:

\[
 \mu_i=\operatorname{Tr}(\rho F_i),\qquad
 C_{ij}=\operatorname{Tr}(\rho F_i F_j)-\mu_i\mu_j.
\]

The F_i commute as multiplication observables. Compute covariance by centred full actions when possible to
avoid subtracting nearly equal large moments. Negative variances, non-Hermitian covariance, or positivity
failure beyond 1e-10 hybrid tolerance are refusals; do not silently clip the data to make a plot work.

Also report the dimensionless shape ratio

\[
 a_\rho=\frac{3\langle F_A\rangle}{2\langle(\operatorname{tr}G)^2\rangle},\qquad 0\leq a_\rho\leq1.
\]

This is a ratio of moments, not the mean of a pointwise ratio. A denominator at or below 1e-12 is unresolved,
not zero anisotropy. Report the q-coordinate raw moments as well, with the explicit conversion to M.

**Do not compute anisotropy from the mean Gram matrix.** A rotationally averaged multiplet can have an isotropic
mean G while retaining nonzero shape fluctuations. Likewise a larger mean determinant does not establish a
probability near the second vacuum. Basin occupancy, singular-value probability densities and tunnelling are
outside this contract; they require an independently validated configuration-space integration later.

The coordinate map x=sqrt(alpha) q gives scale factors g squared/alpha for F_R,
g cubed/alpha^(3/2) for F_V and g^4/alpha^2 for F_A. Test each factor separately. Cross-coupling comparisons of
these physical observables are allowed; overlaps between raw coefficient arrays at different g or alpha are not.

### Full products, not products of truncated matrices

Evaluate a polynomial moment as P_N F_i F_j P_N, not P_N F_i P_N F_j P_N. The omitted intermediate states
can change a variance even when both finite operator matrices look correct. Products have degree at most eight.
Use full F_i actions through oscillator degree N+4 and accumulate their inner products in bounded row tiles;
an independent small-case oracle integrates the resulting polynomials against the Gaussian through degree 2N+8.
The code paths must not share the same product/projection implementation.
The moment oracle uses integer exponent tuples, not the production five-bit packed coordinate encoding:
product degrees must never overflow into another coordinate. Validate production index bounds separately.

### A fixed geometry hypothesis and numerical stability

The secondary directional hypothesis is `mean_FV(scalar) > mean_FV(tensor)` at all six held-out couplings.
Define G_V as that difference. A positive result requires the difference to exceed
`0.001 * max(1, abs(mean_FV_scalar), abs(mean_FV_tensor))` at all three cutoffs, and moment stability below.
A negative difference with that clearance contradicts the hypothesis; a smaller difference is unresolved.
Even a supported result means a signed-volume contrast, not localization toward another vacuum.

Require each raw mean, second moment, covariance entry and shape ratio to change by at most
`0.001 * max(1, abs(new_value))` on BOTH successive increments. This new geometry policy is fixed now and
does not replace any energy, residual or probe criterion. For unbounded observables it is a numerical
stability test, not a rigorous expectation-error bound. No required prediction is imposed on anisotropy;
measure it without forcing the scalar and tensor to have a desired visual shape.

## 7. Follow subspaces and the individual residual contributions

At fixed coupling and frozen alpha, embed oscillator vectors at adjacent cutoffs into the same oscillator
space, including the Gram metric. For complete subspaces U,V of ranks r,s, report all principal angles and

\[
 d_P^2=\frac{r+s-2\|U^\dagger V\|_F^2}{r+s}.
\]

Identical subspaces give zero; orthogonal subspaces give one. Changes of phase or unitary basis within a
multiplet cannot change it. Require equal, unambiguous ranks and d_P squared at most 0.001 on both increments
for each tracked branch. A change in rank or same-spin branch ambiguity is unresolved, not an automatic match.
No cross-coupling state overlap is reported without a separate unitary dilation/Jacobian contract.

Keep H0/H2 response weights, absolute excited norms and the full five-by-five H2 kernel in the fixed probe
basis. A common spatial rotation must transform that kernel covariantly while preserving its trace and
eigenvalues. Do not present one chosen tensor eigenvector's orientation as the direction of the whole quintet.

To investigate the error, let Q_d project onto oscillator degree d, B=U-dagger H U, and define

\[
 A_d=H Q_d U-Q_d U B,\quad R_U=\sum_d A_d,\quad
 K_{de}=A_d^\dagger A_e,\quad R_U^\dagger R_U=\sum_{d,e}K_{de}.
\]

Save every retained shell's weight, all K_de, and the analogous blocks with an output-degree projector inserted.
Their traces give average squared contributions; the largest eigenvalue of the summed matrix gives the squared
multiplet residual. Individual interference terms can be negative. Summing norms, discarding cross terms, or
equating highest-shell weight with omitted residual must fail a planted cancellation fixture.

This identifies how the numerical mismatch is produced. It cannot by itself prove the best repair. Per-level
width, alternative envelopes and higher degree remain separate future experiments; none is silently introduced.

## 8. Strict gated TDD, including tests of the tests

Write each failing behavioural fixture before its implementation. Save the intended assertion failure, then
the passing receipt, then an isolated mechanism-removal receipt. An import error, crash, timeout or unrelated
exception is not the intended red result. Every fixture has one expected outcome, never an assertion accepting
either sign or either status. Separate uncertainty fixtures from positive and negative fixtures.

| Gate | Planted answer / required behaviour | Mutation or bad evidence that must be rejected |
| --- | --- | --- |
| QC00 Contract and exposure | Exact six couplings, three cutoffs, fixed grid, units, budgets, known-data hashes; review hold defaults on | Inspect a held-out excited result during width choice; alter grid/threshold; use stale registration or missing approval |
| QC01 Independent solver | Inherit CPU/CUDA and independent-oscillator agreement at small cutoffs; preserve H, S and spectrum under bounded assembly | Drop a Hamiltonian term, change metric, sort eigenvalues without vectors, silently fall back to CPU |
| QC02 Crossing and competitors | Analytic positive fixture below has D=-2 lambda; full spectrum audit rejects a lower third sector | Reverse D, follow energy index instead of spin, truncate competing levels to the plotted window |
| QC03 Degeneracy | Scalar plus complete quintet at exact equality, in a nonidentity metric and rotated eigenbasis, resolves both projectors | Average Casimir, drop mixed-spin block, split quintet or select an arbitrary member |
| QC04 No-crossing control | Oscillator plus 0.1 J squared has scalar gap 2, tensor gap 2.6, D=0.6 at N=4,6,8 | Report a crossing merely because H2 has strong response or because the main hypothesis expects one |
| QC05 Geometric identities | Exact point values below; proper rotations preserve scalars, while the nonspherical Gram fixture distinguishes colour invariance from spatial covariance | Replace det by abs(det); compute shape from mean G; swap colour and spatial transformation roles |
| QC06 Full moments | Exact Gaussian moments below, independent polynomial integration, positive covariance | Square a projected matrix instead of projecting the full product; omit intermediate states or physical scaling |
| QC07 Projectors and kernels | Phase and unitary rotations preserve scalar moments, covariance, principal angles and kernel invariants | Compare raw coefficient vectors, ignore S, average kernel entries incorrectly or fit a separate frame per state |
| QC08 Residual anatomy | Sum of all shell blocks reproduces direct full R-dagger R to 1e-10 hybrid tolerance; exact cancellation fixture | Drop off-diagonal interference, omit exterior trace term, report a shell weight as residual |
| QC09 Decision policy | Explicit positive, negative, zero and uncertainty-overlap fixtures; both cutoff increments required | Move 1% line, accept one stable increment, use interpolation as a sign observation, label floating radii certified |
| QC10 Geometry stability | Fixed convergent moment/projector sequence passes; one drifting covariance entry or mismatched rank refuses | Check means only, ignore covariance/overlap, turn missing measurements into zero |
| QC11 Bounded execution | Enforced 45-minute/8-GiB/4-GiB limits, fixed tiles, interrupted/restarted worker remains incomplete | Budget bypass, leaked prior-cutoff arrays, omitted stage logging or resource stop upgraded to COMPLETE |
| QC12 Integrity and replay | Replay matrices, all moments, projectors, kernels, shell blocks and verdicts on pinned sources | Wrong alpha/g, corrupt array, mismatched unit, missing case/log/gate or unverified resume |
| QC13 Honest reporting | Figures and exported records match receipts, all statuses retained; current verified manifest controls PASS | Cached/stale PASS after failed replay, omitted refusal panel, fabricated root/error bars or synthetic data mixed with measurements |

All QC00--QC13 gates currently have status **NOT RUN**. Existing HP gates are prerequisites, not automatic
passes for the new code. Mutation tests must demonstrate the named scientific failure, with failure reasons
checked and no unexpected errors. Use temporary isolated copies; never edit the pinned survey source in place.

### Exact control construction

Positive crossing fixture: vacuum energy 0, scalar energy 2+lambda, a complete spin-2 quintet at 2-lambda,
and a higher spectator sector. Test lambda=-0.2,0,0.2: D=+0.4,0,-0.4. Transform the entire problem by one fixed
invertible non-unitary matrix T: S=T-dagger T, H=T-dagger diag(E) T, J2=T-dagger diag(J(J+1)) T.
Use a second fixture with a complete spin-3 level at 1.5; branch crossing remains, but absolute-floor support
must be false. Do not silently reuse this synthetic fixture as the physical model.

Negative model control: the nine-coordinate free oscillator plus 0.1 J squared, in the same colour-singlet
space at alpha=1. The vacuum is 9/2; the degree-two scalar and spin-2 gaps are respectively 2 and 2.6. This
control is deliberately different from the experimental Hamiltonian and labelled as such. It has no scanned
coupling dependence. These exact answers, not the production solver's output, are the oracle.

Pointwise geometry fixtures: M=I gives (F_R,F_V,F_A)=(1,1,0); M=0 gives (0,0,0);
M=diag(1,0,0) gives (1/3,0,2/3) and shape ratio 1; M=diag(-1,1,1) gives (1,-1,0).
Proper rotations preserve determinant; a reflection changes its sign and is NOT a gauge equivalence.
For M=diag(2,1,1/2), G=diag(4,1,1/4). With the proper rotation
R=[[0,-1,0],[1,0,0],[0,0,1]], a colour change M R^T leaves G unchanged, whereas the spatial change R M gives
G'=diag(1,4,1/4). Scalar invariance alone cannot test this distinction; this matrix fixture must be present.

Gaussian state fixture: normalized exp(-tr(xx^T)/2), g=alpha=1, has independent coordinates of variance 1/2.
Independent Gaussian moment identities give mean(F_R)=3/2, mean(F_V)=0, mean(F_V squared)=3/4,
mean(F_R squared)=11/4, mean(F_A)=15/2 and shape ratio 5/11. Test also nonunit g and alpha by the declared
scale factors. At the one-vector ground-state cutoff, squaring the projected F_R gives 9/4 instead of 11/4:
this fixture must detect precisely that false variance. Other moments use independent exact Gaussian sums.

Shell-cancellation fixture: H=[[1,-1],[-1,1]], U=(1,1)/sqrt(2), B=0, with Q_0 and Q_2 projecting onto the
first and second coordinate. Then A_0=(1,-1)/sqrt(2), A_2=-A_0, their diagonal Gram entries are 1 and their
cross entries are -1. The true residual is exactly zero, not sqrt(2) or 2. Repeat after a complex unitary
change inside a higher-rank subspace so a transpose substituted for a conjugate transpose also fails.

## 9. Engineering and resource gate before new data

The last registered attempt stopped above 8 GiB before saving N=16. Do not simply relaunch a larger survey.
First specify bounded storage for the unchanged basis and action, including solver workspaces, file compression,
validation copies and retained caches. Reuse the existing implementation at small cutoffs as a reference.
Tile oscillator actions and moment accumulation, free each cutoff before advancing, and retain complete small
residual Gram blocks rather than every large temporary action. No reduced precision or discarded terms.

Focused resource tests must exercise the production allocation paths using representative dimensions and
synthetic payloads, under the same supervisor, without revealing held-out spectra. Synthetic resource tests
cannot guarantee a later physical run will fit. If it still exceeds budget, record RESOURCE_STOP and stop.
Changing the budget or representation requires a separately reviewed amendment, not an automatic retry.

The supervisor covers selection, construction, solves, geometry, persistence and replay within each registered
attempt. Sample host and process GPU allocations at most one second apart, record exact phase boundaries and
every case immediately, and preserve stdout/stderr from startup. Device-wide utilization is labelled separately.
Sampled RSS enforcement can overshoot between observations; preserve the actual peak and stop reason rather
than advertising the configured budget as a proven allocation bound. No unmonitored compute subprocesses.
Retain periodic heartbeats while a case is unfinished. Flush events and publish case files atomically with hashes;
do not infer case completion from observation time. The current late-attached observer is not sufficient alone.

Store source/spec/test hashes, dependency and GPU versions, tile sizes, initial RNG seeds for controls, all
width trials, every checked array and exact failure reasons. Figures derive from those artifacts. Logs cannot
be ignored by the evidence commit. A clean implementation revision and frozen registration are required.

Use RUNNING -> COMPLETE, FAILED or RESOURCE_STOP for execution, separately from scientific status. COMPLETE
only means every planned case and replay completed. Interrupted attempts retain immutable partial evidence.
Resuming requires identical contract/source/environment compatibility checks and replay of completed cases;
preserve original attempt boundaries and cumulative consumed budget, not a fresh hidden 45-minute allowance.

## 10. Planned implementation and evidence

These are proposed files, not existing runnable commands:

- `scripts/ym_helike_crossing.py`: sector projectors, full-action state geometry, shell decomposition and decisions.
- `scripts/test_ym_helike_crossing.py`: QC00--QC10 planted fixtures and focused numerical tests.
- `scripts/check_ym_helike_crossing_mutations.py`: mechanism-removal checks with asserted failure reasons.
- `scripts/run_ym_helike_crossing.py`: budgeted worker/supervisor, registration, logging and atomic receipts.
- `scripts/verify_ym_helike_crossing.py`: independent artifact replay and invalidation of stale acceptance.
- `scripts/test_ym_helike_crossing_runner.py`: QC11--QC13 interruption, integrity and output tests.

If reference kernels require changes for bounded allocation, keep their physics/API intact and require their
existing focused tests plus new mechanism-removal checks. No unrelated GIGI engine work or full Rust suite is
part of this Python experiment. If later engine code changes, its normal all-target and live-server gates apply.

Evidence goes under `research/helike-quantum-probe-20261123/crossing-<UTC-run-id>/`, including registration,
sources, red/green/mutation receipts, events.jsonl, worker.log, resource.csv, cases.jsonl, arrays, geometry.csv,
shell blocks, full spectra and final verification. Publish a relative-path SHA-256 manifest and preserve exact
bytes across machines. A missing/corrupt item makes verification fail closed. A previous valid PASS file cannot
survive a failed verification as an authoritative current status; the verifier records a new failure and the
renderer checks the current manifest rather than trusting a cached badge.

GIGI receives immutable model/run/case/level/probe identities and both physical and numerical descriptors under
the [evidence atlas plan](ym-evidence-continuum-gigi-plan-2026-11-23.md). Source arrays remain the authority.
Import is a separate YG00--YG02 live-server gate, not silently part of this execution. No claim that covariance
analysis rediscovers spin unless tested later with energy, spin, channel weights, coupling and cutoff excluded
from its feature set and normalization frozen beforehand.

## 11. The high-fidelity result we must be able to show

Generate four reusable figures from the same pinned receipt, not hand-entered numbers:

1. **The crossing:** E_s-E_ground, E_t-E_ground and their difference at all six couplings and all three cutoffs.
   Show the fixed prediction bracket, numerical residual allowances and unresolved points. No smoothed line
   presented as a measured root; no uniqueness claim.
2. **State geometry:** size, signed volume and anisotropy, plus their covariance for ground/scalar/tensor.
   Put the classical reference values beside them. Any covariance ellipse is a moment summary, not a measured
   density contour, confidence region or classical picture of a quantum state.
3. **Same state under refinement:** principal angles/projector distances and H2/H0 response kernels, in the
   common declared frame. No arbitrary eigenvector sign flip should change the picture.
4. **Where the equation misses:** input/output degree contributions and signed interference next to the actual
   full residual. This is the test of the high-shell explanation, not just a second plot of shell weights.

Each figure displays source identity, units, cutoff, alpha, model scope and acceptance status. Captions explain
the difference between a geometry measurement, an interpretation and a demonstrated mechanism. If a gate fails,
the panel remains present and labelled. Missing N=16 results never acquire interpolated stand-ins.

## 12. Decision and review sequence

Scientific status is `NOT_EVALUATED` before execution; `UNRESOLVED` for missing/inadequate numerical evidence;
`NOT_SUPPORTED` for a resolved contradiction of the registered crossing predictions; or
`SUPPORTED_NUMERICALLY_IN_THIS_MODEL` when every primary requirement is met. Geometry-hypothesis status is
separate. Supported crossing plus contradicted volume contrast is a valid result with a rejected explanation,
not permission to invent a new explanation after seeing the data. No status is `certified_gap` or a Clay claim.

1. Commit and push these corrections and this specification. **Stop for review.**
2. After approval, write behavioural red tests, implement the smallest required pieces, run focused tests and
   mutation checks, and publish their receipts. Do not touch held-out spectra during this phase.
3. Freeze and push the implemented registration, full executable source and resource plan for pre-run review.
4. Only after run authorization, select widths and execute the bounded 18-case experiment with startup logging.
5. Independently replay, publish the four figures and all refusals, then evaluate the named claims. No gate
   closes merely because a gap curve looks convincing or a geometry interpretation is appealing.

This plan can establish a reproducible numerical crossing and geometric contrast in the named finite-mode
model. It cannot identify that crossing with the classical Berger edge without the separate operator/gauge map,
prove its mechanism from moments alone, or turn a reduced-model basis limit into the four-dimensional continuum.
