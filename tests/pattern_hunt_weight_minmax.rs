//! Phase 3 follow-up: `min(...)` and `max(...)` in WEIGHT expressions.
//!
//! Per SCJ 2026-06-09 letter §1: the clip semantic `min(sum, MAX_SCORE)`
//! is the load-bearing missing piece for translating their flat
//! 10-weight linear scorer end-to-end. With `min()` available in
//! WEIGHT, their `risk_score.py` translates without consumer-side
//! post-processing.
//!
//! Function-call atom grammar:
//!   atom := number | ident | '(' expr ')' | ident '(' expr (',' expr)* ')'
//!
//! Phase 3.1 contract: two-arg `min` and `max` only. Variadic min/max,
//! conditional functions (`if`, `select`), and statistical aggregators
//! are deferred to v0.2 per spec OQ-2.

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

fn score_of(row: &gigi::types::Record) -> f64 {
    match row.get("_score") {
        Some(Value::Float(f)) => *f,
        Some(Value::Integer(i)) => *i as f64,
        _ => 0.0,
    }
}

// ─── min(a, b) ──────────────────────────────────────────────────────────────

#[test]
fn weight_min_two_literals_picks_smaller() {
    let mut engine = fresh_engine();
    run(&mut engine, "CREATE BUNDLE b (id INT BASE, x INT FIBER)").expect("create");
    run(&mut engine, "INSERT INTO b (id, x) VALUES (1, 0)").expect("insert");
    run(&mut engine, "DEFINE PATTERN p AS x >= 0 WEIGHT (min(7.5, 10.0))").expect("define");
    let rows = match run(&mut engine, "HUNT p IN b").expect("hunt") {
        ExecResult::Rows(rs) => rs,
        _ => unreachable!(),
    };
    assert_eq!(rows.len(), 1);
    assert_eq!(score_of(&rows[0]), 7.5);
}

#[test]
fn weight_min_clips_sum_to_max_score() {
    // SCJ's load-bearing use case: min(linear_sum, MAX_SCORE).
    let mut engine = fresh_engine();
    run(
        &mut engine,
        "CREATE BUNDLE b (id INT BASE, a INT FIBER, c INT FIBER, d INT FIBER)",
    )
    .expect("create");
    // Row scores would be a*5 + c*5 + d*5 unclipped — all 15 for these inputs.
    for id in 1..=3 {
        let sql = format!("INSERT INTO b (id, a, c, d) VALUES ({id}, 1, 1, 1)");
        run(&mut engine, &sql).expect("insert");
    }
    run(
        &mut engine,
        "DEFINE PATTERN clipped AS a >= 0 \
         WEIGHT (min(a * 5 + c * 5 + d * 5, 10.0))",
    )
    .expect("define clipped");
    let rows = match run(&mut engine, "HUNT clipped IN b").expect("hunt") {
        ExecResult::Rows(rs) => rs,
        _ => unreachable!(),
    };
    for row in &rows {
        assert_eq!(
            score_of(row),
            10.0,
            "clip should hold sum (15) down to MAX_SCORE (10): {row:?}"
        );
    }
}

#[test]
fn weight_min_with_field_reference() {
    let mut engine = fresh_engine();
    run(
        &mut engine,
        "CREATE BUNDLE b (id INT BASE, x INT FIBER, cap INT FIBER)",
    )
    .expect("create");
    for (id, x, cap) in [(1, 3, 10), (2, 15, 10), (3, 7, 8)] {
        let sql = format!("INSERT INTO b (id, x, cap) VALUES ({id}, {x}, {cap})");
        run(&mut engine, &sql).expect("insert");
    }
    // Each row's _score = min(x, cap)
    run(
        &mut engine,
        "DEFINE PATTERN cap_pattern AS x >= 0 WEIGHT (min(x, cap))",
    )
    .expect("define");
    let rows = match run(&mut engine, "HUNT cap_pattern IN b").expect("hunt") {
        ExecResult::Rows(rs) => rs,
        _ => unreachable!(),
    };
    // Sorted by _score DESC, tie-break by id ASC:
    //   id=2: min(15, 10) = 10
    //   id=3: min(7, 8)   = 7
    //   id=1: min(3, 10)  = 3
    assert_eq!(rows.len(), 3);
    assert_eq!(score_of(&rows[0]), 10.0);
    assert_eq!(score_of(&rows[1]), 7.0);
    assert_eq!(score_of(&rows[2]), 3.0);
}

