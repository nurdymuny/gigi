//! `field_coverage` — the health measurement that would have caught the
//! 2026-09-13 Halcyon outage in a week instead of a quarter.
//!
//! `marcella_source_embeddings_bge_v2` served 30,356 records whose entire
//! 384-field vector space read back `Null`. `/health` reported
//! `k_global 0.0, confidence 1.0, record_count 30356` throughout. Every one of
//! those numbers was correct: a bundle with no variance to measure and a bundle
//! with nothing in it are indistinguishable by curvature. Curvature was never
//! the measurement that could catch this. Coverage is.
//!
//! These gates fix the shape of that bundle — declared fields, never written —
//! and assert the check names it.

use std::fs;

use gigi::engine::Engine;
use gigi::types::{BundleSchema, FieldDef, Record, Value};

fn dir(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("gigi_fieldcov_{tag}"))
}

/// Build the v2 shape: a key, two fibers that the ingest DID write, and
/// `n_vec` numeric fibers that it never wrote.
fn build_v2_shape(d: &std::path::Path, n_vec: usize, n_records: usize, write_vectors: bool) {
    let mut schema = BundleSchema::new("emb")
        .base(FieldDef::categorical("record_id"))
        .fiber(FieldDef::categorical("tier"))
        .fiber(FieldDef::numeric("ingested_at"));
    for i in 0..n_vec {
        schema = schema.fiber(FieldDef::numeric(&format!("v{i}")));
    }

    let mut e = Engine::open(d).unwrap();
    e.compaction_policy_mut().disabled = true;
    e.create_bundle(schema).unwrap();

    for r in 0..n_records {
        let mut rec = Record::new();
        rec.insert("record_id".into(), Value::Text(format!("doc_{r}")));
        rec.insert("tier".into(), Value::Text("gauge".into()));
        rec.insert("ingested_at".into(), Value::Float(1779381503.007586));
        if write_vectors {
            for i in 0..n_vec {
                rec.insert(format!("v{i}"), Value::Float(r as f64 + i as f64 / 1000.0));
            }
        }
        e.insert("emb", &rec).unwrap();
    }
    e.snapshot().expect("snapshot must succeed");
}

/// The outage, in miniature: 386 declared fields, 3 ever written.
/// Coverage must name the 384 that were not.
#[test]
fn coverage_names_declared_fields_that_were_never_written() {
    let d = dir("never_written");
    let _ = fs::remove_dir_all(&d);
    build_v2_shape(&d, 384, 200, false);

    {
        let e = Engine::open_mmap(&d).expect("snapshot must reopen");
        let store = e.bundle("emb").expect("bundle present");
        let cov = store.field_coverage(0); // full scan

        assert!(cov.complete_scan, "max_sample 0 must scan every record");
        assert_eq!(cov.records, 200);
        assert_eq!(cov.fields_declared, 387, "1 base + 386 fiber");
        assert_eq!(
            cov.fields_non_empty, 3,
            "only record_id, tier, ingested_at were ever written"
        );
        assert_eq!(
            cov.fields_empty_in_sample.len(),
            384,
            "all 384 v-fields must be reported empty"
        );
        assert!(
            cov.fields_empty_in_sample.iter().any(|f| f == "v0"),
            "v0 is the field Marcella's gate refused on; it must be named"
        );

        // The reading that misled: curvature is 0.0 and confidence 1.0 on
        // exactly this bundle. Coverage is what distinguishes it.
        assert_eq!(
            store.scalar_curvature(),
            0.0,
            "curvature reads 0.0 here — the reading Hallie was given"
        );
    }

    let _ = fs::remove_dir_all(&d);
}

/// A healthy bundle of the SAME shape must report full coverage and name
/// nothing. Without this, the check above would pass on an implementation
/// that always reports fields empty.
#[test]
fn coverage_reports_a_fully_written_bundle_as_complete() {
    let d = dir("written");
    let _ = fs::remove_dir_all(&d);
    build_v2_shape(&d, 384, 200, true);

    {
        let e = Engine::open_mmap(&d).expect("snapshot must reopen");
        let store = e.bundle("emb").expect("bundle present");
        let cov = store.field_coverage(0);

        assert_eq!(cov.fields_declared, 387);
        assert_eq!(
            cov.fields_non_empty, 387,
            "every declared field was written and must count as covered"
        );
        assert!(
            cov.fields_empty_in_sample.is_empty(),
            "a fully-written bundle must name no empty fields, got {:?}",
            cov.fields_empty_in_sample
        );
        for f in &cov.per_field {
            assert_eq!(
                f.coverage_in_sample, 1.0,
                "field {} must be fully covered",
                f.field
            );
        }
    }

    let _ = fs::remove_dir_all(&d);
}

/// Sampling must be honest about being a sample: `complete_scan` false, and
/// `sampled` reported so a zero is read as unobserved rather than established.
#[test]
fn bounded_sample_reports_itself_as_partial() {
    let d = dir("sampled");
    let _ = fs::remove_dir_all(&d);
    build_v2_shape(&d, 8, 500, true);

    {
        let e = Engine::open_mmap(&d).expect("snapshot must reopen");
        let store = e.bundle("emb").expect("bundle present");
        let cov = store.field_coverage(100);

        assert_eq!(cov.sampled, 100, "the cap must be honoured");
        assert_eq!(cov.records, 500, "the true total must still be reported");
        assert!(
            !cov.complete_scan,
            "a bounded sample must not claim a complete scan"
        );
    }

    let _ = fs::remove_dir_all(&d);
}

/// Coverage must work identically on a heap bundle — the storage mode is not
/// supposed to change the answer, and Hallie's control was a heap bundle.
#[test]
fn coverage_works_on_heap_bundles_too() {
    let d = dir("heap");
    let _ = fs::remove_dir_all(&d);

    let mut e = Engine::open(&d).unwrap();
    e.compaction_policy_mut().disabled = true;
    e.create_bundle(
        BundleSchema::new("h")
            .base(FieldDef::categorical("record_id"))
            .fiber(FieldDef::numeric("written"))
            .fiber(FieldDef::numeric("never")),
    )
    .unwrap();
    for i in 0..10i64 {
        let mut r = Record::new();
        r.insert("record_id".into(), Value::Text(format!("d{i}")));
        r.insert("written".into(), Value::Float(i as f64));
        e.insert("h", &r).unwrap();
    }

    let store = e.bundle("h").expect("heap bundle");
    let cov = store.field_coverage(0);
    assert_eq!(cov.fields_declared, 3);
    assert_eq!(cov.fields_non_empty, 2);
    assert_eq!(cov.fields_empty_in_sample, vec!["never".to_string()]);

    drop(e);
    let _ = fs::remove_dir_all(&d);
}
