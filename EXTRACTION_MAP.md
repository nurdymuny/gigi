# EXTRACTION_MAP — decomposing `src/bin/gigi_stream.rs`

The stream binary reached ~26k lines (25,922 at commit `1bdcff0`) with 125
routes in one file. This map records the route-family survey, what has been
extracted so far, and the recommended order + risks for the rest, so any
future session can continue mechanically.

**Line numbers below are anchored to commit `1bdcff0` (pre-extraction) unless
marked otherwise.** After phase 1, everything past old line 9313 has shifted;
use the symbol names (grep) as the durable anchors.

## Method (proven in phase 1)

1. Branch `refactor/stream-extraction-<phase>`.
2. Move the family's free functions + request/response structs into lib
   modules (`src/<area>/…`), `pub` visibility, wired into `src/lib.rs`.
   Handlers stay in the binary as thin wrappers calling `gigi::<area>::*`.
   Mechanical move only: the sole edits to moved text are `gigi::` →
   `crate::` paths and `pub` on items/fields.
3. Move the inline `#[cfg(test)]` tests that exercise the moved free
   functions into the lib modules; test fixtures go in a `cfg(test)`
   `test_support` module inside the area (do NOT reach back into the
   binary). Tests that drive handlers stay in the binary.
4. Gates after each family, all four must be green and total test count
   (lib + bin) must not drop:
   ```
   cargo test --lib
   cargo test --bin gigi-stream
   cargo test --lib --features post_kahler_phase1
   cargo test --bin gigi-stream --features post_kahler_phase1
   ```
5. One commit per family, prefix `refactor(stream):`.

File practicalities: the binary is UTF-8 **with BOM**, working tree is CRLF
(`core.autocrlf=true`). Scripted edits must preserve both.

## Status

| # | Family | Routes | ~Lines | Risk | Status |
|---|--------|--------|--------|------|--------|
| 1 | ML suite (scan/scan_fit/cluster/infer/reduce/prescribe/solve/circulation/factorize/changepoints + ml_catalog) | 11 | 4,410 | LOW | **DONE — phase 1** (`e4fdc8b`) |
| 2 | Post-Kähler PK-1..4 REST (fisher_metric/persistence/wasserstein/reeb_flow) | 4 | 600 | LOW-MEDIUM | pending |
| 3 | Patterns / hunt (Ask G surface) | 4 | 400 | LOW | pending |
| 4 | Transactions Phase-A (tx_begin/write/commit/rollback/status) | 5 | 390 | LOW-MEDIUM | pending |
| 5 | WebSockets + dashboard | 4 | 700 | MEDIUM | pending |
| 6 | Brain primitives (`/brain/*` ×17) | 17 | 5,320 | MEDIUM-HIGH | pending |
| 7 | Halcyon / Kähler gauge verbs (perceive … wish) | 16 | 2,230 | MEDIUM | pending |
| 8 | Geometry analytics reads (curvature … predict[volatility]) | 16 | 1,150 | MEDIUM | pending |
| 9 | Core bundle CRUD + query + import/export + dhoom ingest + vector search | 39 | 4,180 | HIGH-MEDIUM | pending |
| 10 | GQL executor block (`/v1/gql` + `/v1/public/gql`) | 2 | 3,430 | HIGH | pending |
| 11 | Admin / durability / infra root (auth, StreamState, middleware, metrics, snapshot, main) | 7 | 3,080 | LOW (stays as binary root) | shrinks as others leave |

Total routes: 125.

## Phase 1 — ML suite (DONE)

Branch `refactor/stream-extraction-phase1`, two code commits:

- `a1e4a70` — pre-fix: gated `extract_field_samples_skips_poisoned_record`
  on `kahler`. The test (added in `44161e2`) calls the cfg(kahler)-only
  `extract_field_samples`, so `cargo test --bin gigi-stream` with **no
  features had been failing to compile since it landed**. With kahler on,
  nothing changes.
- `e4fdc8b` — the extraction itself.

What moved (old lines 9314–12569, minus the handlers) → `src/ml/`:

