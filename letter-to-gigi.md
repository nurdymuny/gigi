# ICARUS → GIGI

**Subject:** Engine extensions required by the ICARUS GNC substrate

**From:** Bee Rosa Davis · Davis Geometric · ICARUS project
**To:** GIGI Engineering

## Context

ICARUS — Integrated Curvature-Aware Routing & Uncertainty Substrate — is a Guidance, Navigation, and Control architecture built entirely on GIGI. The thesis is that state estimation, envelope awareness, trajectory planning, tracking, fault detection, digital twin consistency, regime classification, forecasting, and safety monitoring can all be expressed as GQL queries against fiber bundles, with no separate bespoke GNC math layer. Every geometric operation ICARUS needs is one that GIGI's query language was designed to express.

Almost. A small number of capabilities sit just past the current edge of GIGI v0.5.0 / GQL v2.1. This letter enumerates those extensions precisely so the engine team has a clean specification of what ICARUS is asking for and why.

**Two operational modes shape the performance requirements.** In the first — pre-flight or pre-mission — the full terrain is known and loaded into GIGI before the vehicle moves. Planning runs once: one `GEODESIC` call over the full terrain bundle, one trajectory section written back. The control loop then runs against a known, stable plan and issues only O(1) point queries during flight. No sub-second query latency is required for the planning step; a GEODESIC that takes a few hundred milliseconds is entirely acceptable. In the second mode — active terrain streaming — a forward-looking sensor scans ahead of the vehicle and writes terrain chunks into GIGI in real time. A subscription (`SUBSCRIBE CURVATURE terrain ON occupied DRIFT > threshold`) fires when new obstacles appear, triggering a replan over the freshly-scanned segment. At a lookahead distance of roughly 400–500 yards and cruise speeds common to fixed-wing UAVs (50–100 knots), that horizon gives approximately 9–18 seconds of lead time per planning cycle — ample budget for a bounded GEODESIC, not a hard real-time constraint. Ask 6 addresses the streaming path; the remaining asks are correctness requirements that apply to both modes.

Each ask in this letter proposes a capability that is architecturally general — useful to any GIGI client that needs multi-field constraints, high-dimensional transport, cross-bundle predicates, or streaming path planning. ICARUS is the immediate consumer, but none of these asks introduces domain-specific semantics into the engine. GIGI should remain domain-agnostic; ICARUS provides the schema and query shapes that sit on top.

There are seven asks. They are ordered roughly by how central each one is to ICARUS's core claim — the first three are load-bearing on the patent language itself; the next two are correctness-preserving conveniences; the last two are operational enhancements for streaming terrain and high-rate supervisor loops.

Each ask is structured the same way: **context** (why ICARUS needs it), **current state** (what GIGI does today), **proposed behavior** (desired semantics), **proposed syntax** (illustrative, open to revision), **acceptance test** (how to verify), **notes** (edge cases and dependencies).

## Proposed Schema Agreement

The following bundle definitions represent the agreed mapping between ICARUS GNC operations and GIGI schema shapes. They are provided so the engine team can validate implementation decisions for the asks below against real consumer schemas. Fields marked `INDEX` are prerequisites for Asks 3 and 4; fields with `INVARIANT` require Ask 1. All four schemas listed here are complete and mutually consistent.

### TerrainBundle — world model, static and streaming

```sql
BUNDLE terrain
  BASE (x NUMERIC, y NUMERIC, z NUMERIC)
  FIBER (
    occupied     BOOLEAN  DEFAULT FALSE,
    no_fly       BOOLEAN  DEFAULT FALSE,
    elevation    NUMERIC  RANGE 10000,
    wind_x       NUMERIC  RANGE 50,
    wind_y       NUMERIC  RANGE 50,
    wind_z       NUMERIC  RANGE 30,
    threat_prob  NUMERIC  RANGE 1,
    source       CATEGORICAL INDEX,
    updated_at   NUMERIC
  )
  OPTIONS (STORAGE AUTO, TOLERANCE 0.5, ANOMALY_THRESHOLD AUTO);
```

Base fields `(x, y, z)` form the composite key, so Ask 3's `EXISTS` join from capability resolves in O(1) per outer record via hash lookup on terrain's base — no additional index required on the terrain side. Update contract: static layers via `INGEST` at boot; semi-dynamic (wind, threats) via `SECTIONS terrain (…) UPSERT` (Ask 5); live sensor lookahead via `POST /v1/bundles/terrain/stream` NDJSON. Replanning is triggered by `SUBSCRIBE CURVATURE terrain ON occupied DRIFT > 0.02`, not by polling.

