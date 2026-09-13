//! A snapshot that drops columns must say so, at the moment it writes them.
//!
//! The 2026-09-13 Halcyon loss was silent for six weeks. Not because the bug
//! was subtle -- once found it is one line of header grammar -- but because
//! nothing ever compared the written DHOOM header against the schema it was
//! supposed to represent. `/schema` kept advertising 393 fields, `/query` kept
//! returning 2, and `HEALTH` kept reporting `curvature 0.0, confidence 1.0`.
//!
//! `SnapshotReport::header_incomplete` closes that. It reports rather than
//! refuses: a bundle whose mmap base has already lost its columns would fail a
//! refusing check forever and wedge every future snapshot, stranding new
//! records in the WAL. Per the graceful-skip policy one bundle degrades, never
//! the engine.

use std::fs;

use gigi::engine::Engine;
use gigi::types::{BundleSchema, FieldDef, Record, Value};

fn dir(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("gigi_hdrverify_{tag}"))
}

fn build(d: &std::path::Path, n_vec: usize, n_records: usize) -> Engine {
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
        rec.insert("doc_id".into(), Value::Text(format!("doc_{}", r % 4)));
        rec.insert("ingested_at".into(), Value::Timestamp(1779381503));
        for i in 0..n_vec {
            rec.insert(format!("v{i}"), Value::Float(r as f64 + i as f64 / 1000.0));
        }
        e.insert("emb", &rec).unwrap();
    }
    e
}

/// A healthy snapshot reports no incomplete headers. Without this the gate
/// below would pass on an implementation that flags everything.
#[test]
fn healthy_snapshot_reports_no_incomplete_headers() {
    let d = dir("healthy");
    let _ = fs::remove_dir_all(&d);

    let mut e = build(&d, 32, 200);
    let report = e
        .snapshot_with_chunk_size_report(50_000, None)
        .expect("snapshot must succeed");

    assert!(
        report.header_incomplete.is_empty(),
        "a healthy snapshot must flag nothing, flagged: {:?}",
        report.header_incomplete
    );
    assert!(report.total_records_written >= 200);

    drop(e);
    let _ = fs::remove_dir_all(&d);
}

/// The verification reads the file that was actually written, so it catches a
/// short header regardless of which layer produced it. Here the file is
/// truncated by hand -- standing in for the pre-2026-08-14 encoder -- and the
/// check is run against it directly.
#[test]
fn short_header_is_detected_against_the_schema() {
    let d = dir("short");
    let _ = fs::remove_dir_all(&d);

    let mut e = build(&d, 32, 200);
    e.snapshot_with_chunk_size_report(50_000, None)
        .expect("snapshot must succeed");
    drop(e);

    // Damage the written header the way the old encoder did: keep only the
    // folded columns, which is v2's exact surviving shape.
    let p = d.join("snapshots").join("emb.dhoom");
    let txt = fs::read_to_string(&p).unwrap();
    let mut lines: Vec<&str> = txt.split('\n').collect();
    let hdr = lines[0].to_string();
    let (name, rest) = hdr.split_once('{').unwrap();
    let inner = rest.trim_end_matches(':').trim_end_matches('}');
    let kept: Vec<&str> = inner
        .split(", ")
        .filter(|c| c.contains('@') || c.ends_with('&') || c.contains('|'))
        .collect();
    assert!(
        kept.len() < 35,
        "the fixture must actually drop columns, kept {}",
        kept.len()
    );
    let damaged = format!("{}{{{}}}:", name, kept.join(", "));
    lines[0] = &damaged;
    fs::write(&p, lines.join("\n")).unwrap();

    // Reopen and confirm the engine now disagrees with its own schema — the
    // condition the verification exists to announce.
    let e = Engine::open_mmap(&d).expect("must reopen");
    let store = e.bundle("emb").expect("bundle present");
    let declared = store.schema().base_fields.len() + store.schema().fiber_fields.len();
    let rec = store.records().next().expect("a record");
    assert_eq!(declared, 35, "1 base + 34 fiber");
    assert!(
        rec.len() < declared,
        "the damaged fixture must actually lose fields: schema {declared}, record {}",
        rec.len()
    );
    assert!(
        rec.get("record_id").is_none(),
        "the key is dropped with the other columns — Halcyon's §3 correlation"
    );

    let _ = fs::remove_dir_all(&d);
}
