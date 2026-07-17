//! Marcella dials wave 2, ask 3 — WINDOWED_COHERENCE one-shot
//! (GEODESIC_LOOM_PLAN.md ask #3, signed Hallie, 2026-07-16).
//!
//! POST /v1/bundles/{b}/windowed_coherence takes an ordered record-key
//! path, a base key_field, a window size, a fiber scope (same scoping
//! as ask 4's `fields=`: scalar family with `..` range sugar, or one
//! Value::Vector fiber), and an optional laminar threshold. It
//! composes, SERVER-SIDE, the same math Marcella's laminar gate
//! previously round-tripped per segment:
//!
//!   1. per-segment transport rotation — the TRANSPORT_ROTATION
//!      Rodrigues construction (now `dials::transport_rotation_matrix`,
//!      the exact code the GQL verb executes) with the angle DERIVED
//!      from the data: θ_seg = arccos(clamp(cos_sim(v_i, v_{i+1}), −1, 1)),
//!      the minimal rotation carrying v_i to v_{i+1} in their plane;
//!   2. cumulative frames R_acc via matrix product;
//!   3. per-window `curvature::local_holonomy(R_acc[i+w−1], R_acc[i])`
//!      → R_window = R_acc[i+w−1]·R_acc[i]ᵀ, holonomy_defect =
//!      ‖R_window − I‖_F, coherence = 1 − defect/(2√dim);
//!   4. laminar = coherence ≥ threshold (default 0.91 — Marcella's
//!      COHERENCE_CONFIDENT constant, the accept gate her loom uses;
//!      her hedge tier is 0.85 and the server's older local_holonomy
//!      prose says 0.9 — 0.91 is the pinned default here).
//!
//! n_windows = len(path) − window + 1, sliding by 1.
//!
//! For 2-dim fibers on a circle advancing by a fixed angle φ per
//! record, a window covering w records composes w−1 segment rotations:
//! defect = ‖R(α) − I‖_F = 2√2·|sin(α/2)| with α = (w−1)·φ — the
//! hand-computable anchor A3-2 pins to 1e-9.

use gigi::curvature;
use gigi::dials::{self, WindowedCoherenceRequest};
use gigi::types::{BundleSchema, FieldDef, FieldType, Record, Value};
use gigi::{BundleRef, BundleStore};
use serde_json::json;

const PHI: f64 = 0.05;
const JUMP: f64 = std::f64::consts::FRAC_PI_2;

fn req(
    path: &[&str],
    key_field: &str,
    window: usize,
    fiber: &[&str],
    threshold: Option<f64>,
) -> WindowedCoherenceRequest {
    WindowedCoherenceRequest {
        path: path.iter().map(|s| json!(s)).collect(),
        key_field: key_field.to_string(),
        window,
        fiber: fiber.iter().map(|s| s.to_string()).collect(),
        threshold,
    }
}

/// 5 records with IDENTICAL fiber vectors (0.6, 0.8).
fn ident_store() -> BundleStore {
    let schema = BundleSchema::new("wc_ident")
        .base(FieldDef::categorical("id"))
        .fiber(FieldDef::numeric("f0"))
        .fiber(FieldDef::numeric("f1"));
    let mut store = BundleStore::new(schema);
    for i in 0..5 {
        let mut r = Record::new();
        r.insert("id".into(), Value::Text(format!("r{i}")));
        r.insert("f0".into(), Value::Float(0.6));
        r.insert("f1".into(), Value::Float(0.8));
        store.insert(&r);
    }
    store
}

/// 9 records on the unit circle: consecutive records rotated by φ,
/// except one deliberate discontinuity of π/2 between r5 and r6.
/// Segment angles: s_i = φ for i ≠ 5, s_5 = π/2.
fn rot_store() -> BundleStore {
    let schema = BundleSchema::new("wc_rot")
        .base(FieldDef::categorical("id"))
        .fiber(FieldDef::numeric("f0"))
        .fiber(FieldDef::numeric("f1"));
    let mut store = BundleStore::new(schema);
    for (i, theta) in circle_angles().iter().enumerate() {
        let mut r = Record::new();
        r.insert("id".into(), Value::Text(format!("r{i}")));
        r.insert("f0".into(), Value::Float(theta.cos()));
        r.insert("f1".into(), Value::Float(theta.sin()));
        store.insert(&r);
    }
    store
}

