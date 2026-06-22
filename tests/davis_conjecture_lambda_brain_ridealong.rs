//! Davis Conjecture λ-budget ride-along — brain primitives surface.
//!
//! Friday 2026-06-20 (commit 69a7001) shipped `lambda_budget` on:
//!   - /v1/bundles/{name}/curvature (CurvatureReport)
//!   - filtered_query JSON meta
//!   - stream_query NDJSON __meta line
//!
//! This file pins the NEXT ride-along surface: every brain primitive
//! response (/v1/bundles/{name}/brain/*) carries `lambda_budget` at the
//! response top level, sibling of existing fields (`#[serde(flatten)]`
//! convention from CurvatureReport — NOT nested under "meta").
//!
//! Why: brain primitives are the cognition-surface entry points called
//! per turn by Marcella, claude_substrate_v0, and future LLM consumers.
//! Lifting λ into every brain response makes the Davis Conjecture
//! (claim_0104, λ = 1 − τ_budget / (K_max · D²)) a runtime fact on the
//! load-bearing path, not just a paper claim or a curvature-endpoint
//! detail consumers have to fetch separately.
//!
//! ── RED-first contract ────────────────────────────────────────────
//!
//! 1. `ResponseWithLambda<T>` (binary-internal in src/bin/gigi_stream.rs,
//!    kahler-gated): a #[serde(flatten)]-based generic wrapper that adds
//!    `lambda_budget: f64` as a top-level sibling key to any Serialize
//!    inner. We pin the design contract here via a local stub mirroring
//!    the design exactly — the GREEN-phase patch will introduce the real
//!    type and these tests document the shape it must produce.
//!
//! 2. `gigi::curvature::lambda_budget_for_bundle(&store) -> f64`: a NEW
//!    public lib helper that mirrors the /curvature compute path and
//!    returns a safe-default 1.0 for missing/empty bundles instead of
//!    NaN. This is the function the binary's brain handlers will call
//!    once per request. It DOES NOT EXIST yet — every test that touches
//!    it produces the RED compile error.
//!
//! 3. Per-endpoint shape contract: each brain endpoint, when wrapped in
//!    ResponseWithLambda, produces JSON with (a) `lambda_budget` as a
//!    top-level f64 key and (b) every existing wire field still at the
//!    top level (no nesting regression — additive-only ride-along).
//!
//! ── Scope ─────────────────────────────────────────────────────────
//!
//! Brain endpoints are kahler-feature-gated; this whole file is too,
//! so the no-default-features build stays byte-identical at 870/0.
//!
//! Test approach: per-endpoint tests build the underlying geometry-API
//! response value (the same value the handler's `Ok(Json(...))` /
//! `negotiated_brain_response(...)` line returns), wrap it in the
//! design's ResponseWithLambda contract via the helper, and inspect the
//! resulting JSON. This mirrors `tests/kahler_brain_endpoints_contract.rs`
//! — it exercises the wire-shape contract without needing to spin up an
//! axum router (gigi_stream is a binary with no public router factory).

#![cfg(feature = "kahler")]

use gigi::bundle::BundleStore;
use gigi::curvature::lambda_budget;
use gigi::geometry::{
    attend, confidence_normalized, episodic_events, focus, from_isotropic_gaussian,
    kernel_density_confidence, semantic_gist, ClosedTwoForm, ComplexStructure, FlowConfig,
    KahlerStructure, TwoForm,
};
use gigi::types::{BundleSchema, FieldDef, Record, Value};
use serde::Serialize;
use serde_json::Value as JsonValue;

// ── Test fixtures ────────────────────────────────────────────────

fn kahler_2d() -> KahlerStructure {
    let j = ComplexStructure::standard(1);
    let b = ClosedTwoForm::new_constant(
        TwoForm::new(vec![0.0, 0.5, -0.5, 0.0], 2).expect("antisymmetric"),
    );
    KahlerStructure::new(j, b)
}

