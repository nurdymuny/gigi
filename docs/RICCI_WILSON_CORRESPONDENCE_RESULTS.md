# RF2A: a specific Ricci / Yang-Mills correspondence

Date: 2026-11-17. RF2A local gates CW00-CW09 PASS: live mathematical,
mechanism-removal, adversarial-evidence and both full regression runs.
This closes the specified continuum audit, not the broader Davis-Wilson
lattice-correspondence or release gate. No commit or deployment.

## Result, with the three claims kept separate

1. **Instantaneous mapping: supported, including exact symbolic algebra.**
   For positive diagonal left-invariant SU(2) metrics, mapping the Ricci
   velocity through the stated Levi-Civita connection gives the independently
   computed negative Yang-Mills gradient at that same metric. The common
   clock factor is fixed to one, not fitted to examples. All three symbolic
   residuals reduce exactly to zero under positive squared-length substitution.
2. **Moving-background trajectories: supported in the tested cases.**
   An independent connection integration supplied with the evolving Ricci
   metric tracks the mapped native GIGI trajectory. The metric is an input
   to this experiment, not something the connection evolution recovers.
3. **Fixed-background trajectory equivalence: rejected in the tested
   nonround cases.** Holding the initial background metric fixed produces
   a reproducible, analytically predicted separation. This cannot be
   reclassified as successful equivalence by quietly switching to claim 2.

These are claims about the declared homogeneous continuum operators, not
GIGI's existing lattice Wilson force. No general Ricci-flow, surgery,
Poincare-conjecture, or Davis-Wilson lattice-equivalence theorem is claimed.

## A native, nonstationary example

```sql
RICCI_WILSON_AUDIT SU2 METRIC (3,1,1);
```

Selected actual output from the isolated rebuilt engine:

```json
{
  "metric": [3.0, 1.0, 1.0],
  "ricci_velocity": [-36.0, 4.0, 4.0],
  "connection": [-1.0, 1.7320508075688774, 1.7320508075688774],
  "mapped_ricci_velocity": [48.0, -13.856406460551018, -13.856406460551018],
  "yang_mills_velocity": [48.0, -13.85640646055102, -13.85640646055102],
  "residual": [0.0, 1.7763568394002505e-15, 1.7763568394002505e-15],
  "relative_residual": 7.401486830834378e-17,
  "time_scale": 1.0,
  "time_scale_fitted": false,
  "lattice_correspondence_tested": false
}
```

The exact expected connection velocity is `(48,-8*sqrt(3),-8*sqrt(3))`.
Unlike a round critical point, these velocities are nonzero. Returning zero
for both operators, or copying one operator into the other, is not accepted
as evidence. The output reports the small computed floating-point residual
rather than overwriting it with the symbolic answer zero.

## Independent evidence

The native Rust implementation differentiates a closed-form connection map
and evaluates the independently derived action gradient. Python instead
constructs the connection using Koszul's formula, uses real SO(3) matrices
and structure brackets, differentiates all nine action coordinates by
complex-step differentiation, and integrates using DOP853. SymPy supplies
a separate exact algebraic identity check. GIGI's Ricci integrator uses RK4.
The full nine-coordinate action also has six exactly zero off-diagonal
derivatives on diagonal connections; the reduced model does not hide those
components by projecting a larger force.

- 227 live native operator audits: a 64-metric grid, 12 seeded held-out
  metrics, both normalizations, scale/topology controls and flow checkpoints.
- Eight native Ricci trajectories, with every returned metric checked
  independently; 2,056 metric states in total.
- Largest measured relative operator residual: `7.993775916407522e-16`.
- Four explicit refusals: invalid metric, excessive anisotropy, unsupported
  manifold and attempted fitted clock.
- Arbitrary-connection action/gradient, constant-gauge covariance, zero
  torsion and Riemann-curvature agreement checks.

At time `0.02`, unnormalized runs give these maximum component differences
over the 65 sampled connection times:

| Initial metric | Moving-background difference from mapped native Ricci | Fixed-background difference | Fixed-solver tolerance refinement |
|---|---:|---:|---:|
| `(1,1,1)` | 0 | 0 | 0 |
| `(3,1,1)` | 2.17304e-12 | 0.0642991 | 2.08622e-10 |
| `(0.5,1,2)` | 1.57507e-12 | 0.0624778 | 1.55792e-10 |
| `(5,1,1)` | 7.62279e-12 | 0.3533990 | 6.03135e-10 |

The reference connection integrations run outside GIGI. The native query
audits the operators at actual native Ricci checkpoints; it does not claim
to provide a native Wilson trajectory solver. Both normalized and
unnormalized reference trajectories are retained in the evidence.

For the Berger control `(3,1,1)`, the initial moving-minus-fixed acceleration
is exactly `(-672,-16*sqrt(3),-16*sqrt(3))`. Centered time differentiation at
steps `0.0004`, `0.0002`, `0.0001` reduces the maximum acceleration error by
factors `4.000345865856613` and `4.000080348992574`. Thus the separation has
an operator-level explanation; it is not inferred from similar-looking or
different-looking curves.

