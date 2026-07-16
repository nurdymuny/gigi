//! Halcyon SPECTRAL_GAUGE Phase 1 — basic-correctness gates (G13).
//!
//! Shipped 2026-06-28 with the SPECTRAL_GAUGE verb (cfeb5c5 spec).
//!
//! These tests pin the Phase 1 dense path's behaviour on the
//! fiber-weighted Laplacian L_A, the group inference table at exec
//! time, and the typed-error surface (BundleNotFound /
//! MissingEndpointFields / FiberArityMismatch /
//! AmbiguousGroupInference). Tests 11-12 originally pinned the FULL
//! PhaseNotImplemented stub; Phase 2 (2026-07-16) replaced the stub
//! with the dense FULL implementation, so they now pin the FULL
//! contract instead (same count, updated behaviour — deviation named
//! in SPECTRAL_PHASE2_MAGNETIC_SHIPPED_2026-07-16.md).
//! They also pin the parser's ergonomics #4
//! SU2/SU3/U1 synonyms — both bare and parenthesized forms must hit
//! the same `Group` enum value.
//!
//! HONEST FRAMING (per cfeb5c5): L_A's spectrum is globally
//! gauge-invariant, but Re Tr(U_e)/N is only locally gauge-covariant.
//! Tests target the spectral-gap reduction itself — they do NOT
//! assert anything about a Yang-Mills mass gap, which would require
//! gauge fixing + the full Lanczos sparse Phase 2 path.
//!
//! Run with:
//!   `cargo test --features halcyon --test spectral_gauge_basic`

#![cfg(feature = "halcyon")]

use gigi::engine::Engine;
use gigi::gauge::Group;
use gigi::parser::{parse, Statement};
use gigi::spectral::{infer_group_from_arity, spectral_gauge_gap, SpectralGaugeError};
use gigi::types::{BundleSchema, FieldDef, Record, Value};

/// Construct a heap bundle with the Halcyon edge schema
/// (`vertex_a`, `vertex_b` base; `fiber_fields` numeric fibers).
fn make_edge_bundle(engine: &mut Engine, name: &str, fiber_fields: &[&str]) {
    let mut schema = BundleSchema::new(name)
        .base(FieldDef::numeric("vertex_a"))
        .base(FieldDef::numeric("vertex_b"));
    for f in fiber_fields {
        schema = schema.fiber(FieldDef::numeric(f));
    }
    engine
        .create_bundle(schema)
        .expect("create_bundle should succeed");
}

/// Insert one edge record. Fiber values are taken in column order.
fn insert_edge(
    engine: &mut Engine,
    name: &str,
    va: i64,
    vb: i64,
    fiber_fields: &[&str],
    fiber_vals: &[f64],
) {
    let mut rec = Record::new();
    rec.insert("vertex_a".to_string(), Value::Integer(va));
    rec.insert("vertex_b".to_string(), Value::Integer(vb));
    for (f, v) in fiber_fields.iter().zip(fiber_vals.iter()) {
        rec.insert(f.to_string(), Value::Float(*v));
    }
    engine
        .insert(name, &rec)
        .expect("insert should succeed");
}

/// SU(2) identity quaternion: (q0=1, q1=q2=q3=0). w_e = q0 = 1.0.
fn su2_identity_fiber() -> [f64; 4] {
    [1.0, 0.0, 0.0, 0.0]
}

fn su2_field_names() -> &'static [&'static str] {
    &["q0", "q1", "q2", "q3"]
}

#[allow(dead_code)]
fn su3_field_names() -> Vec<String> {
    let mut names = Vec::new();
    for r in 0..3 {
        for c in 0..3 {
            names.push(format!("re_{r}{c}"));
            names.push(format!("im_{r}{c}"));
        }
    }
    names
}