// ─── max(a, b) ──────────────────────────────────────────────────────────────

#[test]
fn weight_max_two_literals_picks_larger() {
    let mut engine = fresh_engine();
    run(&mut engine, "CREATE BUNDLE b (id INT BASE, x INT FIBER)").expect("create");
    run(&mut engine, "INSERT INTO b (id, x) VALUES (1, 0)").expect("insert");
    run(&mut engine, "DEFINE PATTERN p AS x >= 0 WEIGHT (max(3.0, 8.0))").expect("define");
    let rows = match run(&mut engine, "HUNT p IN b").expect("hunt") {
        ExecResult::Rows(rs) => rs,
        _ => unreachable!(),
    };
    assert_eq!(score_of(&rows[0]), 8.0);
}

#[test]
fn weight_max_with_field_reference_acts_as_floor() {
    let mut engine = fresh_engine();
    run(
        &mut engine,
        "CREATE BUNDLE b (id INT BASE, raw INT FIBER)",
    )
    .expect("create");
    for (id, raw) in [(1, -5), (2, 3), (3, 7)] {
        let sql = format!("INSERT INTO b (id, raw) VALUES ({id}, {raw})");
        run(&mut engine, &sql).expect("insert");
    }
    // Floor each row's score at zero — i.e. max(raw, 0).
    run(
        &mut engine,
        "DEFINE PATTERN floored AS raw >= -100 WEIGHT (max(raw, 0))",
    )
    .expect("define");
    let rows = match run(&mut engine, "HUNT floored IN b").expect("hunt") {
        ExecResult::Rows(rs) => rs,
        _ => unreachable!(),
    };
    assert_eq!(rows.len(), 3);
    assert_eq!(score_of(&rows[0]), 7.0); // id=3
    assert_eq!(score_of(&rows[1]), 3.0); // id=2
    assert_eq!(score_of(&rows[2]), 0.0); // id=1 (floored from -5)
}

// ─── Composition: min(max(...), ...) and the full SCJ-shape clip ────────────

#[test]
fn weight_min_max_compose_for_two_sided_clip() {
    let mut engine = fresh_engine();
    run(
        &mut engine,
        "CREATE BUNDLE b (id INT BASE, raw INT FIBER)",
    )
    .expect("create");
    for (id, raw) in [(1, -3), (2, 5), (3, 17)] {
        let sql = format!("INSERT INTO b (id, raw) VALUES ({id}, {raw})");
        run(&mut engine, &sql).expect("insert");
    }
    // Two-sided clip: floor at 0, cap at 10 → min(max(raw, 0), 10).
    run(
        &mut engine,
        "DEFINE PATTERN clipped AS raw >= -100 \
         WEIGHT (min(max(raw, 0), 10))",
    )
    .expect("define");
    let rows = match run(&mut engine, "HUNT clipped IN b").expect("hunt") {
        ExecResult::Rows(rs) => rs,
        _ => unreachable!(),
    };
    // id=3: min(max(17, 0), 10) = min(17, 10) = 10
    // id=2: min(max(5,  0), 10) = min(5,  10) = 5
    // id=1: min(max(-3, 0), 10) = min(0,  10) = 0
    assert_eq!(score_of(&rows[0]), 10.0);
    assert_eq!(score_of(&rows[1]), 5.0);
    assert_eq!(score_of(&rows[2]), 0.0);
}

