# RF2D: operator consistency diagnosis

Date: 2026-11-17. Diagnosis completed; **no operator correction shipped**.
The original full-flow gate still FAILS. Correspondence NOT ESTABLISHED.
Mapping, mesh, clock, native action and acceptance thresholds are unchanged.

## Finding in plain English

GIGI correctly differentiates the discrete action we gave it. The problem is
that the full discrete derivative is not approaching the continuum derivative
under the current edge/face interpolation and mass-inverse construction.

The data locate the dominant discrepancy on the action/curvature side,
not on the mass/interpolation side. A separate infinitesimal Abelian control
reproduces a strong-flow defect even with no noncommuting field interactions
or finite-angle sign crossing. Its boundary differentiation passes an
independent Stokes test. The failing step is the discrete adjoint, or
codifferential, acting on canonically interpolated curvature.

Spectral accounting shows the mechanism in that control: a tiny amount of
field mass in higher modes is magnified into nearly all of its flow residual.
This is stronger localization than a comparison of similar-looking curves.
It does not establish that every nonlinear Berger error has the same cause,
or a general theorem of nonconvergence.

## Frozen provenance

- [Primary spec](RICCI_OPERATOR_CONSISTENCY_SPEC.md), fixed before new measurements.
- [Spectral addendum](RICCI_OPERATOR_SPECTRAL_CONTROL_ADDENDUM.md), explicitly
  post-diagnostic, fixed before the additional spectral partition measurement.
- Same RF2C shape-regular resolutions n=1,2,3,5 and S3/120-fold quotient.
- Same Berger `(3,1,1)` data, full mass norm, exact link mapping and continuum oracle.
- Same BW05: final relative error below 5%, both final orders at least 0.5,
  numerical error estimates below one tenth of spatial error.
- Same retained native assembly SHA256:
  `71fb30dbbb5fbfcd01e7a516f72849bef9303cbae8b046f7fbd2fe7824b196ad`.
- HEAD `0a64e2f848693221bf25f330ea4fb19041249106`, dirty worktree with actual
  source hashes recorded. No previous evidence was overwritten.

The original native outputs had already replayed byte-for-byte in RF2C.
RF2D validates their hashes, recomputes all eight new diagnostic cases and
replays six new native characterization tests. It does not pretend those are
eight additional native assembly runs. Everything remains an isolated
diagnostic, not a new GQL feature or deployment.

## Exact residual split

The independent weak load integrates the known continuum field against the
same covariant trial field that defines the mass. It does not use the native
force to invent a target. A planted representable field verifies that the
load equals the independently computed cell mass times its coefficients.

With B (bee) the mass, a the action derivative, v the continuum link velocity,
and L (ell) the independent load:

    r_total = a+Bv = (a+L) + (Bv-L) = r_action+r_mass.

The following are volume-normalized mass norms after solving with B:

| n | Total error | Action/load error | Mass/interpolation error |
|---|---:|---:|---:|
| 1 | 8.54645144 | 8.35765208 | 2.63144538 |
| 2 | 8.30694972 | 8.32262408 | 0.99935602 |
| 3 | 8.42992824 | 8.44710533 | 0.54101638 |
| 5 | 8.64205265 | 8.65041556 | 0.24762320 |

These norms must NOT be added as percentages of the total. The components
are not orthogonal. All cross terms are retained; vector reconstruction
errors are below 2.8e-12. The finest cross inner product is -0.10296633.
The small mass/interpolation component cannot by itself explain the large
total discrepancy, by the triangle inequality.

Order-4 versus order-6 weak-load differences in the dual mass norm decrease
from 5.754e-6 to 6.462e-9. They are far smaller than the action discrepancy.
No diagnostic correction is substituted into the actual flow.

## Double cover and phase checks

The native tests preserve the distinction between SU(2) and SO(3):

- Identity and minus identity have equal adjoints but different fundamental
  holonomies and different native action values in the planted fixture.
- Legitimate vertex-sign gauge transformations preserve action, derivative
  and mass. An arbitrary edge-sign change need not do so.
- Smooth signed paths across the hemisphere boundary and the logarithm cut
  have smooth native action. Forced positive-scalar canonicalization is
  detected by a planted discontinuity.
- Simultaneous base antipodes preserve the link; that is not link-sign erasure.
- The wrong half-angle and removed transports both make focused gates fail.

For the actual Berger data, every recorded link and face occurrence has
positive signed scalar part. The smallest face scalar component rises from
0.99561348 to 0.99999168 under refinement. Its largest SU(2) eigenphase falls
from 0.09369880 to 0.00407902 radians. The finest lifted sameness minimum is
0.99999584; the bridge identity is checked without confusing it with squared
SO(3) overlap. Counts describe cell-face occurrences, not distinct faces.

Thus there is no measured local branch crossing or erased sign in these
configurations. More importantly, the infinitesimal control fails without
requiring such a crossing. This is NOT a survey ruling out collective,
global-topological, or other physical phase transitions in Davis's work.

## Native-anchored Abelian control

On the same round S3 meshes, use the first Maurer-Cartan one-form, theta one.
Its exact edge integrals are known; its continuum codifferential eigenvalue
is 4. The control uses the ordinary Whitney stiffness and full mass:

    stiffness = D^T M2 D; velocity = -M1^{-1} stiffness * edge_integrals.

This is a separate mechanism control, not a change to the Berger mapping.
The native SU(2) action's infinitesimal single-axis Hessian agrees with this
stiffness. Halving the amplitude gives normalized derivative errors
`1.4751175e-4, 3.6878092e-5, 9.2195327e-6`, the expected quadratic decrease.
The physical tangent-mass factor of four is checked directly.

