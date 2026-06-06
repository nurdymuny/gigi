//! Phase 3 of the Pattern Hunt spec (Ask G — Patterns):
//! HUNT planner + executor with WEIGHT evaluation, sort, and TOP-N.
//!
//! Gates PH9–PH12 from `theory/scj/PATTERN_HUNT_SPEC_v0.1.md` §5.4.
//!
//! ### Strategy
//!
//! HUNT desugars into a Cover-shaped plan:
//!
//!   1. Resolve pattern from registry; substitute pred → WHERE.
//!   2. Recursively execute the equivalent COVER (no PROJECT, no TOP yet
//!      — we want every matching row with every field for WEIGHT eval).
//!   3. For each surviving row, evaluate WEIGHT → write `_score` field.
//!   4. Sort by `_score` DESC, tie-break by base PK ASC.
//!   5. Apply TOP n.
//!   6. Apply user's PROJECT (if any).
//!
//! ### What this file covers
//!
//! - **PH9**  Single-pattern HUNT against a hand-built corpus produces
//!            a ranking identical to the equivalent COVER + post-compute.
//! - **PH10** `_score` is computed correctly with NULL / missing fields
//!            coerced to 0.0 (spec §5.3).
//! - **PH11** Planted-anchor recovery: HUNT surfaces a deliberately
//!            high-weighted row at rank 1 against a 1,000-row synthetic
//!            corpus. Generic equivalent of the JUROJIN recovery in spec
//!            §16; the substrate doesn't know what JUROJIN is, it just
//!            knows how to rank weighted predicate-filtered rows.
//! - **PH12** Tie-breaking is stable: rows with identical `_score`
//!            order by base PK ASC, deterministically across runs.
//!
//! Domain-neutral throughout. The planted anchor is just "row with
//! features all firing"; the substrate has no domain semantics.

#![cfg(feature = "patterns")]

use gigi::engine::Engine;
use gigi::parser::{execute, parse, ExecResult};
use gigi::types::Value;
use tempfile::tempdir;

fn fresh_engine() -> Engine {
    let dir = tempdir().expect("tempdir");
    let path = dir.into_path();
    Engine::open(&path).expect("engine open")
}

fn run(engine: &mut Engine, sql: &str) -> Result<ExecResult, String> {
    let stmt = parse(sql).map_err(|e| format!("parse `{sql}`: {e}"))?;
    execute(engine, &stmt)
}

/// Setup helper — common bundle + sample data used by PH9-PH12. Fields
/// are domain-neutral integers (`a`, `b`, `c`) so the substrate can't
/// possibly specialize.
fn build_three_feature_bundle(engine: &mut Engine) {
    run(
        engine,
        "CREATE BUNDLE feats (id INT BASE, a INT FIBER, b INT FIBER, c INT FIBER)",
    )
    .expect("CREATE BUNDLE");
}

/// Extract `_score` from a row. Returns 0.0 if missing or non-numeric.
fn score_of(row: &gigi::types::Record) -> f64 {
    match row.get("_score") {
        Some(Value::Float(f)) => *f,
        Some(Value::Integer(i)) => *i as f64,
        _ => 0.0,
    }
}

/// Extract `id` from a row.
fn id_of(row: &gigi::types::Record) -> i64 {
    match row.get("id") {
        Some(Value::Integer(i)) => *i,
        _ => -1,
    }
}

// ─── PH9 — HUNT ranking matches manual COVER + post-compute ─────────────────

#[test]
fn ph9_hunt_ranking_matches_manual_score_calculation() {
    let mut engine = fresh_engine();
    build_three_feature_bundle(&mut engine);

    // Insert 5 rows. Weighted score = a*3 + b*2 + c*1.
    // Expected ranking by score: row3(15) > row1(12) > row5(8) > row2(7) > row4(6).
    for (id, a, b, c) in [
        (1, 2, 3, 0), // 2*3 + 3*2 + 0 = 12
        (2, 1, 2, 0), // 1*3 + 2*2 + 0 = 7
        (3, 3, 3, 0), // 3*3 + 3*2 + 0 = 15
        (4, 0, 3, 0), // 0 + 6 + 0 = 6
        (5, 2, 1, 0), // 6 + 2 + 0 = 8
    ] {
        let sql = format!("INSERT INTO feats (id, a, b, c) VALUES ({id}, {a}, {b}, {c})");
        run(&mut engine, &sql).expect("insert");
    }

    run(
        &mut engine,
        "DEFINE PATTERN p AS a >= 0 WEIGHT (a * 3 + b * 2 + c * 1)",
    )
    .expect("DEFINE PATTERN");

    let result = run(&mut engine, "HUNT p IN feats").expect("HUNT must succeed");
    let rows = match result {
        ExecResult::Rows(rs) => rs,
        other => panic!("expected Rows, got {other:?}"),
    };

    assert_eq!(rows.len(), 5, "should return all 5 rows");

    // Verify ordering: row 3 first (score 15), then 1, 5, 2, 4.
    let expected_id_order = vec![3, 1, 5, 2, 4];
    let actual_id_order: Vec<i64> = rows.iter().map(id_of).collect();
    assert_eq!(actual_id_order, expected_id_order, "ranking by _score DESC");

    // Verify scores themselves.
    assert_eq!(score_of(&rows[0]), 15.0);
    assert_eq!(score_of(&rows[1]), 12.0);
    assert_eq!(score_of(&rows[2]), 8.0);
    assert_eq!(score_of(&rows[3]), 7.0);
    assert_eq!(score_of(&rows[4]), 6.0);
}

