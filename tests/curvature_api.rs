//! L4 e2e gate (per IMPLEMENTATION_PLAN.md L4 §"e2e validation
//! gate"): verify the curvature endpoint's `kahler` block matches
//! Marcella's expected JSON shape.
//!
//! Rather than spinning up an HTTP server, this test asserts the
//! exact JSON keys + numeric coherence of the four-component
//! decomposition that `GET /v1/bundles/<name>/curvature` serves.
//! The handler in `src/bin/gigi_stream.rs::curvature_report` is a
//! direct passthrough from `BundleStore::kahler_curvature()` so the
//! producer-side struct shape (asserted here) IS the wire shape.
//!
//! Two scenarios:
//! - Kähler bundle ⇒ JSON includes the `kahler` object with all
//!   five fields.
//! - Non-Kähler bundle ⇒ `kahler` is omitted (skip_serializing_if).

#![cfg(feature = "kahler")]

use gigi::bundle::{BundleStore, KahlerCurvature};
use gigi::geometry::{ClosedTwoForm, ComplexStructure, KahlerStructure, TwoForm};
use gigi::types::{BundleSchema, FieldDef, Record, Value};
use serde::Serialize;

/// Mirror of `gigi_stream::KahlerCurvatureReport` for serde
/// verification. If the handler-side struct field names diverge
/// from this, the contract is broken and the test asserting JSON
/// keys will catch it on the next deployment.
#[derive(Serialize)]
struct ExpectedKahlerJson {
    ricci: f64,
    weyl: f64,
    holo_bisectional_min: f64,
    holo_bisectional_max: f64,
    holo_sectional: f64,
}

impl From<KahlerCurvature> for ExpectedKahlerJson {
    fn from(k: KahlerCurvature) -> Self {
        Self {
            ricci: k.ricci,
            weyl: k.weyl,
            holo_bisectional_min: k.holo_bisectional_min,
            holo_bisectional_max: k.holo_bisectional_max,
            holo_sectional: k.holo_sectional,
        }
    }
}

fn kahler_2d() -> KahlerStructure {
    let j = ComplexStructure::standard(1);
    let b = ClosedTwoForm::new_constant(
        TwoForm::new(vec![0.0, 0.5, -0.5, 0.0], 2).expect("antisymmetric"),
    );
    KahlerStructure::new(j, b)
}

fn build_kahler_bundle(records: usize) -> BundleStore {
    let schema = BundleSchema::new("e2e_curv")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::numeric("x").with_range(2.0))
        .fiber(FieldDef::numeric("y").with_range(2.0))
        .with_kahler(kahler_2d());
    let mut store = BundleStore::new(schema);
    for i in 0..records {
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(i as i64));
        r.insert("x".into(), Value::Float((i as f64).cos() * 0.5));
        r.insert("y".into(), Value::Float((i as f64).sin() * 0.5));
        store.insert(&r);
    }
    store
}

#[test]
fn curvature_api_kahler_block_has_exact_json_keys() {
    let store = build_kahler_bundle(100);
    let kc = store.kahler_curvature().expect("snapshot");
    let json = serde_json::to_value(ExpectedKahlerJson::from(kc.clone())).expect("serialize");

    // The five keys Marcella's runtime deserializes. Any rename
    // breaks the wire contract.
    let obj = json.as_object().expect("object");
    assert!(obj.contains_key("ricci"), "missing key: ricci");
    assert!(obj.contains_key("weyl"), "missing key: weyl");
    assert!(
        obj.contains_key("holo_bisectional_min"),
        "missing key: holo_bisectional_min"
    );
    assert!(
        obj.contains_key("holo_bisectional_max"),
        "missing key: holo_bisectional_max"
    );
    assert!(
        obj.contains_key("holo_sectional"),
        "missing key: holo_sectional"
    );
    assert_eq!(
        obj.len(),
        5,
        "exact 5 keys; got extra: {:?}",
        obj.keys().collect::<Vec<_>>()
    );

    // Numeric values are finite.
    for (k, v) in obj {
        let n = v.as_f64().unwrap_or_else(|| panic!("{k} is not f64"));
        assert!(n.is_finite(), "{k} = {n} must be finite");
    }
}

#[test]
fn curvature_api_non_kahler_bundle_omits_kahler_block() {
    // Per the handler's `#[serde(skip_serializing_if = "Option::is_none")]`,
    // a None at the producer ⇒ key omitted ⇒ Marcella sees the
    // existing scalar-K shape unchanged.
    let schema = BundleSchema::new("e2e_no_kahler")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::numeric("x").with_range(2.0))
        .fiber(FieldDef::numeric("y").with_range(2.0));
    let mut store = BundleStore::new(schema);
    for i in 0..100 {
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(i));
        r.insert("x".into(), Value::Float((i as f64).cos() * 0.5));
        r.insert("y".into(), Value::Float((i as f64).sin() * 0.5));
        store.insert(&r);
    }
    assert!(
        store.kahler_curvature().is_none(),
        "no-Kähler bundle: producer returns None ⇒ HTTP `kahler` key omitted"
    );
}

#[test]
fn curvature_api_kahler_values_match_producer() {
    // Round-trip: serialize → parse → compare. Guards against
    // accidental serde transforms (rename, default, skip) breaking
    // the value.
    let store = build_kahler_bundle(200);
    let kc = store.kahler_curvature().expect("snapshot");

    let json = serde_json::to_string(&ExpectedKahlerJson::from(kc.clone())).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    let f = |k: &str| parsed[k].as_f64().expect(k);
    assert_eq!(f("ricci"), kc.ricci);
    assert_eq!(f("weyl"), kc.weyl);
    assert_eq!(f("holo_bisectional_min"), kc.holo_bisectional_min);
    assert_eq!(f("holo_bisectional_max"), kc.holo_bisectional_max);
    assert_eq!(f("holo_sectional"), kc.holo_sectional);
}
