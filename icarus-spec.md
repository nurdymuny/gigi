# ICARUS

**Integrated Curvature-Aware Routing & Uncertainty Substrate**

*Guidance, Navigation, and Control as GQL queries against the fiber bundle database engine.*

Author: Bee Rosa Davis · Davis Geometric
Status: working spec
Depends on: GIGI Stream (with the extensions enumerated in `icarus-gigi-requests.md`), GQL v2.1

## What ICARUS Is

ICARUS — **I**ntegrated **C**urvature-**A**ware **R**outing & **U**ncertainty **S**ubstrate — is a flight control architecture in which the entire GNC stack — state estimation, envelope awareness, trajectory planning, tracking, fault detection, digital twin consistency, regime classification, forecasting, and safety monitoring — is implemented as GQL queries against bundles stored in GIGI. There is no bespoke GNC math library. There is no separate trajectory planner, no separate Kalman filter, no separate anomaly detector. Every one of those operations is either a GQL primitive that already exists or one of a small set of engine extensions enumerated in the companion GIGI requests letter. ICARUS is the schema, the query set, and the thin Rust adapter that binds a real-time control loop to the database.

The name is not decorative. The thesis embedded in it is the primary patent claim.

## The ICARUS Claim

The name encodes the claim. **I**ntegrated — the whole GNC stack is one substrate, not four adapted layers. **C**urvature-**A**ware — the vehicle measures the local geometry of its own capability manifold as a first-class operation. **R**outing — trajectories are geodesics on that manifold, queried not computed. **U**ncertainty — confidence is a derived quantity `1/(1+K)`, carried through every operation, never stripped at a layer boundary. **S**ubstrate — the database is load-bearing infrastructure.

**A flight control architecture in which the vehicle's awareness of its own operational envelope is computed as a local holonomy measurement on a capability fiber bundle whose fiber carries both configuration state and reachable-next-state bounds, wherein attitude is encoded as a unit quaternion on S³ providing a genuine double cover of SO(3) and therefore singularity-free rotation through arbitrary angles, and controller aggressiveness is gated by the confidence score `1/(1+K)` derived from that measurement, such that the vehicle degrades gracefully toward known-safe trajectory regions as the fiber curvature near its current state increases, while preserving a floor of recoverable control authority at maximum envelope violation.**

Icarus flew too close to the sun because he had no way to measure how close he was. ICARUS does.

## Why GIGI Is Load-Bearing

Every GNC operation in ICARUS corresponds to a GQL primitive. This is not a list of features GIGI happens to have; this is a list of operations ICARUS cannot run without.

| GNC Operation | GQL Primitive |
|---|---|
| State estimation (fused-state read) | `SECTION sensor_fusion AT t = @latest_t` — O(1) point query |
| Fusion residual (covariance analog) | Residual stored as fiber field at ingest; `CURVATURE sensor_fusion` provides geometric equivalent of filter covariance |
| Envelope awareness | `HOLONOMY capability NEAR (...) WITHIN r ON FIBER (...) AROUND regime` |
| Trajectory planning | `GEODESIC ... FROM (...) TO (...)` on a traversable subspace defined by predicate-sourced `COVER` |
| Tracking error | `TRANSPORT trajectory FROM (...) TO (...) ON FIBER (q0, q1, q2, q3)` |
| Actuator fault detection | `SUBSCRIBE ANOMALIES actuator_state ON residual` |
| Digital twin consistency | `GAUGE current_flight VS reference_flight ON FIBER (...) AROUND class` (via LENS projections) |
| Regime classification | `SPECTRAL state_history ON FIBER (...) MODES k` — spectral gap tracking |
| Forecasting | `PREDICT state_history ON x BY t` |
| Curvature and phase drift warning | `SUBSCRIBE CURVATURE state_history ON fused_conf DRIFT > ε`, `SUBSCRIBE PHASE state_history ON fused_conf` |
| Distribution drift across sensors | `DIVERGENCE FROM sensor_now TO sensor_reference` |
| Envelope confidence | `CONFIDENCE() ... WHERE CONFIDENCE() >= threshold` inline in any COVER |

