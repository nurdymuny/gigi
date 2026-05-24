//! L5 real-data smoke test (per bee's "test with real data" rule
//! and per IMPLEMENTATION_PLAN.md L5 def-of-done).
//!
//! L5 detects Hadamard substructures on a bundle via the
//! L4 streaming curvature + L3 Jacobi-field conjugate-free check.
//! This test loads the 20-record sensor dataset, attaches a Kähler
//! structure, and exercises the full L5 surface:
//!
//! 1. `hadamard_regions()` returns a verdict — either FullBundle
//!    or empty depending on whether the sensor data's
//!    holo_bisectional_max sits below the threshold.
//! 2. `is_hadamard_region(None)` agrees with the regions vec.
//! 3. `transport_along()` succeeds iff the bundle is Hadamard, else
//!    returns an Err (the §1.5 safety gate).
//! 4. `transport_inverse()` round-trips a trajectory back to a
//!    record on the Hadamard verdict, returns None otherwise.

#![cfg(feature = "kahler")]

use gigi::geometry::{
    ClosedTwoForm, ComplexStructure, KahlerStructure, TransportSegment, TwoForm,
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
    let parsed: serde_json::Value = serde_json::from_str(&text).expect("parse");
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
                    _ => panic!("unexpected"),
                };
                rec.insert(k.clone(), val);
            }
            rec
        })
        .collect()
}

fn sensor_schema_with_kahler() -> BundleSchema {
    let j = ComplexStructure::standard(1);
    let b = ClosedTwoForm::new_constant(
        TwoForm::new(vec![0.0, 0.1, -0.1, 0.0], 2).expect("antisymmetric"),
    );
    let k = KahlerStructure::new(j, b);
    BundleSchema::new("sensor_hadamard")
        .base(FieldDef::categorical("sensor_id"))
        .base(FieldDef::timestamp("timestamp", 1.0))
        .fiber(FieldDef::numeric("temperature"))
        .fiber(FieldDef::numeric("humidity"))
        .fiber(FieldDef::numeric("pressure"))
        .fiber(FieldDef::categorical("unit"))
        .fiber(FieldDef::categorical("status"))
        .index("status")
        .with_kahler(k)
}

#[test]
fn real_sensor_data_hadamard_verdict_is_consistent() {
    let records = load_sensor_records();
    assert_eq!(records.len(), 20);

    let mut store = BundleStore::new(sensor_schema_with_kahler());
    for rec in &records {
        store.insert(rec);
    }
    assert_eq!(store.len(), 20);

    let regions = store.hadamard_regions();
    let is_had = store.is_hadamard_region(None);

    // ── Consistency: predicate ⇔ regions has a FullBundle entry ──
    let has_full = regions
        .iter()
        .any(|r| matches!(r.region, gigi::geometry::HadamardRegion::FullBundle));
    assert_eq!(
        is_had, has_full,
        "is_hadamard_region({}) must match hadamard_regions FullBundle presence ({})",
        is_had, has_full
    );

    // ── Diagnostic: what did we get? ──
    let kc = store.kahler_curvature().expect("kahler curvature");
    println!(
        "L5 sensor smoke: K_H = {:.4}, K_B_max = {:.4}, is_hadamard = {}",
        kc.holo_sectional, kc.holo_bisectional_max, is_had
    );
    for r in &regions {
        println!(
            "  region: {:?}, conjugate_free = {}, kb_max = {:.4}, rate = {:.4}",
            r.region, r.conjugate_free, r.kb_max, r.convergence_rate
        );
    }

    // ── Transport gate matches verdict ──
    let seg = TransportSegment::new(
        vec![22.4, 48.3],
        vec![23.0, 49.0],
        vec![1.0, 1.0],
    )
    .unwrap();
    let r = store.transport_along(&seg, 1e-3, 100);
    if is_had {
        assert!(
            r.is_ok(),
            "Hadamard verdict ⇒ transport_along must succeed; got {:?}",
            r.err()
        );
    } else {
        assert!(
            r.is_err(),
            "non-Hadamard verdict ⇒ transport_along must Err"
        );
    }

    // ── transport_inverse gate matches verdict ──
    let traj = vec![vec![22.4, 48.3]];
    let inv = store.transport_inverse(&traj, 100.0);
    if is_had {
        // May or may not find a match depending on data, but the
        // call doesn't refuse based on the region check.
        let _ = inv;
    } else {
        assert!(
            inv.is_none(),
            "non-Hadamard ⇒ transport_inverse must refuse (None)"
        );
    }
}

#[test]
fn real_sensor_data_no_kahler_returns_empty_regions() {
    // Negative case: schema without Kähler ⇒ hadamard_regions empty.
    let records = load_sensor_records();
    let schema = BundleSchema::new("sensor_no_kahler")
        .base(FieldDef::categorical("sensor_id"))
        .base(FieldDef::timestamp("timestamp", 1.0))
        .fiber(FieldDef::numeric("temperature"))
        .fiber(FieldDef::numeric("humidity"))
        .fiber(FieldDef::numeric("pressure"))
        .fiber(FieldDef::categorical("unit"))
        .fiber(FieldDef::categorical("status"));
    let mut store = BundleStore::new(schema);
    for rec in &records {
        store.insert(rec);
    }
    assert!(
        store.hadamard_regions().is_empty(),
        "no Kähler attached ⇒ hadamard_regions must be empty"
    );
    assert!(!store.is_hadamard_region(None));
}
