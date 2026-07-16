//! EXPLAIN SECTION … AT — error contract (Marcella EXPLAIN-family ask 5a).
//!
//! Two behaviors under test:
//!
//! 1. MISSING KEY IS TYPED, NOT A BLANKET FAILURE. A point read that
//!    matches nothing returns an executor error carrying the
//!    `"NOT_FOUND: "` sentinel prefix and NAMING the key and the
//!    bundle. The HTTP layer (gql_query in gigi_stream) strips the
//!    sentinel and maps it to 404 — mirroring the REST section-fetch
//!    handler's `Record '<id>' not found in bundle '<name>'` 404 shape.
//!    Before this contract, the miss collapsed into the blanket
//!    Err→500 mapping and Marcella saw
//!    `HTTP 500 {"error":"EXPLAIN: no section at that key"}` for a
//!    plain typo'd id (live-confirmed on
//!    marcella_source_embeddings_bge_v2, 2026-07-16).
//!
//! 2. WIDE BUNDLES EXPLAIN FINE WITH A CORRECT KEY. The reported
//!    "500 on the 393-field bundle" was the miss path in disguise —
//!    there is no wide-record defect. This suite pins that with a
//!    393-field fixture mirroring marcella_source_embeddings_bge_v2's
//!    shape (record_id TEXT base + v0..v383 + 8 extra numeric fibers).

use gigi::engine::Engine;
use gigi::parser::{self, ExecResult};

fn run(e: &mut Engine, stmt: &str) -> Result<ExecResult, String> {
    let ast = parser::parse(stmt)?;
    parser::execute(e, &ast)
}

/// The typed-miss sentinel: executor errors carrying this prefix are
/// "record not found" (mapped to HTTP 404 by the server), everything
/// else stays a 500. Pinned as a literal here so a silent rename of
/// the constant breaks the fence.
const NOT_FOUND_SENTINEL: &str = "NOT_FOUND: ";

// ── fixture builders ────────────────────────────────────────────────

fn small_engine() -> (tempfile::TempDir, Engine) {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();
    run(&mut e, "BUNDLE st BASE (id TEXT) FIBER (a NUMERIC, b NUMERIC);").unwrap();
    run(&mut e, "SECTION st (id='r1', a=1.0, b=10.0);").unwrap();
    run(&mut e, "SECTION st (id='r2', a=2.0, b=30.0);").unwrap();
    run(&mut e, "SECTION st (id='r3', a=4.0, b=20.0);").unwrap();
    (dir, e)
}

/// 393-field fixture mirroring marcella_source_embeddings_bge_v2:
/// record_id (TEXT base) + v0..v383 + 8 extra scalar numeric fibers.
fn wide_engine() -> (tempfile::TempDir, Engine) {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();

    let mut ddl = String::from("BUNDLE wide BASE (record_id TEXT) FIBER (");
    for j in 0..384 {
        if j > 0 {
            ddl.push_str(", ");
        }
        ddl.push_str(&format!("v{j} NUMERIC"));
    }
    for j in 0..8 {
        ddl.push_str(&format!(", extra{j} NUMERIC"));
    }
    ddl.push_str(");");
    run(&mut e, &ddl).unwrap();

    for i in 0..3usize {
        let mut ins = format!("SECTION wide (record_id='claim:test_branch/claim_{i:04}'");
        for j in 0..384usize {
            // Varies with i for every j so each field has count=3 and
            // a strictly positive observed range.
            let val = (i as f64 + 1.0) * (0.1 + ((j % 7) as f64) * 0.05);
            ins.push_str(&format!(", v{j}={val:.6}"));
        }
        for j in 0..8usize {
            let val = (i as f64 + 1.0) * (1.0 + j as f64);
            ins.push_str(&format!(", extra{j}={val:.6}"));
        }
        ins.push_str(");");
        run(&mut e, &ins).unwrap();
    }
    (dir, e)
}

// ── 1. typed miss ───────────────────────────────────────────────────

#[test]
fn explain_missing_key_is_typed_not_found_naming_key_and_bundle() {
    let (_d, mut e) = small_engine();
    let err = run(&mut e, "EXPLAIN SECTION st AT id='definitely_missing_xyz';").unwrap_err();
    assert!(
        err.starts_with(NOT_FOUND_SENTINEL),
        "miss must carry the typed NOT_FOUND sentinel (drives the 404 \
         mapping instead of the blanket 500): {err}"
    );
    assert!(
        err.contains("id='definitely_missing_xyz'"),
        "miss must name the key: {err}"
    );
    assert!(err.contains("'st'"), "miss must name the bundle: {err}");
    // The explain_kappa fence greps for "no section" — keep the phrase.
    assert!(err.contains("no section"), "{err}");
}

