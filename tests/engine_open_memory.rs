//! AURORA Phase 3 (Rory 2026-06-22) — `Engine::open_memory()` constructor.
//!
//! Replaces the `tempdir() + Engine::open_empty(td.path())?` boilerplate that
//! every dev/CI/test harness was carrying. The new constructor owns its
//! tempdir, cleans up on Drop, and is feature-flag-agnostic — these tests
//! therefore run under `--no-default-features` and never reach for any of
//! the feature-gated surfaces.
//!
//! Gates:
//!   1. open_memory_succeeds        — constructor returns Ok with a live data_dir
//!   2. basic_operations_work       — CREATE BUNDLE + INSERT + COVER round-trips
//!   3. no_persistence_between_runs — each open_memory() gets a fresh tempdir;
//!                                    a second instance does NOT see the first
//!                                    instance's bundle
//!   4. concurrent_instances_isolated — two simultaneous instances do not
//!                                       cross-contaminate
//!   5. tempdir_cleaned_up_on_drop  — data_dir() path is removed after the
//!                                     Engine value is dropped (Drop gate)
//!   6. drop_safe_with_open_wal     — Windows field-drop-order gate: WAL
//!                                     handle must close before TempDir is
//!                                     removed; tests INSERT then drop

use gigi::engine::Engine;
use gigi::parser::{execute, parse, ExecResult};
use gigi::types::{Record, Value};

fn run(engine: &mut Engine, sql: &str) -> Result<ExecResult, String> {
    let stmt = parse(sql).map_err(|e| format!("parse `{sql}`: {e}"))?;
    execute(engine, &stmt)
}

fn id_of(row: &Record) -> i64 {
    match row.get("id") {
        Some(Value::Integer(i)) => *i,
        _ => -1,
    }
}

#[test]
fn test_open_memory_succeeds() {
    let engine = Engine::open_memory().expect("open_memory() must succeed");
    let dir = engine.data_dir().to_path_buf();
    assert!(
        dir.exists(),
        "data_dir() must point at a live directory while the engine is alive: {dir:?}"
    );
}

#[test]
fn test_open_memory_basic_operations_work() {
    let mut engine = Engine::open_memory().expect("open_memory()");

    run(
        &mut engine,
        "CREATE BUNDLE foo (id INT BASE, name TEXT FIBER)",
    )
    .expect("CREATE BUNDLE foo");

    run(
        &mut engine,
        "INSERT INTO foo (id, name) VALUES (1, 'alpha')",
    )
    .expect("INSERT 1");
    run(
        &mut engine,
        "INSERT INTO foo (id, name) VALUES (2, 'beta')",
    )
    .expect("INSERT 2");

    let result = run(&mut engine, "COVER foo").expect("COVER foo");
    let rows = match result {
        ExecResult::Rows(r) => r,
        other => panic!("expected Rows, got {other:?}"),
    };
    assert_eq!(rows.len(), 2, "expected 2 records, got {}", rows.len());
    let mut ids: Vec<i64> = rows.iter().map(id_of).collect();
    ids.sort();
    assert_eq!(ids, vec![1, 2], "ids round-trip");
}

#[test]
fn test_open_memory_no_persistence_between_instances() {
    // Instance 1: create bundle, insert, drop.
    {
        let mut e1 = Engine::open_memory().expect("open_memory #1");
        run(
            &mut e1,
            "CREATE BUNDLE persist_check (id INT BASE, x INT FIBER)",
        )
        .expect("CREATE persist_check");
        run(
            &mut e1,
            "INSERT INTO persist_check (id, x) VALUES (42, 99)",
        )
        .expect("INSERT");
        // e1 dropped at end of scope; tempdir cleaned up automatically.
    }

    // Instance 2: brand-new tempdir, brand-new engine — bundle must not exist.
    let mut e2 = Engine::open_memory().expect("open_memory #2");
    let cover = run(&mut e2, "COVER persist_check");
    assert!(
        cover.is_err(),
        "second open_memory() instance must NOT see the first instance's bundle, \
         but COVER returned: {cover:?}"
    );
}

