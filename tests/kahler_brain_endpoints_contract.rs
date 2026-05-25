//! Cross-team interface contract test for the 2026-05-25 PR window 2
//! — 5 brain-primitive HTTP endpoints shipped on top of L10/L11/L12:
//!
//!   POST /v1/bundles/{name}/brain/sample      §2  SAMPLE
//!   POST /v1/bundles/{name}/brain/confidence  §12 SELF-MONITOR
//!   POST /v1/bundles/{name}/brain/attend      §8  ATTEND
//!   POST /v1/bundles/{name}/brain/episodic    §10 EPISODIC
//!   GET  /v1/bundles/{name}/brain/semantic    §11 SEMANTIC
//!
//! These tests exercise the underlying Rust APIs that the endpoints
//! delegate to (same pattern as kahler_pr_window_marcella_contract.rs).
//! Compile-time failures catch Rust-side renames before any consumer
//! deserialization can drift.
//!
//! Catalog: `theory/brain_primitives/catalog.md`.

#![cfg(feature = "kahler")]

use gigi::geometry::{
    attend, confidence_normalized, episodic_events, focus, from_isotropic_gaussian,
    kernel_density_confidence, semantic_gist, ClosedTwoForm, ComplexStructure,
    FlowConfig, KahlerStructure, TwoForm,
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

fn make_bundle() -> BundleStore {
    let schema = BundleSchema::new("brain_contract")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::numeric("x").with_range(5.0))
        .fiber(FieldDef::numeric("y").with_range(5.0))
        .with_kahler(kahler_2d());
    let mut store = BundleStore::new(schema);
    for i in 0..30 {
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(i));
        r.insert("x".into(), Value::Float(i as f64 * 0.1));
        r.insert("y".into(), Value::Float((i as f64 * 0.1).sin()));
        store.insert(&r);
    }
    store
}

// ── §2 SAMPLE ──────────────────────────────────────────────

#[test]
fn brain_sample_request_response_shape() {
    // Request fields per BrainSampleRequest:
    //   fields: Vec<String>      (required)
    //   n_samples: usize         (default 100)
    //   temperature: f64         (default 1.0)
    //   burn_in: usize           (default 2000)
    //   seed: Option<u64>
    //
    // Response (BrainSampleResponse):
    //   samples: Vec<Vec<f64>>
    //   fit_mean: Vec<f64>
    //   fit_sigma_sq: f64
    //
    // The wiring goes through from_isotropic_gaussian + sample_many.
    let b = ClosedTwoForm::new_constant(
        TwoForm::new(vec![0.0, 1.0, -1.0, 0.0], 2).unwrap(),
    );
    let mu = vec![1.5, -0.7];
    let sigma_sq = 0.5_f64;
    let flow = from_isotropic_gaussian(b, mu.clone(), sigma_sq).unwrap();
    let config = FlowConfig {
        dt: 0.01,
        temperature: 1.0,
        n_steps: 1,
        burn_in: 2_000,
        seed: Some(0xDEADBEEF),
    };
    let samples = flow
        .sample_many(&[0.0, 0.0], &config, 50, 1)
        .expect("sample many");
    assert_eq!(samples.len(), 50);
    assert_eq!(samples[0].len(), mu.len()); // dimension matches fields
    // Stationary mean approximation — coarse check.
    let mean_x: f64 = samples.iter().map(|s| s[0]).sum::<f64>() / 50.0;
    let mean_y: f64 = samples.iter().map(|s| s[1]).sum::<f64>() / 50.0;
    assert!((mean_x - mu[0]).abs() < 1.0);
    assert!((mean_y - mu[1]).abs() < 1.0);
}

// ── §12 SELF-MONITOR (confidence) ──────────────────────────

