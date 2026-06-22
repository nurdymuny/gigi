# WISH Extensions — Implementation Log

**Date**: 2026-06-22
**Trigger**: Halcyon Reply 4 (`HALCYON_TO_GIGI_LETTER_2026-06-22.md`) — 4 WISH asks + 1 ride-along ask (§5 LOOP_TRANSPORT non-U_lt)
**Substrate reply**: `GIGI_TO_HALCYON_REPLY_2026-06-22_WISH_EXTENSIONS.md` (committed at `fabc3a9`)
**Connection-as-primary correction**: `GIGI_TO_HALCYON_REPLY_2026-06-22_CONNECTION_AS_PRIMARY_ACCEPTED.md` (committed at `2ebdae9`)

This log records the four substrate-side ships that landed in response, plus the §5 ride-along.

## Scope — what shipped

### ASK 1 — Lift WISH off `dim == 2` (connection-as-primary trait surface)

**Problem**: `src/imagine/wish.rs:191` returned `WishError::UnsupportedDim` for `dim != 2`. The `WishMetric2D` trait was closed-sealed with four hardcoded 2D impls (`S2Stereographic`, `T2Flat`, `CurvaturePinch`, `CP1FubiniStudy`). Halcyon's buckyball substrate could not use WISH because:

1. Buckyball is a 3D embedding of a 2-sphere; the natural induced metric is not the load-bearing surface for LGT canon.
2. The load-bearing object is the SU(2) gauge **connection** on the principal bundle over the buckyball, not the induced metric.
3. Davis Duality bound `c1·||Ω||·h² ≤ ε ≤ c2·||Ω||·h²` is on the **connection's** curvature 2-form Ω, not the metric's Riemann tensor.

**Ship** (`src/imagine/wish.rs`):

```rust
pub trait WishMetric {
    fn dim(&self) -> usize;
    fn name(&self) -> &'static str;
    fn metric_tensor(&self, p: &[f64]) -> Vec<Vec<f64>>;
    fn connection(&self, p: &[f64]) -> WishConnectionLocal;
    fn segment_energy_nd(&self, a: &[f64], b: &[f64]) -> f64;
    fn evaluate_observable(&self, _name: &str, _accumulated: &[f64]) -> Result<f64, WishError> {
        Err(WishError::ObservableUnknown(_name.to_string()))
    }
}

pub struct WishConnectionLocal {
    pub christoffel: Vec<Vec<Vec<f64>>>,  // Γ^k_{ij}(p)
}

pub struct WishMetricRegistry {
    factories: HashMap<String, Box<dyn Fn() -> Box<dyn WishMetric>>>,
}
// pub fn register / get_factory / list / contains / clear  (mirrors HamiltonianRegistry)
```

**Migration of legacy 2D impls** — `S2Stereographic`, `T2Flat`, `CurvaturePinch`, `CP1FubiniStudy` get a default trivial connection derived from their existing `metric_tensor` (Levi-Civita Christoffel symbols). Existing WISH paths recompute byte-identical. The `WishMetric2DAdapter<T: WishMetric2D>` blanket adapter wraps any 2D impl through the n-D trait — no rewrites required.

**N-D dispatch entry point**:

```rust
pub fn relaxation_solve_nd(
    metric: &dyn WishMetric,
    seed: &[f64],
    target: &WishTarget,
    cfg: &WishConfig,
) -> Result<WishOutcome, WishError>;
```

**Out of scope for this commit** (deferred to Halcyon-side follow-up + a later substrate workflow):

- WAL event `WishMetricDeclare` / `WishBundleDeclare` op code
- Full `WishBundle` surface with `parallel_transport` / `curvature` 2-form / `Holonomy` types — Hallie's verbatim sketch includes `parallel_transport(p, v, ξ) → FiberVec` + `curvature(p, X, Y) → CurvatureOp` + `evaluate_observable(name, accumulated_holonomy) → f64`; the substrate ships the trait surface + registry first so `dim != 2` no longer panics, then the full bundle surface lands once Halcyon's SU(2) impl is sketched
- `BasePoint` / `FiberVec` / `TangentVec` / `Holonomy` / `CurvatureOp` concrete types — `WishConnectionLocal` is a placeholder; substantive shape decision deferred

### ASK 2 — `WishTarget::Observable { name, value, err }`

**Problem**: `WishTarget` had only `Coords(Vec<f64>)` and `Record { bundle, record_id }`. Halcyon needs to target paths by observable evaluation (e.g. "find the path whose endpoint has `davis_capacity = 0.42 ± 0.01`"), not by endpoint coordinates.

**Ship**:

```rust
pub enum WishTarget {
    Coords(Vec<f64>),
    Record { bundle: String, record_id: String },
    Observable { name: String, value: f64, err: f64 },  // NEW
}
```