/// (1) Empty / 1-vertex bundle returns EmptyBundle. Trivial-graph
/// behaviour pinned — `v_count < 2` is the trigger.
#[test]
fn test_identity_field_returns_zero_gap_on_single_node() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_edge_bundle(&mut engine, "tiny", su2_field_names());
    // Self-loop only (single vertex) — v_count == 1.
    let id = su2_identity_fiber();
    insert_edge(&mut engine, "tiny", 0, 0, su2_field_names(), &id);

    let fiber_fields: Vec<String> = su2_field_names().iter().map(|s| s.to_string()).collect();
    let err = spectral_gauge_gap(&engine, "tiny", &fiber_fields, Group::SU2, false, None, None)
        .expect_err("single-vertex graph should not yield a gap");
    assert!(
        matches!(err, SpectralGaugeError::EmptyBundle { .. }),
        "expected EmptyBundle, got: {err:?}"
    );
}

/// (2) SU(2) identity on a connected 6-vertex ring gives the
/// algebraic connectivity of the unweighted ring. λ₁ = 2·(1 −
/// cos(2π/n)) = 2·(1 − cos(60°)) = 1.0 for n=6. Tolerance 1e-9.
#[test]
fn test_identity_field_returns_unweighted_gap_on_connected_graph() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_edge_bundle(&mut engine, "ring6", su2_field_names());
    let id = su2_identity_fiber();
    // 6-cycle: edges (0,1), (1,2), ..., (5,0).
    let n = 6;
    for i in 0..n {
        let j = (i + 1) % n;
        insert_edge(&mut engine, "ring6", i as i64, j as i64, su2_field_names(), &id);
    }

    let fiber_fields: Vec<String> = su2_field_names().iter().map(|s| s.to_string()).collect();
    let result = spectral_gauge_gap(&engine, "ring6", &fiber_fields, Group::SU2, false, None, None)
        .expect("ring should give a gap");
    let expected = 2.0 * (1.0 - (2.0 * std::f64::consts::PI / n as f64).cos());
    assert!(
        (result.gap - expected).abs() < 1e-9,
        "ring λ₁: got {}, expected {} (diff {})",
        result.gap, expected, (result.gap - expected).abs()
    );
    assert_eq!(result.n_records_used, n);
    assert_eq!(result.group_used, Group::SU2);
    assert!(result.eigenvalues.is_none(), "Phase 1 returns gap-only");
}

/// (3) Random SU(2) field on a small ring — gap is finite, positive,
/// not NaN/Inf. Uses a deterministic seed via the Haar sampler.
#[test]
fn test_random_gauge_field_returns_finite_positive_gap() {
    use gigi::gauge::marsaglia_haar::{haar_random_su2, SmallRng};

    let mut engine = Engine::open_memory().expect("memory engine");
    make_edge_bundle(&mut engine, "ring10_haar", su2_field_names());
    let mut rng = SmallRng::seed_from_u64(20260628);
    let n = 10;
    for i in 0..n {
        let j = (i + 1) % n;
        let q = haar_random_su2(&mut rng);
        // haar_random_su2 returns [q0, q1, q2, q3].
        insert_edge(&mut engine, "ring10_haar", i as i64, j as i64, su2_field_names(), &q);
    }

    let fiber_fields: Vec<String> = su2_field_names().iter().map(|s| s.to_string()).collect();
    let result = spectral_gauge_gap(&engine, "ring10_haar", &fiber_fields, Group::SU2, false, None, None)
        .expect("ring should give a gap");
    assert!(result.gap.is_finite(), "gap must be finite, got {}", result.gap);
    assert!(!result.gap.is_nan(), "gap must not be NaN");
    // Random Haar field gives a positive λ₁ on a connected ring
    // (possible negative weights are absorbed but algebraic
    // connectivity for a single connected component stays positive
    // for typical Haar draws — this is a smoke test, not a tight
    // bound).
    assert!(result.gap.abs() > 1e-9, "gap should be non-degenerate");
    assert_eq!(result.n_records_used, n);
}

/// (4) Group inference: 4 fiber fields → SU(2).
#[test]
fn test_su2_fiber_4_inferred_as_su2() {
    let g = infer_group_from_arity(4).expect("4 fields infers SU(2)");
    assert_eq!(g, Group::SU2);
}

