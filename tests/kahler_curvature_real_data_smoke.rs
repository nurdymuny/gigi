//! L4 real-data smoke test (per bee's "test with real data" rule
//! and per IMPLEMENTATION_PLAN.md L4 def-of-done).
//!
//! L4 ships the Kähler curvature decomposition snapshot
//! `(ricci, weyl, holo_bisectional_min/max, holo_sectional)` on
//! BundleStore. This test loads the 20-record sensor dataset, picks
//! two numeric fields as a complex pair, attaches a Kähler
//! structure, and verifies:
//!
//! 1. The snapshot is returned (Some) with all five components
//!    finite and in expected sign ranges.
//! 2. The Einstein-normalization identity `ricci = (n+1)·K_H/4`
//!    holds on real data (not just synthetic FS samples).
//! 3. Bisectional min ≤ max invariant holds.
//! 4. A non-Kähler version of the same bundle returns None.

#![cfg(feature = "kahler")]

use gigi::geometry::{ClosedTwoForm, ComplexStructure, KahlerStructure, TwoForm};
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

/// Sensor schema with Kähler attached: (temperature, humidity) is
/// the complex pair. n=1, J = standard 90° rotation, B = 0.1·dT∧dH.
fn sensor_schema_with_kahler() -> BundleSchema {
    let j = ComplexStructure::standard(1);
    let b = ClosedTwoForm::new_constant(
        TwoForm::new(vec![0.0, 0.1, -0.1, 0.0], 2).expect("antisymmetric"),
    );
    let k = KahlerStructure::new(j, b);
    BundleSchema::new("sensor_curvature")
        .base(FieldDef::categorical("sensor_id"))
        .base(FieldDef::timestamp("timestamp", 1.0))
        .fiber(FieldDef::numeric("temperature"))
        .fiber(FieldDef::numeric("humidity"))
        .fiber(FieldDef::numeric("pressure"))
        .fiber(FieldDef::categorical("unit"))
        .fiber(FieldDef::categorical("status"))
        .with_kahler(k)
}

#[test]
fn real_sensor_data_kahler_curvature_decomposition() {
    let records = load_sensor_records();
    assert_eq!(records.len(), 20);

    let mut store = BundleStore::new(sensor_schema_with_kahler());
    for rec in &records {
        store.insert(rec);
    }
    assert_eq!(store.len(), 20);

    let kc = store
        .kahler_curvature()
        .expect("Kähler attached + 20 records ⇒ snapshot must return Some");

    // ── All components finite ──
    assert!(kc.ricci.is_finite(), "ricci must be finite; got {}", kc.ricci);
    assert!(kc.weyl.is_finite(), "weyl must be finite; got {}", kc.weyl);
    assert!(
        kc.holo_sectional.is_finite(),
        "holo_sectional must be finite; got {}",
        kc.holo_sectional
    );
    assert!(
        kc.holo_bisectional_min.is_finite() && kc.holo_bisectional_max.is_finite(),
        "bisectional bounds must be finite; got [{}, {}]",
        kc.holo_bisectional_min,
        kc.holo_bisectional_max
    );

    // ── Sign / range sanity ──
    // Real sensor data has spread but is not FS-distributed. K_H ≥ 0
    // (variance is non-negative) and < FS asymptote 4.
    assert!(
        kc.holo_sectional >= 0.0,
        "K_H from real data must be ≥ 0; got {}",
        kc.holo_sectional
    );
    assert!(
        kc.weyl >= 0.0,
        "Weyl (std-dev) must be non-negative; got {}",
        kc.weyl
    );

    // ── Einstein identity holds on real data too ──
    // n=1 (one complex pair: temperature, humidity).
    let expected_ricci = (1.0 + 1.0) * kc.holo_sectional / 4.0;
    assert!(
        (kc.ricci - expected_ricci).abs() < 1e-12,
        "Einstein identity ricci = (n+1)·K_H/4 must hold on real data; \
         got ricci={}, expected={}",
        kc.ricci,
        expected_ricci
    );

    // ── Bisectional sandwich ──
    assert!(
        kc.holo_bisectional_min <= kc.holo_bisectional_max + 1e-12,
        "bisectional sandwich: min ({}) ≤ max ({})",
        kc.holo_bisectional_min,
        kc.holo_bisectional_max
    );

    // ── Diagnostic ──
    println!(
        "L4 sensor smoke: ricci = {:.6}, weyl = {:.6}, K_H = {:.6}, \
         K_B ∈ [{:.6}, {:.6}]",
        kc.ricci, kc.weyl, kc.holo_sectional, kc.holo_bisectional_min, kc.holo_bisectional_max
    );
}

#[test]
fn real_sensor_data_no_kahler_returns_none() {
    // Negative case: same sensor data, schema WITHOUT Kähler ⇒
    // kahler_curvature returns None ⇒ HTTP `kahler` field omitted.
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
        store.kahler_curvature().is_none(),
        "schema without Kähler: kahler_curvature must return None"
    );
}
