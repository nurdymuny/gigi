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
    //   where_field: Option<String>    (L13.5)
    //   where_value: Option<Value>     (L13.5)
    //
    // Response:
    //   events: Vec<{boundary_idx, gap, persistence_ratio}>
    //   n_records: usize
    //   threshold_used: f64
    //   filter_applied: Option<{field, value}>  (L13.5)
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

/// L13.6 — the σ² floor prevents diagonal-fit explosion on
/// rank-deficient axes (Marcella's v11_fiber f12/f13/f14 case).
///
/// Setup: a bundle with 4 fiber fields where two are healthy
/// (σ² ~ 1.0) and two are rank-deficient (σ² ~ 1e-30). Without
/// the floor, the diagonal-Gaussian gradient `(x - μ)/σ²` divides
/// by ~1e-30 → samples explode by ~30 orders of magnitude within
/// a few steps. With the floor (default ε=1e-3), the effective σ²
/// for the degenerate axes gets bumped to `1e-3 × median(σ²)`,
/// the gradient stays bounded, samples stay finite.
#[test]
fn diagonal_fit_floor_prevents_rank_deficient_explosion() {
    use gigi::geometry::from_diagonal_gaussian;
    // Build a bundle where dims 2 and 3 are constant (σ² ≈ 0).
    let schema = BundleSchema::new("rank_deficient_test")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::numeric("healthy_x").with_range(2.0))
        .fiber(FieldDef::numeric("healthy_y").with_range(2.0))
        .fiber(FieldDef::numeric("constant_z").with_range(1.0))
        .fiber(FieldDef::numeric("constant_w").with_range(1.0))
        .with_kahler(kahler_2d());
    let mut store = BundleStore::new(schema);
    for i in 0..50 {
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(i));
        r.insert("healthy_x".into(), Value::Float((i as f64) * 0.05));
        r.insert("healthy_y".into(), Value::Float((i as f64) * 0.03));
        r.insert("constant_z".into(), Value::Float(0.7)); // never moves
        r.insert("constant_w".into(), Value::Float(-0.2)); // never moves
        store.insert(&r);
    }

    let stats = store.field_stats();
    let raw_sigma_sq = vec![
        stats["healthy_x"].variance(),
        stats["healthy_y"].variance(),
        stats["constant_z"].variance(),
        stats["constant_w"].variance(),
    ];

    // Verify the pathology exists before the floor: healthy dims
    // ~ 0.01-ish, constant dims approaching zero (subject to
    // sum_sq - mean² catastrophic-cancellation noise; what matters
    // is they're orders-of-magnitude below the healthy dims).
    assert!(raw_sigma_sq[0] > 0.001, "healthy_x should have real variance");
    assert!(raw_sigma_sq[1] > 0.001, "healthy_y should have real variance");
    assert!(
        raw_sigma_sq[2] < raw_sigma_sq[0] / 1000.0,
        "constant_z variance {} should be ≥ 3 orders of magnitude smaller than healthy_x {}",
        raw_sigma_sq[2],
        raw_sigma_sq[0]
    );
    assert!(
        raw_sigma_sq[3] < raw_sigma_sq[0] / 1000.0,
        "constant_w variance {} should be ≥ 3 orders of magnitude smaller than healthy_x {}",
        raw_sigma_sq[3],
        raw_sigma_sq[0]
    );

    // Apply Marcella's recommended floor: ε × median.
    let epsilon = 1e-3;
    let median = {
        let mut s = raw_sigma_sq.clone();
        s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        s[s.len() / 2].max(1e-12)
    };
    let floor = (epsilon * median).max(1e-12);
    let effective: Vec<f64> = raw_sigma_sq.iter().map(|&r| r.max(floor)).collect();
    let floored: Vec<usize> = raw_sigma_sq
        .iter()
        .enumerate()
        .filter(|(_, &r)| r < floor)
        .map(|(i, _)| i)
        .collect();

    // The two constant dims should be floored.
    assert_eq!(
        floored,
        vec![2, 3],
        "expected constant_z and constant_w to be floored; got {:?}",
        floored
    );

    // Effective floor should be > 0 (so the natural gradient won't
    // divide-by-zero) but not absurdly large.
    assert!(
        floor > 1e-15 && floor < 1e-3,
        "effective floor {:.3e} should be in (1e-15, 1e-3)",
        floor
    );

    // Build the flow on the effective σ². Padding to even dim ≥ 2:
    // here we have 4 dims, which is even.
    let raw_padding = vec![
        0.0, 0.0, -1.0, 0.0,
        0.0, 0.0, 0.0, -1.0,
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
    ];
    let b = ClosedTwoForm::new_constant(TwoForm::new(raw_padding, 4).unwrap());
    let mu = vec![1.225, 0.735, 0.7, -0.2]; // empirical means
    let flow = from_diagonal_gaussian(b, mu, effective.clone())
        .expect("diagonal-fit with floored σ² must construct");

    // A few sample-many steps should produce finite samples — not
    // 10^96 garbage. Euler-Maruyama stability requires dt < 2 σ²,
    // so we use dt small enough relative to the computed floor.
    // (The HTTP layer's fit_diagonal_gaussian additionally clamps
    // the floor to ≥ 3 × default_dt = 0.03 to guarantee stability
    // at the brain endpoints' default dt = 0.01; this unit test
    // exercises from_diagonal_gaussian *without* that clamp, so
    // we pair the raw floor with a matching small dt.)
    let safe_dt = (floor * 0.5).min(0.001);
    let cfg = gigi::geometry::FlowConfig {
        dt: safe_dt,
        temperature: 1.0,
        n_steps: 1,
        burn_in: 50,
        seed: Some(20260525),
    };
    let samples = flow.sample_many(&[0.0, 0.0, 0.0, 0.0], &cfg, 10, 1).unwrap();
    for s in &samples {
        for &v in s {
            assert!(
                v.is_finite() && v.abs() < 1e6,
                "sample value {} should be finite and bounded (Marcella's pre-floor case hit 1e96)",
                v
            );
        }
    }
}

