//! Cross-team interface contract test for L4 — the
//! `kahler` block on `GET /v1/bundles/<name>/curvature`.
//!
//! Source of truth: catalog §E.3 ("Kähler curvature decomposition
//! in CurvatureStats") + IMPLEMENTATION_PLAN.md L4 spec. Marcella's
//! runtime reads the four invariants off the existing curvature
//! endpoint and uses them for:
//!
//! - `ricci > 0` ⇒ Fano regime ⇒ bounded generation diversity
//! - `weyl == 0` ⇒ conformally flat ⇒ pure-scaling gauges lossless
//! - `holo_bisectional_min ≤ 0` ⇒ Hadamard region (catalog §1.4
//!   guarantees apply automatically)
//! - `holo_sectional` ⇒ feeds rose-mechanism complexity bound
//!
//! This test gates the Rust struct shape that powers the JSON
//! serialization. If a field is renamed or removed on the Rust
//! side, compilation here fails BEFORE Marcella's deserialization
//! can drift.
//!
//! ### Contract fields under test
//!
//! | JSON field              | Rust field              | Type |
//! |-------------------------|-------------------------|------|
//! | `ricci`                 | `ricci`                 | f64  |
//! | `weyl`                  | `weyl`                  | f64  |
//! | `holo_bisectional_min`  | `holo_bisectional_min`  | f64  |
//! | `holo_bisectional_max`  | `holo_bisectional_max`  | f64  |
//! | `holo_sectional`        | `holo_sectional`        | f64  |

#![cfg(feature = "kahler")]

use gigi::bundle::{BundleStore, KahlerCurvature};
use gigi::geometry::{ClosedTwoForm, ComplexStructure, KahlerStructure, TwoForm};
use gigi::types::{BundleSchema, FieldDef, Record, Value};

fn kahler_2d() -> KahlerStructure {
    let j = ComplexStructure::standard(1);
    let b = ClosedTwoForm::new_constant(
        TwoForm::new(vec![0.0, 0.5, -0.5, 0.0], 2).expect("antisymmetric"),
    );
    KahlerStructure::new(j, b)
}

/// Build a bundle with a (x, y) complex pair + n records sampled on
/// the open unit disc.
fn disc_sample_bundle(n: usize) -> BundleStore {
    let schema = BundleSchema::new("contract_curvature")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::numeric("x").with_range(2.0))
        .fiber(FieldDef::numeric("y").with_range(2.0))
        .with_kahler(kahler_2d());
    let mut store = BundleStore::new(schema);
    // Deterministic seeded LCG for disc-uniform sampling.
    let mut state: u64 = 0xDEADBEEF;
    let mut inserted = 0u64;
    while inserted < n as u64 {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let u = ((state >> 32) as u32 as f64) / (u32::MAX as f64);
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let v = ((state >> 32) as u32 as f64) / (u32::MAX as f64);
        let x = 2.0 * u - 1.0;
        let y = 2.0 * v - 1.0;
        if x * x + y * y >= 1.0 {
            continue;
        }
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(inserted as i64));
        r.insert("x".into(), Value::Float(x));
        r.insert("y".into(), Value::Float(y));
        store.insert(&r);
        inserted += 1;
    }
    store
}

#[test]
fn snapshot_struct_has_catalog_e3_field_set() {
    let store = disc_sample_bundle(500);
    let kc: KahlerCurvature = store.kahler_curvature().expect("snapshot");

    // If any field renames or moves, compilation here fails first —
    // before Marcella's JSON deserialization can drift.
    let _: f64 = kc.ricci;
    let _: f64 = kc.weyl;
    let _: f64 = kc.holo_bisectional_min;
    let _: f64 = kc.holo_bisectional_max;
    let _: f64 = kc.holo_sectional;
}

#[test]
fn bisectional_min_le_max_invariant() {
    // Marcella's Hadamard gate reads `holo_bisectional_min` directly;
    // if min > max, the gate inverts and silent wrong-region routing
    // happens. Defensively assert the invariant.
    let store = disc_sample_bundle(500);
    let kc = store.kahler_curvature().expect("snapshot");
    assert!(
        kc.holo_bisectional_min <= kc.holo_bisectional_max + 1e-12,
        "min ({}) must be ≤ max ({})",
        kc.holo_bisectional_min,
        kc.holo_bisectional_max
    );
}

#[test]
fn ricci_formula_matches_catalog_einstein_normalization() {
    // Per catalog §E.3 the Einstein normalization is
    // `Ric = (n+1) g` for CP^n FS, and our streaming recipe sets
    // `ricci = (n+1) · holo_sectional / 4`. Verify the algebraic
    // relation holds for any input (independent of FS-ness).
    let store = disc_sample_bundle(500);
    let kc = store.kahler_curvature().expect("snapshot");
    // n = 1 (one complex pair).
    let n = 1.0_f64;
    let expected = (n + 1.0) * kc.holo_sectional / 4.0;
    assert!(
        (kc.ricci - expected).abs() < 1e-12,
        "ricci ({}) must equal (n+1) · K_H / 4 = {}",
        kc.ricci,
        expected
    );
}

#[test]
fn weyl_is_nonneg() {
    // Weyl is std-dev across complex pairs ⇒ always ≥ 0.
    let store = disc_sample_bundle(500);
    let kc = store.kahler_curvature().expect("snapshot");
    assert!(kc.weyl >= 0.0, "weyl must be non-negative; got {}", kc.weyl);
}

#[test]
fn endpoint_returns_none_when_no_kahler_attached() {
    // Negative case: schema without Kähler ⇒ kahler_curvature returns
    // None ⇒ HTTP `kahler` field is omitted (skip_serializing_if).
    let schema = BundleSchema::new("no_kahler")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::numeric("x").with_range(2.0))
        .fiber(FieldDef::numeric("y").with_range(2.0));
    let mut store = BundleStore::new(schema);
    for i in 0..50 {
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(i));
        r.insert("x".into(), Value::Float(0.1 * i as f64));
        r.insert("y".into(), Value::Float(0.2 * i as f64));
        store.insert(&r);
    }
    assert!(
        store.kahler_curvature().is_none(),
        "no Kähler attached: kahler_curvature must return None"
    );
}

#[test]
fn flat_bundle_returns_all_zero_components() {
    // Negative case: every fiber value identical ⇒ var = 0 ⇒ all
    // four invariants are 0. (Bundle is "flat" / trivial Kähler.)
    let schema = BundleSchema::new("flat")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::numeric("x").with_range(2.0))
        .fiber(FieldDef::numeric("y").with_range(2.0))
        .with_kahler(kahler_2d());
    let mut store = BundleStore::new(schema);
    for i in 0..20 {
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(i));
        r.insert("x".into(), Value::Float(0.0));
        r.insert("y".into(), Value::Float(0.0));
        store.insert(&r);
    }
    let kc = store.kahler_curvature().expect("snapshot");
    assert_eq!(kc.ricci, 0.0);
    assert_eq!(kc.weyl, 0.0);
    assert_eq!(kc.holo_bisectional_min, 0.0);
    assert_eq!(kc.holo_bisectional_max, 0.0);
    assert_eq!(kc.holo_sectional, 0.0);
}
