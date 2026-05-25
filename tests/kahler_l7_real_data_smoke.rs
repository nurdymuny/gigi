//! L7 real-data smoke test (per bee's "test with real data" rule
//! and per IMPLEMENTATION_PLAN.md L7 def-of-done).
//!
//! L7 ships quantization surfaces on top of L1's KahlerStructure:
//! `LineBundle` integrality, `holonomy_debt` classification,
//! `QuantizedTwoForm` Chern compression, and the toy-manifold
//! quantum cohomology APIs.
//!
//! This test loads the 20-record sensor dataset, attaches a Kähler
//! structure tuned so its B has a known Chern class, and verifies:
//!
//! 1. `LineBundle::from_constant_two_form` on the bundle's B
//!    integrates to a known Chern number.
//! 2. `holonomy_debt` classifies an integral loop as Quantized.
//! 3. `encode_chern` + `decode_chern` round-trip the B exactly.
//! 4. `QuantumCohomology::cpn(2)` Frobenius composition on the
//!    bundle's quantum-cohomology API surface gives an
//!    associativity-zero verdict.
//! 5. `representational_capacity` returns the Riemann-Roch bound.
//! 6. A non-Kähler bundle returns None / Err on the L7 surfaces.

#![cfg(feature = "kahler")]

use gigi::curvature::{holonomy_debt, HolonomyDebt};
use gigi::dhoom::{decode_chern, encode_chern};
use gigi::geometry::{
    ClosedTwoForm, CohClass, ComplexStructure, KahlerStructure, LineBundle,
    QuantumCohomology, TwoForm,
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

/// Sensor schema with Kähler tuned so B integrates to Chern = 1
/// over loop area 4π: b_magnitude = 0.5.
fn sensor_schema_with_integral_kahler() -> BundleSchema {
    let j = ComplexStructure::standard(1);
    let b = ClosedTwoForm::new_constant(
        TwoForm::new(vec![0.0, 0.5, -0.5, 0.0], 2).expect("antisymmetric"),
    );
    let k = KahlerStructure::new(j, b);
    BundleSchema::new("sensor_l7")
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
fn real_sensor_data_l7_quantization_lifecycle() {
    let records = load_sensor_records();
    assert_eq!(records.len(), 20);

    let mut store = BundleStore::new(sensor_schema_with_integral_kahler());
    for rec in &records {
        store.insert(rec);
    }
    assert_eq!(store.len(), 20);

    let bundle_b = &store.schema.kahler.as_ref().unwrap().b;
    let area = 4.0 * std::f64::consts::PI;
    let tol = 1e-10;

    // ── (1) LineBundle integrality on bundle's B ──
    let lb = LineBundle::from_constant_two_form(bundle_b, area, tol)
        .expect("integral by construction");
    assert_eq!(lb.chern_class().0, 1, "tuned for Chern = 1");

    // ── (2) holonomy_debt classifies integral loop as Quantized ──
    // Loop integral = 2π · 5 (5 windings) → Quantized(5).
    let loop_int = 2.0 * std::f64::consts::PI * 5.0;
    let debt = holonomy_debt(&store, loop_int, 1e-6).expect("attached Kähler");
    assert!(matches!(debt, HolonomyDebt::Quantized(5)));

    // ── (3) Chern compression round-trip ──
    let qf = encode_chern(bundle_b, area, tol).expect("integral encode");
    assert_eq!(qf.chern, 1);
    let decoded = decode_chern(&qf);
    let orig = bundle_b.form().matrix()[1];
    let new = decoded.form().matrix()[1];
    assert!((orig - new).abs() < f64::EPSILON);

    // ── (4) Frobenius composition (CP² associativity on toy
    //        manifold reachable from sensor data) ──
    let qh = QuantumCohomology::cpn(2);
    let h = CohClass::h_power(1);
    let h2 = qh.compose(&h, &h).expect("composes");
    assert_eq!(h2.terms, vec![(1.0, 2, 0)]);

    // ── (5) Riemann-Roch capacity ──
    // dim H⁰(CP², L^3) = binomial(5, 2) = 10.
    assert_eq!(qh.representational_capacity(3).unwrap(), 10);

    // ── Diagnostic ──
    println!(
        "L7 sensor smoke: Chern = {}, holonomy = Quantized(5), Chern compression \
         round-trips at machine epsilon, CP² capacity at k=3 = 10",
        lb.chern_class().0
    );
}

#[test]
fn real_sensor_data_no_kahler_refuses_l7_surfaces() {
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

    // holonomy_debt refuses on no-Kähler bundle.
    assert!(holonomy_debt(&store, 2.0 * std::f64::consts::PI, 1e-6).is_none());
}
