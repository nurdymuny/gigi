# Native Ricci flow: RF1 evidence

Status: RF1 live mathematical, mechanism-removal, evidence-adversary and full
local regression checks PASS. No commit or deployment. The broader
manifold-to-topology and Davis-Wilson/Ricci correspondence gate is NOT closed
by this experiment. The earlier combined MONODROMY release runner also still
needs its clean end-to-end replay after the build-cache isolation repair;
RF1 does not relabel that older open release gate.

## What actually changed

GIGI now has a native `RICCI_FLOW` query which evolves a declared smooth
three-sphere metric by Hamilton's Ricci equation. It handles general diagonal
left-invariant SU(2) metrics, including triaxial metrics and metrics with
negative initial scalar curvature. It is not a renamed action statistic.

```sql
RICCI_FLOW SU2 METRIC (1,1,1) TIME 0.2 STEPS 16;
RICCI_FLOW SU2 METRIC (0.5,1,2) TIME 2 STEPS 1024 NORMALIZED;
RICCI_FLOW SU2 METRIC (5,1,1) TIME 0.03 STEPS 256;
```

RF1 outputs the metric, Ricci tensor, sectional and scalar curvature, cover
volume, distortion and numerical diagnostics at each sampled time. Insufficient
resolution and singularity crossing refuse; they do not become successful
partial trajectories. The exact scope is in `RICCI_FLOW_SU2_SPEC.md`.

## Observed live values

These are output from the native GQL server, not values supplied by a client.

| Experiment | Initial state | Final native result |
|---|---|---|
| Round shrink at t=0.2 | A=B=C=1, R=6 | A=B=C=0.19999999999999937; R=30.000000000000096 |
| Volume-normalized rounding at t=2 | (0.5,1,2); squared-length max/min=4 | (0.9999998120944759,1.0000000903536639,1.0000000948562435); squared-length max/min=1.0000002827618208 |
| Negative scalar curvature at t=0.03 | (5,1,1); R=-2 | (3.371040767507654,1.1961567756386808,1.1961567756386808); R=1.9759507901914468 |

The independent checker derives the connection from the Lie brackets using
Koszul's formula, contracts the Riemann tensor, and integrates with SciPy
DOP853. GIGI uses the closed-form Ricci components and RK4. This checks both
the equations and the numerical trajectory using different computations.

- 76 native trajectory cases, including 64 varied metric initializations.
- Six explicit native refusals, including at/beyond round extinction.
- Worst non-refinement trajectory disagreement: 1.213278810752172e-9 relative
  to the maximum initial metric coefficient, across every returned time.
- Step refinement by two reduces measured error by 16.758650384452846 and
  16.386145312169884, consistent with fourth order on this tested fixture.
- The independent oracle also checks 128 scalar-curvature evolution identities
  and volume-rate identities across its 64-metric grid.

These are numerical comparisons, not certified interval bounds or a proof
about all metrics. Fixtures are explicit mathematical models, not measured
sensor data. This distinction is appropriate for this mathematical question.
Metric coefficients are squared lengths. The initial 4:1 coefficient ratio
therefore corresponds to a 2:1 directional length ratio, not 4:1 lengths.

## The topology control, and why it matters

Take the analytic model SU(2), the simply connected three-sphere, and a finite
subgroup Gamma acting by LEFT translations. This action is free: h*x=x implies
h=identity. The quotient Gamma\SU(2) is a smooth manifold; its universal cover
is SU(2), and its fundamental group is Gamma. This construction is external
mathematical input, not something inferred from arbitrary gauge links.

For the binary icosahedral generators, an independent exact calculation in
Q(sqrt(5)) verifies the finite group of order 120 and derived subgroup of
order 120. The native floating-point GIGI closure independently returns 120
and 120, with its numerical-table checks. The identity control returns 1 and 1.
Both abelianizations are trivial, but the groups are not the same.
These are deck groups of the specified covers, not Levi-Civita holonomy
groups. Parallel transport on a curved simply connected sphere need not be
trivial; this experiment does not conflate that connection with deck actions.

The same left-invariant metric and local Ricci solution descend to BOTH
quotients. In particular, round radius-one metrics on S3 and the Poincare
homology sphere share A(t)=B(t)=C(t)=1-4t and extinction time 1/4, but their
fundamental groups differ. Quotient volume is cover volume/|Gamma| by the
covering construction; RF1 reports cover volume only and does not calculate
or claim a quotient volume.

