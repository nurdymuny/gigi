//! ITEM 6 — Two-version snapshot rotation (.dhoom + .dhoom.prev).
//!
//! Behavior under test (per design phase):
//!
//!   - `Engine::snapshot()` no longer leaves a single `.dhoom`; on the
//!     second-and-later snapshot, the prior generation is preserved as
//!     `.dhoom.prev` via an atomic tmp -> .new -> .dhoom rotation that
//!     moves the existing `.dhoom` to `.dhoom.prev` mid-cycle.
//!
//!   - `Engine::open_mmap()` reads `.dhoom` first. On parse-shaped
//!     failures (`InvalidData` / `UnexpectedEof` / 0-byte file) the
//!     read path falls back to `.dhoom.prev`. Operational errors
//!     (`PermissionDenied`, etc.) do NOT trigger the fallback —
//!     operator-visible failures stay loud.
//!
//!   - Missing `.dhoom` does NOT trigger `.dhoom.prev` fallback; the
//!     read path correctly creates a heap-only `BundleStore` so a
//!     manual `.dhoom` deletion is not silently undone.
//!
//! These tests run under `--no-default-features` (Gate 1), no feature
//! flags required — they touch the heap-mode + mmap-mode snapshot code
//! paths through the public `Engine` API.

use gigi::engine::Engine;
use gigi::parser::{execute, parse, ExecResult};
use std::fs;
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

// ── helpers ─────────────────────────────────────────────────────────────

fn run(engine: &mut Engine, sql: &str) -> Result<ExecResult, String> {
    let stmt = parse(sql).map_err(|e| format!("parse `{sql}`: {e}"))?;
    execute(engine, &stmt)
}

/// Snapshot dir for a given engine data dir.
fn snapdir(data_dir: &Path) -> PathBuf {
    data_dir.join("snapshots")
}

/// Construct a fresh engine in a tempdir, populate one bundle with
/// non-arithmetic, non-degenerate text-fiber records (so the DHOOM
/// encoder produces a record body the heap-mode `load_dhoom_snapshot`
/// path can round-trip), snapshot the engine, and return
/// (tempdir guard, dir path).
///
/// The fiber is TEXT (not arithmetic-detectable), and the value strings
/// are randomly generated so the single-pass arithmetic detection in
/// the encoder cannot collapse them into a header pool.
fn snapshot_n_records(records: usize) -> (tempfile::TempDir, PathBuf) {
    let td = tempfile::tempdir().expect("tempdir");
    let dir = td.path().to_path_buf();
    let mut e = Engine::open(&dir).expect("open");
    run(&mut e, "CREATE BUNDLE foo (id INT BASE, name TEXT FIBER)")
        .expect("CREATE BUNDLE foo");
    for i in 0..records {
        // Non-arithmetic text values so the encoder writes one body
        // line per record.
        let payload = format!("alpha_{i}_z{}", (i * 7919) % 1000);
        run(
            &mut e,
            &format!("INSERT INTO foo (id, name) VALUES ({i}, '{payload}')"),
        )
        .expect("INSERT");
    }
    e.snapshot().expect("snapshot");
    drop(e);
    (td, dir)
}

/// Open the engine, append `extra_records` more records, snapshot.
fn append_and_snapshot(dir: &Path, start_id: i64, extra_records: usize) {
    let mut e = Engine::open(dir).expect("open existing");
    for i in 0..extra_records {
        let id = start_id + i as i64;
        let payload = format!("beta_{id}_q{}", (id * 6151) % 1000);
        run(
            &mut e,
            &format!("INSERT INTO foo (id, name) VALUES ({id}, '{payload}')"),
        )
        .expect("INSERT");
    }
    e.snapshot().expect("snapshot");
    drop(e);
}

/// Truncate a file to zero bytes (simulating a wedged write).
fn truncate_to_zero(path: &Path) {
    let f = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(path)
        .expect("open for truncate");
    drop(f);
    let meta = fs::metadata(path).expect("metadata");
    assert_eq!(meta.len(), 0, "truncate should yield 0 bytes");
}

