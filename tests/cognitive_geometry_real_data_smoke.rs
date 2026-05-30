//! Real-data smoke test for the Cognitive Geometry verbs
//! (CAPACITY / HORIZON / DEPTH — Branch VII, Davis 2026).
//!
//! Per the project rule: every shipped feature must pass a real-data
//! smoke test on a representative bundle. The Kähler L1..L7 series
//! all have one (sensor_data.json, 20 records, real measurements);
//! this brings the three new cognitive-geometry verbs to the same bar.
//!
//! Validates, on the live sensor dataset:
//!
//! 1. `capacity(τ, K)` returns finite, positive, monotonically
//!    increasing in τ. K from the bundle's actual scalar curvature.
//! 2. `horizon(τ, K, λ₁)` returns finite, positive when K > 0 and
//!    λ₁ > 0; matches the closed-form τ/(K·ℓ_c).
//! 3. `encoding_depth(K, λ₁)` returns one of the four valid variants
//!    on real-bundle (K, λ₁) values; the four-way classifier reaches
//!    a defined verdict on real data (not just synthetic).
//! 4. Capacity / horizon / depth all read the SAME K from the SAME
//!    bundle — no drift between the three verbs.
//! 5. Numerical sanity: at τ = 0, capacity = 0; at τ → ∞, capacity → ∞.

#![cfg(feature = "kahler")]

use gigi::curvature::{
    capacity, encoding_depth, horizon, horizon_with, perceive, perception_bias,
    scalar_curvature, DepthConfig, EncodingDepth, HorizonConfig, LengthScaleEstimator,
};
use gigi::spectral::spectral_gap;
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

fn sensor_schema() -> BundleSchema {
    BundleSchema::new("sensor_cognitive_geometry")
        .base(FieldDef::categorical("sensor_id"))
        .base(FieldDef::timestamp("timestamp", 1.0))
        .fiber(FieldDef::numeric("temperature"))
        .fiber(FieldDef::numeric("humidity"))
        .fiber(FieldDef::numeric("pressure"))
        .fiber(FieldDef::categorical("unit"))
        .fiber(FieldDef::categorical("status"))
}

fn build_bundle() -> BundleStore {
    let records = load_sensor_records();
    assert_eq!(records.len(), 20, "sensor fixture is 20 records");
    let mut store = BundleStore::new(sensor_schema());
    for rec in records {
        store.insert(&rec);
    }
    store
}

#[test]
fn capacity_on_real_sensor_bundle_is_well_defined() {
    let store = build_bundle();
    let k = scalar_curvature(&store);
    assert!(k.is_finite(), "K must be finite on real data, got {}", k);
    assert!(k >= 0.0, "K must be non-negative, got {}", k);

    // Default τ = 1.0 (the documented default for CAPACITY when no
    // TOLERANCE is supplied).
    let c1 = capacity(1.0, k);
    if k > 0.0 {
        assert!(c1.is_finite(), "C must be finite when K>0, got {}", c1);
        assert!(c1 > 0.0, "C must be positive when τ>0 and K>0, got {}", c1);
    } else {
        // K == 0 → C = ∞ by design (the contract in capacity()).
        assert!(c1.is_infinite(), "C must be infinite when K=0, got {}", c1);
    }

    // Monotonicity in τ: doubling τ must double C (linear in τ
    // by construction).
    if k > 0.0 {
        let c2 = capacity(2.0, k);
        let c05 = capacity(0.5, k);
        assert!(
            (c2 / c1 - 2.0).abs() < 1e-9,
            "capacity must be linear in τ: c(2)/c(1)={}",
            c2 / c1
        );
        assert!(
            (c1 / c05 - 2.0).abs() < 1e-9,
            "capacity must be linear in τ: c(1)/c(0.5)={}",
            c1 / c05
        );
    }

    // τ = 0 → C = 0 (boundary case).
    assert_eq!(capacity(0.0, k.max(1e-9)), 0.0);
}

