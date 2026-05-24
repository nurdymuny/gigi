//! L5 e2e gate (per IMPLEMENTATION_PLAN.md L5 §"e2e validation
//! gate"): set up a Hadamard sub-bundle, run a continuous
//! propagation, verify the returned `convergence_rate` matches
//! Adachi's bound.
//!
//! The bound, computed independently:
//!     `r = max(|K_B|, ε)` where ε = f64::EPSILON
//! For a flat Hadamard bundle (`K_B = 0`), the rate floors at
//! ε to keep the log-rate calculation downstream finite.
//!
//! Marcella reads this rate to set the convergence horizon for
//! streaming queries: iteration count for ε-convergence is
//! `⌈log(1/ε) / rate⌉`.

#![cfg(feature = "kahler")]

use gigi::geometry::{ClosedTwoForm, ComplexStructure, KahlerStructure, TwoForm};
use gigi::sheaf::propagate_with_convergence_bound;
use gigi::types::{BundleSchema, FieldDef, Record, Value};
use gigi::BundleStore;

fn kahler_2d() -> KahlerStructure {
    let j = ComplexStructure::standard(1);
    let b = ClosedTwoForm::new_constant(
        TwoForm::new(vec![0.0, 0.5, -0.5, 0.0], 2).expect("antisymmetric"),
    );
    KahlerStructure::new(j, b)
}

fn flat_hadamard_bundle() -> BundleStore {
    let schema = BundleSchema::new("had_prop")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::numeric("x").with_range(2.0))
        .fiber(FieldDef::numeric("y").with_range(2.0))
        .with_kahler(kahler_2d());
    let mut store = BundleStore::new(schema);
    for i in 0..30 {
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(i));
        r.insert("x".into(), Value::Float(0.0));
        r.insert("y".into(), Value::Float(0.0));
        store.insert(&r);
    }
    store
}

fn spherical_bundle() -> BundleStore {
    let schema = BundleSchema::new("non_had")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::numeric("x").with_range(2.0))
        .fiber(FieldDef::numeric("y").with_range(2.0))
        .with_kahler(kahler_2d());
    let mut store = BundleStore::new(schema);
    // Disc-uniform fill ⇒ K_H ≈ 4 ⇒ NOT Hadamard.
    let mut state: u64 = 0x12345678;
    let mut n = 0u64;
    while n < 300 {
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
        r.insert("id".into(), Value::Integer(n as i64));
        r.insert("x".into(), Value::Float(x));
        r.insert("y".into(), Value::Float(y));
        store.insert(&r);
        n += 1;
    }
    store
}

#[test]
fn hadamard_propagate_surfaces_convergence_rate() {
    let store = flat_hadamard_bundle();
    let mut assumption = Record::new();
    assumption.insert("id".into(), Value::Integer(999));
    assumption.insert("x".into(), Value::Float(0.0));
    assumption.insert("y".into(), Value::Float(0.0));

    let (_records, rate) = propagate_with_convergence_bound(&store, &assumption);

    // Hadamard ⇒ rate must be Some + positive + finite.
    let rate = rate.expect("Hadamard bundle: propagate must surface a rate");
    assert!(rate > 0.0, "convergence rate must be > 0; got {}", rate);
    assert!(rate.is_finite(), "convergence rate must be finite");

    // Independent Adachi-bound recomputation: rate = max(|K_B|, ε).
    // For a fully-flat bundle K_B = 0 so the floor is f64::EPSILON.
    let kc = store.kahler_curvature().expect("kc");
    let expected = kc.holo_bisectional_max.abs().max(f64::EPSILON);
    assert!(
        (rate - expected).abs() < 1e-12,
        "convergence rate ({}) must match Adachi bound max(|K_B|, ε) = {}",
        rate,
        expected
    );
}

#[test]
fn non_hadamard_propagate_returns_no_rate() {
    let store = spherical_bundle();
    let mut assumption = Record::new();
    assumption.insert("id".into(), Value::Integer(999));
    assumption.insert("x".into(), Value::Float(0.0));
    assumption.insert("y".into(), Value::Float(0.0));

    let (_records, rate) = propagate_with_convergence_bound(&store, &assumption);
    assert!(
        rate.is_none(),
        "non-Hadamard bundle: propagate must NOT surface a rate; got {:?}",
        rate
    );
}

#[test]
fn no_kahler_propagate_returns_no_rate() {
    // Negative case: bundle without Kähler attached has no L4
    // evidence ⇒ no Hadamard detection ⇒ no rate.
    let schema = BundleSchema::new("plain")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::numeric("x").with_range(2.0))
        .fiber(FieldDef::numeric("y").with_range(2.0));
    let mut store = BundleStore::new(schema);
    for i in 0..10 {
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(i));
        r.insert("x".into(), Value::Float(0.0));
        r.insert("y".into(), Value::Float(0.0));
        store.insert(&r);
    }
    let mut assumption = Record::new();
    assumption.insert("id".into(), Value::Integer(999));
    assumption.insert("x".into(), Value::Float(0.0));
    assumption.insert("y".into(), Value::Float(0.0));
    let (_records, rate) = propagate_with_convergence_bound(&store, &assumption);
    assert!(rate.is_none(), "no-Kähler: rate must be None");
}