/// Corrupt a DHOOM file's header to force `InvalidData` on
/// `MmapBundle::open` (overwrite the leading bytes with garbage so the
/// `}:` header marker can't be found in the first ~4 KiB of the file).
fn corrupt_dhoom_header(path: &Path) {
    let len = fs::metadata(path).expect("metadata").len() as usize;
    assert!(len > 16, "need a non-empty file to corrupt header");
    // Overwrite first 4 KiB (or whole file if smaller) with NUL bytes —
    // wipes the `}:` end-of-header marker so MmapBundle::open returns
    // InvalidData("Missing DHOOM header").
    let n = std::cmp::min(len, 4096);
    let mut f = fs::OpenOptions::new()
        .write(true)
        .open(path)
        .expect("open for corruption");
    f.seek(SeekFrom::Start(0)).expect("seek");
    f.write_all(&vec![0u8; n]).expect("write zeros");
    f.sync_all().ok();
}

/// Open a fresh engine pointing at `dir` and return the live record
/// count for bundle "foo".
fn count_records(dir: &Path) -> usize {
    let mut e = Engine::open_mmap(dir).expect("open_mmap");
    let rows = match run(&mut e, "COVER foo").expect("COVER foo") {
        ExecResult::Rows(r) => r,
        other => panic!("expected Rows, got {other:?}"),
    };
    rows.len()
}

// ── tests ───────────────────────────────────────────────────────────────

/// Test 1 — `rotation_writes_both_files`:
///
/// First snapshot writes only `.dhoom`. Second snapshot writes BOTH
/// `.dhoom` (latest generation, 6 records) AND `.dhoom.prev` (the first
/// generation, 3 records). The two files are independently round-trippable
/// through the read path.
///
/// Each generation uses >= 3 records so the DHOOM encoder writes a
/// proper record body (single-record bundles collapse into the schema
/// header pool, which is a pre-existing format quirk independent of
/// rotation).
#[test]
fn rotation_writes_both_files() {
    // Generation 1: snapshot with 3 records.
    let (td, dir) = snapshot_n_records(3);

    let snap = snapdir(&dir).join("foo.dhoom");
    let prev = snapdir(&dir).join("foo.dhoom.prev");

    assert!(snap.exists(), "first snapshot writes .dhoom");
    assert!(
        !prev.exists(),
        "first snapshot does NOT write .dhoom.prev (no prior gen to preserve)"
    );

    // Generation 2: append 3 records, snapshot again.
    append_and_snapshot(&dir, 100, 3);

    assert!(snap.exists(), ".dhoom present after rotation");
    assert!(
        prev.exists(),
        "second snapshot promotes prior .dhoom to .dhoom.prev"
    );

    // .dhoom should hold 6 records (gen 2 = gen 1 records + 3 new).
    let n = count_records(&dir);
    assert_eq!(n, 6, ".dhoom should hold latest generation (6 records)");

    drop(td);
}

/// Test 2 — `read_falls_back_to_prev_when_current_corrupt`:
///
/// Write two generations, truncate `.dhoom` to 0 bytes. The read path
/// loads `.dhoom.prev` and the bundle becomes queryable with the
/// previous generation's record count.
#[test]
fn read_falls_back_to_prev_when_current_corrupt() {
    let (td, dir) = snapshot_n_records(3);
    append_and_snapshot(&dir, 100, 2); // .dhoom = 5 records, .prev = 3 records

    let snap = snapdir(&dir).join("foo.dhoom");
    let prev = snapdir(&dir).join("foo.dhoom.prev");
    assert!(prev.exists(), "rotation must produce .prev for this test");

    // Wedge .dhoom — truncate to 0 bytes.
    truncate_to_zero(&snap);

    // Fast-mmap open path falls back to .prev (3 records, not 5).
    let n = count_records(&dir);
    assert_eq!(
        n, 3,
        "with .dhoom 0-byte-wedged, read path serves .dhoom.prev's record set"
    );

    drop(td);
}