`WishTargetProvenance` gains the matching variant + `From<&WishTarget>` impl. `render_target_label` formats as `observable:<name>=<value>±<err>`.

**Convergence logic**: `relaxation_solve_target` dispatches by target variant. Observable target uses a different convergence condition — named observable evaluation closeness, not coordinate closeness — with σ-weighted residual on failure.

**Default observable evaluators on legacy 2D impls**: `scalar_curvature`, `exp2phi`, `radius_chart` covered. Bundle-specific observables route through `WishMetric::evaluate_observable`.

### ASK 3 — Per-segment capacity (path-aware) with optional flag

**Problem**: `src/imagine/wish.rs:166-167` capacity was "Populated by Phase 4; Phase 3 reports `f64::NAN`." Whole-path capacity landed NaN on the path Halcyon hit. Halcyon's substrate catalog needs per-segment `τ_segment / κ_segment` at every interior WISH path node.

**Hallie §4 refinement (verbatim)**: *"If the segment-level integration is cheap to add, that's the version we want. If it costs significantly more than whole-path-only, a flag in WishConfig (`compute_per_segment_capacity: bool`) is fine — the substrate-catalog protocol always wants it; downstream consumers that only need the endpoint capacity can leave it off."*

**Ship**:

```rust
pub struct WishConfig {
    // ... existing fields ...
    #[serde(default)]
    pub compute_per_segment_capacity: bool,  // NEW, default false (backwards-compatible)
}

pub enum WishOutcome {
    Granted {
        // ... existing fields ...
        segment_capacities: Option<Vec<f64>>,  // NEW, None by default (additive)
    },
    // ...
}
```

When `compute_per_segment_capacity == true`:

- In `relaxation_solve`'s path-building loop, compute per-segment `τ` (curvature integral) + `κ` (energy integral) at each interior node
- Capacity = `τ_i / κ_i` per Davis Duality `C = τ/K` observable
- NaN handling: capacity = NaN if `κ_segment < 1e-12`; otherwise finite `τ/κ`
- Sum of per-segment τ + κ agrees with the whole-path number (consistency check covered by test)

When `compute_per_segment_capacity == false` (default): byte-identical to existing behavior. Existing destructures in `src/bin/gigi_stream.rs:16005` and `tests/wish_wire.rs` use `..` rest pattern so the additive field is non-breaking.

### ASK 4 — `INTEGRATE_ALONG_PATH` verb (two-form path-handle syntax)

**Hallie §4 decision (verbatim)**: *"the two-form syntax you sketched — 'LET path = IMAGINE FROM ... TO ...; INTEGRATE OBSERVABLE \<name\> ALONG path;' — is what we would reach for. The path-handle pattern lets the catalog protocol bind the path once and integrate multiple observables along it (davis_capacity, tau_density, kappa_density, plus whatever signatures we add later) without re-running WISH."*

**Ship** — new GQL surface in `src/parser.rs`:

```sql
LET path = IMAGINE FROM (x, y) DIRECTION (dx, dy)
  PATH_LENGTH <l> STEPS <n> ON <bundle>;
INTEGRATE OBSERVABLE <name> ALONG path [RETURNS SCALAR];
```

**Mechanism**:

- `Statement::LetPathFromImagine { ident, ... }` binds the path expression into a process-wide `path_registry` (`OnceLock<Mutex<HashMap>>` mirroring `gauge::loop_transport::REG`)
- `Statement::IntegrateAlongPath { observable_name, path_ident }` looks the binding up and trapezoidally line-integrates
- Multiple `INTEGRATE OBSERVABLE` calls against the same handle reuse the bound records — no IMAGINE re-run (the whole point of the path-handle pattern)
- Observable evaluation dispatches by name through `src/imagine/observables.rs` (`arc_length_unit`, `local_k`, `accumulated_holonomy`, `path_length_so_far`) for canonical observables, with bundle-specific dispatch via `WishMetric::evaluate_observable`
- `δs` (segment arc length) from the bundle's induced metric (or connection's natural arc-length if no metric)
- Trapezoidal accuracy first ship; Simpson's-rule upgrade can land later if a downstream consumer needs it
- `INTEGRATE` keyword peeks at the next word so existing `INTEGRATE … OVER … MEASURE` aggregator callers are unaffected

**Out of scope** for this commit:

- `LetPathFromWish` variant — only `LetPathFromImagine` ships; the WISH-path version lands after the full `WishBundle` surface is in place

### ASK 5 (ride-along) — `LOOP_TRANSPORT` accepts non-`U_lt` first arg

