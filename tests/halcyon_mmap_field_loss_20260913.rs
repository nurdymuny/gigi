//! Halcyon's 2026-09-13 ask: mmap+overlay bundles losing fields on read.
//!
//! Two claims to reproduce, stated separately because they may have
//! different causes:
//!
//!   (1) the base/key field is not returned by any read path on an mmap
//!       bundle, on any width;
//!   (2) numeric fiber fields are additionally lost at embedding width
//!       (392 fiber fields) but not at analytics width (8).
//!
//! Hallie's control (fresh heap bundle, same schema, same key) passed, so
//! the write path and typing are not implicated. These tests take the
//! difference she isolated -- heap vs snapshot -- and put it under the
//! engine's own snapshot/reopen cycle.
//!
//! See theory/halcyon/HALCYON_TO_GIGI_ASK_2026-09-13_MMAP_SNAPSHOT_FIELD_LOSS.md

use std::fs;

use gigi::engine::Engine;
use gigi::types::{BundleSchema, FieldDef, Record, Value};

fn dir(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("gigi_halcyon0913_{tag}"))
}

/// Build a bundle with `n_vec` numeric fiber fields named v0..v{n_vec-1},
/// plus a categorical key and the two fibers that survived on v2.
fn build(d: &std::path::Path, name: &str, n_vec: usize, n_records: usize) {
    let mut schema = BundleSchema::new(name)
        .base(FieldDef::categorical("record_id"))
        .fiber(FieldDef::categorical("tier"))
        .fiber(FieldDef::numeric("ingested_at"));
    for i in 0..n_vec {
        schema = schema.fiber(FieldDef::numeric(&format!("v{i}")));
    }

    let mut e = Engine::open(d).unwrap();
    e.compaction_policy_mut().disabled = true;
    e.create_bundle(schema).unwrap();

    for r_i in 0..n_records {
        let mut r = Record::new();
        r.insert("record_id".into(), Value::Text(format!("doc_{r_i}")));
        r.insert("tier".into(), Value::Text("gauge".into()));
        r.insert("ingested_at".into(), Value::Float(1779381503.007586));
        for i in 0..n_vec {
            // distinct per (record, field) so nothing can be recovered by luck
            r.insert(
                format!("v{i}"),
                Value::Float((r_i as f64) * 1000.0 + (i as f64) * 0.001),
            );
        }
        e.insert(name, &r).unwrap();
    }
    e.snapshot().expect("snapshot must succeed");
}

/// CLAIM 1 -- the key field must come back from an mmap bundle.
/// Narrow bundle, so field count cannot be the explanation.
#[test]
fn mmap_bundle_returns_its_key_field_at_analytics_width() {
    let d = dir("key_narrow");
    let _ = fs::remove_dir_all(&d);
    build(&d, "narrow", 6, 40);

    {
        let e = Engine::open_mmap(&d).expect("the .dhoom must reopen");
        let store = e.bundle("narrow").expect("bundle present after reload");
        let rec = store.records().next().expect("at least one record");
        assert!(
            rec.get("record_id").is_some(),
            "record_id is the declared key and is absent from the reloaded record; \
             fields present were: {:?}",
            rec.keys().collect::<Vec<_>>()
        );
    }

    let _ = fs::remove_dir_all(&d);
}

/// CLAIM 2 -- every declared numeric fiber must come back at embedding width.
/// 384 v-fields + 2 others + 1 key = 387, spanning the 64-candidate
/// computed-field cap and any plausible header-width limit.
#[test]
fn mmap_bundle_returns_all_numeric_fibers_at_embedding_width() {
    let d = dir("vec_wide");
    let _ = fs::remove_dir_all(&d);
    build(&d, "wide", 384, 120);

    {
        let e = Engine::open_mmap(&d).expect("the .dhoom must reopen");
        let store = e.bundle("wide").expect("bundle present after reload");
        let rec = store.records().next().expect("at least one record");

        let missing: Vec<String> = (0..384usize)
            .map(|i| format!("v{i}"))
            .filter(|f| rec.get(f).is_none())
            .collect();

        assert!(
            missing.is_empty(),
            "{} of 384 numeric fibers absent after snapshot+reload \
             (first few: {:?}); record carried {} fields",
            missing.len(),
            &missing[..missing.len().min(5)],
            rec.len()
        );
    }

    let _ = fs::remove_dir_all(&d);
}

