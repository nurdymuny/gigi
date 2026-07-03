//! INGEST … FORMAT CSV — end-to-end through the parser executor.
//!
//! Policy under test (documented in GQL_REFERENCE.md):
//! header row names the fields; base key = KEY clause or first column;
//! numeric-unless-proven-otherwise column typing; loud errors for a
//! missing KEY column, a header with no rows, and an empty base cell.

use gigi::engine::Engine;
use gigi::parser::{self, ExecResult};
use gigi::types::Value;

mod common;

fn run(e: &mut Engine, stmt: &str) -> Result<ExecResult, String> {
    let ast = parser::parse(stmt)?;
    parser::execute(e, &ast)
}

/// Writes the fixture under the tempdir and returns the GIGI_INGEST_DIR-
/// relative source string the gated INGEST verb requires.
fn write_csv(dir: &std::path::Path, name: &str, body: &str) -> String {
    let p = dir.join(name);
    std::fs::write(&p, body).unwrap();
    common::ingest_rel_str(&p)
}

#[test]
fn csv_ingest_autocreates_and_loads() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();
    let csv = write_csv(
        dir.path(),
        "stations.csv",
        "station_id,city,temp\n\
         s1,Moscow,-3.0\n\
         s2,Moscow,-25.5\n\
         s3,\"Lagos, Island\",31.0\n\
         s4,Lagos,29.5\n",
    );
    run(&mut e, &format!("INGEST stations FROM '{csv}' FORMAT CSV;")).unwrap();

    // 4 records landed
    match run(&mut e, "COVER stations ALL;").unwrap() {
        ExecResult::Rows(rows) => assert_eq!(rows.len(), 4),
        other => panic!("expected rows, got {other:?}"),
    }
    // first column became the base key; types inferred
    match run(&mut e, "SHOW FIELDS ON stations;").unwrap() {
        ExecResult::Rows(rows) => {
            let by = |n: &str| {
                rows.iter()
                    .find(|r| r["field"] == Value::Text(n.into()))
                    .unwrap_or_else(|| panic!("no field row for {n}"))
            };
            assert_eq!(by("station_id")["kind"], Value::Text("base".into()));
            assert_eq!(by("city")["kind"], Value::Text("fiber".into()));
            assert!(
                format!("{:?}", by("temp")["type"]).contains("Numeric"),
                "temp should infer Numeric, got {:?}",
                by("temp")["type"]
            );
            assert!(
                format!("{:?}", by("city")["type"]).contains("Categorical"),
                "city should infer Categorical, got {:?}",
                by("city")["type"]
            );
        }
        other => panic!("expected rows, got {other:?}"),
    }
    // the numeric column aggregates; the quoted comma survived
    match run(&mut e, "INTEGRATE stations MEASURE count(*), avg(temp);").unwrap() {
        ExecResult::Rows(rows) => {
            let avg = rows[0]
                .iter()
                .find(|(k, _)| k.contains("avg"))
                .and_then(|(_, v)| v.as_f64())
                .expect("avg(temp) column present");
            assert!((avg - 8.0).abs() < 0.01, "avg temp = (-3-25.5+31+29.5)/4 = 8, got {avg}");
        }
        other => panic!("expected rows, got {other:?}"),
    }
    match run(&mut e, "SECTION stations AT station_id='s3';").unwrap() {
        ExecResult::Rows(rows) => {
            assert_eq!(rows[0]["city"], Value::Text("Lagos, Island".into()));
        }
        other => panic!("expected rows, got {other:?}"),
    }
}

#[test]
fn csv_key_clause_overrides_base_column() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();
    let csv = write_csv(
        dir.path(),
        "mols.csv",
        "mw,chembl_id\n180.16,CHEMBL25\n206.28,CHEMBL521\n",
    );
    run(&mut e, &format!("INGEST chem FROM '{csv}' FORMAT CSV KEY chembl_id;")).unwrap();
    match run(&mut e, "SHOW FIELDS ON chem;").unwrap() {
        ExecResult::Rows(rows) => {
            let key_row = rows
                .iter()
                .find(|r| r["field"] == Value::Text("chembl_id".into()))
                .unwrap();
            assert_eq!(key_row["kind"], Value::Text("base".into()));
        }
        other => panic!("expected rows, got {other:?}"),
    }
}

#[test]
fn csv_bad_key_lists_columns() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();
    let csv = write_csv(dir.path(), "x.csv", "a,b\n1,2\n");
    let err = run(&mut e, &format!("INGEST x FROM '{csv}' FORMAT CSV KEY nope;")).unwrap_err();
    assert!(err.contains("nope") && err.contains("a, b"), "{err}");
}

#[test]
fn csv_header_only_is_loud() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();
    let csv = write_csv(dir.path(), "empty.csv", "a,b\n");
    let err = run(&mut e, &format!("INGEST x FROM '{csv}' FORMAT CSV;")).unwrap_err();
    assert!(err.contains("no data rows"), "{err}");
}

#[test]
fn csv_empty_base_cell_is_loud() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();
    let csv = write_csv(dir.path(), "gap.csv", "id,v\nr1,1\n,2\n");
    let err = run(&mut e, &format!("INGEST x FROM '{csv}' FORMAT CSV;")).unwrap_err();
    assert!(err.contains("base-key column"), "{err}");
}

#[test]
fn csv_mixed_column_goes_categorical() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();
    let csv = write_csv(dir.path(), "mixed.csv", "id,v\nr1,12\nr2,twelve\n");
    run(&mut e, &format!("INGEST x FROM '{csv}' FORMAT CSV;")).unwrap();
    match run(&mut e, "SECTION x AT id='r1';").unwrap() {
        ExecResult::Rows(rows) => {
            // the numeric-looking value in a mixed column is stored as text
            assert_eq!(rows[0]["v"], Value::Text("12".into()));
        }
        other => panic!("expected rows, got {other:?}"),
    }
}