| n | Relative strong-flow error | Rayleigh quotient, target 4 | Energy density, target 2 |
|---|---:|---:|---:|
| 1 | 29.8111% | 4.02172954 | 1.96717383 |
| 2 | 25.9850% | 4.00575878 | 1.99128453 |
| 3 | 25.8603% | 4.00258432 | 1.99608826 |
| 5 | 26.1324% | 4.00093488 | 1.99858487 |

The exact coarse energy value is retained in the JSON receipt; table values
are rounded. Scalar velocity differences between quadrature orders fall from
5.262e-7 to 3.057e-9. The defect is not caused by insufficient quadrature.

Independent radial-triangle quadrature confirms Stokes' identity against
the exact edge integrals. The oriented incidence also satisfies boundary of
boundary equals zero; a reversed sign is detected. This isolates the
problematic commuting step to the discrete adjoint acting on interpolated
curvature, not the exterior derivative itself.

## What the HHE/Crouzeix work contributed

The actual GIGI harmonic-conjugate kernel recovers
`p(z)=(1-z)/2` from `u(x,y)=(1-x)/2`, including its conjugate and derivatives.
Its unit-disk upper bound is **1.0000000000000568**, with disclosed allowance
5.6843419e-14 and zero harmonic/reconstruction residuals. This is a new
polynomial for this operator, not the old 352-term polynomial transplanted
from another domain. It remains floating-point evidence, not interval proof.

On the actual 142-dimensional coarse scalar operator:

| Check | Measured result |
|---|---:|
| Symmetry error after mass whitening | 4.82e-16 relative |
| Maximum eigenvalue | 2585.51465983 |
| Eigenpair residual | 1.91e-15 relative |
| Normalized polynomial norm in physical mass coordinates | 1.0000000000000004 |
| Same polynomial's unwhitened Euclidean norm | 1.0346611327 |

Whitening means expressing the operator in coordinates where the physical
mass norm is the ordinary Euclidean norm. The whitened operator is symmetric;
its normalized spectrum is in [-1,1], inside HHE's declared disk. The spectral
theorem applies directly. A general non-normal Crouzeix bound is not needed
to certify this particular finite control. The larger unwhitened norm must
not be mistaken for a physical instability or a phase transition.

The companion spectral calculation uses the predeclared partition at
eigenvalue 8, twice the known continuum eigenvalue 4. Modes above it carry:

- **0.00237851%** of the field's squared mass norm;
- **99.97958017%** of the squared strong-flow residual.

The sum over spectral modes agrees with a direct mass solve to 4.23e-14.
No modes were removed, no smoothing was applied, and no acceptance score
was recomputed after filtering. This demonstrates amplification of small
interpolation components in the actual linear control; it is not a nonlinear
stability proof or a claim that HHE has already repaired the flow.

## Tests and receipts

New focused tests: 6 native Rust checks and 15 Python checks, with behavioral
RED/GREEN logs. Removal tests catch sign folding, wrong half-angle, absent
transport and wrong orientation. Adversarial evidence tests reject missing
levels, changed mapping, invented zero action error, altered phase values,
forged arrays, invented PASS and unfinished runs, and remove stale receipts.

Evidence: `output/ricci-consistency-20261117/diagnostic/`:

- `manifest.json`, `diagnostic.json`, eight `level*_q*.arrays.npz` files.
- `VERIFIED_DIAGNOSTIC.json`: eight recalculations and six native test replays.
- `NATIVE_RECEIPT.json`: executable/source hashes and native HHE receipt.
- `coarse_q*_mass.npz`, `coarse_q*_stiffness.npz`.
- `SPECTRAL_ACCOUNTING.json`: full coarse spectrum and coefficients, hashes,
  exact norm accounting and explicitly limited scope.
- Retained test logs and final focused test receipt.

No full default/Halcyon suite, remaining manifold campaign, commit or deploy.
The original finest full-flow error is still **25.5066%**, not below 5%.

## Reproduce

From the repository root, with a fresh output directory:

```powershell
$env:OPENBLAS_NUM_THREADS='1'
$env:OMP_NUM_THREADS='1'
cargo test --features halcyon,kahler --test ricci_operator_consistency --target-dir C:/Users/nurdm/.cache/gigi-ricci-regression -- --nocapture
python -m unittest discover -s e2e/probes -p 'test_ricci_consistency_*.py' -v
python e2e/probes/ricci_consistency_diagnostic.py output/ricci-consistency-reproduction
$native = (Get-Content output/ricci-consistency-20261117/diagnostic/NATIVE_RECEIPT.json | ConvertFrom-Json).binary
python e2e/probes/ricci_consistency_evidence.py output/ricci-consistency-reproduction --native-binary $native
python e2e/probes/ricci_consistency_spectral.py output/ricci-consistency-reproduction
```

The focused evidence attack fixtures use the retained original diagnostic
directory; the primary replay accepts the new directory. The test executable
must include both feature flags and run six tests, not a filtered subset.

## Smallest justified next target

Specify a compatible curvature projection/reconstruction or controlled
regularization that addresses this adjoint/interpolation defect. First prove
it on the planted Abelian control, including its full strong norm; then
differentiate the resulting SU(2) action and test the SAME Berger mapping and
BW05 thresholds. Any polynomial regularization needs its own approximation,
energy and degree/refinement contracts. The present HHE check alone does not
justify inserting a filter, changing a clock, or discarding transverse modes.
