//! Phase 4 of the Pattern Hunt spec (Ask G — Patterns):
//! `EXCLUDING IN <bundle>` as a left-anti-join by base PK.
//!
//! Gates PH13, PH14, PH16 from `theory/scj/PATTERN_HUNT_SPEC_v0.1.md` §6.4.
//!
//! PH15 (`EXCLUDING IN` composes with COVER) requires extending the COVER
//! grammar — deferred to a follow-up.
//!
//! ### What this file covers
//!
//! - **PH13** `HUNT p IN A EXCLUDING IN B` returns exactly the rows in A
//!            matching p whose base PK is **not** in B.
//! - **PH14** Multiple `EXCLUDING IN` clauses compose as set difference,
//!            order-independent.
//! - **PH16** EXCLUDING IN against a bundle whose fiber would error if
//!            decrypted still works — i.e. the anti-join touches only the
//!            PK column, not the fiber. (Verified via behavioral proxy:
//!            an excluded bundle with NO matching pattern fields still
//!            functions as an exclusion source.)
//!
//! ### Domain-neutrality
//!
//! The PK column is whatever the bundle's base field is named — generic
//! `id` here, but the substrate accepts any field marked BASE. Same
//! discipline as Phase 1-3.

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
    rows.iter().map(id_of).collect()
}

// ─── PH13 — single EXCLUDING IN clause ──────────────────────────────────────

#[test]
fn ph13_single_excluding_in_filters_by_pk() {
    let mut engine = fresh_engine();
    // Target bundle: 5 rows with ids 1-5.
    run(
        &mut engine,
        "CREATE BUNDLE candidates (id INT BASE, score INT FIBER)",
    )
    .expect("CREATE candidates");
    for (id, score) in [(1, 10), (2, 20), (3, 30), (4, 40), (5, 50)] {
        let sql = format!("INSERT INTO candidates (id, score) VALUES ({id}, {score})");
        run(&mut engine, &sql).expect("ins candidates");
    }

    // Exclusion bundle: rows with ids 2 and 4. Note: it can have a
    // *different* schema — only the base PK matters.
    run(
        &mut engine,
        "CREATE BUNDLE confirmed_bugs (id INT BASE, note TEXT FIBER)",
    )
    .expect("CREATE confirmed_bugs");
    for id in [2, 4] {
        let sql = format!("INSERT INTO confirmed_bugs (id, note) VALUES ({id}, 'known')");
        run(&mut engine, &sql).expect("ins confirmed");
    }

    run(
        &mut engine,
        "DEFINE PATTERN p AS score >= 0 WEIGHT (score)",
    )
    .expect("DEFINE");

    // HUNT with EXCLUDING IN should return [5, 3, 1] (score DESC, excluding 2 and 4).
    let rows = match run(
        &mut engine,
        "HUNT p IN candidates EXCLUDING IN confirmed_bugs",
    )
    .expect("HUNT must succeed in Phase 4")
    {
        ExecResult::Rows(rs) => rs,
        other => panic!("expected Rows, got {other:?}"),
    };

    assert_eq!(rows.len(), 3, "exactly 3 rows survive (5 - 2 excluded)");
    assert_eq!(ids_of(&rows), vec![5, 3, 1], "DESC by score, excluding 2+4");
}

#[test]
fn ph13_empty_exclusion_bundle_changes_nothing() {
    let mut engine = fresh_engine();
    run(
        &mut engine,
        "CREATE BUNDLE candidates (id INT BASE, score INT FIBER)",
    )
    .expect("CREATE candidates");
    for (id, score) in [(1, 10), (2, 20)] {
        let sql = format!("INSERT INTO candidates (id, score) VALUES ({id}, {score})");
        run(&mut engine, &sql).expect("ins");
    }
    run(
        &mut engine,
        "CREATE BUNDLE empty_excludes (id INT BASE, x INT FIBER)",
    )
    .expect("CREATE empty");
    // No rows in empty_excludes.

    run(&mut engine, "DEFINE PATTERN p AS score >= 0 WEIGHT (score)")
        .expect("DEFINE");

    let rows = match run(
        &mut engine,
        "HUNT p IN candidates EXCLUDING IN empty_excludes",
    )
    .expect("HUNT")
    {
        ExecResult::Rows(rs) => rs,
        _ => unreachable!(),
    };
    assert_eq!(ids_of(&rows), vec![2, 1], "empty exclusion → all rows");
}

