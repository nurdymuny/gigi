//! Marcella dials wave 2, ask 4 — locus + vector-only statistics on the
//! HORIZON / CAPACITY dial surface (GEODESIC_LOOM_PLAN.md ask #4, signed
//! Hallie, 2026-07-16).
//!
//! The census finding this suite turns into a regression fence:
//! `marcella_source_embeddings_bge_v2` has a healthy K (0.0178) but a
//! Welford radius blown to 4.7e6 by one huge-variance ts-like scalar
//! (`ingested_at`), which (a) pollutes `l_c` and therefore `s_max`
//! (1.2e-5 on prod) and (b) saturates the Davis-Conjecture
//! `lambda_budget` at 0.999999 on every prompt, keeping Marcella's
//! Pattern-3 refusal branch dormant (it would refuse everything).
//!
//! New OPT-IN query params on GET /v1/bundles/{b}/horizon and /capacity:
//!
//!   fields=v0..v383 | fields=a,b,c | fields=<vector_field>
//!       statistics over ONLY the named scalar family (wave-1 `..` range
//!       sugar) or one Value::Vector fiber field, per-component.
//!   locus=<field>=<value> [&k=<n>]
//!       statistics over the k-nearest records (cosine chord distance in
//!       the scoped vector space) to the locus record. k default 64.
//!
//! Composition: fields alone (whole bundle, vector-scoped) · locus alone
//! (neighborhood, all numeric scalar fibers) · both (neighborhood +
//! vector-scoped). Precedence: estimator=fixed WINS over locus/fields
//! (the escape hatch) > locus/fields > default whole-bundle.
//!
//! K, l_c, s_max, lambda are recomputed from the scoped statistics
//! THROUGH THE SAME FORMULA FNS the whole-bundle path uses
//! (`curvature::scalar_curvature`, `curvature::horizon_with`,
//! `curvature::capacity`, `curvature::confidence`,
//! `curvature::lambda_budget_for_bundle`, `spectral::spectral_gap`) —
//! applied to a scoped statistics population. The math is not forked.
//!
//! Defaults fence: absent params → byte-identical current behavior.
//! Two belts:
//!   (a) in-process byte fence — `dials::horizon_report` /
//!       `dials::capacity_report` with empty params must serialize
//!       byte-for-byte equal to a replica of the PRE-CHANGE handler
//!       code (copied verbatim from src/bin/gigi_stream.rs @ 6b7a22d);
//!   (b) cross-process goldens — bodies captured over HTTP from the
//!       PRE-CHANGE gigi-stream binary on the same fixture, compared
//!       with exact key order + exact strings and floats to 1e-9
//!       relative (scalar_curvature / welford_radius sum per-field
//!       contributions in HashMap iteration order, which is
//!       process-seeded, so the last ULP of a float can differ across
//!       processes; everything else is exact).

use std::collections::HashMap;

use gigi::curvature::{self, LengthScaleEstimator};
use gigi::dials::{self, CapacityReport, HorizonReport};
use gigi::engine::Engine;
use gigi::parser::{self};
use gigi::spectral;
use gigi::types::{BundleSchema, FieldDef, FieldType, Record, Value};
use gigi::{BundleRef, BundleStore};

// ── fixture plumbing ────────────────────────────────────────────────

/// Execute one GQL statement per non-empty line.
fn run_gql_lines(e: &mut Engine, lines: &str) {
    for line in lines.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let ast = parser::parse(line).unwrap_or_else(|err| panic!("parse {line}: {err}"));
        parser::execute(e, &ast).unwrap_or_else(|err| panic!("execute {line}: {err}"));
    }
}

/// The A4-1 census-reproduction fixture. 64 records:
///   v0..v15      alternating c_j ± 0.05 → per-field K = 0.25 exactly
///                (two-point equal-count distribution), var = 0.0025
///   ingested_at  1e9·(i+1) — the ts-like polluter (values 1e9 apart)
///   temp, wind   benign scalars
///
/// The SAME file is replayed over HTTP against the pre-change binary
/// to produce the goldens in tests/fixtures/dials/golden_*.json.
fn open_dial_fixture() -> (tempfile::TempDir, Engine) {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();
    run_gql_lines(&mut e, include_str!("fixtures/dials/dial_fixture.gql"));
    (dir, e)
}

