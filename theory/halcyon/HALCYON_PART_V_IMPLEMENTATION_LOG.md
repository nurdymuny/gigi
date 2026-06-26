# HALCYON Part V — Implementation Log

**Companion to:** HALCYON_PART_V_SNAPSHOT_GATES.md, HALCYON_PART_IV_IMPLEMENTATION_LOG.md.
**Format:** one entry per closed gate (TDD-HAL-V.N) — gate id + name, red test path (file::function), files touched, green criterion (verbatim from the spec / gate description), receipt (the `cargo test` pass line from the commit body), commit SHA.

The Part V pass criterion (quoted verbatim from `HALCYON_PART_V_SNAPSHOT_GATES.md` header, line 7):

> **Goal:** add the minimum WAL op + GQL verb that lets a thermalized `GAUGE_FIELD` survive a `gigi-stream` restart, so the public-receipt verifier in Solves Vol. 4 Appendix A.4 can hand a reader a one-line `GET` instead of a 30-second thermalization on every request.

What that translates into operationally, per `§3` of the gates doc:

> The op is **append-only on top of** the matching `OP_GAUGE_FIELD_DECLARE`. Replay walks WAL in order: the `0x0A` re-installs the declaration, the subsequent `0x0B` (if any) overwrites the link buffer with the snapshot's bytes.

Five engineering gates close that contract: a WAL op (V.1), a GQL verb + executor (V.2), a replay path with typed rejection on orphan / group-mismatch / checksum-mismatch (V.3), and two integration-level smoke + failure-mode gates that drive the full chain through `parser::execute` against a temp data dir, close the engine, re-open, and re-derive the snapshot SHA-256 from outside the WAL module (V.4 + V.5). Two earlier P-1 commits (V.0 + V.0b, from a prior session) closed the precondition `§2.5` named — `POST /v1/gql` was silently swallowing every gauge-feature statement, and `Statement::GaugeField` only registered into the dyn map, so the SU(2)-mut path could not find the field for the subsequent `GIBBS_SAMPLE`. Both are recorded here as gates V.0 and V.0b because they are the load-bearing prerequisites for everything Part V claims.

The closing entry records:

- **Optionality contract intact.** `cargo test --no-default-features --lib` still produces `test result: ok. 852 passed; 0 failed` at every gate. Byte-identical across the sprint. Every Part V surface is `#[cfg(feature = "gauge")]`-gated.
- **All four of Bee's locked decisions (D-V-A through D-V-D, ratified 2026-06-19) are wired through the executor.** Explicit little-endian in the WAL payload, `/v1/gql` only on the HTTP surface, SHA-256 over the LE buffer bytes as the canonical citation handle, and `PERSIST` REQUIRED on the GQL verb.
- **No `Co-Authored-By: Claude` footer on any commit.** Every commit in this sprint is authored solely by Bee Rosa Davis (`nurdymuny <bee_davis@alumni.brown.edu>`) per `feedback_no_ai_coauthor.md`.

---

## Entries

### TDD-HAL-V.0 — `/v1/gql` gauge-feature dispatch (P-1 precondition)

- **Red test:** `tests/halcyon_part_v_p1_gql_dispatch.rs::test_p1_gql_dispatches_gauge_statements_end_to_end` (the 5-step receipt from spec `§2.5`: `LATTICE` → `GET /v1/lattice/bb` → `GAUGE_FIELD` → `GIBBS_SAMPLE` → `SELECT PLAQUETTE`, each on `POST /v1/gql`)
- **Files:**
  - `src/bin/gigi_stream.rs` — feature-gated match prefix in `gql_query` before the bundle-aware path. Dispatches `Statement::Lattice`, `GaugeField`, `ShowGaugeField`, `GibbsSample`, `EField`, `SymplecticFlow`, `ShowEField`, `SelectHTotal`, `SelectGaussResidualMax` through `gigi::parser::execute` → `exec_result_to_response`. ~30 LOC, same shape as the existing bundle-statement arm.
  - `src/halcyon_gql_dispatch.rs` — new helper module enumerating the gauge-family variants in one place so the wire and the executor stay in sync. (V.2 later extends this list with `Statement::Snapshot`.)
  - `src/lib.rs` — module wiring for the new helper.
  - `tests/halcyon_part_v_p1_gql_dispatch.rs` — the red test exercising the full chain.
- **Green criterion (quoted from `§P-1` of the spec):**
  > The gauge executors do not need a bundle handle (they operate over `gauge_registry` + `lattice_registry`, which are process-global singletons today). So the dispatch is straight from `parser.rs::execute` through `exec_result_to_response` — no bundle resolution at all.
