//! Cross-team interface contract test for the Cognitive Geometry
//! HTTP endpoints (Branch VII, Davis 2026):
//!
//!   GET /v1/bundles/{name}/capacity[?tau=N]
//!   GET /v1/bundles/{name}/horizon[?tau=N&estimator=…&fixed_value=N]
//!   GET /v1/bundles/{name}/depth[?k_metric=…&k_connection=…
//!                                 &lambda1_topological=…&lambda1_connection=…]
//!
//! Pattern follows kahler_curvature_marcella_contract.rs +
//! kahler_brain_endpoints_contract.rs: rather than spin up axum and
//! deserialize, we pin the public Rust types that the response structs
//! serialize FROM. The wire JSON is derived 1:1 by `serde::Serialize`
//! from these types, so any rename/retype on the Rust side fails
//! compilation here BEFORE Marcella's or any other consumer's
//! deserialization can drift.
//!
//! ### Coverage matrix
//!
//! | Endpoint  | Backing Rust API                           | Wire-relevant type        |
//! |-----------|--------------------------------------------|---------------------------|
//! | capacity  | `curvature::capacity(τ, K) -> f64`         | (scalars only)            |
//! | horizon   | `curvature::horizon_with(...)` →           | `HorizonResult`           |
//! |           |   `HorizonResult { s_max, l_c,             | `LengthScaleEstimator`    |
//! |           |     estimator_used, fallback_engaged }`    |                           |
//! | depth     | `curvature::encoding_depth_with(K, λ₁,&c)` | `EncodingDepth`           |
//! |           |   → `EncodingDepth` + `DepthConfig`        | `DepthConfig`             |
//!
//! We also assert the serde rename rules that the HTTP JSON depends
//! on (LengthScaleEstimator → snake_case, EncodingDepth → lowercase).

#![cfg(feature = "kahler")]

use gigi::curvature::{
    capacity, encoding_depth, encoding_depth_with, horizon, horizon_with, local_holonomy,
    perceive, perception_bias, scalar_curvature, DepthConfig, EncodingDepth, HorizonConfig,
    HorizonResult, LengthScaleEstimator, LocalHolonomyResult, PerceiveError, PerceptionResult,
};
use gigi::spectral::spectral_gap;
use gigi::types::{BundleSchema, FieldDef, Record, Value};
use gigi::BundleStore;

/// Build a synthetic bundle with enough geometric structure for K and
/// λ₁ to be well-defined (≥ 2 records, ≥ 1 numeric fiber field with
/// real variance, ≥ 1 categorical index for the graph Laplacian).
fn contract_bundle() -> BundleStore {
    let schema = BundleSchema::new("cg_endpoints_contract")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::numeric("x").with_range(5.0))
        .fiber(FieldDef::numeric("y").with_range(5.0))
        .fiber(FieldDef::categorical("cat"))
        .index("cat");
    let mut store = BundleStore::new(schema);
    for i in 0..30 {
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(i));
        r.insert("x".into(), Value::Float(i as f64 * 0.1));
        r.insert("y".into(), Value::Float((i as f64 * 0.2).sin()));
        r.insert(
            "cat".into(),
            Value::Text(if i % 2 == 0 { "A".into() } else { "B".into() }),
        );
        store.insert(&r);
    }
    store
}

// ── CAPACITY contract ──────────────────────────────────────────────

/// CapacityReport serializes `capacity: f64` directly from
/// `curvature::capacity(τ, K)`. The wire field `capacity` is a plain
/// f64, no enum encoding. Pin the signature so a future change can't
/// silently change the wire type (e.g. `f64 → Option<f64>` would
/// break Marcella's `let cap: f64 = …`).
#[test]
fn capacity_api_returns_f64() {
    let store = contract_bundle();
    let k = scalar_curvature(&store);
    let _: f64 = capacity(1.0, k);
    // Linearity in τ: doubling τ doubles C. The wire layer depends on
    // this being a pure function of (τ, K) — no hidden state.
    let c1 = capacity(1.0, k.max(1e-9));
    let c2 = capacity(2.0, k.max(1e-9));
    assert!(
        (c2 / c1 - 2.0).abs() < 1e-9,
        "capacity must be linear in τ for the HTTP report to scale right"
    );
}

