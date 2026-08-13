# TDD-DUR — Closing the WAL-truncation data-loss class

**Status:** implementation spec. Analysis was read-only; nothing in the tree was edited.
**Ground truth:** every line reference below was re-read today against `main` @ `4fcca32`.

---

## 1. THE INVARIANT

> **INV-D.** The WAL file may be replaced by a shorter one only at an instant when, for every bundle the engine knows about, the state reconstructible from disk *alone* after the replacement equals the state the engine is serving from RAM — and the same is true of every non-bundle WAL op kind the boot path replays into live state.

Equality, not containment. Bee's candidate wording ("every byte the WAL is about to drop must already be durable in a `.dhoom`") is correct for the *erasure* mode and blind to the *resurrection* mode: a dropped `Delete` op has no bytes in any `.dhoom` by construction, so it satisfies containment while still corrupting the database. It is also blind to the case where the data is already absent from RAM (a partial boot view) and present in a stale file — there, nothing is "only in the WAL", and truncation still destroys the database.

### The exact predicate

Evaluated at the instant immediately before `fs::rename(gigi.wal.tmp → gigi.wal)`. That rename is the *only* physical truncation primitive in the tree; it appears at exactly three sites — `engine.rs:2193` (`Engine::compact`), `engine.rs:2753` (`compact_wal_to_schemas`), `engine.rs:2997` (the hand-copied duplicate inside `snapshot_with_chunk_size`). `WalWriter::open` is create+append only (`wal.rs:427-428`), nothing calls `set_len`, and no `fs::remove_file` targets the WAL. One choke point, physically.

Let

- `K = self.schemas.keys() ∪ self.bundles.keys() ∪ self.mmap_bundles.keys()` — the **union**. Every existing snapshot loop takes a subset of this and that is the bug.
- `live(B)` = the visible record multiset the engine currently serves for `B`.
- `durable(B)` = `decode(snapshots/B.dhoom)` if present, else `∅`. **Not** `.prev` — boot deliberately refuses `.prev` when `.dhoom` is absent (`engine.rs:759-770`), so `.prev` is not durable state for this purpose.
- `newwal(B)` = ops for `B` in the WAL about to be installed. `compact_wal_to_schemas` emits `CreateBundle` / `CreateTrigger` / `LatticeDeclare` / `GaugeFieldDeclare` / `Checkpoint` and nothing else (`engine.rs:2711-2750`), so **`newwal(B) = ∅` always**. After compaction the `.dhoom` is the sole durable form. Say that out loud; it is the whole reason the margin for error is zero.

Then compaction is permitted iff all five clauses hold:

| | clause | why |
|---|---|---|
| **P1** REACHABILITY | `∀B ∈ K : B ∈ self.schemas` | boot's Phase 2 is schema-driven (`engine.rs:681`); a `.dhoom` with no `CreateBundle` in the new WAL is never opened. Durable bytes + no schema ≡ loss. |
| **P2** COVERAGE | `∀B ∈ K : durable(B) ≡ live(B)`, established by **reading back the file just written** — `MmapBundle::open(snap_path)?.len() == |live(B)|` | `rotate_snapshot` (`engine.rs:2271-2320`) never re-reads what it wrote; a 0-byte or truncated `.dhoom` passes today. |
| **P3** NON-REGRESSION | `∀B` written this cycle: `new_count + accounted_deletions ≥ old_count`, where `old_count` is read from the `.dhoom` about to be overwritten | catches a *partial RAM view* writing a small-but-valid snapshot over a good one. See H2. |
| **P4** NON-BUNDLE STATE | every `WalEntry` variant the boot path installs into live state is either re-emitted by the compactor or absent from the WAL being dropped | `GaugeFieldSnapshot` is live in prod and satisfies neither. See H3. |
| **P5** COMPLETE READ | the WAL being replaced was replayed to EOF — no CRC prefix truncation | `finish_wal_replay_prefix` (`engine.rs:584-590`) converts a CRC mismatch *anywhere* in the file into `Ok(())` and silently discards the rest. If the logical WAL is a strict prefix of the physical WAL, `self.schemas` is not the true schema set and P1 is unprovable. |

P3 has an O(1) exact shortcut for mmap bundles: `ob.overlay_len() == 0 && ob.tombstone_len() == 0` (`mmap_bundle.rs:352-359`) proves RAM ≡ base, because for an overlay bundle the overlay and the tombstone set **are** the entire delta between RAM and the file. Empty both ⇒ nothing to lose. This is derived from storage state that already exists, so it cannot drift when someone adds a mutation path — which matters concretely, because `truncate_bundle` (`gigi_stream.rs:10858-10879`) and `ttl_eviction_task` (`gigi_stream.rs:16441-16445`) mutate bundles with **no WAL entry at all**. An LSN-based proof would call those bundles clean and be wrong. An overlay/tombstone-derived proof sees them.

**Operational corollary, worth writing on the wall:** an oversized WAL is never an emergency. A WAL that shrank without a verified snapshot behind it always is. Every guard fails toward keeping the WAL.

---

## 2. THE FIX

### 2.1 Verdict on type-state: yes, affordable — with two qualifications

Take it. The cost is genuinely small because the target surface is small: three truncation sites collapse to one, five `.dhoom` writers collapse to one enumerator, and the compile error lands on exactly five call sites (`engine.rs:2492`, `2789`, `2820`, `3178`, plus the deleted inline copy). Converting "did you remember `self.mmap_bundles`?" from a code-review question into a build failure is precisely what "close the class, not the instance" means, and nothing else on the table does it.

Two qualifications that must be honored or the guarantee is theatre:

1. **The token must live in a new module.** Rust privacy is module-scoped and `engine.rs` is ~6,900 lines. A `CheckpointReceipt` defined inside `engine.rs` can be constructed by any of the buggy paths in `engine.rs`. Put it in `src/durability.rs` with private fields, no `pub fn new`, no `Default`, no `Clone`.
2. **The token is a *claim*; `verify` is the *check*.** Even with the module boundary, someone can call `record_written(name)` next to a write for a different bundle. So `compact_wal_to_schemas` recomputes coverage against `self.bundles`, `self.mmap_bundles`, `self.schemas` and the actual `snapshots/` directory, and re-stats every claimed file. **The type system routes; the filesystem adjudicates.**

What type-state does *not* buy: it guards the WAL, and the adversarial pass proved the data can die in the `.dhoom` before the WAL guard ever runs. Hence two gates, not one.

### 2.2 Chosen design — three gates plus a backstop