/// Two well-separated cosine clusters + one benign scalar:
///   A0..A9: (a0, a1) = (1.0, 0.01·i)   — near the +x axis
///   B0..B9: (a0, a1) = (0.01·i, 1.0)   — near the +y axis
///   noise = 100 + i (a scalar fiber that fields=a0..a1 must exclude)
fn locus_cluster_gql(bundle: &str, with_noise: bool) -> String {
    let mut out = String::new();
    if with_noise {
        out.push_str(&format!(
            "BUNDLE {bundle} BASE (id TEXT) FIBER (a0 NUMERIC, a1 NUMERIC, noise NUMERIC);\n"
        ));
    } else {
        out.push_str(&format!(
            "BUNDLE {bundle} BASE (id TEXT) FIBER (a0 NUMERIC, a1 NUMERIC);\n"
        ));
    }
    for i in 0..10 {
        let noise = if with_noise {
            format!(", noise={}.0", 100 + i)
        } else {
            String::new()
        };
        out.push_str(&format!(
            "SECTION {bundle} (id='A{i}', a0=1.000000, a1={:.6}{noise});\n",
            0.01 * i as f64
        ));
    }
    for i in 0..10 {
        let noise = if with_noise {
            format!(", noise={}.0", 110 + i)
        } else {
            String::new()
        };
        out.push_str(&format!(
            "SECTION {bundle} (id='B{i}', a0={:.6}, a1=1.000000{noise});\n",
            0.01 * i as f64
        ));
    }
    out
}

