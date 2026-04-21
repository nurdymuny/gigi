# FŪJIN — GNC Substrate on GIGI

**Working name.** Rename as needed. Suggested because it fits the Japanese aerospace mythology already established in the IZANAGI / KAGUYA / AMATERASU / TSUKUYOMI naming (Fūjin = wind god, master of flight and motion).

## Overview

FŪJIN is a Guidance, Navigation, and Control substrate that runs on GIGI. The thesis is that the controller geometry and the database geometry are the same geometry, and any GNC stack that treats compute and storage as separate substrates is losing information at the boundary every control cycle. FŪJIN eliminates that boundary: trajectories are sections, state history is sections, sensor fusion returns confidence natively, anomaly detection falls out of curvature, and the attitude representation is singularity-free by construction via the Double Cover Principle.

The controller's inner loop does not touch GIGI. The outer GNC stack — trajectory library, state history, sensor fusion, fault detection, digital twin comparison, replanning — is GIGI-native. Both ends of the stack speak fiber bundles.

## Module Location

```
gigi/
├── crates/
│   ├── gigi-core/              (existing)
│   ├── gigi-gql/               (existing)
│   ├── gigi-dhoom/             (existing)
│   └── fujin-gnc/              (NEW — this spec)
│       ├── src/
│       │   ├── lib.rs
│       │   ├── bundles/
│       │   ├── operations/
│       │   ├── controller/
│       │   ├── davis.rs
│       │   └── adapter.rs
│       ├── tests/
│       └── Cargo.toml
└── tests/
    ├── rust/
    └── python_math/
        └── fujin_validation/    (NEW — math-side TDD)
```

Pure Rust. Zero unsafe. Single dependency on `gigi-core`. No external GNC libraries, no matrix crates unless strictly necessary (small fixed-size linear algebra inlined).

## Field Equations — Where Each Piece Lives

**Double Cover `S + d² = 1`** — attitude bundle. Every rotational degree of freedom is carried as `(S, d) = (cos θ, sin θ)` on the unit circle, never as a scalar angle. For SO(3), carry quaternions under the same constraint `q · q = 1`. Guarantees singularity-free attitude across unlimited rotation. This is the piece that lets FŪJIN handle rocket-flip, multirotor-flip, swing-up-through-top maneuvers without special-casing.

**Davis Field Equation `C = τ/K`** — control allocation. `τ` is generalized force (torque demand), `K` is available control authority (effective stiffness), `C` is the curvature of the commanded trajectory at that point. Inverted: `τ_commanded = K · C_desired`. This is geometric feedforward. Used in `controller/allocation.rs`.

**Lift operator `Λ(A, B)`** — sensor fusion and tracking-error coupling. Two signals A and B projected onto a joint manifold with a connection. The connection 1-form evaluated at the current state is the fusion residual (sensor-to-sensor disagreement) or tracking error (commanded-vs-measured). Drive the connection flat → drive the error to zero. Used in `operations/fusion.rs` and `controller/tracking.rs`.

**Curvature `K`** — confidence and anomaly. GIGI already measures scalar curvature per region live. FŪJIN consumes it: `conf = 1/(1+K)` on every fused state, every trajectory lookup, every actuator residual read. When K spikes on a region, something off-manifold is happening there. Same primitive as KRAKEN SIGINT detection.

## Bundle Schema

Written in GQL against GIGI's existing BUNDLE / BASE / FIBER / MORPHISM / SECTION syntax.

### AttitudeBundle

Singularity-free rotational state. Planar case shown; SO(3) version swaps `(S, d)` for a unit quaternion.

```gql
BUNDLE AttitudeBundle {
    BASE  t: f64;                   // time parameter
    FIBER {
        S: f64;                     // cos(theta)
        d: f64;                     // sin(theta)
        omega: f64;                 // angular rate
    }
    INVARIANT S*S + d*d = 1.0 +/- 1e-9;   // double cover enforced at insert
}
```

**Insert contract:** on every insert, the engine verifies the double-cover invariant. Violations are rejected with a `DoubleCoverViolation` error, not silently renormalized (renormalization masks bugs in the upstream integrator).

