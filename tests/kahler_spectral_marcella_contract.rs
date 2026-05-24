//! Cross-team interface contract test for L3.4 — the
//! `GET /v1/bundles/<name>/spectral_gap` Marcella surface.
//!
//! Source of truth: `theory/kahler_upgrade/marcella_kahler_consumption_v2.md §4`
//! plus the GIGI reply Q4 (2026-05-24): "Two surfaces ship at L3 —
//! in-response field on every retrieval AND a dedicated endpoint."
//!
//! This test gates the *Rust struct* shape that powers the JSON
//! serialization. If a field is renamed or removed on the Rust side,
//! compilation here fails BEFORE Marcella's deserialization can
//! drift in the wild.
//!
//! ### Contract fields under test (consumption draft v2 §4)
//!
//! | JSON field        | Rust field        | Type      |
//! |-------------------|-------------------|-----------|
//! | `lambda_2`        | `lambda_2`        | f64       |
//! | `mix_time`        | `mix_time`        | u64       |
//! | `cheeger_lower`   | `cheeger_lower`   | f64       |
//! | `cheeger_upper`   | `cheeger_upper`   | f64       |
//! | `cached_at`       | `cached_at`       | String    |
//!
//! Also asserts the mathematical invariants Marcella depends on:
//!   * `cheeger_lower = λ₂ / 2`
//!   * `cheeger_upper = √(2 λ₂)`
//!   * `mix_time = ⌈(1/λ₂) · ln(1/ε)⌉`, ε = 1e-3
//!   * disconnected graph (λ₂ = 0) yields `mix_time = u64::MAX`

#![cfg(feature = "kahler")]

use gigi::bundle::{BundleStore, SpectralGapSnapshot};
use gigi::types::{BundleSchema, FieldDef, Record, Value};

/// Build a small two-field bundle with enough records so the
/// field-index graph has structure (`λ₂ > 0` whenever connected).
/// Mirrors the `make_spectral_test_store` helper in `bundle.rs`
/// tests so the contract test exercises the same shape Marcella
/// will see in production.
fn bundle_with_records(n: usize) -> BundleStore {
    let schema = BundleSchema::new("contract_spectral")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::categorical("tier"))
        .fiber(FieldDef::categorical("topic"))
        .index("tier")
        .index("topic");
    let mut store = BundleStore::new(schema);
    let tiers = ["A", "B"];
    let topics = ["math", "code", "geo"];
    for i in 0..n {
        let mut r = Record::new();
        r.insert("id".into(), Value::Float(i as f64));
        r.insert("tier".into(), Value::Text(tiers[i % tiers.len()].into()));
        r.insert("topic".into(), Value::Text(topics[i % topics.len()].into()));
        store.insert(&r);
    }
    store
}

#[test]
fn snapshot_struct_has_marcella_v2_field_set() {
    // If any field renames or moves, compilation here fails first —
    // before Marcella's JSON deserialization can drift.
    let s = bundle_with_records(8);
    let snap: SpectralGapSnapshot = s.spectral_gap_cached().expect("≥ 2 records");

    let _: f64 = snap.lambda_2;
    let _: u64 = snap.mix_time;
    let _: f64 = snap.cheeger_lower;
    let _: f64 = snap.cheeger_upper;
    let _: String = snap.cached_at;
}

#[test]
fn cheeger_bounds_satisfy_consumption_draft_formulas() {
    // Consumption draft v2 §4: cheeger_lower = λ₂/2; cheeger_upper = √(2λ₂)
    let s = bundle_with_records(8);
    let snap = s.spectral_gap_cached().expect("≥ 2 records");

    let expected_lower = snap.lambda_2 / 2.0;
    let expected_upper = (2.0 * snap.lambda_2).sqrt();

    assert!(
        (snap.cheeger_lower - expected_lower).abs() < 1e-12,
        "cheeger_lower contract: λ₂/2 = {}; got {}",
        expected_lower,
        snap.cheeger_lower
    );
    assert!(
        (snap.cheeger_upper - expected_upper).abs() < 1e-12,
        "cheeger_upper contract: √(2λ₂) = {}; got {}",
        expected_upper,
        snap.cheeger_upper
    );

    // Cheeger sandwich must hold: lower ≤ upper. Independent of λ₂.
    assert!(
        snap.cheeger_lower <= snap.cheeger_upper + 1e-12,
        "cheeger_lower ({}) must be ≤ cheeger_upper ({})",
        snap.cheeger_lower,
        snap.cheeger_upper
    );
}

#[test]
fn mix_time_matches_marcella_formula() {
    // Marcella uses α = 1 - 1/sqrt(mix_time). We compute mix_time as
    // ⌈(1/λ₂) · ln(1/ε)⌉, ε = 1e-3.
    let s = bundle_with_records(8);
    let snap = s.spectral_gap_cached().expect("≥ 2 records");

    if snap.lambda_2 > 0.0 {
        let expected = ((1.0 / snap.lambda_2) * (1.0_f64 / 1e-3).ln()).ceil() as u64;
        assert_eq!(
            snap.mix_time, expected,
            "mix_time contract: ⌈(1/λ₂)·ln(1/ε)⌉ = {}; got {}",
            expected, snap.mix_time
        );
    } else {
        // Disconnected: Marcella reads mix_time = u64::MAX as the
        // "do not apply spectral tuning to this bundle" signal.
        assert_eq!(
            snap.mix_time,
            u64::MAX,
            "disconnected graph contract: mix_time must be u64::MAX (got {})",
            snap.mix_time
        );
    }
}

#[test]
fn cached_at_is_non_empty_and_marcella_can_parse_it() {
    // The exact format is opaque to Marcella — she only needs to
    // detect "did this string change" between reads. We assert
    // non-empty + ASCII so the JSON serialization is stable.
    let s = bundle_with_records(8);
    let snap = s.spectral_gap_cached().expect("≥ 2 records");

    assert!(!snap.cached_at.is_empty(), "cached_at must be non-empty");
    assert!(
        snap.cached_at.is_ascii(),
        "cached_at must be ASCII for stable JSON serialization (got {})",
        snap.cached_at
    );
}

#[test]
fn endpoint_returns_none_for_bundle_with_too_few_records() {
    // Negative case: the HTTP handler in `gigi_stream::spectral_gap_endpoint`
    // maps `None` from `spectral_gap_cached()` to a 404 response.
    // Here we exercise the producer side directly — if a regression
    // makes 1-record bundles return Some, the endpoint would happily
    // serve a degenerate snapshot and Marcella would compute α from
    // garbage.
    let s = bundle_with_records(1);
    assert!(
        s.spectral_gap_cached().is_none(),
        "single-record bundle: spectral_gap_cached must return None"
    );

    // And empty bundle, the other degenerate case.
    let schema = BundleSchema::new("empty")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::categorical("k"))
        .index("k");
    let s_empty = BundleStore::new(schema);
    assert!(
        s_empty.spectral_gap_cached().is_none(),
        "empty bundle: spectral_gap_cached must return None"
    );
}