- **Receipt:**
  ```
  cargo test --no-default-features --lib
    test result: ok. 852 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  cargo test --features halcyon --lib -- --test-threads=1
    test result: ok. 965 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  cargo test --features halcyon --test halcyon_part_v_p1_gql_dispatch
    test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  cargo test --features halcyon --test halcyon_part_iii_http
    test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out  (no regression)
  cargo test --features halcyon --test halcyon_part_iv_http
    test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out  (no regression)
  ```
- **Commit:** `5b555ce`

### TDD-HAL-V.0b — `GAUGE_FIELD` declaration must populate both dyn + SU(2)-mut registries

- **Red test:** the same `tests/halcyon_part_v_p1_gql_dispatch.rs::test_p1_gql_dispatches_gauge_statements_end_to_end` extended so step (d) `GIBBS_SAMPLE` no longer manually pre-registers via `register_su2` and step (e) asserts the post-Gibbs `per_face` values are not all `1.0`. With the workaround removed, the previously-latent II.5-era bug surfaced as `HTTP 500 "source field U_p1 is not declared"` on the `GIBBS_SAMPLE` call.
- **Files:**
  - `src/parser.rs` — `Statement::GaugeField` executor arm now calls `register_su2` alongside the existing `register`. Additive, no behavior change on the read path.
  - `src/gauge/http.rs` — `POST /v1/gauge_field` handler matched: same dual-registry parking.
  - `tests/halcyon_part_v_p1_gql_dispatch.rs` — workaround removed, post-Gibbs assertions tightened (10-element `mean_plaquette` Vector column, at least one `< 1.0`, `per_face` not all identity).
- **Green criterion (latent II.5 gap, quoted from the commit body):**
  > Root cause: Statement::GaugeField executor arm in src/parser.rs only called gauge::registry::register (dyn map for reads) and never called register_su2 (SU(2)-mut map for mutators). Same gap in POST /v1/gauge_field handler in src/gauge/http.rs (gauge_field_create).
- **Receipt:**
  ```
  cargo test --features halcyon --test halcyon_part_v_p1_gql_dispatch
    test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  cargo test --no-default-features --lib
    test result: ok. 852 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out  (byte-identical)
  cargo test --features halcyon --lib --test-threads=1
    test result: ok. 965 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  cargo test --features halcyon --test halcyon_part_iv_http
    test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out  (no regression)
  cargo test --features halcyon --test halcyon_part_iii_http
    test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out  (no regression)
  cargo build --bin gigi-stream --features "kahler imagine sharded transactions patterns causal_states wish halcyon" --release
    clean
  ```
- **Commit:** `9c5b614`

### TDD-HAL-V.1 — `OP_GAUGE_FIELD_SNAPSHOT` (`0x0B`) WAL op + `GaugeFieldSnapshotPayload` + LE encoding

- **Red test:** `src/wal.rs::tests::tdd_hal_v_1_snapshot_payload_le_roundtrip` (plus the two sibling unit tests `tdd_hal_v_1_snapshot_size_buckyball` and `tdd_hal_v_1_snapshot_sha256_recomputed`)
- **Files:**
  - `src/wal.rs` — `const OP_GAUGE_FIELD_SNAPSHOT: u8 = 0x0B` with the LE encoding ratified at the op site per **D-V-A**. `GaugeFieldSnapshotPayload { name, group, buffer: Vec<f64>, sha256: [u8; 32] }` struct with explicit `to_le_bytes` / `from_le_bytes` writers + readers using `f64::to_le_bytes` and `f64::from_le_bytes`. `from_buffer(name, group, buffer)` constructor that mints the SHA-256 over the LE-encoded buffer bytes (the same bytes the replay path will see — that is the **D-V-C** citation handle). `compute_buffer_sha256(buffer: &[f64]) -> [u8; 32]` helper exposed for replay-path verification. `WalWriter::log_gauge_field_snapshot(payload)` writer with CRC32 + length-prefix framing matching ops `0x09` and `0x0A`. `WalEntry::GaugeFieldSnapshot(payload)` reader variant.
  - `src/engine.rs` — replay match extended for exhaustiveness; the variant is accepted but currently a no-op at this gate (V.3 wires it into the registry install path).
- **Green criterion (quoted from spec `§P0.1`):**
  > Row-major (n_edges, repr_dim) f64 bytes. For SU(2) on the buckyball: 90 * 4 * 8 = 2880 bytes. Native endianness is fine IF the WAL is reread on the same architecture; otherwise emit little-endian.

  Per **D-V-A** ratification 2026-06-19, Bee chose explicit LE unconditionally; the documentation is at the `OP_GAUGE_FIELD_SNAPSHOT` const site.
