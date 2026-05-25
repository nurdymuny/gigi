//! Cross-team interface contract test for the 2026-05-25 PR window
//! — four HTTP endpoints shipped for Marcella's Hopf + Riemann-Roch
//! wiring:
//!
//!   POST /v1/quantum_cohomology/compose
//!   POST /v1/quantum_cohomology/capacity
//!   POST /v1/bundles/{name}/holonomy_debt
//!   POST /v1/bundles/{name}/flat_transport
//!
//! Source of truth: cross-team thread 2026-05-25. Each test pins
//! the request/response field set and value semantics that
//! Marcella's runtime pattern-matches on. Compile-time failures
//! catch Rust-side renames before any wire deserialization can
//! drift.
//!
//! These tests exercise the underlying Rust APIs that the
//! endpoints delegate to (the handlers themselves are thin
//! adapters; testing the underlying calls + the JSON shapes
//! together is the right granularity for the contract).

#![cfg(feature = "kahler")]

use gigi::curvature::{holonomy_debt, HolonomyDebt};
use gigi::geometry::{
    flat_transport, BSource, ClosedTwoForm, CohClass, ComplexStructure,
    KahlerStructure, QuantumCohomology, TransportSegment, TwoForm,
};
use gigi::types::{BundleSchema, FieldDef, Record, Value};
use gigi::BundleStore;

fn kahler_2d() -> KahlerStructure {
    let j = ComplexStructure::standard(1);
    let b = ClosedTwoForm::new_constant(
        TwoForm::new(vec![0.0, 0.5, -0.5, 0.0], 2).expect("antisymmetric"),
    );
    KahlerStructure::new(j, b)
}

fn flat_bundle() -> BundleStore {
    let schema = BundleSchema::new("pr_window_test")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::numeric("x").with_range(2.0))
        .fiber(FieldDef::numeric("y").with_range(2.0))
        .with_kahler(kahler_2d());
    let mut store = BundleStore::new(schema);
    for i in 0..10 {
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(i));
        r.insert("x".into(), Value::Float(0.0));
        r.insert("y".into(), Value::Float(0.0));
        store.insert(&r);
    }
    store
}

// ── L7.5 frobenius_compose ──────────────────────────────────

#[test]
fn frobenius_compose_cpn_round_trip() {
    // The endpoint delegates to QuantumCohomology::cpn(n).compose(a, b).
    // CP² H · H = H² is the canonical sanity check.
    let qh = QuantumCohomology::cpn(2);
    let h = CohClass::h_power(1);
    let result = qh.compose(&h, &h).expect("composes");
    assert_eq!(result.terms, vec![(1.0, 2, 0)]);
}

#[test]
fn frobenius_compose_nontoy_refuses_with_unsupported_manifold() {
    // NonToy → endpoint returns 400 BadRequest with the
    // UnsupportedManifold error variant. Marcella pattern-matches
    // on the error message; this test pins the shape.
    let qh = QuantumCohomology::NonToy;
    let err = qh
        .compose(&CohClass::h_power(0), &CohClass::h_power(1))
        .expect_err("NonToy must refuse");
    match err {
        gigi::geometry::QuantumError::UnsupportedManifold { reason } => {
            assert!(
                reason.contains("GW_invariants"),
                "reason must mention GW invariants (Marcella matches on this substring); got '{}'",
                reason
            );
        }
    }
}

#[test]
fn frobenius_compose_coh_class_term_shape() {
    // The wire shape for CohClass terms is `[coefficient, h_power, q_power]`
    // (3-element tuple). This pins the in-memory triple form on
    // gigi::geometry::CohClass.terms — the wire derives from it.
    let c = CohClass {
        terms: vec![(1.5, 2, 1), (-0.5, 0, 0)],
    };
    for (coeff, h, q) in &c.terms {
        let _: &f64 = coeff;
        let _: &usize = h;
        let _: &usize = q;
    }
    assert_eq!(c.terms.len(), 2);
}

// ── L7.7 representational_capacity ──────────────────────────

#[test]
fn capacity_cpn_191_returns_binomial() {
    // The endpoint delegates to QC.representational_capacity(k_max).
    // For Marcella's Hopf substrate (CP^191), k=1 gives binomial(192, 191) = 192.
    let qh = QuantumCohomology::cpn(191);
    let cap = qh.representational_capacity(1).expect("toy capacity");
    assert_eq!(cap, 192);
}

#[test]
fn capacity_sphere2_k_plus_one() {
    // CP^1 = S^2: dim H^0(L^k) = k + 1. Pinned for parity with the
    // Python closed form in Marcella's hopf.py.
    let qh = QuantumCohomology::Sphere2;
    for k in 0..=5 {
        let cap = qh.representational_capacity(k).expect("toy capacity");
        assert_eq!(cap, k + 1);
    }
}

#[test]
fn capacity_nontoy_returns_unsupported_manifold() {
    // NonToy → endpoint returns 400. Same error variant as compose.
    let r = QuantumCohomology::NonToy.representational_capacity(3);
    assert!(matches!(
        r,
        Err(gigi::geometry::QuantumError::UnsupportedManifold { .. })
    ));
}

// ── L7.2 holonomy_debt ──────────────────────────────────────