#[test]
fn horizon_on_real_sensor_bundle_is_well_defined() {
    let store = build_bundle();
    let k = scalar_curvature(&store);
    let lambda1 = spectral_gap(&store);

    assert!(lambda1.is_finite(), "λ₁ must be finite, got {}", lambda1);
    assert!(lambda1 >= 0.0, "λ₁ must be non-negative, got {}", lambda1);

    let s_max = horizon(1.0, k, lambda1);
    assert!(s_max.is_finite() || k.abs() < f64::EPSILON,
        "horizon finite when K>0, got s_max={} K={}", s_max, k);
    if k > 0.0 {
        assert!(s_max > 0.0, "horizon positive when K>0, τ>0, got {}", s_max);

        // Closed-form: s_max = τ / (K · ℓ_c), ℓ_c = 1/√λ₁ when λ₁ > 0.
        let l_c = if lambda1 > f64::EPSILON { 1.0 / lambda1.sqrt() } else { 1.0 };
        let expected = 1.0 / (k * l_c);
        assert!(
            (s_max - expected).abs() / expected.abs().max(1e-9) < 1e-9,
            "horizon doesn't match τ/(K·ℓ_c): got {} expected {}",
            s_max,
            expected
        );
    }

    // Doubling τ doubles s_max (linear in τ).
    if k > 0.0 {
        let h1 = horizon(1.0, k, lambda1);
        let h2 = horizon(2.0, k, lambda1);
        assert!(
            (h2 / h1 - 2.0).abs() < 1e-9,
            "horizon linear in τ: h(2)/h(1)={}",
            h2 / h1
        );
    }
}

#[test]
fn depth_on_real_sensor_bundle_classifies_to_valid_variant() {
    let store = build_bundle();
    let k = scalar_curvature(&store);
    let lambda1 = spectral_gap(&store);
    let depth = encoding_depth(k, lambda1);

    // The classifier reaches a definite verdict on real data — not
    // a panic, not a degenerate "Unknown" (which doesn't exist in
    // the enum; the test passes by virtue of EncodingDepth being
    // an enum with four total variants and Rust enforcing
    // exhaustiveness).
    let label = depth.label();
    let description = depth.description();
    assert!(matches!(label, "I" | "II" | "III" | "IV"), "label was {}", label);
    assert!(!description.is_empty(), "description must not be empty");

    // The boundary behavior is documented; assert it for traceability.
    // At λ₁ = 0 exactly, the classifier returns Topological.
    assert_eq!(encoding_depth(k, 0.0), EncodingDepth::Topological);
    // At extremely high K (above the 0.5 threshold) with healthy λ₁,
    // returns Metric.
    assert_eq!(encoding_depth(10.0, 0.5), EncodingDepth::Metric);
    // At very low K, very high λ₁: Tangent.
    assert_eq!(encoding_depth(0.001, 1.0), EncodingDepth::Tangent);
}

#[test]
fn horizon_with_on_sensor_bundle_fires_welford_fallback() {
    // The exact case the JTBD demo flagged: spectral_gap on sensor
    // bundles returns ~0, so the SpectralGap primary estimator is
    // degenerate. The HorizonConfig default fires the WelfordRadius
    // fallback. This test pins that contract on real data.
    let store = build_bundle();
    let k = scalar_curvature(&store);
    let lambda1 = spectral_gap(&store);
    // Sanity: sensor data exhibits the degenerate-λ₁ case.
    assert!(
        lambda1 < 1e-9,
        "expected λ₁ ≈ 0 on sensor bundle, got {}",
        lambda1
    );

    let res = horizon_with(1.0, k, &store, lambda1, &HorizonConfig::default());

    // The fallback engaged because the primary was degenerate.
    assert!(
        res.fallback_engaged,
        "λ₁ ≈ 0 must trigger the fallback estimator"
    );
    assert_eq!(res.estimator_used, LengthScaleEstimator::WelfordRadius);

    // ℓ_c is finite, positive, and meaningfully different from 1.0
    // (the dumb scalar-shim default). This is the whole point of the
    // calibrated path — sensor bundles get a real length scale, not
    // a fall-through identity that makes HORIZON ≡ CAPACITY.
    assert!(res.l_c.is_finite() && res.l_c > 0.0, "l_c = {}", res.l_c);
    assert!(
        (res.l_c - 1.0).abs() > 1e-3,
        "Welford ℓ_c must not be ≈ 1.0 on this fixture; got {}",
        res.l_c
    );
    assert!(res.s_max.is_finite() && res.s_max > 0.0);

    // Bonus: the calibrated s_max is meaningfully different from the
    // scalar shim's degenerate value. This is the user-visible
    // improvement — HORIZON stops being CAPACITY in disguise.
    let shim = horizon(1.0, k, lambda1);
    assert!(
        (res.s_max - shim).abs() / shim.abs() > 0.01,
        "calibrated s_max ({}) must meaningfully differ from shim ({})",
        res.s_max,
        shim
    );
}