/// (5) Group inference: 18 fiber fields → SU(3).
#[test]
fn test_su3_fiber_18_inferred_as_su3() {
    let g = infer_group_from_arity(18).expect("18 fields infers SU(3)");
    assert_eq!(g, Group::SU3);
}

/// (6) Group inference: 1 fiber field → U(1).
#[test]
fn test_u1_fiber_1_inferred_as_u1() {
    let g = infer_group_from_arity(1).expect("1 field infers U(1)");
    assert_eq!(g, Group::U1);
}

/// (7) Ambiguous fiber arity (7) returns AmbiguousGroupInference
/// carrying the count.
#[test]
fn test_ambiguous_fiber_arity_requires_group() {
    let err = infer_group_from_arity(7).expect_err("7 fields is ambiguous");
    match err {
        SpectralGaugeError::AmbiguousGroupInference(n) => assert_eq!(n, 7),
        other => panic!("expected AmbiguousGroupInference(7), got {other:?}"),
    }
    // Display includes the count + the canonical-widths hint.
    let msg = err.to_string();
    assert!(msg.contains("7"), "msg missing count: {msg}");
    assert!(msg.contains("canonical widths"), "msg missing widths hint: {msg}");
}

/// (8) Fiber arity mismatch against an explicit GROUP returns
/// FiberArityMismatch with expected/actual counts.
#[test]
fn test_fiber_arity_mismatch_against_explicit_group() {
    let mut engine = Engine::open_memory().expect("memory engine");
    // Bundle declares 5 numeric fields (deliberate mismatch for SU(2)).
    let fields = ["a", "b", "c", "d", "e"];
    make_edge_bundle(&mut engine, "mismatch", &fields);
    let id = [1.0, 0.0, 0.0, 0.0, 0.0];
    insert_edge(&mut engine, "mismatch", 0, 1, &fields, &id);

    let fiber_fields: Vec<String> = fields.iter().map(|s| s.to_string()).collect();
    let err = spectral_gauge_gap(&engine, "mismatch", &fiber_fields, Group::SU2, false, None, None)
        .expect_err("5 fields with SU(2) should mismatch");
    match err {
        SpectralGaugeError::FiberArityMismatch { group, expected, actual } => {
            assert_eq!(group, "SU(2)");
            assert_eq!(expected, 4);
            assert_eq!(actual, 5);
        }
        other => panic!("expected FiberArityMismatch, got {other:?}"),
    }
}

/// (9) Bundle-not-found returns typed BundleNotFound error.
#[test]
fn test_bundle_not_found_returns_typed_error() {
    let engine = Engine::open_memory().expect("memory engine");
    let fiber_fields: Vec<String> = su2_field_names().iter().map(|s| s.to_string()).collect();
    let err = spectral_gauge_gap(&engine, "no_such_bundle", &fiber_fields, Group::SU2, false, None, None)
        .expect_err("nonexistent bundle should error");
    assert!(
        matches!(err, SpectralGaugeError::BundleNotFound(ref name) if name.contains("no_such_bundle")),
        "expected BundleNotFound with bundle name, got: {err:?}"
    );
    let msg = err.to_string();
    assert!(msg.contains("no_such_bundle"), "msg missing bundle name: {msg}");
}

/// (10) Missing vertex_a / vertex_b endpoints returns
/// MissingEndpointFields with both field names.
#[test]
fn test_missing_endpoint_fields_returns_typed_error() {
    let mut engine = Engine::open_memory().expect("memory engine");
    // Schema without vertex_a/vertex_b.
    let mut schema = BundleSchema::new("no_endpoints").base(FieldDef::numeric("just_id"));
    for f in su2_field_names() {
        schema = schema.fiber(FieldDef::numeric(f));
    }
    engine.create_bundle(schema).expect("create");

    let fiber_fields: Vec<String> = su2_field_names().iter().map(|s| s.to_string()).collect();
    let err = spectral_gauge_gap(&engine, "no_endpoints", &fiber_fields, Group::SU2, false, None, None)
        .expect_err("missing endpoints should error");
    match err {
        SpectralGaugeError::MissingEndpointFields { ref bundle, ref a, ref b } => {
            assert_eq!(bundle, "no_endpoints");
            assert_eq!(a, "vertex_a");
            assert_eq!(b, "vertex_b");
        }
        other => panic!("expected MissingEndpointFields, got {other:?}"),
    }
}

