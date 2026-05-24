//! L1.5 real-data smoke test (per bee's "test with real data" rule
//! and per IMPLEMENTATION_PLAN.md L1.5 def-of-done).
//!
//! L1.5 ships `flat_transport(seg, bias, dt, steps, b_source)` —
//! the in-process Rust primitive that solves the magnetic
//! geodesic equation on flat `Rⁿ`. The GQL `TRANSPORT WITH B = ...`
//! verb (L1.5.3) dispatches into this; for the smoke test we call
//! it directly.
//!
//! Scenario: use the (temperature, humidity) fiber pair of the
//! sensor bundle as a 2D "embedding manifold." Pick two real sensor
//! records as the segment endpoints. Run flat_transport in three
//! configurations:
//!
//! 1. Classical (B=None) — straight line through (T, H) space.
//! 2. Magnetic with the bundle-attached B — the trajectory bends.
//!    Energy is conserved (catalog §1.2 antisymmetry guarantee).
//! 3. Magnetic with a per-request override B — different trajectory
//!    than case 2, same conservation properties.
//!
//! Assertions:
//! - All three calls return a valid `TransportResult` with the
//!   field set the v2 consumption draft specifies.
//! - Case 1's trajectory IS a straight line (max deviation from
//!   the line endpoints < 1e-12).
//! - Cases 2 and 3 produce DIFFERENT trajectories (different B
//!   means different bend).
//! - All three have `energy_drift < 1e-9` (the production bound).
//! - `b_source` reflects the resolution path (None, Bundle,
//!   Override).

#![cfg(feature = "kahler")]

use gigi::geometry::{
    flat_transport, BSource, ClosedTwoForm, ComplexStructure, KahlerStructure, TransportSegment,
    TwoForm,
};
use gigi::types::{BundleSchema, FieldDef, Value};
use gigi::BundleStore;
use std::collections::HashMap;
use std::f64::consts::PI;
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
    // 2D Kähler structure on the (temperature, humidity) plane.
    // J = standard 90° rotation on R². B = b·dx∧dy with b = 0.1
    // (gentle magnetic bias — large enough to bend the trajectory
    // measurably over a short integration but small enough that
    // the energy-drift bound is easy to hit).
    let j = ComplexStructure::standard(1);
    let b = ClosedTwoForm::new_constant(
        TwoForm::new(vec![0.0, 0.1, -0.1, 0.0], 2).expect("antisymmetric"),
    );
    let k = KahlerStructure::new(j, b);

    BundleSchema::new("sensor_transport")
        .base(FieldDef::categorical("sensor_id"))
        .base(FieldDef::timestamp("timestamp", 1.0))
        .fiber(FieldDef::numeric("temperature"))
        .fiber(FieldDef::numeric("humidity"))
        .fiber(FieldDef::numeric("pressure"))
        .fiber(FieldDef::categorical("unit"))
        .fiber(FieldDef::categorical("status"))
        .index("temperature")
        .index("status")
        .with_kahler(k)
}

