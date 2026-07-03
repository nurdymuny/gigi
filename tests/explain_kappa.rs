//! EXPLAIN SECTION b AT key — WHY a record is priced the way it is.
//!
//! The decomposition rows must come from the exact loop
//! compute_record_k runs: the mean of the kappa column equals the
//! record's kappa, and the loudest field sorts first.
//!
//! The invariant is asserted UNCONDITIONALLY (2026-07-03 hardening):
//! every EXPLAIN row carries `record_kappa` — the record's κ computed
//! by compute_record_k, the INDEPENDENT total path — and
//! mean(kappa column) must equal it. The cross-check runs on every
//! test execution; if either loop drifts (field skip policy, range
//! normalization, stats source), this fails.

use gigi::engine::Engine;
use gigi::parser::{self, ExecResult};
use gigi::types::Value;

fn run(e: &mut Engine, stmt: &str) -> Result<ExecResult, String> {
    let ast = parser::parse(stmt)?;
    parser::execute(e, &ast)
}

#[test]
fn explain_names_the_guilty_field() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();
    run(
        &mut e,
        "BUNDLE st BASE (id TEXT) FIBER (temp NUMERIC RANGE(100), wind NUMERIC RANGE(50));",
    )
    .unwrap();
    // 150 plain readings, then the moon: temp wild, wind ordinary
    for i in 0..150 {
        run(
            &mut e,
            &format!(
                "SECTION st (id='s{i}', temp={:.4}, wind={:.4});",
                20.0 + 0.6 * (i as f64) / 149.0,
                5.0 + 0.1 * ((i % 10) as f64)
            ),
        )
        .unwrap();
    }
    run(&mut e, "SECTION st (id='moon', temp=500.0, wind=5.5);").unwrap();

    match run(&mut e, "EXPLAIN SECTION st AT id='moon';").unwrap() {
        ExecResult::Rows(rows) => {
            assert_eq!(rows.len(), 2, "two numeric fibers, two rows");
            // loudest first — temp, by ~two orders of magnitude
            assert_eq!(rows[0]["field"], Value::Text("temp".into()));
            let k_temp = rows[0]["kappa"].as_f64().unwrap();
            let k_wind = rows[1]["kappa"].as_f64().unwrap();
            assert!(
                k_temp > 3.0 && k_wind < 0.1,
                "temp should carry the blame: temp {k_temp}, wind {k_wind}"
            );
            // z-score present and huge for the guilty field
            assert!(rows[0]["z"].as_f64().unwrap() > 10.0);

            // ── The invariant, asserted unconditionally ─────────────
            // Every row carries record_kappa: the record's κ from
            // compute_record_k (the total path pricing runs at insert
            // time), NOT derived from these rows — so this cross-checks
            // two implementations of the same loop against each other.
            // mean(kappa column) == record_kappa. Tolerance 1e-9: the
            // two sides are identical f64 arithmetic differing only in
            // summation order (schema order vs loudest-first), so the
            // drift bound is a few ULPs at κ ≈ O(1); 1e-9 sits ~7
            // orders of magnitude above that while still failing on
            // any real divergence (a dropped field costs O(κ/n)).
            let record_kappa = rows[0]
                .get("record_kappa")
                .and_then(|v| v.as_f64())
                .expect("every EXPLAIN row carries record_kappa (the compute_record_k total)");
            for row in &rows {
                let rk = row
                    .get("record_kappa")
                    .and_then(|v| v.as_f64())
                    .expect("record_kappa present on every row");
                assert!(
                    (rk - record_kappa).abs() < 1e-12,
                    "record_kappa is one number stamped on all rows: {rk} vs {record_kappa}"
                );
            }
            let mean_of_rows = (k_temp + k_wind) / 2.0;
            assert!(
                (mean_of_rows - record_kappa).abs() < 1e-9,
                "decomposition mean {mean_of_rows} must equal the record's κ {record_kappa}"
            );
        }
        other => panic!("expected decomposition rows, got {other:?}"),
    }
}

#[test]
fn explain_missing_record_is_loud() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();
    run(&mut e, "BUNDLE st BASE (id TEXT) FIBER (v NUMERIC);").unwrap();
    run(&mut e, "SECTION st (id='a', v=1.0);").unwrap();
    let err = run(&mut e, "EXPLAIN SECTION st AT id='ghost';").unwrap_err();
    assert!(err.contains("no section"), "{err}");
}

#[test]
fn explain_non_point_reads_notice_the_alternative() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();
    run(&mut e, "BUNDLE st BASE (id TEXT) FIBER (v NUMERIC);").unwrap();
    match run(&mut e, "EXPLAIN COVER st ALL;").unwrap() {
        ExecResult::Notice(msg) => assert!(msg.contains("EXPLAIN"), "{msg}"),
        ExecResult::Ok => {} // legacy placeholder shape also acceptable
        other => panic!("expected Notice or Ok, got {other:?}"),
    }
}