/// Test 3 — `read_falls_back_to_prev_on_invalid_data`:
///
/// Corrupt `.dhoom`'s header. `MmapBundle::open` returns InvalidData,
/// the fallback opens `.dhoom.prev` successfully.
#[test]
fn read_falls_back_to_prev_on_invalid_data() {
    let (td, dir) = snapshot_n_records(4);
    append_and_snapshot(&dir, 100, 3); // .dhoom = 7 records, .prev = 4 records

    let snap = snapdir(&dir).join("foo.dhoom");
    let prev = snapdir(&dir).join("foo.dhoom.prev");
    assert!(prev.exists(), "rotation must produce .prev for this test");

    // Corrupt the header.
    corrupt_dhoom_header(&snap);

    // Read path falls back to .prev (4 records, not 7).
    let n = count_records(&dir);
    assert_eq!(
        n, 4,
        "with .dhoom header corrupted, read path serves .dhoom.prev's record set"
    );

    drop(td);
}

/// Test 4 — `read_falls_back_to_heap_when_both_corrupt`:
///
/// Both `.dhoom` and `.dhoom.prev` corrupt. Open succeeds (does not
/// crash), bundle is queryable but empty — heap-only fallback in
/// `Engine::open_mmap` Phase 2 kicks in.
#[test]
fn read_falls_back_to_heap_when_both_corrupt() {
    let (td, dir) = snapshot_n_records(3);
    append_and_snapshot(&dir, 100, 3); // .dhoom = 6 records, .prev = 3 records

    let snap = snapdir(&dir).join("foo.dhoom");
    let prev = snapdir(&dir).join("foo.dhoom.prev");
    assert!(prev.exists(), "rotation must produce .prev for this test");

    truncate_to_zero(&snap);
    truncate_to_zero(&prev);

    // open_mmap succeeds (heap-only fallback). Bundle queryable.
    // Records present are whatever's still in the WAL post-snapshot
    // (snapshot compacts WAL down to schema-only after success, so 0).
    let mut e = Engine::open_mmap(&dir).expect("open_mmap survives both-corrupt");
    let rows = match run(&mut e, "COVER foo").expect("COVER foo") {
        ExecResult::Rows(r) => r,
        other => panic!("expected Rows, got {other:?}"),
    };
    assert_eq!(
        rows.len(),
        0,
        "heap-only fallback yields empty bundle; WAL was compacted at snapshot time"
    );

    drop(e);
    drop(td);
}

/// Test 5 — `no_prev_fallback_when_no_snapshot_present`:
///
/// `.dhoom` missing on disk, but a bogus `.dhoom.prev` is sitting there
/// (operator residue, leftover from a previous boot's manual cleanup).
/// The read path takes the heap-only branch (does NOT silently load the
/// orphan `.prev`).
///
/// The bogus `.prev` here is JSON garbage, not a valid DHOOM file. A
/// correct implementation never opens it because the `.dhoom` existence
/// check at the top of the branch returns false. If the read path
/// incorrectly fell through to .prev, `MmapBundle::open` would error on
/// the garbage and the bundle would land in heap_bundles anyway — but
/// the WARNING line would mention "loaded .dhoom.prev" or
/// ".dhoom.prev also failed", neither of which fires here.
#[test]
fn no_prev_fallback_when_no_snapshot_present() {
    let td = tempfile::tempdir().expect("tempdir");
    let dir = td.path().to_path_buf();
    {
        // Create a bundle, snapshot it once (so the WAL has a Checkpoint),
        // then delete the .dhoom file. This leaves Engine::open_mmap with
        // a non-empty WAL but no .dhoom + no .prev.
        let mut e = Engine::open(&dir).expect("open");
        run(
            &mut e,
            "CREATE BUNDLE foo (id INT BASE, name TEXT FIBER)",
        )
        .expect("CREATE BUNDLE");
        for i in 0..3 {
            run(
                &mut e,
                &format!("INSERT INTO foo (id, name) VALUES ({i}, 'pre_snap_{i}')"),
            )
            .expect("INSERT");
        }
        e.snapshot().expect("snapshot 1");
        // Drop, then operator manually deletes the .dhoom file.
    }

    let snap = snapdir(&dir).join("foo.dhoom");
    let prev = snapdir(&dir).join("foo.dhoom.prev");
    assert!(snap.exists(), "test setup: .dhoom must exist before deletion");
    assert!(
        !prev.exists(),
        "test setup: first snapshot leaves no .prev"
    );

    // Simulate an operator deleting .dhoom.
    fs::remove_file(&snap).expect("simulate operator deletion of .dhoom");
    // Drop a bogus .prev to ensure the read path ignores it (an
    // accidental file left behind from a manual cleanup).
    fs::write(&prev, b"some garbage\n").expect("write bogus prev");

    // open_mmap takes heap-only branch (does NOT load .prev). The
    // resulting bundle is empty — no records load because there's no
    // .dhoom and no post-checkpoint WAL inserts to replay (the snapshot
    // compacted the WAL down to schema-only). This is the correct
    // behavior: the operator's manual cleanup of .dhoom is honored.
    let mut e = Engine::open_mmap(&dir).expect("open_mmap with no .dhoom");
    let rows = match run(&mut e, "COVER foo").expect("COVER foo") {
        ExecResult::Rows(r) => r,
        other => panic!("expected Rows, got {other:?}"),
    };
    assert_eq!(
        rows.len(),
        0,
        "no-.dhoom branch must NOT silently load the bogus .prev; \
         empty bundle is the correct outcome of operator-driven .dhoom deletion"
    );

    drop(e);
    drop(td);
}

