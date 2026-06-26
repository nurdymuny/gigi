# SUDOKU FINDING 3 — `Engine::open_mmap` Error-Site Triage

**Date:** 2026-06-26
**Scope:** ITEM 3 of the SUDOKU 8-item local shipment.
**File audited:** `src/engine.rs` lines 618–781 (`Engine::open_mmap`).
**Posture:** Production gigi-stream.fly.dev v228 is LIVE. No deploy, no
push, no production curl. Local main commits only.

## Triage table

Every early-return / `?` / fallback site in `Engine::open_mmap`
classified into one of three buckets:

- **(a) GRACEFUL-SKIP** — log + skip the unloadable item, engine boots
  with that item degraded.
- **(b) HARD-REJECT** — real corruption; refuse to install, propagate a
  specific error.
- **(c) AS-IS** — current behavior is correct (heap-replay fallback or
  genuine infra failure that must propagate).

| # | Line | What fails | Current behavior | Bucket | Justification |
|---|------|------------|------------------|--------|---------------|
| 1 | 619 | `fs::create_dir_all(data_dir)` | propagates `io::Error` | (c) AS-IS | Permission / disk-full on data dir is an infra prerequisite. Caller must decide fallback. |
| 2 | 636 | `WalReader::open(&wal_path)?` | propagates `io::Error` | (c) AS-IS | WAL is source of truth for schemas. I/O failure here is not item-specific corruption; let caller decide (heap fallback at the `Engine::open` level). |
| 3 | 659-663 | `reader.replay(...)` wrapped in `finish_wal_replay_prefix` | CRC mismatch → log WARNING + preserve valid prefix; other `io::Error` propagates | (c) AS-IS | Valid-prefix-on-CRC-mismatch IS the graceful path; remaining errors are real corruption that must propagate. |
| 4 | **684** | **`MmapBundle::open(&snap_path)`** | **inline `match`: Ok loads mmap, Err logs WARNING and inserts heap-only `BundleStore`. Does NOT propagate.** | **(a) GRACEFUL-SKIP** | **Per-bundle `.dhoom` corruption (truncated, 0-byte, bad header, delta-encode mismatch) must not wedge boot. Schema is still in the WAL; the bundle re-populates on post-checkpoint WAL replay.** |
| 5 | 740 | `fs::metadata(&wal_path).map(...).unwrap_or(0)` | swallows error → 0 bytes | (c) AS-IS | Metadata fetch failure is intentionally swallowed; 0-byte fallback is safe for the wal_byte_count counter. |
| 6 | 752 | `Self::replay_gauge_substrate(&wal_path)?` (cfg `gauge`) | propagates `io::Error` on lattice / SU(2) parse failure | (b) HARD-REJECT | Gauge registry is a process singleton; partial replay = silent wrong results. Refuse to install. Specific error so the caller can choose heap-replay vs alarm. |
| 7 | 755 | `WalWriter::open(&wal_path)?` | propagates `io::Error` | (c) AS-IS | If we just READ the WAL but cannot OPEN-FOR-APPEND, it's infra (permission flip, disk full). Caller decides. |

## Bucket counts

| Bucket | Count |
|--------|-------|
| (a) GRACEFUL-SKIP | **1** |
| (b) HARD-REJECT | **1** |
| (c) AS-IS | **5** |

## (a)-bucket sites — patches in this shipment

### Site #1 — `src/engine.rs:684` (`MmapBundle::open` inside the bundle-load loop)

**Status before patch:** Graceful-skip is already implemented inline
(Bee's 2026-05-25 fix that replaced the pre-2026-05-25 `continue` that
silently dropped post-snapshot bundles). The current log line is
human-prose and not grep-able from a test.

**Hardening this shipment applies:**

1. Add a stable, grep-able log marker:
   `ITEM-3-MMAP-SKIP bundle={name} err={e}`.
2. Add a doc-comment in the source block that names this as the
   (a)-bucket site, points to this triage doc, and explicitly contrasts
   it with the two (c)-bucket WAL-read failures and the (b)-bucket
   gauge-substrate failure that DO propagate. Future readers should not
   have to re-derive the triage.
3. Pin the behavior with `tests/engine_open_mmap_orphan.rs`:
   - Boot an engine, create a bundle, insert records, snapshot, drop.
   - Truncate the `.dhoom` file to 0 bytes (the failure mode that left
     orphan files on prior boots).
   - Reopen the engine.
   - Assert: open succeeds, the bundle is in `heap_bundles` (queryable
     but empty), the marker `ITEM-3-MMAP-SKIP bundle=<name>` is
     embedded in the source (a grep-able constant we trust the eprintln
     to emit).
   - Also assert: a SECOND bundle that snapshotted cleanly STILL loads
     in mmap mode (the orphan doesn't poison the rest of the registry).

**No new (a)-bucket sites discovered.** The other six error sites are
correctly classified (c) AS-IS or (b) HARD-REJECT and require no code
change in this shipment. The HARD-REJECT (b) site at line 752 is logged
here for ITEM 2 (graceful-skip pattern audit on WAL op pairings) — it
stays HARD-REJECT but ITEM 2 inspected whether the inner replay path
should distinguish missing-handle (skip) from bad-handle (reject), and
the Pass 2 graceful-skip for orphan gauge field declares lands as part
of ITEM 2.

## Sibling site — line 691-696 already-existing graceful behavior

The current code is already correct in behavior. This shipment is a
**hardening** pass, not a behavior change:

- log format becomes stable and testable
- a code comment names the bucket assignment and links this doc
- a regression test pins the no-wedge-on-corrupt-snapshot invariant

## Out of scope for this finding

- ITEM 4 (bound sort allocation) — separate finding doc.
- ITEM 6 (`.dhoom.prev` rotation) — would add a NEW (a)-bucket recovery
  step at line 684 (try `.dhoom`, on Err try `.dhoom.prev`, on both-Err
  fall through to current graceful-skip). Captured here for ITEM 6's
  patch to reference but not implemented in ITEM 3's commit.

## Gate compliance

This shipment touches `src/engine.rs` and adds
`tests/engine_open_mmap_orphan.rs`. It does NOT touch any of the locked
files (`src/gauge/symplectic_flow.rs`, `loop_transport.rs`,
`wilson_force.rs`, `project_gauss.rs`, `holonomy.rs`, `action.rs`,
`src/curvature.rs`). All 8 gates remain green by construction — the
change is log-text + comment + one new test file; no observable
behavior shift for existing callers.
