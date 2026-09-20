//! Does an insert after a snapshot APPEND, or replace what is there?
//!
//! Reported 2026-09-20 against the deployed gigi-stream by the Marcella corpus
//! repair: 25 records sent, the reply says `count: 25`, the bundle holds 1.
//! Six separate single-record calls left exactly one record, the last. Their
//! timeline puts it after an engine-wide snapshot and reload, and a batched
//! 13,371-record ingest had worked a week earlier.
//!
//! If that is real it is the worst class of defect this engine can have: a
//! delete-then-insert repair against it destroys the corpus while every reply
//! says success.
//!
//! These reproduce the timeline locally rather than reasoning about it.
use gigi::engine::Engine;
use gigi::types::{BundleSchema, FieldDef, Record, Value};
use std::fs;

fn dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("gigi_ins_{tag}"));
    let _ = fs::remove_dir_all(&d);
    d
}

fn schema() -> BundleSchema {
    BundleSchema::new("sections")
        .base(FieldDef::categorical("section_id"))
        .fiber(FieldDef::categorical("content"))
        .fiber(FieldDef::categorical("doc_id"))
}

fn rec(id: &str, content: &str) -> Record {
    let mut r = Record::new();
    r.insert("section_id".into(), Value::Text(id.into()));
    r.insert("content".into(), Value::Text(content.into()));
    r.insert("doc_id".into(), Value::Text("d1".into()));
    r
}

fn count(e: &Engine, name: &str) -> usize {
    e.bundle(name).expect("bundle").records().count()
}

/// The baseline: on a fresh heap bundle, six inserts are six records.
#[test]
fn inserts_append_on_a_fresh_bundle() {
    let d = dir("fresh");
    let mut e = Engine::open(&d).unwrap();
    e.create_bundle(schema()).unwrap();
    for i in 0..6 {
        e.insert("sections", &rec(&format!("s{i}"), "x")).unwrap();
    }
    let n = count(&e, "sections");
    drop(e);
    let _ = fs::remove_dir_all(&d);
    assert_eq!(n, 6, "six distinct keys must be six records");
}

/// Their timeline: write, snapshot, reopen memory-mapped, then insert.
#[test]
fn inserts_append_after_a_snapshot_and_reload() {
    let d = dir("after_snap");
    {
        let mut e = Engine::open(&d).unwrap();
        e.create_bundle(schema()).unwrap();
        for i in 0..4 {
            e.insert("sections", &rec(&format!("base{i}"), "x")).unwrap();
        }
        e.snapshot_with_chunk_size_report(50_000, None)
            .expect("snapshot");
    }

    let mut e = Engine::open_mmap(&d).expect("reopen mmap");
    let before = count(&e, "sections");
    for i in 0..6 {
        e.insert("sections", &rec(&format!("new{i}"), "y")).unwrap();
    }
    let after = count(&e, "sections");
    drop(e);
    let _ = fs::remove_dir_all(&d);

    assert_eq!(before, 4, "the snapshot must hold the original four");
    assert_eq!(
        after, 10,
        "four existing plus six new; getting {after} means an insert replaced \
         rather than appended"
    );
}

/// A batch through the same door, which is what their repair would use.
#[test]
fn a_batch_after_a_snapshot_stores_every_record() {
    let d = dir("batch_after_snap");
    {
        let mut e = Engine::open(&d).unwrap();
        e.create_bundle(schema()).unwrap();
        e.insert("sections", &rec("seed", "x")).unwrap();
        e.snapshot_with_chunk_size_report(50_000, None)
            .expect("snapshot");
    }
    let mut e = Engine::open_mmap(&d).expect("reopen mmap");
    let batch: Vec<Record> = (0..25).map(|i| rec(&format!("b{i:03}"), "y")).collect();
    for r in &batch {
        e.insert("sections", r).unwrap();
    }
    let n = count(&e, "sections");
    drop(e);
    let _ = fs::remove_dir_all(&d);
    assert_eq!(n, 26, "one seed plus twenty-five; got {n}");
}

/// The mechanism that reproduces their symptom exactly, now refused.
///
/// A record with no base field is stored with a null key, so every such record
/// lands on the same base point and overwrites the last: twenty-five inserts
/// leave one row, and the reply says twenty-five. That is what they saw.
///
/// It is now a refusal, and the message says what would have happened.
#[test]
fn a_record_with_no_key_is_refused_rather_than_silently_collapsing() {
    let d = dir("nokey");
    let mut e = Engine::open(&d).unwrap();
    e.create_bundle(schema()).unwrap();

    let mut r = Record::new();
    r.insert("content".into(), Value::Text("c0".into()));
    r.insert("doc_id".into(), Value::Text("d1".into()));
    let got = e.insert("sections", &r);
    let n = count(&e, "sections");
    drop(e);
    let _ = fs::remove_dir_all(&d);

    let msg = got.expect_err("a record with no key is not a record").to_string();
    assert!(
        msg.contains("section_id"),
        "the refusal must name the missing field; got: {msg}"
    );
    assert!(
        msg.contains("overwrites"),
        "the refusal must say what would have happened; got: {msg}"
    );
    assert_eq!(n, 0, "nothing should have been stored");
}

/// A batch is refused whole, not partly applied.
#[test]
fn a_batch_containing_a_keyless_record_is_refused_whole() {
    let d = dir("nokey_batch");
    let mut e = Engine::open(&d).unwrap();
    e.create_bundle(schema()).unwrap();

    let mut bad = Record::new();
    bad.insert("content".into(), Value::Text("no key".into()));
    let batch = vec![rec("a", "x"), bad, rec("b", "y")];
    let got = e.batch_insert("sections", &batch);
    let n = count(&e, "sections");
    drop(e);
    let _ = fs::remove_dir_all(&d);

    assert!(got.is_err(), "one keyless record must refuse the batch");
    assert_eq!(n, 0, "a refused batch must store nothing, stored {n}");
}
