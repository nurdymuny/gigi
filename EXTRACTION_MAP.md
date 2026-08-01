# EXTRACTION_MAP — decomposing `src/bin/gigi_stream.rs`

The stream binary reached ~26k lines (25,922 at commit `1bdcff0`) with 125
routes in one file. This map records the route-family survey, what has been
extracted so far, and the recommended order + risks for the rest, so any
future session can continue mechanically.

**Line numbers below are anchored to commit `1bdcff0` (pre-extraction) unless
marked otherwise.** After phases 1–2, everything past old line 4548 has
shifted; use the symbol names (grep) as the durable anchors.

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
| 2 | Post-Kähler PK-1..4 REST (fisher_metric/persistence/wasserstein/reeb_flow) | 4 | 600 | LOW-MEDIUM | **DONE — phase 2** (`7f6d7a8`) |
| 3 | Patterns / hunt (Ask G surface) | 4 | 400 | LOW | **DONE — phase 2** (`8c9fb04`) |
| 4 | Transactions Phase-A (tx_begin/write/commit/rollback/status) | 5 | 390 | LOW-MEDIUM | **DONE — phase 2** (`c2e20d3`; wire types + helpers only, handler bodies stay in root — see record) |
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
| `src/ml/cluster.rs` | `ClusterRequest`, `ClusterOpts`, `ClusterResult`, `gmm_em`, `mat_inv_logdet`, `kmeans_lloyd`, `cluster_records` |
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

## Phase 2 — PK REST + patterns/hunt + tx wire types (DONE)

Branch `refactor/stream-extraction-phase2`, base `554ab62` (binary at
22,429 lines — post-phase-1 fixes on main had grown it from 22,157).
Three commits, one per family:

- `7f6d7a8` — family 2: PK-1..4 REST + the shared-helper-island hoist.
- `8c9fb04` — family 3: patterns/hunt + the `value_to_json` hoist.
- `c2e20d3` — family 4: transactions Phase-A wire types + helpers.

The hoists (two rows of the cross-family shared-modules table landed):

- `src/stream_shared.rs` (`gigi::stream_shared`) — `ErrorResponse`
  (ungated, `error` field now pub; required by ungated consumers) + the
  cfg(kahler) island `not_found` / `bad_request` / `heap_or_promote` +
  the triple-shared `extract_field_samples` (single definition now at
  `src/stream_shared.rs:21`) with its poisoned-record regression test
  (line 199). The binary re-imports all of them under the same cfg the
  removed items had, so every former call site (brain endpoints,
  `materialize_matrix_cached`, GQL verb arms, PK REST via the lib)
  resolves to the shared copy.
- `src/wire.rs` (`gigi::wire`) — `value_to_json` only (family 3's one
  cross-family touch). The rest of the wire-converter set stays in the
  binary and hoists with family 9.

What moved:

| Module | Contents |
|--------|----------|
| `src/geometry/pk_http.rs` | `fisher_metric`, `wasserstein` + `WassersteinRequest`, `reeb_flow` + `ReebFlowRequest` (cfg `post_kahler_phase1`) |
| `src/discrete/pk_http.rs` | `persistence` (cfg `post_kahler_phase1`) |
| `src/patterns/http.rs` | `PatternListEntry`, `DefinePatternRequest`, `HuntRequest` + `uses_v02_envelope`, the four handler bodies (list/define/drop/hunt), `envelope_to_json`, `hunt_row_to_json` + its two tests (cfg `patterns`) |
| `src/transactions/http.rs` | The seven Phase-A wire structs (`TxBeginRequest`/`Response`, `TxWriteRequest`/`Response`, `TxCommitResponse`, `TxRollbackResponse`, `TxStatusResponse`) + `parse_tx_id` + `sys_time_to_iso` (doubly gated: lib.rs cfg + inner `#![cfg]` at `src/transactions/mod.rs:66`) |

What stayed in the binary:

- All handlers as thin wrappers (lock acquisition + one call into the lib
  fn taking `&Engine` / `&mut Engine`) and all route registrations —
  sorted `.route(` diff vs main is empty, 120 route lines both sides.
- Family 4's five handler bodies (`tx_begin` … `tx_status`) **whole**, by
  design: they ARE the shared-state seam the map warned about (`OpenTx`,
  `StreamState.tx_registry`, `tx_snap_counter`, tx_commit's interleaved
  engine-write/registry lock discipline, `json_to_value` in tx_write).
  They now import their wire types from the lib; no state-abstraction
  layer was invented. They leave with family 11's root or a later pass.
- PK GQL verb arms + `pk_gql_verbs_end_to_end` (GQL family, as planned);
  `stream_env_lock` / `post_gql_for_test` helpers stay shared in the
  binary.