fn qp(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

// ── pre-change replica (the in-process byte fence) ──────────────────
//
// Copied VERBATIM from src/bin/gigi_stream.rs @ 6b7a22d
// (bundle_horizon_report lines 2977-3043, bundle_capacity_report lines
// 2936-2953), including struct field order, format strings and unicode.
// If dials::horizon_report / dials::capacity_report with no scoping
// params ever drift from this replica by a byte, the defaults fence
// broke.

#[derive(serde::Serialize)]
struct PreChangeHorizonReport {
    s_max: f64,
    k: f64,
    tau: f64,
    l_c: f64,
    lambda1: f64,
    estimator_used: LengthScaleEstimator,
    fallback_engaged: bool,
    interpretation: String,
}

fn prechange_horizon_json(store: &BundleRef, params: &HashMap<String, String>) -> String {
    let tau: f64 = params.get("tau").and_then(|s| s.parse().ok()).unwrap_or(1.0);
    let k = store.scalar_curvature();
    let lambda1 = store.as_heap().map(spectral::spectral_gap).unwrap_or(0.0);

    let estimator = match params.get("estimator").map(|s| s.as_str()) {
        Some("welford_radius") => curvature::LengthScaleEstimator::WelfordRadius,
        Some("fixed") => {
            let v: f64 = params
                .get("fixed_value")
                .and_then(|s| s.parse().ok())
                .expect("replica: fixed requires fixed_value");
            curvature::LengthScaleEstimator::Fixed(v)
        }
        Some("spectral_gap") | None => curvature::LengthScaleEstimator::SpectralGap,
        Some(other) => panic!("replica: unknown estimator {other}"),
    };
    let cfg = curvature::HorizonConfig {
        estimator,
        ..curvature::HorizonConfig::default()
    };

    let (s_max, l_c, estimator_used, fallback_engaged) = if let Some(heap) = store.as_heap() {
        let res = curvature::horizon_with(tau, k, heap, lambda1, &cfg);
        (res.s_max, res.l_c, res.estimator_used, res.fallback_engaged)
    } else {
        let l_c_shim = if lambda1 > f64::EPSILON { 1.0 / lambda1.sqrt() } else { 1.0 };
        let s = curvature::horizon(tau, k, lambda1);
        (s, l_c_shim, curvature::LengthScaleEstimator::SpectralGap, lambda1 < f64::EPSILON)
    };

    let interpretation = if s_max.is_infinite() {
        "K ≈ 0: infinite horizon. Flat geometry — all positions remain \
         individually attributable indefinitely.".to_string()
    } else {
        let fallback_note = if fallback_engaged {
            " [fallback estimator engaged; primary was degenerate]"
        } else {
            ""
        };
        format!(
            "s_max = {s_max:.1}: coherent attribution extends {s_max:.0} positions. \
             Beyond this, accumulated frame rotation cannot be decomposed into \
             individual contributions. (K={k:.4}, ℓ_c={l_c:.4}, τ={tau}){fallback_note}"
        )
    };

    serde_json::to_string(&PreChangeHorizonReport {
        s_max, k, tau, l_c, lambda1,
        estimator_used, fallback_engaged, interpretation,
    })
    .unwrap()
}

#[derive(serde::Serialize)]
struct PreChangeCapacityReport {
    capacity: f64,
    k: f64,
    tau: f64,
    confidence: f64,
    regime: &'static str,
    interpretation: String,
}

fn prechange_capacity_json(store: &BundleRef, params: &HashMap<String, String>) -> String {
    let tau: f64 = params.get("tau").and_then(|s| s.parse().ok()).unwrap_or(1.0);
    let k = store.scalar_curvature();
    let c = curvature::capacity(tau, k);
    let conf = curvature::confidence(k);

    let (regime, interpretation) = if k < f64::EPSILON {
        ("flat", format!("K ≈ 0: flat space, infinite capacity. No curvature barriers — every query resolves cleanly."))
    } else if c > 10.0 {
        ("low", format!("C = {c:.2}: low-curvature region. Room for {c:.0} distinct interpretations per unit τ. Synthesis is reliable."))
    } else if c >= 1.0 {
        ("moderate", format!("C = {c:.2}: moderate curvature. The system can hold {c:.1} interpretations simultaneously. Watch for ambiguity."))
    } else if c > 0.1 {
        ("high", format!("C = {c:.3}: high curvature — fewer than one interpretation per unit τ. Ambiguity detection recommended before synthesis."))
    } else {
        ("critical", format!("C = {c:.4}: near-critical curvature. The system cannot reliably distinguish interpretations. Query is at a topological fork."))
    };

    serde_json::to_string(&PreChangeCapacityReport {
        capacity: c, k, tau, confidence: conf, regime, interpretation,
    })
    .unwrap()
}

// ── cross-process golden comparison ─────────────────────────────────
//
// Key sequence + types + strings + bools exact; numbers bit-exact OR
// within 1e-9 relative (cross-process HashMap iteration order shifts
// the per-field summation order inside scalar_curvature /
// welford_radius by a few ULPs).

fn assert_json_matches_golden(built: &str, golden: &str, ctx: &str) {
    let b: serde_json::Value = serde_json::from_str(built).expect("built parses");
    let g: serde_json::Value = serde_json::from_str(golden.trim()).expect("golden parses");
    assert_value_matches(&b, &g, ctx, "$");
}

fn assert_value_matches(b: &serde_json::Value, g: &serde_json::Value, ctx: &str, path: &str) {
    use serde_json::Value as J;
    match (b, g) {
        (J::Object(bo), J::Object(go)) => {
            let bk: Vec<&String> = bo.keys().collect();
            let gk: Vec<&String> = go.keys().collect();
            assert_eq!(bk, gk, "{ctx}: key sequence drift at {path}");
            for (k, gv) in go {
                assert_value_matches(&bo[k], gv, ctx, &format!("{path}.{k}"));
            }
        }
        (J::Array(ba), J::Array(ga)) => {
            assert_eq!(ba.len(), ga.len(), "{ctx}: array length drift at {path}");
            for (i, (bv, gv)) in ba.iter().zip(ga.iter()).enumerate() {
                assert_value_matches(bv, gv, ctx, &format!("{path}[{i}]"));
            }
        }
        (J::Number(bn), J::Number(gn)) => {
            let (x, y) = (bn.as_f64().unwrap(), gn.as_f64().unwrap());
            if x != y {
                let denom = y.abs().max(f64::MIN_POSITIVE);
                assert!(
                    ((x - y).abs() / denom) < 1e-9,
                    "{ctx}: number drift at {path}: built {x} vs golden {y}"
                );
            }
        }
        (a, b) => assert_eq!(a, b, "{ctx}: value drift at {path}"),
    }
}

// ═════════════════════════════════════════════════════════════════════
// A4-1 — CENSUS REPRODUCTION (the load-bearing test)
// ═════════════════════════════════════════════════════════════════════

/// Whole-bundle statistics reproduce Hallie's census pathology: one
/// huge-variance ts-like scalar blows the Welford radius (l_c), which
/// crushes s_max and saturates lambda. Scoping to the vector family
/// v0..v15 lands every dial in the vector geometry's own scale.
///
/// Hand math for the fixture (population formulas):
///   scoped:  K = 0.25, l_c = √0.0025 = 0.05, s_max = 1/(0.25·0.05) = 80,
///            λ = 1 − 1/(0.25·0.05²) = −1599   (desaturated, ≪ 0.95)
///   whole:   var(ingested_at) = 1e18·(64²−1)/12 = 3.4125e20
///            l_c ≈ √(3.4125e20/19) ≈ 4.238e9  (the pollution)
///            s_max ≈ 1.0e-9, λ ≈ 1.0          (saturated)
#[test]
fn a4_1_census_pollution_reproduced_then_cured_by_fields_scope() {
    let (_dir, e) = open_dial_fixture();
    let store = e.bundle("dial_fixture").expect("fixture bundle");

    // ── the pathology, reproduced (whole-bundle, no params) ─────────
    let whole = dials::horizon_report(&store, &qp(&[])).expect("whole-bundle horizon");
    let lambda_before =
        curvature::lambda_budget_for_bundle(store.as_heap().expect("heap fixture"));
    assert!(
        whole.l_c > 1e5,
        "A4-1 pollution: whole-bundle l_c must be huge (census: 4.7e6 on v2); got l_c={}",
        whole.l_c
    );
    assert!(
        whole.s_max < 1e-3,
        "A4-1 pollution: whole-bundle s_max crushed by polluted l_c; got s_max={} (l_c={})",
        whole.s_max,
        whole.l_c
    );
    assert!(
        lambda_before > 0.999,
        "A4-1 pollution: whole-bundle lambda saturated (census: 0.999999 on v2); got {lambda_before}"
    );
    assert_eq!(
        whole.estimator_used,
        LengthScaleEstimator::WelfordRadius,
        "λ₁=0 on a non-graph bundle → Welford fallback (census provenance)"
    );
    assert!(whole.fallback_engaged, "primary spectral_gap estimator is degenerate here");
    assert!(whole.scope.is_none() && whole.lambda_budget.is_none(),
        "no scoping params → no scope echo (defaults fence)");

    // ── the cure: fields=v0..v15 (wave-1 range sugar) ───────────────
    let scoped =
        dials::horizon_report(&store, &qp(&[("fields", "v0..v15")])).expect("scoped horizon");

    assert!(
        (scoped.k - 0.25).abs() < 1e-9,
        "A4-1 scoped K: hand value 0.25 (two-point per-field distributions); got {} \
         [polluted whole-bundle: l_c={}, s_max={}, λ={}]",
        scoped.k, whole.l_c, whole.s_max, lambda_before
    );
    assert!(
        (scoped.l_c - 0.05).abs() < 1e-9,
        "A4-1 scoped l_c: hand value √0.0025 = 0.05 (the vector geometry's own scale); got {} \
         [polluted whole-bundle l_c={}]",
        scoped.l_c, whole.l_c
    );
    assert!(
        (scoped.s_max - 80.0).abs() < 1e-6,
        "A4-1 scoped s_max: hand value 1/(0.25·0.05) = 80; got {} \
         [polluted whole-bundle s_max={}]",
        scoped.s_max, whole.s_max
    );

    let lambda_after = scoped
        .lambda_budget
        .expect("scoped response carries the recomputed lambda_budget");
    assert!(
        lambda_after < 0.95,
        "A4-1 desaturation: scoped λ must clear the 0.95 refusal threshold; got {lambda_after} \
         [saturated whole-bundle λ={lambda_before}]"
    );
    assert!(
        (lambda_after - (-1599.0)).abs() < 1e-5,
        "A4-1 scoped λ: hand value 1 − 1/(0.25·0.0025) = −1599; got {lambda_after}"
    );

    // Scope echo names the scope (D2 probe contract).
    let echo = scoped.scope.expect("scoped response echoes its scope");
    assert_eq!(echo.fields.as_deref(), Some("v0..v15"));
    assert_eq!(echo.n_fields, 16, "16 per-component statistics in scope");
    assert_eq!(echo.n_records, 64, "whole-bundle population");
    assert!(echo.locus.is_none());

    // Print the four report numbers for the wave-2 ship report.
    println!(
        "A4-1 numbers: polluted_lc={} scoped_lc={} lambda_before={} lambda_after={}",
        whole.l_c, scoped.l_c, lambda_before, lambda_after
    );
}

/// CAPACITY with fields= recomputes C and confidence from the scoped K
/// through the same public fns (capacity = τ/K, confidence = 1/(1+K)).
#[test]
fn a4_1_capacity_scoped_through_same_formulas() {
    let (_dir, e) = open_dial_fixture();
    let store = e.bundle("dial_fixture").expect("fixture bundle");

    let scoped =
        dials::capacity_report(&store, &qp(&[("fields", "v0..v15")])).expect("scoped capacity");
    assert!(
        (scoped.k - 0.25).abs() < 1e-9,
        "scoped K = 0.25 by hand; got {}",
        scoped.k
    );
    assert!(
        (scoped.capacity - 4.0).abs() < 1e-7,
        "scoped C = τ/K = 1/0.25 = 4.0; got {}",
        scoped.capacity
    );
    assert!(
        (scoped.confidence - 0.8).abs() < 1e-9,
        "scoped confidence = 1/(1+0.25) = 0.8; got {}",
        scoped.confidence
    );
    assert_eq!(scoped.regime, "moderate", "C = 4.0 lands in the moderate band");
    assert!(scoped.lambda_budget.is_some(), "scoped capacity carries λ too");
    let echo = scoped.scope.expect("scope echo");
    assert_eq!(echo.n_fields, 16);
    assert_eq!(echo.n_records, 64);
}

/// estimator=welford_radius composes with fields=: the estimator param
/// picks the primary estimator ON the scoped population (no fallback
/// engaged, same scoped l_c).
#[test]
fn a4_1_estimator_welford_composes_with_fields_scope() {
    let (_dir, e) = open_dial_fixture();
    let store = e.bundle("dial_fixture").expect("fixture bundle");

    let scoped = dials::horizon_report(
        &store,
        &qp(&[("fields", "v0..v15"), ("estimator", "welford_radius")]),
    )
    .expect("scoped horizon, welford primary");
    assert_eq!(scoped.estimator_used, LengthScaleEstimator::WelfordRadius);
    assert!(!scoped.fallback_engaged, "welford as PRIMARY → no fallback");
    assert!((scoped.l_c - 0.05).abs() < 1e-9);
    assert!((scoped.s_max - 80.0).abs() < 1e-6);
}

/// A true Value::Vector fiber field scopes per-component: fields=emb on
/// a dims=4 vector fiber behaves like a 4-scalar family.
/// Hand math: components each take values {0.1+0.1i} shifted — per
/// component var = var{x, x+0.1, x+0.2} = 0.1²·var{0,1,2} = 0.01·(2/3),
/// range = 0.2 → K_comp = (0.01·2/3)/0.04 = 1/6; l_c = √(0.02/3).
#[test]
fn a4_1_vector_fiber_field_scopes_per_component() {
    let mut schema = BundleSchema::new("emb_fixture")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::numeric("ts"));
    let mut emb = FieldDef::numeric("emb");
    emb.field_type = FieldType::Vector { dims: 4 };
    schema = schema.fiber(emb);
    let mut store = BundleStore::new(schema);
    for i in 0..3 {
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(i));
        r.insert(
            "emb".into(),
            Value::Vector(vec![
                0.1 + 0.1 * i as f64,
                0.2 + 0.1 * i as f64,
                0.3 + 0.1 * i as f64,
                0.4 + 0.1 * i as f64,
            ]),
        );
        // ts-like polluter so the scope has something to exclude
        r.insert("ts".into(), Value::Float(1.0e9 * (i + 1) as f64));
        store.insert(&r);
    }
    let bref = BundleRef::Heap(&store);

    let scoped = dials::horizon_report(&bref, &qp(&[("fields", "emb")])).expect("vector scope");
    let expect_k = 1.0 / 6.0;
    let expect_lc = (0.02_f64 / 3.0).sqrt();
    assert!(
        (scoped.k - expect_k).abs() < 1e-9,
        "per-component K = 1/6; got {}",
        scoped.k
    );
    assert!(
        (scoped.l_c - expect_lc).abs() < 1e-9,
        "per-component l_c = √(0.02/3) = {expect_lc}; got {}",
        scoped.l_c
    );
    let echo = scoped.scope.expect("scope echo");
    assert_eq!(echo.n_fields, 4, "dims=4 vector → 4 per-component stats");
    assert_eq!(echo.n_records, 3);
}