### CapabilityBundle — vehicle envelope as a function of state

```sql
BUNDLE capability
  BASE (state_hash NUMERIC)
  FIBER (
    x              NUMERIC  INDEX RANGE 5000,
    y              NUMERIC  INDEX RANGE 5000,
    z              NUMERIC  INDEX RANGE 5000,
    vx             NUMERIC  INDEX RANGE 100,
    vy             NUMERIC  INDEX RANGE 100,
    vz             NUMERIC  INDEX RANGE 100,
    vx_max         NUMERIC  RANGE 100,
    vy_max         NUMERIC  RANGE 100,
    vz_max         NUMERIC  RANGE 100,
    ang_rate_max   NUMERIC  RANGE 20,
    thrust_margin  NUMERIC  RANGE 1,
    regime         CATEGORICAL INDEX,
    linearization_valid BOOLEAN DEFAULT TRUE,
    validated      BOOLEAN  DEFAULT FALSE
  )
  OPTIONS (STORAGE HASHED, TOLERANCE 0.1);
```

The `INDEX` annotations on `x, y, z, vx, vy, vz` are the prerequisite for O(log n) fiber-endpoint resolution in Ask 4's multi-field `GEODESIC`. Without them, endpoint resolution falls back to a full scan of the capability bundle.

### StateHistoryBundle — continuous flight record

```sql
BUNDLE state_history
  BASE (t NUMERIC)
  FIBER (
    x          NUMERIC  RANGE 5000,
    y          NUMERIC  RANGE 5000,
    z          NUMERIC  RANGE 5000,
    vx         NUMERIC  RANGE 100,
    vy         NUMERIC  RANGE 100,
    vz         NUMERIC  RANGE 100,
    q0         NUMERIC,
    q1         NUMERIC,
    q2         NUMERIC,
    q3         NUMERIC,
    omega_x    NUMERIC  RANGE 50,
    omega_y    NUMERIC  RANGE 50,
    omega_z    NUMERIC  RANGE 50,
    regime     CATEGORICAL INDEX,
    fused_conf NUMERIC  RANGE 1
  )
  INVARIANT q0*q0 + q1*q1 + q2*q2 + q3*q3 = 1.0 +/- 1e-9
  OPTIONS (STORAGE SEQUENTIAL, WAL ENABLED);
```

Note: attitude migrates from the two-field `(S_att, d_att)` representation in the current draft spec to the full unit quaternion `(q0, q1, q2, q3)` here. This migration is a co-requisite of Asks 1 and 2 and applies to `trajectory`, `state_history`, and `capability` simultaneously.

### TrajectoryBundle — planned paths as sections

```sql
BUNDLE trajectory
  BASE (traj_id NUMERIC, s NUMERIC)
  FIBER (
    x       NUMERIC  RANGE 5000,
    y       NUMERIC  RANGE 5000,
    z       NUMERIC  RANGE 5000,
    vx      NUMERIC  RANGE 100,
    vy      NUMERIC  RANGE 100,
    vz      NUMERIC  RANGE 100,
    q0      NUMERIC,
    q1      NUMERIC,
    q2      NUMERIC,
    q3      NUMERIC,
    thrust  NUMERIC  RANGE 1,
    class   CATEGORICAL INDEX
  )
  INVARIANT q0*q0 + q1*q1 + q2*q2 + q3*q3 = 1.0 +/- 1e-9
  OPTIONS (STORAGE HASHED, TOLERANCE 0.05);
```

### GNC Operation → GQL Call Mapping