fn circle_angles() -> Vec<f64> {
    let mut angles = Vec::new();
    let mut theta = 0.0;
    for i in 0..9 {
        angles.push(theta);
        let seg = if i == 5 { JUMP } else { PHI };
        theta += seg;
    }
    angles
}

/// Same geometry as `rot_store`, but the fiber is one Value::Vector
/// field of dims 2 (ask-4 scoping parity: fiber=["emb"]).
fn rot_vector_store() -> BundleStore {
    let mut schema = BundleSchema::new("wc_vec").base(FieldDef::categorical("id"));
    let mut emb = FieldDef::numeric("emb");
    emb.field_type = FieldType::Vector { dims: 2 };
    schema = schema.fiber(emb);
    let mut store = BundleStore::new(schema);
    for (i, theta) in circle_angles().iter().enumerate() {
        let mut r = Record::new();
        r.insert("id".into(), Value::Text(format!("r{i}")));
        r.insert("emb".into(), Value::Vector(vec![theta.cos(), theta.sin()]));
        store.insert(&r);
    }
    store
}

/// Expected Frobenius defect of a 2-dim rotation by total angle α:
/// ‖R(α) − I‖_F = 2√2·|sin(α/2)|.
fn defect_2d(alpha: f64) -> f64 {
    2.0 * 2.0_f64.sqrt() * (alpha / 2.0).sin().abs()
}

// ═════════════════════════════════════════════════════════════════════
// A3-1 — identity transport: identical vectors → defect exactly 0.0
// ═════════════════════════════════════════════════════════════════════

#[test]
fn a3_1_identity_transport_defect_exactly_zero() {
    let store = ident_store();
    let bref = BundleRef::Heap(&store);
    let report = dials::windowed_coherence_report(
        &bref,
        &req(&["r0", "r1", "r2", "r3", "r4"], "id", 2, &["f0", "f1"], None),
    )
    .expect("identity path");

    assert_eq!(report.n_windows, 4);
    assert_eq!(report.windows.len(), 4);
    for w in &report.windows {
        assert_eq!(
            w.holonomy_defect, 0.0,
            "identical vectors → derived angle 0 → R_seg = I exactly → defect must be \
             EXACTLY 0.0, got {} at start_index {}",
            w.holonomy_defect, w.start_index
        );
        assert!(w.laminar, "zero defect is laminar at any threshold in (0,1]");
        assert!((w.coherence - 1.0).abs() < 1e-15);
    }
    assert!(report.laminar_all, "every window laminar → laminar_all");
    assert_eq!(report.threshold_used, dials::DEFAULT_LAMINAR_THRESHOLD);
    assert_eq!(report.dim, 2);
}

#[test]
fn a3_1_identity_transport_window_covering_whole_path() {
    let store = ident_store();
    let bref = BundleRef::Heap(&store);
    let report = dials::windowed_coherence_report(
        &bref,
        &req(&["r0", "r1", "r2", "r3", "r4"], "id", 5, &["f0", "f1"], None),
    )
    .expect("identity path, window 5");
    assert_eq!(report.n_windows, 1);
    assert_eq!(report.windows[0].holonomy_defect, 0.0);
    assert!(report.laminar_all);
}

/// The default laminar threshold is Marcella's COHERENCE_CONFIDENT.
#[test]
fn a3_1_default_threshold_is_marcellas_confident_gate() {
    assert_eq!(dials::DEFAULT_LAMINAR_THRESHOLD, 0.91);
}

// ═════════════════════════════════════════════════════════════════════
// A3-2 — known rotation: hand-computed defects, pinned to 1e-9;
//         laminar flips exactly at the constructed discontinuity
// ═════════════════════════════════════════════════════════════════════

#[test]
fn a3_2_known_rotation_defects_pinned_window_2() {
    let store = rot_store();
    let bref = BundleRef::Heap(&store);
    let path: Vec<String> = (0..9).map(|i| format!("r{i}")).collect();
    let path_refs: Vec<&str> = path.iter().map(|s| s.as_str()).collect();
    let report = dials::windowed_coherence_report(
        &bref,
        &req(&path_refs, "id", 2, &["f0", "f1"], None),
    )
    .expect("rotation path");

    assert_eq!(report.n_windows, 8);
    for (i, w) in report.windows.iter().enumerate() {
        let alpha = if i == 5 { JUMP } else { PHI };
        let expected = defect_2d(alpha);
        assert!(
            (w.holonomy_defect - expected).abs() < 1e-9,
            "window {i}: hand defect 2√2·sin({alpha}/2) = {expected}; got {}",
            w.holonomy_defect
        );
        // coherence = 1 − defect/(2√2): 0.975 for φ-windows, 0.2929 at
        // the jump. Default threshold 0.91 flips laminar EXACTLY at the
        // constructed discontinuity.
        assert_eq!(
            w.laminar,
            i != 5,
            "laminar must flip exactly at the discontinuity window (i=5); \
             window {i} coherence {}",
            w.coherence
        );
        assert_eq!(w.start_index, i);
    }
    assert!(!report.laminar_all);
}

