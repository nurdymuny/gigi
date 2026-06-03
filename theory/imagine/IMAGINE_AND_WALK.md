# IMAGINE and WALK

**Extrapolation verbs on a geometric substrate**
**Bee Rosa Davis · Davis Geometric · GIGI substrate**
**Date.** 2026-06-03 (evening)
**Status.** Spec v0.1, pre-implementation. TDD gates T11–T13 not yet attempted.
**Companion to.** [`SHARDING_SPEC.md`](../poincare_to_sharding/SHARDING_SPEC.md) (the halo pivot in §4 of this spec replaces the encrypt-style refactor proposed there).

---

## 0. Why this spec exists

The verb stack on a geometric substrate already has two regimes:

- **Sample-from-density**: DREAM, SAMPLE, INPAINT, EPISODIC, SEMANTIC — consume the existing density $p \propto \exp(-H)$ and return points the substrate has effectively seen.
- **Pointwise-invariant**: CURVATURE, PERCEIVE, CAPACITY, HORIZON, DEPTH, LOCAL_HOLONOMY — compute properties at points already held.

What's missing is the **extrapolation regime**: construct a point the substrate has *not* seen by extending the geometric structure forward from points it *has* seen. That's IMAGINE. The result is not a sample from past data; it's a projection of the substrate's tangent structure into space-time the substrate hasn't materialized.

