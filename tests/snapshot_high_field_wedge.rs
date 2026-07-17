//! Regression + repro suite for the durability snapshot-encoder wedge on
//! WIDE numeric bundles (2026-06-26 / 2026-07-16 incident).
//!
//! ## What actually wedged
//!
//! Production `gigi-stream` hung indefinitely on the boot heap-replay
//! snapshot (`Engine::snapshot_with_report` → `snapshot_with_chunk_size_report`)
//! while snapshotting Marcella's `marcella_source_embeddings_bge_v2`
//! (9,964 records) and `stacks_passages` (70,849 records). The earlier
//! ITEM-4 fix (`should_bypass_sort`) only removed the *engine-level*
//! `Vec<serde_json::Value>` sort/clone — it left the wedge in place.
//!
//! The real cost is inside `dhoom::encode_bundle` **Phase 2**:
//! `detect_computed_field` is O(F²·N) per key and is called once per
//! remaining key, so Phase 2 is O(F³·N) in the number of numeric
//! candidate fields F. It runs inside a single `StreamEncoder::new`
//! (reached from `StreamingDhoomEncoder::finish`/`flush_chunk`), so the
//! between-records timeout in the snapshot loop can NEVER interrupt it.
//!
//! ## Why the pre-existing `encoder_high_dim_smoke::smoke_bge` MISSED it
//!
//! That test stores the 384-dim embedding as ONE `Value::Vector` fiber
//! (2 keys total). `detect_computed_field` calls `.as_f64()` on the array
//! field, gets `None`, and early-returns — the cubic never runs.
//!
//! The PRODUCTION bundle stores the embedding as **384 SEPARATE scalar
//! fibers** `v0..v383` (all numeric). Every candidate passes `.as_f64()`,
//! so the full O(F³·N) fires. This suite reproduces the real shape.
//!
//! All tests run under `--no-default-features` (no feature flag needed):
//! they drive the public `Engine::snapshot_with_report` path — the exact
//! call the boot heap-replay path makes at `gigi_stream.rs`.

use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use gigi::engine::Engine;
use gigi::parser::{execute, parse, ExecResult};
use gigi::types::{BundleSchema, FieldDef, Record, Value};

/// Wall-clock ceiling that separates "fixed" (completes) from "wedged"
/// (days). Comfortably above the post-fix completion time and far below the
/// production per-bundle budget (600s).
const WEDGE_GUARD_SECS: u64 = 30;

/// Records for the wedge pin. Production `marcella_source_embeddings_bge_v2`
/// had 9,964; the wedge is O(F³·N), so it hangs for *days* at any N ≥ a few
/// hundred. We pin at 2,000 so the pre-fix run wedges unambiguously while the
/// post-fix run completes fast enough for CI. The SHAPE (384 separate scalar
/// fibers) is the exact production trigger and is unchanged.
const WEDGE_RECORDS: usize = 2_000;

// ── fixture builders ────────────────────────────────────────────────────

/// Schema mirroring `marcella_source_embeddings_bge_v2`:
///   * one numeric BASE field `id` (sequential → arithmetic-detected),
///   * one high-variance numeric fiber `ts` (NOT arithmetic),
///   * `n_scalar` SEPARATE numeric fibers `v0..v{n-1}`.
///
/// This is the shape that wedges. It is deliberately NOT a single
/// `Value::Vector` fiber (that shape early-returns out of the cubic and
/// is already covered by `encoder_high_dim_smoke::smoke_bge`).
fn wide_scalar_schema(name: &str, n_scalar: usize) -> BundleSchema {
    let mut schema = BundleSchema::new(name)
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::numeric("ts"));
    for i in 0..n_scalar {
        schema = schema.fiber(FieldDef::numeric(&format!("v{i}")));
    }
    schema
}

/// One record with `id`, a high-variance `ts`, and `n_scalar` pseudo-random
/// float fibers. Values are deterministic (LCG) so the fixture is
/// reproducible; their magnitudes are irrelevant to the encoder's cost.
fn wide_record(id: i64, n_scalar: usize) -> Record {
    let mut r = Record::new();
    r.insert("id".into(), Value::Integer(id));
    // `ts` perturbs a linear step by `id % 7` so it is NOT a clean
    // arithmetic progression — it survives Phase-1 arithmetic detection
    // and remains a numeric candidate, exactly like a real timestamp.
    r.insert("ts".into(), Value::Integer(1_700_000_000 + id * 37 + (id % 7)));
    let mut s = (id as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    for i in 0..n_scalar {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let f = ((s >> 32) as i32 as f64) / (i32::MAX as f64);
        r.insert(format!("v{i}"), Value::Float(f));
    }
    r
}

// ── wall-clock guard ────────────────────────────────────────────────────

enum SnapOutcome {
    /// Snapshot returned; engine handed back so the caller can reopen/drop.
    Done {
        engine: Engine,
        records: usize,
        elapsed: Duration,
    },
    /// Guard fired before the snapshot returned. The snapshot thread is
    /// DETACHED and still burning CPU — this is the wedge signature.
    Wedged,
    Failed(String),
}

/// Run `engine.snapshot_with_report()` on a worker thread and join it with
/// a wall-clock deadline. `recv_timeout` returns the instant the snapshot
/// finishes, so the GREEN path is fast; only a genuine wedge waits the
/// full `guard`. On success the engine is returned (moved back) so there
/// is no drop race with a subsequent reopen.
fn snapshot_guarded(engine: Engine, guard: Duration) -> SnapOutcome {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut engine = engine;
        let start = Instant::now();
        let res = engine.snapshot_with_report();
        let elapsed = start.elapsed();
        let _ = tx.send(match res {
            Ok(report) => Ok((engine, report.total_records_written, elapsed)),
            Err(e) => Err(e.to_string()),
        });
    });
    match rx.recv_timeout(guard) {
        Ok(Ok((engine, records, elapsed))) => SnapOutcome::Done {
            engine,
            records,
            elapsed,
        },
        Ok(Err(e)) => SnapOutcome::Failed(e),
        Err(_) => SnapOutcome::Wedged,
    }
}