**Problem**: `src/parser.rs:10380` hardcoded `loop_transport(stmt, "U_lt", "E_lt")`. If GIBBS_SAMPLE turns out to be the chain-continues-from-current-state case (Hallie's hypothesis from CSPRNG sentinel data), the orchestrator-side fix needs per-seed UUID-suffixed scratch GAUGE_FIELDs — and that requires `LOOP_TRANSPORT` to accept a non-`U_lt` first arg.

**Ship**:

- `Statement::LoopTransport` variant gains `gauge_field_name: String` + `e_field_name: String` fields, parsed from optional `GAUGE_FIELD <name>` and `E_FIELD <name>` clauses between the lattice ident and `ALONG_LOOP`
- Defaults: `U_lt` and `E_lt` (byte-identical to every existing Halcyon Part VI gold fixture)
- Executor destructures the new fields and passes them as `&str` to `gauge::loop_transport::loop_transport(stmt, u, e)` — that function already accepted `u_name` / `e_name` as `&str` params, so the change is purely structural
- Five existing test files that destructured `Statement::LoopTransport` exhaustively (no `..`) had to absorb the two new fields — byte-neutral to the gold fixtures
- Registry-miss surfaces through existing typed `UFieldNotDeclared(String)` channel

**Backwards compat**: any GQL still saying `LOOP_TRANSPORT U_lt ...` works unchanged (looks up `"U_lt"` in the registry).

## Tests

- `tests/wish_extensions_halcyon_asks.rs` — 12 tests for ASKs 1+2+3 (trait surface, observable target, per-segment capacity flag)
- `tests/wish_integrate_along_path.rs` — 5 tests for ASK 4 (two-form syntax, multiple integrals same path, unknown observable / path errors)
- `tests/loop_transport_first_arg_flex.rs` — 4 tests for ASK 5 (non-U_lt named field, U_lt backwards compat, unknown field clear error)

## Locked gates green

```
cargo test --no-default-features --lib                                       875/0  (was 870 baseline; +5 from new virtual_bundles unit tests in commit 3)
cargo test --features kahler --test davis_conjecture_lambda_brain_ridealong  25/0
cargo test --features halcyon --test halcyon_part_iv_gold --release          4/0 + 1 ignored (tdd_hal_iv_10_a_symplectic_flow_canonical by design)
cargo test --features halcyon --test halcyon_part_vi_bit_identity_gold       3/0
cargo test --features halcyon --test halcyon_part_vi_6_semantic_thermalized  5/0
cargo test --features halcyon --test halcyon_part_vi_parser_grammar          3/0
cargo test --features halcyon --test halcyon_part_vi_gc_acceptance           6/0
```

## Files

**Modified**:
- `src/imagine/wish.rs` — WishMetric trait surface, WishConnectionLocal, WishMetricRegistry, WishTarget::Observable variant, WishConfig.compute_per_segment_capacity, WishOutcome::Granted.segment_capacities, relaxation_solve_nd entry point
- `src/imagine/provenance.rs` — WishTargetProvenance::Observable sister variant + From impl + render_target_label
- `src/imagine/mod.rs` — module registrations for observables / path_registry
- `src/parser.rs` — INTEGRATE OBSERVABLE ALONG <ident> parser + Statement::IntegrateAlongPath + Statement::LetPathFromImagine + Statement::LoopTransport gauge_field_name/e_field_name fields + executor
- `src/gauge/loop_transport.rs` — destructure absorbs the new fields via `gauge_field_name: _` / `e_field_name: _`
- `tests/halcyon_part_vi_bit_identity_gold.rs`, `tests/halcyon_part_vi_gc_acceptance.rs`, `tests/halcyon_part_vi_parser_grammar.rs`, `tests/halcyon_part_vi_sham_dispatch.rs` — exhaustive destructure absorption (byte-neutral)

**Created**:
- `src/imagine/observables.rs` — canonical observable dispatch (arc_length_unit, local_k, accumulated_holonomy, path_length_so_far)
- `src/imagine/path_registry.rs` — process-wide path-handle registry (OnceLock<Mutex<HashMap>> pattern)
- `tests/wish_extensions_halcyon_asks.rs` — 12 tests (ASKs 1+2+3)
- `tests/wish_integrate_along_path.rs` — 5 tests (ASK 4)
- `tests/loop_transport_first_arg_flex.rs` — 4 tests (ASK 5)

## Deferred (NOT in this commit)

- Full `WishBundle` surface (parallel_transport / curvature 2-form / Holonomy types)
- WAL `WishMetricDeclare` / `WishBundleDeclare` op codes
- Buckyball SU(2) `WishBundle` impl — Halcyon-side, after the substrate trait surface lands
- `LetPathFromWish` variant — bind a WISH-emitted path to a `LET` ident
- Simpson's-rule line integration (trapezoidal first ship is sufficient)
- GIBBS_SAMPLE auto-correlation discriminator test — separate substrate investigation per Halcyon §3