- **Receipt:**
  ```
  cargo test --features gauge --lib wal::tests::tdd_hal_v_1
    test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 965 filtered out
  cargo test --no-default-features --lib
    test result: ok. 852 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  cargo test --features gauge --lib -- --test-threads=1
    test result: ok. 968 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  cargo test --features halcyon --lib wal::tests::tdd_hal_v_1
    test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 965 filtered out
  ```
  The 2880-byte SU(2)-on-buckyball buffer round-trips byte-identical through `to_le_bytes` / `from_le_bytes`; total entry size (payload + 42-byte framing) = 2922 bytes for the `"U"` name. The `sha256_recomputed` test exercises **D-V-C**: the hash computed over `f64::to_le_bytes` of the LE-encoded buffer equals the hash recomputed from the round-tripped buffer at the reader.
- **Commit:** `5bd2291`

### TDD-HAL-V.2 — `SNAPSHOT GAUGE_FIELD U PERSIST;` GQL verb + executor + dispatch helper update

- **Red test:** `src/parser.rs::tests::tdd_hal_v_2_parse_snapshot_with_persist` (plus four siblings: `tdd_hal_v_2_parse_snapshot_bare_rejected`, `tdd_hal_v_2_executor_snapshot_succeeds`, `tdd_hal_v_2_executor_snapshot_undeclared_field`, `tdd_hal_v_2_executor_snapshot_sha256_deterministic`)
- **Files:**
  - `src/parser.rs` — `Statement::Snapshot { name, persist: bool }` variant + `parse_snapshot` method. Bare `SNAPSHOT GAUGE_FIELD U;` parse-errors pointing at `expected PERSIST | TRANSIENT` per **D-V-D**. `PERSIST` clause flips the `persist: true` field — the slot is here so TRANSIENT (spec `§6`, deferred) can flip it false without grammar surgery. Executor arm reads the gauge handle through `gauge::registry::get_su2_mut`, copies out the buffer, builds `GaugeFieldSnapshotPayload::from_buffer` (which mints the SHA-256 per **D-V-C**), then calls `engine.snapshot_gauge_field_durable`. Returns `ExecResult::Rows` with the single-row envelope from spec `§P0.3` carrying `field`, `n_edges`, `repr_dim`, `sha256` (lowercase hex via inline `hex_encode`), and `wal_offset`.
  - `src/engine.rs` — `pub struct SnapshotResponse { sha256: String, wal_offset: u64 }` + `pub fn snapshot_gauge_field_durable(&mut self, name: &str, group: Group, buffer: Vec<f64>) -> io::Result<SnapshotResponse>`. Mirrors `declare_gauge_field_durable` including `maybe_checkpoint` at the tail.
  - `src/halcyon_gql_dispatch.rs` — `Statement::Snapshot` added to the gauge-family match arm (13 → 14 variants). Routes the new statement through `/v1/gql` per **D-V-B** — no dedicated `POST /v1/gauge_field/{name}/snapshot` route; matches the D5 precedent that put `GIBBS_SAMPLE` / `SYMPLECTIC_FLOW` / `E_FIELD` declarer on `/v1/gql` only.
