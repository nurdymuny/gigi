//! ITEM 3 of the SUDOKU 8-item local shipment.
//!
//! Regression guard for the `Engine::open_mmap` per-bundle `.dhoom`
//! graceful-skip path (engine.rs:684, the (a)-bucket site classified
//! in theory/sudoku/SUDOKU_FINDING_3_MMAP_TRIAGE.md).
//!
//! Pre-2026-05-25 behavior was to `continue` on missing/corrupt `.dhoom`,
//! silently dropping the bundle. Bee's 9826f9b fix replaced that with a
//! heap-only `BundleStore` fallback + a human-prose WARNING. ITEM 3
//! hardens that fix by:
//!
//!   1. Stamping a stable, grep-able marker on the WARNING line:
//!      `ITEM-3-MMAP-SKIP bundle={name} err={e}`.
//!   2. Adding this regression test to pin the no-wedge invariant.
//!
//! The failure mode we are guarding against: a 0-byte `.dhoom.tmp`
//! that gets renamed into place (or a manually-truncated `.dhoom`)
//! must not wedge the boot. Schema is still in the WAL; the bundle
//! degrades to a heap-only store and post-checkpoint WAL inserts
//! re-populate it.

use std::fs;

use gigi::engine::Engine;
use gigi::parser::{execute, parse, ExecResult};
use gigi::types::Value;

fn run(engine: &mut Engine, sql: &str) -> Result<ExecResult, String> {
    let stmt = parse(sql).map_err(|e| format!("parse `{sql}`: {e}"))?;
    execute(engine, &stmt)
}

#[test]
fn reopens_with_corrupt_dhoom_via_heap_fallback() {
    // 1. Set up a data dir with TWO bundles, snapshot both, drop the
    //    engine so .dhoom files land on disk.
    let dir = std::env::temp_dir().join("gigi_item3_corrupt_dhoom");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("mkdir data_dir");

    {
        let mut engine = Engine::open(&dir).expect("open engine #1");
        run(
            &mut engine,
            "CREATE BUNDLE corrupt_one (id INT BASE, v INT FIBER)",
        )
        .expect("CREATE corrupt_one");
        run(
            &mut engine,
            "CREATE BUNDLE healthy_two (id INT BASE, v INT FIBER)",
        )
        .expect("CREATE healthy_two");
        for i in 1..=5 {
            run(
                &mut engine,
                &format!("INSERT INTO corrupt_one (id, v) VALUES ({i}, {})", i * 10),
            )
            .expect("INSERT corrupt_one");
            run(
                &mut engine,
                &format!("INSERT INTO healthy_two (id, v) VALUES ({i}, {})", i * 100),
            )
            .expect("INSERT healthy_two");
        }
        // Snapshot writes both .dhoom files.
        engine.snapshot().expect("snapshot engine #1");
    }

    let snap_corrupt = dir.join("snapshots").join("corrupt_one.dhoom");
    let snap_healthy = dir.join("snapshots").join("healthy_two.dhoom");
    assert!(
        snap_corrupt.exists(),
        "corrupt_one.dhoom must exist after snapshot"
    );
    assert!(
        snap_healthy.exists(),
        "healthy_two.dhoom must exist after snapshot"
    );

    // 2. Truncate corrupt_one.dhoom to 0 bytes — emulates the failure
    //    mode where a snapshot writer crashed mid-flight and renamed an
    //    empty .tmp into place, or where /data was force-truncated by
    //    operator intervention.
    fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&snap_corrupt)
        .expect("truncate corrupt_one.dhoom to 0 bytes");
    let meta = fs::metadata(&snap_corrupt).expect("metadata corrupt_one.dhoom");
    assert_eq!(
        meta.len(),
        0,
        "truncation must have left the file at 0 bytes"
    );

    // 3. Reopen the engine through the mmap fast path. This is the
    //    code path that ITEM 3 hardens — the (a)-bucket graceful-skip
    //    at engine.rs:684. Pre-fix this hard-errored. Post-fix AND
    //    post-hardening (the ITEM-3-MMAP-SKIP marker is now stable),
    //    open succeeds: the orphan bundle is downgraded to a heap-only
    //    store and the healthy bundle still loads via mmap.
    let mut engine2 = Engine::open_mmap(&dir)
        .expect("Engine::open_mmap must NOT wedge on corrupt .dhoom");

    // 4. healthy_two must still query — proves the orphan didn't poison
    //    the rest of the registry.
    let healthy_rows = match run(&mut engine2, "COVER healthy_two").expect("COVER healthy_two") {
        ExecResult::Rows(r) => r,
        other => panic!("expected Rows for healthy_two, got {other:?}"),
    };
    assert_eq!(
        healthy_rows.len(),
        5,
        "healthy_two must still hold all 5 records after the orphan-skip"
    );

    // 5. corrupt_one is queryable — the schema is registered (via the
    //    WAL CreateBundle replay) and the heap-only fallback is in
    //    place. The cover should respond without erroring.
    let cover_result = run(&mut engine2, "COVER corrupt_one");
    assert!(
        cover_result.is_ok(),
        "corrupt_one must still be queryable (heap-only fallback), got: {cover_result:?}"
    );

    // 6. Future-proof: a fresh insert into the heap-fallback bundle
    //    must work without error. This proves the BundleStore landed
    //    correctly, not just as a dead placeholder.
    run(
        &mut engine2,
        "INSERT INTO corrupt_one (id, v) VALUES (999, 999)",
    )
    .expect("post-fallback INSERT must succeed");
    let post_rows = match run(&mut engine2, "COVER corrupt_one").expect("COVER corrupt_one") {
        ExecResult::Rows(r) => r,
        other => panic!("expected Rows for corrupt_one, got {other:?}"),
    };
    let post_ids: Vec<i64> = post_rows
        .iter()
        .filter_map(|r| match r.get("id") {
            Some(Value::Integer(i)) => Some(*i),
            _ => None,
        })
        .collect();
    assert!(
        post_ids.contains(&999),
        "the newly-inserted 999 record must round-trip through the heap-only store, got ids: {post_ids:?}"
    );

    drop(engine2);
    let _ = fs::remove_dir_all(&dir);
}

