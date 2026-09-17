//! `column` / `numeric_column` must agree with `records()`, row for row.
//!
//! The column read exists because `records()` rebuilds a `HashMap<String, Value>`
//! for every stored record, which dominates any scan that wants one field. The
//! saving is only usable if the column is the *same* data in the *same* order:
//! callers zip two columns of a bundle to get row-aligned pairs, so a drift in
//! ordering between the two paths would silently pair the wrong values.
//!
//! `BaseStorage` picks between three layouts and each orders records
//! differently, so each needs its own fixture:
//!
//! * **Sequential** — one numeric base field whose keys form an arithmetic
//!   progression, detected after 32 records. Iterates in insertion order.
//! * **Hybrid** — the same, with keys that leave the progression. Iterates the
//!   array run in insertion order, then the overflow keys sorted.
//! * **Hashed** — anything else, including *any* schema with more than one base
//!   field. Iterates in sorted-base-point order, not insertion order.
//!
//! A fixture that does not actually reach the layout it names tests nothing: an
//! earlier version of this file used a two-base-field schema throughout, which
//! is hash-stored, so the sequential and hybrid paths were never executed and a
//! deliberate reversal of their ordering left the suite green.

use gigi::bundle::BundleStore;
use gigi::types::{BundleSchema, FieldDef, Record, Value};

/// One numeric base field: eligible for array storage once the keys are seen to
/// be arithmetic.
fn keyed_schema(name: &str) -> BundleSchema {
    BundleSchema::new(name)
        .base(FieldDef::numeric("edge_id"))
        .fiber(FieldDef::numeric("vertex_a"))
        .fiber(FieldDef::numeric("weight"))
}

fn keyed_row(id: i64, u: i64, w: f64) -> Record {
    let mut r = Record::new();
    r.insert("edge_id".into(), Value::Integer(id));
    r.insert("vertex_a".into(), Value::Integer(u));
    r.insert("weight".into(), Value::Float(w));
    r
}

