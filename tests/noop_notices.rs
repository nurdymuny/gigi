//! Silent success is a lie the database tells. Statements that parse
//! and validate but intentionally do nothing must say so — the
//! ExecResult::Notice contract (audit 2026-07-02).

use gigi::engine::Engine;
use gigi::parser::{self, ExecResult};

fn engine_with_bundle(dir: &std::path::Path) -> Engine {
    let mut e = Engine::open(dir).expect("engine open");
    let ast = parser::parse(
        "BUNDLE sensors BASE (id TEXT) FIBER (temp NUMERIC);",
    )
    .unwrap();
    parser::execute(&mut e, &ast).unwrap();
    e
}

#[test]
fn maintenance_verbs_return_notice_not_bare_ok() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = engine_with_bundle(dir.path());
    for stmt in [
        "COMPACT sensors;",
        "ANALYZE sensors;",
        "VACUUM sensors;",
        "REBUILD INDEX sensors;",
        "CHECK sensors;",
        "REPAIR sensors;",
    ] {
        let ast = parser::parse(stmt).unwrap_or_else(|e| panic!("{stmt}: {e}"));
        match parser::execute(&mut e, &ast) {
            Ok(ExecResult::Notice(msg)) => assert!(
                msg.contains("not implemented") || msg.contains("nothing was"),
                "{stmt}: notice must say what did not happen, got: {msg}"
            ),
            other => panic!(
                "{stmt}: a no-op maintenance verb must return Notice, got {other:?}"
            ),
        }
    }
}

#[test]
fn maintenance_on_missing_bundle_still_errors() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();
    let ast = parser::parse("COMPACT ghosts;").unwrap();
    assert!(parser::execute(&mut e, &ast).is_err());
}

#[test]
fn session_stubs_return_notice() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = engine_with_bundle(dir.path());
    let ast = parser::parse("SET timeout 5;").unwrap();
    match parser::execute(&mut e, &ast) {
        Ok(ExecResult::Notice(msg)) => {
            assert!(msg.contains("no effect") || msg.contains("not implemented"), "{msg}")
        }
        other => panic!("SET must return Notice, got {other:?}"),
    }
}

#[test]
fn show_fields_returns_real_rows() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();
    let ast = parser::parse(
        "BUNDLE sensors BASE (id TEXT) FIBER (city TEXT INDEX, temp NUMERIC);",
    )
    .unwrap();
    parser::execute(&mut e, &ast).unwrap();

    let ast = parser::parse("SHOW FIELDS ON sensors;").unwrap();
    match parser::execute(&mut e, &ast).unwrap() {
        ExecResult::Rows(rows) => {
            assert_eq!(rows.len(), 3, "id + city + temp");
            let by_name = |n: &str| {
                rows.iter()
                    .find(|r| r["field"] == gigi::types::Value::Text(n.into()))
                    .unwrap_or_else(|| panic!("no row for field {n}"))
            };
            assert_eq!(by_name("id")["kind"], gigi::types::Value::Text("base".into()));
            assert_eq!(by_name("temp")["kind"], gigi::types::Value::Text("fiber".into()));
            assert_eq!(by_name("city")["indexed"], gigi::types::Value::Bool(true));
            assert_eq!(by_name("temp")["indexed"], gigi::types::Value::Bool(false));
        }
        other => panic!("SHOW FIELDS must return rows, got {other:?}"),
    }
}
