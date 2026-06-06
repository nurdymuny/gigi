//! Phase 4 follow-up: `EXCLUDING IN` composes with COVER, not just HUNT.
//!
//! Gate PH15 from `theory/scj/PATTERN_HUNT_SPEC_v0.1.md` §6.4:
//!
//!   > `EXCLUDING IN` on COVER yields the same result set as the
//!   > equivalent `WHERE NOT EXISTS (COVER ...)` form.
//!
//! Same anti-join semantics as HUNT (Phase 4 main commit): match by
//! base PK value, ignore fiber, union multiple clauses set-wise,
//! order-independent.
//!
//! ### Why this matters
//!
//! Patterns own the "DEFINE-then-HUNT" mode for operators with a stable
//! ranked-candidate workflow. But ad-hoc COVER queries are still the
//! shorter spelling for one-shot exploration — and they need the same
//! exclusion ergonomics. Without PH15, an analyst running
//!
//!     COVER candidates WHERE field_a = 1
//!
//! would have to fall through to a `WHERE NOT EXISTS (COVER ...)` subquery
//! to mask known cases. PH15 lets them write the natural form.
//!
//! ### Domain-neutral
//!
//! All bundle, field, and PK names are generic.

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

fn id_of(row: &gigi::types::Record) -> i64 {
    match row.get("id") {
        Some(Value::Integer(i)) => *i,
        _ => -1,
    }
}

fn ids_of(rows: &[gigi::types::Record]) -> Vec<i64> {
    let mut v: Vec<i64> = rows.iter().map(id_of).collect();
    v.sort();
    v
}

// ─── PH15 main: COVER EXCLUDING IN matches NOT EXISTS subquery form ─────────

#[test]
fn ph15_cover_excluding_in_equals_not_exists_subquery() {
    let mut engine = fresh_engine();
    run(
        &mut engine,
        "CREATE BUNDLE candidates (id INT BASE, score INT FIBER)",
    )
    .expect("CREATE candidates");
    for (id, s) in [(1, 10), (2, 20), (3, 30), (4, 40), (5, 50)] {
        run(
            &mut engine,
            &format!("INSERT INTO candidates (id, score) VALUES ({id}, {s})"),
        )
        .expect("ins");
    }
    run(
        &mut engine,
        "CREATE BUNDLE confirmed (id INT BASE, x INT FIBER)",
    )
    .expect("CREATE confirmed");
    for id in [2, 4] {
        run(&mut engine, &format!("INSERT INTO confirmed (id, x) VALUES ({id}, 0)"))
            .expect("ins confirmed");
    }

    // The new natural form.
    let excluding_rows = match run(
        &mut engine,
        "COVER candidates WHERE score > 0 EXCLUDING IN confirmed",
    )
    .expect("COVER EXCLUDING IN must parse + execute in PH15")
    {
        ExecResult::Rows(rs) => rs,
        _ => unreachable!(),
    };

    // PH15 v0.1: the parser doesn't yet support `NOT EXISTS (subquery)`
    // (only positive EXISTS). The spec's PH15 equivalence is therefore
    // demonstrated against the deterministic expected set rather than
    // against an unsupported textual form. Once `NOT EXISTS` lands,
    // this test should add a side-by-side execution comparison.
    //
    // Expected set: candidates {1..=5} minus confirmed {2, 4} = {1, 3, 5}.
    assert_eq!(ids_of(&excluding_rows), vec![1, 3, 5]);
}

// ─── Multi-clause + order independence (mirrors HUNT's PH14) ────────────────

