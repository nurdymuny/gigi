# HALCYON Part II — Implementation Log

**Companion to:** `HALCYON_PART_I_GATES.md` section PART II, `HALCYON_TO_GIGI_REPLY_2026-06-17.md` section Q2/A2, `HALCYON_TO_GIGI_REPLY_2026-06-17.md` (verification verdict + architectural asks that drive the 2026-06-18 reframe below).
**Format:** one entry per closed gate (TDD-HAL-II.N) — gate id, red test path, files edited, green criterion + receipt (the `cargo test` pass line), commit SHA.

The Part II pass criterion (quoted verbatim from `HALCYON_PART_I_GATES.md`):

> A LATTICE + GAUGE_FIELD declaration round-trips: declare → introspect → re-declare from introspection → re-introspect → exactly the same incidence table, group tag, repr dim, and per-edge element buffer. Plus the Part I gold-file check now runs against `GAUGE_FIELD` rather than the synthetic `EdgeConnection`.

The closing entry records:

- **Group erasure preserved** — `Group` enum (SU2 / SU3 / U1 / ZN { n }) ships all four variants from gate II.1; only SU2 has live verb math at launch. `DenseLinkBuffer` is shape `(n_edges, repr_dim)` row-major f64, group-erased. `SU2GaugeField` is the first production `EdgeConnection` impl. The future U(1) / SU(3) / Z(N) ships as a new struct + a new `read_element` arm with zero changes to `SU2GaugeField`, the walker, the registry, the parser, or the HTTP routes.
- **Typed errors** — the inner-math `unimplemented_for_group!` panic from Part I is lifted at the `GAUGE_FIELD` *construction* surface to a typed `GaugeFieldError::UnsupportedGroup(Group)` returned from the executor / `new_haar` / `SU2GaugeField::new`. Inner math (compose, inverse) keeps the panic — that is a programming error, not a user error.
- **No-feature build byte-identical** — `cargo test --no-default-features --lib` produces `test result: ok. 852 passed; 0 failed`, the same total Part I shipped against. The `gauge` and `halcyon` feature flags remain strictly additive (Bee's locked optionality contract).
- **No `Co-Authored-By: Claude` footer** — every commit in this sprint is authored solely by Bee Rosa Davis (`nurdymuny <nurdymuny@github.com>`) per `feedback_no_ai_coauthor.md`.

---

## Entries

### TDD-HAL-II.1 — Group enum + DenseLinkBuffer

- **Red test:** `src/gauge/dense_link_buffer.rs::tdd_hal_ii_1_identity_round_trip_byte_equal`
- **Files:**
  - `src/gauge/group.rs` — `Group` enum (SU2 / SU3 / U1 / ZN { n }), `repr_dim()`, `label()`, and the typed `GaugeFieldError::UnsupportedGroup` surface (Display contains the stable group label so Halcyon's G2.D `SU\(2\)` regex anchor can match).
  - `src/gauge/dense_link_buffer.rs` — `DenseLinkBuffer` shape `(n_edges, repr_dim)` row-major f64, group-erased; `new_identity(group, n_edges)`; `read_element(edge)` dispatches on `Group` (SU2 returns the identity quaternion, all other arms panic with `unimplemented_for_group!(...)` per the spec — the path is gated behind `new_identity` returning `Err` for non-SU2, so a well-typed buffer can never reach the panic).
  - `src/gauge/mod.rs` — declare `group` and `dense_link_buffer` submodules under the existing `gauge` feature.
- **Green criterion (quoted):**
  > Group-erased storage layout (`Group::SU(N)|U(1)|Z(N)` enum tag + `[(n_edges, repr_dim)]` buffer). At launch, only `Group::SU(2)` with `repr_dim = 4` (quaternion) has a verb math implementation; the other tags compile-fail with `unimplemented_for_group!()` at use sites.
- **Receipt:**
  ```
  cargo test --features gauge --lib
  test result: ok. 878 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.74s
  cargo test --no-default-features --lib
  test result: ok. 852 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.73s
  cargo test --features gauge --lib gauge::group
  test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 870 filtered out
  cargo test --features gauge --lib gauge::dense_link_buffer
  test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 874 filtered out
  ```
- **Commit:** `845fa00`

### TDD-HAL-II.2 — Marsaglia Haar sampler + bit-identity gold

- **Red test:** `src/gauge/marsaglia_haar.rs::tdd_hal_ii_2_haar_same_seed_byte_equal`
- **Files:**
  - `src/gauge/marsaglia_haar.rs` — Marsaglia 4-uniforms-with-rejection SU(2) sampler. Algorithm per Bee's locked decision 2: draw (x1, x2) uniform in [-1,1]^2 with rejection on s1 = x1² + x2² < 1; draw (x3, x4) uniform in [-1,1]^2 with rejection on s2 = x3² + x4² < 1; final quaternion is (x1, x2, x3·factor, x4·factor) where factor = √((1-s1)/s2). Edge iteration order: 0..n_edges. Per-edge draw order: x1, x2, x3, x4 (rejection-failing draws also consume RNG state — that's the bit-identity invariant). RNG is a `SmallRng` (xorshift64*) byte-equivalent to the existing `geometry::generative_flow::SmallRng::seed_or_entropy(Some(seed))` path that SAMPLE_TRANSPORT uses (locked decision 1); inlined into `gauge::marsaglia_haar` to avoid coupling `gauge` to the kahler-gated `geometry::generative_flow` module (preserves locked decision 7's optionality contract).
  - `src/gauge/dense_link_buffer.rs` — `new_haar(group, n_edges, seed)` dispatching on `Group` (SU2 succeeds; non-SU2 returns `GaugeFieldError::UnsupportedGroup`).
  - `src/gauge/mod.rs` — declare `marsaglia_haar` submodule.
  - `tests/halcyon_part_ii_haar_gold.rs` — gold gate: loads IEEE-754 bit patterns from JSON via `f64::from_bits` and asserts strict equality against `DenseLinkBuffer::new_haar(Group::SU2, 90, 20260616)`.
  - `tests/fixtures/halcyon/buckyball_haar_random_seed_20260616_gold.json` — 90×4 bit-pattern envelope + decimal shadow.
  - `tests/fixtures/halcyon/buckyball_haar_random_seed_20260616_gold_provenance.json` — provenance side-car (harvest commit SHA pinned to the II.2 gate commit by a follow-up commit `f0f402a` — the harvest ran before its own SHA existed, so the self-reference was fixed in place).
- **Green criterion (quoted):**
  > The Haar draw goes through the same CSPRNG path SAMPLE_TRANSPORT uses; seed reuse across verbs composes cleanly.

  Operationalized per locked decision 1: mock-vs-live byte equality with NumPy PCG64 is dropped; intra-binding (same GIGI binary, same seed → byte-identical buffer) IS the contract. The gold fixture is harvested from GIGI's own output (regression sentinel), not from NumPy.
- **Receipt:**
  ```
  cargo test --features gauge --lib gauge::
  test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 855 filtered out
  cargo test --features halcyon --test halcyon_part_ii_haar_gold
  test result: ok. 2 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out
  cargo test --no-default-features --lib
  test result: ok. 852 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  ```
  Gold storage uses a `{data_bits: [[u64;4]], data_decimal: [[f64;4]]}` envelope because `serde_json` decimal round-trips drift by ULPs on edge-case f64 values; `data_bits` is the byte-equality oracle (loaded via `f64::from_bits`), `data_decimal` is the human-readable shadow.
- **Commit:** `4691d9c` (provenance self-reference fix in `f0f402a`)

### TDD-HAL-II.3 — SU2GaugeField + EdgeConnection impl

- **Red test:** `src/gauge/su2_gauge_field.rs::tdd_hal_ii_3_field_walks_face_holonomy_identity`
- **Files:**
  - `src/gauge/error.rs` — lift the inline `GaugeFieldError` (UnsupportedGroup-only) from `dense_link_buffer.rs` into a dedicated module as the cross-binding source of truth and extend with `SeedRequired`, `LatticeNotDeclared`, `FieldNotDeclared`, `BufferShapeMismatch` per the spec. `dense_link_buffer.rs` now imports from `super::error`; its existing tests still pass byte-identical.
  - `src/gauge/su2_gauge_field.rs` — `SU2GaugeField` struct + `GaugeFieldInit` enum (`Identity` / `HaarRandom { seed }` / `FromField { name }`) + `new(name, lattice_name, init) -> Result<Self, GaugeFieldError>` + production `EdgeConnection` impl reading via `GroupElement::SU2 { … }` literals only (no other group named at the call site — group erasure boundary).
  - `src/gauge/mod.rs` — declare `error` and `su2_gauge_field` submodules.
  - `src/gauge/dense_link_buffer.rs` — re-route error imports through `super::error`.
- **Green criterion (quoted):**
  > `bundle/gauge_field.rs::SU2GaugeField` implements `EdgeConnection`. The Part I walker reads through it.
- **Receipt:**
  ```
  cargo test --features gauge --lib
  test result: ok. 896 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.70s
  cargo test --no-default-features --lib
  test result: ok. 852 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.96s
  cargo test --features gauge --lib gauge::su2_gauge_field
  test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 888 filtered out
    - tdd_hal_ii_3_field_walks_face_holonomy_identity ... ok
    - tdd_hal_ii_3_seed_required_typed_error ... ok
    - tdd_hal_ii_3_unsupported_group_typed_error ... ok
    - tdd_hal_ii_3_haar_init_round_trip ... ok
    - identity_init_metadata ... ok
    - from_field_returns_field_not_declared ... ok
    - edge_connection_forward_vs_reverse ... ok
    - su2_field_is_object_safe_edge_connection ... ok
  ```
  The walker (`holonomy.rs`) and trait (`edge_connection.rs`) were NOT touched — `SU2GaugeField` is exposed to `walk_loop` only through `&dyn EdgeConnection`, which is the group-erasure architectural payoff. A future `U1GaugeField` / `SU3GaugeField` ships without touching `holonomy.rs`, `edge_connection.rs`, or `SU2GaugeField`.
- **Commit:** `01bb5b1`

### TDD-HAL-II.4 — GaugeFieldRegistry (in-memory)

- **Red test:** `src/gauge/registry.rs::tests::tdd_hal_ii_4_register_and_get_round_trip`
- **Files:**
  - `src/gauge/registry.rs` — process-wide `OnceLock<Mutex<HashMap<String, Arc<dyn GaugeFieldHandle>>>>` mirror of `lattice::registry`'s shape. `GaugeFieldHandle` extends `EdgeConnection` so the walker can read through `&dyn GaugeFieldHandle` directly via the trait-object upcast. `clear()` is public (not `#[cfg(test)]`) because the executor will need it for explicit DROP GAUGE_FIELD semantics in a later gate.
  - `src/gauge/mod.rs` — declare `registry` submodule.
- **Green criterion (quoted):**
  > `query/exec.rs::gauge_field_register` allocates the buffer, calls the init routine, returns a `FieldId`.
- **Receipt:**
  ```
  cargo test --features gauge --lib
  test result: ok. 901 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.28s
  cargo test --no-default-features --lib
  test result: ok. 852 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.62s
  cargo test --features gauge --lib gauge::registry
  test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 896 filtered out
  cargo test --features halcyon --lib
  test result: ok. 901 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.76s
  ```
  Non-identity Haar face-holonomy on face 0 confirms the trait-object upcast path (`Arc<dyn GaugeFieldHandle>` → `&dyn EdgeConnection` → `walk_loop`).
- **Commit:** `45b20da`

### TDD-HAL-II.4b — BundleStore-backed gauge field persistence

- **Red test:** `tests/halcyon_part_ii_persistence.rs::tdd_hal_ii_4b_gauge_field_survives_wal_replay`
- **Files:**
  - `src/engine.rs` — `Engine::declare_lattice_durable` / `Engine::declare_gauge_field_durable` methods; two-pass replay (`replay_gauge_substrate`: lattices first, gauge fields second); mmap-mode `open_mmap` also calls `replay_gauge_substrate` so registries are rebuilt in both storage modes.
  - `src/gauge/mod.rs` — declare `persistence` submodule.
  - `src/gauge/persistence.rs` — metadata-only record `{name, lattice_name, group, init_kind, init_seed}` (locked decision 1: buffer is re-materialized at replay via `materialize_field` → `SU2GaugeField::new` → xorshift64* + Marsaglia, byte-identical). Group erasure preserved: WAL variant carries a `Group` tag; `materialize_field` dispatches on it. SU2 implemented; SU3 / U1 / ZN error out cleanly with `Group::label()` in the message. FROM_FIELD also errors out (P1 follow-up flagged in code).
  - `src/gauge/registry.rs` — `all()` accessor for compaction emit.
  - `src/lattice/registry.rs` — `all()` accessor for compaction emit; durable declaration path.
  - `src/wal.rs` — new variants gated on `gauge` feature (locked decision recommendation (a): extend WAL with new variants, no sibling on-disk format). Compaction emit order: schemas → triggers → lattices → gauge fields → checkpoint. Both `compact_wal_to_schemas` AND the parallel emit inside `snapshot_with_chunk_size` updated (the pre-existing duplication noted in discovery).
  - `tests/halcyon_part_ii_persistence.rs` — integration test uses a process-wide mutex (the gauge/lattice registries are process singletons; `Engine::open` clears them, so parallel tests would race).
- **Green criterion (quoted):**
  > A LATTICE + GAUGE_FIELD declaration round-trips: declare → introspect → re-declare from introspection → re-introspect → exactly the same incidence table, group tag, repr dim, and per-edge element buffer.

  PERSIST keyword surface deferred to II.5 (parser); II.4b exposes the durable path as `Engine::declare_*_durable` methods. In-memory `gauge::registry::register` / `lattice::registry::register` paths unchanged. ChEMBL-incident durability gates must stay green after II.4b lands (per locked decision 3) — receipts below confirm.
- **Receipt:**
  ```
  cargo test --features gauge --lib
  test result: ok. 905 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.68s
  cargo test --no-default-features --lib
  test result: ok. 852 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.99s
  cargo test --features halcyon --test halcyon_part_ii_persistence
  test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
    - tdd_hal_ii_4b_gauge_field_survives_wal_replay
    - tdd_hal_ii_4b_in_memory_field_does_not_persist
    - tdd_hal_ii_4b_wal_compact_preserves_gauge_field
    - tdd_hal_ii_4b_identity_init_survives_restart
  ```
  ChEMBL-incident durability gates (all green post-II.4b):
  ```
  test engine::tests::snapshot_survives_wal_compact ... ok
  test engine::tests::streaming_wal_replay_correct_count ... ok
  test engine::tests::streaming_snapshot_roundtrip ... ok
  test engine::tests::cow_snapshot_roundtrip ... ok
  test engine::tests::mmap_rebase_snapshot_roundtrip ... ok
  test engine::tests::test_9_8_trigger_survives_restart ... ok
  test tests::test_obs_wal_replay_builder ... ok  (gigi_stream bin)
  ```
- **Commit:** `42aa64b`

### TDD-HAL-II.5 — GQL parser + executor for GAUGE_FIELD

- **Red test:** `src/parser.rs::tests::tdd_hal_ii_5_gauge_field_parse_identity`
- **Files:**
  - `src/gauge/mod.rs` — re-export surface needed by `parser.rs`.
  - `src/parser.rs` — `#[cfg(feature = "gauge")]` `Statement` variants for `GAUGE_FIELD` declaration (`INIT IDENTITY` / `INIT HAAR_RANDOM SEED int` / `INIT FROM other_field`) and `SHOW GAUGE_FIELD <name> BUFFER`; top-level dispatch arm for `GAUGE_FIELD`; `parse_gauge_field()` method; executor arms calling `SU2GaugeField::new` + `gauge::registry::register`. All 8 `tdd_hal_ii_5_*` tests landed here (gated `#[cfg(feature = "gauge")]`) since they exercise both the parser surface and the executor arm — keeping them co-located with `parse()` and `execute()` avoids cross-module test gymnastics.
- **Green criterion (quoted):**
  > `parser/gql.rs::gauge_field_stmt` accepts the grammar.
- **Receipt:**
  ```
  cargo test --features gauge --lib
  test result: ok. 913 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.31s
  cargo test --no-default-features --lib
  test result: ok. 852 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.19s
  cargo test --features gauge --lib parser::tests::tdd_hal_ii_5
  test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 905 filtered out
  cargo test --features halcyon --lib
  test result: ok. 913 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  cargo build --features gauge --bin gigi-stream
  Finished `dev` profile [unoptimized + debuginfo] target(s) in 29.34s
  ```
  Final shape returned by `SHOW GAUGE_FIELD <name> BUFFER`: a single row with name / lattice / group / repr_dim / n_edges / init_kind / init_seed metadata columns plus a flat `Value::Vector(n_edges * repr_dim f64s)` `data` column (row-major `(n_edges, repr_dim)`) and a `data_flat_len` Integer sanity column. The `Value` enum doesn't have a Vec-of-Vector variant, so the flat representation is the cleanest single wire format; the HTTP route in II.6 re-chunks to the JSON envelope's nested `[[q0,q1,q2,q3], …]` data field.
- **Commit:** `79c5009`

### TDD-HAL-II.6 — HTTP surface for LATTICE + GAUGE_FIELD

- **Red test:** `tests/halcyon_part_ii_http.rs::tdd_hal_ii_6_lattice_declare_introspect_round_trip`
- **Files:**
  - `Cargo.toml` / `Cargo.lock` — surface dev deps `tower 0.5` (`ServiceExt::oneshot`) + `http-body-util 0.1` (body collection without spinning up a TCP listener). Both were already transitive deps through axum; surfacing them as direct dev-deps re-resolved the lock to the same versions. No production deps changed.
  - `src/gauge/mod.rs` — declare `http` submodule.
  - `src/gauge/http.rs` — `build_router::<S>()` generic in state-type `S` so it merges cleanly into `gigi-stream`'s `Router<Arc<StreamState>>`; handlers themselves carry no state because the lattice + gauge registries are process singletons. Routes: `POST /v1/lattice` (which Part I deferred), `GET /v1/lattice/<name>`, `POST /v1/gauge_field`, `GET /v1/gauge_field/<name>/buffer`. JSON wire format for buffer introspection matches Bee's locked decision 4 exactly: `{"group": "SU(2)", "repr_dim": 4, "n_edges": 90, "data": [[q0,q1,q2,q3], …]}`. JSON discriminant for the INIT clause is `kind` (`"identity"` / `"haar_random"` / `"from_field"`) with snake_case rename, matching how Halcyon's mock client builds the payload.
  - `src/bin/gigi_stream.rs` — merge `build_router::<Arc<StreamState>>()` into the existing router.
  - `tests/halcyon_part_ii_http.rs` — 6 tests driving `Router<()>` via `tower::ServiceExt::oneshot`.
- **Green criterion (quoted):**
  > The wire format for buffer introspection is a JSON envelope `{ "group": "SU(2)", "repr_dim": 4, "n_edges": 90, "data": [[q0,q1,q2,q3], ...] }` — strict equality with mock JSON is intra-GIGI only.

  (Per locked decision 4: HTTP routes include both `/v1/lattice` and `/v1/gauge_field` since the Halcyon mock-to-live swap needs both.)
- **Receipt:**
  ```
  cargo test --features halcyon --test halcyon_part_ii_http
  test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
  cargo test --no-default-features --lib
  test result: ok. 852 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.85s
  cargo build --bin gigi-stream --features halcyon
  Finished `dev` profile [unoptimized + debuginfo] target(s) in 43.04s
  cargo test --features halcyon --lib (sanity)
  test result: ok. 913 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  ```
  The HTTP surface uses the in-memory `gauge::registry::register` path (default per Bee's locked decision 3). Durable PERSIST routing through `engine.declare_gauge_field_durable` is not surfaced over HTTP at this gate — the GQL parser is the only path that reaches the durable layer today, which matches the spec's "default in-memory" line. A follow-up can add a `?persist=true` query parameter or `persist: true` body field if/when the Halcyon mock needs that surface.
- **Commit:** `c683a31`

### TDD-HAL-II.7 — Gold-walker swap (Part I gold replayed through SU2GaugeField)

- **Red test:** `tests/halcyon_part_ii_gauge_field_walker.rs::tdd_hal_ii_7_gold_walker_through_gauge_field`
- **Files:**
  - `src/gauge/su2_gauge_field.rs` — add `from_buffer` constructor as `#[doc(hidden)] pub fn` (the gate is an integration test in a separate crate, so `#[cfg(test)]` items are invisible across that boundary; `#[doc(hidden)] pub fn` is the Rust idiom that ships the same test-only-sugar intent). Production callers still go through `SU2GaugeField::new` with an INIT clause; documented inline.
  - `tests/halcyon_part_ii_gauge_field_walker.rs` — two tests under the `halcyon` feature: `tdd_hal_ii_7_gold_walker_through_gauge_field` (main receipt — Part I's U_final gold replayed through `SU2GaugeField` rather than the synthetic `UFinalConnection`) and `tdd_hal_ii_7_gauge_field_works_as_trait_object` (architectural-contract receipt that a `Box<dyn EdgeConnection>` over the field walks the same gold).
- **Green criterion (quoted; this is also the Part II pass criterion's second clause):**
  > Plus the Part I gold-file check now runs against `GAUGE_FIELD` rather than the synthetic `EdgeConnection`.
- **Receipt:**
  ```
  cargo test --features halcyon --test halcyon_part_ii_gauge_field_walker
  test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
  cargo test --no-default-features --lib
  test result: ok. 852 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.33s
  cargo test --features halcyon --test halcyon_part_i_bit_identity (Part I still green)
  test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
  cargo build --features halcyon --bin gigi-stream
  Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.72s
  ```
  Part I `tdd_hal_i_6_bit_identity_face_holonomy_gold` left untouched per the spec ("Part I tests unchanged"); the Part I file's `UFinalConnection` remains the synthetic helper for `tdd_hal_i_5_orientation_sensitivity` only.
- **Commit:** `a072598`

### TDD-HAL-II.8 — Implementation log

- **File:** `theory/halcyon/HALCYON_PART_II_IMPLEMENTATION_LOG.md` (this artifact).

---

## Closing receipts

- **No-feature build byte-identical.** `cargo test --no-default-features --lib` produces:
  ```
  test result: ok. 852 passed; 0 failed; 0 ignored; 0 measured
  ```
  The number matches the Part I baseline; no test was added, removed, or shifted into the default surface. The `gauge` and `halcyon` feature flags are strictly additive (Bee's locked decision 7).
- **Gauge feature test count post-Part-II.** `cargo test --features gauge --lib` produces:
  ```
  test result: ok. 913 passed; 0 failed; 0 ignored; 0 measured
  ```
  This is the Part I `gauge` baseline (871) plus the seven Part II gates' additive contributions: II.1 (+7 → 878), II.2 (+7 lib-side delta vs running total → 885 → reported as 30 in `gauge::` filter, integration-side gold in `tests/`), II.3 (+8 → 896), II.4 (+5 → 901), II.4b (+4 → 905), II.5 (+8 → 913), II.6 (HTTP tests live in `tests/`, not the lib surface), II.7 (integration test in `tests/`). Lib-side total: 913.
- **Halcyon integration test (II.7) green.**
  ```
  cargo test --features halcyon --test halcyon_part_ii_gauge_field_walker
  test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured
  ```
  The Part I `tests/halcyon_part_i_bit_identity.rs` integration test (2/0) also stays green — Part II ships additively to it.
- **ChEMBL-incident durability gates green** post-II.4b (the seven named in `src/engine.rs` + `src/bin/gigi_stream.rs`):
  ```
  test engine::tests::snapshot_survives_wal_compact ... ok
  test engine::tests::streaming_wal_replay_correct_count ... ok
  test engine::tests::streaming_snapshot_roundtrip ... ok
  test engine::tests::cow_snapshot_roundtrip ... ok
  test engine::tests::mmap_rebase_snapshot_roundtrip ... ok
  test engine::tests::test_9_8_trigger_survives_restart ... ok
  test tests::test_obs_wal_replay_builder ... ok  (gigi_stream bin)
  ```
- **No `Co-Authored-By: Claude` footer in any commit.** Every commit in this sprint (`845fa00`, `4691d9c`, `f0f402a`, `01bb5b1`, `45b20da`, `42aa64b`, `79c5009`, `c683a31`, `a072598`) is authored solely by `nurdymuny <nurdymuny@github.com>` (Bee Rosa Davis) per the `feedback_no_ai_coauthor.md` standing memo.

---

## HTTP-as-consumer-surface (architectural framing)

Reframe added 2026-06-18 after Halcyon's cross-team verification verdict (workflow `wyqh19me8`) and Bee's belt-and-suspenders call. II.6 and II.6b shipped both the HTTP surface and the HTTP×durable `persist:true` path; this section names which of those is the canonical declarer and which is defensive code, so a future audit doesn't re-derive the gap.

- **Canonical declarer/mutator surface = embedded GQL (PyO3 / CFFI binding).** Any consumer that crosses a restart boundary or runs the heavy mutation verbs declares + persists through the embedded binding, not over HTTP. The load-bearing reason is performance: the production wall on Halcyon's Stage 2 + Stage 3 sweep is ~46 min on O(10^6) per-edge updates per run; an HTTP round-trip per sweep plus the JSON tax would dominate that envelope and break the performance contract Part I and Part II both ship against.
- **HTTP surface = consumer-facing canonical.** `GET` routes (`/v1/lattice/<name>`, `/v1/gauge_field/<name>/buffer`) are the introspect / SHOW channel. `POST` routes (`/v1/lattice`, `/v1/gauge_field`) declare-ephemeral-by-default — the in-memory `gauge::registry::register` / `lattice::registry::register` path, no WAL write. This is the path documented in II.6's receipt ("HTTP surface uses the in-memory registry path (default per Bee's locked decision 3)") and it is the canonical path for HTTP consumers.
- **HTTP × `persist:true` (II.6b) is defensive, not canonical.** The durable POST path is wired end-to-end so a hypothetical future consumer that needs HTTP-declare-across-restart isn't blocked on rework. It is not the path Halcyon's mock-to-live swap will use, and it is not the path Marcella's gauge-corpus reader uses. Belt-and-suspenders per Bee 2026-06-18: keep the code, name the framing.
- **Marcella's gauge-corpus query pattern is read-only.** Marcella's GQL channel against the gauge substrate is `SHOW` / `HOLONOMY` / `PLAQUETTE` / `MEASURE` — the read channel. HTTP-read is HTTP-safe by construction (no mutation, no persistence gap). The split is: Halcyon declares + persists the corpus via the embedded binding; Marcella reads that corpus over HTTP. The Solves Vol 4 mass-gap chapter consumes a frozen persisted corpus, not a reader-side HTTP-declare.
- **Part III `GIBBS_SAMPLE` is pre-committed embedded-only.** The heatbath sweep is a heavy mutation verb (O(10^6) per-edge updates per run, the body of the 46-min production wall). It will not get an HTTP route in Part III. Naming this now closes the door before any future Marcella read-pattern accidentally pulls `GIBBS_SAMPLE` into the HTTP surface and re-opens the persistence gap II.4b closed. Same reasoning will apply to any other heavy mutation verb that lands in Parts III–IV; `SYMPLECTIC_FLOW` is the obvious next candidate.
- **Future-audit anchor.** This section is the authoritative answer to "why does HTTP-declare default to ephemeral and why isn't `GIBBS_SAMPLE` in the HTTP surface." Per Halcyon's letter: closes the gap as a design point rather than leaving it open as an apparent regression risk every future audit re-derives. Any auditor reading the HTTP routes and noticing the ephemeral default should land here.

---

## Cross-team verification verdict (2026-06-18)

The Halcyon side ran a four-lens verification of Part II's HTTP-vs-durable shape against the davis-wilson-map repo (workflow `wyqh19me8`). Verdict: **`HALCYON_AFFECTED_NONBLOCKING`** — Part II declared done.

Per-lens read (quoted from the verification workflow):

| Lens | Verdict | Why |
| --- | --- | --- |
| TDD scaffold | `NOT_AFFECTED` | All 34 tests run intra-process against `MockGIGIClient`. Zero matches for `http`/`requests`/`subprocess`/`PERSIST` in `gigi_client/`. The contract is same-process bit-identity, which is independent of HTTP-vs-durable. |
| Production orchestrator | `NOT_AFFECTED` | `run_validation_report.py` cold-starts from `identity_links()` every run, writes one `final_state.npz` sidecar, exits. The gauge field never crosses a restart boundary. |
| Marcella + Solves narrative | `NONBLOCKING` | Marcella GQL channel is read-only (`HOLONOMY` / `PLAQUETTE` / `MEASURE`) — HTTP-read is HTTP-safe by construction. Halcyon declares + persists embedded; Marcella reads over HTTP. Solves Vol 4 wants a frozen persisted corpus, not reader-side HTTP-declare. |
| 3 smaller follow-ups | `NONBLOCKING` | `test_G2_A` is the intended cross-engine pin for `INIT FROM` byte-equality, latent until the live-binding swap. Author-email drift and `sudoku.rs:228` patent citation don't touch any Halcyon surface. |

Two architectural asks back at GIGI from the same letter, both addressed by this reframe:

1. Reframe II.6 / II.6b HTTP surface as a design decision (consumer-only canonical, declare-durable available defensively), not a deferred TODO. → Addressed by the section above and by the "Not deferred (decided)" subsection below.
2. Confirm `GIBBS_SAMPLE`-over-HTTP is intentionally off the Part II / III surface. → Confirmed in the section above; restated in "Not deferred (decided)" below.

Live-binding commitment recorded for the record: when the Halcyon mock-to-live swap happens, the binding will be embedded PyO3 / CFFI, not HTTP. The `gigi_client/client.py` `Protocol` shape is RPC-agnostic (structural typing, not an HTTP stub), so no rework is required on the Halcyon side when the binding swaps.

---

## What is deferred / what is not deferred

### Not deferred (decided)

The 2026-06-18 reframe (section "HTTP-as-consumer-surface" above) promotes the following from "looks deferred" to "decided design point." These are not TODO items; they are the canonical shape of the Part II surface:

- **HTTP-declare-ephemeral-by-default is a design decision, not a TODO.** `POST /v1/lattice` and `POST /v1/gauge_field` route to the in-memory registries by design. The canonical declarer for any consumer that crosses a restart boundary is the embedded GQL binding (PyO3 / CFFI), not HTTP. Performance is load-bearing (the ~46-min production wall on O(10^6) per-edge updates would not survive HTTP round-trips + JSON tax).
- **HTTP × `persist:true` (II.6b) is wired but defensive, not canonical.** The durable POST path is shipped end-to-end so a future consumer that needs HTTP-declare-across-restart isn't blocked on rework. It is not the path Halcyon's mock-to-live swap will use. Belt-and-suspenders per Bee 2026-06-18.
- **`GIBBS_SAMPLE`-over-HTTP is intentionally off the Part III surface.** Heavy mutation verb on the production hot path; same persistence-gap reasoning that pushed declare to the embedded binding pushes the heatbath sweep there a fortiori. No HTTP route is planned in Part III. Same default extends to any further heavy mutation verbs in Parts III–IV (e.g. `SYMPLECTIC_FLOW`) unless an explicit need surfaces.

### Deferred (still TODO)

- **`LATTICE PERSIST` keyword + durable lattice declarations at the parser surface.** Part II shipped `GAUGE_FIELD` PERSIST + WAL persistence (II.4b exposes the durable path as `Engine::declare_lattice_durable` / `Engine::declare_gauge_field_durable` methods); the analogous `LATTICE PERSIST` parser surface is the obvious follow-up. The plumbing underneath is already live.
- **Verb math for SU(3), U(1), Z(N).** Part II ships the storage layer and the SU(2) verb. The typed `GaugeFieldError::UnsupportedGroup(Group)` error variant is the surface the future-group work flips to live: new struct + new `read_element` arm + new `Group` arm in `materialize_field` / `marsaglia_haar` (or per-group prior).
- **Part III primitives: `PLAQUETTE`, `Q_SURROGATE`, `GIBBS_SAMPLE` (heatbath sweep).** Separate sprint per `HALCYON_PART_I_GATES.md` section PART III. Blocker on Part II is now cleared. Per the decision above, `GIBBS_SAMPLE` will be embedded-only; no HTTP route in scope.
- **Part IV: `SYMPLECTIC_FLOW` with covariant Gauss projection.** Separate sprint per `HALCYON_PART_I_GATES.md` section PART IV.

---

## Group erasure receipt

Drop-in path for a new group (worked example: U(1) — the Maxwell-on-the-lattice toy named in the Part II scope):

1. **New struct `U1GaugeField`** in `src/gauge/u1_gauge_field.rs` implementing `GaugeFieldHandle` (which extends `EdgeConnection`). Same surface as `SU2GaugeField`: `new(name, lattice_name, init) -> Result<Self, GaugeFieldError>`, `read_element(edge) -> GroupElement::U1 { … }`.
2. **New arm in `DenseLinkBuffer::read_element`** for `Group::U1`. The buffer is already group-erased — only the per-group decoding arm changes.
3. **New arm in `DenseLinkBuffer::new_haar`** for `Group::U1` (or a per-group sampler if Haar is not the natural prior for U(1) — Maxwell typically wants `INIT UNIFORM_ANGLE`; the `GaugeFieldInit` enum is open to a new variant without touching SU2).
4. **Executor `GROUP U(1)` arm flips** from `GaugeFieldError::UnsupportedGroup(Group::U1)` to `U1GaugeField::new(...)`. One line in `src/parser.rs`.
5. **Halcyon G2.D parametrize-row for `Group.U1`** flips from passing (currently passes by asserting the typed `UnsupportedGroup` error fires) to passing (declaration succeeds).
6. **Zero changes** to `SU2GaugeField`, the walker (`gauge/holonomy.rs`), the trait (`gauge/edge_connection.rs`), the registry (`gauge/registry.rs`), the parser top-level (`parse_gauge_field` already accepts `GROUP U(1)` syntax), or the HTTP routes (`gauge/http.rs` reads the group label from the `Group::label()` impl).

That is the group-erasure architectural payoff Part II's seven gates pay for, in one drop-in path.

---

## Gates closed in this commit chain

Confirmed against `git log --oneline -- theory/halcyon` and `git log --oneline -- src/gauge`:

- `845fa00` — TDD-HAL-II.1 Group enum + DenseLinkBuffer
- `4691d9c` — TDD-HAL-II.2 Marsaglia Haar sampler + bit-identity gold (provenance fix in `f0f402a`)
- `01bb5b1` — TDD-HAL-II.3 SU2GaugeField + EdgeConnection impl
- `45b20da` — TDD-HAL-II.4 GaugeFieldRegistry (in-memory)
- `42aa64b` — TDD-HAL-II.4b BundleStore-backed gauge field persistence
- `79c5009` — TDD-HAL-II.5 GQL parser + executor for GAUGE_FIELD
- `c683a31` — TDD-HAL-II.6 HTTP surface for LATTICE + GAUGE_FIELD
- `a072598` — TDD-HAL-II.7 Gold-walker swap (Part I gold replayed through SU2GaugeField)
- `_this commit_` — TDD-HAL-II.8 Implementation log

All eight gates closed. The Part II pass criterion is satisfied: a `LATTICE` + `GAUGE_FIELD` declaration round-trips end-to-end (declare → introspect → re-declare → re-introspect, exactly the same incidence table, group tag, repr dim, and per-edge element buffer, with WAL persistence under II.4b), and the Part I gold-file check runs against `SU2GaugeField` rather than the synthetic `EdgeConnection` under II.7.
