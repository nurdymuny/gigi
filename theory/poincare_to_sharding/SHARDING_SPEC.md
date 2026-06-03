# GIGI Sharding Spec

**Implementation specification for the atlas-cover sharding model**
**Gated on:** `poincare_to_sharding.md` theory doc + 6 GREEN TDD gates under `validation/`
**Status:** Spec v0.1 (pre-implementation; SHIP gates not yet attempted)
**Date:** 2026-06-03

---

## 0. Reading order

This spec is the implementation handoff for sharded GIGI. It assumes the reader has:

1. Read [`poincare_to_sharding.md`](poincare_to_sharding.md) §§1–4 (the math) — or at least skimmed §3 to see the six TDD-gated claims.
2. Knows GIGI's current single-node architecture (`src/bundle.rs`, `src/mmap_bundle.rs`, `src/geometry/transport.rs`, `src/discrete/`).
3. Accepts that **all spec claims are gated on green tests** under `validation/`. Sections without a green test do not ship.

---

## 1. Spec scope and non-scope

### In scope

- On-disk format for sharded bundle storage with explicit transition functions.
- Rust API for cross-shard execution of CURVATURE, PERCEIVE, CAPACITY, HORIZON, DEPTH, LOCAL_HOLONOMY, HOLONOMY, TRANSPORT, BETTI, SEMANTIC, SPECTRAL.
- The `sharded_write_resolve()` resolver primitive.
- Per-bundle `SpectralRegime` declaration and routing.
- Non-vacuity gate enforcement at shard boundaries.

### Out of scope (sprint follow-ups)

- Distributed transactions across shards (single-shard ACID continues to hold; cross-shard atomicity is a follow-up).
- Schur-complement-based sharded SPECTRAL for expander substrates (T5 §3.5 disclosed this gap; future TDD gate T7).
- Network-layer concerns (RPC framing, retry policy, backpressure) — these are operational details, not math.
- Migration of in-place single-node bundles to sharded form — covered in §9 as a one-way conversion script, not as in-place mutation.

---

## 2. Core types

### 2.1 `ChartId` and `Atlas`

```rust
/// Identifier for a chart in an atlas. The atlas owns charts; each shard
/// holds one or more charts. A chart is the unit of locality for sharded
/// computation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChartId(pub u32);

/// The atlas of a sharded bundle. Stores chart metadata (where each
/// chart lives, what region of the configuration manifold it covers)
/// and transition functions between overlapping charts.
///
/// This is the on-disk representation of "the chart-stitching data"
/// from Geometry of Sameness §4 -- first-class, queryable, indexed.
pub struct Atlas {
    /// All charts in this atlas, indexed by ChartId.
    pub charts: HashMap<ChartId, ChartMetadata>,

    /// Pairwise overlaps. Key is canonicalized as (min_id, max_id).
    /// Each overlap holds the transition function `T_ij : V_i -> V_j`
    /// plus its inverse if invertible.
    pub transitions: HashMap<(ChartId, ChartId), Transition>,

    /// The declared cocycle slack budget. From Geometry of Sameness
    /// Definition 21: ||T_jk o T_ij - T_ik|| <= delta_cocycle.
    /// This is a property of the atlas, declared at construction;
    /// the engine validates it on each insert (§7.2).
    pub delta_cocycle_budget: f64,

    /// Per-atlas spectral regime declaration. Routes SPECTRAL queries
    /// (§5.7). Required field.
    pub spectral_regime: SpectralRegime,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChartMetadata {
    pub id: ChartId,
    pub shard_id: ShardId,
    /// What region of the underlying manifold this chart covers, as
    /// a predicate on configuration coordinates (e.g., bounding box,
    /// indicator function, or learned classifier).
    pub region: ChartRegion,
    /// The connection 1-form on this chart, in chart-local coordinates.
    pub connection: ConnectionData,
    /// The metric tensor coefficients on this chart.
    pub metric: MetricData,
}
```

### 2.2 `Transition`