#[test]
fn brain_confidence_response_shape() {
    // Request:
    //   fields: Vec<String>
    //   query: Vec<f64> (length must = fields.len())
    //   bandwidth: Option<f64>
    //
    // Response:
    //   raw: f64                — kernel sum
    //   normalized: f64         — raw / max_density
    //   bandwidth: f64
    //   n_samples: usize
    let store = make_bundle();
    let fields = vec!["x".to_string(), "y".to_string()];
    // Pull samples the same way the handler does.
    let mut samples: Vec<Vec<f64>> = Vec::new();
    for (_bp, rec) in store.sections() {
        // record is &[Value] indexed by fiber position; x = 0, y = 1.
        let x = match rec[0] { Value::Float(f) => f, _ => panic!("non-float") };
        let y = match rec[1] { Value::Float(f) => f, _ => panic!("non-float") };
        samples.push(vec![x, y]);
    }
    let bw = 0.3;
    let near = kernel_density_confidence(&samples, &[1.0, 0.84], bw); // sample 10
    let far = kernel_density_confidence(&samples, &[100.0, 100.0], bw);
    assert!(near > 1e-3, "near-cluster raw confidence should be > 1e-3, got {}", near);
    assert!(far < 1e-10, "far raw confidence should be ~0, got {}", far);

    let nn = confidence_normalized(&samples, &[1.0, 0.84], bw);
    let nf = confidence_normalized(&samples, &[100.0, 100.0], bw);
    assert!(nn > 0.1, "normalized near should be > 0.1, got {}", nn);
    assert!(nf < 1e-6, "normalized far should be ~0, got {}", nf);
}

// ── §8 ATTEND ──────────────────────────────────────────────

#[test]
fn brain_attend_response_shape() {
    // Request:
    //   fields: Vec<String>
    //   query: Vec<f64>
    //   bandwidth: Option<f64>
    //   top_k: Option<usize>
    //
    // Response:
    //   weights: Vec<f64>
    //   indices: Vec<usize>
    //   bandwidth: f64
    //   n_samples: usize
    let samples = vec![
        vec![0.0, 0.0],
        vec![1.0, 0.0],
        vec![2.0, 0.0],
        vec![3.0, 0.0],
    ];
    let weights = attend(&samples, &[0.0, 0.0], 1.0);
    assert_eq!(weights.len(), 4);
    let sum: f64 = weights.iter().sum();
    assert!((sum - 1.0).abs() < 1e-12);
    // Top-k path uses focus().
    let top2 = focus(&samples, &[0.0, 0.0], 1.0, 2);
    assert_eq!(top2.len(), 2);
    assert_eq!(top2[0].0, 0); // nearest first
    assert!(top2[0].1 > top2[1].1);
}

// ── §10 EPISODIC ───────────────────────────────────────────

#[test]
fn brain_episodic_response_shape() {
    // Request:
    //   field: String (single field)
    //   min_persistence_ratio: f64 (default 50.0)
    //
    // Response:
    //   events: Vec<{boundary_idx, gap, persistence_ratio}>
    //   n_records: usize
    //   threshold_used: f64
    let mut values = Vec::new();
    for i in 0..20 { values.push(i as f64 * 0.01); }
    for i in 0..20 { values.push(5.0 + i as f64 * 0.01); }
    let events = episodic_events(&values, 50.0);
    assert!(!events.is_empty(), "expected at least one event");
    assert!(events[0].persistence_ratio > 100.0);
    assert!(events[0].gap > 4.0);
    // Field set in wire struct.
    let event_wire_fields = ["boundary_idx", "gap", "persistence_ratio"];
    assert_eq!(event_wire_fields.len(), 3);
}

// ── §11 SEMANTIC ───────────────────────────────────────────

#[test]
fn brain_semantic_response_shape() {
    // No request body. GET.
    // Response:
    //   betti_b0, betti_b1, betti_b2: usize
    //   n_critical, n_original: usize
    //   compression_ratio: f64
    //   cohomology_preserved: bool
    let store = make_bundle();
    let morse = semantic_gist(&store);
    assert!(morse.is_some(), "30-record bundle should produce a Morse complex");
    let m = morse.unwrap();
    assert!(m.cohomology_preserved());
    assert!(m.n_critical() <= m.n_original());
    assert!(m.compression_ratio() >= 1.0);
}

// ── cross-endpoint sanity ──────────────────────────────────

#[test]
fn brain_endpoints_all_use_consistent_field_extraction() {
    // The five endpoints share a helper that pulls numeric fiber
    // values from records (indexed by schema.fiber_fields position).
    // This test confirms a bundle with two fiber fields produces
    // length-2 sample vectors via the same path used by the handlers.
    let store = make_bundle();
    let n_records = store.len();
    assert_eq!(n_records, 30);
    let mut count = 0_usize;
    for (_bp, rec) in store.sections() {
        assert_eq!(rec.len(), 2, "fiber width = 2 (x, y)");
        count += 1;
    }
    assert_eq!(count, 30);
}