fn build_engine(name: &str, n_scalar: usize, n_records: usize) -> Engine {
    let mut engine = Engine::open_memory().expect("open_memory");
    engine
        .create_bundle(wide_scalar_schema(name, n_scalar))
        .expect("create_bundle");
    let t = Instant::now();
    for id in 0..n_records as i64 {
        engine
            .insert(name, &wide_record(id, n_scalar))
            .expect("insert");
    }
    // Inserts are plain WAL appends; if THIS regresses the test would be
    // measuring the wrong thing, so fail loudly here instead.
    assert!(
        t.elapsed().as_secs() < 120,
        "insert of {n_records}×{n_scalar} took {:?} — WAL-append regression, not the encoder wedge",
        t.elapsed()
    );
    engine
}

// ── tests ───────────────────────────────────────────────────────────────

/// **THE WEDGE PIN (RED → GREEN).**
///
/// Production shape: 384 SEPARATE scalar fibers (+ id + ts). Against the
/// pre-fix encoder the snapshot never returns within the guard — the
/// O(F³·N) computed-field detection runs inside a single `StreamEncoder::new`
/// and the between-records snapshot timeout cannot interrupt it. Against the
/// fixed encoder (cached column extraction + wide-bundle detection skip) it
/// completes well within the guard.
#[test]
fn high_field_count_snapshot_completes_within_budget() {
    let engine = build_engine("wedge_v2", 384, WEDGE_RECORDS);

    match snapshot_guarded(engine, Duration::from_secs(WEDGE_GUARD_SECS)) {
        SnapOutcome::Done { records, elapsed, .. } => {
            assert_eq!(
                records, WEDGE_RECORDS,
                "snapshot must persist every record"
            );
            // The fix removes the O(F³·N) computed-field cube; what remains
            // is the encoder's inherent O(F·N) work, which completes. Assert
            // it lands under the guard with margin — a partial regression
            // that re-introduces cubic cost would blow past this.
            assert!(
                elapsed.as_secs() < WEDGE_GUARD_SECS,
                "snapshot completed but took {elapsed:?} — cubic cost may be creeping back"
            );
            eprintln!(
                "wedge_v2 ({WEDGE_RECORDS}×384 scalar) snapshot completed in {elapsed:?}"
            );
        }
        SnapOutcome::Wedged => panic!(
            "WEDGE REPRODUCED: {WEDGE_RECORDS}×384-scalar-field snapshot did not complete \
             within {WEDGE_GUARD_SECS}s. The O(F³·N) computed-field detection in \
             dhoom::encode_bundle Phase 2 ran inside a single StreamEncoder::new and \
             the between-records timeout could not interrupt it."
        ),
        SnapOutcome::Failed(e) => panic!("snapshot errored: {e}"),
    }
}

/// **N-INDEPENDENT CUBIC DISCRIMINATOR.**
///
/// The pre-fix cost had two parts: an O(F³·N) column re-fetch AND, even with
/// cached columns, an O(F³) op×a×b *enumeration* that does not depend on N
/// (385³ ≈ 5.7e7 triples ≈ 14s regardless of record count). This test uses a
/// TINY record count so the encoder's inherent O(F·N) work is negligible,
/// isolating the cubic-in-field-count term. Post-fix (wide-bundle detection
/// skipped) it is well under a second. If the field-count guard is ever
/// removed but caching kept, this fails at ~14s; if caching is also removed,
/// it wedges — either way the regression is caught cheaply.
#[test]
fn wide_bundle_encode_is_not_cubic_in_field_count() {
    let engine = build_engine("cubic_probe", 384, 150);

    match snapshot_guarded(engine, Duration::from_secs(WEDGE_GUARD_SECS)) {
        SnapOutcome::Done { records, elapsed, .. } => {
            assert_eq!(records, 150);
            assert!(
                elapsed.as_secs() < 4,
                "384-field / 150-record snapshot took {elapsed:?} — the O(F³) \
                 computed-field enumeration (N-independent) has crept back"
            );
            eprintln!("cubic_probe (150×384 scalar) snapshot completed in {elapsed:?}");
        }
        SnapOutcome::Wedged => panic!(
            "WEDGE: 384-field encode did not complete within {WEDGE_GUARD_SECS}s even at \
             150 records — the O(F³·N) computed-field re-fetch is back."
        ),
        SnapOutcome::Failed(e) => panic!("snapshot errored: {e}"),
    }
}