Several of these (unit-quaternion INVARIANT enforcement, 4×4 TRANSPORT on S³, predicate-sourced COVER with EXISTS subquery, batch SECTIONS UPSERT, multi-field GEODESIC endpoints, inline RESTRICT TO on GEODESIC, and K as a subscription pseudo-field) rely on engine extensions enumerated in `icarus-gigi-requests.md`.

## Bundle Schema

Eight bundles. Each bundle's BASE and FIBER are specified. Field types are drawn from GIGI's type system (NUMERIC, CATEGORICAL, TEXT, BOOLEAN, TIMESTAMP).

### TerrainBundle

The world the vehicle flies through. Static or slowly-updated obstacles, no-fly zones, elevation, ambient wind, threat volumes.

```sql
BUNDLE terrain
  BASE (x NUMERIC, y NUMERIC, z NUMERIC)
  FIBER (
    occupied      BOOLEAN INDEX,
    elevation     NUMERIC RANGE 20000,
    wind_x        NUMERIC RANGE 100,
    wind_y        NUMERIC RANGE 100,
    wind_z        NUMERIC RANGE 50,
    threat_prob   NUMERIC RANGE 1 INDEX,
    no_fly        BOOLEAN INDEX,
    source        CATEGORICAL,
    updated_at    TIMESTAMP
  )
  OPTIONS (STORAGE AUTO, TOLERANCE 0.1);
```

Update contract: static layers written once at boot via `INGEST`. Semi-dynamic layers (threat updates, wind forecasts) via batch `SECTIONS ... UPSERT`. Dynamic sensor layers (perceived obstacles) via streaming NDJSON from the sensor thread.

### CapabilityBundle

The vehicle's operational envelope as a function of state. The fiber carries both the state coordinates (so HOLONOMY NEAR can search by proximity in state space) and the reachable-next-state bounds (the capability information proper). This is the bundle ICARUS monitors holonomy on.

```sql
BUNDLE capability
  BASE (state_hash NUMERIC)
  FIBER (
    x             NUMERIC INDEX,
    y             NUMERIC INDEX,
    z             NUMERIC INDEX,
    vx            NUMERIC INDEX,
    vy            NUMERIC INDEX,
    vz            NUMERIC INDEX,
    q0            NUMERIC,
    q1            NUMERIC,
    q2            NUMERIC,
    q3            NUMERIC,
    vx_max        NUMERIC,
    vy_max        NUMERIC,
    vz_max        NUMERIC,
    ang_rate_max  NUMERIC,
    thrust_margin NUMERIC RANGE 1,
    regime        CATEGORICAL INDEX,
    linearization_valid BOOLEAN INDEX,
    validated     BOOLEAN
  )
  INVARIANT q0*q0 + q1*q1 + q2*q2 + q3*q3 = 1.0 +/- 1e-9;
```

The `state_hash` base provides O(1) point access when the controller already knows which state point it's interrogating. The INDEX-modified coordinate fields let HOLONOMY NEAR perform the proximity neighbourhood search. The unit-quaternion INVARIANT (engine extension, see GIGI requests) is checked on every insert; violations are rejected rather than silently renormalized, because silent renormalization masks integrator bugs.

### TrajectoryBundle

Planned trajectories as sections. Base parameter s ∈ [0,1]; fiber is full configuration with attitude as unit quaternion on S³.

```sql
BUNDLE trajectory
  BASE (traj_id NUMERIC, s NUMERIC)
  FIBER (
    x NUMERIC, y NUMERIC, z NUMERIC,
    vx NUMERIC, vy NUMERIC, vz NUMERIC,
    q0 NUMERIC, q1 NUMERIC, q2 NUMERIC, q3 NUMERIC,
    omega_x NUMERIC, omega_y NUMERIC, omega_z NUMERIC,
    thrust_cmd NUMERIC,
    torque_x_cmd NUMERIC,
    torque_y_cmd NUMERIC,
    torque_z_cmd NUMERIC,
    planned_at TIMESTAMP,
    class CATEGORICAL INDEX
  )
  INVARIANT q0*q0 + q1*q1 + q2*q2 + q3*q3 = 1.0 +/- 1e-9;
```