#[test]
fn a3_2_known_rotation_defects_pinned_window_3() {
    let store = rot_store();
    let bref = BundleRef::Heap(&store);
    let path: Vec<String> = (0..9).map(|i| format!("r{i}")).collect();
    let path_refs: Vec<&str> = path.iter().map(|s| s.as_str()).collect();
    let report = dials::windowed_coherence_report(
        &bref,
        &req(&path_refs, "id", 3, &["f0", "f1"], None),
    )
    .expect("rotation path, window 3");

    assert_eq!(report.n_windows, 7);
    for (i, w) in report.windows.iter().enumerate() {
        // window i composes segments s_i + s_{i+1}; s_5 = π/2, rest φ.
        let alpha = if i == 4 || i == 5 { PHI + JUMP } else { 2.0 * PHI };
        let expected = defect_2d(alpha);
        assert!(
            (w.holonomy_defect - expected).abs() < 1e-9,
            "window {i}: hand defect {expected}; got {}",
            w.holonomy_defect
        );
        // 1 − sin(φ) = 0.95 laminar; 1 − sin((φ+π/2)/2) ≈ 0.2753 not.
        assert_eq!(w.laminar, !(i == 4 || i == 5), "window {i}");
    }
    assert!(!report.laminar_all);
}

/// threshold is a caller dial: low threshold admits the jump, high
/// threshold rejects even the calm windows. threshold_used echoes.
#[test]
fn a3_2_threshold_override_moves_the_verdict() {
    let store = rot_store();
    let bref = BundleRef::Heap(&store);
    let path: Vec<String> = (0..9).map(|i| format!("r{i}")).collect();
    let path_refs: Vec<&str> = path.iter().map(|s| s.as_str()).collect();

    let lenient = dials::windowed_coherence_report(
        &bref,
        &req(&path_refs, "id", 2, &["f0", "f1"], Some(0.2)),
    )
    .expect("lenient threshold");
    assert_eq!(lenient.threshold_used, 0.2);
    assert!(lenient.laminar_all, "0.2929 ≥ 0.2 → even the jump window passes");

    let strict = dials::windowed_coherence_report(
        &bref,
        &req(&path_refs, "id", 2, &["f0", "f1"], Some(0.99)),
    )
    .expect("strict threshold");
    assert_eq!(strict.threshold_used, 0.99);
    assert!(
        strict.windows.iter().all(|w| !w.laminar),
        "0.975 < 0.99 → every window fails the strict gate"
    );
    assert!(!strict.laminar_all);
}

/// The same geometry through a Value::Vector fiber (ask-4 scoping):
/// fiber=["emb"] behaves identically to the scalar pair.
#[test]
fn a3_2_vector_fiber_scoping_matches_scalar_family() {
    let store = rot_vector_store();
    let bref = BundleRef::Heap(&store);
    let path: Vec<String> = (0..9).map(|i| format!("r{i}")).collect();
    let path_refs: Vec<&str> = path.iter().map(|s| s.as_str()).collect();
    let report = dials::windowed_coherence_report(
        &bref,
        &req(&path_refs, "id", 2, &["emb"], None),
    )
    .expect("vector fiber path");
    assert_eq!(report.dim, 2, "dims=2 vector fiber → dim 2");
    for (i, w) in report.windows.iter().enumerate() {
        let alpha = if i == 5 { JUMP } else { PHI };
        assert!(
            (w.holonomy_defect - defect_2d(alpha)).abs() < 1e-9,
            "window {i} through the vector fiber"
        );
    }
}