#[test]
fn test_open_memory_concurrent_instances_isolated() {
    // Two simultaneous engines — each gets its own tempdir; writes do not leak.
    let mut a = Engine::open_memory().expect("open_memory a");
    let mut b = Engine::open_memory().expect("open_memory b");

    assert_ne!(
        a.data_dir(),
        b.data_dir(),
        "each open_memory() must allocate a distinct tempdir"
    );

    run(&mut a, "CREATE BUNDLE only_a (id INT BASE, v INT FIBER)")
        .expect("CREATE only_a");
    run(&mut a, "INSERT INTO only_a (id, v) VALUES (1, 100)").expect("ins a");

    run(&mut b, "CREATE BUNDLE only_b (id INT BASE, v INT FIBER)")
        .expect("CREATE only_b");
    run(&mut b, "INSERT INTO only_b (id, v) VALUES (1, 200)").expect("ins b");

    // a does not see b's bundle, b does not see a's.
    assert!(
        run(&mut a, "COVER only_b").is_err(),
        "engine a must NOT see only_b"
    );
    assert!(
        run(&mut b, "COVER only_a").is_err(),
        "engine b must NOT see only_a"
    );

    // and each sees its own data correctly.
    let rows_a = match run(&mut a, "COVER only_a").expect("COVER only_a") {
        ExecResult::Rows(r) => r,
        other => panic!("expected Rows, got {other:?}"),
    };
    assert_eq!(rows_a.len(), 1);

    let rows_b = match run(&mut b, "COVER only_b").expect("COVER only_b") {
        ExecResult::Rows(r) => r,
        other => panic!("expected Rows, got {other:?}"),
    };
    assert_eq!(rows_b.len(), 1);
}

#[test]
fn test_open_memory_tempdir_cleaned_up_on_drop() {
    // Capture data_dir before dropping.
    let dir = {
        let engine = Engine::open_memory().expect("open_memory()");
        let d = engine.data_dir().to_path_buf();
        assert!(d.exists(), "tempdir alive while engine alive");
        d
        // engine drops here — TempDir::drop should remove the directory.
    };

    // After drop the path should no longer exist on disk.
    // (TempDir::drop is best-effort and ignores errors per std semantics, so
    // a stale directory here means we hit a Windows file-handle leak — that's
    // exactly what we want this gate to catch.)
    assert!(
        !dir.exists(),
        "tempdir at {dir:?} must be removed when Engine is dropped"
    );
}

#[test]
fn test_open_memory_drop_safe_with_open_wal_handle() {
    // Field-drop-order gate. On Windows, dropping a TempDir while the WAL
    // file handle is still open errors silently inside TempDir::drop and
    // leaves a stale directory on disk. The struct layout in engine.rs
    // declares `_tempdir` LAST so it is dropped LAST — after wal: WalWriter
    // has already closed the file. This test inserts a row (forcing WAL
    // writes) then drops the engine; if the field-drop order ever regresses,
    // the cleanup gate below will fail.
    let dir = {
        let mut engine = Engine::open_memory().expect("open_memory()");
        run(
            &mut engine,
            "CREATE BUNDLE handle_check (id INT BASE, v INT FIBER)",
        )
        .expect("CREATE handle_check");
        run(
            &mut engine,
            "INSERT INTO handle_check (id, v) VALUES (1, 1)",
        )
        .expect("INSERT");
        let d = engine.data_dir().to_path_buf();
        assert!(d.exists());
        d
        // engine dropped here — must close WAL THEN remove tempdir.
    };

    assert!(
        !dir.exists(),
        "tempdir at {dir:?} must be cleaned up even after WAL writes; \
         a stale directory here means a Windows-style 'file in use' regression"
    );
}