The quaternion `(q0, q1, q2, q3)` is the double cover of SO(3). Attitude is therefore singularity-free across arbitrary rotation — the vehicle can flip, invert, tumble, and return without the representation ever wrapping, gimbal-locking, or discontinuing.

### ReferenceLibraryBundle

Known-good trajectories indexed for geodesic lookup.

```sql
BUNDLE reference_library
  BASE (ref_id NUMERIC AUTO)
  FIBER (
    class       CATEGORICAL INDEX,
    start_x NUMERIC, start_y NUMERIC, start_z NUMERIC,
    goal_x  NUMERIC, goal_y  NUMERIC, goal_z  NUMERIC,
    traj_id     NUMERIC,
    validated   BOOLEAN,
    last_flown  TIMESTAMP,
    success_count NUMERIC,
    confidence_prior NUMERIC
  );
```

### StateHistoryBundle

Continuously appended during flight. O(1) insert per record.

```sql
BUNDLE state_history
  BASE (t NUMERIC)
  FIBER (
    x NUMERIC, y NUMERIC, z NUMERIC,
    vx NUMERIC, vy NUMERIC, vz NUMERIC,
    q0 NUMERIC, q1 NUMERIC, q2 NUMERIC, q3 NUMERIC,
    omega_x NUMERIC, omega_y NUMERIC, omega_z NUMERIC,
    thrust_cmd NUMERIC,
    torque_x_realized NUMERIC,
    torque_y_realized NUMERIC,
    torque_z_realized NUMERIC,
    fused_conf NUMERIC,
    class CATEGORICAL INDEX
  )
  OPTIONS (STORAGE SEQUENTIAL, WAL ENABLED)
  INVARIANT q0*q0 + q1*q1 + q2*q2 + q3*q3 = 1.0 +/- 1e-9;
```

SEQUENTIAL storage is correct here — monotonic arithmetic base, K=0 on the base space, O(1) sequential append. The `class` field aligns with `trajectory.class` and `reference_library.class` for GAUGE comparisons. The capability bundle's `regime` field is the morphism target for this `class`.

### SensorFusionBundle

Per-sensor readings and the fused result. The residual and confidence are computed by the sensor thread at ingest time and stored as fiber fields — the sensor thread is the one module in ICARUS that is not pure GQL, because Kalman-style fusion is preprocessing that produces the inputs GIGI then treats as native geometry. Once written, GIGI's `CURVATURE` on the fused values is the covariance analog.

```sql
BUNDLE sensor_fusion
  BASE (t NUMERIC)
  FIBER (
    imu_x NUMERIC, imu_y NUMERIC, imu_z NUMERIC,
    gps_x NUMERIC, gps_y NUMERIC, gps_z NUMERIC,
    baro_z NUMERIC,
    vision_x NUMERIC, vision_y NUMERIC, vision_z NUMERIC,
    fused_x NUMERIC, fused_y NUMERIC, fused_z NUMERIC,
    residual NUMERIC,
    fused_conf NUMERIC
  );
```

### ActuatorStateBundle

Commanded vs. realized control per actuator, per timestep. Anomalies here drive the fault detector.

```sql
BUNDLE actuator_state
  BASE (t NUMERIC, actuator_id NUMERIC)
  FIBER (
    commanded NUMERIC,
    realized  NUMERIC,
    residual  NUMERIC,
    healthy   BOOLEAN INDEX
  );
```

### CommandLogBundle

Every control output, with a text record of the GQL query that produced it.

```sql
BUNDLE command_log
  BASE (t NUMERIC)
  FIBER (
    thrust_cmd NUMERIC,
    torque_x_cmd NUMERIC,
    torque_y_cmd NUMERIC,
    torque_z_cmd NUMERIC,
    plan_ref_id NUMERIC,
    envelope_conf NUMERIC,
    controller_mode CATEGORICAL,
    query_text TEXT
  );
```

