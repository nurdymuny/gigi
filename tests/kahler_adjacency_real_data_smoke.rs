//! L2 real-data smoke test (per bee's "test with real data" rule).
//!
//! L2 introduces dual adjacency: split a bundle's indexed fields
//! into a principal subset and an auxiliary subset, build adjacency
//! operators on each, and classify whether they commute.
//!
//! This test exercises the path on real sensor data
//! (`test_data/sensor_data.json`, 20 records). The schema indexes
//! `status` (categorical: normal/alert) and `temperature` (float).
//! Two real records are "adjacent in the status axis" iff they
//! share a status, and "adjacent in the temperature axis" iff they
//! share a temperature value.
//!
//! What we assert:
//! 1. The principal (status) adjacency has SOME edges — sensor
//!    records cluster by status so there's structure to observe.
//! 2. The auxiliary (temperature) adjacency is mostly sparse —
//!    individual temperature readings rarely repeat exactly.
//! 3. `commute(P, A)` returns a Commute or NotCommute verdict
//!    (NOT Unknown — 20 nodes is well within the dense-check
//!    threshold).
//! 4. The verdict is the same across two independent re-builds
//!    of the same adjacencies — deterministic, no nondeterminism
//!    from HashMap ordering leaking through.

#![cfg(feature = "kahler")]

use gigi::graph::{
    commute, AuxiliaryAdjacency, CommutativityClass, PrincipalAdjacency, SparseAdjacency,
};
use gigi::types::{BundleSchema, FieldDef, Value};
use gigi::BundleStore;
use std::collections::HashMap;
use std::fs;

fn load_sensor_records() -> Vec<HashMap<String, Value>> {
    let path = std::env::var("CARGO_MANIFEST_DIR")
        .map(|d| format!("{}/test_data/sensor_data.json", d))
        .expect("CARGO_MANIFEST_DIR not set");
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path, e));
    let parsed: serde_json::Value = serde_json::from_str(&text).expect("parse sensor_data.json");
    parsed
        .as_array()
        .expect("array")
        .iter()
        .map(|item| {
            let obj = item.as_object().expect("object");
            let mut rec = HashMap::new();
            for (k, v) in obj {
                let val = match v {
                    serde_json::Value::String(s) => Value::Text(s.clone()),
                    serde_json::Value::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            Value::Integer(i)
                        } else {
                            Value::Float(n.as_f64().expect("f64"))
                        }
                    }
                    serde_json::Value::Bool(b) => Value::Bool(*b),
                    _ => panic!("unexpected JSON variant"),
                };
                rec.insert(k.clone(), val);
            }
            rec
        })
        .collect()
}

fn sensor_schema() -> BundleSchema {
    BundleSchema::new("sensor_adj")
        .base(FieldDef::categorical("sensor_id"))
        .base(FieldDef::timestamp("timestamp", 1.0))
        .fiber(FieldDef::numeric("temperature"))
        .fiber(FieldDef::numeric("humidity"))
        .fiber(FieldDef::numeric("pressure"))
        .fiber(FieldDef::categorical("unit"))
        .fiber(FieldDef::categorical("status"))
        .index("temperature")
        .index("status")
}

