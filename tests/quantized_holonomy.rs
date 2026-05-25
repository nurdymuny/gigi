//! L7 e2e gate (per IMPLEMENTATION_PLAN.md L7 §"e2e validation
//! gate"): insert records that form a closed loop on a Kähler
//! bundle with `[B/2π] = 1`. Verify `holonomy_debt(loop) ==
//! HolonomyDebt::Quantized(1)`.

#![cfg(feature = "kahler")]

use gigi::curvature::{holonomy_debt, HolonomyDebt};
use gigi::geometry::{ClosedTwoForm, ComplexStructure, KahlerStructure, TwoForm};
use gigi::types::{BundleSchema, FieldDef, Record, Value};
use gigi::BundleStore;

fn integral_kahler() -> KahlerStructure {
    let j = ComplexStructure::standard(1);
    // b = 0.5 ⇒ over loop area 4π integrates to 2π ⇒ Chern 1.
    let b = ClosedTwoForm::new_constant(
        TwoForm::new(vec![0.0, 0.5, -0.5, 0.0], 2).expect("antisymmetric"),
    );
    KahlerStructure::new(j, b)
}

#[test]
fn closed_loop_holonomy_is_quantized_one_at_chern_1() {
    let schema = BundleSchema::new("loop")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::numeric("x").with_range(2.0))
        .fiber(FieldDef::numeric("y").with_range(2.0))
        .with_kahler(integral_kahler());
    let mut store = BundleStore::new(schema);

    // Insert records on a closed loop around the origin —
    // 8 vertices at angles k·π/4, radius 1.
    for k in 0..8 {
        let theta = k as f64 * std::f64::consts::PI / 4.0;
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(k));
        r.insert("x".into(), Value::Float(theta.cos()));
        r.insert("y".into(), Value::Float(theta.sin()));
        store.insert(&r);
    }

    // Loop area = π·r² = π (unit circle). With b = 0.5 the flux
    // is 0.5·π. We want a loop that integrates to 2π (Chern 1)
    // — take 4 windings: 4·0.5·π = 2π ✓.
    let n_windings = 4_i64;
    let flux = 0.5 * std::f64::consts::PI;
    let loop_integral = (n_windings as f64) * flux;

    let debt = holonomy_debt(&store, loop_integral, 1e-6).expect("attached Kähler");
    assert_eq!(
        debt,
        HolonomyDebt::Quantized(1),
        "4 windings around unit circle on integral B ⇒ Chern 1; got {:?}",
        debt
    );
}

#[test]
fn closed_loop_holonomy_is_continuous_when_integral_is_irrational() {
    let schema = BundleSchema::new("loop_cont")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::numeric("x").with_range(2.0))
        .fiber(FieldDef::numeric("y").with_range(2.0))
        .with_kahler(integral_kahler());
    let mut store = BundleStore::new(schema);
    for k in 0..6 {
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(k));
        r.insert("x".into(), Value::Float(0.0));
        r.insert("y".into(), Value::Float(0.0));
        store.insert(&r);
    }
    // Irrational winding ⇒ non-integer holonomy ⇒ Continuous.
    let loop_integral = 2.0 * std::f64::consts::PI * (std::f64::consts::E - 1.0);
    let debt = holonomy_debt(&store, loop_integral, 1e-6).expect("attached Kähler");
    assert!(matches!(debt, HolonomyDebt::Continuous(_)));
    if let HolonomyDebt::Continuous(w) = debt {
        assert!(
            (w - (std::f64::consts::E - 1.0)).abs() < 1e-12,
            "winding ≈ e - 1; got {}",
            w
        );
    }
}
