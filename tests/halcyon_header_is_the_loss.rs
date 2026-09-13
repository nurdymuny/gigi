//! The mechanism behind Halcyon's field loss, and the gate against it.
//!
//! On an mmap bundle the schema and the records come from different sources:
//! the schema is rebuilt from the WAL's `CreateBundle` payload, while the
//! records are decoded from the `.dhoom`, and `json_to_record` copies whatever
//! keys the decoder produced -- which is whatever columns the HEADER names.
//!
//! When those disagree you get Hallie's exact symptom: `/schema` reports 393
//! declared fields while `/query` returns 2, with the rest **absent** rather
//! than null. Nothing else reproduces it: unwritten fields come back `Null`
//! (see `halcyon_null_probe.rs`), and width, record count, chunk boundary,
//! rebase cycles and mixed records all round-trip clean.
//!
//! `59d4aa3` ("never encode a bundle body with no columns left in it") fixed
//! the writer, but a re-snapshot reads from the mmap base, so a file already
//! damaged stays damaged through every later snapshot. These gates hold the
//! line on both halves: the writer must not emit a degenerate header, and the
//! engine must not silently disagree with its own schema.

use std::fs;

use gigi::engine::Engine;
use gigi::types::{BundleSchema, FieldDef, Record, Value};

fn dir(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("gigi_hdrloss_{tag}"))
}

fn build(d: &std::path::Path, n_vec: usize, n_records: usize) {
    let mut schema = BundleSchema::new("emb")
        .base(FieldDef::categorical("record_id"))
        .fiber(FieldDef::categorical("doc_id"))
        .fiber(FieldDef::timestamp("ingested_at", 1.0));
    for i in 0..n_vec {
        schema = schema.fiber(FieldDef::numeric(&format!("v{i}")));
    }
    let mut e = Engine::open(d).unwrap();
    e.compaction_policy_mut().disabled = true;
    e.create_bundle(schema).unwrap();
    for r in 0..n_records {
        let mut rec = Record::new();
        rec.insert("record_id".into(), Value::Text(format!("rec_{r}")));
        // one repeated value -> interned; one constant -> folded. These are the
        // two that survived on v2, and folded columns are exactly the ones a
        // degenerate header keeps.
        rec.insert("doc_id".into(), Value::Text(format!("doc_{}", r % 4)));
        rec.insert("ingested_at".into(), Value::Timestamp(1779381503));
        for i in 0..n_vec {
            rec.insert(format!("v{i}"), Value::Float(r as f64 + i as f64 / 1000.0));
        }
        e.insert("emb", &rec).unwrap();
    }
    e.snapshot().expect("snapshot must succeed");
}

/// The written header must name every declared field. This is the writer half
/// of `59d4aa3`: a body with its columns stripped is the damage, and it is
/// detectable in the file itself without reloading.
#[test]
fn snapshot_header_names_every_declared_field() {
    let d = dir("header_complete");
    let _ = fs::remove_dir_all(&d);
    build(&d, 64, 300);

    let txt = fs::read_to_string(d.join("snapshots").join("emb.dhoom"))
        .expect("snapshot file must exist");
    let header = txt.lines().next().expect("header line");
    let inner = header
        .trim_end_matches(':')
        .split_once('{')
        .expect("header shape")
        .1
        .trim_end_matches('}');
    let n_cols = inner.split(", ").count();

    assert_eq!(
        n_cols, 67,
        "header must name all 67 declared fields (1 base + 66 fiber); it named \
         {n_cols}. A header shorter than the schema is the v2 failure mode: the \
         reader is faithful to it, so every column it drops is a field that \
         reads back ABSENT. Header was: {}",
        &header[..header.len().min(200)]
    );
    for probe in ["record_id", "v0", "v63"] {
        assert!(
            inner.contains(probe),
            "declared field `{probe}` missing from the written header"
        );
    }

    let _ = fs::remove_dir_all(&d);
}

/// The engine must not disagree with its own schema after a reload. Every
/// declared field must be present on a reloaded record -- as a value or as
/// `Null`, but never missing.
#[test]
fn reloaded_records_carry_every_declared_field() {
    let d = dir("no_disagreement");
    let _ = fs::remove_dir_all(&d);
    build(&d, 64, 300);

    {
        let e = Engine::open_mmap(&d).expect("snapshot must reopen");
        let store = e.bundle("emb").expect("bundle present");
        let declared: Vec<String> = store
            .schema()
            .base_fields
            .iter()
            .chain(store.schema().fiber_fields.iter())
            .map(|f| f.name.clone())
            .collect();
        assert_eq!(declared.len(), 67);

        let rec = store.records().next().expect("a record");
        let absent: Vec<&String> = declared.iter().filter(|f| rec.get(*f).is_none()).collect();
        assert!(
            absent.is_empty(),
            "{} declared fields absent from a reloaded record (schema says {}, \
             record carries {}): {:?}",
            absent.len(),
            declared.len(),
            rec.len(),
            &absent[..absent.len().min(6)]
        );
    }

    let _ = fs::remove_dir_all(&d);
}
