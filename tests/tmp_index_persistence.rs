//! Probe for the HELICITY verb-audit review's `add_index` finding.
//! Temporary — not a committed gate. Proves (or refutes) two claims:
//!   1. an index added through the live path is lost on restart
//!   2. the engine's schema map never learns about it
use gigi::engine::Engine;
use gigi::types::{BundleSchema, FieldDef, Record, Value};
use std::fs;
use std::path::PathBuf;

fn dir(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!("gigi_idx_{tag}"))
}

#[ignore = "OPEN BUG (found 2026-08-15 by the HELICITY verb-audit review). An index added through POST /v1/bundles/{name}/index is lost on the next restart. add_index writes no WAL entry, and BundleStore holds a *clone* of the schema (engine.rs:1325 `BundleStore::new(schema.clone())`) while Engine::schemas keeps the original — so the engine's schema map never learns about the index, and compact_wal_to_schemas re-emits the stale pre-index schema. Per the review's own Laplacian argument, lambda-derived quantities are functions of the index SET, so the same bundle returns different geometry before and after a restart with no data change. Run with `cargo test -- --ignored` to see it fail."]
#[test]
fn add_index_survives_restart() {
    let d = dir("persist");
    let _ = fs::remove_dir_all(&d);

    {
        let mut e = Engine::open(&d).unwrap();
        e.compaction_policy_mut().disabled = true;
        let s = BundleSchema::new("b")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("tag"));
        e.create_bundle(s).unwrap();
        for i in 0..4i64 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("tag".into(), Value::Text(format!("t{}", i % 2)));
            e.insert("b", &r).unwrap();
        }

        // exactly what POST /v1/bundles/{name}/index does
        e.bundle_mut("b").unwrap().add_index("tag");

        let live = e.bundle("b").unwrap().schema().indexed_fields.clone();
        println!("  after add_index, store schema : {live:?}");
        assert!(
            live.contains(&"tag".to_string()),
            "index should be live in-session"
        );
        e.snapshot().unwrap();
    }

    {
        let e = Engine::open(&d).unwrap();
        let after = e.bundle("b").unwrap().schema().indexed_fields.clone();
        println!("  after restart, store schema  : {after:?}");
        assert!(
            after.contains(&"tag".to_string()),
            "INDEX LOST ON RESTART — add_index is not journalled and never \
             reaches Engine::schemas, which is what compaction re-emits from"
        );
    }

    let _ = fs::remove_dir_all(&d);
}