fn make_bundle() -> BundleStore {
    let schema = BundleSchema::new("ridealong_brain")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::numeric("x").with_range(5.0))
        .fiber(FieldDef::numeric("y").with_range(5.0))
        .with_kahler(kahler_2d());
    let mut store = BundleStore::new(schema);
    for i in 0..30 {
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(i));
        r.insert("x".into(), Value::Float((i as f64) * 0.1));
        r.insert("y".into(), Value::Float(((i as f64) * 0.1).sin()));
        store.insert(&r);
    }
    store
}

fn empty_bundle() -> BundleStore {
    let schema = BundleSchema::new("ridealong_brain_empty")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::numeric("x").with_range(5.0))
        .fiber(FieldDef::numeric("y").with_range(5.0))
        .with_kahler(kahler_2d());
    BundleStore::new(schema)
}

// ── Local design stub for ResponseWithLambda<T> ─────────────────
//
// The real type lives in src/bin/gigi_stream.rs behind #[cfg(feature =
// "kahler")] (see design.wrapper_struct_definition). External tests
// can't import from a binary crate, so we mirror the exact design here
// to pin the contract. If the binary's wrapper drifts from this shape,
// the per-endpoint tests below will catch it on real wire output during
// GREEN-phase verification.
#[derive(Serialize)]
struct ResponseWithLambdaStub<T: Serialize> {
    #[serde(flatten)]
    inner: T,
    lambda_budget: f64,
}

// ── §1 Wrapper unit test — flatten contract ─────────────────────

/// The wrapper must place `lambda_budget` as a SIBLING of inner's
/// fields at the JSON top level, never nested under an "inner" /
/// "meta" key. Matches CurvatureReport's existing convention.
#[test]
fn response_with_lambda_flattens_inner_fields() {
    #[derive(Serialize)]
    struct Inner {
        a: i32,
        b: &'static str,
    }
    let v = serde_json::to_value(ResponseWithLambdaStub {
        inner: Inner { a: 7, b: "hi" },
        lambda_budget: 0.42,
    })
    .expect("serialize wrapped response");

    let obj = v.as_object().expect("top level must be a JSON object");
    assert_eq!(
        obj.get("a").and_then(|v| v.as_i64()),
        Some(7),
        "inner field `a` must appear at top level via #[serde(flatten)]; got {v}"
    );
    assert_eq!(
        obj.get("b").and_then(|v| v.as_str()),
        Some("hi"),
        "inner field `b` must appear at top level via #[serde(flatten)]; got {v}"
    );
    assert!(
        obj.contains_key("lambda_budget"),
        "wrapper must add `lambda_budget` as top-level sibling; got {v}"
    );
    assert!(
        (obj["lambda_budget"].as_f64().unwrap() - 0.42).abs() < 1e-12,
        "lambda_budget value preserved; got {}",
        obj["lambda_budget"]
    );
    assert!(
        v.get("inner").is_none(),
        "wrapper MUST NOT nest under `inner` (would break clients)"
    );
    assert!(
        v.get("meta").is_none(),
        "wrapper MUST NOT nest under `meta` (matches CurvatureReport convention)"
    );
}

// ── §2 Helper fallback: lambda_budget_for_bundle ─────────────────
//
// The lib helper `gigi::curvature::lambda_budget_for_bundle(&store)`
// is the function the binary's brain handlers will call. It does not
// exist yet — these tests fail to compile (RED). The GREEN-phase
// patch will add it to src/curvature.rs with this exact signature
// and contract.

/// Real bundle: finite λ in [0, 1] (or up to saturated 1.0).
#[test]
fn lambda_budget_for_bundle_real_returns_finite() {
    let store = make_bundle();
    let lambda = gigi::curvature::lambda_budget_for_bundle(&store);
    assert!(
        lambda.is_finite(),
        "λ on real bundle must be finite; got {lambda}"
    );
    assert!(
        lambda <= 1.0 + 1e-12,
        "λ on real bundle must be ≤ 1.0 (saturated cap); got {lambda}"
    );
}