The `query_text` field stores the GQL statement that drove each control output, making post-flight replay and debugging a straightforward COVER on the log.

## Morphisms

Foreign-key-like relationships that GIGI enforces on PULLBACK joins:

- `trajectory.class` ↔ `reference_library.class` — pullback to find references of the current maneuver class.
- `reference_library.traj_id` ↔ `trajectory.traj_id` — reference library points into the trajectory bundle.
- `state_history.class` ↔ `capability.regime` — flight regime joins to envelope regime. The field name difference is intentional: `regime` is the capability bundle's term for it; `class` is the shared name across trajectory, reference, and state.
- `actuator_state.t` ↔ `state_history.t` — actuator events sync to state timestamps.
- `sensor_fusion.t` ↔ `state_history.t` — fused state sync.
- `command_log.plan_ref_id` ↔ `reference_library.ref_id` — commands trace back to their reference.

## The Nine Canonical Operations

Each of these is one or two GQL statements.

### 1. State estimation (fast path)

The sensor thread writes the fused state into `sensor_fusion` with residual and confidence pre-computed at ingest. The controller reads it:

```sql
SECTION sensor_fusion AT t = @latest_t
  PROJECT (fused_x, fused_y, fused_z, fused_conf);
```

O(1) point query. The filtering step that produces the fused estimate from raw sensors runs in the sensor thread before the write. GIGI's curvature on the `sensor_fusion` bundle is the geometric equivalent of the filter's covariance: high curvature means sensors disagree strongly at that point, which is what a large covariance means in the Kalman formulation. Once the fused estimate is in GIGI, everything downstream — ICARUS and anything else — consumes it as a native geometric section with confidence attached.

### 2. Envelope awareness (supervisor)

**The ICARUS move.** Is the vehicle in a region of the capability bundle where known-good behavior is geometrically consistent?

```sql
HOLONOMY capability
  NEAR (x = @current_x, y = @current_y, z = @current_z,
        vx = @current_vx, vy = @current_vy, vz = @current_vz)
  WITHIN @envelope_radius
  ON FIBER (x, y, z, vx, vy, vz)
  AROUND regime;
```

The NEAR clause and the ON FIBER clause reference the same fiber fields — the state coordinates that `CapabilityBundle` carries explicitly to support proximity search. Returns `{ local_holonomy_angle, neighbourhood_size }`.

Controller gain is modulated:

```
controller_gain = nominal_gain / (1 + |local_holonomy_angle|)
```

Because `local_holonomy_angle ∈ [0, 2π]`, the minimum gain is `nominal_gain / (1 + 2π) ≈ 0.137 · nominal_gain`. This floor is deliberate: a controller that can fully disable itself is a controller that can drop the vehicle. ICARUS biases toward recoverable degraded flight rather than unrecoverable shutdown. The envelope response curve is nonlinear in `|holonomy|`, and its shape is tuned through `envelope_radius` (private document).

### 3. Trajectory planning (supervisor)

The intent is to return a geodesic through the subspace of capability points that are reachable, obstacle-free, and inside the linearization regime. A predicate-sourced COVER (engine extension) restricts capability to the traversable subspace, then GEODESIC on the result.

```sql
WITH traversable AS (
  COVER capability
  WHERE linearization_valid = TRUE
    AND EXISTS (
      COVER terrain
      ON x = capability.x AND y = capability.y AND z = capability.z
      WHERE occupied = FALSE AND no_fly = FALSE
    )
)
GEODESIC traversable
  FROM (x = @start_x, y = @start_y, z = @start_z,
        vx = @start_vx, vy = @start_vy, vz = @start_vz)
  TO   (x = @goal_x,  y = @goal_y,  z = @goal_z,
        vx = 0, vy = 0, vz = 0);
```