// ─── PH10 — NULL / missing field coerces to 0.0 ─────────────────────────────

#[test]
fn ph10_weight_handles_missing_or_null_fields_as_zero() {
    let mut engine = fresh_engine();
    // Bundle has `a` only; pattern WEIGHT references a + nonexistent.
    // Per spec §5.3, missing field → 0.0.
    run(
        &mut engine,
        "CREATE BUNDLE numerics (id INT BASE, a INT FIBER)",
    )
    .expect("CREATE BUNDLE");
    for (id, a) in [(1, 5), (2, 3), (3, 7)] {
        let sql = format!("INSERT INTO numerics (id, a) VALUES ({id}, {a})");
        run(&mut engine, &sql).expect("insert");
    }

    // WEIGHT references both `a` (present) and `b` (absent).
    // Per coercion rule, b → 0.0, so each row's score == a's value.
    run(
        &mut engine,
        "DEFINE PATTERN p AS a >= 0 WEIGHT (a + b * 100)",
    )
    .expect("DEFINE PATTERN");

    let rows = match run(&mut engine, "HUNT p IN numerics").expect("HUNT") {
        ExecResult::Rows(rs) => rs,
        _ => unreachable!(),
    };

    assert_eq!(rows.len(), 3);
    // Scores should equal each row's a value (because b coerces to 0).
    assert_eq!(score_of(&rows[0]), 7.0);
    assert_eq!(score_of(&rows[1]), 5.0);
    assert_eq!(score_of(&rows[2]), 3.0);
}

// ─── PH11 — planted-anchor recovery against a synthetic 1000-row corpus ─────

#[test]
fn ph11_hunt_recovers_planted_anchor_at_rank_one() {
    let mut engine = fresh_engine();
    build_three_feature_bundle(&mut engine);

    // Insert 999 background rows with random small a, b, c. Each scores
    // between 0 and ~30 under our weight scheme.
    // Then plant ONE row (id=1000) with a=100, b=100, c=100 — score 600,
    // dramatically above any background row.
    for id in 1..=999 {
        // Deterministic pseudo-random based on id, keeping a/b/c in [0, 9].
        let a = (id * 7) % 10;
        let b = (id * 11) % 10;
        let c = (id * 13) % 10;
        let sql = format!("INSERT INTO feats (id, a, b, c) VALUES ({id}, {a}, {b}, {c})");
        run(&mut engine, &sql).expect("background insert");
    }
    // Plant the anchor.
    run(
        &mut engine,
        "INSERT INTO feats (id, a, b, c) VALUES (1000, 100, 100, 100)",
    )
    .expect("planted-anchor insert");

    run(
        &mut engine,
        "DEFINE PATTERN p AS a >= 0 WEIGHT (a * 3 + b * 2 + c * 1)",
    )
    .expect("DEFINE PATTERN");

    let rows = match run(&mut engine, "HUNT p IN feats TOP 10").expect("HUNT TOP 10") {
        ExecResult::Rows(rs) => rs,
        _ => unreachable!(),
    };

    assert_eq!(rows.len(), 10, "TOP 10 should return exactly 10");
    assert_eq!(id_of(&rows[0]), 1000, "planted anchor must be rank 1");
    assert_eq!(score_of(&rows[0]), 600.0, "anchor score = 100*3 + 100*2 + 100*1");

    // And the planted anchor's score dominates the next-best by ≥10×.
    assert!(
        score_of(&rows[0]) > score_of(&rows[1]) * 10.0,
        "anchor's lead over rank-2 should be huge: {} vs {}",
        score_of(&rows[0]),
        score_of(&rows[1])
    );
}

// ─── PH12 — tie-breaking is stable by base PK ASC ───────────────────────────