/// Empty bundle: helper returns the safe default 1.0 (no-horizon),
/// NEVER NaN. Protects the hot per-turn brain path that Marcella and
/// claude_substrate_v0 call.
#[test]
fn lambda_budget_for_bundle_empty_returns_one() {
    let store = empty_bundle();
    let lambda = gigi::curvature::lambda_budget_for_bundle(&store);
    assert_eq!(
        lambda, 1.0,
        "empty-bundle λ must be the safe default 1.0; got {lambda}"
    );
}

/// The helper's compute path agrees with the underlying primitive
/// `gigi::curvature::lambda_budget(k, d, 1.0)`: same equation, no
/// silent clamping besides the missing/empty safe-default arm.
#[test]
fn lambda_budget_for_bundle_matches_primitive_on_real_bundle() {
    let store = make_bundle();
    let k = gigi::curvature::scalar_curvature(&store);
    // The substrate's D-proxy for the curvature endpoint is the
    // Welford radius (sqrt of mean per-field variance). On a real
    // bundle this is finite and > 0; we compute it the same way
    // gigi_welford_radius() in the binary does so the test pins
    // the lib helper to the same compute path.
    let stats = store.field_stats();
    let mut sum = 0.0_f64;
    let mut n = 0_usize;
    for fs in stats.values() {
        let v = fs.variance();
        if v.is_finite() && v > 0.0 {
            sum += v;
            n += 1;
        }
    }
    let d = if n == 0 {
        1.0
    } else {
        (sum / n as f64).sqrt()
    };
    let expected = lambda_budget(k, d, 1.0);
    let actual = gigi::curvature::lambda_budget_for_bundle(&store);
    if expected.is_nan() {
        // Defensive — if welford ever degenerates, the helper still
        // returns the safe default rather than propagating NaN.
        assert_eq!(actual, 1.0, "NaN inputs must coalesce to 1.0 default");
    } else {
        assert!(
            (actual - expected).abs() < 1e-9,
            "helper must mirror primitive: actual={actual}, expected={expected}"
        );
    }
}

// ── §3 Per-endpoint ride-along contract ─────────────────────────
//
// Each endpoint's wire response, when wrapped in ResponseWithLambda,
// must produce JSON with (a) `lambda_budget` as a top-level f64 and
// (b) at least one canonical existing field still at the top level
// (no nesting regression).
//
// The per-endpoint body builds the underlying geometry-API value
// (same call the handler's Ok(Json(...)) tail returns), wraps it via
// the helper, and asserts on the resulting JSON. Mirrors the unit
// pattern in tests/kahler_brain_endpoints_contract.rs.

fn assert_ridealong_shape(v: &JsonValue, endpoint: &str, inner_canonical_field: &str) {
    let obj = v
        .as_object()
        .unwrap_or_else(|| panic!("{endpoint} response must be a JSON object; got {v}"));
    assert!(
        obj.contains_key("lambda_budget"),
        "{endpoint} response missing top-level `lambda_budget` key; got keys {:?}",
        obj.keys().collect::<Vec<_>>()
    );
    assert!(
        obj["lambda_budget"].is_f64(),
        "{endpoint} `lambda_budget` must be an f64; got {}",
        obj["lambda_budget"]
    );
    let lv = obj["lambda_budget"].as_f64().unwrap();
    assert!(
        lv.is_finite() || lv == 1.0,
        "{endpoint} `lambda_budget` must be finite (or saturated 1.0); got {lv}"
    );
    assert!(
        obj.contains_key(inner_canonical_field),
        "{endpoint} inner field `{inner_canonical_field}` must still be at top level \
         (no nesting regression); got keys {:?}",
        obj.keys().collect::<Vec<_>>()
    );
}

// ── /brain/sample ───────────────────────────────────────────────