| GNC Operation | GQL Call | Bundle(s) | Complexity |
|---|---|---|---|
| State read (fast path) | `SECTION state_history AT t = @latest_t` | state_history | O(1) |
| Envelope check | `HOLONOMY capability NEAR (vx=@vx, …) WITHIN r ON FIBER (vx_max, …) AROUND regime` | capability | O(\|N_r\|) |
| Full-terrain plan (pre-flight) | `COVER capability WHERE linearization_valid=TRUE AND EXISTS (terrain WHERE occupied=FALSE)` → `GEODESIC` | capability, terrain | O(n) once |
| Streaming replan (active sensor) | `SUBSCRIBE CURVATURE terrain ON occupied DRIFT > 0.02` → `GEODESIC … RESTRICT TO (…)` | terrain, capability | event-driven |
| Tracking error | `TRANSPORT trajectory FROM (…) TO (…) ON FIBER (q0,q1,q2,q3)` | trajectory | O(1) |
| Fault detection | `SUBSCRIBE ANOMALIES actuator_state ON residual` | actuator_state | push |
| Digital twin check | `GAUGE state_history VS trajectory ON FIBER (q0,q1,q2,q3) AROUND class` | state_history, trajectory | O(n) |
| Regime classification | `SPECTRAL state_history ON FIBER (vx,vy,vz) MODES 5` | state_history | O(n·iter) |
| Forecasting | `PREDICT state_history ON x BY t` | state_history | O(n) |
| Phase warning | `SUBSCRIBE PHASE state_history ON fused_conf` | state_history | push |
| Distribution drift | `DIVERGENCE FROM sensor_now TO sensor_reference` | sensor_fusion | O(1) cached |

---

## Ask 1 — Multi-field fiber INVARIANT declaration

### Context

ICARUS carries vehicle attitude as a unit quaternion in the fiber of the `trajectory`, `state_history`, and `capability` bundles. The mathematical constraint `q₀² + q₁² + q₂² + q₃² = 1` is what makes the quaternion a point on S³, which is the double cover of SO(3), which is what makes ICARUS singularity-free across arbitrary rotation.

This constraint must be enforced at insert time. Renormalizing silently after the fact would hide integrator bugs and drift — which is exactly the class of failure aerospace systems are unforgiving of. A vehicle whose attitude representation has slowly drifted off the unit sphere cannot be trusted to recover from a flip.

### Current state

GQL supports single-field modifiers on fibers (`UNIQUE`, `REQUIRED`, `NULLABLE`, `INDEX`, `ENCRYPTED`, `DEFAULT`, `RANGE`, `AUTO`, `ARITHMETIC`). There is no syntax for declaring a constraint that spans multiple fields of a single fiber.

### Proposed behavior

A bundle definition may declare one or more `INVARIANT` clauses that reference multiple fiber fields and a tolerance. On every insert (`SECTION`, `SECTIONS`, `UPSERT`), the engine evaluates each invariant; any violation rejects the write with a clearly-typed error (`InvariantViolation`), not a silent coercion.

### Proposed syntax

```sql
BUNDLE state_history
  BASE (t NUMERIC)
  FIBER (
    ...
    q0 NUMERIC, q1 NUMERIC, q2 NUMERIC, q3 NUMERIC
    ...
  )
  INVARIANT q0*q0 + q1*q1 + q2*q2 + q3*q3 = 1.0 +/- 1e-9;
```

The invariant expression is a scalar-valued expression in fiber fields; the comparator is one of `=`, `<`, `<=`, `>`, `>=`, `!=`; the tolerance `+/- ε` is optional and defaults to 0 for exact comparisons, to a small machine-epsilon for equality on floats.

Multiple invariants are allowed per bundle; they evaluate in declaration order and short-circuit on first failure.

### Acceptance test

1. Create a bundle with a unit-norm invariant on four fields.
2. Insert a record with `(1, 0, 0, 0)` — succeeds.
3. Insert a record with `(1, 1, 0, 0)` (norm ≈ 1.414) — rejected with `InvariantViolation`.
4. Insert a record with `(1.0000001, 0, 0, 0)` and tolerance `1e-6` — succeeds.
5. Insert the same record with tolerance `1e-9` — rejected.
6. The engine never silently renormalizes.

### Notes

Invariants are not constraints on queries; reads are unaffected. The check is at write time only. For bundles using the SEQUENTIAL storage mode with array fiber storage, the invariant can be batched over a write set.

**Schema migration co-requisite:** The current ICARUS draft spec carries attitude as `(S_att, d_att)` — two fields satisfying S² + d² = 1, which is the unit circle S¹. This is insufficient for genuine 3D rotation (which requires the unit 3-sphere S³, i.e. four fields). This ask is the prerequisite for migrating to `(q0, q1, q2, q3)` with the unit-norm invariant enforced at write time. The Proposed Schema Agreement above shows the post-migration bundle shapes. The migration of `trajectory`, `state_history`, and `capability` should be treated as a single atomic change: all three bundles must be migrated before any quaternion-dependent GQL operations are issued.