The nested EXISTS block is the predicate joining capability to terrain by shared coordinates. The result is a section of a virtual traversable bundle; GEODESIC returns the shortest path as a trajectory section. Both the predicate-sourced COVER and the multi-field GEODESIC lookup are engine extensions in the GIGI requests letter.

### 4. Tracking error (fast path)

Attitude error is parallel transport on the quaternion bundle:

```sql
TRANSPORT trajectory
  FROM (traj_id = @active_plan, s = @commanded_s)
  TO   (traj_id = @active_plan, s = @achieved_s)
  ON FIBER (q0, q1, q2, q3);
```

Returns the rotation between commanded and achieved attitude as a transport on S³ (engine extension: current GIGI returns a 2×2 matrix regardless of fiber dimensionality; ICARUS requires 4×4 on unit-quaternion fibers). The returned rotation gives the controller its tracking input. No Euler angles, no atan2, no singularity at any rotation angle.

### 5. Actuator fault detection (streaming)

```sql
SUBSCRIBE ANOMALIES actuator_state ON residual;
```

WebSocket push. The supervisor receives an event whenever any actuator's residual deviates beyond its local K threshold.

### 6. Digital twin consistency (supervisor)

Define LENS views that project `state_history` and `trajectory` into a common shape, then GAUGE across them:

```sql
LENS current_flight AS
  COVER state_history
    PROJECT (t, x, y, z, vx, vy, vz, q0, q1, q2, q3, class);

LENS reference_flight AS
  PULLBACK trajectory ALONG traj_id ONTO reference_library
    WHERE reference_library.validated = TRUE
    PROJECT (t: s, x, y, z, vx, vy, vz, q0, q1, q2, q3, class);

GAUGE current_flight VS reference_flight
  ON FIBER (x, y, z, vx, vy, vz, q0, q1, q2, q3)
  AROUND class;
```

Returns `{ holonomy_1, holonomy_2, gauge_difference, gauge_invariant }`. `gauge_invariant = TRUE` means the current flight is behaving geometrically like the reference.

### 7. Regime classification (supervisor)

How many distinct regimes is the vehicle operating in, from the recent state history?

```sql
SPECTRAL state_history
  ON FIBER (vx, vy, vz, omega_x, omega_y, omega_z, q0, q1, q2, q3)
  MODES 5;
```

Returns five eigenvalues of the fiber Laplacian. The diagnostic the supervisor tracks is the **spectral gap** `λ₂ − λ₁`. A large gap with `λ₁ ≈ 0` means the vehicle is cleanly in a single regime — one connected component in the state graph. A shrinking gap means the state history is straddling multiple regimes, which indicates a transition or an anomaly. The supervisor uses the gap to drive the `controller_mode` field on the command log.

### 8. Forecasting (supervisor)

```sql
PREDICT state_history ON x BY t
  TRAIN BEFORE t < @now
  TEST AFTER t >= @now;
```

Curvature-ranked forecast of future position. Used for lookahead: if the predicted state at `t + lookahead` lies outside the traversable subspace, replan.

### 9. Curvature drift and phase warning (streaming)

```sql
SUBSCRIBE CURVATURE state_history ON fused_conf DRIFT > 0.01;
SUBSCRIBE PHASE state_history ON fused_conf;
```

The first fires when the scalar curvature of the state-history bundle shifts by more than the configured drift. The second fires when `fused_conf` shows a discontinuity — phase transitions in the confidence signal, which catch regime boundaries the controller must respect. Both events route directly into the controller mode state machine: `nominal → degraded → safe → abort`.

## Thread Architecture

Three threads, all talking to a single GIGI Stream instance.

### Fast-path thread

Runs the inner control loop. Calls only operations 1 and 4 (state read, tracking error). Writes to `state_history` and `command_log` via streaming NDJSON at the end of each cycle.