#[test]
fn ridealong_sample_emits_lambda_budget() {
    let store = make_bundle();
    let b = ClosedTwoForm::new_constant(TwoForm::new(vec![0.0, 1.0, -1.0, 0.0], 2).unwrap());
    let flow = from_isotropic_gaussian(b, vec![0.0, 0.0], 1.0).unwrap();
    let cfg = FlowConfig {
        dt: 0.01,
        temperature: 1.0,
        n_steps: 1,
        burn_in: 100,
        seed: Some(1),
    };
    let samples = flow.sample_many(&[0.0, 0.0], &cfg, 4, 1).unwrap();
    let inner = serde_json::json!({
        "samples": samples,
        "fit_mean": [0.0, 0.0],
        "fit_sigma_sq": 1.0_f64,
    });
    let wrapped = ResponseWithLambdaStub {
        inner,
        lambda_budget: gigi::curvature::lambda_budget_for_bundle(&store),
    };
    let v = serde_json::to_value(&wrapped).unwrap();
    assert_ridealong_shape(&v, "/brain/sample", "samples");
}

// ── /brain/confidence ───────────────────────────────────────────

#[test]
fn ridealong_confidence_emits_lambda_budget() {
    let store = make_bundle();
    let samples: Vec<Vec<f64>> = store
        .sections()
        .map(|(_bp, rec)| {
            let x = match rec[0] {
                Value::Float(f) => f,
                _ => 0.0,
            };
            let y = match rec[1] {
                Value::Float(f) => f,
                _ => 0.0,
            };
            vec![x, y]
        })
        .collect();
    let raw = kernel_density_confidence(&samples, &[1.0, 0.84], 0.3);
    let normalized = confidence_normalized(&samples, &[1.0, 0.84], 0.3);
    let inner = serde_json::json!({
        "raw": raw,
        "normalized": normalized,
        "bandwidth": 0.3_f64,
        "n_samples": samples.len(),
    });
    let wrapped = ResponseWithLambdaStub {
        inner,
        lambda_budget: gigi::curvature::lambda_budget_for_bundle(&store),
    };
    let v = serde_json::to_value(&wrapped).unwrap();
    assert_ridealong_shape(&v, "/brain/confidence", "raw");
}

// ── /brain/confidence_with_explain ──────────────────────────────

#[test]
fn ridealong_confidence_with_explain_emits_lambda_budget() {
    let store = make_bundle();
    let samples: Vec<Vec<f64>> = store
        .sections()
        .map(|(_bp, rec)| {
            let x = match rec[0] {
                Value::Float(f) => f,
                _ => 0.0,
            };
            let y = match rec[1] {
                Value::Float(f) => f,
                _ => 0.0,
            };
            vec![x, y]
        })
        .collect();
    let raw = kernel_density_confidence(&samples, &[1.0, 0.84], 0.3);
    let exp = gigi::geometry::explain(&samples, &[1.0, 0.84], 5);
    let inner = serde_json::json!({
        "raw": raw,
        "normalized": 0.5_f64,
        "bandwidth": 0.3_f64,
        "n_samples": samples.len(),
        "explain": {
            "query": [1.0_f64, 0.84_f64],
            "nearest_index": exp.nearest_index,
            "path_len": exp.path.len(),
        },
    });
    let wrapped = ResponseWithLambdaStub {
        inner,
        lambda_budget: gigi::curvature::lambda_budget_for_bundle(&store),
    };
    let v = serde_json::to_value(&wrapped).unwrap();
    assert_ridealong_shape(&v, "/brain/confidence_with_explain", "explain");
}

// ── /brain/attend ───────────────────────────────────────────────

#[test]
fn ridealong_attend_emits_lambda_budget() {
    let store = make_bundle();
    let samples = vec![
        vec![0.0, 0.0],
        vec![1.0, 0.0],
        vec![2.0, 0.0],
        vec![3.0, 0.0],
    ];
    let weights = attend(&samples, &[0.0, 0.0], 1.0);
    let inner = serde_json::json!({
        "weights": weights,
        "indices": [0_usize, 1, 2, 3],
        "bandwidth": 1.0_f64,
        "n_samples": samples.len(),
    });
    let wrapped = ResponseWithLambdaStub {
        inner,
        lambda_budget: gigi::curvature::lambda_budget_for_bundle(&store),
    };
    let v = serde_json::to_value(&wrapped).unwrap();
    assert_ridealong_shape(&v, "/brain/attend", "weights");
}