### TrajectoryBundle

A planned trajectory is a section of this bundle. The base is the parameterization of the trajectory (arc length or time); the fiber is the full configuration.

```gql
BUNDLE TrajectoryBundle {
    BASE  s: f64;                   // parameter in [0, 1]
    FIBER {
        position: Vec3;
        attitude: AttitudeBundle;   // nested bundle
        velocity: Vec3;
        angular_velocity: Vec3;
        thrust_cmd: f64;
        torque_cmd: Vec3;
    }
}
```

### StateHistoryBundle

Continuously appended during flight. O(1) insert is a hard requirement — the control loop cannot stall on logging.

```gql
BUNDLE StateHistoryBundle {
    BASE  t: f64;                   // timestamp, monotonic
    FIBER {
        measured_state: ConfigVector;
        fused_state: ConfigVector;
        fused_conf: f64;            // 1/(1+K), baked in
        control_commanded: ControlVector;
        control_realized: ControlVector;
        sensor_packet: SensorPacket;
    }
    INSERT_COMPLEXITY O(1);
}
```

### ReferenceLibraryBundle

Known-good trajectories indexed for O(1) nearest-neighbor lookup by start point, goal point, or maneuver class.

```gql
BUNDLE ReferenceLibraryBundle {
    BASE  maneuver_id: u64;
    FIBER {
        class: ManeuverClass;       // hover | traverse | flip | land | ...
        start_region: Region;
        goal_region: Region;
        trajectory: SECTION OF TrajectoryBundle;
        validated: bool;            // has this been flown successfully
        confidence_prior: f64;
    }
    MORPHISM pullback_by_goal(goal: Region) -> Vec<SECTION>;
    MORPHISM pullback_by_start(start: Region) -> Vec<SECTION>;
}
```

### SensorFusionBundle

Multi-sensor fused state with the fusion residual (connection 1-form) exposed directly.

```gql
BUNDLE SensorFusionBundle {
    BASE  t: f64;
    FIBER {
        imu:   SensorReading;
        gps:   Option<SensorReading>;
        baro:  Option<SensorReading>;
        vision: Option<SensorReading>;
        fused_state: ConfigVector;
        residual: f64;              // ||connection 1-form||
        K_local: f64;               // scalar curvature at this base point
        conf: f64;                  // 1/(1+K_local)
    }
}
```

### ActuatorStateBundle

Commanded vs. realized control, with residual for fault detection.

```gql
BUNDLE ActuatorStateBundle {
    BASE  t: f64;
    FIBER {
        actuator_id: u32;
        commanded: f64;
        realized: f64;
        residual: f64;              // commanded - realized
        K_local: f64;
        healthy: bool;              // K_local < threshold
    }
}
```

## Core API — Rust Traits

```rust
// crates/fujin-gnc/src/lib.rs

pub trait FujinEngine {
    /// O(1) append to state history. MUST NOT block the caller longer than
    /// the configured control-loop budget (default 100 microseconds).
    fn record_state(&self, t: f64, state: &State, control: &Control,
                    packet: &SensorPacket) -> Result<(), FujinError>;

    /// Fuse sensors into a single state estimate with confidence.
    /// Returns (fused_state, conf = 1/(1+K)).
    fn fuse(&self, packet: &SensorPacket) -> Result<FusedState, FujinError>;

    /// O(|current|) nearest-reference lookup. Returns candidate trajectories
    /// whose start region contains the current state.
    fn nearest_reference(&self, current: &State, class: Option<ManeuverClass>)
        -> Result<Vec<ReferenceMatch>, FujinError>;

    /// Scan the most recent window for anomalies. Returns axis of deviation
    /// if detected (per-channel, not just a scalar flag).
    fn detect_anomaly(&self, window: Duration) -> Result<Option<Anomaly>, FujinError>;

    /// Check actuator health from the residual stream.
    fn actuator_health(&self, id: u32) -> Result<ActuatorHealth, FujinError>;

    /// Plan a trajectory as a geodesic on the configuration bundle.
    fn plan(&self, start: &State, goal: &State, constraints: &Constraints)
        -> Result<TrajectorySection, FujinError>;
}

pub trait AttitudeOps {
    /// Compute commanded attitude (S*, d*) from desired thrust vector.
    /// No atan2, no angle unwrapping.
    fn commanded_attitude(&self, desired_thrust: &Vec3) -> (f64, f64);

    /// Connection 1-form: geometric error between current (S, d) and
    /// desired (S*, d*). This IS the attitude error — no special cases
    /// at +/- pi.
    fn attitude_error(&self, current: (f64, f64), desired: (f64, f64)) -> f64;

    /// Evolve (S, d) forward by angular rate omega over dt, preserving
    /// S + d² = 1 to machine precision.
    fn integrate(&self, state: (f64, f64), omega: f64, dt: f64) -> (f64, f64);
}

pub trait ControlAllocation {
    /// Davis Field Equation inverted: tau = K * C_desired.
    /// Returns torque command given desired trajectory curvature.
    fn allocate(&self, C_desired: f64, K_available: f64) -> f64;
}
```