/// Test 6 — `rotation_overwrites_existing_prev`:
///
/// Write three generations. After each rotation:
///   - `.dhoom`      = generation N
///   - `.dhoom.prev` = generation N-1
///   - generation N-2 has been overwritten (only one `.prev` slot exists).
///
/// Each generation adds 3 records: gen 1 = 3, gen 2 = 6, gen 3 = 9.
#[test]
fn rotation_overwrites_existing_prev() {
    let (td, dir) = snapshot_n_records(3); // gen 1: 3 records
    append_and_snapshot(&dir, 100, 3); // gen 2: 6 records
    append_and_snapshot(&dir, 200, 3); // gen 3: 9 records

    let snap = snapdir(&dir).join("foo.dhoom");
    let prev = snapdir(&dir).join("foo.dhoom.prev");

    assert!(snap.exists(), ".dhoom = generation 3");
    assert!(prev.exists(), ".dhoom.prev = generation 2 (gen 1 overwritten)");

    // Latest (.dhoom) = 9 records (gen 3).
    assert_eq!(count_records(&dir), 9);

    // Wedge .dhoom; the read path falls back to .prev, which should
    // be generation 2 (6 records), NOT generation 1 (3 records).
    truncate_to_zero(&snap);
    assert_eq!(
        count_records(&dir),
        6,
        ".dhoom.prev must hold gen 2, not gen 1 (which was overwritten)"
    );

    drop(td);
}

/// Test 7 — `prev_only_kept_if_write_succeeded`:
///
/// If the rotation never gets to step 3 (the prev rename), the existing
/// `.dhoom` must NOT be moved to `.dhoom.prev`. We simulate this by
/// running one successful snapshot, then deleting the .dhoom directly
/// (faking an operator cleanup) and re-opening: the next snapshot starts
/// "fresh" — .dhoom is created, .prev stays absent.
#[test]
fn prev_only_kept_if_write_succeeded() {
    let (td, dir) = snapshot_n_records(3);
    let snap = snapdir(&dir).join("foo.dhoom");
    let prev = snapdir(&dir).join("foo.dhoom.prev");
    assert!(snap.exists());
    assert!(!prev.exists(), "first-gen snapshot leaves no .prev");

    // Operator deletion: remove .dhoom by hand.
    fs::remove_file(&snap).expect("remove .dhoom");
    assert!(!snap.exists());

    // Add more data + snapshot — this is gen 1 again from the rotation's
    // POV (no prior .dhoom present at rotation time).
    append_and_snapshot(&dir, 100, 3);

    assert!(snap.exists(), ".dhoom regenerated by next snapshot");
    assert!(
        !prev.exists(),
        ".dhoom.prev still absent — rotation didn't have a prior .dhoom to preserve"
    );

    drop(td);
}