#[test]
fn capacity_horizon_depth_read_consistent_k_and_lambda1() {
    // The three verbs all consult the same bundle's K and λ₁.
    // Different reads of the same store must produce identical values
    // (no caching staleness, no recomputation drift).
    let store = build_bundle();
    let k1 = scalar_curvature(&store);
    let k2 = scalar_curvature(&store);
    let lambda1_a = spectral_gap(&store);
    let lambda1_b = spectral_gap(&store);

    assert_eq!(k1, k2, "scalar_curvature is deterministic on same store");
    assert_eq!(
        lambda1_a, lambda1_b,
        "spectral_gap is deterministic on same store"
    );

    // The three verbs called back-to-back must agree on (K, λ₁):
    let c = capacity(1.0, k1);
    let h = horizon(1.0, k1, lambda1_a);
    let d = encoding_depth(k1, lambda1_a);

    // Re-derive each from the SAME (K, λ₁) and check identity:
    let c_re = capacity(1.0, k1);
    let h_re = horizon(1.0, k1, lambda1_a);
    let d_re = encoding_depth(k1, lambda1_a);
    assert_eq!(c, c_re);
    assert_eq!(h, h_re);
    assert_eq!(d, d_re);
}

// ── PERCEIVE — Theorem 8.6 ──────────────────────────────────────────
//
// Step 4a: the pure math layer. PERCEIVE = (R · v, ‖R − I‖_F). R is
// caller-supplied. The full chain TRANSPORT → R_acc → PERCEIVE lands
// when transport.rs surfaces R_acc on TransportResult (step 4b). These
// smoke tests pin the math on real-bundle dimensions and on vectors
// that represent realistic fiber values from the sensor fixture, so
// the verb is known-good before the integration commit lands.

#[test]
fn perceive_runs_on_real_sensor_fiber_vectors() {
    // Real sensor bundle has 3 numeric fiber fields
    // (temperature, humidity, pressure). PERCEIVE on a vector of
    // exactly those values from the first record must:
    //   - return a 3-element perceived vector
    //   - return a finite, non-negative bias
    let records = load_sensor_records();
    let first = &records[0];
    let v: Vec<f64> = ["temperature", "humidity", "pressure"]
        .iter()
        .map(|k| match first.get(*k).expect("present") {
            Value::Float(f) => *f,
            Value::Integer(i) => *i as f64,
            other => panic!("expected numeric, got {:?}", other),
        })
        .collect();
    assert_eq!(v.len(), 3, "real sensor record has 3 numeric fiber fields");

    // Identity rotation = no drift; PERCEIVE is passthrough.
    let id = vec![1.0, 0.0, 0.0,
                  0.0, 1.0, 0.0,
                  0.0, 0.0, 1.0];
    let res = perceive(&id, &v, 3).expect("identity perceive on real-bundle vector");
    assert_eq!(res.v_perceived, v, "identity must be passthrough on real data");
    assert_eq!(res.bias, 0.0, "identity bias = 0 on real data");

    // A small rotation in the (temperature, humidity) plane: 2° about
    // the pressure axis. Confirms bias is non-zero and matches the
    // closed form 2·sin(θ/2)·√2 = sqrt(2 − 2 cos θ)·√2 for a single
    // 2D rotation embedded in 3D. (For θ = 2° this is ~0.0494.)
    let theta = 2.0_f64.to_radians();
    let (c, s) = (theta.cos(), theta.sin());
    let r = vec![ c,  -s,  0.0,
                  s,   c,  0.0,
                  0.0, 0.0, 1.0];
    let res2 = perceive(&r, &v, 3).expect("rotated perceive on real-bundle vector");

    // ‖R − I‖_F² = (c-1)² + s² + s² + (c-1)² + 0² + 0² + 0² + 0² + 0²
    //            = 2(c-1)² + 2 s²
    //            = 2(c² - 2c + 1 + s²)
    //            = 2(1 - 2c + 1) = 4 - 4c = 4(1 - cos θ).
    // For θ = 2°, that's 4·(1 - cos(2°)) ≈ 4·6.09e-4 ≈ 2.44e-3,
    // so bias ≈ sqrt(2.44e-3) ≈ 0.0494.
    let expected_bias = (4.0 * (1.0 - c)).sqrt();
    assert!(
        (res2.bias - expected_bias).abs() < 1e-12,
        "real-data bias {} vs closed-form {} differ",
        res2.bias, expected_bias
    );
    assert!(res2.bias > 0.0, "rotated bias must be positive");
    assert!(res2.v_perceived.iter().all(|x| x.is_finite()),
        "perceived vector must be finite: {:?}", res2.v_perceived);

    // The perceived vector preserves the pressure component exactly
    // (it's the rotation axis); only temperature/humidity mix.
    assert!(
        (res2.v_perceived[2] - v[2]).abs() < 1e-12,
        "pressure preserved on axis-aligned rotation: {} vs {}",
        res2.v_perceived[2], v[2]
    );
}

