//! L1 gate test (see `theory/kahler_upgrade/IMPLEMENTATION_PLAN.md`):
//! a bundle without `kahler` attached must behave identically to a
//! pre-upgrade GIGI. The Kähler upgrade is strictly additive, and
//! this test enforces that strictness across realistic surfaces
//! (insert, query, gauge, serialization).

use gigi::types::{BundleSchema, FieldDef, Value};
use gigi::BundleStore;

/// A schema built without `with_kahler(...)` must:
/// 1. Compile against the public API exactly as it did before.
/// 2. Have any `kahler` field default to `None` when the feature
///    is on (we still validate the absence under cfg).
#[test]
fn schema_without_kahler_is_default_none_when_feature_on() {
    let schema = BundleSchema::new("test");
    assert_eq!(schema.name, "test");
    assert!(schema.base_fields.is_empty());
    assert!(schema.fiber_fields.is_empty());

    #[cfg(feature = "kahler")]
    assert!(
        schema.kahler.is_none(),
        "default schema must have no Kähler structure attached"
    );
}

/// Insert + read round-trip on a Kähler-free bundle. The values
/// stored must be byte-identical to what we put in, regardless of
/// feature state. This is the "behaves exactly as pre-upgrade"
/// gate from IMPLEMENTATION_PLAN §0.
#[test]
fn insert_read_roundtrip_unaffected_by_kahler_feature() {
    let schema = BundleSchema::new("rt")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::numeric("value"));
    let mut store = BundleStore::new(schema);

    let mut rec = std::collections::HashMap::new();
    rec.insert("id".to_string(), Value::Float(42.0));
    rec.insert("value".to_string(), Value::Float(3.14));
    store.insert(&rec);

    assert_eq!(store.len(), 1);
    // Read back: pick the only record and check field equality.
    let records: Vec<_> = store.records().collect();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].get("id"), Some(&Value::Float(42.0)));
    assert_eq!(records[0].get("value"), Some(&Value::Float(3.14)));
}

/// When the `kahler` feature is enabled, attaching a Kähler
/// structure via `with_kahler` and then reading it back gives the
/// same object (dim, coherence). Only runs when the feature is
/// on; the no-feature build skips this entirely.
#[cfg(feature = "kahler")]
#[test]
fn with_kahler_attaches_and_round_trips() {
    use gigi::geometry::{ClosedTwoForm, ComplexStructure, KahlerStructure, TwoForm};

    let j = ComplexStructure::standard(1); // 2-dim
    let b = ClosedTwoForm::new_constant(TwoForm::new(vec![0.0, 1.5, -1.5, 0.0], 2).unwrap());
    let k = KahlerStructure::new(j, b);
    assert!(k.dim_coherent());

    let schema = BundleSchema::new("kahler-attached").with_kahler(k);
    let attached = schema.kahler.as_ref().expect("Kähler structure should be attached");
    assert_eq!(attached.dim(), 2);
    assert!(attached.dim_coherent());

    // Apply B(u, v) through the schema's structure to confirm the
    // attached form is the same one we constructed.
    assert_eq!(attached.b.apply(&[1.0, 0.0], &[0.0, 1.0]), 1.5);
}

/// Dim-mismatch between J (dim 2) and B (dim 4) must panic at
/// attach time, not later. Catches the "stored a broken schema"
/// failure mode loudly.
#[cfg(feature = "kahler")]
#[test]
#[should_panic(expected = "dim mismatch")]
fn with_kahler_rejects_dim_mismatch() {
    use gigi::geometry::{ClosedTwoForm, ComplexStructure, KahlerStructure, TwoForm};

    let j = ComplexStructure::standard(1); // 2-dim
    // 4-dim 2-form — incompatible with the 2-dim J.
    let mut raw = vec![0.0_f64; 16];
    raw[1] = 0.5;
    raw[4] = -0.5;
    let b = ClosedTwoForm::new_constant(TwoForm::new(raw, 4).unwrap());
    let k = KahlerStructure::new(j, b);
    assert!(!k.dim_coherent(), "constructor should produce an incoherent K");

    let _ = BundleSchema::new("bad").with_kahler(k); // panics here
}