| Module | Contents |
|--------|----------|
| `src/ml/scan.rs` | `scan_trigrams`, `scan_jaccard`, `scan_solve`, `ScanRequest`, `ScanLenses`, `scan_compute_lenses`, `ScanFitRequest`, fold/epoch defaults |
| `src/ml/cluster.rs` | `ClusterRequest`, `ClusterOpts`, `ClusterResult`, `gmm_labels`, `mat_inv_logdet`, `kmeans_lloyd`, `cluster_records` |
| `src/ml/infer.rs` | `SupervisedPredictRequest`, `PredictResult`, `local_linear_at`, `local_linear_scaled`, `build_diffusion_graph`, `diffuse`, `predict_field` |
| `src/ml/reduce.rs` | `ReduceRequest`, `ReduceResult`, `pca_reduce` |
| `src/ml/prescribe.rs` | `prescribe_top_eigs`, `PrescribeRequest`, `PrescribeResult`, `prescribe_fingerprint` |
| `src/ml/solve.rs` | `SolveRequest`, `RidgePoint`, `SolveResult`, `fit_store_solve` (nalgebra thin SVD) |
| `src/ml/circulation.rs` | `CirculationRequest`, `CirculationResult`, `circulation_flow` |
| `src/ml/factorize.rs` | `FactorizeRequest`, `FactorizeResult`, `factorize_matrix` |
| `src/ml/changepoints.rs` | `ChangepointRequest`, `ChangepointResult`, `detect_changepoints` |
| `src/ml/test_support.rs` | cfg(test) fixtures: `tmp_dir`, `cleanup`, `scan_rec`, `scan_env`, `scan_lens` |

What stayed in the binary:

- The 11 async handlers (`bundle_scan` … `bundle_changepoints`) and the
  `ml_catalog` handler — thin wrappers over `gigi::ml::*` (they own
  `Arc<StreamState>` / `ErrorResponse` / JSON shaping).
