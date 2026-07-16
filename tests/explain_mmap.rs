//! EXPLAIN SECTION … AT on mmap-backed bundles (Marcella
//! EXPLAIN-family ask 5b).
//!
//! Before this ask, an OverlayBundle (mmap DHOOM base + heap overlay)
//! declined with "EXPLAIN κ needs heap-resident field statistics;
//! this bundle is mmap-backed" because the executor read stats off
//! `store.as_heap()` only. The ruling: ON-DEMAND stats — the EXPLAIN
//! executor gets per-field Welford stats from the polymorphic
//! `BundleRef::field_stats()` (OverlayBundle computes them in a single
//! O(N) scan over the mmap base on first access, merged with overlay
//! stats; cached in memory, NOTHING persisted). O(N) per first call is
//! accepted — EXPLAIN is a diagnostic verb.
//!
//! Fixtures build a heap engine, snapshot to DHOOM, and reopen with
//! `Engine::open_mmap` — the same shape as production voice_math.

use gigi::engine::Engine;
use gigi::parser::{self, ExecResult};
use gigi::types::Record;

fn run(e: &mut Engine, stmt: &str) -> Result<ExecResult, String> {
    let ast = parser::parse(stmt)?;
    parser::execute(e, &ast)
}

fn rows(e: &mut Engine, stmt: &str) -> Vec<Record> {
    match run(e, stmt).unwrap() {
        ExecResult::Rows(rows) => rows,
        other => panic!("expected Rows from `{stmt}`, got {other:?}"),
    }
}

/// Build st (id TEXT; a, b NUMERIC), snapshot, reopen mmap-backed.
fn mmap_engine3() -> (tempfile::TempDir, Engine) {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut e = Engine::open(dir.path()).unwrap();
        run(&mut e, "BUNDLE st BASE (id TEXT) FIBER (a NUMERIC, b NUMERIC);").unwrap();
        run(&mut e, "SECTION st (id='r1', a=1.0, b=10.0);").unwrap();
        run(&mut e, "SECTION st (id='r2', a=2.0, b=30.0);").unwrap();
        run(&mut e, "SECTION st (id='r3', a=4.0, b=20.0);").unwrap();
        e.snapshot().unwrap();
    }
    let e = Engine::open_mmap(dir.path()).unwrap();
    // The fixture must actually be mmap-backed or this suite tests
    // nothing: BundleRef::as_heap() is None for OverlayBundle.
    assert!(
        e.bundle("st").expect("bundle loaded").as_heap().is_none(),
        "fixture must reopen as an mmap-backed OverlayBundle"
    );
    (dir, e)
}

#[test]
fn mmap_backed_bundle_returns_real_rows_via_on_demand_stats() {
    // a over {1,2,4}: mean 7/3, range 3 → kappa(r3.a) = |4−7/3|/3 = 5/9
    // b over {10,30,20}: mean 20, range 20 → kappa(r3.b) = 0
    // record_kappa(r3) = (5/9 + 0)/2 = 5/18
    let (_d, mut e) = mmap_engine3();
    let all = rows(&mut e, "EXPLAIN SECTION st AT id='r3';");
    assert_eq!(all.len(), 2, "two numeric fibers, two rows — not a decline notice");
    // loudest first
    assert_eq!(all[0]["field"].as_str().unwrap(), "a");
    let ka = all[0]["kappa"].as_f64().unwrap();
    let kb = all[1]["kappa"].as_f64().unwrap();
    assert!((ka - 5.0 / 9.0).abs() < 1e-9, "kappa(a) = 5/9 from mmap-scanned stats: {ka}");
    assert!(kb.abs() < 1e-12, "kappa(b) = 0: {kb}");
    let record_kappa = all[0]["record_kappa"].as_f64().unwrap();
    assert!(
        ((ka + kb) / 2.0 - record_kappa).abs() < 1e-9,
        "invariant holds on mmap: mean(kappa) == record_kappa"
    );
    assert!((record_kappa - 5.0 / 18.0).abs() < 1e-9);
}

#[test]
fn mmap_missing_key_is_still_typed_not_found() {
    let (_d, mut e) = mmap_engine3();
    let err = run(&mut e, "EXPLAIN SECTION st AT id='ghost';").unwrap_err();
    assert!(err.starts_with("NOT_FOUND: "), "{err}");
    assert!(err.contains("id='ghost'"), "{err}");
    assert!(err.contains("'st'"), "{err}");
}

#[test]
fn mmap_vector_clause_gets_kappa_v_from_record_scan() {
    // The vector-suite fixture, snapshot→mmap: a=(2,0), b=(0,1) over
    // v0,v1. kappa_v(a)=√5−2 — pins that the VECTOR clause's mu/R_cos
    // record scans run fine over the mmap base.
    let dir = tempfile::tempdir().unwrap();
    {
        let mut e = Engine::open(dir.path()).unwrap();
        run(&mut e, "BUNDLE st BASE (id TEXT) FIBER (v0 NUMERIC, v1 NUMERIC);").unwrap();
        run(&mut e, "SECTION st (id='a', v0=2.0, v1=0.0);").unwrap();
        run(&mut e, "SECTION st (id='b', v0=0.0, v1=1.0);").unwrap();
        e.snapshot().unwrap();
    }
    let mut e = Engine::open_mmap(dir.path()).unwrap();
    assert!(e.bundle("st").unwrap().as_heap().is_none());

    let all = rows(&mut e, "EXPLAIN SECTION st AT id='a' VECTOR (v0..v1);");
    let v = all
        .iter()
        .find(|r| r.get("kind").and_then(|v| v.as_str()) == Some("vector"))
        .expect("vector row present on mmap-backed bundle");
    assert_eq!(v["field"].as_str().unwrap(), "vector(v0..v1)");
    let kv = v["kappa"].as_f64().unwrap();
    assert!((kv - (5f64.sqrt() - 2.0)).abs() < 1e-9, "kappa_v(a) = √5−2: {kv}");
}

#[test]
fn mmap_batch_with_miss_entry_works() {
    let (_d, mut e) = mmap_engine3();
    let all = rows(&mut e, "EXPLAIN SECTION st AT id IN ('r1', 'ghost');");
    // r1 group: 2 rows; ghost: 1 miss row.
    assert_eq!(all.len(), 3);
    let miss: Vec<&Record> = all
        .iter()
        .filter(|r| r.get("kind").and_then(|v| v.as_str()) == Some("miss"))
        .collect();
    assert_eq!(miss.len(), 1);
    assert_eq!(miss[0]["id"].as_str().unwrap(), "ghost");
    let found: Vec<&Record> = all
        .iter()
        .filter(|r| r.get("id").and_then(|v| v.as_str()) == Some("r1"))
        .collect();
    assert_eq!(found.len(), 2, "full group for the found key");
}