```rust
/// A chart-transition function T_ij : V_i -> V_j on overlap U_i n U_j.
///
/// Per Geometry of Sameness Def 18: each transition is a smooth map
/// with bounded Jacobian and inverse Jacobian. We store both the
/// forward map and its empirical Lipschitz constant for use in the
/// cocycle bound check.
pub struct Transition {
    pub from: ChartId,
    pub to: ChartId,
    /// Forward map. For analytic atlases this is a closed-form
    /// function; for learned atlases this is a neural network or
    /// piecewise-linear interpolator.
    pub forward: Box<dyn Fn(&FiberPoint) -> Result<FiberPoint, TransitionError> + Send + Sync>,
    /// Inverse if available (most natural transitions are invertible).
    pub inverse: Option<Box<dyn Fn(&FiberPoint) -> Result<FiberPoint, TransitionError> + Send + Sync>>,
    /// Empirical Lipschitz constant on the overlap region. Used in
    /// the cocycle-discrepancy first-order bound (T2 §3.2).
    pub lipschitz_estimate: f64,
}
```

### 2.3 `SpectralRegime` (the T5 honest-disclosure type)

```rust
/// Declares the spectral regime of a sharded bundle so the engine can
/// route SPECTRAL queries correctly. From T5 §3.5: naive per-shard
/// lambda_1 only bounds global lambda_1 for naturally-clustered
/// partitions; expanders require Schur-complement-based computation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpectralRegime {
    /// The substrate is naturally clustered. Per-shard lambda_1 is a
    /// tight first-order bound on global lambda_1. Validated for slow-
    /// mixing graphs by T5 part (B).
    NaturallyCluster,

    /// The substrate is an expander. Naive sharded SPECTRAL is unreliable.
    /// The engine MUST EITHER refuse SPECTRAL queries (with a clear
    /// error) OR use the Schur-complement path (future sprint).
    Expander,

    /// The substrate has been certified via the Fiedler-vector-aligned
    /// partition test (conductance below threshold). Lower-confidence
    /// version of NaturallyCluster.
    CertifiedClusteredAt { conductance: f64 },
}
```

---

## 3. On-disk format

A sharded bundle is stored as a directory tree:

```
bundles/<name>/
├── atlas.dhoom           # Atlas struct serialized via DHOOM
├── transitions/
│   ├── 001-002.dhoom     # Transition from ChartId(1) to ChartId(2)
│   ├── 001-003.dhoom
│   └── ...
├── shards/
│   ├── shard-0001/
│   │   ├── charts.dhoom            # List of ChartIds owned
│   │   ├── records/
│   │   │   ├── chart-001.dhoom     # Records local to chart 001
│   │   │   ├── chart-002.dhoom
│   │   │   └── ...
│   │   └── overlap-cache.dhoom     # Pre-computed overlap data
│   └── shard-0002/
│       └── ...
├── manifest.dhoom         # Top-level: SpectralRegime, delta_cocycle_budget,
                           # shard registry, version
└── wal/
    └── ...                 # Per-shard WAL (one WAL per shard)
```

**Atomicity:** each shard's WAL provides single-shard durability. Cross-shard atomicity is a follow-up; current spec assumes single-shard writes are the unit.

**Compatibility:** for `n_shards = 1`, this format is equivalent to current single-node GIGI (the atlas has one chart and no transitions). The migration script in §9 produces this trivial-atlas form first; the genuine multi-shard form is a follow-up conversion.

---

## 4. Rust API surfaces

Located at `src/sharded/mod.rs` (new module):

```rust
//! Sharded GIGI primitives. Gated by `poincare_to_sharding.md` §3.

pub mod atlas;       // Atlas, ChartId, Transition, ChartMetadata
pub mod execution;   // Per-verb sharded execution recipes
pub mod resolver;    // Clean Finger Move write-conflict resolver
pub mod regime;      // SpectralRegime + routing decisions
pub mod gates;       // Non-vacuity gates (Davis 2026a A5)

pub use atlas::{Atlas, ChartId, Transition};
pub use execution::{shard_betti, shard_holonomy, shard_curvature, ...};
pub use resolver::{sharded_write_resolve, ResolverTrace};
pub use regime::SpectralRegime;
pub use gates::non_vacuity_check;
```

### 4.1 Bundle store extensions

The existing `BundleStore` trait grows a new method:

```rust
pub trait BundleStore {
    // ... existing methods ...

    /// Returns Some(&Atlas) if this bundle is sharded, None for
    /// single-node bundles. Allows sharded execution paths to be
    /// gated cleanly.
    fn atlas(&self) -> Option<&Atlas>;
}
```

### 4.2 New `ShardedBundle` type