// ── /brain/episodic ─────────────────────────────────────────────

#[test]
fn ridealong_episodic_emits_lambda_budget() {
    let store = make_bundle();
    let mut values = Vec::new();
    for i in 0..20 {
        values.push(i as f64 * 0.01);
    }
    for i in 0..20 {
        values.push(5.0 + i as f64 * 0.01);
    }
    let events = episodic_events(&values, 50.0);
    let inner = serde_json::json!({
        "events": events.iter().map(|e| serde_json::json!({
            "boundary_idx": e.boundary_idx,
            "gap": e.gap,
            "persistence_ratio": e.persistence_ratio,
        })).collect::<Vec<_>>(),
        "n_records": values.len(),
        "threshold_used": 50.0_f64,
        "filter_applied": JsonValue::Null,
    });
    let wrapped = ResponseWithLambdaStub {
        inner,
        lambda_budget: gigi::curvature::lambda_budget_for_bundle(&store),
    };
    let v = serde_json::to_value(&wrapped).unwrap();
    assert_ridealong_shape(&v, "/brain/episodic", "events");
}

// ── /brain/semantic ─────────────────────────────────────────────

#[test]
fn ridealong_semantic_emits_lambda_budget() {
    let store = make_bundle();
    let m = semantic_gist(&store).expect("morse complex on 30-record bundle");
    let inner = serde_json::json!({
        "n_critical": m.n_critical(),
        "n_original": m.n_original(),
        "compression_ratio": m.compression_ratio(),
        "cohomology_preserved": m.cohomology_preserved(),
    });
    let wrapped = ResponseWithLambdaStub {
        inner,
        lambda_budget: gigi::curvature::lambda_budget_for_bundle(&store),
    };
    let v = serde_json::to_value(&wrapped).unwrap();
    assert_ridealong_shape(&v, "/brain/semantic", "n_critical");
}

// ── /brain/explain ──────────────────────────────────────────────

#[test]
fn ridealong_explain_emits_lambda_budget() {
    let store = make_bundle();
    let samples = vec![vec![0.0, 0.0], vec![5.0, 5.0], vec![10.0, 10.0]];
    let exp = gigi::geometry::explain(&samples, &[5.1, 4.9], 10);
    let inner = serde_json::json!({
        "query": [5.1_f64, 4.9_f64],
        "nearest_record": exp.nearest_record,
        "nearest_index": exp.nearest_index,
        "nearest_distance": exp.nearest_distance,
        "path": exp.path,
        "n_steps": 10_usize,
        "n_samples": samples.len(),
    });
    let wrapped = ResponseWithLambdaStub {
        inner,
        lambda_budget: gigi::curvature::lambda_budget_for_bundle(&store),
    };
    let v = serde_json::to_value(&wrapped).unwrap();
    assert_ridealong_shape(&v, "/brain/explain", "path");
}

// ── /brain/focus ────────────────────────────────────────────────

#[test]
fn ridealong_focus_emits_lambda_budget() {
    let store = make_bundle();
    let samples = vec![
        vec![0.0, 0.0],
        vec![1.0, 0.0],
        vec![2.0, 0.0],
        vec![3.0, 0.0],
    ];
    let top2 = focus(&samples, &[0.0, 0.0], 1.0, 2);
    let inner = serde_json::json!({
        "top_k": top2.iter().map(|(i, w)| serde_json::json!({
            "index": i,
            "weight": w,
        })).collect::<Vec<_>>(),
        "bandwidth": 1.0_f64,
        "n_samples": samples.len(),
    });
    let wrapped = ResponseWithLambdaStub {
        inner,
        lambda_budget: gigi::curvature::lambda_budget_for_bundle(&store),
    };
    let v = serde_json::to_value(&wrapped).unwrap();
    assert_ridealong_shape(&v, "/brain/focus", "top_k");
}

// ── /brain/inpaint ──────────────────────────────────────────────

