//! Cross-team interface contract test (per
//! `theory/kahler_upgrade/marcella_kahler_consumption_v2.md §2`
//! + GIGI reply Q1–Q2).
//!
//! Marcella's runtime deserializes the transport response and
//! expects specific field names. The GQL serialization layer
//! (L1.5.3, not yet shipped) will be the source of those JSON
//! field names — but the SHAPE comes from the Rust
//! `TransportResult` struct. If we rename a field on the Rust
//! side, this test fails first, before the JSON layer drift can
//! reach Marcella's deserialization.
//!
//! ### Contract fields under test (from consumption draft §2)
//!
//! | JSON field             | Rust field          | Type      |
//! |------------------------|---------------------|-----------|
//! | `transported_section`  | n/a (lives in GQL)  | id        |
//! | `path_length`          | `path_length`       | f64       |
//! | `energy_drift`         | `energy_drift`      | f64       |
//! | `holonomy_norm`        | `holonomy_norm`     | f64       |
//! | `used_magnetic`        | `used_magnetic`     | bool      |
//! | `b_source`             | `b_source`          | enum      |
//! | `closedness_norm`      | `closedness_norm`   | Option<f64>|
//!
//! Also asserts the four `BSource` variants match the JSON enum
//! Marcella's runtime expects: `"bundle" | "override" | "none" |
//! "fallback_non_closed"`.
//!
//! When L1.5.3 lands and JSON serialization is wired, this test
//! gets a sibling that asserts the actual serialized field names.

#![cfg(feature = "kahler")]

use gigi::geometry::{
    flat_transport, BSource, ClosedTwoForm, TransportResult, TransportSegment, TwoForm,
};

#[test]
fn transport_result_struct_has_marcella_v2_field_set() {
    // Construct any valid call so we get a real TransportResult.
    let seg =
        TransportSegment::new(vec![0.0, 0.0], vec![1.0, 0.0], vec![1.0, 0.0]).unwrap();
    let r: TransportResult = flat_transport(&seg, None, 0.01, 10, BSource::None).unwrap();

    // Field-access contract. If any of these renames or moves,
    // compilation here fails first — Marcella's JSON
    // deserialization would break in lockstep, so we want the
    // test to fail before that can happen.
    let _: &Vec<Vec<f64>> = &r.trajectory;
    let _: &Vec<f64> = &r.final_velocity;
    let _: f64 = r.path_length;
    let _: f64 = r.energy_drift;
    let _: f64 = r.holonomy_norm;
    let _: bool = r.used_magnetic;
    let _: BSource = r.b_source;
    let _: Option<f64> = r.closedness_norm;
}

#[test]
fn b_source_variants_match_v2_consumption_enum() {
    // The JSON enum Marcella deserializes: "bundle" | "override"
    // | "none" | "fallback_non_closed". Rust enum variants below
    // map to those JSON strings (the L1.5.3 GQL layer will
    // serialize via serde; this test gates the variant set).
    let _: BSource = BSource::Bundle;
    let _: BSource = BSource::Override;
    let _: BSource = BSource::None;
    let _: BSource = BSource::FallbackNonClosed;

    // Exhaustive match — if a new variant is added without
    // updating the consumption draft, this fails to compile.
    fn assert_exhaustive(s: BSource) -> &'static str {
        match s {
            BSource::Bundle => "bundle",
            BSource::Override => "override",
            BSource::None => "none",
            BSource::FallbackNonClosed => "fallback_non_closed",
        }
    }
    assert_eq!(assert_exhaustive(BSource::Bundle), "bundle");
    assert_eq!(assert_exhaustive(BSource::Override), "override");
    assert_eq!(assert_exhaustive(BSource::None), "none");
    assert_eq!(
        assert_exhaustive(BSource::FallbackNonClosed),
        "fallback_non_closed"
    );
}

#[test]
fn classical_path_b_source_is_none_and_closedness_norm_is_none() {
    // Per consumption draft §2: when no B supplied, b_source =
    // "none" and closedness_norm is omitted (Rust = None).
    let seg =
        TransportSegment::new(vec![0.0, 0.0], vec![1.0, 0.0], vec![1.0, 0.0]).unwrap();
    let r = flat_transport(&seg, None, 0.01, 10, BSource::None).unwrap();
    assert_eq!(r.b_source, BSource::None);
    assert!(r.closedness_norm.is_none());
    assert!(!r.used_magnetic);
}

#[test]
fn override_path_b_source_is_override_and_used_magnetic_is_true() {
    // When the caller passes an override bias, b_source =
    // "override" and used_magnetic = true.
    let bias = ClosedTwoForm::new_constant(TwoForm::new(vec![0.0, 0.5, -0.5, 0.0], 2).unwrap());
    let seg =
        TransportSegment::new(vec![0.0, 0.0], vec![1.0, 0.0], vec![1.0, 0.0]).unwrap();
    let r = flat_transport(&seg, Some(&bias), 0.01, 10, BSource::Override).unwrap();
    assert_eq!(r.b_source, BSource::Override);
    assert!(r.used_magnetic);
    assert!(r.closedness_norm.is_none());
}

#[test]
fn bundle_path_b_source_is_bundle_and_used_magnetic_is_true() {
    // When the bundle's attached B is the source, b_source =
    // "bundle" and used_magnetic = true.
    let bias = ClosedTwoForm::new_constant(TwoForm::new(vec![0.0, 0.5, -0.5, 0.0], 2).unwrap());
    let seg =
        TransportSegment::new(vec![0.0, 0.0], vec![1.0, 0.0], vec![1.0, 0.0]).unwrap();
    let r = flat_transport(&seg, Some(&bias), 0.01, 10, BSource::Bundle).unwrap();
    assert_eq!(r.b_source, BSource::Bundle);
    assert!(r.used_magnetic);
}

#[test]
fn production_energy_drift_bound_is_1e_minus_9() {
    // Per consumption draft §2: energy_drift MUST be < 1e-9 per
    // turn in production. The cyclotron test in the unit-test
    // suite hits this on the toy case; this is a duplicate
    // assertion that the contract is checked here in the
    // contract test too.
    use std::f64::consts::PI;
    let b = 1.5_f64;
    let period = 2.0 * PI / b;
    let dt = 1e-4;
    let n_steps = (period / dt).round() as usize;
    let bias = ClosedTwoForm::new_constant(TwoForm::new(vec![0.0, b, -b, 0.0], 2).unwrap());
    let seg =
        TransportSegment::new(vec![0.0, 0.0], vec![0.0, 0.0], vec![1.0, 0.0]).unwrap();
    let r = flat_transport(&seg, Some(&bias), dt, n_steps, BSource::Override).unwrap();
    assert!(
        r.energy_drift < 1e-9,
        "production contract: energy_drift < 1e-9; got {}",
        r.energy_drift
    );
}