```rust
/// A bundle store implementation backed by an atlas of charts split
/// across shards. Implements BundleStore.
pub struct ShardedBundle {
    atlas: Atlas,
    shards: HashMap<ShardId, Box<dyn ShardStore + Send + Sync>>,
}

impl BundleStore for ShardedBundle {
    fn atlas(&self) -> Option<&Atlas> { Some(&self.atlas) }
    // ... delegate per-record ops to the relevant shard via chart routing ...
}
```

---

## 5. Per-verb sharded execution recipes

Each recipe cites the §3.X claim from the theory doc and the TDD gate that validates it.

### 5.1 CURVATURE (T3 §3.3) — pure pointwise, no coordination

```rust
pub fn shard_curvature_at(bundle: &ShardedBundle, p: &FiberPoint) -> f64 {
    // Find the chart that owns p
    let chart_id = bundle.atlas.find_chart_for(p)
        .expect("point not in any chart");
    let shard = bundle.shard_for_chart(chart_id);
    // Compute K from chart-local metric -- no inter-shard traffic
    shard.compute_curvature_at(chart_id, p)
}
```

By T3: sheafification is exact. Multiple charts holding p (i.e., p in an overlap) MUST agree on K to within finite-difference precision; the engine asserts this in debug builds.

### 5.2 PERCEIVE, CAPACITY, HORIZON, DEPTH, LOCAL_HOLONOMY (T3 §3.3) — local-pointwise / local-temporal

All five Cognitive Geometry verbs follow the same per-chart pattern as CURVATURE. LOCAL_HOLONOMY specifically: the windowed-rotation defect is computed within whatever chart the trajectory is in; if the window crosses a chart boundary, the engine composes via the cocycle (§5.3 below).

### 5.3 HOLONOMY across shards (T4 §3.4) — cocycle composition

```rust
pub fn shard_holonomy_around_loop(
    bundle: &ShardedBundle,
    loop_points: &[FiberPoint],
) -> Result<RotationMatrix, ShardedExecError> {
    let mut accumulated = RotationMatrix::identity(bundle.fiber_dim());
    let mut current_chart: Option<ChartId> = None;

    for window in loop_points.windows(2) {
        let (from, to) = (&window[0], &window[1]);
        let target_chart = bundle.atlas.find_chart_for(to)?;

        if current_chart != Some(target_chart) {
            // Crossed a chart boundary -- apply the transition
            if let Some(prev) = current_chart {
                let trans = bundle.atlas.transition(prev, target_chart)?;
                let g = trans.jacobian_at(from)?;  // transition gauge factor
                accumulated = g.compose(&accumulated);
            }
            current_chart = Some(target_chart);
        }

        // Per-chart transport step
        let shard = bundle.shard_for_chart(target_chart);
        let segment_rot = shard.transport_step(target_chart, from, to)?;
        accumulated = segment_rot.compose(&accumulated);
    }

    Ok(accumulated)
}
```

By T4: this composition exactly equals direct global transport for analytic atlases; deviates by first-order slack proportional to `delta_cocycle` for learned atlases.

### 5.4 TRANSPORT, geodesic, SAMPLE_TRANSPORT (T4 §3.4) — same composition pattern

All path-based verbs follow the cocycle-composition pattern. The existing `flat_transport` becomes the per-chart primitive; the sharded wrapper handles chart-boundary crossings via the transition map.

### 5.5 BETTI / SEMANTIC (T1 §3.1) — Mayer-Vietoris assembly

```rust
pub fn shard_betti(bundle: &ShardedBundle, max_dim: usize) -> Vec<u32> {
    // Phase 1: each shard computes its per-chart boundary matrices in
    // parallel. No inter-shard communication.
    let per_chart_complexes: HashMap<ChartId, ChainComplex> = bundle
        .atlas
        .charts
        .keys()
        .par_iter()  // rayon
        .map(|chart_id| {
            let shard = bundle.shard_for_chart(*chart_id);
            (*chart_id, shard.local_chain_complex(*chart_id))
        })
        .collect();

    // Phase 2: assemble via the M-V short exact sequence. Pure linear
    // algebra on the (chain_complex, inclusion_data) tuples; no record
    // shipping.
    let assembled = mayer_vietoris_assemble(&per_chart_complexes, &bundle.atlas);
    assembled.betti_numbers(max_dim)
}
```