## Rust Module Map

```
src/lib.rs                         — public API, re-exports, engine trait
src/bundles/attitude.rs            — AttitudeBundle + double-cover invariant checker
src/bundles/trajectory.rs          — TrajectoryBundle + section operations
src/bundles/state_history.rs       — StateHistoryBundle + O(1) append
src/bundles/reference_library.rs   — ReferenceLibraryBundle + pullback morphisms
src/bundles/sensor_fusion.rs       — SensorFusionBundle
src/bundles/actuator_state.rs      — ActuatorStateBundle
src/operations/fusion.rs           — Lift operator implementation, connection 1-form
src/operations/geodesic.rs         — trajectory planning as geodesic flow
src/operations/curvature.rs        — K extraction, conf = 1/(1+K)
src/operations/pullback.rs         — trajectory pullback by start/goal
src/controller/attitude_ops.rs     — AttitudeOps trait impl
src/controller/allocation.rs       — C = tau/K implementation
src/controller/tracking.rs         — tracking-error bundle
src/davis.rs                       — field equations, invariant checks, constants
src/adapter.rs                     — live-system adapter (the shim between the
                                     real-time controller and GIGI)
src/error.rs                       — FujinError enum
```

## The Adapter

The adapter is the only piece of FŪJIN that runs inside the real-time control loop. Its job is to be fast, non-blocking, and to never stall the controller.

```rust
pub struct FujinAdapter {
    engine: Arc<dyn FujinEngine>,
    write_buffer: Arc<Mutex<RingBuffer<StateRecord>>>,
    background_flush: thread::JoinHandle<()>,
}

impl FujinAdapter {
    /// Called at control loop rate (100-1000 Hz). MUST return in under
    /// 10 microseconds. Writes to ring buffer; background thread flushes
    /// to GIGI at a lower rate.
    pub fn observe(&self, t: f64, state: &State, control: &Control,
                   packet: &SensorPacket) {
        let record = StateRecord::new(t, state, control, packet);
        let _ = self.write_buffer.lock().unwrap().push(record);
        // non-blocking: if buffer is full, drop the record rather than stall
    }

    /// Called by the controller to get the latest fused state + conf.
    /// Reads from GIGI's most recent SensorFusionBundle section.
    pub fn latest_fusion(&self) -> FusedState { /* ... */ }
}
```

Design rule: the controller calls `observe` and `latest_fusion`. Nothing else. Everything else (anomaly detection, reference lookup, replanning) runs in a supervisory thread at a lower rate and communicates with the controller via a `ControlDirective` channel (e.g., "switch to safe mode," "replan to goal Y").

## Performance Targets

Match GIGI's existing performance profile. Any regression is a bug.