/// Every declared field's column equals the same field read out of `records()`.
fn assert_columns_match_records(store: &BundleStore, fields: &[&str]) {
    let via_records: Vec<Record> = store.records().collect();
    for f in fields {
        let col = store
            .column(f)
            .unwrap_or_else(|| panic!("column {f} missing"));
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
fn sequential_storage_column_matches_records() {
    // Arithmetic keys, well past the 32-record detection threshold.
    let mut s = BundleStore::new(keyed_schema("seq"));
    for i in 0..200i64 {
        s.insert(&keyed_row(i, i % 17, 1.0 + (i as f64) * 0.25));
    }
    assert_eq!(s.len(), 200);
    assert_eq!(s.storage_mode(), "sequential", "fixture must reach array storage");
    assert_columns_match_records(&s, &["edge_id", "vertex_a", "weight"]);
}

#[test]
fn sequential_storage_with_step_column_matches_records() {
    // A step other than 1 still detects as flat.
    let mut s = BundleStore::new(keyed_schema("seq_step"));
    for i in 0..120i64 {
        s.insert(&keyed_row(1000 + i * 7, i % 5, (i as f64) * 0.5));
    }
    assert_columns_match_records(&s, &["edge_id", "vertex_a", "weight"]);
}

#[test]
fn hybrid_storage_column_matches_records() {
    // Hybrid is chosen only when the first 32 keys are 95-99% arithmetic: a
    // perfectly arithmetic prefix gives Sequential, and a ragged one gives
    // Hashed. One break in 31 intervals is 0.968, inside that window.
    // Afterwards, keys off the progression land in overflow, and the overflow
    // share must stay under 5% or the store promotes itself to hashed.
    let mut s = BundleStore::new(keyed_schema("hybrid"));
    for i in 0..31i64 {
        s.insert(&keyed_row(i, i, i as f64));
    }
    s.insert(&keyed_row(1000, 1, 1000.0)); // the 32nd key breaks the run
    for i in 31..400i64 {
        s.insert(&keyed_row(i, i, i as f64));
    }
    assert_eq!(s.storage_mode(), "hybrid", "fixture must reach hybrid storage");
    assert_columns_match_records(&s, &["edge_id", "vertex_a", "weight"]);
}

#[test]
fn hashed_storage_column_matches_records() {
    // A categorical base field is hash-stored and iterates in sorted-base-point
    // order, which is not the insertion order used here.
    let schema = BundleSchema::new("hashed")
        .base(FieldDef::categorical("name"))
        .fiber(FieldDef::numeric("weight"));
    let mut s = BundleStore::new(schema);
    for i in 0..120i64 {
        let mut r = Record::new();
        r.insert("name".into(), Value::Text(format!("v{:04}", (i * 37) % 120)));
        r.insert("weight".into(), Value::Float(i as f64));
        s.insert(&r);
    }
    assert_eq!(s.storage_mode(), "hashed", "fixture must reach hash storage");
    assert_columns_match_records(&s, &["name", "weight"]);
}

#[test]
fn multi_base_field_storage_column_matches_records() {
    // More than one base field is hash-stored regardless of key shape. This is
    // the schema an edge bundle uses.
    let schema = BundleSchema::new("edges")
        .base(FieldDef::numeric("edge_id"))
        .base(FieldDef::numeric("vertex_a"))
        .base(FieldDef::numeric("vertex_b"))
        .fiber(FieldDef::numeric("weight"));
    let mut s = BundleStore::new(schema);
    for i in 0..150i64 {
        let mut r = Record::new();
        r.insert("edge_id".into(), Value::Integer(i));
        r.insert("vertex_a".into(), Value::Integer(i % 19));
        r.insert("vertex_b".into(), Value::Integer((i * 5) % 19));
        r.insert("weight".into(), Value::Float(1.0 + (i as f64) * 0.75));
        s.insert(&r);
    }
    assert_eq!(s.storage_mode(), "hashed", "multiple base fields are hash-stored");
    assert_columns_match_records(&s, &["edge_id", "vertex_a", "vertex_b", "weight"]);
}

#[test]
fn two_columns_zip_row_aligned() {
    // The property callers rely on: zipping two columns gives the pairs that
    // appear together in a record, not some other pairing.
    let mut s = BundleStore::new(keyed_schema("zip"));
    for i in 0..150i64 {
        s.insert(&keyed_row(i, (i * 7) % 23, (i as f64) * 1.5));
    }
    let tails = s.numeric_column("vertex_a").expect("vertex_a numeric");
    let weights = s.numeric_column("weight").expect("weight numeric");
    let expected: Vec<(f64, f64)> = s
        .records()
        .map(|r| {
            (
                r["vertex_a"].as_f64().unwrap(),
                r["weight"].as_f64().unwrap(),
            )
        })
        .collect();
    let got: Vec<(f64, f64)> = tails.into_iter().zip(weights).collect();
    assert_eq!(got, expected);
}

#[test]
fn two_columns_zip_row_aligned_hashed() {
    // Same property on the layout whose order is not insertion order.
    let schema = BundleSchema::new("zip_hashed")
        .base(FieldDef::categorical("name"))
        .fiber(FieldDef::numeric("a"))
        .fiber(FieldDef::numeric("b"));
    let mut s = BundleStore::new(schema);
    for i in 0..90i64 {
        let mut r = Record::new();
        r.insert("name".into(), Value::Text(format!("k{:03}", (i * 53) % 90)));
        r.insert("a".into(), Value::Integer(i));
        r.insert("b".into(), Value::Float((i as f64) * -2.0));
        s.insert(&r);
    }
    let a = s.numeric_column("a").unwrap();
    let b = s.numeric_column("b").unwrap();
    let expected: Vec<(f64, f64)> = s
        .records()
        .map(|r| (r["a"].as_f64().unwrap(), r["b"].as_f64().unwrap()))
        .collect();
    assert_eq!(a.into_iter().zip(b).collect::<Vec<_>>(), expected);
}

#[test]
fn absent_field_is_none() {
    let mut s = BundleStore::new(keyed_schema("absent"));
    s.insert(&keyed_row(0, 0, 1.0));
    assert!(s.column("no_such_field").is_none());
    assert!(s.numeric_column("no_such_field").is_none());
}

#[test]
fn numeric_column_refuses_a_non_numeric_field() {
    // Refusing beats substituting a placeholder: a caller that asked for numbers
    // must not receive a column where unreadable values became NaN or zero.
    let schema = BundleSchema::new("mixed")
        .base(FieldDef::categorical("name"))
        .fiber(FieldDef::numeric("weight"));
    let mut s = BundleStore::new(schema);
    for i in 0..10i64 {
        let mut r = Record::new();
        r.insert("name".into(), Value::Text(format!("n{i}")));
        r.insert("weight".into(), Value::Float(i as f64));
        s.insert(&r);
    }
    assert!(s.column("name").is_some(), "text column still readable");
    assert!(
        s.numeric_column("name").is_none(),
        "numeric_column must refuse a text field rather than coerce it"
    );
    assert_eq!(s.numeric_column("weight").unwrap().len(), 10);
}

#[test]
fn empty_bundle_yields_empty_columns() {
    let s = BundleStore::new(keyed_schema("empty"));
    assert_eq!(s.column("weight").unwrap().len(), 0);
    assert_eq!(s.numeric_column("weight").unwrap().len(), 0);
}

#[test]
fn column_reflects_updates() {
    // The column is read from live storage, so it must track mutation rather
    // than any cached snapshot.
    let mut s = BundleStore::new(keyed_schema("mutate"));
    for i in 0..40i64 {
        s.insert(&keyed_row(i, i, i as f64));
    }
    s.insert(&keyed_row(7, 7, 999.0)); // upsert over an existing base key
    assert_columns_match_records(&s, &["edge_id", "vertex_a", "weight"]);
    let w = s.numeric_column("weight").unwrap();
    assert!(w.contains(&999.0), "updated value must appear in the column");
    assert!(
        !w.contains(&7.0),
        "superseded value must not appear in the column"
    );
}

#[test]
fn columns_match_repeated_column_calls() {
    // The multi-field read must return exactly what the single-field read does,
    // field for field and row for row; it is only an optimisation.
    let mut s = BundleStore::new(keyed_schema("multi"));
    for i in 0..150i64 {
        s.insert(&keyed_row(i, (i * 3) % 29, (i as f64) * 0.75));
    }
    let names = ["edge_id", "vertex_a", "weight"];
    let cols = s.columns(&names).expect("columns");
    assert_eq!(cols.len(), names.len());
    for (c, name) in names.iter().enumerate() {
        assert_eq!(&cols[c], &s.column(name).unwrap(), "column {name} differs");
    }
}

#[test]
fn columns_refuses_when_any_field_is_absent() {
    let mut s = BundleStore::new(keyed_schema("multi_absent"));
    s.insert(&keyed_row(0, 0, 1.0));
    assert!(s.columns(&["weight", "nope"]).is_none());
    assert!(s.numeric_columns(&["weight", "nope"]).is_none());
    assert!(s.columns(&["weight"]).is_some());
}