All discrepancies are measured floating-point errors, not certified
interval bounds. These fixtures are specified smooth mathematical models,
not observational data or arbitrary triangulated manifolds.

## The same manifolds, without conflating local and global data

The models are SU(2), diffeomorphic to S3, and its quotient by LEFT
translations of the binary icosahedral group. Left-invariant metrics and
connections descend. Both the integrated action and the L2 mass are divided
by 120 on the quotient; an independent differentiation check verifies their
factors cancel in the gradient. Repeating a local query with a different
label is not the argument for descent.

An independent exact Q(sqrt(5)) calculation and native full-table closure
still give deck-group and commutator orders 120 versus 1. Those deck groups
are not the Levi-Civita holonomy groups. The four-edge graph stores group
generators only; it is not a manifold mesh. Local correspondence does not
recover the distinction between these global topologies.

## What this enables, and what remains

Researchers can now ask GIGI to compare two specified geometric evolution
operators at the same metric, inspect every component and normalization,
and distinguish an instantaneous identity from a standalone flow claim.
This narrows the next Davis-Wilson experiment: a candidate lattice mapping
must reproduce this continuum operator with a metric-dependent Hodge/L2
operator and demonstrate mesh/time convergence on the same manifold.

RF2B, the actual lattice adapter/convergence gate, remains open. Overall
metric scale must remain explicit because the connection map discards it.
The earlier combined MONODROMY release runner also retains its separate
open replay requirement; this experiment does not close it implicitly.

## Acceptance evidence

| Gate | Recorded result |
|---|---|
| CW00 | Six behavioral RED tests before the verb existed, then six GREEN; explicit feature-off and HTTP row tests |
| CW01-CW04 | Independent Koszul/Riemann/action checks, full-matrix invariance, exact symbolic identity, held-out metrics, scale/normalization and constant-gauge controls PASS |
| CW05 | Eight reference/native trajectory comparisons, solver refinement and independent acceleration refinement PASS; H3 is scientifically rejected, not relabeled as H2 |
| CW06 | Quotient action/mass scaling and exact/native deck-group 120 versus 1 checks PASS |
| CW07 | Five actual mechanism removals cause named behavioral failures; both clean baselines pass; separate source and build caches |
| CW08 | Live evidence PASS; 18 coherent tampering cases and four missing/malformed-input cases rejected; stale PASS removed |
| CW09 | Default: 1,622 passed, 0 failed, 24 ignored. Halcyon: 2,286 passed, 0 failed, 40 ignored. Both complete `cargo test` runs include integration and doc tests |

The evidence-format review additionally demonstrated a behavioral RED for a
boolean `time_scale` being accepted as numeric one. Its explicit numerical
type check now rejects that input; the retained RED and GREEN logs document
the correction. Revision tampering is also rejected. Existing ignored tests
are reported as ignored, not passed.

## Reproduce

```powershell
cargo build --features halcyon --bin gigi-stream --target-dir C:\Users\nurdm\.cache\gigi-monodromy-target
python e2e/probes/ricci_wilson_demo.py --binary C:\Users\nurdm\.cache\gigi-monodromy-target\debug\gigi-stream.exe --output output/ricci-wilson-replay
$env:RICCI_WILSON_EVIDENCE='output/ricci-wilson-replay/live.json'
python -m unittest discover -s e2e/probes -p test_ricci_wilson_evidence.py -v
python scripts/ricci_wilson_mutations.py --output output/ricci-wilson-replay/mutations --target-dir C:\Users\nurdm\.cache\gigi-monodromy-target
$env:CARGO_PROFILE_TEST_OPT_LEVEL='1'
$env:CARGO_PROFILE_TEST_DEBUG='0'
$env:CARGO_BUILD_JOBS='4'
cargo test --target-dir C:\Users\nurdm\.cache\gigi-ricci-regression
cargo test --features halcyon --target-dir C:\Users\nurdm\.cache\gigi-ricci-regression
```

The demo starts and terminates its own server on an available loopback port
with isolated storage. It records requests, raw replies, source hashes,
binary hash, dependencies, exact algebra and independent trajectories.
The verifier invalidates any previous PASS before checking the evidence.

Base revision: `0a64e2f848693221bf25f330ea4fb19041249106`, with the uncommitted
source manifest captured in `output/ricci-wilson-20261117/final-live/live.json`.
The base revision alone does not identify this dirty-tree build.
Live binary SHA-256:
`dadefcd74c0282b673e179873ff83ec5e25fde53e831ba71acef92c7efd63e48`.
Receipts and raw results live beside that file. Test logs are
`ricci-wilson-*.log`; mutation logs are in the sibling `mutations` directory.
`output/ricci-wilson-20261117/gate.PASS.json` binds the local gate summary to
the evidence, source manifest, binary and retained test-log hashes.