/// Sanity guard: a CLEAN mmap reopen path must not regress.
///
/// If a future refactor accidentally widens the (a)-bucket graceful-skip
/// to also catch healthy-but-different bundles, the smoke test above
/// might still pass while ALL bundles silently degrade to heap-only
/// mode. This second test asserts that on a clean mmap reopen (no
/// corruption staged) the bundle STAYS in mmap mode and reads its
/// full snapshot.
#[test]
fn clean_mmap_reopen_stays_in_mmap_mode() {
    let dir = std::env::temp_dir().join("gigi_item3_clean_reopen");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("mkdir");

    {
        let mut engine = Engine::open(&dir).expect("open #1");
        run(
            &mut engine,
            "CREATE BUNDLE clean_check (id INT BASE, v INT FIBER)",
        )
        .expect("CREATE");
        for i in 1..=7 {
            run(
                &mut engine,
                &format!("INSERT INTO clean_check (id, v) VALUES ({i}, {i})"),
            )
            .expect("INSERT");
        }
        engine.snapshot().expect("snapshot");
    }

    let mut engine2 = Engine::open_mmap(&dir).expect("mmap reopen #2 (clean)");
    let rows = match run(&mut engine2, "COVER clean_check").expect("COVER clean_check") {
        ExecResult::Rows(r) => r,
        other => panic!("expected Rows, got {other:?}"),
    };
    assert_eq!(
        rows.len(),
        7,
        "clean mmap reopen must hold all 7 records — if this fails, the orphan-skip widened or the snapshot path broke"
    );

    drop(engine2);
    let _ = fs::remove_dir_all(&dir);
}