GIGI resolves point queries in ~500 ns at the engine layer, but the full round trip is dominated by transport framing — HTTP keep-alive over localhost adds tens to hundreds of microseconds per call. At two GQL calls per cycle plus controller arithmetic and motor command emission, an HTTP-framed loop fits a 2 ms cycle budget with headroom. A future binary wire transport (Unix domain socket + DHOOM framing) would take per-query latency below 50 µs and widen the envelope to higher-rate inner loops, but is not required for the current operational profile.

### Sensor thread

Reads raw sensors, computes fusion, writes to `sensor_fusion` via streaming ingest. Residual and confidence are computed at ingest time using GIGI's scalar curvature on the sensor subset. Fast-path thread reads the result in operation 1.

### Supervisor thread

Runs operations 2, 3, 6, 7, 8. Maintains the current active plan in the trajectory bundle. Subscribes to operations 5 and 9. On any subscription event, evaluates whether to change controller mode, replan, or abort, and publishes the decision back to the fast-path thread via a `ControlDirective` channel.

Supervisor-to-fast-path communication is one-way: the supervisor publishes directives, the fast-path reads the latest directive non-blockingly. No shared state except through GIGI.

## Rust Adapter

The adapter is an HTTP + WebSocket client that exposes a small typed API to the control-loop code.

```
crates/icarus-adapter/
├── src/
│   ├── lib.rs              — public API
│   ├── client.rs           — GIGI HTTP + WebSocket client
│   ├── gql.rs              — GQL statement builders (typed, not string-spliced)
│   ├── fast_path.rs        — methods called at control-loop rate
│   ├── supervisor.rs       — methods called at supervisor rate
│   ├── subscriptions.rs    — WebSocket subscription handlers
│   ├── quaternion.rs       — quaternion helpers (slerp, log, exp, mul, unit-norm)
│   └── directive.rs        — ControlDirective channel types
├── tests/
└── Cargo.toml
```

Dependencies (all pure Rust):
- `reqwest` or `ureq` for HTTP
- `tungstenite` for WebSocket
- `serde` + `serde_json` for body encoding
- `crossbeam-channel` for the directive channel

The adapter is the only ICARUS code that is not a GQL query. Its role is I/O plus the quaternion math necessary to construct valid attitude sections before writing them to GIGI.

## Reference Controller

Kept in `/examples/multirotor/controller.rs`, separate from this spec.

## Integration Test Plan

Port the Multirotor dynamics from `janismac/ControlChallenges` into Rust and run ICARUS against:

| Level | What it validates |
|---|---|
| MultirotorIntro (hover) | Operations 1, 4 — state read and tracking work at cycle rate |
| MultirotorObstacles | Operations 3, 5, 8 — planning via geodesic, fault subscription, lookahead forecast |
| MultirotorFlip | Operations 2, 7 — envelope awareness permits the flip, regime classification sees the transition, quaternion transport stays smooth through the full rotation |

MultirotorFlip is the decisive test. A conventional controller has to special-case the flip because linearization fails. ICARUS doesn't special-case it — the quaternion double cover on attitude plus holonomy-gated aggressiveness means the vehicle's confidence modulates naturally as it approaches inversion and comes out the other side.

Additional ControlChallenges levels worth porting: RocketLandingUpsideDown (starts at the antipode of upright, trivial for ICARUS, catastrophic for naive LQR), SwingUpSinglePendulum (passes through the unstable equilibrium), and RocketLandingHoverslam (demands precision near the envelope edge).

## What Is Deliberately Not In This Spec

Held in private companion documents:

- Controller gain schedules and tuning constants
- The `envelope_radius` value and its adaptation rule
- The state_hash quantization scheme
- The capability-bundle characterization sweep procedure
- The Davis Field Equation forms beyond `τ = K · C` and the unit-quaternion double cover
- Specific predicate bodies used in the GAUGE invariance tests

These live in `/private/icarus-implementation.md`.

## Patent Strategy Notes

The ICARUS claim is narrow enough to defend and broad enough to cover the useful space. Key claim language:

> A method for controlling a vehicle, comprising: (a) maintaining a capability fiber bundle in a geometric database wherein the base space parameterizes vehicle state and the fiber at each base point carries both said state coordinates and the reachable-next-state bounds, with attitude encoded as a unit quaternion satisfying `q₀² + q₁² + q₂² + q₃² = 1` thereby providing a double cover of SO(3); (b) computing a local holonomy measurement on said bundle in a neighborhood of the current vehicle state via proximity search on said coordinate fiber fields; (c) deriving a confidence score from the scalar curvature of said measurement; (d) modulating controller gain as a monotonically decreasing function of said confidence score, floored to preserve recoverable control authority at maximum envelope violation; (e) thereby causing the vehicle to degrade gracefully toward known-safe operating regions as it approaches the edge of its operational envelope, without requiring special-case handling for attitude singularities, linearization failure, or inversion maneuvers.

Dependent claims cover: the specific GQL operators used, the quaternion transport in place of Euler-angle tracking, the LENS-based digital twin comparison, the integration with trajectory library pullback for safe-mode fallback, the integration with phase-transition subscriptions for abrupt transition handling, and the application to multirotor, rocket, aircraft, and spacecraft control.

Prior art position: GIGI's filing covers the geometric database engine including the HOLONOMY, TRANSPORT, GAUGE, SPECTRAL, and PULLBACK operators. ICARUS claims the *application* of holonomy-gated control to flight systems with quaternion-native attitude representation, which is novel against both aerospace control literature and database literature.

## Implementation Order

1. Stand up a local GIGI Stream instance with the extensions from the GIGI requests letter applied. Verify health endpoint.
2. Implement `BUNDLE` creation for all eight schemas via the CREATE endpoint. Test that each returns 201 and that SHOW BUNDLES lists them. Verify the unit-quaternion INVARIANT rejects non-unit writes.
3. Implement the Rust adapter's HTTP client and a single `insert_state` method that writes to `state_history`. Verify O(1) timing at N = 1, 10K, 100K records.
4. Implement `latest_state` via `SECTION AT t = ...`. Benchmark the full round trip.
5. Implement the WebSocket client and `subscribe_anomalies` on `actuator_state`.
6. Implement the tracking error query via `TRANSPORT ... ON FIBER (q0, q1, q2, q3)`. Unit test against known rotations: identity, 90° yaw, 180° pitch, quaternion composition invariants.
7. Implement the envelope awareness query via `HOLONOMY NEAR ... ON FIBER (x, y, z, vx, vy, vz) ...`. Unit test against synthetic capability bundles with known twist.
8. Implement the supervisor loop that consumes subscriptions and publishes directives.
9. Port the Multirotor dynamics from ControlChallenges to Rust in `/examples/multirotor/dynamics.rs`.
10. Implement the reference controller in `/examples/multirotor/controller.rs`.
11. Run against MultirotorIntro. Assert stable hover.
12. Run against MultirotorObstacles. Assert obstacle avoidance via replan.
13. Run against MultirotorFlip. This is the decisive test.

Every step has a test. No step merges without its test green.

## The Architectural Argument

A conventional GNC stack loses information at every layer boundary. The Kalman filter produces a state estimate with a covariance; the covariance gets stripped when the estimate lands in the flight database because standard databases store scalars. The trajectory planner produces a path with tangent structure; the tangent reduces to a list of waypoints. Attitude gets logged as Euler angles, and the wraparounds become discontinuities in post-flight analysis. Every adapter between layers is a place where geometric structure is destroyed.

ICARUS does not have those boundaries. The covariance is curvature, and GIGI stores curvature natively. The trajectory is a section, and GIGI stores sections natively. The attitude is a unit quaternion satisfying an enforced invariant, and GIGI stores invariant-bound fibers natively. The digital twin comparison is a gauge test, and GIGI executes gauge tests natively.

Every piece of information the GNC stack produces is representable in the same fiber bundle algebra that GIGI executes. Nothing is marshaled across a boundary because both ends of the stack — controller and database — speak the same mathematical language.

This is what "math is load-bearing" means at the architecture level. The database and the controller are the same program.
