//! Personal-list #3 (2026-06-22) — GIGI hosting itself.
//!
//! Integration tests for the `__bundles__` virtual bundle that
//! exposes the live engine bundle registry as a queryable bundle.
//!
//! Three required tests per the workflow spec:
//!
//!   - test_cover_virtual_bundles_returns_live_registry
//!   - test_cover_virtual_bundles_after_create_bundle_includes_new
//!   - test_insert_into_virtual_bundle_rejected
//!
//! Plus a few sanity cases covering the COVER clause vocabulary
//! (RANK BY / WHERE / FIRST / PROJECT) wired through the virtual
//! bundle path.

use gigi::engine::Engine;
use gigi::parser::{execute, parse, ExecResult};

fn run(engine: &mut Engine, gql: &str) -> ExecResult {
    let stmt = parse(gql).expect("parse failed");
    execute(engine, &stmt).expect("execute failed")
}

fn rows_of(result: ExecResult) -> Vec<gigi::types::Record> {
    match result {
        ExecResult::Rows(rs) => rs,
        other => panic!("expected Rows, got {other:?}"),
    }
}

#[test]
fn test_cover_virtual_bundles_returns_live_registry() {
    let mut engine = Engine::open_memory().expect("open_memory");

    // Create three real bundles, varying record counts.
    run(&mut engine, "BUNDLE users BASE (id TEXT) FIBER (score NUMERIC)");
    run(&mut engine, "BUNDLE orders BASE (id TEXT) FIBER (total NUMERIC)");
    run(&mut engine, "BUNDLE events BASE (id TEXT) FIBER (ts TIMESTAMP)");

    let rows = rows_of(run(&mut engine, "COVER __bundles__"));

    let names: Vec<String> = rows
        .iter()
        .map(|r| match r.get("name").expect("name field") {
            gigi::types::Value::Text(s) => s.clone(),
            other => panic!("name must be Text, got {other:?}"),
        })
        .collect();

    // Three real bundles + self-row.
    assert!(names.contains(&"users".to_string()), "users missing: {names:?}");
    assert!(names.contains(&"orders".to_string()), "orders missing: {names:?}");
    assert!(names.contains(&"events".to_string()), "events missing: {names:?}");
    assert!(
        names.contains(&"__bundles__".to_string()),
        "self-row missing: {names:?}"
    );
    assert_eq!(rows.len(), 4, "expected 3 user bundles + 1 self-row, got {}", rows.len());

    // The self-row must classify as type=virtual; everything else
    // is heap (no overlays in this in-memory engine).
    for r in &rows {
        let name = match r.get("name").unwrap() {
            gigi::types::Value::Text(s) => s.clone(),
            _ => unreachable!(),
        };
        let ty = match r.get("type").unwrap() {
            gigi::types::Value::Text(s) => s.clone(),
            _ => unreachable!(),
        };
        if name == "__bundles__" {
            assert_eq!(ty, "virtual", "__bundles__ row must be type=virtual");
        } else {
            assert_eq!(ty, "heap", "{name} row must be type=heap");
        }
    }
}

#[test]
fn test_cover_virtual_bundles_after_create_bundle_includes_new() {
    let mut engine = Engine::open_memory().expect("open_memory");

    // Pre-create state.
    run(&mut engine, "BUNDLE alpha BASE (id TEXT) FIBER (k NUMERIC)");
    let before = rows_of(run(&mut engine, "COVER __bundles__"));
    let before_names: Vec<String> = before
        .iter()
        .map(|r| match r.get("name").unwrap() {
            gigi::types::Value::Text(s) => s.clone(),
            _ => unreachable!(),
        })
        .collect();
    assert!(before_names.contains(&"alpha".to_string()));
    assert!(!before_names.contains(&"beta".to_string()));

    // Mutate registry, re-query.
    run(&mut engine, "BUNDLE beta BASE (id TEXT) FIBER (k NUMERIC)");
    let after = rows_of(run(&mut engine, "COVER __bundles__"));
    let after_names: Vec<String> = after
        .iter()
        .map(|r| match r.get("name").unwrap() {
            gigi::types::Value::Text(s) => s.clone(),
            _ => unreachable!(),
        })
        .collect();
    assert!(after_names.contains(&"alpha".to_string()), "alpha disappeared: {after_names:?}");
    assert!(after_names.contains(&"beta".to_string()), "beta missing post-create: {after_names:?}");
    assert!(after.len() > before.len(), "row count must grow on CREATE BUNDLE");
}