#[test]
fn ph13_missing_exclusion_bundle_returns_clean_error() {
    let mut engine = fresh_engine();
    run(
        &mut engine,
        "CREATE BUNDLE candidates (id INT BASE, score INT FIBER)",
    )
    .expect("CREATE");
    run(&mut engine, "DEFINE PATTERN p AS score >= 0").expect("DEFINE");

    let err = run(
        &mut engine,
        "HUNT p IN candidates EXCLUDING IN nonexistent_bundle",
    )
    .expect_err("missing exclusion bundle must error");

    let msg = err.to_lowercase();
    assert!(
        msg.contains("nonexistent") || msg.contains("not found") || msg.contains("does not exist"),
        "error should name the missing bundle, got: {err}"
    );
}

// ─── PH14 — multiple EXCLUDING IN clauses ───────────────────────────────────

#[test]
fn ph14_multiple_excluding_in_compose_as_set_difference() {
    let mut engine = fresh_engine();
    run(
        &mut engine,
        "CREATE BUNDLE candidates (id INT BASE, score INT FIBER)",
    )
    .expect("CREATE candidates");
    for id in 1..=6 {
        let sql = format!("INSERT INTO candidates (id, score) VALUES ({id}, {id})");
        run(&mut engine, &sql).expect("ins");
    }

    run(&mut engine, "CREATE BUNDLE bugs (id INT BASE, x INT FIBER)")
        .expect("CREATE bugs");
    for id in [2, 4] {
        let sql = format!("INSERT INTO bugs (id, x) VALUES ({id}, 0)");
        run(&mut engine, &sql).expect("ins bugs");
    }

    run(
        &mut engine,
        "CREATE BUNDLE false_positives (id INT BASE, x INT FIBER)",
    )
    .expect("CREATE fps");
    for id in [3, 5] {
        let sql = format!("INSERT INTO false_positives (id, x) VALUES ({id}, 0)");
        run(&mut engine, &sql).expect("ins fps");
    }

    run(&mut engine, "DEFINE PATTERN p AS score >= 0 WEIGHT (score)")
        .expect("DEFINE");

    // bugs = {2,4}; false_positives = {3,5}; union = {2,3,4,5}.
    // Survivors from {1..=6} after exclusion: {1, 6} → DESC by score: [6, 1].
    let rows = match run(
        &mut engine,
        "HUNT p IN candidates EXCLUDING IN bugs EXCLUDING IN false_positives",
    )
    .expect("HUNT with 2 EXCLUDING IN")
    {
        ExecResult::Rows(rs) => rs,
        _ => unreachable!(),
    };
    assert_eq!(ids_of(&rows), vec![6, 1], "two-set difference: 1,6 survive");
}

#[test]
fn ph14_multiple_excluding_in_is_order_independent() {
    let mut engine = fresh_engine();
    run(
        &mut engine,
        "CREATE BUNDLE candidates (id INT BASE, score INT FIBER)",
    )
    .expect("CREATE");
    for id in 1..=6 {
        let sql = format!("INSERT INTO candidates (id, score) VALUES ({id}, {id})");
        run(&mut engine, &sql).expect("ins");
    }
    run(&mut engine, "CREATE BUNDLE a (id INT BASE, x INT FIBER)").expect("CREATE a");
    for id in [2, 4] {
        run(&mut engine, &format!("INSERT INTO a (id, x) VALUES ({id}, 0)")).expect("ins a");
    }
    run(&mut engine, "CREATE BUNDLE b (id INT BASE, x INT FIBER)").expect("CREATE b");
    for id in [3, 5] {
        run(&mut engine, &format!("INSERT INTO b (id, x) VALUES ({id}, 0)")).expect("ins b");
    }
    run(&mut engine, "DEFINE PATTERN p AS score >= 0 WEIGHT (score)").expect("DEFINE");

    let rows_ab = match run(
        &mut engine,
        "HUNT p IN candidates EXCLUDING IN a EXCLUDING IN b",
    )
    .expect("HUNT a-then-b")
    {
        ExecResult::Rows(rs) => rs,
        _ => unreachable!(),
    };
    let rows_ba = match run(
        &mut engine,
        "HUNT p IN candidates EXCLUDING IN b EXCLUDING IN a",
    )
    .expect("HUNT b-then-a")
    {
        ExecResult::Rows(rs) => rs,
        _ => unreachable!(),
    };
    assert_eq!(
        ids_of(&rows_ab),
        ids_of(&rows_ba),
        "EXCLUDING IN should be order-independent"
    );
}

