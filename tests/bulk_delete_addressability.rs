//! A delete must address the rows its predicate matched, or say it could not.
//!
//! Reported 2026-09-20 by the Marcella corpus repair. A delete-then-insert over
//! a short-header bundle deleted the 27 rows it had just written and left all
//! 28 originals in place, returning a success and a count. The corpus ended up
//! holding both the damaged rows and the repaired ones.
//!
//! Two separate faults, one per storage path.
//!
//! On the heap, `bulk_delete` matched by predicate and then rebuilt a key from
//! the record's own fields and hashed it. When the key column is unreadable
//! every matched row rebuilds the same all-null key, which hashes to a point
//! where nothing lives.
//!
//! On the mmap path a tombstone IS the primary key, so a base row whose key
//! column is unreadable cannot be tombstoned at all. That row was dropped
//! silently and not even counted.
use gigi::engine::Engine;
use gigi::bundle::QueryCondition;
use gigi::types::{BundleSchema, FieldDef, Record, Value};
use std::fs;

fn dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("gigi_bd_{tag}"));
    let _ = fs::remove_dir_all(&d);
    d
}

fn schema() -> BundleSchema {
    BundleSchema::new("sections")
        .base(FieldDef::categorical("section_id"))
        .fiber(FieldDef::categorical("doc_id"))
        .fiber(FieldDef::categorical("content"))
}

fn rec(id: &str, doc: &str) -> Record {
    let mut r = Record::new();
    r.insert("section_id".into(), Value::Text(id.into()));
    r.insert("doc_id".into(), Value::Text(doc.into()));
    r.insert("content".into(), Value::Text("x".into()));
    r
}

fn eq(field: &str, v: &str) -> QueryCondition {
    QueryCondition::Eq(field.to_string(), Value::Text(v.into()))
}

/// The baseline, and the thing the old implementation got right by accident:
/// with readable keys, a filtered delete removes exactly the matched rows.
#[test]
fn a_filtered_delete_removes_every_matched_row() {
    let d = dir("basic");
    let mut e = Engine::open(&d).unwrap();
    e.create_bundle(schema()).unwrap();
    for i in 0..6 {
        e.insert("sections", &rec(&format!("s{i}"), if i < 4 { "keep" } else { "drop" }))
            .unwrap();
    }
    let mut store = e.bundle_mut("sections").unwrap();
    let removed = store.bulk_delete(&[eq("doc_id", "drop")]);
    let left = e.bundle("sections").unwrap().records().count();
    drop(e);
    let _ = fs::remove_dir_all(&d);
    assert_eq!(removed, 2, "both matching rows removed");
    assert_eq!(left, 4);
}

/// Rows sharing a key value are each removed, not just the first.
///
/// The old implementation deleted by rebuilt key, so a second row rebuilding an
/// identical key found nothing at that point and was left behind.
#[test]
fn rows_that_rebuild_the_same_key_are_all_removed() {
    let d = dir("dupkey");
    let mut e = Engine::open(&d).unwrap();
    // Two base fields: rows can differ in storage while one key column repeats.
    let s = BundleSchema::new("two")
        .base(FieldDef::categorical("a"))
        .base(FieldDef::categorical("b"))
        .fiber(FieldDef::categorical("tag"));
    e.create_bundle(s).unwrap();
    for i in 0..4 {
        let mut r = Record::new();
        r.insert("a".into(), Value::Text("same".into()));
        r.insert("b".into(), Value::Text(format!("b{i}")));
        r.insert("tag".into(), Value::Text("drop".into()));
        e.insert("two", &r).unwrap();
    }
    let mut store = e.bundle_mut("two").unwrap();
    let removed = store.bulk_delete(&[eq("tag", "drop")]);
    let left = e.bundle("two").unwrap().records().count();
    drop(e);
    let _ = fs::remove_dir_all(&d);
    assert_eq!(removed, 4, "every matched row removed, not one per key value");
    assert_eq!(left, 0);
}

/// A healthy bundle reports nothing unaddressable, so the signal is a gate.
#[test]
fn a_healthy_bundle_reports_nothing_unaddressable() {
    let d = dir("clean");
    let mut e = Engine::open(&d).unwrap();
    e.create_bundle(schema()).unwrap();
    e.insert("sections", &rec("s1", "d1")).unwrap();
    let mut store = e.bundle_mut("sections").unwrap();
    store.bulk_delete(&[eq("doc_id", "d1")]);
    let un = store.unaddressable_last_bulk_delete();
    drop(e);
    let _ = fs::remove_dir_all(&d);
    assert_eq!(un, 0, "nothing should be unaddressable on a readable bundle");
}

/// The real failing case, and the only one that distinguishes the fix.
///
/// The three tests above pass whether the delete addresses a matched row by its
/// own base point or by a key rebuilt from its fields, because on a healthy
/// bundle those are the same point. They are controls that cannot fire. This
/// one builds the state the Marcella corpus is actually in: a snapshot whose
/// header omits the key column, so the rows are there, a predicate matches
/// them, and the key they would be addressed by reads back as absent.
#[test]
fn rows_whose_key_column_is_unreadable_are_reported_not_silently_skipped() {
    let d = dir("shorthdr");
    {
        let mut e = Engine::open(&d).unwrap();
        e.create_bundle(schema()).unwrap();
        for i in 0..5 {
            e.insert("sections", &rec(&format!("s{i}"), "doomed")).unwrap();
        }
        e.snapshot_with_chunk_size_report(50_000, None).expect("snapshot");
    }

    // Drop the key column from the written header, the way the old encoder did.
    let p = d.join("snapshots").join("sections.dhoom");
    let txt = fs::read_to_string(&p).unwrap();
    let mut lines: Vec<&str> = txt.split('\n').collect();
    let hdr = lines[0].to_string();
    let (name, rest) = hdr.split_once('{').unwrap();
    let inner = rest.trim_end_matches(':').trim_end_matches('}');
    let kept: Vec<&str> = inner
        .split(", ")
        .filter(|c| !c.trim_start_matches(|ch: char| !ch.is_alphanumeric()).starts_with("section_id"))
        .collect();
    assert!(kept.len() < inner.split(", ").count(), "the fixture must drop the key column");
    let damaged = format!("{}{{{}}}:", name, kept.join(", "));
    lines[0] = &damaged;
    fs::write(&p, lines.join("\n")).unwrap();

    let mut e = Engine::open_mmap(&d).expect("reopen mmap");
    let before = e.bundle("sections").unwrap().records().count();
    let key_readable = e
        .bundle("sections")
        .unwrap()
        .records()
        .filter(|r| r.get("section_id").is_some())
        .count();

    let mut store = e.bundle_mut("sections").unwrap();
    let deleted = store.bulk_delete(&[eq("doc_id", "doomed")]);
    let unaddressable = store.unaddressable_last_bulk_delete();
    let after = e.bundle("sections").unwrap().records().count();
    drop(e);
    let _ = fs::remove_dir_all(&d);

    assert_eq!(before, 5, "the rows are present");
    assert_eq!(key_readable, 0, "and their key column is not readable");
    assert_eq!(
        unaddressable, 5,
        "every matched row the engine cannot address must be COUNTED, not dropped in silence"
    );
    assert_eq!(deleted, 0, "none of them can actually be deleted");
    assert_eq!(after, 5, "so they are all still there, which the caller must be told");
}