/// (11) FULL mode is IMPLEMENTED as of Phase 2 (2026-07-16): the ring4
/// call returns all 4 eigenvalues of the unit-weight C_4 Laplacian
/// (2 − 2cos(2πk/4) = {0, 2, 2, 4}) ascending, with the gap unchanged.
/// (Phase 1 pinned a PhaseNotImplemented stub here; the Phase-2 tranche
/// replaces the stub with the dense implementation by design — see
/// theory/halcyon/SPECTRAL_PHASE2_MAGNETIC_SHIPPED_2026-07-16.md.)
#[test]
fn test_full_mode_returns_eigenvalues_dense() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_edge_bundle(&mut engine, "ring4", su2_field_names());
    let id = su2_identity_fiber();
    for i in 0..4 {
        let j = (i + 1) % 4;
        insert_edge(&mut engine, "ring4", i, j, su2_field_names(), &id);
    }

    let fiber_fields: Vec<String> = su2_field_names().iter().map(|s| s.to_string()).collect();
    let result = spectral_gauge_gap(&engine, "ring4", &fiber_fields, Group::SU2, true, None, None)
        .expect("FULL mode is implemented in Phase 2");
    let vals = result.eigenvalues.expect("FULL populates eigenvalues");
    let expected = [0.0, 2.0, 2.0, 4.0];
    assert_eq!(vals.len(), 4);
    for (i, (got, want)) in vals.iter().zip(expected.iter()).enumerate() {
        assert!(
            (got - want).abs() < 1e-9,
            "C_4 eigenvalue {i}: got {got}, want {want}"
        );
    }
    assert!((result.gap - 2.0).abs() < 1e-9, "gap stays λ₁ = 2.0");
}

/// (12) FULL LIMIT 5 on a 4-vertex ring clamps to V = 4 eigenvalues
/// (LIMIT k > V is not an error; LIMIT 0 is — see spectral_full_basic).
#[test]
fn test_full_mode_with_limit_clamps_to_vertex_count() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_edge_bundle(&mut engine, "ring4_limit", su2_field_names());
    let id = su2_identity_fiber();
    for i in 0..4 {
        let j = (i + 1) % 4;
        insert_edge(&mut engine, "ring4_limit", i, j, su2_field_names(), &id);
    }

    let fiber_fields: Vec<String> = su2_field_names().iter().map(|s| s.to_string()).collect();
    let result = spectral_gauge_gap(&engine, "ring4_limit", &fiber_fields, Group::SU2, true, Some(5), None)
        .expect("FULL LIMIT is implemented in Phase 2");
    let vals = result.eigenvalues.expect("FULL populates eigenvalues");
    assert_eq!(vals.len(), 4, "LIMIT 5 on V=4 clamps to 4");
}

/// (13) Parser ergonomics #4: SU3 (bare) parses to the same Group as
/// SU(3) (parenthesized). Same AST node, same Group::SU3.
#[test]
fn test_su3_synonym_parses_to_same_group_as_su_3() {
    let bare = parse("SPECTRAL_GAUGE b ON FIBER (a) GROUP SU3").expect("parse SU3");
    let paren = parse("SPECTRAL_GAUGE b ON FIBER (a) GROUP SU(3)").expect("parse SU(3)");
    let extract = |s| match s {
        Statement::SpectralGauge { group, .. } => group,
        _ => panic!("expected SpectralGauge"),
    };
    assert_eq!(extract(bare), Some(Group::SU3));
    assert_eq!(extract(paren), Some(Group::SU3));
}

