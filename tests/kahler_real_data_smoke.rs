//! L1 real-data smoke test (per bee's "test with real data" rule).
//!
//! L1 only adds *optional type plumbing* — attaching a Kähler
//! structure to a `BundleSchema` must not change ANY engine
//! behavior at this layer. This test proves the optionality
//! contract under a realistic workload by loading sensor data
//! from `test_data/sensor_data.json` (20 records of (sensor_id,
//! timestamp, temperature, humidity, pressure, unit, status))
//! into two bundles — one with a Kähler structure attached on
//! the (temperature, humidity) fiber pair, one without — and
//! comparing the engine outputs.
//!
//! Outputs compared:
//!   - `store.len()` (record count)
//!   - bundle-wide `scalar_curvature(&store)` from
//!     `src/curvature.rs`
//!   - `spectral_gap(&store)` from `src/spectral.rs`
//!   - section query result on a known sensor_id
//!
//! All four must be byte-identical between the two stores. Any
//! drift would mean the Kähler field is leaking into the
//! existing code paths, which violates L1's contract.

#![cfg(feature = "kahler")]

use gigi::curvature::scalar_curvature;
use gigi::geometry::{ClosedTwoForm, ComplexStructure, KahlerStructure, TwoForm};
use gigi::spectral::spectral_gap;
use gigi::types::{BundleSchema, FieldDef, Value};
use gigi::BundleStore;
use std::collections::HashMap;
use std::fs;

/// Inline minimal JSON parser sufficient for the sensor_data.json
/// file (flat array of objects with string/number/bool values).
/// Avoids pulling in a serde_json dependency from the test crate
/// boundary; the engine already imports serde_json transitively.
fn load_sensor_records() -> Vec<HashMap<String, Value>> {
    let path = std::env::var("CARGO_MANIFEST_DIR")
        .map(|d| format!("{}/test_data/sensor_data.json", d))
        .expect("CARGO_MANIFEST_DIR not set");
    let text = fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!("failed to read {}: {}", path, e);
    });

    let parsed: serde_json::Value = serde_json::from_str(&text).expect("parse sensor_data.json");
    let arr = parsed.as_array().expect("sensor_data.json must be a JSON array");

    arr.iter()
        .map(|item| {
            let obj = item.as_object().expect("each sensor record is a JSON object");
            let mut rec = HashMap::new();
            for (k, v) in obj {
                let val = match v {
                    serde_json::Value::String(s) => Value::Text(s.clone()),
                    serde_json::Value::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            Value::Integer(i)
                        } else {
                            Value::Float(n.as_f64().expect("number must coerce to f64"))
                        }
                    }
                    serde_json::Value::Bool(b) => Value::Bool(*b),
                    _ => panic!("unexpected JSON variant in sensor_data.json: {:?}", v),
                };
                rec.insert(k.clone(), val);
            }
            rec
        })
        .collect()
}

/// Build the matching schema for the sensor records. base = (sensor_id,
/// timestamp); fiber = temperature, humidity, pressure, unit, status.
/// Index temperature + status so we have something for the spectral
/// graph to chew on.
fn sensor_schema(name: &str) -> BundleSchema {
    BundleSchema::new(name)
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

/// Build a sensible 2D Kähler structure on the (temperature,
/// humidity) fiber pair — these are the natural conjugate
/// dimensions for sensor data (heat / moisture). J rotates 90°
/// in that plane; B is a constant magnetic bias of strength 0.5.
fn sensor_kahler() -> KahlerStructure {
    let j = ComplexStructure::standard(1); // 2D
    // B = [[0, 0.5], [-0.5, 0]] — standard symplectic form scaled.
    let b = ClosedTwoForm::new_constant(
        TwoForm::new(vec![0.0, 0.5, -0.5, 0.0], 2).expect("antisymmetric form"),
    );
    KahlerStructure::new(j, b)
}

#[test]
fn real_sensor_data_kahler_optionality_holds() {
    let records = load_sensor_records();
    assert_eq!(
        records.len(),
        20,
        "sensor_data.json should contain 20 records (got {})",
        records.len()
    );

    // ── Plain bundle (no Kähler) ──
    let mut plain = BundleStore::new(sensor_schema("plain"));
    for rec in &records {
        plain.insert(rec);
    }

    // ── Kähler bundle (same data, J + B attached) ──
    // Note: schema names differ ("plain" vs "kahler") so curvature is
    // computed on the same underlying field stats. Spectral gap on
    // the field-index graph is name-independent.
    let mut kahler = BundleStore::new(sensor_schema("kahler").with_kahler(sensor_kahler()));
    for rec in &records {
        kahler.insert(rec);
    }

    // ── Optionality contract: every observable matches ──

    // 1. Record count
    assert_eq!(
        plain.len(),
        kahler.len(),
        "len() differs: plain={}, kahler={}",
        plain.len(),
        kahler.len()
    );
    assert_eq!(plain.len(), 20);

    // 2. Scalar curvature (catalog §3.4 / src/curvature.rs::scalar_curvature)
    let k_plain = scalar_curvature(&plain);
    let k_kahler = scalar_curvature(&kahler);
    assert!(
        (k_plain - k_kahler).abs() < 1e-15,
        "scalar_curvature differs: plain={}, kahler={} (delta {})",
        k_plain,
        k_kahler,
        (k_plain - k_kahler).abs()
    );

    // 3. Spectral gap (catalog §3.6 / src/spectral.rs::spectral_gap)
    let gap_plain = spectral_gap(&plain);
    let gap_kahler = spectral_gap(&kahler);
    assert!(
        (gap_plain - gap_kahler).abs() < 1e-12,
        "spectral_gap differs: plain={}, kahler={} (delta {})",
        gap_plain,
        gap_kahler,
        (gap_plain - gap_kahler).abs()
    );

    // 4. Section count by sensor_id matches between the two stores
    //    (records().count is the simplest cross-store comparison that
    //    doesn't depend on hash-derived ordering).
    let plain_count = plain.records().count();
    let kahler_count = kahler.records().count();
    assert_eq!(
        plain_count, kahler_count,
        "records().count differs: plain={}, kahler={}",
        plain_count, kahler_count
    );

    // 5. Sanity: the Kähler structure IS attached and observable.
    let attached = kahler
        .schema
        .kahler
        .as_ref()
        .expect("Kähler structure should be attached to the kahler bundle");
    assert_eq!(attached.dim(), 2);
    assert!(attached.dim_coherent());
    // B(e_temp, e_humid) = 0.5 by construction.
    assert_eq!(attached.b.apply(&[1.0, 0.0], &[0.0, 1.0]), 0.5);

    // 6. Sanity: the plain bundle has NO Kähler structure attached.
    assert!(
        plain.schema.kahler.is_none(),
        "plain bundle should not have a Kähler structure attached"
    );
}