// ═════════════════════════════════════════════════════════════════════
// A4-2 — DEFAULTS FENCE (byte-identical current behavior)
// ═════════════════════════════════════════════════════════════════════

/// In-process byte fence: no params → dials::horizon_report serializes
/// byte-for-byte identical to the pre-change handler replica.
#[test]
fn a4_2_defaults_fence_horizon_byte_identical_in_process() {
    let (_dir, e) = open_dial_fixture();
    let store = e.bundle("dial_fixture").expect("fixture bundle");
    let params = qp(&[]);

    let built = serde_json::to_string(
        &dials::horizon_report(&store, &params).expect("default horizon"),
    )
    .unwrap();
    let replica = prechange_horizon_json(&store, &params);
    assert_eq!(built, replica, "A4-2: default horizon body drifted from pre-change bytes");
}

/// Same fence for capacity.
#[test]
fn a4_2_defaults_fence_capacity_byte_identical_in_process() {
    let (_dir, e) = open_dial_fixture();
    let store = e.bundle("dial_fixture").expect("fixture bundle");
    let params = qp(&[]);

    let built = serde_json::to_string(
        &dials::capacity_report(&store, &params).expect("default capacity"),
    )
    .unwrap();
    let replica = prechange_capacity_json(&store, &params);
    assert_eq!(built, replica, "A4-2: default capacity body drifted from pre-change bytes");
}