#[test]
fn ph12_tie_breaking_is_stable_by_base_pk_ascending() {
    let mut engine = fresh_engine();
    build_three_feature_bundle(&mut engine);

    // Insert 5 rows that ALL score the same (a + b + c = 5 each).
    for (id, a, b, c) in [
        (5, 2, 2, 1),
        (1, 1, 2, 2),
        (3, 0, 5, 0),
        (2, 5, 0, 0),
        (4, 3, 1, 1),
    ] {
        let sql = format!("INSERT INTO feats (id, a, b, c) VALUES ({id}, {a}, {b}, {c})");
        run(&mut engine, &sql).expect("insert");
    }

    run(
        &mut engine,
        "DEFINE PATTERN p AS a >= 0 WEIGHT (a + b + c)",
    )
    .expect("DEFINE PATTERN");

    let rows = match run(&mut engine, "HUNT p IN feats").expect("HUNT") {
        ExecResult::Rows(rs) => rs,
        _ => unreachable!(),
    };

    assert_eq!(rows.len(), 5);
    // All scores should be 5.
    for row in &rows {
        assert_eq!(score_of(row), 5.0, "all ties at score=5");
    }
    // Order should be id ASC (tie-break).
    let id_order: Vec<i64> = rows.iter().map(id_of).collect();
    assert_eq!(id_order, vec![1, 2, 3, 4, 5], "ties break by base PK ASC");
}

// ─── bonus — TOP n truncation ───────────────────────────────────────────────

#[test]
fn top_n_truncates_to_n_rows() {
    let mut engine = fresh_engine();
    build_three_feature_bundle(&mut engine);
    for id in 1..=20 {
        let sql = format!("INSERT INTO feats (id, a, b, c) VALUES ({id}, {id}, 0, 0)");
        run(&mut engine, &sql).expect("insert");
    }
    run(&mut engine, "DEFINE PATTERN p AS a >= 0 WEIGHT (a)").expect("DEFINE");
    let rows = match run(&mut engine, "HUNT p IN feats TOP 5").expect("HUNT") {
        ExecResult::Rows(rs) => rs,
        _ => unreachable!(),
    };
    assert_eq!(rows.len(), 5);
    // Highest scores first (id=20 scores 20).
    assert_eq!(id_of(&rows[0]), 20);
    assert_eq!(id_of(&rows[4]), 16);
}

// ─── bonus — PROJECT filters returned fields ────────────────────────────────

#[test]
fn project_clause_filters_returned_fields() {
    let mut engine = fresh_engine();
    build_three_feature_bundle(&mut engine);
    run(&mut engine, "INSERT INTO feats (id, a, b, c) VALUES (1, 5, 3, 1)")
        .expect("insert");
    run(&mut engine, "DEFINE PATTERN p AS a >= 0 WEIGHT (a * 2)").expect("DEFINE");
    let rows = match run(&mut engine, "HUNT p IN feats PROJECT (id, _score)").expect("HUNT")
    {
        ExecResult::Rows(rs) => rs,
        _ => unreachable!(),
    };
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    // Should have `id` and `_score`, and NOT `a`, `b`, `c`.
    assert!(row.contains_key("id"), "id projected");
    assert!(row.contains_key("_score"), "_score projected");
    assert!(!row.contains_key("a"), "a should be excluded by PROJECT");
    assert!(!row.contains_key("b"), "b should be excluded by PROJECT");
    assert!(!row.contains_key("c"), "c should be excluded by PROJECT");
    assert_eq!(score_of(row), 10.0);
}

// ─── domain-neutrality smoke ────────────────────────────────────────────────

#[test]
fn hunt_works_across_consumer_styles() {
    let mut engine = fresh_engine();

    // Fraud-shape: tiny merchant + big amount → suspicious.
    run(
        &mut engine,
        "CREATE BUNDLE txns (id INT BASE, amount INT FIBER, merchant_age INT FIBER)",
    )
    .expect("CREATE");
    for (id, amount, age) in [(1, 50, 365), (2, 50000, 5), (3, 200, 1000)] {
        let sql = format!(
            "INSERT INTO txns (id, amount, merchant_age) VALUES ({id}, {amount}, {age})"
        );
        run(&mut engine, &sql).expect("ins");
    }
    run(
        &mut engine,
        "DEFINE PATTERN suspicious AS amount > 0 WEIGHT (amount + 1000 - merchant_age)",
    )
    .expect("DEFINE");
    let rows = match run(&mut engine, "HUNT suspicious IN txns TOP 1").expect("HUNT") {
        ExecResult::Rows(rs) => rs,
        _ => unreachable!(),
    };
    assert_eq!(rows.len(), 1);
    assert_eq!(id_of(&rows[0]), 2, "high-amount + tiny merchant should win");
}