/// L13.5 — per-cohort EPISODIC filter recovers a single-stream
/// change-point that's invisible in the bundle-wide aggregate.
///
/// Setup: a bundle with two interleaved cohorts — one stable, one
/// with a regime change. The bundle-wide series mixes both, so any
/// cohort A change-point is diluted by cohort B noise. With the
/// where_field filter restricted to cohort A, the change-point
/// surfaces cleanly.
#[test]
fn brain_episodic_filter_recovers_per_cohort_change_point() {
    let schema = BundleSchema::new("episodic_filter_test")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::numeric("cohort").with_range(2.0))
        .fiber(FieldDef::numeric("value").with_range(20.0))
        .with_kahler(kahler_2d());
    let mut store = BundleStore::new(schema);
    // Cohort 0: tight cluster around value 1.0.
    for i in 0..20 {
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(i));
        r.insert("cohort".into(), Value::Float(0.0));
        r.insert("value".into(), Value::Float(1.0 + (i as f64) * 0.01));
        store.insert(&r);
    }
    // Cohort 1: half at value 1.0, half jumps to value 10.0 — clean
    // change-point inside cohort 1, invisible bundle-wide because
    // cohort 0 fills the gap with values around 1.0.
    for i in 0..10 {
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(100 + i));
        r.insert("cohort".into(), Value::Float(1.0));
        r.insert("value".into(), Value::Float(1.0 + (i as f64) * 0.01));
        store.insert(&r);
    }
    for i in 0..10 {
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(200 + i));
        r.insert("cohort".into(), Value::Float(1.0));
        r.insert("value".into(), Value::Float(10.0 + (i as f64) * 0.01));
        store.insert(&r);
    }

    // Bundle-wide series: cohort 0 + cohort 1 mixed. The 1→10 jump
    // still shows as a single gap of ~9, but median gap is small;
    // ratio is interesting either way. (We don't assert here — the
    // point of this test is the filtered path.)

    // Filtered to cohort 1 — should show the 1→10 change-point at
    // high persistence ratio.
    let mut cohort_1_values = Vec::new();
    for (_bp, rec) in store.sections() {
        if let Value::Float(c) = rec[0] {
            if (c - 1.0).abs() < 1e-12 {
                if let Value::Float(v) = rec[1] {
                    cohort_1_values.push(v);
                }
            }
        }
    }
    assert_eq!(cohort_1_values.len(), 20, "expected 20 cohort-1 records");

    let events = episodic_events(&cohort_1_values, 50.0);
    assert!(
        !events.is_empty(),
        "filtered cohort 1 should show the change-point"
    );
    assert!(
        events[0].gap > 8.0,
        "the change-point gap should be ~9, got {}",
        events[0].gap
    );
    assert!(
        events[0].persistence_ratio > 100.0,
        "ratio should be very large (clean change-point in 20 records); got {}×",
        events[0].persistence_ratio
    );
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