// ── HORIZON contract ───────────────────────────────────────────────

/// HorizonReport's wire JSON has the shape:
///   { s_max, k, tau, l_c, lambda1,
///     estimator_used: "spectral_gap"|"welford_radius"|{"fixed":N},
///     fallback_engaged: bool,
///     interpretation: String }
///
/// The k/tau/lambda1/interpretation fields are computed in the
/// endpoint itself; the remaining four come from HorizonResult via
/// `horizon_with`. Pin the HorizonResult field set + types so the
/// wire shape can't drift silently.
#[test]
fn horizon_result_has_expected_field_set() {
    let store = contract_bundle();
    let k = scalar_curvature(&store);
    let lambda1 = spectral_gap(&store);
    let res: HorizonResult = horizon_with(1.0, k, &store, lambda1, &HorizonConfig::default());

    // Compile-time pinning of every wire-relevant field. Marcella's
    // deserializer relies on these names + types.
    let _: f64 = res.s_max;
    let _: f64 = res.l_c;
    let _: LengthScaleEstimator = res.estimator_used;
    let _: bool = res.fallback_engaged;
}

/// HorizonConfig is what the HTTP query params build up before calling
/// `horizon_with`. Pin its field set + types so the wire surface can't
/// drift. (HorizonConfig is itself Serialize/Deserialize too, in case
/// a future endpoint version chooses to echo `config_used` like DEPTH.)
#[test]
fn horizon_config_has_expected_field_set() {
    let cfg = HorizonConfig::default();
    let _: LengthScaleEstimator = cfg.estimator;
    let _: LengthScaleEstimator = cfg.fallback;
    let _: f64 = cfg.epsilon;

    // The default is the JTBD-flagged "SpectralGap with Welford
    // fallback" pair. Pin it — a default change is a wire-visible
    // behavior change for callers who don't supply ?estimator.
    assert_eq!(cfg.estimator, LengthScaleEstimator::SpectralGap);
    assert_eq!(cfg.fallback, LengthScaleEstimator::WelfordRadius);
}

/// LengthScaleEstimator serializes to snake_case strings (with
/// `Fixed(v)` becoming `{"fixed": v}` per serde's default for
/// tuple-variants). The HTTP query-param parser also accepts the
/// snake_case names. Pin both directions so the wire string and the
/// query-param string stay in lock-step.
#[test]
fn length_scale_estimator_serializes_to_snake_case() {
    let cases = [
        (LengthScaleEstimator::SpectralGap, "\"spectral_gap\""),
        (LengthScaleEstimator::WelfordRadius, "\"welford_radius\""),
    ];
    for (variant, expected) in cases {
        let serialized = serde_json::to_string(&variant).expect("serialize");
        assert_eq!(
            serialized, expected,
            "{:?} must serialize as {} for the wire contract",
            variant, expected
        );
    }
    // Tuple variant: serde_json default is {"fixed": v}.
    let fixed = LengthScaleEstimator::Fixed(42.0);
    let s = serde_json::to_string(&fixed).expect("serialize");
    assert_eq!(s, "{\"fixed\":42.0}");

    // Round-trip: the parser path on the HTTP side accepts the same
    // snake_case names back. Pin that here too via serde.
    let back: LengthScaleEstimator =
        serde_json::from_str("\"welford_radius\"").expect("deserialize");
    assert_eq!(back, LengthScaleEstimator::WelfordRadius);
}

/// The scalar shim `horizon(τ, K, λ₁) -> f64` (used by the endpoint's
/// overlay-only fallback path when no heap store is available) keeps
/// the public surface backward-compat. Pin the signature.
#[test]
fn horizon_scalar_shim_returns_f64() {
    let _: f64 = horizon(1.0, 0.5, 0.25);
}

// ── DEPTH contract ─────────────────────────────────────────────────