The wrapper seam (declared in the commits, verifier-confirmed
outcome-identical, no client-visible change): moving bodies behind the
lock-then-call wrapper shape widens lock scope marginally — wrappers now
take the engine lock before request validation/parse that previously ran
pre-lock (`list_patterns`, `define_pattern`, `drop_pattern`, `hunt`,
`reeb_flow`'s arity check), and `bundle_wasserstein` direct
(sample_a/sample_b) mode now takes the read lock it previously never
touched. Fine for these read-mostly families; see the family 6/9 notes
below before reusing the shape where lock order feeds caches or events.

Verified 2026-07-31 by two independent passes: (a) mechanical — all 31
moved items diffed body-for-body against main (not sampled); every delta
is `gigi::` → `crate::`, `pub`, the declared lock-line wrapper seam, or
`execute(&mut engine, ..)` → `execute(engine, ..)` where `engine` is the
`&mut` param; full 2,411-line src diff read, no smuggled behavior edits;
gating preserved exactly (`post_kahler_phase1 = ["kahler"]` in Cargo.toml
makes pk_http's use of kahler-gated helpers sound); cargo check under the
production feature combo clean with a warning set byte-identical to main.
(b) suites — all gates green, counts below observed directly.

Counts (before → after): no features lib 977 → 977, bin 72 → 72; with
`post_kahler_phase1` lib 1277 → 1278, bin 94 → 93 (total 1371 = 1371 —
the ±1 is `extract_field_samples_skips_poisoned_record` relocating
bin → lib, confirmed by sorted `cargo test -- --list` diff: that one name
is the only difference); with `patterns` lib 977 → 979, bin 74 → 72 (the
two `hunt_row_to_json` tests moved); lib with
`kahler patterns transactions` 1328. Zero failures everywhere.
`src/bin/gigi_stream.rs`: 22,429 → 21,524 lines.

## Remaining families — recommended order + seams

### 5. WebSockets + dashboard (next)

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
  25120–25258 (sample_transport), 25259–25496 (intent_gate). The
  poisoned-record test already landed with `extract_field_samples` in
  `src/stream_shared.rs` (`7f6d7a8`).
- Seams: the helper-island hoist is **DONE** (`7f6d7a8`,
  `gigi::stream_shared` — `not_found`/`bad_request`/`heap_or_promote` +
  `extract_field_samples` at `src/stream_shared.rs:21`); brain callers
  already resolve to the shared copies via the binary re-imports, so this
  family no longer carries a hoist step. Remaining seams:
  `lambda_budget_for_bundle` (845) also serves analytics + CRUD query
  meta. `BundleFlowCache` invalidation is keyed by the per-bundle write
  counter (`bundle_counter_header` 4927) — it must keep observing CRUD
  writes. Phase-2 caution: the thin-wrapper shape takes the lock before
  request parse (harmless for phase 2's read-mostly families) — brain
  handlers that read the write counter / flow cache must preserve their
  existing lock-vs-cache-check order, not blanket-adopt lock-first.
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
- Seams: owns the wire-converter set (`json_to_value`, `schema_coerce`,
  `record_to_json`, `str_to_field_type`, …, 1042–1250) used by GQL,
  websockets, tx_write, log_bundle_writer — hoist the remainder into
  `gigi::wire` FIRST (`value_to_json` is already there, `8c9fb04`).
  Aside (out of this map's scope): `src/edge.rs:463` and
  `src/bin/gigi_edge.rs:142` carry their own pre-existing `value_to_json`
  copies on main — optional later dedup into `gigi::wire`. Write paths
  publish subscription events (WS channels) and bump the brain flow-cache
  write counter — preserve call order; do NOT let the phase-2 lock-first
  wrapper shape reorder event publish vs counter bump.
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
- Seams: reached into three other families; one is resolved —
  `extract_field_samples` is now lib-shared (`gigi::stream_shared`,
  `7f6d7a8`). Still binary-bound: `kahler_transport_dispatch` (halcyon)
  and the CRUD wire converters. Extract second-to-last, after all its
  dependencies are already lib/shared modules. Its env-mutating tests
  (`GIGI_DATA_DIR` under `stream_env_lock`) are order-sensitive.
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
  engine lock discipline.** The tx Phase-A handler bodies
  (`tx_begin` … `tx_status`) now live here too by phase-2 decision
  (`c2e20d3`): they are shared-state logic (`OpenTx`, `tx_registry`,
  interleaved lock discipline), not family code.

## Cross-family shared modules to create along the way

| Shared item | Old lines | Consumers | Hoist before |
|---|---|---|---|
| `not_found` / `bad_request` / `heap_or_promote` (cfg kahler) | 7913–7992 | brain, PK REST | **DONE — `7f6d7a8` → `src/stream_shared.rs`** |
| `extract_field_samples` (cfg kahler) | 4568–4687 | brain, PK REST, GQL verb arms | **DONE — `7f6d7a8` → `src/stream_shared.rs:21`** |
| `lambda_budget_for_bundle` / `ResponseWithLambda` | 821–886 | analytics, brain, CRUD query meta | family 6/8 |
| Wire converters (`json_to_value`, `record_to_json`, `schema_coerce`, …; `value_to_json` already hoisted, `8c9fb04` → `src/wire.rs`) | 1042–1250 | CRUD, GQL, WS, tx, log writer | family 9 |
| `SubscriptionEvent` / `DashboardEvent` / channel accessors | 268–447 | WS (consumer), CRUD writes (producer) | family 5 |
| `kahler_transport_dispatch` | 20328–20442 | halcyon REST + GQL executor | family 7 |
| `dial_error_to_http` | 2875 | analytics capacity/horizon/depth + sharded handlers | family 7/8 |