// ─── The full SCJ 10-weight scorer with clip — end-to-end shape proof ───────
//
// Per SCJ 2026-06-09 Appendix A. Each fiber field is 0 or 1 (boolean shadow).
// The clipped score: min(sum_of_weighted_terms, 10.0).
// AUDIT_THRESHOLD per their letter: 7.0 (used by consumer-side gate).

#[test]
fn weight_full_scj_10_weight_scorer_with_clip() {
    let mut engine = fresh_engine();
    run(
        &mut engine,
        "CREATE BUNDLE candidates (\
           id INT BASE,\
           cast_truncate_alloc INT FIBER,\
           multiply_before_alloc INT FIBER,\
           shift_before_alloc INT FIBER,\
           param_times_const INT FIBER,\
           unchecked_param_to_size INT FIBER,\
           mdl_shift_size INT FIBER,\
           reaches_ExAllocatePool2 INT FIBER,\
           reaches_MmBuildMdlForNonPagedPool INT FIBER,\
           has_probe_read INT FIBER,\
           has_probe_write INT FIBER\
         )",
    )
    .expect("create candidates");

    // Insert three candidates with varying signature density.
    // Domain-neutral: the test cares about the arithmetic, not what the
    // fields mean. These names happen to mirror SCJ's binary-vuln
    // domain, but the substrate doesn't know that.
    let rows_data = [
        // id, [10 feature bits]
        (1, [1, 1, 1, 1, 1, 1, 1, 1, 1, 1]), // all fire → 6*3+? actually: 3+3+3+2+2+2+1+1+1+1 = 19 → clipped 10
        (2, [1, 0, 0, 1, 0, 0, 1, 0, 0, 0]), // 3+0+0+2+0+0+1+0+0+0 = 6
        (3, [0, 0, 0, 0, 0, 0, 1, 1, 0, 0]), // 0+0+0+0+0+0+1+1+0+0 = 2
    ];
    for (id, bits) in &rows_data {
        let cols = "id, cast_truncate_alloc, multiply_before_alloc, shift_before_alloc, \
                    param_times_const, unchecked_param_to_size, mdl_shift_size, \
                    reaches_ExAllocatePool2, reaches_MmBuildMdlForNonPagedPool, \
                    has_probe_read, has_probe_write";
        let vals = format!(
            "{id}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}",
            bits[0], bits[1], bits[2], bits[3], bits[4], bits[5], bits[6], bits[7], bits[8], bits[9]
        );
        let sql = format!("INSERT INTO candidates ({cols}) VALUES ({vals})");
        run(&mut engine, &sql).expect("insert candidate");
    }

    // Per SCJ Appendix A. Flat linear, clipped at MAX_SCORE=10.
    run(
        &mut engine,
        "DEFINE PATTERN scj_v01 AS cast_truncate_alloc >= 0 WEIGHT (\
           min(\
             cast_truncate_alloc * 3 \
             + multiply_before_alloc * 3 \
             + shift_before_alloc * 3 \
             + param_times_const * 2 \
             + unchecked_param_to_size * 2 \
             + mdl_shift_size * 2 \
             + reaches_ExAllocatePool2 * 1 \
             + reaches_MmBuildMdlForNonPagedPool * 1 \
             + has_probe_read * 1 \
             + has_probe_write * 1,\
             10\
           )\
         )",
    )
    .expect("define scj_v01");

    let rows = match run(&mut engine, "HUNT scj_v01 IN candidates").expect("hunt") {
        ExecResult::Rows(rs) => rs,
        _ => unreachable!(),
    };
    assert_eq!(rows.len(), 3);
    // Expected scores: id=1 → clipped to 10; id=2 → 6; id=3 → 2.
    assert_eq!(score_of(&rows[0]), 10.0);
    assert_eq!(score_of(&rows[1]), 6.0);
    assert_eq!(score_of(&rows[2]), 2.0);
}
