//! INGEST … FORMAT JSONL — one object per line, through the parser.
//!
//! Policy under test: KEY is mandatory (JSON has no column order),
//! JSON types drive inference, numeric arrays become Vector fibers
//! with one declared dim, and every failure mode is loud with a line
//! number.

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
fn write_jsonl(dir: &std::path::Path, name: &str, body: &str) -> String {
    let p = dir.join(name);
    std::fs::write(&p, body).unwrap();
    common::ingest_rel_str(&p)
}

#[test]
fn jsonl_requires_key() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();
    let f = write_jsonl(dir.path(), "x.jsonl", r#"{"id":"a","v":1}"#);
    let err = run(&mut e, &format!("INGEST x FROM '{f}' FORMAT JSONL;")).unwrap_err();
    assert!(err.contains("KEY"), "{err}");
}

#[test]
fn jsonl_ingests_with_vectors() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();
    let f = write_jsonl(
        dir.path(),
        "emb.jsonl",
        concat!(
            r#"{"doc":"d1","label":"news","emb":[0.1,0.2,0.3]}"#, "\n",
            r#"{"doc":"d2","label":"spam","emb":[0.9,0.8,0.7]}"#, "\n",
            "\n", // blank lines are fine
            r#"{"doc":"d3","label":"news","emb":[0.2,0.1,0.4]}"#, "\n",
        ),
    );
    run(&mut e, &format!("INGEST docs FROM '{f}' FORMAT JSONL KEY doc;")).unwrap();
    match run(&mut e, "SHOW FIELDS ON docs;").unwrap() {
        ExecResult::Rows(rows) => {
            let by = |n: &str| {
                rows.iter()
                    .find(|r| r["field"] == Value::Text(n.into()))
                    .unwrap_or_else(|| panic!("no field {n}"))
            };
            assert_eq!(by("doc")["kind"], Value::Text("base".into()));
            let emb_ty = format!("{:?}", by("emb")["type"]);
            assert!(emb_ty.contains("Vector"), "emb should be Vector, got {emb_ty}");
        }
        other => panic!("expected rows, got {other:?}"),
    }
    match run(&mut e, "SECTION docs AT doc='d2';").unwrap() {
        ExecResult::Rows(rows) => match &rows[0]["emb"] {
            Value::Vector(v) => assert_eq!(v, &vec![0.9, 0.8, 0.7]),
            other => panic!("emb should round-trip as a vector, got {other:?}"),
        },
        other => panic!("expected rows, got {other:?}"),
    }
}

#[test]
fn jsonl_loud_failures() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();

    // invalid JSON names the line
    let f = write_jsonl(dir.path(), "bad.jsonl", "{\"id\":\"a\"}\nnot json\n");
    let err = run(&mut e, &format!("INGEST x FROM '{f}' FORMAT JSONL KEY id;")).unwrap_err();
    assert!(err.contains("line 2"), "{err}");

    // vector length drift names the line and the field
    let f = write_jsonl(
        dir.path(),
        "drift.jsonl",
        "{\"id\":\"a\",\"e\":[1,2]}\n{\"id\":\"b\",\"e\":[1,2,3]}\n",
    );
    let err = run(&mut e, &format!("INGEST y FROM '{f}' FORMAT JSONL KEY id;")).unwrap_err();
    assert!(err.contains("changed length") && err.contains("'e'"), "{err}");

    // nested objects are refused with advice
    let f = write_jsonl(dir.path(), "nest.jsonl", "{\"id\":\"a\",\"meta\":{\"x\":1}}\n");
    let err = run(&mut e, &format!("INGEST z FROM '{f}' FORMAT JSONL KEY id;")).unwrap_err();
    assert!(err.contains("flatten"), "{err}");

    // KEY not present in any object lists the fields it did see
    let f = write_jsonl(dir.path(), "nokey.jsonl", "{\"id\":\"a\",\"v\":1}\n");
    let err = run(&mut e, &format!("INGEST w FROM '{f}' FORMAT JSONL KEY nope;")).unwrap_err();
    assert!(err.contains("nope") && err.contains("id"), "{err}");
}