| Operation | Target | Measured where |
|-----------|--------|----------------|
| `observe` (ring buffer push) | < 1 μs | Rust bench |
| `record_state` (ring flush to GIGI) | < 10 μs batched | Rust bench |
| `fuse` | < 50 μs | Rust bench |
| `nearest_reference` | < 100 μs | Rust bench |
| `detect_anomaly` on 1s window | < 500 μs | Rust bench |
| AttitudeBundle point query | ~500 ns (GIGI baseline) | GIGI bench |
| `allocate` | < 100 ns | Rust bench |
| Double cover invariant check | < 20 ns | Rust bench |

All targets assume a single host, no network. Numbers scale with GIGI's existing per-region performance and are not expected to degrade with history size (O(1) guarantees).

## Test Specifications — TDD

### Rust Tests (`tests/rust/`)

Minimum set. Each test file is a gate — nothing proceeds without its tests green.

```
tests/rust/
├── attitude_double_cover.rs       — invariant enforcement, 2π wrap, anti-pole
├── attitude_flip.rs               — simulate 360° flip, assert no discontinuity
├── trajectory_section.rs          — section insert, read, pullback
├── state_history_o1.rs            — assert O(1) insert across 1K, 10K, 100K
├── fusion_confidence.rs           — conf decreases as sensors disagree
├── fusion_residual_zero.rs        — agreeing sensors produce ~0 residual
├── anomaly_drift.rs               — inject drift, assert detection
├── anomaly_actuator.rs            — inject actuator fault, assert detection
├── reference_pullback.rs          — nearest-reference correctness
├── control_allocation.rs          — C = τ/K identity across parameter sweep
├── geodesic_endpoint.rs           — planned trajectory hits the goal
├── adapter_nonblocking.rs         — observe() never exceeds budget
└── integration_multirotor.rs      — full loop: fuse → observe → control → fly
```

### Python Math Validation (`tests/python_math/fujin_validation/`)

Every bundle and operation has a math-side test in Python that does the same calculation with NumPy/SymPy and asserts the Rust output matches to within a stated tolerance. 104 tests is the GIGI precedent; FŪJIN should add at least 40 more.

Key validation files:
```
tests/python_math/fujin_validation/
├── test_double_cover_identity.py       — S² + d² = 1 preserved under integration
├── test_davis_field_equation.py        — C = τ/K inversion round-trip
├── test_connection_1form.py            — Lift operator matches analytic form
├── test_curvature_scalar.py            — K from Rust matches sympy Ricci scalar
├── test_geodesic_planar.py             — planar geodesic matches straight line
├── test_geodesic_so3.py                — SO(3) geodesic matches slerp
└── test_confidence_bayes_equivalence.py — conf = 1/(1+K) matches Bayesian posterior
                                           under Gaussian assumption (sanity)
```

### Integration Benchmarks

Port the janismac ControlChallenges multirotor dynamics into a Rust test harness and run FŪJIN against:

1. `MultirotorIntro` — target hover, must stabilize under disturbance
2. `MultirotorObstacles` — target traverse, must replan around static obstacles
3. `MultirotorFlip` — 360° flip, must remain controllable throughout

The third is the decisive one. A controller that handles MultirotorFlip without special-casing the flip is the proof that the double cover is load-bearing, not ornamental.

## Reference Controller Implementation

Minimum controller — planar multirotor, single file, uses only FŪJIN primitives. Serves as the reference for what the adapter-to-controller interface looks like.

```rust
// examples/multirotor_controller.rs

use fujin_gnc::prelude::*;

pub fn control_step(
    adapter: &FujinAdapter,
    target: &Target,
    dt: f64,
) -> ControlOutput {
    // Pull latest fused state from GIGI via adapter
    let fused = adapter.latest_fusion();
    let (s, d) = fused.attitude;          // (cos θ, sin θ) — double cover

    // Position PD
    let ex = target.x - fused.pos.x;
    let ey = target.y - fused.pos.y;
    let ax_des = KP * ex - KD * fused.vel.x;
    let ay_des = KP * ey - KD * fused.vel.y + G;

    // Commanded attitude from desired thrust vector
    let a_mag = (ax_des * ax_des + ay_des * ay_des).sqrt() + 1e-9;
    let s_star = ay_des / a_mag;
    let d_star = -ax_des / a_mag;

    // Attitude error = connection 1-form (NOT angle difference)
    let att_err = s * d_star - d * s_star;

    // Davis Field: τ = K · C
    // Here K is available control authority, C is attitude-error-derived curvature
    let tau = ControlAllocationImpl::allocate(
        -K_ATT * att_err - K_RATE * fused.angular_rate,
        K_AVAILABLE,
    );

    // Total thrust is the projection of desired accel onto body up
    let thrust_total = (ax_des * d + ay_des * s).max(0.0);

    // Differential allocation
    let u_left  = 0.5 * thrust_total - tau;
    let u_right = 0.5 * thrust_total + tau;

    ControlOutput { u_left, u_right }
}
```