## Ask 2 — Higher-dimensional TRANSPORT on S³

### Context

ICARUS computes tracking error and attitude evolution via the TRANSPORT operator on the quaternion fiber. The returned object is consumed directly by the controller as the rotation between commanded and achieved attitude. For this to work on genuine 3D attitude, TRANSPORT must operate correctly when the fiber is four-dimensional and constrained to S³.

Without this, the attitude error returned from TRANSPORT on a four-field fiber collapses to a 2D projection, which loses the information that makes quaternion control singularity-free. The MultirotorFlip validation case and any real 3D aerobatic maneuver — rocket flip, aircraft inversion, spacecraft rotation — depend on this.

### Current state

From GQL_REFERENCE.md and the TRANSPORT semantics documented in the API reference, TRANSPORT on a fiber of dimension `n` currently returns a 2×2 rotation matrix. Higher-dimensional fibers are accepted syntactically but only the first two dimensions participate in the computation.

### Proposed behavior

When TRANSPORT is invoked on a fiber of dimension `n`, the returned object is one of:

- `n = 2`: a 2×2 rotation matrix (current behavior, unchanged).
- `n = 3`: a 3×3 rotation matrix on SO(3), or the equivalent axis-angle pair.
- `n = 4` with a unit-norm invariant declared on the fiber: a unit quaternion representing the rotation from source to target attitude, with an optional return format flag for 4×4 rotation matrix.
- Other `n`: a general orthogonal transform on ℝⁿ.

The engine detects the unit-norm invariant on the fiber (from Ask 1) and selects quaternion semantics accordingly. In the absence of the invariant, four-field transport returns a generic 4×4.

### Proposed syntax

The GQL syntax does not need to change. The return shape adapts based on fiber shape and declared invariant.

```sql
TRANSPORT trajectory
  FROM (traj_id = @plan, s = @commanded_s)
  TO   (traj_id = @plan, s = @achieved_s)
  ON FIBER (q0, q1, q2, q3);
-- returns: { rotation: [q0, q1, q2, q3], displacement: [d0, d1, d2, d3], ... }
-- where the 'rotation' field is a unit quaternion because the fiber has the unit-norm invariant
```

An optional `AS MATRIX` or `AS QUATERNION` suffix can force one representation or the other for callers that need a specific shape:

```sql
TRANSPORT ... ON FIBER (q0, q1, q2, q3) AS MATRIX;  -- returns 4x4
TRANSPORT ... ON FIBER (q0, q1, q2, q3) AS QUATERNION;  -- returns unit quaternion
```

### Acceptance test

1. Create two sections in a quaternion-fibered bundle representing identity `(1, 0, 0, 0)` and a 90° yaw rotation `(√2/2, 0, 0, √2/2)`.
2. TRANSPORT FROM identity TO 90°yaw ON FIBER (q0, q1, q2, q3) returns a quaternion whose vector part points along the z-axis and whose scalar part is approximately `cos(45°) = √2/2`.
3. TRANSPORT through a full 360° rotation (identity → 90° → 180° → 270° → identity) returns identity or a quaternion numerically close to it, with no discontinuity at 180°.
4. TRANSPORT from `(1, 0, 0, 0)` to `(-1, 0, 0, 0)` (the antipode, representing the same physical rotation) returns identity. Required sign convention: when the raw quaternion product `q_b · q_a⁻¹` yields a quaternion with negative scalar part, the engine negates the entire quaternion before returning, so the result always has non-negative scalar part (canonical double-cover representative). This convention must be consistent across all TRANSPORT calls — two compliant implementations with opposite sign conventions produce opposing control outputs even though both are mathematically valid quaternions.
5. The returned quaternion always satisfies the unit-norm invariant within engine tolerance.

### Notes

The GIGI-internal representation of the rotation computation should use the quaternion product (cheap, four multiplies, four adds) rather than building an explicit matrix. Matrix form is only constructed if `AS MATRIX` is requested.

For `n = 3` without a unit-norm invariant, the transport reduces to the current 2D behavior on the dominant subspace. The quaternion path is only activated when the invariant is present.

## Ask 3 — Predicate-sourced COVER

### Context