By T1: the assembly is exact (β_n recovered to integer precision on S¹, S², T²). Implementation reuses existing F₂-rank infrastructure from `src/discrete/f2_rank.rs` and `MorseCache` from `src/morse_cache.rs` (#216) keyed by per-chart `mutation_counter`.

### 5.6 SPECTRAL (T5 §3.5) — regime-routed

```rust
pub fn shard_lambda_1(bundle: &ShardedBundle) -> Result<f64, ShardedExecError> {
    match bundle.atlas.spectral_regime {
        SpectralRegime::NaturallyCluster | SpectralRegime::CertifiedClusteredAt {..} => {
            // T5 Part (B) validated: naive bound holds
            let per_shard_lambdas: Vec<f64> = bundle.shards.values()
                .par_iter()
                .map(|s| s.local_lambda_1())
                .collect();
            Ok(per_shard_lambdas.into_iter().fold(f64::INFINITY, f64::min))
        }
        SpectralRegime::Expander => {
            // T5 disclosed: naive recipe unreliable. Two options:
            //   (a) refuse with explicit error -- current default
            //   (b) Schur-complement path -- future sprint (gate T7)
            Err(ShardedExecError::ExpanderRegimeUnsupportedSPECTRAL)
        }
    }
}
```

The engine refuses rather than silently lies. Consumers can:
- Set `SpectralRegime::NaturallyCluster` explicitly when they know it.
- Use `CertifiedClusteredAt` after running a Fiedler-vector partition check.
- Wait for Schur-complement support (TBD sprint).

### 5.7 Writes — Clean Finger Move resolver (T6 §3.6)

```rust
pub fn sharded_write_resolve(
    bundle: &mut ShardedBundle,
    conflicts: Vec<WriteConflict>,
) -> Result<ResolverTrace, ResolverError> {
    // Precondition: every conflict has a canceling partner (H_2 = 0
    // analog). The engine validates this -- if a conflict has no
    // partner, it is rejected at the gate before resolution begins.
    let unresolved = validate_canceling_pair_structure(&conflicts)?;

    let mut trace = ResolverTrace::new(unresolved.len());
    while let Some(pair) = find_canceling_pair(&unresolved) {
        bundle.apply_resolution(&pair)?;
        unresolved.remove(pair.a_id);
        unresolved.remove(pair.b_id);
        trace.step(pair, unresolved.len());
        // Invariant: trace.last_decrease_by == 2  (Davis Thm 5.3)
    }

    Ok(trace)
}
```

By T6: terminates in `initial_count / 2` steps for any density / ordering when the precondition holds. Engine asserts the monotonic-decrease invariant in-loop.

---

## 6. Configuration: `BundleSchema` extensions

```rust
pub struct BundleSchema {
    // ... existing fields ...

    /// If Some, this bundle is sharded with this atlas declaration.
    /// If None, single-node bundle (current behavior; backward
    /// compatible).
    pub atlas: Option<AtlasDeclaration>,
}

pub struct AtlasDeclaration {
    pub n_shards: u32,
    pub chart_assignment: ChartAssignmentStrategy,
    pub delta_cocycle_budget: f64,
    pub spectral_regime: SpectralRegime,
}

pub enum ChartAssignmentStrategy {
    /// Assign records to charts by a hash on the primary key.
    /// Suitable when the manifold structure has no natural geometric
    /// partition.
    HashByPrimaryKey,
    /// Assign records to charts via a Fiedler-vector partition of
    /// the adjacency graph. Optimal for SpectralRegime::NaturallyCluster.
    FiedlerVector { n_partitions: u32 },
    /// User-defined predicate. Each record routed by a callback.
    Custom(String),  // path to a registered chart-router fn
}
```

---

## 7. Non-vacuity gates (Davis 2026a A5)

At each insert and each shard-boundary computation, the engine enforces:

```rust
pub fn non_vacuity_check(chart: &ChartMetadata, atlas: &Atlas) -> Result<(), GateError> {
    let kappa_soft = chart.kappa_soft();
    let eps_dist = chart.distortion_at(chart.operational_horizon())?;
    let R = chart.geodesic_radius_of_interest();

    if kappa_soft - 2.0 * eps_dist * R <= 0.0 {
        return Err(GateError::NonVacuityViolated {
            chart: chart.id,
            kappa_soft, eps_dist, R,
        });
    }

    // Cocycle slack check on this chart's transitions
    for (other, trans) in atlas.transitions_from(chart.id) {
        let observed = atlas.measure_cocycle_slack(chart.id, other)?;
        if observed > atlas.delta_cocycle_budget {
            return Err(GateError::CocycleBudgetExceeded {
                pair: (chart.id, other),
                budget: atlas.delta_cocycle_budget,
                observed,
            });
        }
    }
    Ok(())
}
```

This is the engine-enforced version of the Davis manifold paper's §A5 non-vacuity condition and Geometry of Sameness Def 21 cocycle bound. Inserts that would violate either gate are rejected, with the budget and observed values surfaced to the consumer for diagnostic.

---

## 8. Acceptance criteria

The implementation is COMPLETE when:

1. All 6 TDD gates in `validation/` remain GREEN.
2. New Rust integration tests under `tests/sharded_*.rs` cover each per-verb recipe in §5 against the validated math (mirror of the Python TDD gates).
3. `cargo test --lib --features kahler` passes 100%, no regression from current 1124 tests.
4. `cargo test --bin gigi-stream --features kahler` passes 100%, with new sharded HTTP endpoints contract-tested.
5. The migration script in §9 produces a valid sharded bundle from any existing single-node bundle, with `n_shards=1` (trivial-atlas form) byte-equivalent to the input.
6. The non-vacuity gates correctly refuse inserts that would violate them, with informative errors.
7. The SpectralRegime routing correctly refuses Expander+SPECTRAL with a clear error (no silent failures).

---

## 9. Migration path

### Phase A (sprint N): single-shard backward compatibility

Existing bundles continue to work unchanged. The atlas API returns `None` for non-sharded bundles. All existing verbs run as before. **Zero regression risk.**

### Phase B (sprint N+1): trivial-atlas conversion

A new CLI tool `gigi-shard --convert <bundle>` produces a sharded form with `n_shards=1`. The trivial atlas has one chart, no transitions. The on-disk format matches §3 but is byte-equivalent to single-node form when decoded.

This validates the codec without introducing real sharding complexity.

### Phase C (sprint N+2): hash sharding with M-V BETTI

`gigi-shard --convert <bundle> --shards=8 --strategy=hash` produces a real multi-shard form. BETTI / SEMANTIC verbs route through M-V assembly (§5.5). Other verbs route through chart-local computation (§5.1, §5.2).

This sprint validates the cocycle composition and M-V assembly paths against real (non-toy) data.

### Phase D (sprint N+3): Fiedler-partition + SPECTRAL routing

`gigi-shard --convert <bundle> --shards=8 --strategy=fiedler` produces a naturally-clustered partition. SPECTRAL routes via §5.7 NaturallyCluster path.

### Phase E (sprint N+4): Schur complement for expanders

Validates and ships T7 (future TDD gate) for Schur-complement-based sharded SPECTRAL. Removes the `Expander` regime's refusal; replaces with the new computation path.

Each phase ships independently. The math holds at every phase; the engineering surface grows monotonically.

---

## 10. Open questions for follow-up

1. **Schur complement implementation** for `SpectralRegime::Expander`. TDD gate T7 needed. Spec section §5.6 gains the new path.

2. **Distributed transaction protocol** for cross-shard atomic writes. Current spec assumes single-shard writes; multi-shard atomicity is a Raft / Paxos / 2PC follow-up.

3. **Dynamic re-sharding** under load. Currently shards are fixed at creation; re-sharding requires offline conversion. Online re-sharding is a sprint-N+5 follow-up.

4. **Cross-atlas bundle joins.** When two sharded bundles need to share data (e.g., Marcella's source embeddings + PRISM's reconciliation manifold), the cross-atlas cocycle is undefined. The Geometry of Sameness functors F_S, G_S give the theoretical answer; engineering is TBD.

5. **GPU acceleration** of M-V assembly. Per-shard F₂ rank parallelizes naturally; assembling the M-V cokernel matrix is non-trivial on GPU. Future perf sprint.

---

## 11. The bottom line

The sharding spec is gated on six math claims. All six are TDD-green as of `d6f4821`. The implementation can ship sprint-by-sprint (§9 phases A–E) without ever breaking single-node compatibility. The math is correct at every phase; the engineering surface only grows.

The "huge client who will need the scale" can run on the Phase A or Phase B form on day one (trivial atlas), with the multi-shard Phase C–E forms ready as load grows. The substrate doesn't change; only the partition shape.

**Companion documents:**
- [`poincare_to_sharding.md`](poincare_to_sharding.md) — theory + TDD gate manifest
- [`validation/`](validation/) — six green tests
- [`validation/run_all.py`](validation/run_all.py) — master TDD runner