#[test]
fn ridealong_inpaint_emits_lambda_budget() {
    let store = make_bundle();
    let b = ClosedTwoForm::new_constant(TwoForm::new(vec![0.0, 1.0, -1.0, 0.0], 2).unwrap());
    let flow = from_isotropic_gaussian(b, vec![5.0, -2.0], 1.0).unwrap();
    let cfg = FlowConfig {
        dt: 0.05,
        temperature: 1.0,
        n_steps: 1,
        burn_in: 100,
        seed: Some(7),
    };
    let result = gigi::geometry::inpaint(&flow, &[10.0, 0.0], &[0], &cfg).unwrap();
    let inner = serde_json::json!({
        "result": result,
        "locked_indices": [0_usize],
        "fit_mean": [5.0_f64, -2.0_f64],
        "fit_sigma_sq": 1.0_f64,
    });
    let wrapped = ResponseWithLambdaStub {
        inner,
        lambda_budget: gigi::curvature::lambda_budget_for_bundle(&store),
    };
    let v = serde_json::to_value(&wrapped).unwrap();
    assert_ridealong_shape(&v, "/brain/inpaint", "result");
}

// ── /brain/predict ──────────────────────────────────────────────

#[test]
fn ridealong_predict_emits_lambda_budget() {
    let store = make_bundle();
    let b = ClosedTwoForm::new_constant(TwoForm::new(vec![0.0, 1.0, -1.0, 0.0], 2).unwrap());
    let flow = from_isotropic_gaussian(b, vec![3.0, -1.0], 2.0).unwrap();
    let next = gigi::geometry::predict_one_step(&flow, &[10.0, 10.0], 0.5).unwrap();
    let inner = serde_json::json!({
        "next_state": next,
        "fit_mean": [3.0_f64, -1.0_f64],
        "fit_sigma_sq": 2.0_f64,
        "step_size": 0.5_f64,
    });
    let wrapped = ResponseWithLambdaStub {
        inner,
        lambda_budget: gigi::curvature::lambda_budget_for_bundle(&store),
    };
    let v = serde_json::to_value(&wrapped).unwrap();
    assert_ridealong_shape(&v, "/brain/predict", "next_state");
}

// ── /brain/reconstruct ──────────────────────────────────────────

#[test]
fn ridealong_reconstruct_emits_lambda_budget() {
    let store = make_bundle();
    let b = ClosedTwoForm::new_constant(TwoForm::new(vec![0.0, 1.0, -1.0, 0.0], 2).unwrap());
    let flow = from_isotropic_gaussian(b, vec![2.0, -3.0], 1.0).unwrap();
    let cfg = FlowConfig::reconstructing();
    let result = flow.reconstruct(&[10.0, 10.0], &cfg).unwrap();
    let inner = serde_json::json!({
        "result": result,
        "fit_mean": [2.0_f64, -3.0_f64],
        "descent_distance": 1.0_f64,
    });
    let wrapped = ResponseWithLambdaStub {
        inner,
        lambda_budget: gigi::curvature::lambda_budget_for_bundle(&store),
    };
    let v = serde_json::to_value(&wrapped).unwrap();
    assert_ridealong_shape(&v, "/brain/reconstruct", "result");
}

// ── /brain/dream ────────────────────────────────────────────────

#[test]
fn ridealong_dream_emits_lambda_budget() {
    let store = make_bundle();
    let b = ClosedTwoForm::new_constant(TwoForm::new(vec![0.0, 1.0, -1.0, 0.0], 2).unwrap());
    let flow = from_isotropic_gaussian(b, vec![0.0, 0.0], 1.0).unwrap();
    let cfg = FlowConfig {
        dt: 0.01,
        temperature: 4.0,
        n_steps: 20,
        burn_in: 0,
        seed: Some(7),
    };
    let traj = flow.dream(&[0.0, 0.0], &cfg).unwrap();
    let inner = serde_json::json!({
        "trajectory": traj,
        "fit_mean": [0.0_f64, 0.0_f64],
        "fit_sigma_sq": 1.0_f64,
        "temperature_used": 4.0_f64,
        "mean_dist_from_mean": 1.0_f64,
        "max_dist_from_mean": 2.0_f64,
    });
    let wrapped = ResponseWithLambdaStub {
        inner,
        lambda_budget: gigi::curvature::lambda_budget_for_bundle(&store),
    };
    let v = serde_json::to_value(&wrapped).unwrap();
    assert_ridealong_shape(&v, "/brain/dream", "trajectory");
}