```
GATE A   rotate_snapshot(..., proof: &NonRegression)      — you may not overwrite a generation
GATE B   compact_wal_to_schemas(receipt: CheckpointReceipt) — you may not shorten the WAL
GATE C   Engine::open_mmap quarantine                      — you may not serve a bundle you couldn't read
BACKSTOP WAL generation retention                          — you may not unlink the old WAL
```

Grafts: the unforgeable token + single enumerator + delete-the-duplicates come from *CheckpointReceipt*; the O(1) clean-clause, the 409 body and the `/v1/health` surface come from *Coverage Certificate*; **retention** comes from the *segmented-WAL* design — I am taking its retention idea and rejecting its segmentation, because its retirement floor quantifies over the same non-durable schema set that H4 breaks, at a much larger change cost.

### 2.3 New file: `src/durability.rs` (~220 lines)

```rust
pub struct CheckpointReceipt {           // private fields; no pub ctor, no Clone, no Default
    cycle_start: SystemTime,
    covered:  BTreeMap<String, u64>,     // bundle -> verified record count (read back)
    catalog_written: bool,
    replay_was_complete: bool,
}

pub struct NonRegression { bundle: String, old_count: u64, new_count: u64, accounted_deletions: u64 }

pub(crate) struct DurabilityLedger { /* accumulates */ }
impl DurabilityLedger {
    pub(crate) fn open_cycle() -> Self;
    pub(crate) fn record_written(&mut self, name: &str, verified_count: u64);
    pub(crate) fn record_catalog(&mut self);
    pub(crate) fn seal(
        self,
        known:   &BTreeSet<String>,        // K, computed by the caller from all three maps
        clean:   &BTreeMap<String, u64>,   // O(1)-clean bundles and their base counts
        replay_complete: bool,
    ) -> Result<CheckpointReceipt, CoverageGap>;
}

pub enum UncoveredReason {
    NeverWritten { live: u64 },
    DirtyOverlay { overlay: usize, tombstones: usize },
    ReadBackMismatch { expected: u64, found: u64 },
    EmptyButStaleDhoom { dhoom_bytes: u64 },
    OrphanDhoom,                    // file on disk, no schema
    UnclassifiedWalOp { op: &'static str },
    ReplayIncomplete,               // P5
    CatalogNotWritten,
}

// Exhaustive classifier — adding a WalEntry variant is a COMPILE ERROR until classified.
pub enum ReplayClass { RebuiltByCompactor, CoveredByDhoom, Diagnostic }
pub fn classify(e: &crate::wal::WalEntry) -> ReplayClass;   // exhaustive match, no `_ =>` arm
```

`CheckpointReceipt::check_against(&self, live_known, live_clean) -> Result<(), CoverageGap>` is what the truncator calls.

### 2.4 Signature changes in `src/engine.rs`

| site | today | becomes |
|---|---|---|
| `engine.rs:2706` | `pub fn compact_wal_to_schemas(&mut self)` | `fn compact_wal_to_schemas(&mut self, receipt: CheckpointReceipt) -> io::Result<()>` — **by value** (cannot be reused for a second compaction), no longer `pub`. First statement is `receipt.check_against(...)?`, placed **before** `WalWriter::open(&tmp_path)` at `engine.rs:2710`, so a refusal leaves not even a stray `gigi.wal.tmp`. |
| `engine.rs:2271` | `fn rotate_snapshot(snapshots_dir, snap_path, tmp_path)` | `fn rotate_snapshot(snapshots_dir, snap_path, tmp_path, proof: &NonRegression) -> io::Result<u64>` — returns the read-back count. |
| **NEW** | — | `pub fn durable_checkpoint(&mut self, chunk_size: usize, budget: Option<u64>) -> io::Result<SnapshotReport>` — the only receipt producer. |
| `engine.rs:2173-2200` | `Engine::compact()` rebuilds the WAL from `self.schemas` + `self.bundles`, writes no `.dhoom`, renames at 2193 | body replaced by `self.durable_checkpoint(...)`. Keep the name for `edge.rs:446` → `gigi_edge.rs:782` and `gigi_stress.rs:1005`. It can never mint a receipt honestly, so it must not reach the choke point. |
| `engine.rs:2955-3002` | hand-copied inline WAL rewrite | **delete**. Its own comment (`engine.rs:2947-2954`) admits the duplication is a drift hazard; leaving a second `fs::rename(tmp, wal)` outside the choke point defeats the design. |
| `engine.rs:2834` `snapshot_with_chunk_size` | heap-only loop + inline truncator | `Ok(self.durable_checkpoint(chunk_size, self.compaction_policy.per_bundle_timeout_secs)?.total_records_written)` — keeps `engine.rs:3648/3711/…`, `tests/snapshot_rotation.rs:67/84/…`, `tests/encoder_high_dim_smoke.rs:109/156/198` compiling unchanged. |
| `engine.rs:2327` `snapshot_with_chunk_size_report` | the incident path | same reduction. |
| `engine.rs:3036-3041` `maybe_auto_compact` | `if self.mmap_bundles.is_empty() { cow_snapshot } else { mmap_rebase_snapshot }` | `self.durable_checkpoint(...)`. This branch is the **only** code in the tree that knows the two maps need different treatment, and it is reachable from exactly one caller. Delete the asymmetry rather than replicating it. |
| `engine.rs:229-234` `SnapshotReport` | `bundles`, `total_records_written`, `timed_out_bundles` | add `bundles_known: usize`, `bundles_covered: usize`, `uncovered: Vec<(String, UncoveredReason)>`, `wal_compacted: bool`, `wal_generation_retained: Option<String>`. Today the struct is *structurally incapable* of reporting the 2026-08-12 failure: `timed_out_bundles` is only ever pushed at `engine.rs:2466`, inside `for (name, store) in &self.bundles`, so a bundle the loop never visited can never appear in it. |

### 2.5 `durable_checkpoint` — the one enumerator

```rust
pub fn durable_checkpoint(&mut self, chunk_size: usize, budget: Option<u64>)
    -> io::Result<SnapshotReport>
{
    let mut ledger = DurabilityLedger::open_cycle();
    let known: BTreeSet<String> = self.schemas.keys()
        .chain(self.bundles.keys())
        .chain(self.mmap_bundles.keys())
        .cloned().collect();
    // 1. catalog first — see H4
    self.write_catalog(&known)?;  ledger.record_catalog();
    // 2. one loop, dispatching per target kind
    for name in &known {
        if let Some(base_n) = self.o1_clean(name) { ledger.record_written(name, base_n); continue; }
        let (tmp, snap) = paths(name);
        let live_n = self.encode_target(name, &tmp, chunk_size, budget)?;   // streaming, honours budget
        let proof  = NonRegression::build(&snap, live_n, self.deletions_this_cycle(name));
        let verified = Self::rotate_snapshot(&snapshots_dir, &snap, &tmp, &proof)?;  // GATE A
        ledger.record_written(name, verified);
        if let Some(ob) = self.mmap_bundles.get_mut(name) { ob.rebase(MmapBundle::open(&snap)?, schema); }
    }
    let receipt = ledger.seal(&known, &clean, self.wal_replay_complete)?;
    self.compact_wal_to_schemas(receipt)?;                                  // GATE B
    Ok(report)
}
```