/// Range sugar in the fiber spec (same scoping as ask 4).
#[test]
fn a3_2_fiber_range_sugar_works() {
    let store = rot_store();
    let bref = BundleRef::Heap(&store);
    let report = dials::windowed_coherence_report(
        &bref,
        &req(&["r0", "r1", "r2"], "id", 2, &["f0..f1"], None),
    )
    .expect("range sugar fiber");
    assert_eq!(report.dim, 2);
    assert!((report.windows[0].holonomy_defect - defect_2d(PHI)).abs() < 1e-9);
}

/// Duplicate consecutive keys are legal: the segment angle is 0 and the
/// window defect exactly 0.
#[test]
fn a3_2_duplicate_consecutive_keys_are_identity_segments() {
    let store = rot_store();
    let bref = BundleRef::Heap(&store);
    let report = dials::windowed_coherence_report(
        &bref,
        &req(&["r0", "r0", "r1"], "id", 2, &["f0", "f1"], None),
    )
    .expect("duplicate keys");
    assert_eq!(report.windows[0].holonomy_defect, 0.0, "r0→r0 is the identity segment");
    assert!((report.windows[1].holonomy_defect - defect_2d(PHI)).abs() < 1e-9);
}

// ═════════════════════════════════════════════════════════════════════
// A3-3 — window arithmetic
// ═════════════════════════════════════════════════════════════════════