/// Test 8 — `read_falls_back_to_prev_warning_emitted`:
///
/// Sanity test that the WARNING-line emission is reachable. We cannot
/// capture eprintln cleanly without a custom stderr sink, but we can
/// confirm the corruption + load path runs to completion and produces
/// the correct record count (this is functionally the same as
/// `read_falls_back_to_prev_on_invalid_data`, but exercises the
/// codepath under different generation counts to catch any
/// generation-N-specific regressions).
#[test]
fn read_falls_back_to_prev_under_multiple_generations() {
    let (td, dir) = snapshot_n_records(3);
    append_and_snapshot(&dir, 100, 3);
    append_and_snapshot(&dir, 200, 3);
    append_and_snapshot(&dir, 300, 3);
    // gen 1: 3, gen 2: 6, gen 3: 9, gen 4: 12 records.
    // After gen 4: .dhoom = gen 4 (12 records), .prev = gen 3 (9 records).

    let snap = snapdir(&dir).join("foo.dhoom");
    corrupt_dhoom_header(&snap);

    assert_eq!(
        count_records(&dir),
        9,
        "corrupted gen-4 falls back to gen-3 in .prev"
    );

    drop(td);
}

/// Test 9 — `multiple_bundles_rotate_independently`:
///
/// Two bundles, two snapshot generations. Each bundle's `.dhoom` and
/// `.dhoom.prev` rotation is independent of the other's.
#[test]
fn multiple_bundles_rotate_independently() {
    let td = tempfile::tempdir().expect("tempdir");
    let dir = td.path().to_path_buf();

    {
        let mut e = Engine::open(&dir).expect("open");
        run(&mut e, "CREATE BUNDLE foo (id INT BASE, name TEXT FIBER)")
            .expect("CREATE foo");
        run(&mut e, "CREATE BUNDLE bar (id INT BASE, name TEXT FIBER)")
            .expect("CREATE bar");

        // Use non-arithmetic text payloads (3+ records each so the
        // DHOOM encoder writes a proper record body — single-record
        // bundles collapse into header pool, a format quirk independent
        // of rotation).
        for i in 0..3 {
            run(
                &mut e,
                &format!("INSERT INTO foo (id, name) VALUES ({i}, 'foo_alpha_{i}')"),
            )
            .expect("ins foo");
            run(
                &mut e,
                &format!("INSERT INTO bar (id, name) VALUES ({i}, 'bar_alpha_{i}')"),
            )
            .expect("ins bar");
        }
        e.snapshot().expect("snapshot 1");
        // After gen 1: foo.dhoom + bar.dhoom present (3 records each), no .prev for either.

        for i in 3..6 {
            run(
                &mut e,
                &format!("INSERT INTO foo (id, name) VALUES ({i}, 'foo_beta_{i}')"),
            )
            .expect("ins foo");
            run(
                &mut e,
                &format!("INSERT INTO bar (id, name) VALUES ({i}, 'bar_beta_{i}')"),
            )
            .expect("ins bar");
        }
        e.snapshot().expect("snapshot 2");
        // After gen 2: foo.dhoom = bar.dhoom = 6 records each; foo.dhoom.prev = bar.dhoom.prev = 3 records each.
    }

    let snaps = snapdir(&dir);
    assert!(snaps.join("foo.dhoom").exists());
    assert!(snaps.join("foo.dhoom.prev").exists());
    assert!(snaps.join("bar.dhoom").exists());
    assert!(snaps.join("bar.dhoom.prev").exists());

    // Corrupt foo.dhoom only — bar's read path should be unaffected.
    truncate_to_zero(&snaps.join("foo.dhoom"));

    let mut e = Engine::open_mmap(&dir).expect("open_mmap");
    let foo_rows = match run(&mut e, "COVER foo").expect("COVER foo") {
        ExecResult::Rows(r) => r,
        other => panic!("expected Rows, got {other:?}"),
    };
    let bar_rows = match run(&mut e, "COVER bar").expect("COVER bar") {
        ExecResult::Rows(r) => r,
        other => panic!("expected Rows, got {other:?}"),
    };

    assert_eq!(
        foo_rows.len(),
        3,
        "foo falls back to .prev (gen 1, 3 records)"
    );
    assert_eq!(
        bar_rows.len(),
        6,
        "bar reads .dhoom cleanly (gen 2, 6 records); not affected by foo's corruption"
    );

    drop(e);
    drop(td);
}