## Integration Example — Full Flight Stack

```rust
fn main() -> Result<(), FujinError> {
    let gigi = GigiEngine::open("flight.gigi")?;
    let fujin = Arc::new(FujinEngineImpl::new(gigi));
    let adapter = FujinAdapter::new(fujin.clone());

    // Background supervisor — replanning, anomaly detection
    let supervisor = std::thread::spawn({
        let fujin = fujin.clone();
        move || supervisor_loop(fujin, /* 10 Hz */)
    });

    // Real-time control loop — 500 Hz
    let mut t = 0.0;
    while running() {
        let packet = read_sensors();
        let state = state_from_sensors(&packet);
        let control = control_step(&adapter, &current_target(), DT);
        emit_to_motors(&control);
        adapter.observe(t, &state, &control, &packet);
        sleep_until(t + DT);
        t += DT;
    }

    supervisor.join().unwrap();
    Ok(())
}

fn supervisor_loop(fujin: Arc<dyn FujinEngine>, rate_hz: u32) {
    loop {
        // Anomaly scan on recent history
        if let Some(anomaly) = fujin.detect_anomaly(Duration::from_secs(1)).unwrap() {
            route_to_safe_mode(anomaly);
        }

        // Actuator health check
        for id in 0..N_MOTORS {
            if !fujin.actuator_health(id).unwrap().healthy {
                degraded_control_mode(id);
            }
        }

        std::thread::sleep(Duration::from_millis(1000 / rate_hz as u64));
    }
}
```

## Open Questions

Decisions to resolve during implementation, not in the spec.

1. **SO(3) attitude representation.** Unit quaternions under `q·q = 1` are the obvious double-cover choice. Alternative: rotation matrices on SO(3) with `R^T R = I` as the invariant. Quaternions are more compact and computationally cheaper but require renormalization discipline. Recommend quaternions, revisit if numerical conditioning issues show up on aggressive maneuvers.

2. **Geodesic solver for trajectory planning.** Analytic geodesic for flat manifolds (straight line), slerp for SO(3), but full configuration-manifold geodesic on SE(3) with obstacles is a nonlinear boundary value problem. Start with shooting method; upgrade to collocation if convergence is slow.

3. **Anomaly detection threshold.** `K_local > threshold` flags an anomaly. The threshold should be learned per-vehicle-class from the ReferenceLibraryBundle rather than hardcoded. First implementation: hardcode to `K_threshold = 0.1` and expose via config; learn later.

4. **Ring buffer sizing in the adapter.** At 500 Hz with 200-byte state records, a 1-second buffer is 100 KB. Default to 10 seconds (1 MB) — large enough that GIGI flush latency spikes never cause drops, small enough that we don't page out.

5. **Sensor fusion algorithm.** The Lift operator `Λ(A, B)` defines the fusion mathematically. First implementation: pairwise lift into a combined manifold, iterative for N sensors. Optimization opportunity later if N is large.

6. **Does FŪJIN need its own REPL?** GIGI has one. Arguments for a FŪJIN-specific REPL: GNC-specific commands (`simulate`, `replay`, `plot_trajectory`). Arguments against: scope creep. Defer.

## Implementation Order

For a Copilot session walking this top to bottom.