/// DepthReport's wire JSON has the shape:
///   { depth: "tangent"|"connection"|"metric"|"topological",
///     level: "I"|"II"|"III"|"IV",
///     k, lambda1, erasure_energy, description,
///     config_used: DepthConfig }
///
/// The depth + level + description fields come straight off the
/// EncodingDepth enum. Pin the enum + its label()/description()
/// accessors so the wire labels can't shift.
#[test]
fn encoding_depth_has_expected_label_set() {
    use EncodingDepth::*;
    let cases: [(EncodingDepth, &str); 4] = [
        (Tangent, "I"),
        (Connection, "II"),
        (Metric, "III"),
        (Topological, "IV"),
    ];
    for (variant, expected_label) in cases {
        assert_eq!(
            variant.label(),
            expected_label,
            "EncodingDepth::{:?}.label() must be {}",
            variant,
            expected_label
        );
        // Description must be non-empty — the wire field is a static
        // string that callers display verbatim.
        assert!(!variant.description().is_empty());
    }
}

/// EncodingDepth serializes to lowercase variant names. This is the
/// `depth` field on DepthReport — Marcella's deserializer relies on
/// the exact strings "tangent" / "connection" / "metric" / "topological".
#[test]
fn encoding_depth_serializes_to_lowercase() {
    let cases = [
        (EncodingDepth::Tangent, "\"tangent\""),
        (EncodingDepth::Connection, "\"connection\""),
        (EncodingDepth::Metric, "\"metric\""),
        (EncodingDepth::Topological, "\"topological\""),
    ];
    for (variant, expected) in cases {
        let s = serde_json::to_string(&variant).expect("serialize");
        assert_eq!(s, expected, "{:?} → {}", variant, expected);
        let back: EncodingDepth = serde_json::from_str(expected).expect("deserialize");
        assert_eq!(back, variant, "round-trip: {} → {:?}", expected, variant);
    }
}

/// DepthConfig is echoed back on the wire as `config_used`. Pin the
/// field set + types so a future field add/rename can't silently break
/// the audit surface.
#[test]
fn depth_config_has_expected_field_set() {
    let c = DepthConfig::default();
    let _: f64 = c.lambda1_topological;
    let _: f64 = c.k_metric;
    let _: f64 = c.k_connection;
    let _: f64 = c.lambda1_connection;

    // The default values are what the wire returns when no overrides
    // are supplied; pin them so a default change is visible at code-
    // review time. Default == for_graph_substrate() (Theorem 8.14).
    assert_eq!(c, DepthConfig::for_graph_substrate());
    assert_eq!(c.lambda1_topological, 0.01);
    assert_eq!(c.k_metric, 0.5);
    assert_eq!(c.k_connection, 0.1);
    assert_eq!(c.lambda1_connection, 0.3);
}

/// `for_continuous_substrate()` is what `auto_for(store, eps)` returns
/// on the sensor JTBD case. Pin its wire values — the HTTP endpoint
/// echoes this struct back so callers can see at-a-glance which
/// substrate-type defaults were used.
#[test]
fn depth_config_continuous_substrate_wire_values_pinned() {
    let c = DepthConfig::for_continuous_substrate();
    assert_eq!(c.lambda1_topological, 0.0);
    assert_eq!(c.lambda1_connection, 0.0);
    // K cuts unchanged from graph-substrate; pinned in case a future
    // refactor decides to specialize them too.
    assert_eq!(c.k_metric, 0.5);
    assert_eq!(c.k_connection, 0.1);
}

/// DepthConfig round-trips through JSON the same way the wire serializes
/// it. The endpoint's `config_used` field is the audit trail callers
/// see in the response; if the round-trip ever changes shape, the audit
/// stops being meaningful.
#[test]
fn depth_config_roundtrips_through_json() {
    let c = DepthConfig {
        lambda1_topological: -1.0,
        k_metric: 0.7,
        k_connection: 0.15,
        lambda1_connection: 0.42,
    };
    let json = serde_json::to_string(&c).expect("serialize");
    let back: DepthConfig = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back, c);
}

/// `encoding_depth(K, λ₁)` (the zero-arg shim) must be bit-identical to
/// `encoding_depth_with(K, λ₁, &DepthConfig::default())`. The HTTP
/// endpoint chooses between them based on whether any threshold query
/// param is supplied; the two paths must agree on the "no override"
/// case or callers see verdict drift between calls with and without
/// params.
#[test]
fn encoding_depth_shim_matches_default_explicit() {
    let cases = [(0.001, 1.0), (0.5, 0.05), (1.0, 0.001), (0.05, 0.5)];
    for (k, l1) in cases {
        assert_eq!(
            encoding_depth(k, l1),
            encoding_depth_with(k, l1, &DepthConfig::default()),
            "shim vs explicit default disagreed at K={k}, λ₁={l1}"
        );
    }
}

