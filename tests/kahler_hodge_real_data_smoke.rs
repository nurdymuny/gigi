//! L6 real-data smoke test (per bee's "test with real data" rule
//! and per IMPLEMENTATION_PLAN.md L6 def-of-done).
//!
//! L6 computes the Hodge complex + Betti numbers + Morse
//! compression on a bundle. This test loads the 20-record sensor
//! dataset, attaches a Kähler structure, and exercises the full
//! L6 surface:
//!
//! 1. `morse_compress()` returns Some with a sensible compression
//!    ratio on real data.
//! 2. Cohomology preservation always holds by construction.
//! 3. Euler characteristic computed from Betti matches the
//!    combinatorial V - E + F (Hodge ↔ Euler identity).
//! 4. A bundle without Kähler attached still produces a Morse
//!    snapshot because the construction only depends on the
//!    field-index graph, but downstream Marcella consumers gate on
//!    the Kähler-attached path. We test the no-Kähler case
//!    succeeds too — it's the same algorithm, no Kähler-specific
//!    branch.
//! 5. Sensor bundle has structure: at least one connected
//!    component (b_0 ≥ 1) on a non-empty bundle.

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

fn sensor_schema_with_kahler() -> BundleSchema {
    let j = ComplexStructure::standard(1);
    let b = ClosedTwoForm::new_constant(
        TwoForm::new(vec![0.0, 0.1, -0.1, 0.0], 2).expect("antisymmetric"),
    );
    let k = KahlerStructure::new(j, b);
    BundleSchema::new("sensor_hodge")
        .base(FieldDef::categorical("sensor_id"))
        .base(FieldDef::timestamp("timestamp", 1.0))
        .fiber(FieldDef::numeric("temperature"))
        .fiber(FieldDef::numeric("humidity"))
        .fiber(FieldDef::numeric("pressure"))
        .fiber(FieldDef::categorical("unit"))
        .fiber(FieldDef::categorical("status"))
        .index("status")
        .index("unit")
        .with_kahler(k)
}

#[test]
fn real_sensor_data_morse_compression_lifecycle() {
    let records = load_sensor_records();
    assert_eq!(records.len(), 20);

    let mut store = BundleStore::new(sensor_schema_with_kahler());
    for rec in &records {
        store.insert(rec);
    }
    assert_eq!(store.len(), 20);

    let m = store
        .morse_compress()
        .expect("≥ 2 records ⇒ morse_compress must return Some");

    // ── Cohomology preservation (always holds by construction) ──
    assert!(
        m.cohomology_preserved(),
        "Morse compression must preserve cohomology (Betti)"
    );

    // ── Sanity: original sizes match what we inserted ──
    assert_eq!(
        m.original_v, 20,
        "original vertex count = number of records (20)"
    );

    // ── At least one connected component on a non-empty bundle ──
    assert!(
        m.n_critical_0 >= 1,
        "non-empty bundle: b_0 ≥ 1; got {}",
        m.n_critical_0
    );

    // ── Hodge ↔ Euler identity ──
    // V - E + F must equal b_0 - b_1 + b_2 by the Euler-Poincaré
    // theorem. This is the cross-check that the algorithm is
    // internally consistent (matches Python test_11's V-E+F = 0
    // check on T²).
    let chi_topological = m.betti.euler_characteristic();
    let chi_combinatorial =
        m.original_v as i64 - m.original_e as i64 + m.original_f as i64;
    assert_eq!(
        chi_topological, chi_combinatorial,
        "Hodge↔Euler identity: b_0-b_1+b_2 ({}) must equal V-E+F ({})",
        chi_topological, chi_combinatorial
    );

    // ── Compression ratio sanity: ≥ 1 (you never expand) ──
    if m.n_critical() > 0 {
        assert!(
            m.compression_ratio() >= 1.0 - 1e-12,
            "compression ratio must be ≥ 1; got {}",
            m.compression_ratio()
        );
    }

    // ── Diagnostic ──
    println!(
        "L6 sensor smoke: V={}, E={}, F={}, Betti=({}, {}, {}), \
         critical=({}, {}, {}), compression={:.2}×",
        m.original_v,
        m.original_e,
        m.original_f,
        m.betti.b0,
        m.betti.b1,
        m.betti.b2,
        m.n_critical_0,
        m.n_critical_1,
        m.n_critical_2,
        m.compression_ratio()
    );
}

#[test]
fn real_sensor_data_no_kahler_also_works() {
    // The Morse compression depends on the cell complex, not the
    // Kähler structure. A bundle without Kähler attached still
    // produces a valid Morse snapshot — exercises the path
    // Marcella uses on legacy bundles.
    let records = load_sensor_records();
    let schema = BundleSchema::new("sensor_no_kahler")
        .base(FieldDef::categorical("sensor_id"))
        .base(FieldDef::timestamp("timestamp", 1.0))
        .fiber(FieldDef::numeric("temperature"))
        .fiber(FieldDef::numeric("humidity"))
        .fiber(FieldDef::numeric("pressure"))
        .fiber(FieldDef::categorical("unit"))
        .fiber(FieldDef::categorical("status"))
        .index("status");
    let mut store = BundleStore::new(schema);
    for rec in &records {
        store.insert(rec);
    }
    let m = store
        .morse_compress()
        .expect("Morse compression works on non-Kähler bundles too");
    assert_eq!(m.original_v, 20);
    assert!(m.n_critical_0 >= 1);
    assert!(m.cohomology_preserved());
}
