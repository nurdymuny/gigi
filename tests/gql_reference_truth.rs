//! The table that confesses — automated.
//!
//! `GQL_REFERENCE.md` carries an implementation-status table. This test
//! makes the ✅ rows produce a receipt: one statement per claimed-working
//! feature, executed against a real engine, asserting it parses AND
//! executes without error. When a doc row and the engine disagree, this
//! file fails instead of a reader's afternoon.
//!
//! Two kinds of entries:
//! - `works(stmt)`  — documented ✅ and must succeed.
//! - `honest_gap(stmt, why)` — documented ✅ historically but currently
//!   refused by the engine; asserted to ERROR (not silently no-op), with
//!   the gap recorded here. Fixing the feature flips the entry to works().
//!
//! Run: cargo test --release --test gql_reference_truth

use gigi::engine::Engine;
use gigi::parser::{self, ExecResult};

fn run(engine: &mut Engine, stmt: &str) -> Result<ExecResult, String> {
    let ast = parser::parse(stmt)?;
    parser::execute(engine, &ast)
}

fn seeded_engine(dir: &std::path::Path) -> Engine {
    let mut e = Engine::open(dir).expect("engine open");
    for stmt in [
        "BUNDLE sensors BASE (id TEXT) FIBER (city TEXT INDEX, temp NUMERIC, wind NUMERIC);",
        "SECTION sensors (id='s1', city='Moscow', temp=-3.0, wind=5.0);",
        "SECTION sensors (id='s2', city='Moscow', temp=-25.5, wind=8.5);",
        "SECTION sensors (id='s3', city='Lagos', temp=31.0, wind=2.0);",
        "SECTION sensors (id='s4', city='Lagos', temp=29.5, wind=3.5);",
        "SECTION sensors (id='s5', city='Lagos', temp=30.2, wind=4.1);",
        "BUNDLE cities BASE (city TEXT) FIBER (region TEXT);",
        "SECTION cities (city='Moscow', region='EU');",
        "SECTION cities (city='Lagos', region='AF');",
    ] {
        run(&mut e, stmt).unwrap_or_else(|err| panic!("seed failed: {stmt}: {err}"));
    }
    e
}

/// Every statement here is marked ✅ in GQL_REFERENCE.md and must execute.
#[test]
fn documented_features_execute() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = seeded_engine(dir.path());
    let works: &[&str] = &[
        // point reads
        "SECTION sensors AT id='s1';",
        "EXISTS SECTION sensors AT id='s1';",
        "SECTION sensors AT id='s1' PROJECT (temp);",
        // covers
        "COVER sensors ALL;",
        "COVER sensors ON city = 'Moscow';",
        "COVER sensors WHERE temp < -20;",
        "COVER sensors ON city = 'Moscow' WHERE temp < -20;",
        "COVER sensors ON city IN ('Moscow', 'Lagos') WHERE wind > 3;",
        "COVER sensors DISTINCT city;",
        "COVER sensors ON city = 'Moscow' RANK BY temp ASC;",
        "COVER sensors RANK BY temp DESC FIRST 2;",
        "COVER sensors RANK BY temp DESC SKIP 1 FIRST 2;",
        "COVER sensors WHERE city MATCHES 'Mos*';",
        "COVER sensors WHERE temp DEFINED;",
        "COVER sensors PROJECT (id, temp);",
        // writes
        "REDEFINE sensors AT id='s4' SET (wind=4.0);",
        "RETRACT sensors AT id='s4';",
        "SECTION sensors (id='s4', city='Lagos', temp=29.5, wind=3.5);",
        // aggregation
        "INTEGRATE sensors OVER city MEASURE count(*), avg(temp), min(temp), max(wind);",
        "INTEGRATE sensors MEASURE count(*), sum(wind);",
        // joins
        "PULLBACK sensors ALONG city ONTO cities;",
        // admin / introspection
        "SHOW BUNDLES;",
        "DESCRIBE sensors;",
        "EXPLAIN COVER sensors ON city = 'Moscow';",
        // geometry ride-alongs
        "CURVATURE sensors;",
        "SPECTRAL sensors;",
        "HEALTH sensors;",
        // this audit's addition (global form: needs >= 4 ordered samples)
        "INTEGRATE sensors MEASURE avg(temp) WITH JACKKNIFE ALONG wind;",
        // thermalization cut: drop the first n ordered samples per group
        "INTEGRATE sensors MEASURE avg(temp) WITH JACKKNIFE ALONG wind SKIP FIRST 1;",
        // information schema: field names/kinds/types as rows
        "SHOW FIELDS ON sensors;",
    ];
    let mut failures = Vec::new();
    for stmt in works {
        if let Err(err) = run(&mut e, stmt) {
            failures.push(format!("  {stmt}\n    -> {err}"));
        }
    }
    assert!(
        failures.is_empty(),
        "\n{} documented-✅ statement(s) failed against the engine — either \
         fix the engine or fix GQL_REFERENCE.md:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Documented gaps: these were listed ✅ in older revisions of the
/// reference but are NOT implemented. The contract this test enforces is
/// honesty — they must ERROR loudly, never silently no-op or return a
/// wrong answer. Implementing one flips it into `documented_features_execute`.
#[test]
fn known_gaps_error_loudly() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = seeded_engine(dir.path());
    let gaps: &[(&str, &str)] = &[
        (
            "INTEGRATE sensors OVER city MEASURE avg(temp) HAVING avg(temp) > 0;",
            "HAVING has no parser support; trailing-token rejection must catch it",
        ),
        (
            "FIBER RANK sensors OVER city RANK BY temp;",
            "window functions are not implemented",
        ),
        (
            "COVER sensors WHERE nonexistent_field > 1;",
            "unknown fields must error with the field list, not match nothing",
        ),
        // discovered by this test's first run — all four were marked ✅ in
        // the reference and none of them parse:
        ("PRODUCT sensors WITH cities;", "PRODUCT is not implemented"),
        (
            "UNION (COVER sensors ON city = 'Moscow') WITH (COVER sensors ON city = 'Lagos');",
            "set operations are not implemented",
        ),
        (
            "INTERSECT (COVER sensors WHERE temp > 0) WITH (COVER sensors WHERE wind > 3);",
            "set operations are not implemented",
        ),
        (
            "SUBTRACT (COVER sensors ALL) MINUS (COVER sensors WHERE temp > 0);",
            "set operations are not implemented",
        ),
    ];
    for (stmt, why) in gaps {
        match run(&mut e, stmt) {
            Err(_) => {} // loud refusal — the honest outcome
            Ok(_) => panic!(
                "'{stmt}' succeeded but should error ({why}); if it was \
                 implemented, move it to documented_features_execute"
            ),
        }
    }
}