// ── PERCEIVE contract (Theorem 8.6, step 4a math layer) ────────────

/// PerceptionResult is the wire-side payload for a future
/// /v1/bundles/{name}/perceive endpoint. Pin the field set + types
/// so the JSON shape can't drift before the endpoint lands.
#[test]
fn perception_result_has_expected_field_set() {
    let r = vec![1.0, 0.0, 0.0, 1.0];
    let v = vec![1.0, 0.0];
    let res: PerceptionResult = perceive(&r, &v, 2).expect("perceive");
    let _: Vec<f64> = res.v_perceived;
    let _: f64 = res.bias;
}

/// Identity rotation: bias is exactly 0, perceived = input. This is
/// the wire-visible "no drift" verdict callers depend on.
#[test]
fn perceive_identity_returns_zero_bias_passthrough() {
    let id = vec![1.0, 0.0, 0.0,
                  0.0, 1.0, 0.0,
                  0.0, 0.0, 1.0];
    let v = vec![3.0, -1.0, 4.0];
    let res = perceive(&id, &v, 3).expect("identity");
    assert_eq!(res.v_perceived, v);
    assert_eq!(res.bias, 0.0);
}

/// PerceiveError variants are exhaustive — the HTTP endpoint will
/// translate them into 400-class responses with field-specific
/// diagnostics. Pin the variant set so a future addition forces a
/// compile-time decision on the wire mapping.
#[test]
fn perceive_error_variants_pinned() {
    // EmptyDimension
    assert_eq!(perceive(&[], &[], 0), Err(PerceiveError::EmptyDimension));
    // NonSquareRotation
    assert_eq!(
        perceive(&[1.0, 0.0, 0.0], &[0.0, 0.0], 2),
        Err(PerceiveError::NonSquareRotation { dim: 2, len: 3 })
    );
    // VectorDimMismatch
    assert_eq!(
        perceive(&[1.0, 0.0, 0.0, 1.0], &[0.0, 0.0, 0.0], 2),
        Err(PerceiveError::VectorDimMismatch { rotation_dim: 2, vector_len: 3 })
    );
    // Error type itself implements Display + Error (the HTTP layer
    // needs both to format diagnostics).
    let e = PerceiveError::EmptyDimension;
    let _: String = format!("{}", e);
    let _: &dyn std::error::Error = &e;
}

/// `perception_bias` (stand-alone) matches the bias field of `perceive`.
/// The endpoint may call either depending on whether v is supplied; the
/// two paths must return identical numbers for the same R.
#[test]
fn perception_bias_standalone_matches_combined() {
    let r = vec![0.6, -0.8,
                 0.8,  0.6];
    let v = vec![1.0, 0.0];
    let standalone = perception_bias(&r, 2).expect("bias");
    let combined = perceive(&r, &v, 2).expect("perceive").bias;
    assert!((standalone - combined).abs() < 1e-15);
}

/// PerceptionResult serialization round-trip — the JSON shape the HTTP
/// endpoint will emit / the consumer will deserialize.
#[test]
fn perception_result_roundtrips_through_json() {
    let r = vec![0.0, -1.0, 1.0, 0.0];
    let v = vec![1.0, 0.0];
    let res = perceive(&r, &v, 2).expect("perceive");
    let json = serde_json::to_string(&res).expect("serialize");
    let back: PerceptionResult = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(res, back);
}

// ── LOCAL_HOLONOMY contract (Marcella COHERENCE_SIGNAL_SPEC §3) ────

/// LocalHolonomyResponse is the wire-side payload for the
/// /v1/bundles/{name}/local_holonomy endpoint. Pin the field set +
/// types so the JSON shape can't drift before consumers (Marcella)
/// deserialize.
#[test]
fn local_holonomy_result_has_expected_field_set() {
    let identity = vec![1.0, 0.0, 0.0, 1.0];
    let res: LocalHolonomyResult = local_holonomy(&identity, &identity, 2).expect("local_holonomy");
    // Compile-time field-shape pin.
    let _: Vec<f64> = res.r_window;
    let _: f64 = res.defect;
    let _: f64 = res.coherence;
}