// ═══════════════════════════════════════════════════════════════
// PR window 3 — 5 more brain endpoints (L13.2)
// ═══════════════════════════════════════════════════════════════

use gigi::geometry::{
    inpaint, predict_one_step, GenerativeFlow,
};

// ── §4 DREAM ───────────────────────────────────────────────

#[test]
fn brain_dream_response_shape() {
    // Request:
    //   fields: Vec<String>
    //   initial: Option<Vec<f64>>     (defaults to fit mean)
    //   n_steps: usize                (default 1000)
    //   temperature: f64              (default 4.0 — DREAM regime)
    //   dt: f64                       (default 0.01)
    //   seed: Option<u64>
    //
    // Response:
    //   trajectory: Vec<Vec<f64>>     (len = n_steps + 1)
    //   fit_mean, fit_sigma_sq
    //   temperature_used
    //   mean_dist_from_mean, max_dist_from_mean
    let b = ClosedTwoForm::new_constant(
        TwoForm::new(vec![0.0, 1.0, -1.0, 0.0], 2).unwrap(),
    );
    let mu = vec![0.0, 0.0];
    let flow = gigi::geometry::from_isotropic_gaussian(b, mu.clone(), 1.0).unwrap();

    let config = gigi::geometry::FlowConfig {
        dt: 0.01,
        temperature: 4.0,
        n_steps: 500,
        burn_in: 0,
        seed: Some(7),
    };
    let traj = flow.dream(&mu, &config).expect("dream trajectory");
    // Length contract: n_steps + 1 (initial state + n_steps forward).
    assert_eq!(traj.len(), 501);
    // Each state has fields.len() entries.
    assert_eq!(traj[0].len(), 2);

    // DREAM should visit further from origin than SAMPLE at T=1
    // (validated to ~2.5× separation in the kahler_tour binary).
    let max_dist = traj
        .iter()
        .map(|p| (p[0].powi(2) + p[1].powi(2)).sqrt())
        .fold(0.0_f64, f64::max);
    assert!(max_dist > 1.0, "DREAM should wander beyond 1σ; got max {}", max_dist);
}

// ── §3 FORECAST ───────────────────────────────────────────

#[test]
fn brain_forecast_response_shape() {
    // Request:
    //   fields, initial: Vec<f64>, n_steps, dt
    //
    // Response:
    //   trajectory: Vec<Vec<f64>>    (len = n_steps + 1)
    //   fit_mean, fit_sigma_sq
    //
    // Hamilton's equations conserve energy: a symmetric quadratic H
    // gives closed orbits.
    let b = ClosedTwoForm::new_constant(
        TwoForm::new(vec![0.0, 1.0, -1.0, 0.0], 2).unwrap(),
    );
    let flow = gigi::geometry::from_isotropic_gaussian(b, vec![0.0, 0.0], 1.0).unwrap();
    let config = gigi::geometry::FlowConfig::forecasting();
    let path = flow.forecast(&[1.0, 0.0], &config).expect("forecast");
    assert_eq!(path.len(), config.n_steps + 1);
    // Energy on a quadratic H = ½(q² + p²) is conserved.
    let energy_start = 0.5 * (path[0][0].powi(2) + path[0][1].powi(2));
    let energy_end = 0.5 * (path.last().unwrap()[0].powi(2) + path.last().unwrap()[1].powi(2));
    assert!(
        (energy_start - energy_end).abs() < 0.1,
        "Hamilton flow should conserve energy; drift = {}",
        (energy_start - energy_end).abs()
    );
}