The four-edge lattice in the demo is only a generator storage adapter. It is
NOT a triangulation of either manifold. No BETTI or PI_1 claim is made from
that graph. There is no hidden label-to-topology lookup in the flow kernel.

This is useful because it prevents a false inference: local Ricci evolution
or extinction alone cannot decide which of these manifolds one has. Geometric
evolution and validated global topology must remain distinct measurements.

## Reproduce

Dependencies for independent verification: Python, NumPy, SciPy. Plotting
additionally uses Matplotlib. The observed run used SciPy 1.16.3.

```powershell
cargo build --features halcyon --bin gigi-stream --target-dir C:\Users\nurdm\.cache\gigi-monodromy-target
python e2e/probes/ricci_flow_demo.py --binary C:\Users\nurdm\.cache\gigi-monodromy-target\debug\gigi-stream.exe --output output/ricci-replay
$env:RICCI_EVIDENCE='output/ricci-replay/live.json'
python -m unittest discover -s e2e/probes -p test_ricci_evidence.py
python scripts/ricci_mutations.py --output output/ricci-replay/mutations --target-dir C:\Users\nurdm\.cache\gigi-monodromy-target
```

The demo starts/stops only its own private server and uses an isolated data
directory. It records every request and raw/parsed response, source hashes,
HEAD and binary hash. Re-run after edits: stale source evidence must refuse.

Evidence: `output/ricci-20261117/final-live/live.json` and its `.PASS.json` receipt.
The PASS scope is explicitly live mathematical evidence, not release approval.

## Gate record

| Gate | Observed result |
|---|---|
| RF00-RF06 | Eight native behavioral tests RED before implementation, GREEN afterward; two internal finite-input/stage-guard tests; feature-off and feature-on HTTP boundary tests |
| RF07 | 76 actual native trajectories, six refusals, native closure orders 120 and 1; raw requests/responses retained |
| RF08 | Five actual source mutations detected: update removed, curvature cross term removed, normalization removed, positivity removed, error gate removed |
| RF09 | Three verifier test methods pass, including nine coherent tampering cases and four missing/malformed-input cases; stale PASS removed before reads |
| RF10 | All-target default: 1,619 passed, 0 failed, 24 existing ignored. All-target halcyon: 2,276 passed, 0 failed, 40 existing ignored |

Rust source and test files were unchanged throughout the final regression
and live checks (427 files compared to the captured manifest). Baseline HEAD:
`0a64e2f848693221bf25f330ea4fb19041249106` plus the uncommitted changes captured
by per-file hashes. HEAD alone does NOT identify this new feature.

Live binary SHA-256:
`211c030545eec2711e1d55fbfc9f29781dcf04df463fdbafa96f8e64865e81bd`.

Full-suite commands used `CARGO_PROFILE_TEST_OPT_LEVEL=1` and
`CARGO_PROFILE_TEST_DEBUG=0`, retaining all targets while avoiding the very
slow unoptimized dense-matrix regressions. No test exclusions were added.

```powershell
$env:CARGO_PROFILE_TEST_OPT_LEVEL='1'
$env:CARGO_PROFILE_TEST_DEBUG='0'
cargo test --target-dir C:\Users\nurdm\.cache\gigi-ricci-regression
cargo test --features halcyon --target-dir C:\Users\nurdm\.cache\gigi-ricci-regression
```

Logs: `ricci-red.log`, `ricci-green.log`, `ricci-default-full.log`,
`ricci-halcyon-full.log`, `ricci-final-evidence-tests.log`,
`ricci-final-live.log`. Mutation commands, baseline and deliberate-failure
logs are in `output/ricci-20261117/mutations/`. Mutation builds use a distinct
`ricci-mutants` cache, never the engine build cache. No compilation failure
or missing test is counted as a successfully detected mutation.

The earlier coarse-resolution preflight is retained separately. Its failure
led to finer requested resolution, not relaxed accuracy requirements.

## Remaining research step

To test the Davis-Wilson correspondence, derive a metric-to-connection map
on an actual discretization of these SAME manifolds; fix the physical time
mapping and observables before running held-out tests. Then compare Wilson
evolution with these independently computed Ricci trajectories as the mesh
and time step are refined. No such map or equivalence has been implemented
or validated here. General Ricci flow with surgery also remains out of scope.