#[test]
fn a3_3_window_arithmetic_and_start_indices() {
    let store = ident_store();
    let bref = BundleRef::Heap(&store);
    let path = ["r0", "r1", "r2", "r3", "r4"];

    // len(path)=5, window=3 → n_windows=3, start indices 0,1,2.
    let w3 = dials::windowed_coherence_report(
        &bref,
        &req(&path, "id", 3, &["f0", "f1"], None),
    )
    .expect("window 3");
    assert_eq!(w3.n_windows, 3);
    assert_eq!(
        w3.windows.iter().map(|w| w.start_index).collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    // Each window echoes its keys (path[i..i+window]).
    assert_eq!(w3.windows[0].keys, vec![json!("r0"), json!("r1"), json!("r2")]);
    assert_eq!(w3.windows[2].keys, vec![json!("r2"), json!("r3"), json!("r4")]);
    assert_eq!(w3.window, 3, "window echoed");

    // window=5 → 1 window.
    let w5 = dials::windowed_coherence_report(
        &bref,
        &req(&path, "id", 5, &["f0", "f1"], None),
    )
    .expect("window 5");
    assert_eq!(w5.n_windows, 1);

    // window=6 → typed error naming the bound.
    let err = dials::windowed_coherence_report(
        &bref,
        &req(&path, "id", 6, &["f0", "f1"], None),
    )
    .expect_err("window 6 exceeds len(path)");
    match err {
        dials::DialError::BadRequest(msg) => {
            assert!(
                msg.contains("window") && msg.contains('6') && msg.contains('5'),
                "names the bound: {msg}"
            );
        }
        other => panic!("expected BadRequest, got {other:?}"),
    }

    // window < 2 → typed error.
    for w in [0usize, 1] {
        let err = dials::windowed_coherence_report(
            &bref,
            &req(&path, "id", w, &["f0", "f1"], None),
        )
        .expect_err("window < 2");
        assert!(matches!(err, dials::DialError::BadRequest(_)), "window={w} is a 400");
    }
}

// ═════════════════════════════════════════════════════════════════════
// A3-4 — parity with the existing two-call surface
// ═════════════════════════════════════════════════════════════════════

/// For one window, the one-shot's defect equals what the existing
/// two-call surface yields on the same segment data: the
/// TRANSPORT_ROTATION Rodrigues construction
/// (`dials::transport_rotation_matrix` — the exact fn the GQL verb now
/// delegates to) with the same derived angle, composed through
/// `curvature::local_holonomy` (the exact fn behind
/// POST /local_holonomy) against the identity past-frame. Same fn
/// bodies, so parity holds to 1e-12.
#[test]
fn a3_4_one_shot_matches_two_call_surface_to_1e12() {
    let store = rot_store();
    let bref = BundleRef::Heap(&store);

    // One-shot: window=2 over [r0, r1].
    let one_shot = dials::windowed_coherence_report(
        &bref,
        &req(&["r0", "r1"], "id", 2, &["f0", "f1"], None),
    )
    .expect("one-shot");
    assert_eq!(one_shot.n_windows, 1);
    let one_shot_defect = one_shot.windows[0].holonomy_defect;

    // Two-call: extract the same segment data the one-shot sees…
    let u = vec![0.0_f64.cos(), 0.0_f64.sin()];
    let v = vec![PHI.cos(), PHI.sin()];
    // …derive the same transport angle (arccos of clamped cosine
    // similarity — the minimal rotation carrying u to v)…
    let dot: f64 = u.iter().zip(&v).map(|(a, b)| a * b).sum();
    let nu: f64 = u.iter().map(|x| x * x).sum::<f64>().sqrt();
    let nv: f64 = v.iter().map(|x| x * x).sum::<f64>().sqrt();
    let theta = (dot / (nu * nv)).clamp(-1.0, 1.0).acos();
    // …call the TRANSPORT_ROTATION construction (call 1)…
    let r_seg = dials::transport_rotation_matrix(&u, &v, theta);
    // …and LOCAL_HOLONOMY against the identity past-frame (call 2).
    let identity = vec![1.0, 0.0, 0.0, 1.0];
    let two_call = curvature::local_holonomy(&r_seg, &identity, 2).expect("local_holonomy");

    assert!(
        (one_shot_defect - two_call.defect).abs() < 1e-12,
        "A3-4 parity: one-shot {one_shot_defect} vs two-call {}",
        two_call.defect
    );
    assert!(
        (one_shot.windows[0].coherence - two_call.coherence).abs() < 1e-12,
        "coherence parity"
    );
}

/// The transport matrix itself carries v_0 to v_1 (the Rodrigues
/// construction with the derived angle is the minimal rotation between
/// them) — the same property TRANSPORT_ROTATION's consumers rely on.
#[test]
fn a3_4_transport_rotation_matrix_carries_u_to_v() {
    let u = vec![1.0, 0.0];
    let v = vec![PHI.cos(), PHI.sin()];
    let r = dials::transport_rotation_matrix(&u, &v, PHI);
    // R·u should land on v (both unit vectors, angle φ apart).
    let ru = [r[0] * u[0] + r[1] * u[1], r[2] * u[0] + r[3] * u[1]];
    assert!((ru[0] - v[0]).abs() < 1e-12 && (ru[1] - v[1]).abs() < 1e-12);
}

// ═════════════════════════════════════════════════════════════════════
// Typed error contract + envelope
// ═════════════════════════════════════════════════════════════════════

#[test]
fn a3_err_unknown_key_field_is_404_class_naming_it() {
    let store = rot_store();
    let bref = BundleRef::Heap(&store);
    let err = dials::windowed_coherence_report(
        &bref,
        &req(&["r0", "r1"], "recod_id", 2, &["f0", "f1"], None),
    )
    .expect_err("unknown key_field");
    match err {
        dials::DialError::NotFound(msg) => {
            assert!(msg.contains("recod_id"), "names the unknown key_field: {msg}")
        }
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[test]
fn a3_err_missing_key_is_404_class_naming_which_key() {
    let store = rot_store();
    let bref = BundleRef::Heap(&store);
    let err = dials::windowed_coherence_report(
        &bref,
        &req(&["r0", "r99", "r1"], "id", 2, &["f0", "f1"], None),
    )
    .expect_err("missing key");
    match err {
        dials::DialError::NotFound(msg) => {
            assert!(
                msg.contains("r99") && msg.contains("id"),
                "names WHICH key is missing: {msg}"
            );
        }
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[test]
fn a3_err_fiber_validation_as_wave_1() {
    let store = rot_store();
    let bref = BundleRef::Heap(&store);
    // Unknown fiber field.
    let err = dials::windowed_coherence_report(
        &bref,
        &req(&["r0", "r1"], "id", 2, &["f9"], None),
    )
    .expect_err("unknown fiber");
    match err {
        dials::DialError::BadRequest(msg) => {
            assert!(msg.contains("f9"), "names the fiber field: {msg}")
        }
        other => panic!("expected BadRequest, got {other:?}"),
    }
    // Empty fiber list.
    let err = dials::windowed_coherence_report(
        &bref,
        &req(&["r0", "r1"], "id", 2, &[], None),
    )
    .expect_err("empty fiber");
    assert!(matches!(err, dials::DialError::BadRequest(_)));
    // Non-numeric fiber (the base field is categorical — but base
    // fields aren't fibers, so name a text FIBER on a fresh store).
    let schema = BundleSchema::new("typed")
        .base(FieldDef::categorical("id"))
        .fiber(FieldDef::numeric("x"))
        .fiber(FieldDef::categorical("label"));
    let mut s2 = BundleStore::new(schema);
    for i in 0..2 {
        let mut r = Record::new();
        r.insert("id".into(), Value::Text(format!("r{i}")));
        r.insert("x".into(), Value::Float(i as f64));
        r.insert("label".into(), Value::Text("blue".into()));
        s2.insert(&r);
    }
    let bref2 = BundleRef::Heap(&s2);
    let err = dials::windowed_coherence_report(
        &bref2,
        &req(&["r0", "r1"], "id", 2, &["x", "label"], None),
    )
    .expect_err("non-numeric fiber");
    match err {
        dials::DialError::BadRequest(msg) => {
            assert!(msg.contains("label") && msg.contains("not numeric"), "{msg}")
        }
        other => panic!("expected BadRequest, got {other:?}"),
    }
}

#[test]
fn a3_err_threshold_out_of_range_is_loud() {
    let store = rot_store();
    let bref = BundleRef::Heap(&store);
    for bad in [0.0, -0.5, 1.5, f64::NAN] {
        let err = dials::windowed_coherence_report(
            &bref,
            &req(&["r0", "r1"], "id", 2, &["f0", "f1"], Some(bad)),
        )
        .expect_err("bad threshold");
        assert!(
            matches!(err, dials::DialError::BadRequest(_)),
            "threshold {bad} must be rejected (valid range (0, 1])"
        );
    }
    // Boundary: threshold = 1.0 is legal (exact-identity gate).
    let ok = dials::windowed_coherence_report(
        &bref,
        &req(&["r0", "r1"], "id", 2, &["f0", "f1"], Some(1.0)),
    );
    assert!(ok.is_ok(), "threshold 1.0 is in range");
}

/// Wire-shape pin: the serialized report carries the ask-3 response
/// contract ({windows: [{start_index, keys, holonomy_defect, laminar}],
/// n_windows, laminar_all, threshold_used}) plus the standard
/// lambda_budget envelope.
#[test]
fn a3_wire_shape_and_lambda_envelope() {
    let store = rot_store();
    let bref = BundleRef::Heap(&store);
    let report = dials::windowed_coherence_report(
        &bref,
        &req(&["r0", "r1", "r2"], "id", 2, &["f0", "f1"], None),
    )
    .expect("report");
    assert!(report.lambda_budget.is_finite(), "standard λ-budget envelope rides along");
    assert_eq!(report.bundle, "wc_rot");

    let j: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&report).unwrap()).unwrap();
    for key in [
        "windows",
        "n_windows",
        "laminar_all",
        "threshold_used",
        "dim",
        "window",
        "bundle",
        "lambda_budget",
    ] {
        assert!(j.get(key).is_some(), "response carries '{key}'");
    }
    let w0 = &j["windows"][0];
    for key in ["start_index", "keys", "holonomy_defect", "coherence", "laminar"] {
        assert!(w0.get(key).is_some(), "window rows carry '{key}'");
    }
    assert_eq!(j["windows"].as_array().unwrap().len(), 2);
}

/// On an all-identical-vector bundle the λ-budget envelope is the
/// documented flat-bundle saturation value 1.0 (K = 0 → horizon fully
/// open) — the envelope is the WHOLE-BUNDLE ride-along, matching every
/// other lambda_budget-carrying response.
#[test]
fn a3_lambda_envelope_is_whole_bundle_ride_along() {
    let store = ident_store();
    let bref = BundleRef::Heap(&store);
    let report = dials::windowed_coherence_report(
        &bref,
        &req(&["r0", "r1"], "id", 2, &["f0", "f1"], None),
    )
    .expect("report");
    assert_eq!(
        report.lambda_budget,
        curvature::lambda_budget_for_bundle(&store),
        "the envelope mirrors curvature::lambda_budget_for_bundle"
    );
}

// ═════════════════════════════════════════════════════════════════════
// Review follow-ups (2026-07-16 ship lens) — edges pinned, convention
// anchored
// ═════════════════════════════════════════════════════════════════════

/// EXACTLY-antipodal consecutive vectors (v_{i+1} = −v_i) derive
/// θ = arccos(−1) = π, but v is collinear with e1 so the Rodrigues e2
/// degenerates and the segment transports as IDENTITY: defect exactly
/// 0.0, coherence 1.0, laminar. This is the verb's inherited collinear
/// guard, now PINNED: an exact 180° semantic reversal reads perfectly
/// laminar, and the signal is discontinuous vs near-antipodal (defect
/// → 2√2). Measure-zero on real float embeddings; callers needing
/// exact-reversal detection gate on the segment cosine instead.
#[test]
fn review_exact_antipodal_transports_as_identity_pinned() {
    let schema = BundleSchema::new("wc_anti")
        .base(FieldDef::categorical("id"))
        .fiber(FieldDef::numeric("f0"))
        .fiber(FieldDef::numeric("f1"));
    let mut store = BundleStore::new(schema);
    let near = std::f64::consts::PI - 0.01;
    for (id, (x, y)) in [
        ("r0", (1.0, 0.0)),
        ("r1", (-1.0, 0.0)),        // exact antipode of r0
        ("r2", (near.cos(), near.sin())), // NEAR-antipodal to r0
    ] {
        let mut r = Record::new();
        r.insert("id".into(), Value::Text(id.to_string()));
        r.insert("f0".into(), Value::Float(x));
        r.insert("f1".into(), Value::Float(y));
        store.insert(&r);
    }
    let bref = BundleRef::Heap(&store);

    // Exact antipode: identity transport, defect EXACTLY 0.0.
    let exact = dials::windowed_coherence_report(
        &bref,
        &req(&["r0", "r1"], "id", 2, &["f0", "f1"], None),
    )
    .expect("exact-antipodal path");
    assert_eq!(
        exact.windows[0].holonomy_defect, 0.0,
        "v→−v is collinear ⇒ e2 degenerate ⇒ identity transport ⇒ defect exactly 0.0"
    );
    assert!((exact.windows[0].coherence - 1.0).abs() < 1e-15);
    assert!(exact.windows[0].laminar, "the pinned (surprising) verdict");

    // Near-antipodal: defect ≈ 2√2·sin((π−0.01)/2) — pins the
    // documented discontinuity right next to the exact case.
    let near_rep = dials::windowed_coherence_report(
        &bref,
        &req(&["r0", "r2"], "id", 2, &["f0", "f1"], None),
    )
    .expect("near-antipodal path");
    let expected = defect_2d(near);
    assert!(
        (near_rep.windows[0].holonomy_defect - expected).abs() < 1e-9,
        "near-antipodal defect: expected {expected}, got {}",
        near_rep.windows[0].holonomy_defect
    );
    assert!(
        near_rep.windows[0].holonomy_defect > 2.8,
        "…which sits at the 2√2 cap — the discontinuity receipt"
    );
    assert!(!near_rep.windows[0].laminar, "near-antipodal is loudly non-laminar (dim 2)");
}

/// Zero-norm endpoints transport as identity through BOTH guards
/// (derived_transport_angle → θ=0; transport_rotation_matrix → ‖u‖
/// guard): a zero vector anywhere in the path yields defect exactly
/// 0.0 on every segment touching it. Pinned (was previously only
/// asserted in prose).
#[test]
fn review_zero_norm_endpoint_transports_as_identity_pinned() {
    let schema = BundleSchema::new("wc_zero")
        .base(FieldDef::categorical("id"))
        .fiber(FieldDef::numeric("f0"))
        .fiber(FieldDef::numeric("f1"));
    let mut store = BundleStore::new(schema);
    for (id, (x, y)) in [("r0", (1.0, 0.0)), ("rz", (0.0, 0.0)), ("r1", (0.0, 1.0))] {
        let mut r = Record::new();
        r.insert("id".into(), Value::Text(id.to_string()));
        r.insert("f0".into(), Value::Float(x));
        r.insert("f1".into(), Value::Float(y));
        store.insert(&r);
    }
    let bref = BundleRef::Heap(&store);
    let report = dials::windowed_coherence_report(
        &bref,
        &req(&["r0", "rz", "r1"], "id", 2, &["f0", "f1"], None),
    )
    .expect("zero-norm path");
    assert_eq!(report.n_windows, 2);
    for w in &report.windows {
        assert_eq!(
            w.holonomy_defect, 0.0,
            "segments into/out of the zero vector transport as identity \
             (start_index {})",
            w.start_index
        );
        assert!((w.coherence - 1.0).abs() < 1e-15);
    }
}

/// Row-major 3×3 product for the composition-order anchor below.
fn matmul3(a: &[f64], b: &[f64]) -> Vec<f64> {
    let mut out = vec![0.0_f64; 9];
    for i in 0..3 {
        for j in 0..3 {
            out[i * 3 + j] = (0..3).map(|k| a[i * 3 + k] * b[k * 3 + j]).sum();
        }
    }
    out
}

/// Composition-order anchor (review follow-up): the 2-dim fixtures
/// cannot distinguish R_seg(s+w−2)···R_seg(s) from its reversal (plane
/// rotations commute), and for TWO segments the defect never can
/// (defect² = 2n − 2·tr is a class function and AB ~ BA). THREE
/// non-coplanar segments at dim 3 break the symmetry: tr(C·B·A) ≠
/// tr(A·B·C) in general.
///
/// Path x → y → z → x in R³: three π/2 segment rotations in the xy,
/// yz, zx planes. Documented order W = C·B·A has tr = 1 → defect = 2;
/// the REVERSED order A·B·C has tr = −1 → defect = 2√2 ≈ 2.828. The
/// one-shot must land on 2 — pinning the left-accumulation convention
/// through the wire observable.
#[test]
fn review_composition_order_pinned_by_3d_non_coplanar_anchor() {
    let schema = BundleSchema::new("wc_order")
        .base(FieldDef::categorical("id"))
        .fiber(FieldDef::numeric("f0"))
        .fiber(FieldDef::numeric("f1"))
        .fiber(FieldDef::numeric("f2"));
    let mut store = BundleStore::new(schema);
    let pts: [(&str, [f64; 3]); 4] = [
        ("r0", [1.0, 0.0, 0.0]), // x
        ("r1", [0.0, 1.0, 0.0]), // y
        ("r2", [0.0, 0.0, 1.0]), // z
        ("r3", [1.0, 0.0, 0.0]), // x again
    ];
    for (id, p) in pts {
        let mut r = Record::new();
        r.insert("id".into(), Value::Text(id.to_string()));
        r.insert("f0".into(), Value::Float(p[0]));
        r.insert("f1".into(), Value::Float(p[1]));
        r.insert("f2".into(), Value::Float(p[2]));
        store.insert(&r);
    }
    let bref = BundleRef::Heap(&store);
    let report = dials::windowed_coherence_report(
        &bref,
        &req(&["r0", "r1", "r2", "r3"], "id", 4, &["f0", "f1", "f2"], None),
    )
    .expect("3-dim non-coplanar path");
    assert_eq!(report.n_windows, 1);
    assert_eq!(report.dim, 3);
    let one_shot_defect = report.windows[0].holonomy_defect;

    // Independent receipt: build the three segment rotations through
    // the SAME public construction, compose both orders explicitly.
    let x = [1.0, 0.0, 0.0];
    let y = [0.0, 1.0, 0.0];
    let z = [0.0, 0.0, 1.0];
    let half_pi = std::f64::consts::FRAC_PI_2;
    let a = dials::transport_rotation_matrix(&x, &y, half_pi); // segment 0
    let b = dials::transport_rotation_matrix(&y, &z, half_pi); // segment 1
    let c = dials::transport_rotation_matrix(&z, &x, half_pi); // segment 2

    let identity = vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
    let forward = matmul3(&c, &matmul3(&b, &a)); // documented: C·B·A
    let reversed = matmul3(&a, &matmul3(&b, &c)); // the wrong order
    let fwd = curvature::local_holonomy(&forward, &identity, 3).expect("fwd");
    let rev = curvature::local_holonomy(&reversed, &identity, 3).expect("rev");

    // Hand values: tr(C·B·A) = 1 → defect √(2·3 − 2·1) = 2;
    // tr(A·B·C) = −1 → defect √(6+2) = 2√2.
    assert!((fwd.defect - 2.0).abs() < 1e-9, "hand: C·B·A defect 2, got {}", fwd.defect);
    assert!(
        (rev.defect - 2.0 * 2.0_f64.sqrt()).abs() < 1e-9,
        "hand: A·B·C defect 2√2, got {}",
        rev.defect
    );
    assert!(
        (fwd.defect - rev.defect).abs() > 0.5,
        "the anchor genuinely discriminates the two orders"
    );
    assert!(
        (one_shot_defect - fwd.defect).abs() < 1e-9,
        "one-shot follows the DOCUMENTED order C·B·A: expected {}, got {one_shot_defect}",
        fwd.defect
    );
}