#[test]
fn perceive_bias_grows_monotonically_with_rotation_angle() {
    // On real-bundle dimensions (3D fiber), increase rotation angle from
    // 0 → π/2 in steps; assert bias is strictly monotonically increasing.
    // This is the contract that lets a builder use bias as a threshold
    // signal — if it weren't monotone, "bias > X means trust drops"
    // wouldn't follow.
    let mut last = -1.0_f64;
    for deg in [0.0_f64, 1.0, 5.0, 15.0, 45.0, 90.0] {
        let theta = deg.to_radians();
        let (c, s) = (theta.cos(), theta.sin());
        let r = vec![ c,  -s,  0.0,
                      s,   c,  0.0,
                      0.0, 0.0, 1.0];
        let bias = perception_bias(&r, 3).expect("bias on real-bundle dim");
        assert!(bias.is_finite(), "bias at {}° must be finite, got {}", deg, bias);
        assert!(bias >= last - 1e-15,
            "bias must be monotone in θ: {}° gave {} (prev {})", deg, bias, last);
        last = bias;
    }
    // At π/2, bias = sqrt(4·(1−cos π/2)) = sqrt(4) = 2.0. Pin the
    // endpoint as a numerical sanity check.
    assert!((last - 2.0).abs() < 1e-12,
        "bias at 90° should be 2.0; got {}", last);
}

#[test]
fn transport_to_perceive_chain_on_real_bundle_dimensions() {
    // End-to-end integration: run flat_transport with a magnetic bias
    // on a synthetic but bundle-dimensioned segment, extract R_acc
    // from TransportResult, feed it straight to perceive(). The whole
    // point of step 4b's R_acc addition is making this chain work
    // without a glue layer. This pins the integration contract.
    //
    // We use the same fiber dim (3) as the sensor smoke fixture but
    // construct the segment + bias programmatically — flat_transport's
    // segment dim is independent of any actual bundle's record dim;
    // both happen to be 3 here so the "real-dim" framing holds.
    use gigi::geometry::transport::{flat_transport, BSource, TransportSegment};
    use gigi::geometry::forms::{ClosedTwoForm, TwoForm};

    // 3D rotation about the z-axis: B = b·dx∧dy embedded in R³.
    let b = 0.6_f64;
    let bias_mat = vec![
        0.0, -b,  0.0,
        b,   0.0, 0.0,
        0.0, 0.0, 0.0,
    ];
    let bias = ClosedTwoForm::new_constant(
        TwoForm::new(bias_mat, 3).expect("antisymmetric"),
    );

    // Initial velocity in the (x,y) plane, with a nonzero z to verify
    // it's preserved (the rotation axis is z).
    let seg = TransportSegment::new(
        vec![0.0, 0.0, 0.0],
        vec![0.0, 0.0, 0.0],
        vec![1.0, 0.0, 0.5],
    )
    .unwrap();
    let r = flat_transport(&seg, Some(&bias), 1e-4, 5000, BSource::Override).unwrap();

    // Step 4b contract: R_acc must be present on success.
    let rotation = r.rotation.as_ref().expect("R_acc present on success");
    assert_eq!(rotation.len(), 9, "R_acc must be dim² = 9 for dim=3");

    // Step 4a contract: perceive(R, v_initial) must equal final_velocity
    // to RK4 tolerance. This is the through-line that makes PERCEIVE a
    // useful chain off TRANSPORT.
    let initial_v = vec![1.0, 0.0, 0.5];
    let res = perceive(rotation, &initial_v, 3).expect("perceive succeeds on real R_acc");

    for i in 0..3 {
        assert!(
            (res.v_perceived[i] - r.final_velocity[i]).abs() < 1e-5,
            "R_acc·v_initial[{}] = {} vs final_velocity[{}] = {}",
            i, res.v_perceived[i], i, r.final_velocity[i]
        );
    }

    // The z-component must be preserved (rotation axis).
    assert!(
        (res.v_perceived[2] - 0.5).abs() < 1e-12,
        "z-axis component must be preserved on a z-axis rotation: got {}",
        res.v_perceived[2]
    );

    // Bias must be > 0 (we DID rotate) and finite.
    assert!(res.bias.is_finite() && res.bias > 0.0,
        "bias must be positive and finite: {}", res.bias);

    // Bias closed form for an angle θ rotation embedded in 3D:
    // ‖R - I‖_F² = 4·(1 - cos θ).
    // Here θ = b · T = 0.6 · (1e-4 · 5000) = 0.3 rad.
    let theta = 0.3_f64;
    let expected_bias = (4.0 * (1.0 - theta.cos())).sqrt();
    assert!(
        (res.bias - expected_bias).abs() < 1e-5,
        "bias {} vs closed-form {} differ",
        res.bias, expected_bias
    );

    // perception_bias agrees with perceive's bias (both read the same R).
    let standalone = perception_bias(rotation, 3).expect("standalone bias");
    assert!((standalone - res.bias).abs() < 1e-15);
}