1. `src/davis.rs` — constants, invariant checkers, field equation primitives. No dependencies. Tests: `test_double_cover_identity.py`, `test_davis_field_equation.py`.
2. `src/bundles/attitude.rs` — AttitudeBundle with invariant enforcement. Tests: `attitude_double_cover.rs`, `attitude_flip.rs`.
3. `src/controller/attitude_ops.rs` — `commanded_attitude`, `attitude_error`, `integrate`. Tests: part of `attitude_flip.rs`.
4. `src/controller/allocation.rs` — `allocate`. Tests: `control_allocation.rs`.
5. `src/bundles/state_history.rs` — O(1) append. Tests: `state_history_o1.rs`.
6. `src/adapter.rs` — ring buffer + background flush. Tests: `adapter_nonblocking.rs`.
7. `examples/multirotor_controller.rs` — reference controller. Validates that 1–6 compose correctly.
8. `src/bundles/sensor_fusion.rs` + `src/operations/fusion.rs` — Lift operator, residual, confidence. Tests: `fusion_confidence.rs`, `fusion_residual_zero.rs`.
9. `src/bundles/actuator_state.rs` + actuator health — curvature on the residual stream. Tests: `anomaly_actuator.rs`.
10. `src/operations/curvature.rs` + `detect_anomaly` — generalized anomaly detection on any bundle. Tests: `anomaly_drift.rs`.
11. `src/bundles/trajectory.rs` + `src/bundles/reference_library.rs` + pullback — trajectory storage and nearest-reference lookup. Tests: `trajectory_section.rs`, `reference_pullback.rs`.
12. `src/operations/geodesic.rs` — trajectory planning. Tests: `geodesic_endpoint.rs`, `geodesic_planar.py`, `geodesic_so3.py`.
13. `tests/rust/integration_multirotor.rs` — full-loop test against ported ControlChallenges dynamics. This is the gate for declaring FŪJIN v0.1 ready.

## Invariants That Must Hold

These are assertions the engine enforces regardless of caller. Violating any of them is a bug, not a configuration choice.

- AttitudeBundle sections always satisfy `|S² + d² - 1| < 1e-9`, or `|q·q - 1| < 1e-9` for quaternions. Check on every insert.
- StateHistoryBundle insert is O(1). Benchmark regression blocks merge.
- Confidence `conf = 1/(1+K)` is always in `(0, 1]`. Negative K is a bug upstream.
- The Lift operator is symmetric: `Λ(A, B) = Λ(B, A)`. Tested.
- Geodesic endpoints are exact: planned trajectory section evaluated at `s = 0` equals start, at `s = 1` equals goal, to within solver tolerance.
- Anomaly detection is per-axis, not just a scalar flag. The engine returns which channel(s) contributed to the K spike.

## What This Is Not

FŪJIN is not a replacement for a full flight management system. It does not implement mission planning, communication protocols, payload management, or hardware abstraction for specific airframes. It is a substrate — the geometric foundation that a flight management system would sit on top of. Specifically out of scope for v0.1:

- Avionics hardware drivers
- Mission planning GUI
- Telemetry downlink protocols (separate from DHOOM)
- Specific vehicle dynamics libraries (vehicle models are provided by the caller)
- Multi-vehicle coordination (single-vehicle only in v0.1)

Multi-vehicle coordination is a natural v0.2 feature because a formation is a section on a product bundle, and relative-state anomalies drop out of curvature exactly like single-vehicle fault detection. Flagged for later.

## Why This Architecture

Every GNC engineer has had the experience of losing information at the compute/storage boundary. Kalman filter returns a state estimate with a covariance; the covariance gets stripped when the estimate lands in the database, because the database stores floats. Trajectory library is a list of waypoints; the tangent structure of the planned trajectory gets lost because the database doesn't know what a tangent is. Attitude gets logged as Euler angles because that's what the database schema supports, and then wraps-around show up as discontinuities in post-flight analysis.

FŪJIN eliminates that loss. The covariance is curvature, stored natively. The tangent structure is part of the section, stored natively. Attitude is double-covered, stored natively. Nothing is marshaled across a boundary because both ends of the stack — controller and database — speak fiber bundles.

This is the architecture you get when the math is load-bearing all the way down.
