//! Halcyon's §2b correction: it is not width, it is SCALE.
//!
//! `marcella_source_documents` (66 records) returns all 12 declared fields
//! including its key. `marcella_source_claims` (7,156) returns 2 of 8.
//! `marcella_source_sections` (161,795) returns 3 of 8. My earlier gates used
//! 200-record fixtures and passed, which is why they missed this.
//!
//! The snapshot writer chunks at 50,000 records (`Engine::snapshot` ->
//! `snapshot_with_chunk_size(50_000)`), and `StreamingDhoomEncoder` fixes the
//! DHOOM header from the FIRST chunk, then encodes every later chunk
//! positionally against it. So 161,795 records cross a boundary that 66 do
//! not. These gates put fixtures on both sides of it.

use std::fs;

use gigi::engine::Engine;
use gigi::types::{BundleSchema, FieldDef, Record, Value};

fn dir(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("gigi_scale_{tag}"))
}

/// The `marcella_source_sections` shape: 8 fiber fields, all written on every
/// record, at a record count that crosses the 50,000 chunk boundary.
fn build_sections(d: &std::path::Path, n_records: usize) {
    let mut e = Engine::open(d).unwrap();
    e.compaction_policy_mut().disabled = true;
    e.create_bundle(
        BundleSchema::new("sections")
            .base(FieldDef::categorical("section_id"))
            .fiber(FieldDef::categorical("content"))
            .fiber(FieldDef::categorical("doc_id"))
            .fiber(FieldDef::categorical("heading"))
            .fiber(FieldDef::numeric("level"))
            .fiber(FieldDef::numeric("line_start"))
            .fiber(FieldDef::numeric("line_end"))
            .fiber(FieldDef::numeric("n_chars"))
            .fiber(FieldDef::categorical("section_path")),
    )
    .unwrap();

    for i in 0..n_records {
        let mut r = Record::new();
        r.insert("section_id".into(), Value::Text(format!("sec_{i}")));
        r.insert("content".into(), Value::Text(format!("body text of section {i}")));
        r.insert("doc_id".into(), Value::Text(format!("doc_{}", i % 66)));
        r.insert("heading".into(), Value::Text(format!("Heading {i}")));
        r.insert("level".into(), Value::Integer((i % 4) as i64 + 1));
        r.insert("line_start".into(), Value::Integer(i as i64 * 3));
        r.insert("line_end".into(), Value::Integer(i as i64 * 3 + 2));
        r.insert("n_chars".into(), Value::Integer(120 + (i % 400) as i64));
        r.insert("section_path".into(), Value::Text(format!("/a/b/sec_{i}")));
        e.insert("sections", &r).unwrap();
    }
    e.snapshot().expect("snapshot must succeed");
}

fn missing_fields(d: &std::path::Path) -> (Vec<String>, usize) {
    let e = Engine::open_mmap(d).expect("snapshot must reopen");
    let store = e.bundle("sections").expect("bundle present");
    let cov = store.field_coverage(0);
    (cov.fields_empty_in_sample.clone(), cov.records)
}

/// Below the chunk boundary. This is the shape my earlier gates covered.
#[test]
fn sections_shape_survives_below_the_chunk_boundary() {
    let d = dir("below");
    let _ = fs::remove_dir_all(&d);
    build_sections(&d, 2_000);
    let (missing, n) = missing_fields(&d);
    assert_eq!(n, 2_000);
    assert!(
        missing.is_empty(),
        "2,000 records: {} declared fields came back empty: {:?}",
        missing.len(),
        missing
    );
    let _ = fs::remove_dir_all(&d);
}

/// ACROSS the chunk boundary. 60,000 records means the snapshot writes chunk 1
/// (50,000) and chunk 2 (10,000), with the header fixed from chunk 1 only.
/// If any declared field is lost here, this is Hallie's defect.
#[test]
fn sections_shape_survives_across_the_chunk_boundary() {
    let d = dir("across");
    let _ = fs::remove_dir_all(&d);
    build_sections(&d, 60_000);
    let (missing, n) = missing_fields(&d);
    assert_eq!(n, 60_000);
    assert!(
        missing.is_empty(),
        "60,000 records (crosses the 50,000 chunk boundary): {} declared \
         fields came back empty: {:?}",
        missing.len(),
        missing
    );
    let _ = fs::remove_dir_all(&d);
}

/// The sharper version of the same question: a field that is written on
/// records in the SECOND chunk but not the first. The header is fixed from
/// chunk 1, so if the encoder cannot admit a field it has not already seen,
/// every value in chunk 2 is dropped.
#[test]
fn field_appearing_only_after_the_first_chunk_survives() {
    let d = dir("late_field");
    let _ = fs::remove_dir_all(&d);

    {
        let mut e = Engine::open(&d).unwrap();
        e.compaction_policy_mut().disabled = true;
        e.create_bundle(
            BundleSchema::new("sections")
                .base(FieldDef::categorical("section_id"))
                .fiber(FieldDef::categorical("content"))
                .fiber(FieldDef::categorical("heading"))
                .fiber(FieldDef::numeric("level"))
                .fiber(FieldDef::numeric("line_start"))
                .fiber(FieldDef::numeric("line_end"))
                .fiber(FieldDef::numeric("n_chars"))
                .fiber(FieldDef::categorical("section_path"))
                .fiber(FieldDef::categorical("doc_id")),
        )
        .unwrap();

        // chunk 1: heading / section_path never written
        for i in 0..52_000usize {
            let mut r = Record::new();
            r.insert("section_id".into(), Value::Text(format!("sec_{i}")));
            r.insert("content".into(), Value::Text(format!("body {i}")));
            r.insert("doc_id".into(), Value::Text("doc_0".into()));
            r.insert("level".into(), Value::Integer(1));
            e.insert("sections", &r).unwrap();
        }
        // chunk 2: the full record shape
        for i in 52_000..60_000usize {
            let mut r = Record::new();
            r.insert("section_id".into(), Value::Text(format!("sec_{i}")));
            r.insert("content".into(), Value::Text(format!("body {i}")));
            r.insert("doc_id".into(), Value::Text("doc_1".into()));
            r.insert("level".into(), Value::Integer(2));
            r.insert("heading".into(), Value::Text(format!("H {i}")));
            r.insert("line_start".into(), Value::Integer(i as i64));
            r.insert("line_end".into(), Value::Integer(i as i64 + 2));
            r.insert("n_chars".into(), Value::Integer(300));
            r.insert("section_path".into(), Value::Text(format!("/p/{i}")));
            e.insert("sections", &r).unwrap();
        }
        e.snapshot().expect("snapshot must succeed");
    }

    let e = Engine::open_mmap(&d).expect("snapshot must reopen");
    let store = e.bundle("sections").expect("bundle present");
    let late: Vec<Record> = store
        .records()
        .filter(|r| r.get("doc_id") == Some(&Value::Text("doc_1".into())))
        .collect();
    assert!(!late.is_empty(), "the chunk-2 records must survive at all");

    let lost = late.iter().filter(|r| r.get("heading").is_none() || matches!(r.get("heading"), Some(Value::Null))).count();
    assert_eq!(
        lost,
        0,
        "{lost} of {} records written WITH `heading` in chunk 2 lost it. The \
         DHOOM header is fixed from chunk 1, where no record carried it.",
        late.len()
    );

    let _ = fs::remove_dir_all(&d);
}
