# DTP development: independent three-pass review

Date: 2026-11-13. Reviewer: GIGI development.

## Scope and decision

This records the requested review of the DTP mathematics, actual GIGI integration,
and representation-aware performance evidence. It does not change the DTP
implementation, run another benchmark, or close a new implementation gate.

**Decision:** the specified sphere experiment is genuine Levi-Civita parallel
transport, and its saved engine answers agree with independent calculations.
The structural composition and reported allocation savings check out. Proceed
with exact byte counting as a bounded optimization experiment. Before stronger
durability or general geometric-invariance claims, address the findings below.

Root HEAD was `c9e31d1887ffb2ae5129950c20be5b8c3c454c09`; the working tree contains
uncommitted DTP and other development, so HEAD alone is NOT its source identity.
The [audit receipt](evidence/dtp-development-review-20261113/independent-checks.json)
hashes the actual files read, including processor, numerics, store, native GIGI
code and raw experiment receipts. All tracked inputs stayed unchanged during
the audit run. DTP's processor hash was
`97199fc3bf618226a167ff704d3638fb05090c3cd9efc6e40f364a02d101ad1d`, matching B03.

Reproduce from the repository root:

```powershell
python scripts/audit_dtp_geometry_20261113.py
```

Dependencies used: Python and the NumPy/SciPy versions in the receipt. The script
imports no DTP implementation. It runs independent oracles and reads saved
receipts; it does NOT execute Rust, start a server, or test disk-failure recovery.

## Findings, ordered by consequence

### 1. Recovery after failed sync needs an explicit uncertain-I/O state

**Priority: high before production durability claims; source-based, not a
reproduced disk-loss incident.**

In `crates/gigi-virtual-dtp/src/store.rs:75`, persistence invalidates the latest
cache, calls Engine insertion and then checkpoint, and caches the new state only
after successful checkpoint. This is correct for the normal acknowledged path.
`src/engine.rs:2031` logs the insertion and mutates the in-memory bundle before
checkpoint; `src/engine.rs:3050` and `src/wal.rs:756` perform the sync.

If checkpoint/sync fails after insertion, a later `Store::get`
(`store.rs:68`) can reconstruct from that same in-memory Engine and repopulate
the cache without another sync. A successful GET is therefore not evidence of
durability after that error. The instruction at `store.rs:13` to read persisted
state before retrying is insufficient if interpreted as a same-process GET.

Smallest justified next design: mark the store uncertain after persistence I/O
failure; refuse normal durable reads/steps until an explicit successful recovery
or sync reconciles state. Alternatively return an explicit uncertain-durability
status. Do not silently roll back an ambiguous WAL outcome.

Required gate: inject failures at insertion, flush and sync boundaries; read,
retry and restart; prove no acknowledged PC is lost or duplicated, no uncertain
PC is advertised as durable, and no cache bypasses recovery. Removing the
uncertain-state protection must fail this gate. This review does not establish
whether the existing WAL would retain or lose a particular failed write.

### 2. Native rotation has different degenerate-input semantics

**Priority: medium; an integration blocker, not a defect in the tested DTP run.**

`src/dials.rs:834` is a useful existing plane-rotation constructor. However,
collinear endpoints, including antipodes, return identity. Its nearby comments
explicitly disclose this convention. It also uses the smaller input dimension.
The GQL handler at `src/bin/gigi_stream.rs:15537` substitutes zero for absent or
nonnumeric requested fiber components. These behaviors do not satisfy a strict
sphere-connection contract.

An adapter must check dimensions, finite data, units, nonzero norms, tangent
membership, endpoint/path consistency, and antipodal ambiguity. Antipodal
endpoints need an explicit arc/plane, not an invented identity. Call the native
kernel only after those checks; preserve the original verb's compatibility.
Do not silently round floating rotations into exact signed permutations.

### 3. One existing holonomy comment overstates a topological equivalence

**Priority: medium, documentation/interpretation.**

`src/holonomy_cycle.rs:3` states an unqualified equivalence between trivial
holonomy and trivial fundamental group. It must not be used as a general theorem
for connection-derived transport. The sphere here is simply connected and its
Levi-Civita connection has nontrivial loop holonomy. A flat connection's holonomy
can factor through the fundamental group; trivial holonomy still does not imply
a trivial fundamental group without additional hypotheses.