#[test]
fn holonomy_debt_quantized_variant_shape() {
    // The wire response shape for a quantized debt:
    //   { variant: "quantized", quantized: 3, winding: 3.0 }
    // pinned via the underlying HolonomyDebt enum.
    let store = flat_bundle();
    let debt =
        holonomy_debt(&store, 2.0 * std::f64::consts::PI * 3.0, 1e-6).unwrap();
    assert!(matches!(debt, HolonomyDebt::Quantized(3)));
    assert!(debt.is_quantized());
    assert_eq!(debt.winding(), 3.0);
}

#[test]
fn holonomy_debt_continuous_variant_shape() {
    // The wire response shape for a continuous debt:
    //   { variant: "continuous", continuous: 0.7, winding: 0.7 }
    let store = flat_bundle();
    let debt =
        holonomy_debt(&store, 2.0 * std::f64::consts::PI * 0.7, 1e-6).unwrap();
    assert!(matches!(debt, HolonomyDebt::Continuous(_)));
    assert!(!debt.is_quantized());
    if let HolonomyDebt::Continuous(w) = debt {
        assert!((w - 0.7).abs() < 1e-12);
    }
}

#[test]
fn holonomy_debt_no_kahler_returns_404() {
    // No Kähler attached → underlying API returns None → endpoint
    // returns 404 with "no Kähler structure attached" message.
    let schema = BundleSchema::new("plain")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::numeric("x").with_range(2.0))
        .fiber(FieldDef::numeric("y").with_range(2.0));
    let mut store = BundleStore::new(schema);
    for i in 0..5 {
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(i));
        r.insert("x".into(), Value::Float(0.0));
        r.insert("y".into(), Value::Float(0.0));
        store.insert(&r);
    }
    let r = holonomy_debt(&store, 2.0 * std::f64::consts::PI, 1e-6);
    assert!(r.is_none(), "no-Kähler bundle must return None");
}

// ── L1.5 flat_transport ─────────────────────────────────────

#[test]
fn flat_transport_classical_request_shape() {
    // Wire shape: bias = null → classical transport.
    // BSource::None, used_magnetic = false, holonomy ~ 0.
    let seg = TransportSegment::new(
        vec![0.0, 0.0],
        vec![1.0, 0.0],
        vec![1.0, 0.0],
    )
    .unwrap();
    let r = flat_transport(&seg, None, 1e-3, 10, BSource::None).unwrap();
    assert_eq!(r.b_source, BSource::None);
    assert!(!r.used_magnetic);
    assert!(r.holonomy_norm < 1e-12);
}

#[test]
fn flat_transport_magnetic_request_shape() {
    // Wire shape: bias = [0, 0.5, -0.5, 0] (flat row-major 2x2),
    // b_source = "override" → BSource::Override + used_magnetic = true.
    let bias = ClosedTwoForm::new_constant(
        TwoForm::new(vec![0.0, 0.5, -0.5, 0.0], 2).expect("antisymmetric"),
    );
    let seg = TransportSegment::new(
        vec![0.0, 0.0],
        vec![1.0, 0.0],
        vec![1.0, 0.0],
    )
    .unwrap();
    let r =
        flat_transport(&seg, Some(&bias), 1e-3, 10, BSource::Override).unwrap();
    assert_eq!(r.b_source, BSource::Override);
    assert!(r.used_magnetic);
}

#[test]
fn flat_transport_b_source_variants_wire_strings() {
    // The endpoint serializes BSource via Debug → lowercase. Pin
    // the four wire strings Marcella pattern-matches on.
    assert_eq!(format!("{:?}", BSource::Bundle).to_lowercase(), "bundle");
    assert_eq!(format!("{:?}", BSource::Override).to_lowercase(), "override");
    assert_eq!(format!("{:?}", BSource::None).to_lowercase(), "none");
    assert_eq!(
        format!("{:?}", BSource::FallbackNonClosed).to_lowercase(),
        "fallbacknonclosed"
    );
}

// ── Cross-endpoint sanity ───────────────────────────────────

#[test]
fn pr_window_apis_are_all_callable_from_one_test_setup() {
    // Smoke check that all four underlying APIs work against a
    // single bundle setup, the way Marcella's runtime will compose
    // them per turn.
    let store = flat_bundle();

    // (1) frobenius_compose CP² check
    let qh = QuantumCohomology::cpn(2);
    let _ = qh
        .compose(&CohClass::h_power(1), &CohClass::h_power(1))
        .expect("frobenius");

    // (2) capacity CP² at k=3
    assert_eq!(qh.representational_capacity(3).unwrap(), 10);

    // (3) holonomy_debt integer winding
    let debt = holonomy_debt(&store, 2.0 * std::f64::consts::PI * 2.0, 1e-6).unwrap();
    assert!(matches!(debt, HolonomyDebt::Quantized(2)));

    // (4) flat_transport on same bundle's dim
    let seg = TransportSegment::new(
        vec![0.0, 0.0],
        vec![1.0, 0.0],
        vec![1.0, 0.0],
    )
    .unwrap();
    let _ = flat_transport(&seg, None, 1e-3, 10, BSource::None).unwrap();
}
