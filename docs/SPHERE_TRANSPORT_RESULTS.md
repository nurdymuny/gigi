# Checked sphere transport: results and consumer handoff

2026-11-13. Scope: additive Rust library functionality, not a deployed endpoint.
Contract: [SPHERE_TRANSPORT_SPEC.md](SPHERE_TRANSPORT_SPEC.md).
Origin: [DTP three-pass review](research/dtp-development-review-2026-11-13.md).

## What is now implemented

`gigi::dials::sphere_parallel_transport` derives a shortest-great-circle
Levi-Civita transport matrix from measured unit-sphere endpoints and applies it
to a supplied tangent vector. `sphere_parallel_transport_records` reads three
named numeric coordinates from two actual records and calls that same function.
It never supplies a missing coordinate as zero.

The return value includes the matrix, transported vector, angle, normalized
endpoints, original endpoint norms, numerical-policy version and measured
endpoint/orthogonality/tangency/norm residuals. Units and orthonormal frame are
an explicit caller contract; no automatic calibration or conversion occurs.

The legacy rotation kernel is reused but its old endpoint conventions remain
unchanged for existing callers. The checked adapter refuses unsupported inputs
before calling it and checks the numerical result afterward. Antipodal and
near-antipodal shortest arcs require a better-resolved intermediate waypoint.
Endpoint normalization is restricted to declared roundoff tolerance and disclosed.

This makes the distinction executable: an existing rotation operation can now
be consumed as sphere parallel transport with a defined mathematical model and
input/output checks. It is not an assertion that every rotation, arbitrary
matrix or projection is parallel transport.

## Worked example, actual embedded GIGI

Reproduce using the real Engine, isolated temporary database, and planted
independent answers:

```powershell
cargo test --test sphere_transport sphere_embedded_gigi_showcase_persists_derived_transport_and_reopens -- --nocapture
```

The test creates `sphere_positions` with a text identity and numeric x,y,z
fields. It inserts two closed paths, reads their endpoints through
`Engine::point_query`, derives every edge using the public record adapter,
transports two independent tangent vectors, and compares against the independent
short-arc formula. It stores results in a native three-component Vector field,
checkpoints the database, closes it, reopens it and requires exact equality of
all four saved result vectors. Fifteen WAL entries replayed twelve records.

Path 1 is x -> y -> z -> x. Path 2 is the normalized sequence
(1,2,3) -> (-2,3,1) -> (3,1,-2) -> (1,2,3). Each starts with the tangent
`normalize((-p_y,p_x,0))` and its perpendicular `p cross tangent`.
These are analytic fixtures, not observations claimed to come from a real
sensor. Both use the same general-purpose implementation with no route-specific
production branches.

Captured output (baseline removal-test harness, same public implementation):

```text
octant_0:
 actual      [0.0, 1.232595164407831e-32, 1.0]
 independent [0.0, 0.0, 1.0]
octant_1:
 actual      [1.1102230246251565e-16, -1.0, 1.232595164407831e-32]
 independent [0.0, -1.0, 0.0]
oblique_0:
 actual      [0.09173612215383764, 0.8141580841153078, -0.5733507634614845]
 independent [0.09173612215383742, 0.8141580841153082, -0.5733507634614847]
oblique_1:
 actual      [-0.959247576209398, 0.22678696689934646, 0.16855788080356832]
 independent [-0.9592475762093984, 0.22678696689934652, 0.16855788080356823]
```

The tiny nonzero components in the octant result are retained, not silently
snapped to zero. DTP's explicitly signed representation can express those
special rotations exactly; this native floating-point construction does not
pretend to be that exact representation.

## Gates and receipts

Evidence directory: [sphere-transport-20261113](research/evidence/sphere-transport-20261113/).

- Initial compiling refusal-only scaffold: seven behavioral failures, one legacy
  compatibility test passed. [Red log](research/evidence/sphere-transport-20261113/red.log).