/// Identity-rotation pair yields perfect coherence on the wire. The
/// "no drift" signal the laminar-cognition consumer would observe.
#[test]
fn local_holonomy_identity_pair_wire_contract() {
    let identity = vec![1.0, 0.0, 0.0,
                         0.0, 1.0, 0.0,
                         0.0, 0.0, 1.0];
    let res = local_holonomy(&identity, &identity, 3).expect("local_holonomy");
    assert_eq!(res.r_window, identity);
    assert_eq!(res.defect, 0.0);
    assert!((res.coherence - 1.0).abs() < 1e-12);
}

/// LocalHolonomyResult JSON round-trip — the wire layer's
/// serialization must be bit-deterministic so consumers can rely on
/// it.
#[test]
fn local_holonomy_result_roundtrips_through_json() {
    let r1 = vec![0.0, -1.0, 1.0, 0.0];
    let r2 = vec![1.0, 0.0, 0.0, 1.0];
    let res = local_holonomy(&r1, &r2, 2).expect("local_holonomy");
    let json = serde_json::to_string(&res).expect("serialize");
    let back: LocalHolonomyResult = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(res, back);
}

/// Gauge-invariance — the LOAD-BEARING property per Marcella's §3
/// proof. The wire response's `defect` and `coherence` MUST be
/// invariant under simultaneous unitary conjugation of both inputs.
/// If a future implementation breaks this, Marcella's gain gate
/// receives a gauge-dependent signal that breaks her training.
#[test]
fn local_holonomy_wire_response_is_gauge_invariant() {
    // Two unrelated 2D rotations.
    let theta1 = 30_f64.to_radians();
    let theta2 = 65_f64.to_radians();
    let (c1, s1) = (theta1.cos(), theta1.sin());
    let (c2, s2) = (theta2.cos(), theta2.sin());
    let r_current = vec![c1, -s1, s1, c1];
    let r_past = vec![c2, -s2, s2, c2];
    let baseline = local_holonomy(&r_current, &r_past, 2).expect("baseline");

    // Q = 50° conjugation rotation.
    let phi = 50_f64.to_radians();
    let (cq, sq) = (phi.cos(), phi.sin());
    let q = [cq, -sq, sq, cq];
    let q_t = [cq, sq, -sq, cq];

    fn matmul(a: &[f64], b: &[f64]) -> [f64; 4] {
        [
            a[0]*b[0] + a[1]*b[2], a[0]*b[1] + a[1]*b[3],
            a[2]*b[0] + a[3]*b[2], a[2]*b[1] + a[3]*b[3],
        ]
    }
    let r_current_conj: Vec<f64> = matmul(&matmul(&q, &r_current), &q_t).into();
    let r_past_conj: Vec<f64> = matmul(&matmul(&q, &r_past), &q_t).into();
    let conj = local_holonomy(&r_current_conj, &r_past_conj, 2).expect("conj");

    assert!(
        (baseline.defect - conj.defect).abs() < 1e-10,
        "wire-layer defect not gauge-invariant: baseline={} conj={}",
        baseline.defect, conj.defect
    );
    assert!(
        (baseline.coherence - conj.coherence).abs() < 1e-10,
        "wire-layer coherence not gauge-invariant"
    );
}

/// PerceiveError variants reused by local_holonomy — pinning the
/// shared error enum surface so the HTTP error mapping is consistent
/// between PERCEIVE and LOCAL_HOLONOMY.
#[test]
fn local_holonomy_uses_perceive_error_variants() {
    // Empty dim.
    assert_eq!(
        local_holonomy(&[], &[], 0),
        Err(PerceiveError::EmptyDimension)
    );
    // R_current wrong shape.
    let r2 = vec![1.0, 0.0, 0.0, 1.0];
    assert_eq!(
        local_holonomy(&[1.0, 0.0, 0.0], &r2, 2),
        Err(PerceiveError::NonSquareRotation { dim: 2, len: 3 })
    );
}