- **Green criterion (quoted from spec `§P0.2` + Bee's `§7` answer #2):**
  > `SNAPSHOT GAUGE_FIELD U;` (no clause) — write to the WAL, ack on response. This is the production path. ... I'd suggest `SNAPSHOT` follow the same precedent: only addressable via `/v1/gql`, no `POST /v1/gauge_field/{name}/snapshot` route.

  Per **D-V-D** ratification 2026-06-19, Bee flipped the default: `PERSIST` is REQUIRED, not the default. Bare `SNAPSHOT GAUGE_FIELD U;` parse-errors pointing at the expected `PERSIST | TRANSIENT` token so every existing caller is already explicit when TRANSIENT lands.
- **Receipt:**
  ```
  cargo test --features halcyon --lib parser::tests::tdd_hal_v_2
    test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 968 filtered out
  cargo test --features halcyon --lib parser::
    test result: ok. 165 passed; 0 failed; 0 ignored; 0 measured; 808 filtered out
  cargo test --features halcyon --lib
    test result: ok. 973 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  cargo test --no-default-features --lib
    test result: ok. 852 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  ```
  Parser test count 160 → 165, full halcyon lib count 968 → 973. Optionality contract held byte-identical at 852/0 — `SnapshotResponse`, `hex_encode`, `snapshot_gauge_field_durable`, `Statement::Snapshot`, `parse_snapshot`, the SNAPSHOT dispatch arm, and the executor arm are all `#[cfg(feature = "gauge")]`-gated.
- **Commit:** `e90839f`

### TDD-HAL-V.3 — Replay restoration + `WalError::OrphanedSnapshot` + `WalError::SnapshotGroupMismatch` + `WalError::SnapshotChecksumMismatch` + `replace_buffer`

- **Red test:** `src/engine.rs::tests::tdd_hal_v_3_replay_snapshot_byte_identity`, `src/engine.rs::tests::tdd_hal_v_3_replay_orphan_snapshot`, `src/engine.rs::tests::tdd_hal_v_3_replay_group_mismatch`, `src/engine.rs::tests::tdd_hal_v_3_replay_checksum_mismatch`
- **Files:**
  - `src/wal.rs` — `WalError` enum with three variants gated on `feature = "gauge"`: `OrphanedSnapshot(String)`, `SnapshotGroupMismatch { name, expected, found }`, `SnapshotChecksumMismatch { name }`. `Display` impl names the field + category so the integration-test crate (V.5) can substring-match on the typed-error surface. `From<WalError> for io::Error` keeps the existing `replay_gauge_substrate -> io::Result<()>` surface intact.
  - `src/gauge/su2_gauge_field.rs` — `SU2GaugeField::replace_buffer(&mut self, Vec<f64>) -> Result<(), GaugeFieldError>`. Validates `new_buffer.len() == n_edges * repr_dim`; `BufferShapeMismatch` on disagreement; in-place overwrite of `self.buffer.data` otherwise. Idempotent (last write wins, matching the spec `§P1` contract).
  - `src/engine.rs` — `replay_gauge_substrate` extended with:
    - Pass 2 modification: SU(2) `GAUGE_FIELD` declarations also call `register_su2`, mirroring the V.0b parser-side fix so post-restart `get_su2_mut(name)` finds the field.
    - Pass 3 (new): walks every `WalEntry::GaugeFieldSnapshot` — `registry::get(name)` for orphan check (`WalError::OrphanedSnapshot`); `handle.group()` vs `payload.group` for group check (`WalError::SnapshotGroupMismatch`); `compute_buffer_sha256(&payload.buffer)` vs `payload.sha256` for checksum check (`WalError::SnapshotChecksumMismatch`); then locks the SU(2)-mut `Arc<Mutex<…>>`, calls `replace_buffer`, and `republish_su2`.
- **Green criterion (quoted from spec `§P1`):**
  > `replace_buffer` is the new internal method on the dyn surface. For `SU2GaugeField` it's a single buffer copy. The replay is idempotent — if multiple `0x0B` ops exist for the same field, the last one wins (the WAL is the source of truth; latest write is current state).
- **Receipt:**
  ```
  cargo test --no-default-features --lib
    test result: ok. 852 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  cargo test --features halcyon --lib engine::tests::tdd_hal_v_3 -- --test-threads=1
    test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 973 filtered out
  cargo test --features halcyon --lib tdd_hal_v_ -- --test-threads=1
    test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 965 filtered out
  cargo test --features halcyon --lib -- --test-threads=1
    test result: ok. 977 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  cargo test --features halcyon --test halcyon_part_ii_persistence -- --test-threads=1
    test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  cargo test --features halcyon --test halcyon_part_v_p1_gql_dispatch -- --test-threads=1
    test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  cargo test --features halcyon --test halcyon_part_iv_http -- --test-threads=1
    test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  ```
  All four tests hold `gauge::registry::test_serial_lock()` to avoid races with parallel gauge tests on the process-global registries. Convention for the halcyon lib suite is `--test-threads=1` (matches V.0b and V.1). ChEMBL-incident durability gates unaffected — Part V adds a new WAL op but does not touch the snapshot-survives-WAL-compact or restart-survival paths; `halcyon_part_ii_persistence` integration tests still 4/0.
- **Commit:** `6be010b`

### TDD-HAL-V.4 — `tdd_hal_v_1_snapshot_writes_and_replays` smoke gate (the load-bearing one)

- **Red test:** `tests/halcyon_part_v_snapshot.rs::tdd_hal_v_1_snapshot_writes_and_replays`
- **Files:**
  - `src/parser.rs` — `Statement::LatticeFromCanonical` executor arm (under `cfg(feature = "gauge")`) now routes through `engine.declare_lattice_durable` instead of just `crate::lattice::registry`. The red test exposed a real architectural bug: declaring a `LATTICE FROM TRUNCATED_ICOSAHEDRON` only wrote to the in-memory lattice registry, so the WAL never knew about the buckyball lattice. On reopen, the `GAUGE_FIELD PERSIST` WAL entry orphaned with `"WAL GaugeFieldDeclare references unknown lattice 'buckyball'"`. The minimal green fix: route through `declare_lattice_durable`. Optionality contract preserved via the `cfg` flip — `no-default-features` still hits the in-memory branch.
  - `tests/halcyon_part_v_snapshot.rs` — new integration test exactly per the spec `§P2.1` sketch: full chain through `parser::execute` (LATTICE FROM TRUNCATED_ICOSAHEDRON → GAUGE_FIELD … PERSIST → GIBBS_SAMPLE → SNAPSHOT GAUGE_FIELD U PERSIST → drop engine → `Engine::open` same data dir → byte-identical `Vec<f64>` + SHA-256 stable). Re-derives the SHA-256 over LE buffer bytes from outside the WAL module (via the `sha2` crate already in `[dependencies]`) to assert the **D-V-C** citation contract end-to-end.
- **Green criterion (quoted from spec `§P2.1`):**
  > Declare → `INIT IDENTITY` → `GIBBS_SAMPLE` thermalization → `SNAPSHOT` → close engine → reopen → read buffer → assert byte-identical to pre-close buffer.
- **Receipt:**
  ```
  cargo test --no-default-features --lib
    test result: ok. 852 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  cargo test --features halcyon --test halcyon_part_v_snapshot tdd_hal_v_1_snapshot_writes_and_replays
    test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  cargo test --features halcyon --lib -- --test-threads=1
    test result: ok. 977 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  cargo test --features halcyon --test halcyon_part_v_p1_gql_dispatch
    test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  cargo test --features halcyon --test halcyon_part_iii_gold
    test result: ok. 4 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out
  cargo test --features halcyon --test halcyon_part_iv_gold
    test result: ok. 4 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out
  ```
  The `halcyon_gql_dispatch.rs` helper already lists `Statement::Snapshot` in its variant list (from V.2 commit `e90839f`); no change required there.
- **Commit:** `5706bcc`

### TDD-HAL-V.5 — `tdd_hal_v_2_snapshot_checksum_rejection` + `tdd_hal_v_3_snapshot_orphan_rejection` failure-mode gates

- **Red test:** `tests/halcyon_part_v_snapshot.rs::tdd_hal_v_2_snapshot_checksum_rejection`, `tests/halcyon_part_v_snapshot.rs::tdd_hal_v_3_snapshot_orphan_rejection`
- **Files:**
  - `tests/halcyon_part_v_snapshot.rs` — two new integration tests. No production changes required at this gate — V.3 already wired `WalError::SnapshotChecksumMismatch` + `WalError::OrphanedSnapshot` through `From<WalError> for io::Error`. V.5's job is to lock the same matrix at the integration-test boundary so the typed-error surface survives the lib → integration-test crate split.

  Both tests drive the V.4 four-statement block via `parser::execute` against a temp data dir, close the engine, surgically rewrite the on-disk WAL bytes, then re-open and match `err.to_string()` on the `Display` impl (`'U'` + `SHA-256` for checksum, `'U'` + `orphan` for orphan) — the integration-level rejection surface. For the orphan gate, the cleanest exercise of `WalError::OrphanedSnapshot` is to splice out the entire `OP_GAUGE_FIELD_DECLARE` entry `[length_prefix .. crc_tail)` — the remaining `LATTICE` + `GIBBS_SAMPLE` residue + `SNAPSHOT` entries replay against a registry that never received the gauge declare, which is exactly the orphan-pass scenario. Zeroing only the op byte would surface `"Unknown WAL op"` from the reader BEFORE the snapshot pass runs, which tests the wrong surface.
- **Green criterion (quoted from spec `§P2.2` + `§P2.3`):**
  > Manually corrupt the SHA-256 in a WAL entry. Replay must reject with `WalError::SnapshotChecksumMismatch`.
  >
  > Manually delete the `OP_GAUGE_FIELD_DECLARE` entry preceding an `OP_GAUGE_FIELD_SNAPSHOT`. Replay must reject with `WalError::OrphanedSnapshot`.
- **Receipt:**
  ```
  cargo test --features halcyon --test halcyon_part_v_snapshot -- --test-threads=1
  running 3 tests
  test tdd_hal_v_1_snapshot_writes_and_replays ... ok
  test tdd_hal_v_2_snapshot_checksum_rejection ... ok
  test tdd_hal_v_3_snapshot_orphan_rejection ... ok
  test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

  cargo test --no-default-features --lib
    test result: ok. 852 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  ```
  Group-mismatch coverage at the integration level was considered and dropped: V.3's in-process unit test `tdd_hal_v_3_replay_group_mismatch` in `src/engine.rs` already covers it, and the V.5 spec `§P2` matrix doesn't list it as required.
- **Commit:** `1165698`

---

## Closing receipts

- **`cargo test --no-default-features --lib` still `852 passed; 0 failed`, byte-identical** across every gate in the sprint. Every Part V surface — `OP_GAUGE_FIELD_SNAPSHOT`, `GaugeFieldSnapshotPayload`, `WalEntry::GaugeFieldSnapshot`, `WalError::{OrphanedSnapshot, SnapshotGroupMismatch, SnapshotChecksumMismatch}`, `SU2GaugeField::replace_buffer`, `engine::snapshot_gauge_field_durable`, `engine::SnapshotResponse`, `Statement::Snapshot { name, persist }`, `parser::parse_snapshot`, the SNAPSHOT executor arm, the `halcyon_gql_dispatch.rs` helper module, and the `cfg`-flipped `LatticeFromCanonical` durable-route — is `#[cfg(feature = "gauge")]`-gated.
- **`cargo test --features halcyon --lib -- --test-threads=1` = `977 passed; 0 failed`.** Per-gate test deltas summed: V.1 added 3 (965 → 968), V.2 added 5 (968 → 973), V.3 added 4 (973 → 977). V.4 + V.5 land in the integration-test crate, not the lib. `--test-threads=1` is convention for the halcyon lib suite (matches V.0b, V.1, V.3).
- **`cargo test --features halcyon --test halcyon_part_v_snapshot -- --test-threads=1` = `3 passed; 0 failed`** — V.4 smoke gate + V.5 two failure-mode rejection gates.
- **`cargo test --features halcyon --test halcyon_part_v_p1_gql_dispatch` = `1 passed; 0 failed`** — the P-1 production-verification gate from V.0/V.0b still green.
- **ChEMBL-incident durability gates green:**
  ```
  test engine::tests::snapshot_survives_wal_compact ... ok
  test engine::tests::streaming_wal_replay_correct_count ... ok
  test engine::tests::streaming_snapshot_roundtrip ... ok
  test engine::tests::cow_snapshot_roundtrip ... ok
  test engine::tests::mmap_rebase_snapshot_roundtrip ... ok
  test engine::tests::test_9_8_trigger_survives_restart ... ok
  ```
  Part V adds a new WAL op but does not touch the snapshot-survives-WAL-compact or restart-survival paths.
- **No `Co-Authored-By: Claude` footer on any commit.** Every commit in this sprint (`5b555ce`, `9c5b614`, `5bd2291`, `e90839f`, `6be010b`, `5706bcc`, `1165698`, plus this log commit) is authored solely by `nurdymuny <bee_davis@alumni.brown.edu>` (Bee Rosa Davis) per the `feedback_no_ai_coauthor.md` standing memo.

---

## Locked decisions inherited (D-V-A through D-V-D, ratified 2026-06-19)

- **D-V-A — WAL op encoding = explicit little-endian.** Use `f64::to_le_bytes` when writing, `f64::from_le_bytes` when reading. Documented at the `OP_GAUGE_FIELD_SNAPSHOT` const site in `src/wal.rs` (V.1).
- **D-V-B — HTTP surface = `/v1/gql` only.** No dedicated `POST /v1/gauge_field/{name}/snapshot` route. `SNAPSHOT` is a mutator and matches the D5 precedent that put `GIBBS_SAMPLE` / `SYMPLECTIC_FLOW` / `E_FIELD` declarer on `/v1/gql` only. Wired through `halcyon_gql_dispatch.rs` (V.2).
- **D-V-C — SHA-256 over the buffer bytes (LE) is the canonical citation handle.** The same SHA-256 lands in the WAL entry AND the Rows envelope returned to the caller. Computed by `compute_buffer_sha256` over the same LE bytes the replay path will see; re-derived from outside the WAL module by V.4's smoke gate (V.1 + V.2 + V.4).
- **D-V-D — `PERSIST` clause REQUIRED, not default.** Bare `SNAPSHOT GAUGE_FIELD U;` parse-errors pointing at `expected PERSIST | TRANSIENT`. When `TRANSIENT` ships later (out of scope per spec `§6`), every existing caller is already explicit; zero behavior drift (V.2).

---

## What is deferred

Mirroring spec `§6`, the following are deliberately out of Part V scope:

- **`TRANSIENT` clause** — in-memory snapshot for fast `RESTORE`, no WAL write. The `Statement::Snapshot.persist: bool` slot is here so TRANSIENT can flip it without grammar surgery. Spec `§6`.
- **`SNAPSHOT E_FIELD`** — same shape will be wanted for symplectic-flow trajectory caching, but defer to a follow-up sprint. Halcyon's current public-verifier doesn't need it; Part V ships gauge-field snapshots first. Spec `§6`.
- **`OP_GIBBS_SAMPLE` statement-replay alternative** — WAL-log the GQL statement and re-thermalize on replay. Tracked so the choice is named; not requested because (a) replay cost grows linearly with thermalization length, (b) snapshot is the cleaner architectural fit with declarations-only persistence. Spec `§6`.
- **`RESTORE GAUGE_FIELD U FROM SNAPSHOT <sha>;` verb** — powerful and almost trivial once Part V ships (the buffer is already addressable by SHA in the WAL), but not on the critical path. Spec `§6`.
- **Multi-engine federation of snapshots** (snapshot on one node, replay on another). Not relevant; `gigi-stream` is a single-engine deploy today. Spec `§6`.
- **Compression of the snapshot buffer** — for SU(2) on the buckyball it's 2,880 bytes, irrelevant. If larger lattices land later (10⁴ or 10⁵ edges) it becomes a real concern; orthogonal to this sprint. Spec `§6`.

---

## Three answers back to Halcyon (closing `§7` of the gates doc)

1. **Endianness — explicit little-endian.** `f64::to_le_bytes` on write, `f64::from_le_bytes` on read. Documented at the `OP_GAUGE_FIELD_SNAPSHOT` const site in `src/wal.rs`. Fly.io's `gigi-stream` runs x86_64 today so native would work, but explicit LE is the future-proofing the gates doc suggested and matches the D-V-A ratification.
2. **HTTP surface — `/v1/gql` only.** No dedicated `POST /v1/gauge_field/{name}/snapshot` route. Matches the D5 precedent from Part III (`GIBBS_SAMPLE` is `/v1/gql`-only) and Part IV (`SYMPLECTIC_FLOW` is embedded-only). The `halcyon_gql_dispatch.rs` helper module names `Statement::Snapshot` in its variant list so the wire and the executor stay in sync.
3. **Citation SHA — snapshot buffer SHA-256 (LE-encoded buffer bytes).** The same hash lands in three places: the WAL entry (V.1's `GaugeFieldSnapshotPayload.sha256` field), the Rows envelope returned by `SNAPSHOT GAUGE_FIELD U PERSIST` (V.2's executor, lowercase hex via the inline `hex_encode` helper), and Solves Vol. 4 Appendix A.4 (the chapter's citation handle of record). V.4's smoke gate re-derives it from outside the WAL module via the `sha2` crate to assert the contract end-to-end.

---

## Bee pushback ratified

The gates doc `§3 P0.2` had `PERSIST` listed as the default with the bare `SNAPSHOT GAUGE_FIELD U;` form treated as production. Bee flipped this on 2026-06-19: **`PERSIST` is REQUIRED**. Bare `SNAPSHOT GAUGE_FIELD U;` parse-errors pointing at `expected PERSIST | TRANSIENT`.

The rationale is the same one the gates doc `§7` flagged ("If the `PERSIST` clause should be required rather than default, that's also a real call"): when `TRANSIENT` ships in a follow-up sprint, every existing caller in the world will already be explicit. Zero behavior drift between Part V launch and the day `TRANSIENT` lands. The `Statement::Snapshot.persist: bool` slot in V.2 is the grammar shape that lets `TRANSIENT` flip it to `false` without touching the parser surface again.

This is the D-V-D decision recorded above. V.2's parser tests (`tdd_hal_v_2_parse_snapshot_with_persist`, `tdd_hal_v_2_parse_snapshot_bare_rejected`) are the regression locks.

---

## Targets unblocked

Mirroring `§8` of the gates doc:

- **`papers/solves_vol4_ym_mass_gap.tex` Appendix A.4 — the public-receipt verifier promotes from 30-second thermalization to sub-100ms cached read** against the production substrate. After `verify_canonical_receipt.py --snapshot` runs once against `gigi-stream.fly.dev`, the canonical thermalized buffer lives in the production WAL. Subsequent verification calls skip thermalization and just read the cached buffer via the existing `GET /v1/gauge_field/halcyon_canonical_U/plaquette?reduction=mean` route.
- **Citation handle = snapshot SHA-256.** The chapter currently cites the Halcyon JSON's commit hash + the GIGI deploy hash. With Part V, the canonical adds a third hash — the SHA-256 of the thermalized buffer itself, computed over the LE-encoded f64 bytes per **D-V-C**. That's a state-level fingerprint independent of code commits or deploys, and is the strongest publicly-citable receipt the architecture can offer.
- **Marcella's gauge-corpus reader gets a stable corpus.** Already named as read-only in the Part II audit. With persistent thermalized fields, her geometric channel can query a stable corpus instead of triggering thermalization on every read. Off-critical-path but real, and the SHA-256 citation handle means the corpus has a verifiable identity that survives engine restarts.

---

## 2026-06-26 amendment — orphan snapshot policy: hard-reject → graceful-skip

**Commit:** `d592313` — `gigi(durability): orphan gauge snapshot → skip + warn instead of hard-fail`.

**Why this is a Part V amendment:** TDD-HAL-V.3 (gate above, original spec) ratified three replay-time rejection paths — `OrphanedSnapshot`, `SnapshotGroupMismatch`, `SnapshotChecksumMismatch`. The original semantic was *all three hard-reject the entire `Engine::open` call*. After today's production incident the orphan branch was flipped to a graceful skip (warn + return `Ok(())` on the missing-handle case, leave the orphan field unregistered). The other two corruption checks (group mismatch, SHA-256 checksum mismatch) remain hard-reject — those genuinely indicate WAL byte corruption or operator error.

**Trigger (2026-06-26 incident chain on `gigi-stream.fly.dev`):**

1. Boot snapshot writer wedged on `marcella_source_embeddings_bge_v2` (~30 min hang). 3c9047d's `spawn_blocking` hardening covered the `POST /v1/admin/snapshot` path; the boot path at `src/bin/gigi_stream.rs:15237` called the no-timeout sibling `engine.snapshot()` instead of `snapshot_with_report()`, so the boot wedged indefinitely.
2. `91dced1` shipped a `GIGI_SKIP_BOOT_SNAPSHOT=1` env-var escape valve + switched the boot path to `snapshot_with_report()` + added stale-`.tmp` cleanup. Production booted on heap (~15GB RSS).
3. Heap-mode RSS + Marcella `IMAGINE` Phase 2 traffic OOM-killed the machine at ~14 min uptime (`exit_code 137`).
4. The wedged-then-OOM-killed prior boot left an orphan `OP_GAUGE_FIELD_SNAPSHOT` for field `U_v` in the gauge WAL — a snapshot entry whose preceding `OP_GAUGE_FIELD_DECLARE` was never flushed.
5. The auto-restart's fast-mmap path (`Engine::open_mmap`) replayed the WAL, hit the orphan, returned `WalError::OrphanedSnapshot("U_v")`, fell back to *full heap replay* (`src/bin/gigi_stream.rs:15209`), and would have OOM-killed again on the same 14-min cycle.

**Architectural read:** the original V.3 design treated *any* missing-handle case as corruption. Today's incident proved that **an orphan can also be an availability hazard arising from legitimate WAL truncation** (Codex's WAL durability prefix-preservation does not preserve the order-invariant `DECLARE → SNAPSHOT` pairing when the tail truncates between them). Refusing the install is still correct (no silent corruption), but cascading to heap replay was the wrong recovery — the orphan's bytes are *not* installed either way; the question is whether the rest of the WAL can be processed.

**Change applied in `d592313`:**

```rust
// src/engine.rs — gauge snapshot replay (Pass 3), inside the replay closure
let handle = match crate::gauge::registry::get(&payload.name) {
    Some(h) => h,
    None => {
        eprintln!(
            "WARNING: gauge field WAL has snapshot for '{}' \
             with no preceding OP_GAUGE_FIELD_DECLARE — \
             skipping orphan snapshot (field unavailable). \
             Boot continues on mmap fast path.",
            payload.name
        );
        return Ok(());
    }
};
// group/sha checks unchanged — those stay hard-reject
```

**Test update — `tdd_hal_v_3_replay_orphan_snapshot`:** revised to assert the new graceful semantic. The test now hand-builds a WAL with one `OP_GAUGE_FIELD_SNAPSHOT` for `U_orphan` and no declare, calls `Engine::open(&dir)`, and asserts:
- `Engine::open` succeeds (`expect("orphan snapshot must be SKIPPED, not hard-error")`)
- `gauge::registry::get("U_orphan").is_none()` (orphan field stays unregistered after the skip)

**Sibling V.3 tests verified intact at the same commit:**

```
running 4 tests
test engine::tests::tdd_hal_v_3_replay_checksum_mismatch ... ok
test engine::tests::tdd_hal_v_3_replay_snapshot_byte_identity ... ok
test engine::tests::tdd_hal_v_3_replay_orphan_snapshot ... ok       (← revised)
test engine::tests::tdd_hal_v_3_replay_group_mismatch ... ok
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 1034 filtered out
```

**Recovery primitive sketched (not yet implemented):** `POST /v1/admin/gauge/repair` — given a field name, write a synthetic `OP_GAUGE_FIELD_DECLARE` matching the orphan snapshot's group tag, so the next replay can install the buffer. Captured here as a follow-up; the orphan-skip alone restores boot availability, and the orphan field's bytes are still in the WAL when the repair primitive lands.

**Two related commits shipped in the same incident response (named here so the V.3 history is complete):**

- `91dced1` — boot snapshot escape valve + timeout-aware default (`engine.snapshot()` → `engine.snapshot_with_report()`) + stale-`.tmp` cleanup at `src/bin/gigi_stream.rs:15234-15290`. Does not modify a Part V gate but is the precondition that revealed the orphan path (without it, production never reached the second restart).
- `a190a72` — IMAGINE coherence Phase 2 (n-D integrator + tame-metric fallback). The Marcella endpoint that drove the load that exposed the OOM cascade. Independent of Part V but named here because the incident chain is otherwise hard to read.

**Production receipt at the closing commit `d592313`:** `flyctl status` reports `1 total, 1 passing` on `gigi-stream.fly.dev` v228 (image `gigi-stream:deployment-01KW2DMMGB10V4A73TT3WYXD02`) — first clean fly-health-check pass of the day. Fast-mmap path completes within fly's smoke-check window. RSS stays at ~200MB (mmap) instead of the ~15GB heap-mode footprint that OOM-killed v226 and v227.