ICARUS trajectory planning requires filtering the capability bundle to the subspace of points that are (a) inside the linearization regime and (b) at coordinates where the terrain bundle has no obstacle. The filter needs to pull the predicate from a different bundle. The current WHERE clause evaluates only against the bundle being covered.

### Current state

COVER supports WHERE predicates that reference the bundle's own fiber fields, and ON predicates that select by indexed base fields. There is no documented support for a predicate that evaluates against another bundle's fiber during the scan.

### Proposed behavior

A COVER's WHERE clause may contain an `EXISTS (…)` subquery referencing another bundle. The subquery resolves per-record of the outer COVER, and the outer record is included only if the subquery returns at least one matching row.

The common case ICARUS needs is a spatial predicate: "include this capability point only if the terrain at the same coordinates is neither occupied nor a no-fly zone."

### Proposed syntax

```sql
COVER capability
  WHERE linearization_valid = TRUE
    AND EXISTS (
      COVER terrain
      ON x = capability.x AND y = capability.y AND z = capability.z
      WHERE occupied = FALSE AND no_fly = FALSE
    );
```

The qualifier `capability.x` inside the nested scope resolves to the outer record's field. Other operators (`NOT EXISTS`, `IN (COVER ... PROJECT field)`) follow the same resolution.

### Acceptance test

1. Create two bundles: one for capability states with x, y, z coordinates; one for terrain with same coordinates and an `occupied` flag.
2. Populate capability with 1000 states; populate terrain such that half the (x, y, z) coordinates are marked occupied.
3. COVER capability WHERE EXISTS (terrain where same coordinates AND occupied = FALSE) returns the 500 unoccupied states.
4. Query plan (EXPLAIN) shows the EXISTS subquery is resolved via index on terrain coordinates, not a full table scan per outer record.

### Notes

This is the enabling primitive for cross-bundle spatial joins in a way that preserves the geometric semantics of COVER (sheaf evaluation over an open set). The alternative — denormalizing terrain into capability at ingest time — would explode storage and break the separation of concerns between the two bundles.

**Join direction and complexity:** In the ICARUS usage, the outer COVER is over `capability` and the EXISTS looks up `terrain` by `(x, y, z)`. Since terrain's BASE is `(x, y, z)`, each EXISTS lookup is a hash key lookup — O(1) per outer record, O(|capability|) total. No additional index is required on the terrain side. The optimization note in earlier drafts about an "index intersection" applies only if capability's fiber coordinate fields are also indexed; see the Proposed Schema Agreement for the correct INDEX annotations on `capability`.

**Streaming terrain note:** In active sensor mode the terrain bundle is updated continuously. The EXISTS predicate reflects terrain state at query time — records inserted after the outer COVER starts are not visible to that sweep. This is the correct behavior: ICARUS replans from scratch on each `SUBSCRIBE CURVATURE` event rather than relying on a long-running COVER to observe live inserts.

## Ask 4 — Multi-field GEODESIC endpoints

### Context

ICARUS plans trajectories by computing a geodesic through the capability bundle from the current vehicle state to a target state. The "state" is a tuple of multiple coordinates — `(x, y, z, vx, vy, vz)` at minimum, often with attitude. The geodesic must identify start and end points by these coordinate tuples, not by a single base key.

### Current state

The GEODESIC statement takes `FROM key_preds TO key_preds`. The `key_preds` form in the grammar allows multiple predicates, but in practice against a bundle with `BASE (state_hash)` the caller must pre-hash the coordinates into a state_hash value before issuing the query. This forces the caller to know the quantization scheme and match it exactly.

### Proposed behavior

GEODESIC accepts endpoint predicates over fiber fields when the fiber contains the coordinate fields. The engine resolves each endpoint to the nearest record (by the same metric used for HOLONOMY NEAR) and computes the geodesic between those resolved records.

If no record within a configurable tolerance matches an endpoint, GEODESIC returns `{ distance: null, path_found: false, reason: "endpoint_unresolved" }`.

### Proposed syntax

```sql
GEODESIC traversable
  FROM (x = @start_x, y = @start_y, z = @start_z,
        vx = @start_vx, vy = @start_vy, vz = @start_vz)
  TO   (x = @goal_x,  y = @goal_y,  z = @goal_z,
        vx = 0, vy = 0, vz = 0)
  TOLERANCE 0.1;
```

The `TOLERANCE` clause is optional and reuses the bundle's declared TOLERANCE if absent.

### Acceptance test