#[test]
fn test_insert_into_virtual_bundle_rejected() {
    let mut engine = Engine::open_memory().expect("open_memory");

    // INSERT into __bundles__ must be rejected at execute time. The
    // parser accepts the syntax (it's a legal-looking SECTION call)
    // but the executor's reserved-name guard rejects.
    let stmt = parse("SECTION __bundles__ (name='nope', type='heap', n_records=0, created_ts=0)")
        .expect("parse must succeed; rejection happens at execute time");
    let err = execute(&mut engine, &stmt).expect_err("execute must reject __bundles__ write");
    assert!(
        err.contains("__bundles__"),
        "error must name the rejected bundle: {err}"
    );
    assert!(
        err.contains("virtual") || err.contains("read-only"),
        "error must explain the rejection: {err}"
    );
}

#[test]
fn test_create_bundle_with_virtual_name_rejected() {
    let mut engine = Engine::open_memory().expect("open_memory");
    let stmt = parse("BUNDLE __bundles__ BASE (id TEXT) FIBER (x NUMERIC)").expect("parse");
    let err = execute(&mut engine, &stmt).expect_err("CREATE BUNDLE __bundles__ must be rejected");
    assert!(err.contains("__bundles__"), "error: {err}");
}

#[test]
fn test_collapse_virtual_bundle_rejected() {
    let mut engine = Engine::open_memory().expect("open_memory");
    let stmt = parse("COLLAPSE __bundles__").expect("parse");
    let err = execute(&mut engine, &stmt).expect_err("COLLAPSE __bundles__ must be rejected");
    assert!(err.contains("__bundles__"), "error: {err}");
}

#[test]
fn test_cover_virtual_bundles_with_where_filters_self_row() {
    let mut engine = Engine::open_memory().expect("open_memory");
    run(&mut engine, "BUNDLE u BASE (id TEXT) FIBER (k NUMERIC)");
    run(&mut engine, "BUNDLE v BASE (id TEXT) FIBER (k NUMERIC)");

    let rows = rows_of(run(&mut engine, "COVER __bundles__ WHERE type='heap'"));
    assert_eq!(rows.len(), 2, "WHERE type='heap' must hide the self-row");
    for r in &rows {
        match r.get("type").unwrap() {
            gigi::types::Value::Text(s) => assert_eq!(s, "heap"),
            _ => panic!("type must be Text"),
        }
    }
}

#[test]
fn test_cover_virtual_bundles_rank_by_n_records_first() {
    let mut engine = Engine::open_memory().expect("open_memory");
    run(&mut engine, "BUNDLE small BASE (id TEXT) FIBER (k NUMERIC)");
    run(&mut engine, "BUNDLE big BASE (id TEXT) FIBER (k NUMERIC)");
    // Stuff records into `big` so n_records differs.
    for i in 0..5 {
        let gql = format!("SECTION big (id='r{i}', k={i})");
        run(&mut engine, &gql);
    }

    let rows = rows_of(run(
        &mut engine,
        "COVER __bundles__ WHERE type='heap' RANK BY n_records DESC FIRST 1",
    ));
    assert_eq!(rows.len(), 1);
    match rows[0].get("name").unwrap() {
        gigi::types::Value::Text(s) => assert_eq!(s, "big"),
        _ => panic!("name must be Text"),
    }
    match rows[0].get("n_records").unwrap() {
        gigi::types::Value::Integer(n) => assert_eq!(*n, 5),
        other => panic!("n_records must be Integer, got {other:?}"),
    }
}

#[test]
fn test_cover_virtual_bundles_project_subset() {
    let mut engine = Engine::open_memory().expect("open_memory");
    run(&mut engine, "BUNDLE p BASE (id TEXT) FIBER (k NUMERIC)");

    let rows = rows_of(run(
        &mut engine,
        "COVER __bundles__ PROJECT (name, type)",
    ));
    assert!(rows.len() >= 2); // p + self-row
    for r in &rows {
        assert!(r.contains_key("name"), "projection must include name");
        assert!(r.contains_key("type"), "projection must include type");
        assert!(
            !r.contains_key("n_records"),
            "projection must NOT include n_records: {r:?}"
        );
        assert!(
            !r.contains_key("created_ts"),
            "projection must NOT include created_ts: {r:?}"
        );
    }
}