/// tau alone (a pre-existing param) must stay on the default path —
/// only the NEW params may trigger scoping.
#[test]
fn a4_2_tau_only_stays_on_default_path() {
    let (_dir, e) = open_dial_fixture();
    let store = e.bundle("dial_fixture").expect("fixture bundle");
    let params = qp(&[("tau", "2.0")]);

    let built_h =
        serde_json::to_string(&dials::horizon_report(&store, &params).unwrap()).unwrap();
    assert_eq!(built_h, prechange_horizon_json(&store, &params));

    let built_c =
        serde_json::to_string(&dials::capacity_report(&store, &params).unwrap()).unwrap();
    assert_eq!(built_c, prechange_capacity_json(&store, &params));
}

/// Cross-process golden: the default horizon body captured over HTTP
/// from the PRE-CHANGE gigi-stream binary on this exact fixture.
#[test]
fn a4_2_defaults_fence_horizon_matches_prechange_binary_golden() {
    let (_dir, e) = open_dial_fixture();
    let store = e.bundle("dial_fixture").expect("fixture bundle");
    let built =
        serde_json::to_string(&dials::horizon_report(&store, &qp(&[])).unwrap()).unwrap();
    assert_json_matches_golden(
        &built,
        include_str!("fixtures/dials/golden_horizon_default.json"),
        "horizon default vs pre-change binary",
    );
}