1. Create a bundle with a coordinate-valued fiber and populate a grid of points.
2. GEODESIC FROM (x=0, y=0) TO (x=1, y=1) returns a path whose endpoints are the grid points nearest to those coordinates.
3. GEODESIC FROM (x=0, y=0) TO (x=100, y=100) where no grid point is within tolerance of (100, 100) returns `path_found: false` with reason `endpoint_unresolved`.
4. The resolved-endpoint lookup is O(log n) via the existing coordinate index, not O(n).

### Notes

This subsumes the single-key case. For bundles with `BASE (id)` and integer keys, `GEODESIC FROM id=1 TO id=42` remains valid and unchanged. The multi-field form activates when the predicates reference fiber fields rather than base fields.

**Index prerequisite:** Fiber-field endpoint resolution is O(log n) only when the referenced fields carry `INDEX`. For the capability bundle this means `x, y, z, vx, vy, vz` must be indexed (see Proposed Schema Agreement). Without those indexes, resolution degrades to O(n). `EXPLAIN` output on a multi-field GEODESIC should surface a warning when the endpoint fields are unindexed.

**Pre-flight vs streaming usage:** In pre-flight mode, GEODESIC is called once over the full capability bundle; the O(n) cost is acceptable because it runs offline. In streaming mode, GEODESIC is called on a bounded terrain segment (Ask 6's `RESTRICT TO` clause); the segment is small enough that O(n) on that subset is fast even without the full index optimization.

## Ask 5 — Batch SECTIONS … UPSERT

### Context

ICARUS's terrain bundle receives semi-dynamic updates — wind forecasts, threat volume changes, datalink updates — that arrive in batches and need to be merged into the existing bundle without duplicating existing records. The clean way to express this is batch upsert. The single-row form `SECTION … UPSERT` is documented; the batch form is not explicitly shown, even though the single-row form clearly admits a batch generalization.

### Current state

`SECTION t (…) UPSERT` is documented for a single record. `SECTIONS t (…)` is documented for batch inserts without upsert semantics. The combination — batch with per-record upsert behavior — is implied but not specified.

### Proposed behavior

`SECTIONS t (…) UPSERT` accepts a batch of records; each record is upserted independently, matching by base fields. Per-record status (`inserted` vs `updated`) is returned in the response array, in the same order as the input.

### Proposed syntax

Following the existing SECTIONS tuple style (no VALUES keyword — not part of the current GQL grammar):

```sql
SECTIONS terrain (
  x, y, z, occupied, elevation, wind_x, wind_y, wind_z,
  threat_prob, no_fly, source, updated_at
) UPSERT
  (0.0, 0.0, 100.0, FALSE, 95.2, 2.1, 0.3, 0.0,
   0.05, FALSE, 'datalink', 1729000000000),
  (0.0, 1.0, 100.0, FALSE, 94.8, 2.0, 0.4, 0.0,
   0.07, FALSE, 'datalink', 1729000000000),
  ...;
```

Response:

```json
{
  "status": "ok",
  "results": [
    { "index": 0, "status": "updated" },
    { "index": 1, "status": "inserted" },
    ...
  ],
  "inserted": 142,
  "updated": 58,
  "total": 10200,
  "curvature": 0.018,
  "confidence": 0.982
}
```

### Acceptance test

1. Insert 1000 records into a bundle.
2. Batch-upsert 500 records where half are existing (by base key) and half are new.
3. Response reports `inserted: 250, updated: 250, total: 1250`.
4. Ordering: the response `results` array is in the same order as input, so callers can correlate per-record outcomes.
5. Performance: a batch of 10,000 upserts completes in comparable time to 10,000 plain inserts; no per-record index-lookup penalty beyond what single-record upsert already incurs.

### Notes

Transactional guarantee: if any record in the batch violates a constraint (including the invariants from Ask 1), the implementation should offer an option to either roll back the whole batch (strict) or continue past the failure and report per-record errors (lenient). ICARUS prefers lenient for the terrain update path — a single bad point should not reject an otherwise-valid batch — but strict is appropriate for other callers. A `MODE strict|lenient` flag on the statement would satisfy both.

## Ask 6 — Inline RESTRICT TO on GEODESIC for streaming terrain replanning

### Context

In missions where terrain is built incrementally from a forward-looking sensor rather than loaded from a pre-computed map, the terrain bundle is populated in real time as the vehicle flies. A forward-looking sensor at 400–500 yards range gives 9–18 seconds of planning lead time at typical UAV cruise speeds (50–100 knots). The event-driven planning loop is:

1. `SUBSCRIBE CURVATURE terrain ON occupied DRIFT > 0.02` fires when new obstacles appear in the scanned segment.
2. On event: issue a `GEODESIC` over the capability bundle, constrained to the currently-scanned terrain window.
3. Write the new plan segment to `trajectory`; the controller switches to it when it reaches the handoff waypoint.

Step 2 is the gap. `GEODESIC` today operates over an entire named bundle. When terrain is streaming in by segment, ICARUS needs to scope the path search to the segment ahead of the vehicle — not the whole terrain bundle, which may contain stale data from earlier in the flight.

### Current state

`SUBSCRIBE CURVATURE t ON f DRIFT > ε` is implemented and fires correctly. `GEODESIC t FROM … TO …` operates over the entire bundle `t`. `RESTRICT t TO (COVER …) AS name` is in the grammar (⚠️ parsed, not wired).

### Proposed behavior

`GEODESIC` accepts an inline `RESTRICT TO` clause that scopes the path search to the subset of the bundle defined by the enclosed COVER predicate. Only records satisfying the COVER are eligible as waypoints. The restriction is applied as a pre-filter before the path search begins — `EXPLAIN` should show it as such.

This is semantically equivalent to `RESTRICT t TO (COVER …) AS name` followed by `GEODESIC name FROM … TO …`, but expressed inline without requiring a named lens as an intermediate object. Wiring the existing parsed `RESTRICT` for use inside `GEODESIC` is the new part; the grammar does not change.

### Proposed syntax

```sql
-- Subscribe: fire when new obstacles appear in the lookahead window
SUBSCRIBE CURVATURE terrain
  ON occupied
  DRIFT > 0.02;

-- On drift event: replan over the scanned segment
GEODESIC capability
  FROM (x = @current_x, y = @current_y, z = @current_z,
        vx = @vx, vy = @vy, vz = @vz)
  TO   (x = @goal_x,  y = @goal_y,  z = @goal_z,
        vx = 0, vy = 0, vz = 0)
  RESTRICT TO (
    COVER terrain
      WHERE occupied = FALSE AND no_fly = FALSE
        AND x BETWEEN @lookahead_min_x AND @lookahead_max_x
        AND y BETWEEN @lookahead_min_y AND @lookahead_max_y
  )
  TOLERANCE 0.5;
```

### Acceptance test

1. Create a capability bundle with 10,000 states and a terrain bundle with 50% of coordinates marked occupied.
2. `GEODESIC capability … RESTRICT TO (COVER terrain WHERE occupied = FALSE)` returns a path that never passes through an occupied terrain coordinate.
3. `SUBSCRIBE CURVATURE terrain ON occupied DRIFT > 0.02` fires when a new obstacle is inserted. A subsequent GEODESIC with RESTRICT TO reflects the new obstacle.
4. `RESTRICT TO (COVER terrain WHERE … AND x BETWEEN @a AND @b)` limits the GEODESIC scope to the specified x range — the path does not route through coordinates outside the window.
5. `EXPLAIN` on a GEODESIC with RESTRICT TO shows the restriction applied before the path search (pre-filter, not post-filter).

### Notes

The planning cycle for streaming terrain is event-driven, not polled. ICARUS issues one GEODESIC per `SUBSCRIBE CURVATURE` event, not one per control tick. At 50 knots with a 400-yard lookahead, events fire roughly every 15 seconds. A GEODESIC that takes hundreds of milliseconds is fine; the controller runs against the current plan segment during replanning and switches when the new segment is ready.

`RESTRICT` is already parsed (⚠️ in GQL_REFERENCE.md). This ask promotes one path from parsed-no-op to implemented for the GEODESIC operator specifically. No grammar change is required.

This ask does not require a binary wire protocol. The HTTP path over localhost is more than fast enough for a planning call that runs every ~15 seconds. A future binary/UDS transport layer is a sensible long-term addition but is not a dependency for any current ICARUS operational mode.

## Ask 7 — K and confidence as first-class subscription pseudo-fields

### Context

ICARUS's supervisor subscribes to phase transitions on `state_history` as a proxy for envelope degradation. The natural subscription target is curvature K itself — the quantity that directly drives the `1/(1+K)` gain formula. The current workaround is to subscribe to `fused_conf` (a fiber field computed from K at write time), but this is one step removed and loses the ability to react to K before confidence drops below a visible threshold.

### Current state

`SUBSCRIBE PHASE t ON f` and `SUBSCRIBE CURVATURE t ON f DRIFT > ε` both require `f` to be a named fiber field. `CONFIDENCE()` and `CURVATURE()` are available as inline projection functions inside COVER, but are not addressable as subscription predicate targets. `SUBSCRIBE CURVATURE` implicitly tracks K but does not expose it by name.

### Proposed behavior

`K` and `CONFIDENCE()` are exposed as pseudo-fields in subscription predicate positions, resolving to the bundle's local curvature and confidence at event-fire time.

```sql
SUBSCRIBE PHASE state_history ON K;
-- fires when K undergoes a phase transition (discontinuity in curvature)

SUBSCRIBE CURVATURE state_history DRIFT > 0.05;
-- already documented — confirming K is the implicit drift field
```

Both subscriptions include the current K value and confidence in the event payload.

### Acceptance test

1. Create a bundle and insert records that cause a step change in K.
2. `SUBSCRIBE PHASE bundle ON K` fires at the step.
3. `SUBSCRIBE CURVATURE bundle DRIFT > ε` fires when K changes by more than ε.
4. Both event payloads contain `{ k: <current_K>, confidence: <1/(1+K)> }`.
5. If a bundle has a fiber field literally named `K`, the fiber field takes precedence and `__curvature__` or `CURVATURE()` is accepted as the unambiguous pseudo-field alias.

### Notes

`SUBSCRIBE CURVATURE` already implicitly operates on K — this ask makes that implicit target explicit and extends the same naming to `SUBSCRIBE PHASE`. The risk of shadowing a user-defined fiber field named `K` is real but rare; the alias fallback in acceptance test item 5 handles it without breaking the general case.

---

## Summary

Seven asks, in priority for ICARUS correctness:

1. **Multi-field fiber INVARIANT declaration** — enforces the unit-quaternion constraint at write time, making the double cover of SO(3) real.
2. **Higher-dimensional TRANSPORT on S³** — makes quaternion attitude tracking work end-to-end, including the sign convention fix for antipodal quaternions.
3. **Predicate-sourced COVER (EXISTS subquery)** — enables cross-bundle spatial joins for traversability-filtered trajectory planning.
4. **Multi-field GEODESIC endpoints** — removes the need for callers to pre-hash state coordinates; endpoint resolution by fiber field values with declared INDEX.
5. **Batch SECTIONS UPSERT** — supports the terrain semi-dynamic update path with per-record inserted/updated reporting and strict/lenient mode.
6. **Inline RESTRICT TO on GEODESIC** — enables event-driven replanning over live sensor-streamed terrain segments; promotes an already-parsed grammar construct to implemented.
7. **K and confidence as subscription pseudo-fields** — makes phase and curvature subscriptions directly addressable by their geometric meaning rather than by proxy fiber fields.

Asks 1, 2, and 3 are load-bearing on the ICARUS patent claim. The claim specifies unit-quaternion double cover of SO(3), requiring Ask 1's invariant enforcement and Ask 2's quaternion-aware TRANSPORT. The claim specifies holonomy-gated aggressiveness over a capability manifold filtered by traversability, requiring Ask 3's cross-bundle predicate. These three are the architectural core.

Asks 4 and 5 are correctness-preserving conveniences that make the spec's GQL directly executable without caller-side workarounds.

Ask 6 enables the active-terrain-streaming operational mode: sensor lookahead populates terrain in real time, curvature subscriptions detect obstacles as they arrive, and GEODESIC replans over the live-scanned segment within the sensor lead time. No sub-second query latency is required — the 9–18 second lead-time budget at typical UAV cruise speeds is the operative planning constraint.

Ask 7 is a small coherence fix that makes K directly addressable in subscription predicates, closing the gap between what GIGI computes and what ICARUS can subscribe to.

All proposed syntax is illustrative; the engine team has final say on the shape that integrates best with the existing grammar. The acceptance tests are the contract — any syntax that passes those is acceptable.

Happy to collaborate on design decisions, review draft implementations, and help shape acceptance test corpora. Thank you for building the substrate that makes this architecture possible.

— ICARUS