#[test]
fn cover_excluding_in_multi_clause_order_independent() {
    let mut engine = fresh_engine();
    run(&mut engine, "CREATE BUNDLE candidates (id INT BASE, score INT FIBER)")
        .expect("CREATE");
    for id in 1..=6 {
        run(
            &mut engine,
            &format!("INSERT INTO candidates (id, score) VALUES ({id}, {id})"),
        )
        .expect("ins");
    }
    run(&mut engine, "CREATE BUNDLE a (id INT BASE, x INT FIBER)").expect("CREATE a");
    for id in [2, 4] {
        run(&mut engine, &format!("INSERT INTO a (id, x) VALUES ({id}, 0)")).expect("ins a");
    }
    run(&mut engine, "CREATE BUNDLE b (id INT BASE, x INT FIBER)").expect("CREATE b");
    for id in [3, 5] {
        run(&mut engine, &format!("INSERT INTO b (id, x) VALUES ({id}, 0)")).expect("ins b");
    }

    let rows_ab = match run(
        &mut engine,
        "COVER candidates WHERE score > 0 EXCLUDING IN a EXCLUDING IN b",
    )
    .expect("COVER 2x EXCLUDING IN")
    {
        ExecResult::Rows(rs) => rs,
        _ => unreachable!(),
    };
    let rows_ba = match run(
        &mut engine,
        "COVER candidates WHERE score > 0 EXCLUDING IN b EXCLUDING IN a",
    )
    .expect("COVER 2x EXCLUDING IN reversed")
    {
        ExecResult::Rows(rs) => rs,
        _ => unreachable!(),
    };

    assert_eq!(ids_of(&rows_ab), ids_of(&rows_ba), "order-independent");
    assert_eq!(ids_of(&rows_ab), vec![1, 6], "1 and 6 survive both exclusions");
}

// ─── Composes with PROJECT, RANK BY, FIRST ──────────────────────────────────

#[test]
fn cover_excluding_in_composes_with_project_rank_first() {
    let mut engine = fresh_engine();
    run(&mut engine, "CREATE BUNDLE candidates (id INT BASE, score INT FIBER)")
        .expect("CREATE");
    for (id, s) in [(1, 50), (2, 40), (3, 30), (4, 20), (5, 10)] {
        run(
            &mut engine,
            &format!("INSERT INTO candidates (id, score) VALUES ({id}, {s})"),
        )
        .expect("ins");
    }
    run(&mut engine, "CREATE BUNDLE confirmed (id INT BASE, x INT FIBER)")
        .expect("CREATE confirmed");
    for id in [1] {
        run(&mut engine, &format!("INSERT INTO confirmed (id, x) VALUES ({id}, 0)"))
            .expect("ins");
    }

    // Top 2 of {2,3,4,5} by score DESC = {2 (40), 3 (30)}.
    let result = run(
        &mut engine,
        "COVER candidates WHERE score > 0 EXCLUDING IN confirmed \
         RANK BY score DESC FIRST 2 PROJECT (id, score)",
    )
    .expect("COVER with full clause set");
    let rows = match result {
        ExecResult::Rows(rs) => rs,
        _ => unreachable!(),
    };
    assert_eq!(rows.len(), 2);
    // Iteration order is deterministic per COVER's RANK BY.
    assert_eq!(id_of(&rows[0]), 2);
    assert_eq!(id_of(&rows[1]), 3);
    // PROJECT should give only id + score; no other field leaks.
    for row in &rows {
        assert!(row.contains_key("id"));
        assert!(row.contains_key("score"));
    }
}

// ─── Empty / missing exclusion bundle (mirrors HUNT) ────────────────────────

#[test]
fn cover_excluding_in_empty_bundle_is_noop() {
    let mut engine = fresh_engine();
    run(&mut engine, "CREATE BUNDLE candidates (id INT BASE, x INT FIBER)")
        .expect("CREATE");
    for id in 1..=3 {
        run(&mut engine, &format!("INSERT INTO candidates (id, x) VALUES ({id}, 0)"))
            .expect("ins");
    }
    run(&mut engine, "CREATE BUNDLE empty (id INT BASE, x INT FIBER)").expect("CREATE empty");
    let rows = match run(&mut engine, "COVER candidates EXCLUDING IN empty").expect("COVER") {
        ExecResult::Rows(rs) => rs,
        _ => unreachable!(),
    };
    assert_eq!(ids_of(&rows), vec![1, 2, 3]);
}

#[test]
fn cover_excluding_in_missing_bundle_errors() {
    let mut engine = fresh_engine();
    run(&mut engine, "CREATE BUNDLE candidates (id INT BASE, x INT FIBER)")
        .expect("CREATE");
    let err = run(&mut engine, "COVER candidates EXCLUDING IN nonexistent")
        .expect_err("missing exclusion must error");
    let msg = err.to_lowercase();
    assert!(
        msg.contains("nonexistent") || msg.contains("not exist") || msg.contains("not found"),
        "error names missing bundle: {err}"
    );
}