/// Control: the SAME 384-wide schema on the heap, no snapshot. If this
/// passes while the test above fails, the difference is the snapshot path
/// and not the width, which is exactly Hallie's isolation.
#[test]
fn heap_bundle_returns_all_numeric_fibers_at_embedding_width() {
    let d = dir("vec_heap");
    let _ = fs::remove_dir_all(&d);

    let mut schema = BundleSchema::new("heapwide").base(FieldDef::categorical("record_id"));
    for i in 0..384usize {
        schema = schema.fiber(FieldDef::numeric(&format!("v{i}")));
    }
    let mut e = Engine::open(&d).unwrap();
    e.compaction_policy_mut().disabled = true;
    e.create_bundle(schema).unwrap();
    let mut r = Record::new();
    r.insert("record_id".into(), Value::Text("doc_0".into()));
    for i in 0..384usize {
        r.insert(format!("v{i}"), Value::Float(i as f64));
    }
    e.insert("heapwide", &r).unwrap();

    let store = e.bundle("heapwide").unwrap();
    let rec = store.records().next().unwrap();
    let missing = (0..384usize)
        .map(|i| format!("v{i}"))
        .filter(|f| rec.get(f).is_none())
        .count();
    assert_eq!(missing, 0, "heap must return all 384 fibers");
    assert!(rec.get("record_id").is_some(), "heap must return the key");

    drop(e);
    let _ = fs::remove_dir_all(&d);
}

// ─────────────────────── the mechanism: header inferred from chunk 1

/// `StreamingDhoomEncoder` builds the DHOOM header from the FIRST chunk of
/// records and then encodes every later record positionally against it
/// (`StreamEncoder::push` walks `self.fiber.record_fields()`). A field that
/// is absent from the sample is therefore absent from the header, and is
/// silently dropped for EVERY record in the bundle -- including the
/// thousands that do carry it.
///
/// This is the shape of Hallie's v2: 18,870 records from the column-shifted
/// July ingest sit in the same bundle as the good ones. If the sparse batch
/// lands in the sample, the whole vector space becomes unreadable while
/// `ingested_at` and `tier` -- the fields the bad batch DOES carry -- survive.
/// That is exactly the surviving/lost split she reported.
#[test]
fn fields_absent_from_the_first_chunk_survive_snapshot() {
    let d = dir("sparse_first");
    let _ = fs::remove_dir_all(&d);

    {
        let mut e = Engine::open(&d).unwrap();
        e.compaction_policy_mut().disabled = true;
        e.create_bundle(
            BundleSchema::new("sparse")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::categorical("tier"))
                .fiber(FieldDef::numeric("v0"))
                .fiber(FieldDef::numeric("v1")),
        )
        .unwrap();

        // 150 records WITHOUT v0/v1 (the "bad batch"), then 150 WITH them.
        // min_sample is 100, so the header is fixed before a v-field is seen.
        for i in 0..150i64 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("tier".into(), Value::Text("bad_batch".into()));
            e.insert("sparse", &r).unwrap();
        }
        for i in 150..300i64 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("tier".into(), Value::Text("good".into()));
            r.insert("v0".into(), Value::Float(i as f64));
            r.insert("v1".into(), Value::Float(i as f64 * 2.0));
            e.insert("sparse", &r).unwrap();
        }
        e.snapshot().expect("snapshot must succeed");
    }

    {
        let e = Engine::open_mmap(&d).expect("the .dhoom must reopen");
        let store = e.bundle("sparse").expect("bundle present");
        let good: Vec<Record> = store
            .records()
            .filter(|r| r.get("tier") == Some(&Value::Text("good".into())))
            .collect();
        assert!(!good.is_empty(), "the good batch must survive at all");

        let lost = good.iter().filter(|r| r.get("v0").is_none()).count();
        assert_eq!(
            lost,
            0,
            "{lost} of {} records that WERE written with v0 came back without it. \
             The header was inferred from a sample that contained none, so the \
             field was dropped for the whole bundle. Fields present on a good \
             record: {:?}",
            good.len(),
            good[0].keys().collect::<Vec<_>>()
        );
    }

    let _ = fs::remove_dir_all(&d);
}