#[test]
fn explain_wrong_key_field_name_is_also_typed_not_found() {
    // Hallie's actual first probe: EXPLAIN … AT id='doc_0001' against a
    // bundle whose base field is record_id. The point query matches
    // nothing; that must be the same typed miss, not a 500.
    let (_d, mut e) = wide_engine();
    let err = run(&mut e, "EXPLAIN SECTION wide AT id='doc_0001';").unwrap_err();
    assert!(
        err.starts_with(NOT_FOUND_SENTINEL),
        "wrong key NAME collapses to the typed miss: {err}"
    );
    assert!(err.contains("id='doc_0001'"), "{err}");
    assert!(err.contains("'wide'"), "{err}");
}

#[test]
fn explain_missing_integer_key_renders_bare_value() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();
    run(&mut e, "BUNDLE nums BASE (id INT) FIBER (x NUMERIC);").unwrap();
    run(&mut e, "SECTION nums (id=1, x=0.5);").unwrap();
    run(&mut e, "SECTION nums (id=2, x=1.5);").unwrap();
    let err = run(&mut e, "EXPLAIN SECTION nums AT id=99;").unwrap_err();
    assert!(err.starts_with(NOT_FOUND_SENTINEL), "{err}");
    assert!(err.contains("id=99"), "numeric keys render unquoted: {err}");
    assert!(err.contains("'nums'"), "{err}");
}

// ── 2. plain SECTION AT contract unchanged ──────────────────────────

#[test]
fn plain_section_at_miss_stays_silent_empty_rows() {
    // The typed miss is EXPLAIN's contract; plain SECTION AT keeps its
    // documented silent shape (HTTP 200 {"rows":[],"count":0}).
    let (_d, mut e) = small_engine();
    match run(&mut e, "SECTION st AT id='ghost';").unwrap() {
        ExecResult::Rows(rows) => assert!(rows.is_empty()),
        other => panic!("expected empty Rows, got {other:?}"),
    }
}

// ── 3. wide-bundle correct-key regression ───────────────────────────

#[test]
fn wide_393_field_bundle_explains_with_correct_key() {
    let (_d, mut e) = wide_engine();
    match run(
        &mut e,
        "EXPLAIN SECTION wide AT record_id='claim:test_branch/claim_0001';",
    )
    .unwrap()
    {
        ExecResult::Rows(rows) => {
            // 384 v-fields + 8 extras, all numeric with count=3 ≥ 2.
            assert_eq!(rows.len(), 392, "one row per numeric fiber field");
            // Sorted loudest-first.
            let kappas: Vec<f64> = rows
                .iter()
                .map(|r| r["kappa"].as_f64().expect("kappa on every row"))
                .collect();
            for w in kappas.windows(2) {
                assert!(w[0] >= w[1], "rows sorted kappa-descending");
            }
            // The invariant: mean(kappa) == record_kappa (1e-9).
            let record_kappa = rows[0]["record_kappa"]
                .as_f64()
                .expect("record_kappa stamped on every row");
            let mean = kappas.iter().sum::<f64>() / kappas.len() as f64;
            assert!(
                (mean - record_kappa).abs() < 1e-9,
                "mean(kappa) {mean} == record_kappa {record_kappa}"
            );
        }
        other => panic!("correct key on the wide bundle must return rows, got {other:?}"),
    }
}

#[test]
fn wide_bundle_missing_key_is_typed_not_found() {
    // The exact prod symptom, reproduced on the 393-field shape: a
    // MISSING key (not a wide-record fault) is what 500'd. Typed now.
    let (_d, mut e) = wide_engine();
    let err = run(
        &mut e,
        "EXPLAIN SECTION wide AT record_id='definitely_missing_xyz';",
    )
    .unwrap_err();
    assert!(err.starts_with(NOT_FOUND_SENTINEL), "{err}");
    assert!(err.contains("record_id='definitely_missing_xyz'"), "{err}");
    assert!(err.contains("'wide'"), "{err}");
}