/// Cross-process golden for capacity.
#[test]
fn a4_2_defaults_fence_capacity_matches_prechange_binary_golden() {
    let (_dir, e) = open_dial_fixture();
    let store = e.bundle("dial_fixture").expect("fixture bundle");
    let built =
        serde_json::to_string(&dials::capacity_report(&store, &qp(&[])).unwrap()).unwrap();
    assert_json_matches_golden(
        &built,
        include_str!("fixtures/dials/golden_capacity_default.json"),
        "capacity default vs pre-change binary",
    );
}

/// The estimator=fixed escape hatch is fenced too (it must keep working
/// EXACTLY as today — it is Marcella's production pin).
#[test]
fn a4_2_fixed_escape_hatch_matches_prechange_binary_golden() {
    let (_dir, e) = open_dial_fixture();
    let store = e.bundle("dial_fixture").expect("fixture bundle");
    let params = qp(&[("estimator", "fixed"), ("fixed_value", "1.0")]);
    let built =
        serde_json::to_string(&dials::horizon_report(&store, &params).unwrap()).unwrap();
    assert_eq!(built, prechange_horizon_json(&store, &params), "in-process fixed fence");
    assert_json_matches_golden(
        &built,
        include_str!("fixtures/dials/golden_horizon_fixed.json"),
        "horizon fixed vs pre-change binary",
    );
}

// ═════════════════════════════════════════════════════════════════════
// A4-3 — LOCUS CORRECTNESS (two well-separated clusters)
// ═════════════════════════════════════════════════════════════════════

/// locus at a cluster-A record with small k → statistics reflect
/// cluster A ONLY, not the global mixture.
///
/// Hand math over cluster A = {(1.0, 0.01·i)}, i = 0..9:
///   a0: constant 1.0 → var 0 → contributes 0 to K, skipped by l_c
///   a1: var = 1e-4·var{0..9} = 8.25e-4, range 0.09
///       → K_a1 = 8.25e-4/0.0081 = 0.101852
///   K_A = (0 + 0.101852)/2 = 0.050926,  l_c_A = √8.25e-4 = 0.028723
/// Global mixture (all 20 records): per-field var ≈ 0.2284, range 1.0
///   → K_mix ≈ 0.23, l_c_mix ≈ 0.478 — an order of magnitude apart.
#[test]
fn a4_3_locus_scopes_statistics_to_cluster_a() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();
    run_gql_lines(&mut e, &locus_cluster_gql("locus_fixture", true));
    let store = e.bundle("locus_fixture").expect("fixture bundle");

    // Mixture baseline (fields-scoped, whole bundle).
    let mixture =
        dials::horizon_report(&store, &qp(&[("fields", "a0..a1")])).expect("mixture horizon");
    assert!(
        mixture.k > 0.2,
        "mixture K over both clusters ≈ 0.23; got {}",
        mixture.k
    );
    assert!(
        mixture.l_c > 0.4,
        "mixture l_c ≈ 0.478; got {}",
        mixture.l_c
    );

    // Locus at A3, k=10 → exactly cluster A (cosine chord distance:
    // within-A d² ≤ 0.002, cross-cluster d² ≥ 0.44).
    let scoped = dials::horizon_report(
        &store,
        &qp(&[("fields", "a0..a1"), ("locus", "id=A3"), ("k", "10")]),
    )
    .expect("locus horizon");

    assert!(
        (scoped.k - 0.050925925925925).abs() < 1e-6,
        "A4-3: neighborhood K must be cluster-A's hand value 0.050926 (mixture: {}); got {}",
        mixture.k,
        scoped.k
    );
    assert!(
        (scoped.l_c - 0.028722813232690143).abs() < 1e-9,
        "A4-3: neighborhood l_c must be cluster-A's hand value √8.25e-4 (mixture: {}); got {}",
        mixture.l_c,
        scoped.l_c
    );

    let echo = scoped.scope.expect("scope echo");
    assert_eq!(echo.n_records, 10, "k=10 neighborhood");
    assert_eq!(echo.n_fields, 2, "vector-scoped to a0..a1 (noise excluded)");
    let locus = echo.locus.expect("locus echo");
    assert_eq!(locus.field, "id");
    assert_eq!(locus.value, "A3");
    assert_eq!(locus.k, 10);
}

/// locus alone (no fields) scopes the statistics to the neighborhood
/// over ALL numeric scalar fibers — the same field population the
/// whole-bundle formulas see, restricted to the k nearest records.
#[test]
fn a4_3_locus_alone_uses_all_numeric_fibers() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();
    run_gql_lines(&mut e, &locus_cluster_gql("locus_fixture2", false));
    let store = e.bundle("locus_fixture2").expect("fixture bundle");

    let scoped = dials::horizon_report(&store, &qp(&[("locus", "id=A3"), ("k", "10")]))
        .expect("locus-alone horizon");
    // Same cluster-A hand values as above (the bundle has only a0, a1).
    assert!(
        (scoped.k - 0.050925925925925).abs() < 1e-6,
        "locus-alone K over cluster A; got {}",
        scoped.k
    );
    assert!(
        (scoped.l_c - 0.028722813232690143).abs() < 1e-9,
        "locus-alone l_c over cluster A; got {}",
        scoped.l_c
    );
    let echo = scoped.scope.expect("scope echo");
    assert_eq!(echo.fields, None, "no fields param → no fields echo");
    assert_eq!(echo.n_fields, 2, "all numeric scalar fibers");
    assert_eq!(echo.n_records, 10);
}