Qualify the comment to its actual lens/lattice model and theorem assumptions in
a separately reviewed edit. This is not a claim that this specialized module's
implemented cycle calculation is incorrect. Standard connection/holonomy
definitions are in [Joyce, sections 1.2-1.3](https://people.maths.ox.ac.uk/joyce/RHCGnotes.pdf).

### 4. Coordinate-invariant decisions require an explicit metric contract

**Priority: medium before frame/unit-generalization; current coordinate contract
is internally consistent.**

DTP's Euclidean singular values and squared Frobenius closure defect are invariant
under orthonormal frame changes, not arbitrary changes of units or basis. The
audit supplies a counterexample: the same sphere holonomy has defect 4 in an
orthonormal basis and 6.25 after rescaling one basis vector by two. Carrying the
transformed metric restores 4. Do not market an unchanged threshold as generally
gauge invariant until the allowed frame group and metric are declared and tested.

## Pass 1: independent mathematics

### Sphere connection, order and sign

Column vectors, right-handed axes, positive quarter-turns:

```text
Rz = [ 0 -1  0 ]   Rx = [ 1  0  0 ]   Ry = [ 0  0  1 ]
     [ 1  0  0 ]        [ 0  0 -1 ]        [ 0  1  0 ]
     [ 0  0  1 ]        [ 0  1  0 ]        [-1  0  0 ]

H = Ry Rx Rz = Rx
H x = x, H y = z, H z = -y.
```

For a unit-speed great-circle rotation with fixed axis n perpendicular to p,
write a tangent field as `V = a n + b (n cross p)` with constant a,b. Its ambient
derivative is `-b p`, purely normal to the sphere. Its tangential covariant
derivative is zero, so this really is Levi-Civita parallel transport along that
arc, not merely an arbitrary rotation given a geometric label.

The independent endpoint formula for non-antipodal unit p,q and tangent v is

```text
T(p -> q)v = v - ((v dot q)/(1 + p dot q)) (p + q).
```

It is not a definition at antipodes. We checked it against the exact axis
rotations on both independent initial tangents, and against integration of
`dV/dt = -p(t) (p'(t) dot V)` on 24 seeded non-axis arcs. Maximum absolute
component disagreement was `9.89364146164462e-12`, inside the declared
`2e-11` absolute/relative comparison tolerance. Norm, tangency and reverse
transport checks passed. These numerical checks complement, not replace, the
derivation above.

| Saved live case | Tangent returned | Angle in oriented (y,z) plane | State error squared | Whole-map error squared |
|---|---|---:|---:|---:|
| Forward x -> y -> z -> x | (0,0,1) | +90 degrees | 2 | 4 |
| Reverse route with inverse edges | (0,0,-1) | -90 degrees | 2 | 4 |
| Backtrack x -> y -> x | (0,1,0) | 0 degrees | 0 | 0 |
| 21 forward circuits, 63 edges | (0,0,1) | +90 degrees modulo 360 | 2 | 4 |

The octant triangle has signed area pi/2 and unit Gaussian curvature; this agrees
with the rotation. Twenty-one circuits accumulate 21*pi/2 before reduction
modulo 2*pi. The returned angle alone cannot recover that winding count.

All eight dense/structured saved cases were checked instruction-by-instruction
against independent dense arithmetic, including intermediate states, loop
defects, trace agreement and diagnostic enclosure of the exact result. Producer
binary SHA256: `1ebb7a2a56340e37ea3442d07ff29ff42d8ee1eab75cee92ab8cd4ffc6d02653`.
The producer reports exact restart replay; this audit did not rerun the restart
or establish that executable's source provenance beyond the supplied receipts.

**Gate lesson:** reversing the middle Rx edge leaves the selected final tangent
y -> z unchanged, although the path and whole map are wrong. An initial
single-tangent mutation control in this audit exposed that blind spot; the
whole-map control catches it. DTP's saved example checks positions too. Keep
position, a tangent basis, intermediate tangency, and whole-map checks in future
fixtures, rather than relying on one plausible final vector.

### Structured maps and sensitivity

For row selectors, `y[i] = w[i] x[p[i]]`. For outer A and inner B:

```text
p_AB[i] = p_B[p_A[i]]
w_AB[i] = w_A[i] w_B[p_A[i]]
singular values = sorted absolute weights
||A-I||_F^2 = sum_i ((w_i-1)^2 if p_i=i else w_i^2+1).
```

The processor implements the correct order. Forty seeded checks span dimensions
2,3,8,32, lengths 1,2,8,32,64, signed maps and non-unit dyadic weights; independent
dense composition, SVD and defect calculations agree. This is an independent
formula check, not a substitute for exercising Rust on new random fixtures.

`proxy = Var(-log(sigma_i))` is log-stretch dispersion, not intrinsic curvature.
It is insensitive to uniform amplification: cI has proxy zero. Keep sigma_min
and sigma_max visible; introduce a separate gain limit if the application needs
one. A spectrum loses orientation and cannot replace the operator itself.

The stable-alpha overflow branch agrees with a 100-digit Decimal calculation
at the smallest positive binary64 epsilon: both yield
`6.41352604994935e-162` for the tested excess. This is an extreme-case check, not
a proof of all rounding behavior.

The v2 sensitivity estimates remain engineering diagnostics, not certified
intervals. Convergence, reconstruction residuals and orthogonality checks are
useful but do not certify nalgebra's total error. LAPACK-specific bounds cannot
be transferred without justification. See the
[LAPACK SVD error analysis](https://netlib.org/lapack/lug/node97.html).

**Next fidelity opportunity:** an exactly signed-permutation loop can compose
signs and indices exactly and compute its integer whole-map defect exactly.
Current weighted composition still adds roundoff padding, even for all +/-1
weights. Saved identity-loop receipts consequently have a positive upper error
bound; a zero closure threshold can refuse. This is conservative, not a wrong
acceptance. Extend exactness only after tests distinguish exactly signed,
near-unit, generic weighted and mixed paths; do not remove general uncertainty.
Arbitrary-vector state-error arithmetic remains a separate issue.

### Frame and factorized-object extensions

For orthonormal local frames Q_i,
`T_i' = Q_(i+1)^T T_i Q_i`, so `H' = Q_0^T H Q_0` on a closed loop.
For arbitrary S use `H' = S^-1 H S` and `G' = S^T G S`. A metric-aware squared
Hilbert-Schmidt defect is `tr(G'^-1 A^T G' A)`, A=H'-I. The audit checks the
orthonormal case and the 4 -> 6.25 -> 4 unit-rescaling counterexample.

Tensor-product operators are a reasonable next representation experiment:
`(A tensor B)(C tensor D) = AC tensor BD`. With column-major vectorization,
`(A tensor B) vec(X) = vec(B X A^T)`. Declare layout explicitly. Singular values
are pairwise products; log dispersion adds for the full multiset of nonzero
singular values. Product singularity thresholds still apply even when every
factor independently passes. The compact Frobenius identity using norms and
traces can suffer cancellation near identity; it needs its own error policy.
None of these tensor operations is claimed implemented by this review.

## Pass 2: GIGI integration and native alternatives

The current division of responsibility is accurate: real GIGI stores initial
programs and per-instruction output events; the new DTP Rust module computes.
The standalone loopback executable is not a new `gigi-stream` verb.

The store declares a text key, indexed run, numeric PC and JSON-text payload.
It explicitly sorts replay by PC. This is essential because text-key storage
does not imply insertion order. Heap `filtered_query_ex` uses the run index
through bitmap candidate selection (`src/bundle.rs:2675`). The mmap base branch
of `Engine::filtered_query` still scans its base; do not generalize indexed
heap performance to every storage mode. Measure cold history reads separately.

Event replay checks shape, sequence, output names, dimensions, numerical policy
and prior-state preservation. It does not recompute arithmetic or authenticate
plausible replacement numbers; the contract correctly discloses this. A one-run
cache is a disposable materialization, not the durable source. The sync-error
finding above is the remaining explicit recovery qualification.

| Native facility | Actual fit for this contract | Required boundary |
|---|---|---|
| `transport_rotation_matrix` / GQL `TRANSPORT_ROTATION` | Construct a plane rotation for validated endpoints and supplied angle; its restriction supplies shortest-arc sphere transport | Add connection/path/input checks; do not inherit antipodal identity or missing-component zero fill |
| `PERCEIVE` | Apply a supplied dense rotation and compute its closure norm | Native bias is unsquared Frobenius norm; DTP stores squared defect and guarded diagnostics. GQL path exposes the scalar, not the complete DTP state |
| `WINDOWED_COHERENCE` | Existing sequence-derived rotations and coherence | Different normalization, degeneracy and reporting contract; not a replacement for DTP control |
| Gauge edge connections / group composition | Connection-aware invertible group paths and reverse-edge convention | Explicit representation adapter; SU(2) is a double cover of SO(3), not the same matrix or error norm |
| Flat magnetic transport in `geometry/transport.rs` | A different dynamics model with magnetic field and initial velocity | Not the sphere's Levi-Civita connection; identical naming is not semantic equivalence |
| Native Vector fields / prepared-object patterns | Potential typed storage or validated reusable operators | Versioned migration, dimensions, policy, exact replay, cold recovery and cache authority must be gated first |
| Scalar variance-curvature / fitted Gaussian / shape verbs | Other statistical questions | Do not substitute them for supplied-operator spectrum, loop defect or connection curvature |

The rotation path was traced through parser dispatch, handler and bundle binding
(`src/parser.rs:2597`, `src/bin/gigi_stream.rs:15537` and `:15908`). It returns a
native flat Vector named `matrix`; an older nearby comment saying string
`matrix_flat` is stale. No new live GQL verification was performed in this review.

Current group-element source has U(1), SU(2) and SU(3) implementations; some
older stability comments are narrower. Inspect the actual operation and test
its needed representation, rather than treating a tier label as proof of a
drop-in match. For unit-quaternion q, SO(3)'s squared closure defect is
`8(1-q_0^2)`, not an unconverted SU(2) trace statistic. Check q/-q equivalence,
normalization, half-angle conventions, multiplication order and inverse edges.

## Pass 3: cost and evidence audit

The independent script recalculates medians/ratios from B01/B02 raw samples and
B03 phase shares from per-phase samples. It checks all 24 allocation rows and
unchanged output/work receipts. No new timings were taken.

| Evidence | Rechecked result | What it establishes |
|---|---|---|
| B01 2D closed loop | dense/structural median ratios 0.9066 and 0.9431 | Structural interpreter was slower in these two cases |
| B01 32D closed loop | ratios 4.2547 and 5.3055 | Faster structural representation in these particular whole-step comparisons |
| B02 allocation receipts | Exactly 3+3*edges fewer requests for compact loops; dense controls unchanged | Removal of specific temporary heap requests, not equal elapsed-time improvement |
| B02 paired compact timing | Per-case median ratios span 0.8654 to 1.1182 across the saved runs | Workload-dependent results, including regressions; no universal speed claim |
| B03 2D compact loop | Representation 92.962% / 93.649%; geometry 4.061% / 3.724% | Shares of instrumented measured sections only |
| B03 same loop size checks | 31.479% / 30.848% combined | Candidate optimization target, not a promised 31% speedup |
| B03 dense 32D spectrum | Geometry 33.946% / 33.550% | Larger dense spectrum still has substantial mathematical work |

B01 excludes input construction, HTTP and GIGI persistence. B03 excludes some
cleanup/instrumentation overhead from its section denominator; instrumented and
production timings differ. These receipts do not measure a GIGI or DHOOM storage
bottleneck. Work counters are named operations, not FLOPs, CPU instructions or
energy. Signed application still moves, indexes and writes coordinates.

The producer's root-test receipt reports 1,580 passed, zero failed, 22 ignored;
the handoff reports 45 DTP tests. This review does not rerun or newly certify
those suites. Source, binary and test-receipt correspondence must remain part of
any release gate, especially in this shared dirty worktree.

### Next bounded optimization: exact byte counting

Use the pinned serde_json serializer with a counting `io::Write` sink rather than
materializing a complete string merely to count bytes. The
[official `to_writer` contract](https://docs.rs/serde_json/latest/serde_json/fn.to_writer.html)
supports writing the serialized UTF-8 stream to a sink. Verify the pinned
serializer/formatter matches `Value::to_string()` exactly; do not sum field
lengths manually, approximate characters as bytes, or change refusal precedence.

Strict gates before acceptance:

1. Compare exact bytes counted against the old string length for nested objects,
   arrays, escaping, control characters, non-ASCII UTF-8, signed zero, subnormal
   and large exponents, all admitted numbers and every register/instruction kind.
2. Plant machines immediately below, at and above 262,144 serialized bytes.
   Exercise both input-size and post-instruction output-size refusals, retaining
   the exact error, PC and persisted trace. Boundary fixtures must be valid apart
   from the one size condition under test.
3. Count with checked arithmetic and preserve serializer failures. A character
   counter, off-by-one limit, skipped check and wrong formatter must each be
   caught by an assertion, not by a compilation error.
4. Compare full state/work/refusal outcomes against the current implementation.
   Demonstrate removed full-string allocations independently of timing.
5. Run paired same-process release measurements with unchanged controls and raw
   receipts; separately measure live HTTP plus sync before an end-to-end claim.
   Run all required Rust targets on the pinned candidate and check recovery.

Do not combine this small optimization with typed-state reuse. That second
change has ownership, validation, immutability, policy-version and durability
questions of its own.

### Fair higher-complexity comparison

Compare four explicitly labeled paths: dense interpreter, structural interpreter,
an equally structure-aware conventional CPU kernel, and prepared-object reuse.
Give each the same source information, accuracy/refusal policy and requested
output. Include preparation, validation, load, query, readout, sync and repeated
reuse separately. Report the reuse count where preparation pays for itself.

If only one transported vector is requested, sequential dense matrix-vector
application can avoid dense matrix-matrix composition; do not handicap that
baseline. The current loop also requests a whole-operator defect, which is a
different task and must be included when comparing it. Preserve intermediate
magnitude checks even if a later map cancels an oversized prefix.

A full n-vector output costs at least n output entries; a materialized n-by-n
matrix costs n squared. Constant-size answers from a previously validated
compact representation can be cheap, but do not imply zero construction,
validation, history, lookup or readout cost. CPUs/GPUs can also operate on compact
representations. These results establish neither hardware impossibility nor a
physical-DTP advantage.

Recommended next complexity sequence: exact signed-loop diagnostics, orthonormal
frame covariance, explicitly metric-aware unit changes, long paths within the
current 64-edge limit, then tensor-factor queries with independent dense small
oracles. A real calibrated sensor-frame or robotics/aerospace case should follow
the geometry gates. This sphere fixture is not a flight-qualification result.

## Status

- Independent sphere math, 24 non-axis ODE comparisons, 40 structural formula
  checks, eight saved sphere cases and benchmark-receipt arithmetic: passed.
- Whole-map wrong-direction negative control: caught; single-tangent blind spot
  documented rather than hidden.
- DTP/core implementation edits, new Rust tests, new live runs and new benchmarks:
  not performed by this review.
- Sync-failure injection, native connection adapter, byte-count optimization,
  exact signed-loop uncertainty and factorized operators: proposed gates, not
  completed functionality.

No DTP implementation, running server or deployment was changed.

## Subsequent producer update: G02

After the pinned review above, DTP reported new version-2 `compose_signed_loop`
and `power_signed` instructions, restricted to exact signed coordinate
permutations. Its receipt is `output/virtual-dtp/power-live/report.json`.
The producer reports eight circuit counts checked on all three basis vectors,
including 1,000,000,001 circuits represented by 3 build compositions, 43 power
compositions and 3 applications, with restart between saved power and use.
It also reports 576 small literal-iteration comparisons, domain/recovery tests,
51 passing crate tests and three caught mechanism-removal mutants.

These are **producer-reported, not independently checked by the audit above**;
that audit pins the earlier processor source. A subsequent review must check
composition order, exact domain, exponent limits and readout cost. This is a
final-map query, not preservation of an expanded per-edge trace, and not a
CPU/GPU advantage claim. DTP confirms uncertain-I/O recovery remains its owned,
unfinished follow-up. See also the separately gated GIGI-side
[checked sphere adapter results](../SPHERE_TRANSPORT_RESULTS.md).

The later producer update reports an isolated root run of 1,589 passed, zero
failed and 22 ignored, with its own private D: TEMP/TMP; earlier snapshot failures
were retained. Our separate isolated root run reached the same counts while
verifying the checked sphere adapter. This independently confirms that root
regression run on our working tree, not the new G02 mathematics or UI claims.