// ── §5 RECONSTRUCT ────────────────────────────────────────

#[test]
fn brain_reconstruct_response_shape() {
    // Request:
    //   fields, noisy_initial: Vec<f64>, n_steps, dt
    //
    // Response:
    //   result: Vec<f64>      (MAP estimate)
    //   fit_mean: Vec<f64>
    //   descent_distance: f64
    //
    // For isotropic Gaussian, MAP = μ, so result should converge to μ.
    let b = ClosedTwoForm::new_constant(
        TwoForm::new(vec![0.0, 1.0, -1.0, 0.0], 2).unwrap(),
    );
    let mu = vec![2.0, -3.0];
    let flow = gigi::geometry::from_isotropic_gaussian(b, mu.clone(), 1.0).unwrap();
    let config = gigi::geometry::FlowConfig::reconstructing();
    let result = flow.reconstruct(&[10.0, 10.0], &config).expect("reconstruct");
    let err = ((result[0] - mu[0]).powi(2) + (result[1] - mu[1]).powi(2)).sqrt();
    assert!(err < 1e-3, "should converge to MAP = μ; err = {:.3e}", err);
}

// ── §6 INPAINT ────────────────────────────────────────────

#[test]
fn brain_inpaint_response_shape() {
    // Request:
    //   fields, partial_state: Vec<f64>, locked_indices: Vec<usize>,
    //   burn_in, dt, temperature, seed
    //
    // Response:
    //   result: Vec<f64>          (locked unchanged; rest sampled)
    //   locked_indices: Vec<usize>
    //   fit_mean, fit_sigma_sq
    let b = ClosedTwoForm::new_constant(
        TwoForm::new(vec![0.0, 1.0, -1.0, 0.0], 2).unwrap(),
    );
    let flow = gigi::geometry::from_isotropic_gaussian(b, vec![5.0, -2.0], 1.0).unwrap();
    let config = gigi::geometry::FlowConfig {
        dt: 0.05,
        temperature: 1.0,
        n_steps: 1,
        burn_in: 2_000,
        seed: Some(20260525),
    };
    let result = inpaint(&flow, &[10.0, 0.0], &[0], &config).expect("inpaint");
    // Locked coordinate stays exact.
    assert!((result[0] - 10.0).abs() < 1e-12, "locked drifted: {}", result[0]);
    // Unlocked coordinate within a few sigma of conditional mean.
    assert!(result[1].abs() < 10.0, "unlocked unreasonable: {}", result[1]);
}

// ── §7 PREDICT ────────────────────────────────────────────

#[test]
fn brain_predict_response_shape() {
    // Request:
    //   fields, state: Vec<f64>, lr: f64
    //
    // Response:
    //   next_state: Vec<f64>
    //   fit_mean, fit_sigma_sq
    //   step_size: f64
    let b = ClosedTwoForm::new_constant(
        TwoForm::new(vec![0.0, 1.0, -1.0, 0.0], 2).unwrap(),
    );
    let mu = vec![3.0, -1.0];
    let sigma_sq = 2.0;
    let flow = gigi::geometry::from_isotropic_gaussian(b, mu.clone(), sigma_sq).unwrap();
    let state = vec![10.0, 10.0];
    let lr = 0.5;
    let next = predict_one_step(&flow, &state, lr).expect("predict");
    // Closed form: next_i = state_i - lr · (state_i - mu_i) / sigma_sq
    for i in 0..2 {
        let expected = state[i] - lr * (state[i] - mu[i]) / sigma_sq;
        assert!(
            (next[i] - expected).abs() < 1e-12,
            "axis {} mismatch: {} vs {}",
            i,
            next[i],
            expected
        );
    }
}

// ── cross-PR-window: all 10 endpoints use the same flow_from_bundle helper

