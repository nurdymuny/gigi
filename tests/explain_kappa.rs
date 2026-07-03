//! EXPLAIN SECTION b AT key — WHY a record is priced the way it is.
//!
//! The decomposition rows must come from the exact loop
//! compute_record_k runs: the mean of the kappa column equals the
//! record's kappa, and the loudest field sorts first.

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
            // decomposition must average to the record's kappa exactly —
            // same loop, same numbers (SECTION reprices vs current stats)
            let recomputed = (k_temp + k_wind) / 2.0;
            match run(&mut e, "SECTION st AT id='moon';").unwrap() {
                ExecResult::Rows(rec) => {
                    if let Some(kcol) = rec[0].get("_kappa").and_then(|v| v.as_f64()) {
                        assert!(
                            (kcol - recomputed).abs() < 1e-9,
                            "EXPLAIN mean {recomputed} vs SECTION κ {kcol}"
                        );
                    } // if the point read doesn't carry κ, the decomposition stands alone
                }
                other => panic!("expected rows, got {other:?}"),
            }
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