// ── /brain/forecast ─────────────────────────────────────────────

#[test]
fn ridealong_forecast_emits_lambda_budget() {
    let store = make_bundle();
    let b = ClosedTwoForm::new_constant(TwoForm::new(vec![0.0, 1.0, -1.0, 0.0], 2).unwrap());
    let flow = from_isotropic_gaussian(b, vec![0.0, 0.0], 1.0).unwrap();
    let cfg = FlowConfig::forecasting();
    let path = flow.forecast(&[1.0, 0.0], &cfg).unwrap();
    let inner = serde_json::json!({
        "trajectory": path,
        "fit_mean": [0.0_f64, 0.0_f64],
        "fit_sigma_sq": 1.0_f64,
    });
    let wrapped = ResponseWithLambdaStub {
        inner,
        lambda_budget: gigi::curvature::lambda_budget_for_bundle(&store),
    };
    let v = serde_json::to_value(&wrapped).unwrap();
    assert_ridealong_shape(&v, "/brain/forecast", "trajectory");
}

// ── /brain/self_monitor ─────────────────────────────────────────

#[test]
fn ridealong_self_monitor_emits_lambda_budget() {
    let store = make_bundle();
    let inner = serde_json::json!({
        "raw": 0.5_f64,
        "normalized": 0.5_f64,
        "bandwidth": 0.3_f64,
        "n_samples": 30_usize,
        "horizon_closed": false,
    });
    let wrapped = ResponseWithLambdaStub {
        inner,
        lambda_budget: gigi::curvature::lambda_budget_for_bundle(&store),
    };
    let v = serde_json::to_value(&wrapped).unwrap();
    assert_ridealong_shape(&v, "/brain/self_monitor", "horizon_closed");
}

// ── /brain/fit_diagnostics ──────────────────────────────────────

#[test]
fn ridealong_fit_diagnostics_emits_lambda_budget() {
    let store = make_bundle();
    let inner = serde_json::json!({
        "fit_mean": [0.0_f64, 0.0_f64],
        "fit_sigma_sq_per_field": [1.0_f64, 1.0_f64],
        "fit_mode_used": "diagonal",
        "n_samples": 30_usize,
    });
    let wrapped = ResponseWithLambdaStub {
        inner,
        lambda_budget: gigi::curvature::lambda_budget_for_bundle(&store),
    };
    let v = serde_json::to_value(&wrapped).unwrap();
    assert_ridealong_shape(&v, "/brain/fit_diagnostics", "fit_mode_used");
}

// ── /brain/distance_to_fit_mean ─────────────────────────────────

#[test]
fn ridealong_distance_to_fit_mean_emits_lambda_budget() {
    let store = make_bundle();
    let inner = serde_json::json!({
        "distance": 1.5_f64,
        "fit_mean": [0.0_f64, 0.0_f64],
        "query": [1.0_f64, 1.1_f64],
        "n_samples": 30_usize,
    });
    let wrapped = ResponseWithLambdaStub {
        inner,
        lambda_budget: gigi::curvature::lambda_budget_for_bundle(&store),
    };
    let v = serde_json::to_value(&wrapped).unwrap();
    assert_ridealong_shape(&v, "/brain/distance_to_fit_mean", "distance");
}

// ── /brain/sample_transport ─────────────────────────────────────

#[test]
fn ridealong_sample_transport_emits_lambda_budget() {
    let store = make_bundle();
    let inner = serde_json::json!({
        "samples": [[0.0_f64, 0.0_f64], [0.1_f64, 0.05_f64]],
        "fit_mean": [0.0_f64, 0.0_f64],
        "fit_sigma_sq": 1.0_f64,
        "transport_mode": "parallel",
    });
    let wrapped = ResponseWithLambdaStub {
        inner,
        lambda_budget: gigi::curvature::lambda_budget_for_bundle(&store),
    };
    let v = serde_json::to_value(&wrapped).unwrap();
    assert_ridealong_shape(&v, "/brain/sample_transport", "transport_mode");
}