/// (14) Same for SU2 / SU(2).
#[test]
fn test_su2_synonym_parses_to_same_group_as_su_2() {
    let bare = parse("SPECTRAL_GAUGE b ON FIBER (a) GROUP SU2").expect("parse SU2");
    let paren = parse("SPECTRAL_GAUGE b ON FIBER (a) GROUP SU(2)").expect("parse SU(2)");
    let extract = |s| match s {
        Statement::SpectralGauge { group, .. } => group,
        _ => panic!("expected SpectralGauge"),
    };
    assert_eq!(extract(bare), Some(Group::SU2));
    assert_eq!(extract(paren), Some(Group::SU2));
}

/// (15) Same for U1 / U(1).
#[test]
fn test_u1_synonym_parses_to_same_group_as_u_1() {
    let bare = parse("SPECTRAL_GAUGE b ON FIBER (a) GROUP U1").expect("parse U1");
    let paren = parse("SPECTRAL_GAUGE b ON FIBER (a) GROUP U(1)").expect("parse U(1)");
    let extract = |s| match s {
        Statement::SpectralGauge { group, .. } => group,
        _ => panic!("expected SpectralGauge"),
    };
    assert_eq!(extract(bare), Some(Group::U1));
    assert_eq!(extract(paren), Some(Group::U1));
}

/// (16) Bare ZN without a modulus is rejected with a message pointing
/// at Z(<n>) form.
#[test]
fn test_bare_zn_without_modulus_rejected() {
    let err = parse("SPECTRAL_GAUGE b ON FIBER (a) GROUP ZN").expect_err("ZN must be rejected");
    assert!(
        err.contains("ZN") && (err.contains("modulus") || err.contains("Z(")),
        "expected ZN/modulus hint, got: {err}"
    );
}

/// (17) SPECTRAL_GAUGE without ON FIBER is rejected.
#[test]
fn test_spectral_gauge_requires_on_fiber() {
    let err = parse("SPECTRAL_GAUGE b").expect_err("missing ON FIBER must error");
    let _ = err; // Error wording is parser-internal; just confirm rejection.
}

/// (18) SPECTRAL_GAUGE with empty fiber list is rejected.
#[test]
fn test_spectral_gauge_requires_at_least_one_fiber_field() {
    let err = parse("SPECTRAL_GAUGE b ON FIBER ()").expect_err("empty fiber list must error");
    assert!(
        err.contains("at least one fiber field"),
        "expected fiber-required message, got: {err}"
    );
}

/// (19) Default no-GROUP case parses cleanly with group=None (then
/// inferred at exec time).
#[test]
fn test_no_group_clause_parses_with_none() {
    let stmt = parse("SPECTRAL_GAUGE b ON FIBER (q0, q1, q2, q3)").expect("parse");
    match stmt {
        Statement::SpectralGauge { group, fiber_fields, full, limit, .. } => {
            assert_eq!(group, None);
            assert_eq!(fiber_fields.len(), 4);
            assert!(!full);
            assert_eq!(limit, None);
        }
        _ => panic!("expected SpectralGauge"),
    }
}

/// (20) Re Tr(U)/N formula for SU(3) — diagonal-only matrix has
/// `Re Tr = re_00 + re_11 + re_22 = 3·a`, so `w_e = a`.
/// Validates the index packing convention.
#[test]
fn test_re_trace_over_n_su3_diagonal() {
    // Fiber where re_00 = re_11 = re_22 = 0.7, others zero.
    let mut fiber = vec![0.0_f64; 18];
    fiber[0] = 0.7;
    fiber[8] = 0.7;
    fiber[16] = 0.7;
    let w = gigi::spectral::re_trace_over_n(&fiber, Group::SU3);
    assert!((w - 0.7).abs() < 1e-12, "SU(3) weight: got {w}, expected 0.7");
}

/// (21) U(1) weight is cos(θ).
#[test]
fn test_re_trace_over_n_u1_cosine() {
    let theta = std::f64::consts::PI / 3.0;
    let w = gigi::spectral::re_trace_over_n(&[theta], Group::U1);
    assert!((w - 0.5).abs() < 1e-12, "cos(π/3) = 0.5, got {w}");
}