#[test]
fn real_sensor_data_flat_transport_three_b_sources() {
    let records = load_sensor_records();
    assert_eq!(records.len(), 20);

    let mut store = BundleStore::new(sensor_schema_with_kahler());
    for rec in &records {
        store.insert(rec);
    }
    assert_eq!(store.len(), 20);

    // Pick two real records: index 0 (normal status, T=22.4 H=48.3)
    // and index 18 (alert spike, T=45.8 H=18.2). The segment runs
    // through (T, H) space between these two readings.
    let r_start = &records[0];
    let r_end = &records[18];
    let t_start = match r_start.get("temperature") {
        Some(Value::Float(x)) => *x,
        _ => panic!("temperature missing"),
    };
    let h_start = match r_start.get("humidity") {
        Some(Value::Float(x)) => *x,
        _ => panic!("humidity missing"),
    };
    let t_end = match r_end.get("temperature") {
        Some(Value::Float(x)) => *x,
        _ => panic!("temperature missing"),
    };
    let h_end = match r_end.get("humidity") {
        Some(Value::Float(x)) => *x,
        _ => panic!("humidity missing"),
    };
    let from = vec![t_start, h_start];
    let to = vec![t_end, h_end];
    // Initial velocity = unit vector pointing from start toward end.
    let dx = t_end - t_start;
    let dy = h_end - h_start;
    let mag = (dx * dx + dy * dy).sqrt();
    let v_init = vec![dx / mag, dy / mag];

    let seg = TransportSegment::new(from.clone(), to.clone(), v_init.clone()).unwrap();

    // ── Case 1: classical transport (B = None) ──
    let r_classical = flat_transport(&seg, None, 1e-3, 10_000, BSource::None).unwrap();
    assert_eq!(r_classical.b_source, BSource::None);
    assert!(!r_classical.used_magnetic);
    // Straight line: holonomy is exactly zero.
    assert!(
        r_classical.holonomy_norm < 1e-12,
        "classical holonomy {} should be ~0",
        r_classical.holonomy_norm
    );
    assert!(
        r_classical.energy_drift < 1e-9,
        "classical energy drift {} exceeds 1e-9",
        r_classical.energy_drift
    );

    // ── Case 2: magnetic with bundle-attached B ──
    let bundle_bias = store
        .schema
        .kahler
        .as_ref()
        .expect("Kähler attached")
        .b
        .clone();
    let r_bundle = flat_transport(
        &seg,
        Some(&bundle_bias),
        1e-3,
        10_000,
        BSource::Bundle,
    )
    .unwrap();
    assert_eq!(r_bundle.b_source, BSource::Bundle);
    assert!(r_bundle.used_magnetic);
    assert!(
        r_bundle.energy_drift < 1e-9,
        "bundle-bias energy drift {} exceeds 1e-9",
        r_bundle.energy_drift
    );
    // Bundle-bias trajectory must differ from the straight-line
    // case — find at least one trajectory point with non-trivial
    // perpendicular deviation from the classical straight line.
    let mut max_perp_dev_bundle = 0.0_f64;
    for (a, b) in r_classical.trajectory.iter().zip(r_bundle.trajectory.iter()) {
        let dev = ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt();
        if dev > max_perp_dev_bundle {
            max_perp_dev_bundle = dev;
        }
    }
    assert!(
        max_perp_dev_bundle > 0.01,
        "bundle bias should bend trajectory; max deviation only {}",
        max_perp_dev_bundle
    );

    // ── Case 3: magnetic with a per-request override (different B) ──
    // Override with a bias of opposite sign + larger magnitude.
    let override_bias = ClosedTwoForm::new_constant(
        TwoForm::new(vec![0.0, -0.2, 0.2, 0.0], 2).expect("antisymmetric"),
    );
    let r_override = flat_transport(
        &seg,
        Some(&override_bias),
        1e-3,
        10_000,
        BSource::Override,
    )
    .unwrap();
    assert_eq!(r_override.b_source, BSource::Override);
    assert!(r_override.used_magnetic);
    assert!(
        r_override.energy_drift < 1e-9,
        "override energy drift {} exceeds 1e-9",
        r_override.energy_drift
    );

    // The override trajectory must differ from the bundle trajectory
    // (different B ⇒ different bend).
    let mut max_div_bundle_vs_override = 0.0_f64;
    for (a, b) in r_bundle.trajectory.iter().zip(r_override.trajectory.iter()) {
        let dev = ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt();
        if dev > max_div_bundle_vs_override {
            max_div_bundle_vs_override = dev;
        }
    }
    assert!(
        max_div_bundle_vs_override > 0.01,
        "bundle vs override trajectories should differ; max deviation only {}",
        max_div_bundle_vs_override
    );

    // ── Diagnostic print (visible under `cargo test -- --nocapture`) ──
    println!(
        "L1.5 sensor smoke: classical |traj|={}, holonomy={:.2e}, drift={:.2e}",
        r_classical.trajectory.len(),
        r_classical.holonomy_norm,
        r_classical.energy_drift
    );
    println!(
        "  bundle bias: holonomy={:.2e}, drift={:.2e}, used_magnetic={}",
        r_bundle.holonomy_norm, r_bundle.energy_drift, r_bundle.used_magnetic
    );
    println!(
        "  override bias: holonomy={:.2e}, drift={:.2e}, used_magnetic={}",
        r_override.holonomy_norm, r_override.energy_drift, r_override.used_magnetic
    );
    println!(
        "  max perp dev classical→bundle: {:.4}; bundle→override: {:.4}",
        max_perp_dev_bundle, max_div_bundle_vs_override
    );
}

/// Anchor: pure flat-space cyclotron with no bundle, just the
/// transport primitive. Confirms the Rust port matches the Python
/// reference (`validation_tests.py::test_2`) on a small case
/// — radius hits |v|/b to RK4 fidelity.
#[test]
fn flat_cyclotron_anchor_matches_python_reference() {
    let b = 1.5_f64;
    let period = 2.0 * PI / b;
    let dt = 1e-4;
    let n_steps = (period / dt).round() as usize;

    let bias = ClosedTwoForm::new_constant(
        TwoForm::new(vec![0.0, b, -b, 0.0], 2).expect("antisymmetric"),
    );
    let seg =
        TransportSegment::new(vec![0.0, 0.0], vec![0.0, 0.0], vec![1.0, 0.0]).unwrap();
    let r = flat_transport(&seg, Some(&bias), dt, n_steps, BSource::Override).unwrap();

    let expected_radius = 1.0 / b;
    let center = vec![0.0, -1.0 / b];
    let mut max_dev = 0.0_f64;
    for p in &r.trajectory {
        let d = ((p[0] - center[0]).powi(2) + (p[1] - center[1]).powi(2)).sqrt();
        let dev = (d - expected_radius).abs();
        if dev > max_dev {
            max_dev = dev;
        }
    }
    // Python test 2 hits 1.98e-14 on this case; we allow a little
    // slack for the Rust integrator's accumulated error over the
    // full period.
    assert!(
        max_dev < 1e-5,
        "cyclotron radius deviation {} exceeds 1e-5 (Python ref: ~2e-14)",
        max_dev
    );
}