/// Build an adjacency on a single bundle indexed field: two
/// records are adjacent iff they share that field's value.
///
/// This is the local reproduction of what
/// `src/spectral.rs::field_index_graph` does, restricted to one
/// field. Production L2 will wire a `from_bundle_fields` helper
/// directly into `PrincipalAdjacency` / `AuxiliaryAdjacency`;
/// for the smoke test we replay the logic here so the dependency
/// surface stays small and explicit.
///
/// `sections()` yields `(BasePoint, &[Value])` — fiber values in
/// schema-declared order. We map the field name to its position
/// in the fiber via `schema.fiber_field_index(name)`. Panics with
/// a clear message if the caller asked for a non-fiber field
/// (this is a test helper; we want loud failures, not silent
/// nothing-buckets).
fn adjacency_on_field(store: &BundleStore, field_name: &str) -> SparseAdjacency {
    use std::collections::BTreeMap;

    let idx = store
        .schema
        .fiber_field_index(field_name)
        .unwrap_or_else(|| panic!("'{}' is not a fiber field of this schema", field_name));

    // Group base points by the field's value (string-keyed for
    // determinism — Value's Hash isn't BTreeMap-friendly).
    let mut buckets: BTreeMap<String, Vec<u64>> = BTreeMap::new();

    for (bp, fiber) in store.sections() {
        // Defensive: a fiber that's shorter than expected is a
        // schema-vs-data bug, not something this test should
        // silently ignore.
        let v = fiber
            .get(idx)
            .unwrap_or_else(|| panic!("fiber too short for field index {}", idx));
        let key = format!("{:?}", v);
        buckets.entry(key).or_default().push(bp);
    }

    // Edges: pairwise within each bucket.
    let mut pairs = Vec::new();
    for ids in buckets.values() {
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                pairs.push((ids[i], ids[j]));
            }
        }
    }
    SparseAdjacency::from_pairs(pairs)
}

#[test]
fn real_sensor_data_dual_adjacency_classifies_deterministically() {
    let records = load_sensor_records();
    assert_eq!(records.len(), 20);

    let mut store = BundleStore::new(sensor_schema());
    for rec in &records {
        store.insert(rec);
    }
    assert_eq!(store.len(), 20);

    // Build the two adjacencies.
    let principal = PrincipalAdjacency::new(adjacency_on_field(&store, "status"));
    let auxiliary = AuxiliaryAdjacency::new(adjacency_on_field(&store, "temperature"));

    // Sanity 1: principal has structure (status groups records).
    assert!(
        principal.adj.edge_count() > 0,
        "principal (status) adjacency should have edges: {} found",
        principal.adj.edge_count()
    );

    // Sanity 2: auxiliary is comparatively sparse (continuous
    // temperatures rarely coincide exactly). We only assert
    // edge_count(aux) < edge_count(principal) — the exact ratio
    // isn't load-bearing.
    assert!(
        auxiliary.adj.edge_count() <= principal.adj.edge_count(),
        "auxiliary (temperature) should be no denser than principal (status); \
         got principal={}, auxiliary={}",
        principal.adj.edge_count(),
        auxiliary.adj.edge_count()
    );

    // The classification must produce a definite verdict on
    // 20 nodes — well below the dense-check threshold.
    let verdict_1 = commute(&principal, &auxiliary);
    assert!(
        !matches!(verdict_1, CommutativityClass::Unknown),
        "20-node bundle must classify, got Unknown"
    );

    // Determinism: rebuild from the same store, classify again,
    // verdict must match exactly. Catches any HashMap-iteration
    // nondeterminism that could leak in via the bucketing step.
    let principal2 = PrincipalAdjacency::new(adjacency_on_field(&store, "status"));
    let auxiliary2 = AuxiliaryAdjacency::new(adjacency_on_field(&store, "temperature"));
    let verdict_2 = commute(&principal2, &auxiliary2);
    assert_eq!(
        verdict_1, verdict_2,
        "classifier produced different verdicts on identical inputs: {:?} vs {:?}",
        verdict_1, verdict_2
    );

    // Print the verdict on a real bundle for posterity (visible
    // when running with `cargo test -- --nocapture`). Don't
    // assert on a specific class — the sensor data could
    // genuinely commute (one alert record sits in its own status
    // bucket of size 1, which limits the principal's
    // non-commutation with the more-spread temperature buckets).
    println!(
        "real sensor data dual-adjacency verdict: {:?}  \
         (principal edges = {}, auxiliary edges = {})",
        verdict_1,
        principal.adj.edge_count(),
        auxiliary.adj.edge_count()
    );
}
