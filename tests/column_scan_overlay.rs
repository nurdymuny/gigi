//! `column` on a memory-mapped bundle must agree with `records()`, row for row.
//!
//! The heap tests cover `BundleStore`. This covers the other storage the engine
//! hands back: after `Engine::open_mmap` a bundle is an overlay — an immutable
//! mmap base, plus an in-memory overlay for later writes, plus tombstones for
//! deletions. `records()` there yields overlay rows first and then base rows
//! that are neither deleted nor superseded, so a column read has to reproduce
//! that ordering and that filtering, not merely the values.

use std::path::PathBuf;

use gigi::engine::Engine;
use gigi::types::{BundleSchema, FieldDef, Record, Value};

fn tmp(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!(
        "gigi_column_overlay_{name}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    p
}

fn schema() -> BundleSchema {
    BundleSchema::new("edges")
        .base(FieldDef::numeric("edge_id"))
        .base(FieldDef::numeric("vertex_a"))
        .fiber(FieldDef::numeric("weight"))
}

fn row(id: i64, u: i64, w: f64) -> Record {
    let mut r = Record::new();
    r.insert("edge_id".into(), Value::Integer(id));
    r.insert("vertex_a".into(), Value::Integer(u));
    r.insert("weight".into(), Value::Float(w));
    r
}

/// Build a snapshot, reopen it memory-mapped, and hand the reopened engine over.
fn stored(root: &PathBuf, n: i64) -> Engine {
    std::fs::create_dir_all(root).unwrap();
    {
        let mut e = Engine::open(root).unwrap();
        e.compaction_policy_mut().disabled = true;
        e.create_bundle(schema()).unwrap();
        for i in 0..n {
            e.insert("edges", &row(i, i % 13, 1.0 + i as f64 * 0.5))
                .unwrap();
        }
        e.snapshot().unwrap();
    }
    Engine::open_mmap(root).unwrap()
}

fn assert_columns_match_records(e: &Engine, fields: &[&str]) {
    let b = e.bundle("edges").unwrap();
    let via_records: Vec<Record> = b.records().collect();
    for f in fields {
        let col = b.column(f).unwrap_or_else(|| panic!("column {f} missing"));
        assert_eq!(
            col.len(),
            via_records.len(),
            "column {f} length differs from records()"
        );
        for (i, rec) in via_records.iter().enumerate() {
            let expected = rec.get(*f).cloned().unwrap_or(Value::Null);
            assert_eq!(
                col[i], expected,
                "column {f} differs from records() at row {i}"
            );
        }
    }
}

#[test]
fn mmap_base_column_matches_records() {
    let root = tmp("base");
    let e = stored(&root, 500);
    assert_eq!(e.bundle("edges").unwrap().len(), 500);
    assert_columns_match_records(&e, &["edge_id", "vertex_a", "weight"]);
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn overlay_writes_shadow_the_base_in_column_order() {
    // Rows written after the snapshot live in the overlay and must appear first,
    // with the base rows they supersede dropped — exactly as records() does it.
    let root = tmp("shadow");
    let mut e = stored(&root, 200);
    for i in 0..20i64 {
        e.insert("edges", &row(i * 7, 99, -1.0 * i as f64)).unwrap();
    }
    let b = e.bundle("edges").unwrap();
    let n_records = b.records().count();
    assert_eq!(
        b.column("weight").unwrap().len(),
        n_records,
        "column must drop the same superseded rows records() drops"
    );
    drop(b);
    assert_columns_match_records(&e, &["edge_id", "vertex_a", "weight"]);
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn two_columns_zip_row_aligned_on_mmap() {
    let root = tmp("zip");
    let mut e = stored(&root, 300);
    for i in 0..15i64 {
        e.insert("edges", &row(i * 11, 77, 500.0 + i as f64)).unwrap();
    }
    let b = e.bundle("edges").unwrap();
    let tails = b.numeric_column("vertex_a").expect("vertex_a numeric");
    let weights = b.numeric_column("weight").expect("weight numeric");
    let expected: Vec<(f64, f64)> = b
        .records()
        .map(|r| {
            (
                r["vertex_a"].as_f64().unwrap(),
                r["weight"].as_f64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        tails.into_iter().zip(weights).collect::<Vec<_>>(),
        expected,
        "zipped mmap columns must give the pairs that share a record"
    );
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn absent_field_is_none_on_mmap() {
    let root = tmp("absent");
    let e = stored(&root, 40);
    let b = e.bundle("edges").unwrap();
    assert!(b.column("no_such_field").is_none());
    assert!(b.numeric_column("no_such_field").is_none());
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn numeric_column_refuses_non_numeric_on_mmap() {
    let root = tmp("refuse");
    std::fs::create_dir_all(&root).unwrap();
    {
        let mut e = Engine::open(&root).unwrap();
        e.compaction_policy_mut().disabled = true;
        e.create_bundle(
            BundleSchema::new("labelled")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::categorical("label")),
        )
        .unwrap();
        for i in 0..50i64 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("label".into(), Value::Text(format!("L{i}")));
            e.insert("labelled", &r).unwrap();
        }
        e.snapshot().unwrap();
    }
    let e = Engine::open_mmap(&root).unwrap();
    let b = e.bundle("labelled").unwrap();
    assert!(b.column("label").is_some(), "text column still readable");
    assert!(
        b.numeric_column("label").is_none(),
        "numeric_column must refuse a text field rather than coerce it"
    );
    assert_eq!(b.numeric_column("id").unwrap().len(), 50);
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn columns_match_repeated_column_calls_on_mmap() {
    // One decode per row instead of one per row per field, with identical output.
    let root = tmp("multi");
    let mut e = stored(&root, 250);
    for i in 0..10i64 {
        e.insert("edges", &row(i * 9, 55, 900.0 + i as f64)).unwrap();
    }
    let b = e.bundle("edges").unwrap();
    let names = ["edge_id", "vertex_a", "weight"];
    let cols = b.columns(&names).expect("columns");
    for (c, name) in names.iter().enumerate() {
        assert_eq!(&cols[c], &b.column(name).unwrap(), "column {name} differs");
    }
    assert!(b.columns(&["weight", "nope"]).is_none());
    drop(b);
    std::fs::remove_dir_all(&root).ok();
}