- The 11 route registrations (unchanged — handlers didn't move).
- `ml_all_endpoints_regression_smoke` + its own copies of
  `scan_rec`/`scan_env` — deliberately kept in the binary so the
  re-exported functions are exercised across the crate boundary.
- `extract_field_samples_skips_poisoned_record` — tests the cfg(kahler)
  shared helper `extract_field_samples` (brain/PK/GQL family), NOT ML.

36 unit tests moved into the lib modules. Counts (before → after):
no features lib 927 → 963, bin 104 → 68 (total 1031 = 1031); with
`post_kahler_phase1` lib 1227 → 1263, bin 126 → 90 (total 1353 = 1353).
Zero failures. `src/bin/gigi_stream.rs`: 25,923 → 22,157 lines.

Dependency proof that made this the cleanest cut: within the moved block
there are zero references to `extract_field_samples`, `heap_or_promote`,
`not_found`/`bad_request`, `lambda_budget_for_bundle`, `record_to_json`/
`value_to_json`/`json_to_value`; the external surface is exactly
`Arc<StreamState>::engine_read()`, `ErrorResponse`, axum/serde, and the
gigi crate. The GQL executor does not dispatch into ML functions.

## Remaining families — recommended order + seams

### 2. Post-Kähler PK-1..4 REST (next)

- Code (old lines): 12571–12912 — `bundle_fisher_metric` 12587,
  `WassersteinRequest`/`bundle_wasserstein` 12658/12683,
  `bundle_persistence` 12763, `ReebFlowRequest`/`bundle_reeb_flow`
  12836/12851. Route regs 19853–19858 (own cfg-gated statement, clean
  splice). Tests 22490–22735 (`pk_endpoints_end_to_end`,
  `pk_gql_verbs_end_to_end`, `pk_reeb_rejects_wrong_arity`).
- Shared helpers: `not_found` (7916), `bad_request` (7926),
  `heap_or_promote` (7957) — all cfg(kahler), shared with brain;
  `extract_field_samples` (4568) — triple-shared (brain + PK + GQL verb
  arms). **Hoist the 7913–7992 helper island to a shared `pub(crate)`
  module first**; `extract_field_samples` goes to shared, never into a
  family module.
- Risk LOW-MEDIUM: feature-gated (`post_kahler_phase1`), but the error/
  store helpers are cfg(kahler), so the extracted module inherits an
  implicit kahler dependency. The PK **GQL verb arms** (`pk_row` 16924,
  arms ~17790–17910 inside `execute_gql_on_store_read`) belong to the GQL
  family and must stay. `pk_gql_verbs_end_to_end` exercises the GQL side —
  keep it with the GQL tests or split it. PK tests share
  `stream_env_lock` / `post_gql_for_test` / `post_gql_body_for_test` with
  GQL tests — leave those helpers shared in the binary.

### 3. Patterns / hunt

- Code 19264–19625 (`PatternListEntry`, `DefinePatternRequest`,
  `HuntRequest`, `list_patterns`, `define_pattern_http`,
  `drop_pattern_http`, `hunt_http`, `envelope_to_json`,
  `hunt_row_to_json`). Route regs 19650–19655 (own cfg `patterns`
  statement). Tests: `hunt_row_to_json` tests 23056–23095.
- Only cross-family touch: `value_to_json` (1082, wire-converter module).
- Risk LOW.

### 4. Transactions Phase-A

- Code 20443–20834 (`TxBeginRequest` … `tx_status`; `sys_time_to_iso`
  20518 is local). Route regs 19721–19727 (own cfg `transactions`
  statement).
- Seam: `OpenTx` / `StreamState.open_txs` (256–267) live on the shared
  state — the module needs visibility into state internals; also
  `json_to_value`/schema coercion in `tx_write`.
- Risk LOW-MEDIUM.

### 5. WebSockets + dashboard

- Code 14874–15529 (`Subscription`, `now_ms`, `build_dashboard_event`, ws
  handlers, `serve_dashboard`). Route regs 19869–19876.
- Seam: `SubscriptionEvent` (268) / `DashboardEvent` (283) /
  `StreamState::get_or_create_channel` (431) are **published into by the
  CRUD write handlers** — the event types and channel accessors must live
  in a shared state module; extract only the handler/protocol code.
  `record_to_json`, `dhoom_value_to_value` (15476) shared.
- Risk MEDIUM.

### 6. Brain primitives (largest)

- Code: 4549–7912 plus 7994–9010 — everything between
  `flat_transport_endpoint`'s close (4547) and `consistency_check` (9011)
  EXCEPT the shared island 7913–7992 (`not_found`/`bad_request`/
  `heap_or_promote`). Route regs 19920–20022 (three cfg kahler
  statements). Tests: 22736–22866 (flow cache), 24741–25119 (sudoku wire),
  25120–25258 (sample_transport), 25259–25496 (intent_gate) — plus the
  poisoned-record test wherever `extract_field_samples` lands.
- Seams: the helper island must be hoisted BEFORE this and PK move (PK at
  order 2 should do the hoist). `extract_field_samples` → shared.
  `lambda_budget_for_bundle` (845) also serves analytics + CRUD query
  meta. `BundleFlowCache` invalidation is keyed by the per-bundle write
  counter (`bundle_counter_header` 4927) — it must keep observing CRUD
  writes.
- Risk MEDIUM-HIGH; all cfg(kahler).

### 7. Halcyon / Kähler gauge verbs

- Code: 3007–4547, 20328–20442 (`kahler_transport_dispatch`),
  20835–21300 (commutator + wish). Route regs scattered across seven
  statements (19799–19807, 19813–19815, 19824–19827, 19888–19892,
  19880–19883, 19897–19914, 19732–19736, 19742–19745); external gauge
  routes merge at 19773–19774 via `gigi::gauge::http::build_router()`
  (already lib). Tests: 24632–24740.
- Seams: `kahler_transport_dispatch` is called from the GQL executor
  (18021) — keep `pub(crate)`/shared. `dial_error_to_http` (2875) shared
  with analytics. Five feature flags (kahler, imagine, causal_states,
  wish, gauge).
- Risk MEDIUM.

### 8. Geometry analytics reads

- Code: three islands — 2752–3006, 9011–9312, 12913–13075. Route regs
  19793–19798, 19830–19837, 19861–19865 (19837 `/anomalies` sits directly
  above the now-removed ML lines in the same `let app = app …;`
  statement). Tests: 25497–25632.
- Seams: `lambda_budget_for_bundle`/`ResponseWithLambda` (821–886) also
  used by brain + CRUD query meta → hoist to shared before this and brain.
  Report structs 710–1006; `gigi_welford_radius` (2826);
  `dial_error_to_http` shared with family 7.
- Risk MEDIUM.

### 9. Core bundle CRUD + query + import/export + dhoom + vector search

- Code: 448–708, 1027–1250, 1860–2751, 13076–14873, 18565–18640. Route
  regs 19657–19709. Tests: 22867–23055, 23096–23153, 23427–24038.
- Seams: owns the wire-converter set (`json_to_value`, `value_to_json`,
  `schema_coerce`, `record_to_json`, `str_to_field_type`, …, 1042–1250)
  used by GQL, websockets, tx_write, patterns, log_bundle_writer — hoist
  to a wire module FIRST. Write paths publish subscription events (WS
  channels) and bump the brain flow-cache write counter — preserve call
  order.
- Risk HIGH-MEDIUM. Extract late, after the shared-module boundary is
  proven by the smaller families.

### 10. GQL executor block

- Code: 15694–18564 contiguous (`validate_public_stmt`, `public_gql_query`,
  `gql_query`, `execute_gql_on_engine`, `execute_gql_with_exists`,
  `execute_gql_on_store_read` — the ~1,400-line verb match —
  `get_bundle_name`, `gql_stmt_type_name`, `exec_error_to_response`,
  `exec_result_to_response`). Route regs 19776–19789 (public route
  conditionally mounted on `state.public_bundles`). Tests: 23154–23426,
  25633–25922, shared `stream_env_lock` + `post_gql_for_test`.
- Seams: reaches into three other families — `extract_field_samples`
  (brain-shared), `kahler_transport_dispatch` (halcyon), the CRUD wire
  converters. Extract second-to-last, after all its dependencies are
  already lib/shared modules. Its env-mutating tests (`GIGI_DATA_DIR`
  under `stream_env_lock`) are order-sensitive.
- Risk HIGH.

### 11. Admin / durability / infra root

- IS the shared core: `GigiClaims`/`verify_gigi_token` (71–160),
  `StreamState` + channels (161–447), middleware (1251–1642), health +
  prometheus (1643–1859), `openapi_spec` (15504), `admin_snapshot`
  (15530–15565), log config (15566–15693), tigris/S3 sync (18641–18689),
  `init_system_bundles`/`init_app_bundles` (18690–19025),
  `log_bundle_writer` (19026–19193), `ttl_eviction_task` (19194–19263),
  `main` + router assembly (19626–20327). Tests: 24039–24631.
- Stays as the binary root and shrinks as families leave. Every extraction
  edits `main()`'s router assembly — splice one family per commit and
  re-run the gates. **`admin_snapshot` is the durability wedge the
  substrate records depend on: do not reorder its handler relative to the
  engine lock discipline.**

## Cross-family shared modules to create along the way

| Shared item | Old lines | Consumers | Hoist before |
|---|---|---|---|
| `not_found` / `bad_request` / `heap_or_promote` (cfg kahler) | 7913–7992 | brain, PK REST | family 2 |
| `extract_field_samples` (cfg kahler) | 4568–4687 | brain, PK REST, GQL verb arms | family 2 |
| `lambda_budget_for_bundle` / `ResponseWithLambda` | 821–886 | analytics, brain, CRUD query meta | family 6/8 |
| Wire converters (`json_to_value`, `value_to_json`, `record_to_json`, `schema_coerce`, …) | 1042–1250 | CRUD, GQL, WS, tx, patterns, log writer | family 9 |
| `SubscriptionEvent` / `DashboardEvent` / channel accessors | 268–447 | WS (consumer), CRUD writes (producer) | family 5 |
| `kahler_transport_dispatch` | 20328–20442 | halcyon REST + GQL executor | family 7 |
| `dial_error_to_http` | 2875 | analytics capacity/horizon/depth + sharded handlers | family 7/8 |