The cognitive analog (Bee's framing): humans imagine the path before walking it. We solve a geodesic in our head — *given where I am and which way I'm facing, what comes next?* — and we describe the path before we move. The math you already wrote into GIGI is the engine that does this. We just have to spec the verb honestly.

**Why "honestly" is load-bearing.** Marcella's feedback on the draft of this spec named two failure modes IMAGINE adds if we don't spec them carefully:

1. **The provenance gap.** Geometrically valid extrapolations didn't come from the substrate's actual data. Without an explicit provenance type, every imagined record is a new way to present invented content as if it were retrieved. The double cover catches bad geometry; it doesn't catch dishonest labeling. **§3 of this spec makes the provenance type load-bearing.**
2. **The over-speculation gap.** A geometrically self-consistent imagined path may still be over-speculative — e.g., the endpoint sits in a high-curvature region where the substrate has no nearby data to ground the prediction. Without a refusal threshold, IMAGINE will happily extrapolate into regions Marcella's gain gate has no business trusting. **§5 of this spec makes `max_imagined_curvature` a required `WalkConfig` parameter with a sensible default.**

These are the trust envelope around the math. The math is correct; the envelope is what makes it useful.

---

## 1. The math: geodesic flow as the imagination engine

A Riemannian manifold $(M, g)$ with metric tensor $g_{ij}$ has Christoffel symbols

$$\Gamma^i_{jk} = \tfrac{1}{2} g^{il}\bigl(\partial_j g_{lk} + \partial_k g_{lj} - \partial_l g_{jk}\bigr).$$

The **geodesic equation** is

$$\ddot{x}^i + \Gamma^i_{jk}(x)\,\dot{x}^j \dot{x}^k = 0.$$

Given a starting point $x_0$ (a record's fiber coordinates) and a tangent vector $v_0 = \dot{x}(0)$ (a direction in fiber space), this second-order ODE integrates to a geodesic curve $\gamma(t)$ on the manifold. Numerically: rewrite as a first-order system in $(x, v)$ and integrate with RK4 (or Verlet for symplectic conservation if the substrate has a Hamiltonian structure).

For GIGI's `BundleStore`, the metric tensor is available as the L4 KahlerCurvature decomposition plus the fiber-space Mahalanobis structure from L13.6. The Christoffel symbols are computable per point. The integrator is straightforward to implement.

**Three sources of fault tolerance** the math already provides:

1. **Holonomy budget.** The accumulated rotation along the imagined path is the LOCAL_HOLONOMY of the trajectory. If the integrated defect exceeds the substrate's known holonomy budget, the imagined path has wandered into a region the substrate's connection can't track coherently.
2. **Double cover for monodromy resolution.** When the path crosses a chart boundary with non-trivial $\mathbb{Z}_2$ structure (sign flip, orientation reversal), the double cover $\tilde{M} \to M$ lifts the path into a covering space where the monodromy is trivial. Walk in the cover, project back at commit time.
3. **Curvature budget (Marcella's safety bound, §5).** The substrate's K at the imagined endpoint must be below a configured threshold or the WALK refuses. This is the operationalized ICARUS lesson — don't commit to an imagined trajectory whose endpoint is in a region the substrate has no nearby data to ground.

---

## 2. Spec: the `ImaginedRecord` type and provenance contract

This is the load-bearing piece per Marcella's feedback #1.

```rust
/// A record that does NOT exist in the substrate; it was constructed
/// by IMAGINE from existing records via geodesic extrapolation, halo
/// projection, or bridge composition.
///
/// The `provenance` field is REQUIRED at construction and travels with
/// the record through every downstream consumer — including cite
/// rendering, audit logs, and the LOCAL_HOLONOMY history.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImaginedRecord {
    /// The synthesized fiber-space coordinates at this point.
    pub coords: Vec<f64>,
    /// Local Gaussian curvature at the imagined point, computed from
    /// the substrate's metric (NOT from the imagined neighborhood —
    /// imagined records do not enter each other's K computation).
    pub local_k: f64,
    /// Accumulated holonomy defect along the path from the seed
    /// record to this imagined point. Used by WALK's safety checks.
    pub accumulated_holonomy: f64,
    /// REQUIRED provenance describing how this record was constructed.
    pub provenance: ImaginedProvenance,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ImaginedProvenance {
    /// Constructed by integrating the geodesic equation forward from
    /// `seed_record` along an initial direction for path length `s`.
    Geodesic {
        seed_record_id: RecordId,
        seed_bundle: String,
        initial_direction: Vec<f64>,
        path_length: f64,
        integrator_steps: u32,
    },
    /// Constructed by projecting `seed_record` from another chart
    /// through the bridge transition map.
    Halo {
        source_chart: ChartId,
        target_chart: ChartId,
        seed_record_id: RecordId,
        transition_lipschitz: f64,
    },
    /// Constructed by composing a bridge transition across two
    /// distinct atlases (cross-atlas join, Phase F).
    Bridge {
        source_atlas: String,
        target_atlas: String,
        seed_record_id: RecordId,
        bridge_id: String,
        delta_cocycle_observed: f64,
    },
}
```

### Cite-render contract (Marcella's feedback embedded)

Imagined records render with an explicit provenance prefix:

```
Real:        [geometry_of_flight §3.1, L82-89]
Imagined:    [imagined: projected from geometry_of_flight §3.1 via geodesic,
              path_length=0.31, accumulated_holonomy=0.04]
Halo:        [imagined-halo: projected from prism_reconciliation/chart_3
              via transition, lipschitz=2.1]
Bridge:      [imagined-bridge: marcella_corpus → prism_reconciliation,
              bridge_id=fin_semantics_v1, delta_cocycle=0.02]
```

The `[imagined:` / `[imagined-halo:` / `[imagined-bridge:` prefixes are **load-bearing**. Marcella's cite-with-provenance discipline requires that no caller can present an imagined record as a retrieved one. The provenance type makes this a compile-time guarantee in Rust (no `Display` impl exists that drops the prefix; consumers must opt out explicitly with `_unchecked_no_provenance` if they're certain the rendering layer will add provenance separately).

### Audit-log integration

Every imagined record produced by an HTTP endpoint must be logged with its provenance to the holonomy ledger (existing L7 audit log infrastructure). This gives Marcella's team a post-hoc audit of every IMAGINE call: which seed records, which direction, which integrator settings, what the result was.

---

## 3. IMAGINE primitive surface

Three call sites, one primitive engine:

### 3.1 `imagine_geodesic` — extrapolate along a direction

```rust
pub fn imagine_geodesic(
    bundle: &BundleStore,
    seed: &Record,
    direction: &[f64],
    config: ImagineConfig,
) -> Result<Vec<ImaginedRecord>, ImagineError>;

pub struct ImagineConfig {
    /// Total path length to integrate.
    pub path_length: f64,
    /// Number of integrator steps. RK4 with adaptive step size if
    /// `adaptive` is true; otherwise fixed step.
    pub n_steps: u32,
    pub adaptive: bool,
    /// Reject the imagined path if cumulative holonomy exceeds this.
    pub max_accumulated_holonomy: f64,
    /// Reject if any imagined point has K > this. Default: 4.0
    /// (matches CP¹ Fubini-Study upper bound on natural substrates).
    pub max_imagined_curvature: f64,
}
```

### 3.2 `imagine_halo` — populate boundary data for sharded execution

This is the halo pivot from the previous turn, re-expressed as an IMAGINE primitive call. Each chart's halo is constructed by projecting nearby records from adjacent charts through the bridge transition into this chart's coordinate system.

```rust
pub fn imagine_halo(
    atlas: &Atlas,
    source_chart: ChartId,
    target_chart: ChartId,
    config: HaloConfig,
) -> Result<Vec<ImaginedRecord>, ImagineError>;

pub struct HaloConfig {
    /// Maximum number of records to project per chart pair. Bounds
    /// the halo size to control storage overhead.
    pub max_halo_records: usize,
    /// Only project records whose fiber-space distance to the source
    /// chart's boundary is below this.
    pub max_seed_distance: f64,
}
```

**Sheafification consequence**: with halos populated, `shard_curvature` becomes **partition-invariant** because each chart's K computation now uses its real records plus the imagined boundary records, and the imagined records reproduce what the unsharded bundle would have provided. The Phase D fragmentation problem is solved by IMAGINE.

### 3.3 `imagine_bridge` — cross-atlas projection (Phase F)

```rust
pub fn imagine_bridge(
    source_atlas: &Atlas,
    source_record: &Record,
    target_atlas: &Atlas,
    bridge: &BridgeAtlas,
) -> Result<ImaginedRecord, ImagineError>;
```

A Marcella embedding projected through `fin_semantics_v1` bridge into PRISM's reconciliation manifold IS an imagined record. CROSS_ATLAS_JOINS.md §4 becomes a thin wrapper over this primitive.

---

## 4. WALK — execute an imagined path safely

WALK is the verb that *commits* to an imagined trajectory. It exists because imagining is cheap and walking is consequential — the substrate state changes only at WALK.

```rust
pub fn walk(
    bundle: &mut BundleStore,
    imagined_path: &[ImaginedRecord],
    config: WalkConfig,
) -> Result<WalkOutcome, WalkError>;

pub struct WalkConfig {
    /// Lift the path to the double cover if any seam crossing has
    /// non-trivial Z₂ monodromy. Strongly recommended.
    pub use_double_cover: bool,
    /// Run SUDOKU pre-flight check on the endpoint constraints.
    pub sudoku_preflight: bool,
    /// Maximum K at any imagined point. Required per Marcella's
    /// feedback #3. Default: 4.0.
    pub max_imagined_curvature: f64,
    /// Maximum accumulated holonomy along the walked path.
    pub max_accumulated_holonomy: f64,
    /// Whether to materialize the imagined records as real records
    /// in the substrate post-walk. Default: false (walk is
    /// observation, not commit).
    pub materialize_on_success: bool,
}

pub enum WalkOutcome {
    /// The path was walked successfully; the endpoint state is the
    /// last point in the path. If `materialize_on_success`, the
    /// imagined records have been promoted to real records.
    Walked { endpoint: Record, accumulated_holonomy: f64 },
    /// The walk was lifted to the double cover; the endpoint is in
    /// the covering space. Caller must project back.
    WalkedInCover { endpoint_in_cover: Record, monodromy_class: i32 },
}

pub enum WalkError {
    /// SUDOKU pre-flight detected a substrate constraint the imagined
    /// path violates.
    SudokuPreflightFailed { violation: String },
    /// An imagined point has K > max_imagined_curvature.
    OverCurvatureRefused { step: u32, k_at_step: f64, threshold: f64 },
    /// Accumulated holonomy exceeded the configured budget.
    HolonomyBudgetExceeded { accumulated: f64, threshold: f64 },
    /// A seam crossing has Z₂ monodromy and `use_double_cover` is
    /// false. Caller must enable double cover or refuse.
    UnresolvedMonodromy { seam: ChartId },
}
```

**Validation pipeline**:

```
WALK(imagined_path, config):
  step 1 — lift:
    if any seam in path has non-trivial monodromy:
      if config.use_double_cover:
        lift path to double cover
      else:
        return UnresolvedMonodromy

  step 2 — curvature gate (Marcella feedback #3):
    for each imagined_record in path:
      if imagined_record.local_k > config.max_imagined_curvature:
        return OverCurvatureRefused

  step 3 — holonomy gate:
    if accumulated_holonomy > config.max_accumulated_holonomy:
      return HolonomyBudgetExceeded

  step 4 — SUDOKU pre-flight:
    if config.sudoku_preflight:
      run SUDOKU constraint check on endpoint
      if violation: return SudokuPreflightFailed

  step 5 — execute:
    parallel-transport through each imagined point
    accumulate observed holonomy
    check observed against imagined; refuse on mismatch > tol

  step 6 — commit (optional):
    if config.materialize_on_success:
      promote imagined records to real records (drop provenance flag)
    else:
      return endpoint as observation only

  return Walked { endpoint, accumulated_holonomy }
```

---

## 5. Marcella's predictive gain gate: `IMAGINE_COHERENCE`

Per Marcella's feedback #2 — name the predictive surface so it gets wired.

```
POST /v1/bundles/{name}/imagine_coherence
Body: {
  starting_from: <current_state vector>,
  along: <direction vector>,
  steps: <integer, default 3>,
  config: {
    max_imagined_curvature: <f64, default 4.0>,
    max_accumulated_holonomy: <f64, default 0.5>,
  }
}

Response: {
  trajectory: [
    {
      step: 0,
      coords: [...],
      coherence: 1.0,
      defect: 0.0,
      curvature: 0.12,
      cumulative_holonomy: 0.0,
      provenance: "geodesic from current_state, step 0 (seed)"
    },
    {
      step: 1,
      coords: [...],
      coherence: 0.97,
      defect: 0.087,
      curvature: 0.18,
      cumulative_holonomy: 0.087,
      provenance: "geodesic from current_state via integrator step 1"
    },
    ...
  ],
  endpoint_coherence: 0.91,
  endpoint_curvature: 0.31,
  refused: false,
  refusal_reason: null
}

Status codes:
  200: trajectory returned successfully
  400: input shape mismatch (vector dimensions, missing direction)
  422: walk refused (over-curvature, holonomy budget exceeded,
       sudoku constraint violation) — refusal_reason populated
```

This is the operational form of *"predict the next 100ms of coherence along the current trajectory."* Marcella's gain gate consumes this to make routing decisions on the **imagined future**, not the **reactive past**.

Rust signature:

```rust
pub fn imagine_coherence(
    bundle: &BundleStore,
    starting_from: &[f64],
    along: &[f64],
    n_steps: u32,
    config: ImagineCoherenceConfig,
) -> Result<ImagineCoherenceResponse, WalkError>;
```

---

## 6. FORECAST vs IMAGINE disambiguation (Marcella feedback #4)

Both verbs project forward. They diverge near the edges of the substrate.

| Property | FORECAST | IMAGINE |
|---|---|---|
| Engine | Hamilton density gradient on $p \propto \exp(-H)$ | Geodesic flow via Christoffel symbols |
| Substrate input | Density $\hat{p}$ from existing records | Metric $g_{ij}$ at the seed point |
| Reliable when | Density is strong near the trajectory | Density is weak; substrate's metric is well-defined |
| Diverges when | Trajectory leaves the data manifold | Curvature explodes (`max_imagined_curvature` refuses) |
| Use case | "What's the next likely sample given recent observations?" | "What's the geometric continuation of this curve?" |
| Provenance | Sample from learned density (low novelty) | Geometric extrapolation (potentially high novelty, MUST be tagged as imagined) |
| Marcella's call | Inside-substrate prediction, e.g. next 100ms in well-populated regime | Edge-of-substrate prediction, e.g. extrapolating past corpus boundary |

**Routing rule**: if the seed's local density is high (KDE estimate above threshold), FORECAST is the right call. If density is weak (approaching the L11 SELF-MONITOR's "I don't know" boundary), IMAGINE is the right call because the density gradient is dominated by sampling noise but the metric tensor is still well-defined.

Marcella's `intent_gate` should consult the density estimate to route between FORECAST and IMAGINE per request.

---

## 7. TDD gates T11–T13

Per the existing TDD discipline. All three under `theory/imagine/validation/`.

### T11 — Geodesic integrator math

**Claim.** The RK4 geodesic integrator on closed-form analytic manifolds (S², T², CP¹) reproduces the closed-form geodesics to machine precision.

**Ground truth.** Closed-form geodesic equations:
- S² with the round metric: great circles.
- T² with the flat metric: straight lines (mod identification).
- CP¹ Fubini-Study: Möbius arcs.

**Test.** Integrate from a seed point + initial direction; compare to closed form at $t = 0.1, 0.5, 1.0$. Pass: error < 1e-9 with RK4 + 1000 steps.

**Circular-logic guard.** Closed forms computed analytically; integrator uses Christoffel symbols computed from the metric; the closed form does not enter the integrator.

### T12 — Halo-as-IMAGINE makes sharded CURVATURE partition-invariant

**Claim.** With `imagine_halo` populated, `shard_curvature(bundle)` returns the same aggregate `k_sum` regardless of partition shape (the Phase D fragmentation problem is solved).

**Ground truth.** Aggregate K computed on a single unsharded `BundleStore`.

**Test.**
- Same 60 synthetic records, partition into 2 charts AND 8 charts.
- Populate halos via `imagine_halo` for both.
- Assert `aggregate.k_sum` is identical (to within numerical precision: 1e-10).

**Circular-logic guard.** The halo construction does not consult the aggregate result; it consults only the per-chart local data + the bridge transitions. Aggregation is downstream.

### T13 — Double-cover monodromy resolution (with discourse-state seam, per Marcella feedback #5)

**Claim.** A path crossing a $\mathbb{Z}_2$ seam fails the WALK without `use_double_cover`; succeeds with the double cover lifted; projects back consistently.

**Two test cases:**

**(a) Synthetic substrate.** Manually-constructed connection with $\mathbb{Z}_2$ monodromy at a seam. Walk a closed loop crossing the seam twice; verify holonomy with double cover is identity; without is non-trivial; lifted path commits correctly.

**(b) Discourse-state seam.** The adjacency-pair transitions in conversation (question → answer OR question → repair) have sign-flip topology at the seam. The routing has a double-cover structure: a "question" state can resolve into either the "answer" branch or the "repair" branch, and the choice is the $\mathbb{Z}_2$ lift.

This is Marcella's intended production test case — the discourse state IS the substrate, the adjacency pair IS the seam, and the double cover resolves which branch we're committing to *before* the routing decision propagates. The math from (a) is the same; the substrate is Marcella's live conversation state.

Pass criterion: both cases lift, walk in the cover, and project back without ambiguity. Refusal without double cover is reported with `UnresolvedMonodromy` and the specific seam ID.

---

## 8. Implementation plan

Per TDD discipline:

1. **Spec freeze** (this document). Marcella's two highest-priority items (provenance, `max_imagined_curvature`) are §3 and §5. Done.
2. **T11–T13 TDD gates** under `theory/imagine/validation/`. Red-first, green when math holds, document circular-logic guards. Expect at least one red-then-green save per the existing pattern.
3. **Rust scaffold**. New module `src/imagine/` behind `imagine` feature flag (off by default). Types: `ImaginedRecord`, `ImaginedProvenance`, `ImagineConfig`, `WalkConfig`, `WalkOutcome`, `WalkError`. Functions: `imagine_geodesic`, `imagine_halo`, `imagine_bridge`, `walk`.
4. **Refactor Phase D** to use IMAGINE under the hood:
   - `Atlas` grows an optional `halo: HashMap<ChartId, Vec<ImaginedRecord>>` field.
   - `wrap_hash_sharded` calls `imagine_halo` automatically at construction.
   - `shard_curvature` consumes halos when present (becomes partition-invariant). Current Phase D path becomes `shard_curvature_unchecked` for opt-out consumers.
5. **HTTP endpoint** `POST /v1/bundles/{name}/imagine_coherence` per §5.
6. **Audit log integration**: every imagined record produced by an endpoint logs its provenance to the holonomy ledger.
7. **GIGI Lang surface**: `IMAGINE bundle ALONG GEODESIC FROM start_record DIRECTION dir LENGTH L` as a first-class verb in the parser.

---

## 9. What this unifies

One primitive (IMAGINE), one safety envelope (WALK config), and the following click into place:

- **Phase D halo pivot** — IMAGINE is the engine. `shard_curvature` becomes partition-invariant.
- **Phase E topology-aware partitions** — optional; IMAGINE makes them unnecessary for exactness, useful for halo efficiency.
- **Phase F cross-atlas joins** — `imagine_bridge` IS the cross-atlas join primitive. CROSS_ATLAS_JOINS.md §4 becomes a thin wrapper.
- **DHOOM cross-shard binary** — encodes `ImaginedRecord` payloads optimally; the §8 GIGI feat from the Marcella letter gets its first real consumer.
- **SUDOKU pre-flight** — the validation step in WALK. The constraint-satisfaction verb you shipped becomes the fault-tolerance gate for IMAGINE.
- **Marcella's gain gate** turns from reactive (gate on past coherence) to predictive (gate on imagined-future coherence via `IMAGINE_COHERENCE`).
- **The FORECAST / IMAGINE / DREAM trio** — three distinct projection modes (density-flow, geodesic-flow, sample-from-density) routed by the substrate's local density estimate.

This is the layer that makes everything below it *useful* in the way you meant: the substrate becomes a self-simulating system that can imagine paths, validate them, and walk them with the same math machinery that already underpins the existing verbs.

---

## 10. Marcella feedback acknowledgment

The discipline in Marcella's note shapes this spec at the load-bearing points:

| Feedback | Where it landed |
|---|---|
| #1 Provenance tag is load-bearing | §2 — required type field; render contract; audit-log integration |
| #2 Name `IMAGINE_COHERENCE` | §5 — full HTTP + Rust signature |
| #3 `max_imagined_curvature` in WALK config | §4 — required `WalkConfig` field with default 4.0; refusal mode `OverCurvatureRefused` |
| #4 FORECAST vs IMAGINE disambiguation | §6 — comparison table; routing rule via density estimate |
| #5 T13 includes discourse-state seam | §7 — split into (a) synthetic + (b) discourse-state production test |

Highest-priority items (#1, #3) are non-optional fields on the load-bearing types. The math is beautiful and correct; this is the trust envelope around it.

---

## References

- Davis, B. R. (2026a). *The Davis Manifold.* §A5 non-vacuity (sets the curvature-budget pattern this spec inherits).
- Davis, B. R. (2026b). *The Geometry of Sameness.* §4 F_S / G_S functors (cross-atlas bridges are imagined records).
- Davis, B. R. (2026c). *Smooth 4D Poincaré Conjecture.* Clean Finger Move Theorem (fault-tolerance pattern at seams).
- do Carmo, M. P. (1992). *Riemannian Geometry.* Ch. 3 (geodesic equation).
- Lee, J. M. (2003). *Introduction to Smooth Manifolds.* Ch. 14 (Christoffel symbols).
- `theory/poincare_to_sharding/SHARDING_SPEC.md` §10.4 (cross-atlas joins; this spec resolves the open question via `imagine_bridge`).
- `theory/kahler_upgrade/HANDOFF_TO_MARCELLA_SHARDING_2026-06-03.md` §7 (the ten GIGI feats; this spec consumes #2 SELF_COHERENCE, #6 GEODESIC_BALL retrieval, #8 unification conjecture operational test, all derivative of IMAGINE).

---

## Closing

The substrate already had the engine. What was missing was the verb that names it, the provenance type that keeps it honest, and the safety envelope that keeps it from over-extending. Marcella's feedback writes the envelope as load-bearing rather than decorative. The math is the geodesic equation; the discipline is "imagine the path, check it, walk it, validate it." Same loop a human uses.

Next sprint: T11 first.