/// **BISECTION CONTROL — field count is the cause.**
///
/// SAME 10,000 records, but only 8 scalar fibers. If the wedge were about
/// record count this would also wedge; it does not. It completes in well
/// under a second both pre- and post-fix, isolating field count as the
/// driver (O(F³·N): 8 fields is trivial, 384 explodes).
#[test]
fn low_field_count_snapshot_is_fast() {
    let engine = build_engine("control_8", 8, 10_000);

    match snapshot_guarded(engine, Duration::from_secs(WEDGE_GUARD_SECS)) {
        SnapOutcome::Done { records, elapsed, .. } => {
            assert_eq!(records, 10_000);
            assert!(
                elapsed.as_secs() < 10,
                "8-field / 10k-record snapshot took {elapsed:?} — expected sub-second"
            );
            eprintln!("control_8 (10k×8 scalar) snapshot completed in {elapsed:?}");
        }
        SnapOutcome::Wedged => panic!(
            "control (8 fields) unexpectedly wedged — the hang is NOT purely field-count?"
        ),
        SnapOutcome::Failed(e) => panic!("snapshot errored: {e}"),
    }
}

fn cover(engine: &mut Engine, bundle: &str) -> Vec<Record> {
    let stmt = parse(&format!("COVER {bundle}")).expect("parse COVER");
    match execute(engine, &stmt).expect("execute COVER") {
        ExecResult::Rows(r) => r,
        other => panic!("expected Rows from COVER, got {other:?}"),
    }
}

/// **ROUND-TRIP INTEGRITY (GREEN half of task 6).**
///
/// A fix that completes but corrupts the snapshot is worse than the hang. This
/// writes a wide-numeric bundle through the same boot path (`snapshot_with_report`),
/// then reopens the `.dhoom` in the fast-mmap mode the boot upgrades to and
/// asserts the SAME record count AND the SAME field set come back — proving the
/// Phase-2 skip (fields emitted as plain variable columns) is read-compatible
/// and loses no data.
#[test]
fn roundtrip_wide_bundle_reopens_in_mmap_with_same_count_and_fields() {
    let n = 2_000usize;
    let td = tempfile::tempdir().expect("tempdir");
    let dir = td.path().to_path_buf();

    let mut engine = Engine::open(&dir).expect("open");
    engine
        .create_bundle(wide_scalar_schema("rt_v2", 384))
        .expect("create_bundle");
    for id in 0..n as i64 {
        engine.insert("rt_v2", &wide_record(id, 384)).expect("insert");
    }

    // Snapshot through the boot path, guarded so a regression fails loudly
    // instead of hanging the whole suite.
    let engine = match snapshot_guarded(engine, Duration::from_secs(WEDGE_GUARD_SECS)) {
        SnapOutcome::Done {
            engine,
            records,
            elapsed,
        } => {
            assert_eq!(records, n, "snapshot must persist all {n} records");
            eprintln!("rt_v2 snapshot completed in {elapsed:?}");
            engine
        }
        SnapOutcome::Wedged => panic!("round-trip snapshot wedged before writing .dhoom"),
        SnapOutcome::Failed(e) => panic!("round-trip snapshot errored: {e}"),
    };
    drop(engine); // flush the WAL writer on the main thread before reopening

    // Reopen in the fast-mmap mode the boot path upgrades to after a clean snapshot.
    let mut mmap = Engine::open_mmap(&dir).expect("open_mmap");
    let rows = cover(&mut mmap, "rt_v2");

    // (1) Record-count integrity.
    assert_eq!(
        rows.len(),
        n,
        "mmap reopen must expose the same record count the snapshot wrote"
    );

    // (2) Field-set integrity: every field survives the .dhoom round-trip.
    let sample = &rows[0];
    assert!(
        sample.contains_key("id"),
        "base field `id` missing after round-trip"
    );
    assert!(
        sample.contains_key("ts"),
        "fiber `ts` missing after round-trip"
    );
    for i in 0..384 {
        let f = format!("v{i}");
        assert!(
            sample.contains_key(&f),
            "fiber `{f}` missing after mmap round-trip — the Phase-2 skip dropped a field"
        );
    }
    assert_eq!(
        sample.len(),
        386,
        "round-tripped record has {} fields, expected 386 (id + ts + v0..v383)",
        sample.len()
    );

    drop(mmap);
    drop(td);
}
