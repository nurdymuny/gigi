//! What `scalar_curvature` returns, pinned so nobody reads it as something else.
//!
//! The SSSP work used a per-vertex sub-bundle's scalar curvature as a proxy for
//! "metric flatness" and got a verdict out of it. The verdict was not a
//! measurement. This file pins the three facts that made it not one, on the
//! engine itself:
//!
//!   1. At two records the statistic is identically 1/4 whatever the values
//!      (Popoviciu equality case).
//!   2. It divides by the squared range, so it is invariant under rescaling
//!      and reads the SHAPE of a fibre's distribution inside its range, not
//!      the spread. {1,50,100} and {49,50,51} return the same number.
//!   3. On i.i.d. uniform values it converges to 1/12. A bundle whose fibre is
//!      pure noise reports K ~ 0.083, and that is the statistic working, not
//!      failing.
//!
//! None of this is a defect. It is what mean_f(var_f / range_f^2) is. The
//! defect was using it as a curvature-of-the-graph gate on 2-4 samples, and
//! the redo that found this lives with the SSSP paper
//! (GeodesicLeet_DavisManifold_DavisFieldEquations/sssp_ablation/src/bin/gigi_gate.rs).

use gigi::bundle::BundleStore;
use gigi::types::{BundleSchema, FieldDef, Record, Value};

fn schema() -> BundleSchema {
    BundleSchema::new("shape")
        .base(FieldDef::numeric("vertex_a"))
        .base(FieldDef::numeric("vertex_b"))
        .fiber(FieldDef::numeric("weight"))
}

fn k_of(weights: &[f64]) -> f64 {
    let mut s = BundleStore::new(schema());
    for (i, &w) in weights.iter().enumerate() {
        let mut r = Record::new();
        r.insert("vertex_a".into(), Value::Integer(0));
        r.insert("vertex_b".into(), Value::Integer(i as i64 + 1));
        r.insert("weight".into(), Value::Float(w));
        s.insert(&r);
    }
    gigi::curvature::scalar_curvature(&s)
}

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() < tol
}

#[test]
fn two_records_are_always_one_quarter() {
    for pair in [[1.0, 100.0], [50.0, 51.0], [7.0, 7.001], [0.5, 1e6]] {
        let k = k_of(&pair);
        assert!(close(k, 0.25, 1e-9), "{pair:?} -> {k}, expected 0.25 exactly");
    }
}

#[test]
fn range_normalised_so_shape_not_spread() {
    // symmetric three-point samples of wildly different spread: same K = 1/6
    let wide = k_of(&[1.0, 50.5, 100.0]);
    let tight = k_of(&[49.0, 50.0, 51.0]);
    assert!(close(wide, 1.0 / 6.0, 1e-6), "wide symmetric -> {wide}");
    assert!(close(tight, 1.0 / 6.0, 1e-6), "tight symmetric -> {tight}");

    // rescaling every value by 10 changes nothing
    let a = k_of(&[1.0, 2.0, 7.0, 30.0, 100.0]);
    let b = k_of(&[10.0, 20.0, 70.0, 300.0, 1000.0]);
    assert!(close(a, b, 1e-9), "scale invariance: {a} vs {b}");

    // but the SHAPE inside the range does move it: mass at the extremes is
    // higher, mass in the middle is lower
    let extremes = k_of(&[1.0, 1.0, 100.0, 100.0]);
    let middle = k_of(&[1.0, 50.0, 50.0, 100.0]);
    assert!(extremes > middle, "extremes {extremes} should exceed middle {middle}");
    assert!(close(extremes, 0.25, 1e-9), "two-point mass at the ends is the 1/4 ceiling");
}

#[test]
fn uniform_noise_reads_one_twelfth() {
    // xorshift uniform on [1, 100): the fibre carries no structure at all
    let mut s: u64 = 0x9E37_79B9_7F4A_7C15;
    let w: Vec<f64> = (0..20_000)
        .map(|_| {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            1.0 + (s % 99_000_000) as f64 / 1_000_000.0
        })
        .collect();
    let k = k_of(&w);
    assert!(
        close(k, 1.0 / 12.0, 0.003),
        "i.i.d. uniform -> {k}, expected 1/12 = 0.08333 (range/var of a uniform law)"
    );
}