// ─── PH16 — anti-join touches only PK, not fiber ────────────────────────────
//
// Behavioral proxy: an exclusion bundle with a TOTALLY DIFFERENT fiber
// schema from the target must still work. If the executor were touching
// fiber fields (decrypting them, type-checking them), schema mismatches
// in the exclusion bundle would cascade. The fact that this passes
// is evidence the executor only reads the PK column.

#[test]
fn ph16_excluding_in_works_with_schema_mismatched_exclusion_bundle() {
    let mut engine = fresh_engine();
    run(
        &mut engine,
        "CREATE BUNDLE candidates (id INT BASE, score INT FIBER)",
    )
    .expect("CREATE candidates");
    for id in 1..=5 {
        run(
            &mut engine,
            &format!("INSERT INTO candidates (id, score) VALUES ({id}, {id})"),
        )
        .expect("ins");
    }

    // Exclusion bundle has a vastly different fiber schema —
    // text fields, multiple fibers, no `score` field at all.
    // The anti-join must still work because it only reads `id`.
    run(
        &mut engine,
        "CREATE BUNDLE alien_schema (id INT BASE, name TEXT FIBER, category TEXT FIBER, weight INT FIBER)",
    )
    .expect("CREATE alien");
    for id in [3, 5] {
        run(
            &mut engine,
            &format!(
                "INSERT INTO alien_schema (id, name, category, weight) \
                 VALUES ({id}, 'name_{id}', 'cat_{id}', {id})"
            ),
        )
        .expect("ins alien");
    }

    run(&mut engine, "DEFINE PATTERN p AS score >= 0 WEIGHT (score)")
        .expect("DEFINE");
    let rows = match run(
        &mut engine,
        "HUNT p IN candidates EXCLUDING IN alien_schema",
    )
    .expect("HUNT must succeed; anti-join is PK-only")
    {
        ExecResult::Rows(rs) => rs,
        _ => unreachable!(),
    };
    assert_eq!(
        ids_of(&rows),
        vec![4, 2, 1],
        "exclusion by PK works regardless of exclusion bundle's fiber shape"
    );
}

// ─── Domain-neutrality smoke ────────────────────────────────────────────────

#[test]
fn excluding_in_works_across_consumer_styles() {
    let mut engine = fresh_engine();

    // Fraud-detection style: exclude already-cleared merchants.
    run(
        &mut engine,
        "CREATE BUNDLE txns (id INT BASE, amount INT FIBER)",
    )
    .expect("CREATE");
    for (id, amount) in [(101, 50), (102, 10000), (103, 200), (104, 99999)] {
        run(
            &mut engine,
            &format!("INSERT INTO txns (id, amount) VALUES ({id}, {amount})"),
        )
        .expect("ins");
    }
    run(
        &mut engine,
        "CREATE BUNDLE cleared (id INT BASE, x INT FIBER)",
    )
    .expect("CREATE cleared");
    // Pre-cleared: 102 (high amount but known legit).
    run(&mut engine, "INSERT INTO cleared (id, x) VALUES (102, 0)").expect("ins");

    run(&mut engine, "DEFINE PATTERN suspicious AS amount > 0 WEIGHT (amount)")
        .expect("DEFINE");

    let rows = match run(
        &mut engine,
        "HUNT suspicious IN txns EXCLUDING IN cleared TOP 2",
    )
    .expect("HUNT")
    {
        ExecResult::Rows(rs) => rs,
        _ => unreachable!(),
    };
    assert_eq!(rows.len(), 2);
    // Highest non-cleared amounts: 104 (99999), then 103 (200).
    assert_eq!(id_of(&rows[0]), 104);
    assert_eq!(id_of(&rows[1]), 103);
}