/// k larger than the bundle takes everything: locus+fields with k=20
/// equals the fields-only report bit-for-bit (same population, same
/// formulas).
#[test]
fn a4_3_locus_with_k_covering_bundle_equals_fields_only() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();
    run_gql_lines(&mut e, &locus_cluster_gql("locus_fixture3", true));
    let store = e.bundle("locus_fixture3").expect("fixture bundle");

    let fields_only =
        dials::horizon_report(&store, &qp(&[("fields", "a0..a1")])).expect("fields-only");
    let covering = dials::horizon_report(
        &store,
        &qp(&[("fields", "a0..a1"), ("locus", "id=A0"), ("k", "20")]),
    )
    .expect("covering locus");
    assert_eq!(covering.k.to_bits(), fields_only.k.to_bits(), "same population → same K bits");
    assert_eq!(covering.l_c.to_bits(), fields_only.l_c.to_bits());
    assert_eq!(covering.s_max.to_bits(), fields_only.s_max.to_bits());
    assert_eq!(covering.scope.as_ref().unwrap().n_records, 20);
}

/// The documented default k is 64 (echoed even when not supplied).
#[test]
fn a4_3_locus_default_k_is_64() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();
    run_gql_lines(&mut e, &locus_cluster_gql("locus_fixture4", false));
    let store = e.bundle("locus_fixture4").expect("fixture bundle");

    assert_eq!(dials::DEFAULT_LOCUS_K, 64, "documented default");
    let scoped =
        dials::horizon_report(&store, &qp(&[("locus", "id=A0")])).expect("locus default k");
    let echo = scoped.scope.expect("scope echo");
    assert_eq!(echo.locus.as_ref().unwrap().k, 64, "default k echoed");
    assert_eq!(echo.n_records, 20, "bundle smaller than k → whole bundle");
}

// ═════════════════════════════════════════════════════════════════════
// A4-4 — FIXED PRECEDENCE (the escape hatch wins over everything)
// ═════════════════════════════════════════════════════════════════════

/// estimator=fixed + fields + locus → fixed_value wins; the response is
/// byte-identical to estimator=fixed alone (scoping params ignored, no
/// scope echo — the escape hatch's wire shape is untouched).
#[test]
fn a4_4_fixed_precedence_over_locus_and_fields() {
    let (_dir, e) = open_dial_fixture();
    let store = e.bundle("dial_fixture").expect("fixture bundle");

    let fixed_alone = serde_json::to_string(
        &dials::horizon_report(&store, &qp(&[("estimator", "fixed"), ("fixed_value", "1.0")]))
            .expect("fixed alone"),
    )
    .unwrap();
    let fixed_with_scope = serde_json::to_string(
        &dials::horizon_report(
            &store,
            &qp(&[
                ("estimator", "fixed"),
                ("fixed_value", "1.0"),
                ("fields", "v0..v15"),
                ("locus", "id=r3"),
                ("k", "8"),
            ]),
        )
        .expect("fixed + scope params"),
    )
    .unwrap();
    assert_eq!(
        fixed_with_scope, fixed_alone,
        "A4-4: precedence is fixed > locus/fields > default — fixed must win byte-for-byte"
    );

    // And the fixed report reads the whole-bundle K with l_c pinned to 1.0.
    let parsed: serde_json::Value = serde_json::from_str(&fixed_alone).unwrap();
    assert_eq!(parsed["l_c"].as_f64().unwrap(), 1.0);
    assert!(parsed.get("scope").is_none(), "escape hatch carries no scope echo");
}

// ═════════════════════════════════════════════════════════════════════
// Typed error contract (wave-1 style: typos are loud, names named)
// ═════════════════════════════════════════════════════════════════════

fn err_of<T>(r: Result<T, dials::DialError>) -> dials::DialError {
    match r {
        Ok(_) => panic!("expected a DialError"),
        Err(e) => e,
    }
}

#[test]
fn a4_err_unknown_field_in_fields_is_loud() {
    let (_dir, e) = open_dial_fixture();
    let store = e.bundle("dial_fixture").expect("fixture bundle");
    let err = err_of(dials::horizon_report(&store, &qp(&[("fields", "v0..v99")])));
    match err {
        dials::DialError::BadRequest(msg) => {
            assert!(msg.contains("v16"), "names the first missing field: {msg}")
        }
        other => panic!("expected BadRequest, got {other:?}"),
    }
}