/// Section XII: EMIT CSV parses and executes, but only behind the
/// GIGI_EMIT_DIR gate — on an engine without the gate (every default
/// server) it must refuse loudly and name the knob, never run the
/// inner statement with the export silently dropped. The full export
/// contract is enforced in tests/emit_csv.rs.
#[test]
fn emit_without_gate_refused_loudly() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = seeded_engine(dir.path());
    std::env::remove_var("GIGI_EMIT_DIR");
    let err = run(&mut e, "COVER sensors ALL EMIT CSV TO 'x.csv';")
        .expect_err("EMIT without GIGI_EMIT_DIR must error");
    assert!(err.contains("GIGI_EMIT_DIR"), "error should name the knob: {err}");
}

/// Section VIII: transaction control is NOT implemented, and must not
/// pretend otherwise.
///
/// `ExecResult::Notice` exists because (per its own doc comment, audit
/// 2026-07-02) "plain `Ok` here would be success theater." That audit
/// caught COMPACT/VACUUM/ANALYZE and missed BEGIN/COMMIT/ROLLBACK,
/// which kept returning a bare `Ok` — and on `/v1/gql`, a bare
/// HTTP 200 `{"status":"ok"}`. A caller could send
/// `BEGIN; INSERT …; ROLLBACK;`, receive three successes, and find the
/// row still there.
///
/// This test pins both halves of the truth:
///   1. all three verbs return `Notice`, never a bare `Ok`;
///   2. ROLLBACK genuinely does not undo the write.
///
/// When transaction control is actually implemented, THIS TEST MUST
/// FAIL — that is the point. Fixing it means flipping GQL_REFERENCE.md
/// §VIII back to ✅ in the same commit.
#[test]
fn transaction_control_does_not_pretend_to_work() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = seeded_engine(dir.path());

    for stmt in [
        "BEGIN;",
        "BEGIN TRANSACTION;",
        "COMMIT;",
        "ROLLBACK;",
        "ATLAS BEGIN;",
        "ATLAS COMMIT;",
        "ATLAS ROLLBACK;",
    ] {
        match run(&mut e, stmt).unwrap_or_else(|err| panic!("'{stmt}' should parse: {err}")) {
            ExecResult::Notice(msg) => {
                assert!(
                    msg.contains("NOT implemented"),
                    "'{stmt}' notice must say it did nothing, got: {msg}"
                );
            }
            other => panic!(
                "'{stmt}' returned {other:?} — a bare Ok is success theater for a \
                 verb that opens no transaction. Either return Notice, or \
                 implement transactions and update GQL_REFERENCE.md §VIII."
            ),
        }
    }

    // The behavioral half: ROLLBACK does not roll back.
    run(&mut e, "BEGIN;").unwrap();
    run(
        &mut e,
        "SECTION sensors (id='tx_probe', city='Nowhere', temp=1.0, wind=1.0);",
    )
    .unwrap();
    run(&mut e, "ROLLBACK;").unwrap();

    let after = run(&mut e, "SECTION sensors AT id='tx_probe';")
        .expect("point read after rollback");
    match after {
        ExecResult::Rows(rows) => assert_eq!(
            rows.len(),
            1,
            "documented behavior: ROLLBACK does NOT undo the write. If this now \
             reads 0 rows, transactions were implemented — update \
             GQL_REFERENCE.md §VIII and this test together."
        ),
        other => panic!("expected Rows from the point read, got {other:?}"),
    }
}

/// Section VIII: the ATLAS sub-forms the reference used to show as
/// working are parse errors. Kept loud so the reference cannot quietly
/// re-promise them.
#[test]
fn atlas_subforms_are_parse_errors() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = seeded_engine(dir.path());
    for stmt in [
        "ATLAS SAVEPOINT cp1;",
        "ATLAS ROLLBACK TO cp1;",
        "ATLAS BEGIN ISOLATION FLAT;",
        "ATLAS BEGIN ISOLATION CURVED;",
    ] {
        assert!(
            run(&mut e, stmt).is_err(),
            "'{stmt}' is documented in GQL_REFERENCE.md §VIII but must refuse \
             loudly until it is built"
        );
    }
}