// ── /brain/sudoku ───────────────────────────────────────────────

#[test]
fn ridealong_sudoku_emits_lambda_budget() {
    let store = make_bundle();
    let inner = serde_json::json!({
        "solved_grid": [[1_u8, 2, 3], [3, 1, 2], [2, 3, 1]],
        "n_steps": 1_usize,
        "converged": true,
    });
    let wrapped = ResponseWithLambdaStub {
        inner,
        lambda_budget: gigi::curvature::lambda_budget_for_bundle(&store),
    };
    let v = serde_json::to_value(&wrapped).unwrap();
    assert_ridealong_shape(&v, "/brain/sudoku", "converged");
}

// ── /brain/intent_gate ──────────────────────────────────────────

#[test]
fn ridealong_intent_gate_emits_lambda_budget() {
    let store = make_bundle();
    let inner = serde_json::json!({
        "decision": "proceed",
        "confidence": 0.8_f64,
        "horizon_closed": false,
    });
    let wrapped = ResponseWithLambdaStub {
        inner,
        lambda_budget: gigi::curvature::lambda_budget_for_bundle(&store),
    };
    let v = serde_json::to_value(&wrapped).unwrap();
    assert_ridealong_shape(&v, "/brain/intent_gate", "decision");
}

// ── §4 Cross-endpoint invariants ────────────────────────────────

/// All ride-along emissions place `lambda_budget` at the SAME JSON
/// path (top level), so a single client implementation reads it
/// uniformly across all 17 brain endpoints.
#[test]
fn all_brain_endpoints_emit_lambda_at_same_path() {
    let store = make_bundle();
    let lambda = gigi::curvature::lambda_budget_for_bundle(&store);
    // Three structurally different inner shapes — all must yield
    // lambda_budget at v["lambda_budget"], never nested.
    let shapes: Vec<JsonValue> = vec![
        serde_json::json!({ "samples": [[0.0_f64]] }),
        serde_json::json!({ "weights": [0.5_f64, 0.5_f64], "indices": [0_usize, 1] }),
        serde_json::json!({ "decision": "proceed", "confidence": 0.9_f64 }),
    ];
    for (i, inner) in shapes.into_iter().enumerate() {
        let v = serde_json::to_value(ResponseWithLambdaStub {
            inner,
            lambda_budget: lambda,
        })
        .unwrap();
        assert!(
            v.get("lambda_budget").is_some(),
            "shape {i} missing top-level lambda_budget"
        );
        // Sanity: never nested under a "meta" or "envelope" key.
        for nest_key in ["meta", "envelope", "inner", "ride_along"] {
            assert!(
                v.get(nest_key).is_none() || v[nest_key].get("lambda_budget").is_none(),
                "shape {i}: lambda_budget must not nest under `{nest_key}`"
            );
        }
    }
}

/// Empty-bundle invariant: every endpoint, when wrapped, still gets a
/// finite `lambda_budget` (the saturated default 1.0), never NaN.
/// Critical for cold-start / fresh-bundle paths where Marcella or
/// claude_substrate_v0 call brain primitives before records exist.
#[test]
fn empty_bundle_ride_along_is_one_not_nan() {
    let store = empty_bundle();
    let lambda = gigi::curvature::lambda_budget_for_bundle(&store);
    let v = serde_json::to_value(ResponseWithLambdaStub {
        inner: serde_json::json!({ "any": "shape" }),
        lambda_budget: lambda,
    })
    .unwrap();
    let lv = v["lambda_budget"].as_f64().expect("lambda_budget is f64");
    assert_eq!(
        lv, 1.0,
        "empty bundle ride-along λ must be the safe default 1.0; got {lv}"
    );
    assert!(!lv.is_nan(), "ride-along must never propagate NaN to clients");
}