#[test]
fn a4_err_non_numeric_field_is_loud() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();
    run_gql_lines(
        &mut e,
        "BUNDLE typed BASE (id TEXT) FIBER (x NUMERIC, label TEXT);\n\
         SECTION typed (id='a', x=1.0, label='blue');\n\
         SECTION typed (id='b', x=2.0, label='red');",
    );
    let store = e.bundle("typed").expect("fixture bundle");
    let err = err_of(dials::horizon_report(&store, &qp(&[("fields", "x,label")])));
    match err {
        dials::DialError::BadRequest(msg) => {
            assert!(msg.contains("label"), "names the guilty field: {msg}");
            assert!(msg.contains("not numeric"), "says why: {msg}");
        }
        other => panic!("expected BadRequest, got {other:?}"),
    }
}

#[test]
fn a4_err_locus_record_missing_is_404_class_naming_the_key() {
    let (_dir, e) = open_dial_fixture();
    let store = e.bundle("dial_fixture").expect("fixture bundle");
    let err = err_of(dials::horizon_report(
        &store,
        &qp(&[("locus", "id=ghost"), ("fields", "v0..v15")]),
    ));
    match err {
        dials::DialError::NotFound(msg) => {
            assert!(msg.contains("id"), "names the field: {msg}");
            assert!(msg.contains("ghost"), "names the missing key value: {msg}");
        }
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[test]
fn a4_err_locus_unknown_field_is_loud() {
    let (_dir, e) = open_dial_fixture();
    let store = e.bundle("dial_fixture").expect("fixture bundle");
    let err = err_of(dials::horizon_report(&store, &qp(&[("locus", "nope=r3")])));
    match err {
        dials::DialError::BadRequest(msg) => {
            assert!(msg.contains("nope"), "names the unknown locus field: {msg}")
        }
        other => panic!("expected BadRequest, got {other:?}"),
    }
}

#[test]
fn a4_err_locus_malformed_is_loud() {
    let (_dir, e) = open_dial_fixture();
    let store = e.bundle("dial_fixture").expect("fixture bundle");
    let err = err_of(dials::horizon_report(&store, &qp(&[("locus", "justavalue")])));
    assert!(matches!(err, dials::DialError::BadRequest(_)), "locus without '=' is a 400");
}

#[test]
fn a4_err_k_without_locus_is_loud() {
    let (_dir, e) = open_dial_fixture();
    let store = e.bundle("dial_fixture").expect("fixture bundle");
    let err = err_of(dials::horizon_report(&store, &qp(&[("k", "8"), ("fields", "v0..v15")])));
    match err {
        dials::DialError::BadRequest(msg) => {
            assert!(msg.contains("locus"), "k is only meaningful with locus: {msg}")
        }
        other => panic!("expected BadRequest, got {other:?}"),
    }
}

#[test]
fn a4_err_k_zero_is_loud() {
    let (_dir, e) = open_dial_fixture();
    let store = e.bundle("dial_fixture").expect("fixture bundle");
    let err = err_of(dials::horizon_report(&store, &qp(&[("locus", "id=r3"), ("k", "0")])));
    assert!(matches!(err, dials::DialError::BadRequest(_)), "k=0 is a 400");
}

#[test]
fn a4_err_empty_fields_is_loud() {
    let (_dir, e) = open_dial_fixture();
    let store = e.bundle("dial_fixture").expect("fixture bundle");
    let err = err_of(dials::horizon_report(&store, &qp(&[("fields", "")])));
    assert!(matches!(err, dials::DialError::BadRequest(_)), "empty fields is a 400");
}

#[test]
fn a4_err_vector_field_in_multi_field_list_is_loud() {
    let mut schema = BundleSchema::new("mixed")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::numeric("x"));
    let mut emb = FieldDef::numeric("emb");
    emb.field_type = FieldType::Vector { dims: 2 };
    schema = schema.fiber(emb);
    let mut store = BundleStore::new(schema);
    for i in 0..3 {
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(i));
        r.insert("x".into(), Value::Float(i as f64));
        r.insert("emb".into(), Value::Vector(vec![i as f64, 1.0]));
        store.insert(&r);
    }
    let bref = BundleRef::Heap(&store);
    let err = err_of(dials::horizon_report(&bref, &qp(&[("fields", "x,emb")])));
    match err {
        dials::DialError::BadRequest(msg) => assert!(
            msg.contains("emb"),
            "a Vector field only scopes alone (fields=<vector_field>): {msg}"
        ),
        other => panic!("expected BadRequest, got {other:?}"),
    }
}

/// Same param validation applies on the capacity dial.
#[test]
fn a4_err_capacity_shares_the_validation() {
    let (_dir, e) = open_dial_fixture();
    let store = e.bundle("dial_fixture").expect("fixture bundle");
    let err = err_of(dials::capacity_report(&store, &qp(&[("fields", "v0..v99")])));
    assert!(matches!(err, dials::DialError::BadRequest(_)));
    let err = err_of(dials::capacity_report(&store, &qp(&[("locus", "id=ghost")])));
    assert!(matches!(err, dials::DialError::NotFound(_)));
}