- Implemented focused integration suite: eight passed. Includes 48 non-axis
  oracle/inverse/frame-covariance fixtures, forward/reverse/backtrack, both basis
  vectors, whole-map and 21-circuit checks, refusal cases and embedded recovery.
- Final isolated baseline: all nine tests passed, including the private bad-output
  guard test. [Captured baseline](research/evidence/sphere-transport-20261113/baseline-test.log).
- Nine isolated source mutations compiled and then failed assertions: identity
  instead of rotation, reversed rotation, omitted unit/tangency/antipodal/magnitude
  guards, omitted normalization, missing-measurement zero fill and omitted
  numerical postconditions. No compile failure counted as a caught mutation.
  [Machine-readable receipt](research/evidence/sphere-transport-20261113/mutations.json).
- Default all-target regression: 1,589 passed, zero failed, 22 existing ignored,
  across 219 summaries including doc tests. Gauge-enabled holonomy library
  regression: five passed. Details and scope follow below.

Reproduce mutations after building the default library, supplying that build's
actual library artifact:

```powershell
python scripts/sphere_transport_mutations.py --rlib target/debug/deps/libgigi-d1800bbf7db81122.rlib
```

The artifact filename is build-specific. The receipt records its SHA256.
Mutants live in a temporary directory; the script never edits the working-tree
implementation. They link the real built GIGI library for the native kernel,
record types and Engine, not mock versions of those components.

## Ownership and remaining limits

The overbroad holonomy equivalence comment in `src/holonomy_cycle.rs` is
corrected to its restricted lens/lattice scope. No holonomy computation changed.

DTP owns the uncertain-I/O store state and disk-fault gates, exact signed-loop
uncertainty, and its byte-count optimization. No DTP source was edited here.
This change does not claim those issues are resolved. General metric-aware
frame/unit changes remain outside the new unit-sphere/orthonormal-frame API.

There is no new GQL syntax or HTTP route, no production deployment, no physical
sensor qualification, and no benchmark or speed claim in this change. DTP can
adopt the library helper through its own integration gates.

## Final verification

The isolated `cargo test` completed successfully: **1,589 passed, zero failed,
22 ignored**, including the new eight integration tests and private numerical
postcondition test. This is the default-feature all-target suite, not an
all-feature claim. [Complete log](research/evidence/sphere-transport-20261113/all-targets-green.log).

`cargo test --features gauge --lib holonomy_cycle` completed with **five passed,
zero failed**. [Feature log](research/evidence/sphere-transport-20261113/feature-green.log).
No new server or production instance was started or changed. The new functionality
was verified through the actual embedded Engine, not a mocked database.

Two verification interruptions are retained rather than hidden:

1. The first feature build failed with OS error 112 (disk full), before tests
   ran. The retry used `CARGO_INCREMENTAL=0`, build artifacts under
   `D:/gigi-verification/sphere-transport-20261113/target`, and a D: temporary
   directory. The attempted cleanup command was blocked; no existing cache was
   deleted by this work. [Original build log](research/evidence/sphere-transport-20261113/feature-disk-full.log).
2. The initial default run stopped at an existing field-coverage snapshot test
   with a missing DHOOM header and zero instead of 200 records. DTP's concurrent
   root run also reported a failure in that test family. The tests use fixed
   global temporary names and delete those directories, making concurrent runs
   unsafe; disk pressure was another confound. This does not prove a specific
   interleaving caused this failure. All four field-coverage tests then passed
   unchanged in an isolated D: temporary directory, followed by the full suite.
   [Original failure](research/evidence/sphere-transport-20261113/default-first-run-failure.log),
   [isolated four-test replay](research/evidence/sphere-transport-20261113/field-coverage-isolated.log).

For future concurrent verification, set both TEMP and TMP to a private directory
for each run. Our default retry used
`D:/gigi-verification/sphere-transport-20261113/default-tests-temp`.
No Engine/storage behavior was changed to mask either interruption.

The bounded library gate is complete and its source and evidence are packaged
as a scoped development checkpoint. DTP integration, disk-fault recovery,
general metrics and deployment are not included in that completion claim.