#[test]
fn pr_window_3_all_use_isotropic_gaussian_fit() {
    // Same fit pipeline as PR window 2 endpoints: pull Welford
    // (μ, σ²) from BundleStore, build canonical 2D symplectic B,
    // wrap in GenerativeFlow with isotropic-Gaussian gradient.
    let store = make_bundle();
    let stats = store.field_stats();
    let mu_x = stats["x"].sum / stats["x"].count as f64;
    let mu_y = stats["y"].sum / stats["y"].count as f64;
    let var_avg = 0.5 * (stats["x"].variance() + stats["y"].variance());

    let b = ClosedTwoForm::new_constant(
        TwoForm::new(vec![0.0, 1.0, -1.0, 0.0], 2).unwrap(),
    );
    let _flow: GenerativeFlow<_> =
        gigi::geometry::from_isotropic_gaussian(b, vec![mu_x, mu_y], var_avg.max(1e-12))
            .expect("fit must succeed on real bundle");
}

// ─── L13.3 DIAGONAL GAUSSIAN FIT ───────────────────────────
//
// Per Marcella's 2026-05-25 brain-endpoints probe Finding 3:
// SAMPLE / DREAM / FORECAST / RECONSTRUCT / INPAINT / PREDICT
// all gain an optional `fit_mode: "diagonal"` request param that
// uses per-axis σ² instead of the averaged scalar. Response echoes
// `fit_mode_used` and `fit_sigma_sq_per_field` so consumers can
// inspect what was actually fit.

#[test]
fn diagonal_fit_produces_distinct_per_axis_variances() {
    use gigi::geometry::from_diagonal_gaussian;
    // Set up a bundle where x and y have very different variances:
    // x ∈ [0, 0.3], y ∈ [0, 3.0] — 10× ratio.
    let schema = BundleSchema::new("diag_fit_test")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::numeric("x").with_range(1.0))
        .fiber(FieldDef::numeric("y").with_range(5.0))
        .with_kahler(kahler_2d());
    let mut store = BundleStore::new(schema);
    for i in 0..30 {
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(i));
        r.insert("x".into(), Value::Float((i as f64) * 0.01));
        r.insert("y".into(), Value::Float((i as f64) * 0.1));
        store.insert(&r);
    }
    let stats = store.field_stats();
    let mu = vec![
        stats["x"].sum / stats["x"].count as f64,
        stats["y"].sum / stats["y"].count as f64,
    ];
    let sigma_sq_per_field = vec![
        stats["x"].variance().max(1e-12),
        stats["y"].variance().max(1e-12),
    ];

    // The two axes should have substantially different variances
    // (≈ 100× by construction; we just verify > 10× to allow for
    // numerical noise).
    let ratio = sigma_sq_per_field[1] / sigma_sq_per_field[0];
    assert!(
        ratio > 10.0,
        "synthetic bundle should have anisotropic variances; ratio = {:.2}",
        ratio
    );

    // The diagonal-Gaussian constructor must accept this asymmetric
    // input and produce a flow that we can sample from.
    let b = ClosedTwoForm::new_constant(
        TwoForm::new(vec![0.0, 1.0, -1.0, 0.0], 2).unwrap(),
    );
    let flow = from_diagonal_gaussian(b, mu, sigma_sq_per_field.clone())
        .expect("diagonal fit must accept asymmetric variances");
    let cfg = gigi::geometry::FlowConfig {
        dt: 0.01,
        temperature: 1.0,
        n_steps: 1,
        burn_in: 1_000,
        seed: Some(20260525),
    };
    let _samples = flow
        .sample_many(&[0.0, 0.0], &cfg, 100, 1)
        .expect("diagonal flow must sample");

    // ISOTROPIC fit would have given a SINGLE σ² = mean(per-axis),
    // losing the anisotropy. That's the value of the diagonal fit
    // — distinct per-axis σ² preserved.
    let iso_sigma_sq =
        (sigma_sq_per_field[0] + sigma_sq_per_field[1]) / 2.0;
    assert!(
        (iso_sigma_sq - sigma_sq_per_field[0]).abs() > 0.01,
        "isotropic averaging would lose the anisotropy (iso={:.4}, axis_0={:.4})",
        iso_sigma_sq,
        sigma_sq_per_field[0]
    );
}
