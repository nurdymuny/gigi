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

/// Section XII honesty: EMIT is documented ❌ and must refuse loudly —
/// the COVER must NOT run with the export silently dropped.
#[test]
fn emit_clause_refused_loudly() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = seeded_engine(dir.path());
    let err = run(&mut e, "COVER sensors ALL EMIT CSV TO 'x.csv';")
        .expect_err("EMIT is not implemented and must error");
    assert!(err.contains("EMIT"), "error should name the refused clause: {err}");
}