Three specifics that are load-bearing:

- **Iterate the schema union, never a storage map.** The incident is literally "iterated the wrong map". Today: `engine.rs:2348` (`&self.bundles`), `engine.rs:2846` (`&self.bundles`), `engine.rs:2504` + `2517` (two loops, hand-unioned), `engine.rs:3056` (`mmap_bundles.keys()`) + `engine.rs:3141-3148` (a second hand-written "and now the heap ones" loop). A third storage mode cannot reintroduce the bug if the loop is over `known`.
- **Stream; do not go through `clone_bundle_data` (`engine.rs:2503`).** It is the only currently-correct collector but it materializes every record as `serde_json::Value` up front — the exact allocation storm behind the 2026-06-26 encoder hang that the bypasses at `engine.rs:2365-2379` and `engine.rs:2857-2877` exist to avoid. `mmap_rebase_snapshot` (`engine.rs:3066-3110`) has the same problem and, separately, **no budget at all** — `per_bundle_timeout_secs` (`engine.rs:200`, default `Some(600)`, added for #104/#105) is silently inactive on the primary production compaction path. The unified enumerator inherits the budget for all target kinds; that is a free fix.
- **The rebase ordering already in `engine.rs:3117-3122` is correct** — `MmapBundle::open` on the promoted file precedes `rebase()`, so the new base is durable before the overlay is cleared (`mmap_bundle.rs:404-413`). Lift it verbatim; no new crash window.

### 2.6 Delete the empty-skip family; write a zero-record `.dhoom`

Five sites: `engine.rs:2353-2355`, `engine.rs:2851-2853`, `engine.rs:3158-3160`, the `store.len() > 0` filter at `engine.rs:3145`, and `clone_bundle_data`'s `engine.rs:2506` + `engine.rs:2541`.

Under set-equality coverage the skip becomes *inexpressible* rather than merely discouraged. Cost is one header per bundle.

**Verified by reading, but write the gate-zero test first anyway:** `StreamingDhoomEncoder::finish` writes `"{name}{{}}:\n"` for zero pushes (`dhoom.rs:3441-3445`); `parse_fiber` accepts `name{}` because `fields_str` is empty and empty tokens are skipped (`dhoom.rs:382-388`); `MmapBundle::from_mmap`'s body is `"\n"`, whose single empty line is consumed by the `pos == header_end` guard (`mmap_bundle.rs:98-102`), leaving `line_offsets` empty → `len() == 0`. Two-line test: encode zero pushes, `MmapBundle::open`, `assert_eq!(len(), 0)`.

Corollaries required for consistency, or E4 turns every dropped bundle into a permanent empty resurrection:

- `drop_bundle` (`engine.rs:1583-1591`) must unlink `snapshots/{name}.dhoom` and `.dhoom.prev`, and its `existed` expression uses `||` short-circuit at `engine.rs:1585-1586` — a name in both maps only gets removed from the heap one. Change to two statements.
- `open_mmap` Phase 1 (`engine.rs:640-648`) must handle `WalEntry::DropBundle`; today it falls into `_ => {}` while `do_replay` handles it correctly at `engine.rs:982-985`. **Same on-disk state, two boot paths, two different answers.**
- `create_bundle` (`engine.rs:1316-1319`) must reject a name already present in `self.mmap_bundles`. Today it inserts a fresh empty `BundleStore` into `self.bundles`, and `insert` checks `self.bundles` first (`engine.rs:1336`), so every subsequent write lands in a shadow heap store while the real mmap data goes unreachable — and then gets snapshotted over. `init_app_bundles`' own comment at `gigi_stream.rs:16072` already warns that POSTing to `/v1/bundles` from a handler is destructive on this version.

### 2.7 The holes the adversarial pass found, and how each closes

**H1 — the guard is placed after the damage.** `rotate_snapshot` runs *inside* the per-bundle loop (`engine.rs:2476`, `2674`, `3114`, `3174`) and unconditionally does `fs::rename(snap_path, &prev_path)` at `engine.rs:2296-2298` before promoting at `engine.rs:2302`. Compaction runs *after* the loop (`engine.rs:2492`, `2943`→`2997`, `3178`). By the time a WAL-level guard returns `Err`, every bundle in the cycle has already had generation N-1 destroyed. A 409 saying `"No data was lost"` would be the same class of lie as `{"status":"ok"}`.
**Closure: GATE A.** `rotate_snapshot` takes a `&NonRegression` and refuses at the top. All five call sites stop compiling until they produce one.

**H2 — coverage is existential, not non-regression.** "File exists, non-zero, mtime ≥ cycle_start" proves a write happened, never that it wasn't a regression. Three verified triggers produce a partial RAM view that writes a small, valid, fresh `.dhoom` and passes: (a) boot heap-fallback on an unopenable `.dhoom` — `engine.rs:740-743` / `754-755` install an **empty** `BundleStore`, and the `parse_shaped` gate at `engine.rs:711-717` only covers `InvalidData`/`UnexpectedEof`/0-byte, so `Unsupported` (the delta-field reject at `mmap_bundle.rs:62-68`), `PermissionDenied`, EMFILE and ENOMEM go straight to empty *without even trying `.prev`*; (b) crashed rotation between `engine.rs:2298` and `2302` → no `.dhoom` → `engine.rs:766-770` "Heap-only (no snapshot)" → empty store; (c) `create_bundle` shadowing (§2.6).
**Closure: P3 in `NonRegression`.** Before `engine.rs:2298`, if `snap_path` exists, `MmapBundle::open(&snap_path)?.len()` — a `memchr` line scan with no per-record parse (`mmap_bundle.rs:74-110`), the same scan boot already pays. Require `new + accounted_deletions ≥ old`. `accounted_deletions` is exact and already on hand: `ob.tombstone_len()` (`mmap_bundle.rs:357`) for overlays, a per-cycle counter for heap. Any unexplained shrink ⇒ refuse the rotation, park the new file as `<name>.dhoom.rejected`, mark the bundle uncovered so GATE B also refuses. All three triggers present identically as *"the file I am about to write has fewer records than the one I am about to overwrite, and I cannot account for the difference."*
**Plus GATE C** so the partial view never exists: `open_mmap` must distinguish three states it currently collapses into one — (i) no `.dhoom` **and** no `.prev` ⇒ genuinely new, heap store, benign (the only case `engine.rs:768`'s message is honest about); (ii) no `.dhoom` **but** `.prev` exists ⇒ crashed rotation; (iii) `.dhoom` exists but failed to open. Cases (ii) and (iii) get `Quarantined`: reads 503, **writes 503** (critical — this is what stops new WAL records from papering over the gap and turning (ii) into (a)), never snapshotted, never rotated, and forces engine-wide `wal_compaction_blocked = true`.

**H3 — non-bundle WAL state.** `WalEntry::GaugeFieldSnapshot` is live in prod (`Cargo.toml:108` `halcyon = ["lattice","gauge"]`; `Dockerfile:11`), its **sole** durable form is the WAL (`engine.rs:2018` is the only durable write; there is no snapshots-dir file), both boot paths depend on it (`engine.rs:537`, `engine.rs:843` → `replay_gauge_substrate` `engine.rs:1071` → `replace_buffer` `engine.rs:1251-1262`), and `compact_wal_to_schemas` re-emits only the *declares* (`engine.rs:2739-2748`), which are metadata-only by design. Every compaction silently reverts every thermalized field to its Haar/identity seed, visible only at the next boot. A coverage certificate quantified over bundle names cannot see this — and would upgrade "nobody checked" into an affirmative `uncovered: []`.
**Closure, two parts.** (i) Re-emit: after the `GaugeFieldDeclare` loop at `engine.rs:2743-2748`, iterate `crate::gauge::registry::all()` and `log_gauge_field_snapshot` for each live buffer. Ordering is load-bearing — the snapshot must follow its declare, or replay hits the orphan-skip at `engine.rs:1215-1226` which `return Ok(())`s after an `eprintln!`, reproducing the loss *and still booting green*. (ii) The exhaustive `classify()` in `durability.rs`: `durable_checkpoint` refuses when the WAL it is about to drop contains any op kind classified `RebuiltByCompactor` that was not re-emitted, and an unclassified variant is a compile error. (i) fixes today's bug; (ii) is what would have caught it the day it was written. Full audit of `wal.rs:774-880` against the compactor: `GaugeFieldSnapshot` — **live loss**; `HamiltonianDeclare` (`wal.rs:846-854`, replay deferred) — latent; `MeasurementOverride` — safe (mutates the store, so the `.dhoom` captures it); `CreateTrigger`/`DropTrigger`/`LatticeDeclare` — safe; `DropBundle` — safe only once §2.6 lands; `IntegratorChoice`/`ImagineFallback` — audit trail erased, not data loss.

**H4 — the catalog is itself only durable as WAL bytes.** `self.schemas` is built exclusively from `WalEntry::CreateBundle` (`engine.rs:641-643`, `engine.rs:903-907`), written once at creation (`engine.rs:1317`) and re-anchored only by compaction (`engine.rs:2711-2713`). Today's compaction is not primarily a *shortening* operation — it is a **re-anchoring** operation, and shortening is the hazardous half. `BundleSchema` also carries `gauge_key: Option<GaugeKey>` — key material — plus `invariants`, `indexed_fields`, `adjacencies`, none of which a `.dhoom` carries. And `finish_wal_replay_prefix` (`engine.rs:584-590`) converts a CRC mismatch *anywhere* in the file into `Ok(())` with a warning, discarding everything after; `WalReader::replay` returns `Err` on the first mismatch (`wal.rs:943-948`, `wal.rs:917`). So a mid-file corruption truncates the *logical* schema set on every boot, and the next compaction makes that truncation **permanent**.
**Closure.** (i) `snapshots/_catalog.json` — schemas + triggers + lattice/gauge declares + per-bundle record counts + the WAL generation id — written through `rotate_snapshot` **before any receipt is minted**, with its own `.prev`. One artifact, three jobs: durable catalog, non-regression baseline, boot audit baseline. No DHOOM format change, no version bump. (ii) Boot's universe becomes `catalog ∪ {CreateBundle in WAL}`. (iii) `Engine` gains `wal_replay_complete: bool`, set false by `finish_wal_replay_prefix`; `seal()` refuses on false (P5). (iv) Boot audit runs the *converse* direction, which cannot empty out with the thing it watches: enumerate `snapshots/*.dhoom` and assert every one has a catalog entry — **a `.dhoom` with no schema is a proven catalog gap and is the exact signature of the ten vanished bundles.** Refuse `ready` on any catalog-cardinality decrease. (v) Bonus: `has_dhoom_files` (`gigi_stream.rs:15856-15866`) currently accepts *one* file as proof the dataset is present, and gates both the Tigris pull (`gigi_stream.rs:17007`) and the fast path (`gigi_stream.rs:17015`) — make it require the catalog, with a compat branch that falls back to today's behaviour and writes a catalog at the end of the first boot.

**H5 — Tigris mirrors the damage.** `tigris_push` shells `aws s3 sync data_dir/ s3://bucket/ --exclude "*.tmp"` (`gigi_stream.rs:15869-15883`, `15894-15899`) on a **detached thread** with no engine lock (`gigi_stream.rs:17030-17036`, `17183-17189`). `gigi.wal` is in scope; there is no versioning and no generation suffix. A push interleaving with a compaction uploads a truncated WAL next to `.dhoom` files the walk already passed — reproducing the 2026-08-12 pairing *in the bucket*, which is the copy used when a Fly volume is lost.
**Closure:** push to `s3://bucket/gen-<ts>/`, all `.dhoom` first, WAL last, then write a `CURRENT` pointer object; a partial push is never `CURRENT`. Interim one-liner if that's too much for the first pass: order the sync so `snapshots/` completes before the WAL, and never push a WAL older than the `.dhoom` set it is paired with. One accidental silver lining today: `.dhoom.prev` is *not* excluded, so previous generations do reach the bucket.

**H6 — swallowed fsync.** `rotate_snapshot` does `let _ = f.sync_all()` at `engine.rs:2284-2286` and `let _ = d.sync_all()` at `engine.rs:2310-2312`, and skips both entirely on non-unix. The P2 read-back goes through the page cache, so it returns the right length even when the fsync failed with EIO/ENOSPC — green receipt, WAL truncated, bytes never reached the platter.
**Closure:** propagate both errors into the `NonRegression` result; a failed sync is an uncovered bundle.

**H7 — token forgeability.** Covered by §2.1: new module + runtime re-check. `verify` re-stats every claimed file (existence, non-zero length, mtime ≥ cycle_start) and recomputes `known` from the live maps, so a claim about a file that does not exist or was not written this cycle fails regardless of who minted it.

**H8 — liveness.** A refusal is a stuck compaction. A quarantined bundle blocks compaction *forever* with no repair path; the WAL then grows to `max_wal_bytes` (`engine.rs:209`, 2 GiB) and then to disk-full, and silent data loss becomes a slow outage. This trade is real and I am not going to pretend otherwise: the design can take the site down in a scenario where today's code stays up and lossy. Bee's stated preference is unambiguous and I agree with it, but ship the release valve in the same commit: an operator-acknowledged override (loud, recorded to a bundle, one bundle at a time), and alert on the **duration** of `wal_compaction_blocked`, not the boolean.

### 2.8 Additional defect found while designing, in scope because it lives on the same boundary

**Three incompatible tombstone key encodings.** A `DELETE` against a snapshot-resident record does not survive a restart *even with a perfectly intact WAL*.

| # | site | produces |
|---|---|---|
| 1 | `Engine::pk_string` `engine.rs:1434-1444` — sorted `Vec<(&str,&Value)>` of all base fields | `[("id", Integer(7))]` |
| 2 | `OverlayBundle::tombstone_key` `mmap_bundle.rs:440-444` — first base field's value only | `Integer(7)` |
| 3 | `open_mmap` Phase 3 `engine.rs:810` — `format!("{key:?}")` of the whole key `Record`, and `Record = HashMap<String,Value>` (`types.rs:549`), so multi-field keys are also nondeterministically ordered | `{"id": Integer(7)}` |

`Engine::delete` writes #1 (`engine.rs:1535-1536`); `point_query`'s tombstone check reads #1 (`engine.rs:1667-1670`); `mmap_rebase_snapshot`'s and `clone_bundle_data`'s tombstone filters read #1 (`engine.rs:3073`, `engine.rs:2528`) — those three agree. But `OverlayBundle::records`/`len`/`distinct` read #2 (`mmap_bundle.rs:582`), so a live delete stays visible to any caller going through `BundleRef::Overlay` while being permanently erased by the next rebase; and the replayed delete writes #3, which matches nothing on any read path, so **the record comes back on restart.** Collapse all three onto `Engine::pk_string`.

### 2.9 HTTP edge

`admin_snapshot` (`gigi_stream.rs:12579-12612`) returns 200 whenever `report.timed_out_bundles.is_empty()`, with the literal message at `gigi_stream.rs:12592`. Make the ok arm `timed_out_bundles.is_empty() && uncovered.is_empty() && wal_compacted`, and add a **409 Conflict** arm — a refusal, not a crash — whose body leads with the reassurance:

```json
{ "status":"refused", "reason":"durability_coverage", "wal_retained": true,
  "wal_generation_retained": "gigi.wal.1755014400.compacted",
  "message":"WAL NOT compacted. No data was lost. 11 of 4912 bundles are not durable in a .dhoom.",
  "bundles_known":4912, "bundles_covered":4901,
  "uncovered":[{"bundle":"jg_kv","reason":"dirty_overlay","overlay_records":180,"tombstones":0}],
  "next_step":"POST /v1/admin/rebase, or restart to force the mmap-aware path. Do NOT delete gigi.wal." }
```

Plus one grep-able error line mirroring the `ITEM-3-MMAP-SKIP` convention whose format stability is already pinned by a test (`engine.rs:732-739`):
`DURABILITY-REFUSED cycle=<ts> known=4912 covered=4901 uncovered=11 bundles=[jg_kv:dirty_overlay(180/0), …]`
And on `/v1/health`: `wal_compaction_blocked`, `durability_refusals_total`, `uncovered_bundles`, `quarantined_bundles` — so it pages instead of waiting for a restart to surface it.

Also fix the silent-skip precedent while you're here: `cow_snapshot`'s safety skip at `engine.rs:2791-2801` only `eprintln!`s and returns `Ok(...)`, so a compaction skipped *for safety* is invisible to every caller.

### 2.10 The backstop — retention (do this even if nothing else ships)

At `engine.rs:2753`, before installing the new WAL: `fs::rename(gigi.wal → gigi.wal.<unix_ts>.compacted)`, then rename tmp into place. Unlink a retained generation only after a subsequent clean boot completes its coverage audit; cap at 3 generations or 8 GiB, whichever binds first. Let it sync to Tigris (`--exclude "*.tmp"` does not touch it).

This requires **zero correctness reasoning about mmap vs heap**. It does not prevent a violation; it makes any violation that slips through recoverable. It is the difference between 2026-08-12 being a ten-minute restore and being permanent — the forensics that recovered 2,341,314 records had to read a WAL that no longer contained the lost ops.

---

## 3. TESTS THAT FAIL WITHOUT THE FIX

All of these belong in `tests/durability_wal_truncation.rs` (integration), **not** in the `engine.rs` test module. That is deliberate and structural: `mmap_bundles` is a private field (`engine.rs:373`), so from `tests/` every assertion must be about durable state read back through a fresh `Engine::open_mmap`. You cannot accidentally assert about RAM. Every test's final assertion happens after the engine value has been dropped.

Shared fixture helpers: `fn bundle(name) -> BundleSchema` = `BundleSchema::new(name).base(FieldDef::numeric("id")).fiber(FieldDef::categorical("tag"))`; `fn rec(i) -> Record`.

---

### T1 · `admin_snapshot_does_not_erase_mmap_overlay_records` — the incident, full restart simulation

```
P1  Engine::open(&dir); policy.disabled = true; create_bundle(b); insert 0..3; snapshot(); drop
P2  Engine::open_mmap(&dir)                    // b now lives in mmap_bundles  (engine.rs:702-708)
    engine.compaction_policy_mut().disabled = true;   // ← LOAD-BEARING, see below
    insert(b, id=99)                            // overlay engine.rs:1338-1339 + WAL engine.rs:1335
    assert_eq!(engine.total_records(), 4);
P3  let report = engine.snapshot_with_report().unwrap();   // the exact admin path
    drop(engine);
P4  let engine = Engine::open_mmap(&dir).unwrap();
    assert_eq!(engine.total_records(), 4, "overlay record must survive the admin snapshot");
    assert!(engine.point_query("b", &key(99)).unwrap().is_some());
```

`policy.disabled = true` in P2 is load-bearing: without it, the `maybe_checkpoint` → `maybe_auto_compact` chain may route to the *correct* `mmap_rebase_snapshot` branch (`engine.rs:3040`) and the test passes for the wrong reason. The check is at `engine.rs:3023`.

**TODAY: FAILS, LOSES DATA.** P4 returns 3 and the point query is `None`. `snapshot_with_chunk_size_report` iterates `&self.bundles` only (`engine.rs:2348`), never visits `b`, writes no `.dhoom` for it, finds `timed_out_bundles` empty (it is only ever pushed at `engine.rs:2466`, inside that same loop), and compacts at `engine.rs:2491-2493`. The overlay's only durable form was the WAL.

**Mechanism-removal gate:** delete the mmap arm of the enumerator in `durable_checkpoint` and re-run. It must fail *with data loss* (P4 → 3), not merely with a different status code.

---

### T2 · `deleted_records_do_not_resurrect_across_restart` — the empty-skip family

```
P1  Engine::open; create b; insert 0..3; snapshot();        // b.dhoom holds 3
P2  same engine: delete(b, key(0..3));                      // WAL Deletes; store.len() == 0
    snapshot();                                             // engine.rs:2851-2853 skips; 2997 truncates
    drop
P3  Engine::open(&dir);  assert_eq!(total_records(), 0);
    for i in 0..3 { assert!(point_query("b", &key(i)).unwrap().is_none()); }
P3' same assertions after Engine::open_mmap(&dir)           // pin both boot paths
```

**TODAY: FAILS, RESURRECTS DATA.** P3 returns 3. `compact_wal_to_schemas` re-emits no `Delete` (`engine.rs:2711-2750`), and `do_replay`'s snapshot load at the first `Checkpoint` fires because `store.is_empty()` is true (`engine.rs:909-916`). On the mmap path, Phase 2 mmaps the stale file at `engine.rs:702-708` and Phase 3 has no Deletes to replay.

For `id_verification`-shaped data a silent un-delete is worse than a loss. This is the cheapest of the tests and should ship first. Add a variant that forces `maybe_auto_compact` (`max_wal_entries = 1`, as `engine.rs:6213` does) to pin `engine.rs:3145` and `engine.rs:3158-3160` on the runtime path too.

---

### T3 · `mmap_delete_survives_restart_with_intact_wal` — the tombstone-encoding trio, isolated

```
P1  Engine::open; create b; insert 0..3; snapshot(); drop
P2  Engine::open_mmap; policy.disabled = true;
    delete("b", key(1));                                    // tombstone keyed by pk_string, engine.rs:1535
    assert!(point_query("b", &key(1)).unwrap().is_none());
    assert_eq!(engine.bundle("b").unwrap().len(), 2);        // OverlayBundle::len, mmap_bundle.rs:545
    drop(engine);                                            // NO snapshot — WAL still holds the Delete
P3  Engine::open_mmap;  assert!(point_query("b", &key(1)).unwrap().is_none());
```

**TODAY: FAILS TWICE.** The `len()` assertion in P2 fails (encoding #2 vs #1) and P3 returns `Some` — the record is back. Phase 3 replay writes encoding #3 (`engine.rs:810`), which matches nothing on any read path. No compaction is involved; a plain restart with a perfectly intact WAL loses the delete. Keep this test separate from T2 so a failure names one mechanism.

---

### T4 · `partial_boot_view_cannot_overwrite_a_good_snapshot` — GATE A + GATE C

```
P1  Engine::open; create b; insert 0..999; snapshot(); drop        // b.dhoom = 1000 records
P2  fs::rename(snapshots/b.dhoom, snapshots/b.dhoom.prev);         // simulate a crash between
                                                                    // rotate step 3 (engine.rs:2298)
                                                                    // and step 4 (engine.rs:2302)
P3  Engine::open_mmap(&dir);            // engine.rs:766-770 → "Heap-only (no snapshot): b", EMPTY store
    insert("b", id=5000);               // one post-crash write
    let r = engine.snapshot_with_report();
    // REQUIRED: either r is a refusal (b quarantined, no b.dhoom written, WAL retained),
    // or b was recovered from .prev. Never: a 1-record b.dhoom.
    drop(engine);
P4  let engine = Engine::open_mmap(&dir).unwrap();
    assert!(engine.total_records() >= 1000,
            "boot manufactured an empty store while a .dhoom.prev existed");
```

**TODAY: FAILS, LOSES 1000 RECORDS.** P4 returns 1. Boot took `engine.rs:759-770` and made an empty store; the snapshot wrote a 1-record `b.dhoom`; because `snap_path.exists()` was false, `rotate_snapshot` skipped step 3 and left `.prev` intact *this cycle* — so the loss is not yet permanent. Run **one more** snapshot cycle and it is: `.dhoom` now exists, `engine.rs:2296-2298` overwrites `.prev` with the 1-record generation, and the 1000 are gone from every copy including Tigris. Add that fifth phase and assert `.prev` still holds 1000.

Uses only `fs::rename` — no corrupt-file construction, fully deterministic. This is the test that proves the WAL guard alone is insufficient.

---

### T5 · `gauge_field_snapshot_survives_wal_compaction` — H3, `#[cfg(feature = "gauge")]`

```
P1  declare lattice + SU(2) field; thermalize (or replace_buffer with a distinctive buffer);
    let r = engine.snapshot_gauge_field_durable(name, group, buf)?;   // engine.rs:2002 → 2018
P2  engine.snapshot_with_report()?;    // compact_wal_to_schemas re-emits declares only, engine.rs:2739-2748
    drop(engine);
P3  Engine::open_mmap(&dir);           // engine.rs:843 → replay_gauge_substrate engine.rs:1071
    assert_eq!(sha256_of_live_buffer(name), r.sha256);
```

**TODAY: FAILS.** The buffer is back to its `(init_kind, init_seed)` state — `GaugeFieldDeclare` is metadata-only by design (`wal.rs:830-841`), and no `GaugeFieldSnapshot` is re-emitted. Verified by reading the path, not by running.

Add a companion compile-time gate: a `classify()` match with no `_ =>` arm, so the next `WalEntry` variant cannot be added without a decision.

---

### T6 · `compaction_refuses_when_wal_replay_stopped_short_of_eof` — H4 / P5, the ten-bundle shape

```
P1  create A, insert A, checkpoint, create B, insert B, snapshot();    // A.dhoom and B.dhoom both exist
    insert one more into B (post-checkpoint); drop
P2  flip one CRC byte in gigi.wal at an offset BEFORE B's CreateBundle
P3  Engine::open_mmap(&dir);           // finish_wal_replay_prefix engine.rs:584-590 swallows it;
                                        // self.schemas == {A}
    engine.snapshot_with_report();      // compact_wal_to_schemas re-emits {A} only, engine.rs:2711-2713
    drop
P4  Engine::open_mmap(&dir);
    assert!(engine.bundle("B").is_some(), "B's schema was erased by compaction");
    assert_eq!(engine.bundle("B").unwrap().len(), <expected>);
```

**TODAY: FAILS, LOSES BUNDLE B PERMANENTLY.** `B.dhoom` sits on disk with no schema; Phase 2 iterates `schemas` (`engine.rs:681`) and never opens it. This is exactly the observed signature — bundle gone from the registry, zero create/insert ops for it in the WAL.

**After the fix:** P3 must return a refusal (`ReplayIncomplete`), and P4's boot audit must report `B.dhoom` as an orphan against `_catalog.json` and refuse `ready`.

---

### T7 · `wal_generation_is_retained_across_compaction` — the backstop

Any successful compaction; assert `gigi.wal.<n>.compacted` exists, that its entry count exceeds the new WAL's, and that `report.wal_generation_retained` names it. **TODAY: FAILS** — no such file can exist; `engine.rs:2753` renames over the only copy.

---

### T8 · `snapshot_report_cannot_report_ok_with_partial_coverage` — the reporting surface

T1's fixture; assert `report.bundles_known == report.bundles_covered` and `report.wal_compacted`.
**TODAY: DOES NOT COMPILE** — `SnapshotReport` (`engine.rs:229-234`) has no such fields. That is the point, and it is the cleanest possible statement of why 2026-08-12 returned 200: the struct is structurally incapable of describing the failure.

---

### Why the existing durability suite missed all of this

- **Not one test in the repository calls a snapshot/compaction entry point on an engine whose `mmap_bundles` map is non-empty.** Every snapshot call site in the suite is preceded by `Engine::open` (heap): `engine.rs:3648, 3711, 3940, 3958, 3988, 4016, 4068, 4279, 4353`; `tests/snapshot_rotation.rs:67, 84, 296, 464, 479`; `tests/encoder_high_dim_smoke.rs:109, 156, 198`; `tests/engine_open_mmap_orphan.rs:66, 182`; `tests/explain_mmap.rs:42, 96`; `tests/snapshot_high_field_wedge.rs:120, 379`. In every mmap test `open_mmap` comes **after** the snapshot, purely to read the result back (`tests/snapshot_high_field_wedge.rs:307` vs the snapshot at `:293`). So `self.bundles` is accidentally total and `engine.rs:2348` is accidentally correct.
- The one mixed heap+mmap regression test, `mmap_rebase_also_snapshots_heap_only_bundles` (`engine.rs:6157-6230`), asserts only `Path::exists()` on the two `.dhoom` files (`engine.rs:6220-6227`) and **never restarts**. It would pass against a build that writes 0-byte snapshots. And it reaches compaction through `maybe_auto_compact` → `mmap_rebase_snapshot` (`engine.rs:6218`), never through `snapshot_with_report`.
- `mmap_rebase_snapshot` — the one path that has always been correct — has **zero restart coverage**. `mmap_rebase_snapshot_roundtrip` (`engine.rs:5231-5307`) asserts `overlay_len()==0`, `tombstone_len()==0`, `base().len()==5` and three point queries **against the same live engine object**, then `cleanup(&dir)`. If that path acquired the defect tomorrow the suite would stay green.
- The tests that *do* simulate restart — `snapshot_survives_wal_compact` (`engine.rs:3628-3690`), `snapshot_then_new_inserts_survive_reopen` (`engine.rs:3693`), `autocompact_data_survives_cycle` (`engine.rs:4433`) — are exactly the right **shape** and are the template above. They miss for one reason only: `Engine::open` at every phase (`engine.rs:3634, 3699, 3716, 3730, 4438, 4463`). One fixture change from `open` to `open_mmap` in phase 2 is enough to make the class detectable.
- The guard that exists is `report.timed_out_bundles.is_empty()` at `engine.rs:2491`, `2787`, `2819`. It is the wrong predicate: it answers *"did anything I visited fail?"* when the invariant needs *"did I visit everything, and is what I wrote what I hold?"* A bundle that is silently never enumerated produces an empty list and passes the guard.
- The named prior incidents each got a test that pinned the **symptom**, never the invariant: 2026-05-26 heap-only-bundles → file-existence only; 2026-06-26 encoder wedge → wall-clock + field-set round-trip (`tests/snapshot_high_field_wedge.rs`), silent on *which bundles get iterated*; #105 → a correct, well-documented guard (`engine.rs:2771-2779`) on the wrong predicate.

### Structural properties of the new tests that prevent a repeat

1. **Integration-only.** `mmap_bundles` is private (`engine.rs:373`), so RAM cannot be asserted about. The restriction is the feature.
2. **Every final assertion follows a drop and a re-open from `data_dir` alone.** That is the only shape that distinguishes "in RAM" from "on disk", and it is what all five nearest-neighbour tests lack.
3. **Coverage is asserted as a set equality over the schema union**, never as a file-existence boolean.
4. **Each test names the mechanism it destroys**, and the spec requires running each with that mechanism deleted and observing a *data-loss* failure — not a different status code. Per your own rule: a gate that accepts two outcomes tests neither.

---

## 4. ORDER OF WORK

**W0 — retention (§2.10). Land alone, first.** `engine.rs:2753` (and the two siblings before they are deleted). No correctness reasoning about mmap vs heap, no behaviour change on the success path, no new refusals. Ships with T7. This is what closes the window soonest: every subsequent instance of the entire class becomes a restart-and-restore instead of a permanent loss, including instances this spec has not anticipated.

**W1 — stop the bleeding on the live route. Same day.** Point `admin_snapshot` (`gigi_stream.rs:12582`), `Engine::snapshot_with_report` (`engine.rs:2221`), `Engine::snapshot` (`engine.rs:2212`) and `Engine::compact` (`engine.rs:2173`) at a single dispatch that does what `maybe_auto_compact` already does at `engine.rs:3037-3041`. Ships with T1. The incident becomes unreproducible before any type-state exists.

**W2 — empty-skip deletion (§2.6) + its three corollaries.** Pure deletion plus three small guards. Ships with T2 and the gate-zero empty-`.dhoom` round-trip. Must precede W3, or clause (E) can never be satisfied by a legitimately-emptied bundle and the new refusal is permanent on prod.

**W3 — the choke point.** `src/durability.rs`, the receipt, delete the two duplicate truncators, `SnapshotReport` coverage fields, the 409, `/v1/health`. Ships with T8. **Run `verify` in warn-only mode for one production cycle** — log `DURABILITY-REFUSED` without returning `Err` — confirm the uncovered set is empty, then flip to hard refusal. The warn-only stage is also the honest way to find out whether the ten bundles have a second cause.

**W4 — GATE A + GATE C.** `NonRegression`, read-back verify, boot quarantine, fsync propagation. Ships with T4.

**W5 — schema durability.** `_catalog.json`, `wal_replay_complete`, orphan-`.dhoom` boot audit, `DropBundle` on the fast path, `has_dhoom_files` hardening. Ships with T6.

**W6 — gauge re-emit + exhaustive `classify()`.** Ships with T5.

**W7 — tombstone key unification (§2.8).** Ships with T3.

**W8 — Tigris ordering + generation prefix + `CURRENT` pointer.** Independent of everything above; can run in parallel with W4-W7.

W0-W2 are the fast, low-risk half and remove the production trigger. W3-W5 close the class. W6-W8 close what the class-closing move does not reach.

---

## 5. WHAT THIS DOES NOT FIX

**The already-lost records: nothing.** The forensics are conclusive — a CRC-validated parse of all 2,341,314 valid WAL records found zero create/insert ops for them. The pre-compaction WAL was renamed over at `engine.rs:2753` and the offsite copy was overwritten by `aws s3 sync` (`gigi_stream.rs:15871`) on the next boot's push (`gigi_stream.rs:17034`). Two things are worth ten minutes before you call it closed, and only two: (1) `.dhoom.prev` is **not** excluded from the sync, so the pre-compaction generation of every *rewritten* snapshot may still be in the bucket — that does not help `jg_kv`'s post-June records or the ten (they were never rewritten), but it may help others; `aws s3 ls` will tell you. (2) Fly volume snapshots, if retention reaches back to 2026-06-26.

**The ten-bundle root cause is a hypothesis, not a conclusion.** On my model those bundles should have survived as *empty*: `compact_wal_to_schemas` re-emits a `CreateBundle` for every entry in `self.schemas` (`engine.rs:2711-2713`) and `open_mmap` reconstructs an empty heap store for any schema lacking a `.dhoom` (`engine.rs:759-770`). Their total disappearance requires `self.schemas` to have been *already incomplete* when compaction ran. Exactly two code-supported routes exist: (i) a CRC prefix truncation upstream (H4 / `engine.rs:584-590`), or (ii) `DropBundle`. The discriminating probe, ~1 minute: `ls /data/snapshots/ | grep <name>` — a `.dhoom` present with no schema is the truncation/orphan signature; and `grep "stopped at corrupted WAL tail"` (`engine.rs:586`) plus `grep "Heap-only (no snapshot):"` (`engine.rs:768`) and `"ITEM-3-MMAP-SKIP"` (`engine.rs:737/752`) across the boot logs on either side of 2026-06-26. Which route it is decides whether W5 or W1 is the load-bearing fix. **If neither route matches, there is a third bug and INV-D does not cover it** — INV-D quantifies over schemas and assumes the schema set is itself durable, which is exactly the assumption W5 exists to make true.

**Unflushed WAL tail on a hard kill.** `write_entry` goes into a `BufWriter` with no flush (`wal.rs:748-764`); `sync()` (`wal.rs:739-742`) fires only from `Engine::checkpoint` (`engine.rs:2164-2169`, every `checkpoint_interval` ops, default 10,000 via `maybe_checkpoint` `engine.rs:3010`) and `batch_insert` (`engine.rs:1622`). `Engine::insert` (`engine.rs:1326-1352`) does not sync, and the graceful-shutdown handler (`gigi_stream.rs:17196-17221`) never calls `checkpoint()`. On a Fly SIGKILL or OOM the buffered tail is gone. Out of scope here, but it means **"WAL-resident" in INV-D must be read as "fsynced WAL-resident"**, and a shutdown-hook `checkpoint()` is a two-line win someone should take.

**WAL-bypass mutations.** `truncate_bundle` (`gigi_stream.rs:10858-10879`) and `ttl_eviction_task` (`gigi_stream.rs:16441-16445`) mutate through `bundle_mut` with no `wal.log_*` call, unlike `insert`/`update`/`delete` which all log first (`engine.rs:1335`, `1465`, `1531`). The coverage predicate *sees* these (they show up as overlay/tombstone deltas or a shrunken store), so after this work they cannot cause silent loss — but the mutations themselves are still undone by the next replay. `ttl_eviction_task` additionally stops working entirely once a log bundle acquires a `.dhoom`, because of the `as_heap_mut()` / `else 0` at `gigi_stream.rs:16441-16445`. There are ~15 other `bundle_mut` call sites in `gigi_stream.rs` (2162, 10554, 10601, 10657, 10702, 10827, 10949, 10984, 11032, 11057, 11416, 11553, 11753, 12351, 16440) that need the same audit. Bounded separate pass.

**Authorization on the destructive route.** `/v1/admin/snapshot` (`gigi_stream.rs:16617`) sits under the same blanket `auth_middleware` layer as everything else (`gigi_stream.rs:16940`) and `admin_snapshot` never reads `GigiClaims`. A namespace-scoped tenant token can trigger a global WAL compaction affecting every tenant. Not a durability bug, but it is the same button. File it.

**Availability.** Covered in H8, restated because it is the real cost: this design can take the site down in a scenario where today's code stays up and lossy. Ship the override and the duration alert with W3.

**Concurrency, read but not proven.** Every mutator is `&mut self` and `admin_snapshot` holds `state.engine_write()` across the whole blocking call (`gigi_stream.rs:12580-12583`), so verify-then-rename is exclusive under the outer lock. `OverlayBundle` mutates through `&self` (`mmap_bundle.rs:298, 311, 320`) and `Engine::mmap_bundle(&self)` hands out `&OverlayBundle` (`engine.rs:1785`), but that `&self` still comes from the outer `RwLock`. I did not audit for an `Arc<OverlayBundle>` escape. If one exists, the O(1) clean-clause has a TOCTOU window and needs a generation counter inside `OverlayBundle` rather than a length read.

**Not audited at all:** `import_bundle` / `ingest_dhoom` handler bodies (routes at `gigi_stream.rs:16606-16607`) for direct writes into `snapshots/`; `concurrent.rs:100`'s own `checkpoint()` and whether it shares the Engine WAL.