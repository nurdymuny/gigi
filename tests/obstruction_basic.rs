//! Yang-Mills topology Phase 1 — OBSTRUCTION verb basic-correctness
//! gates (RED, 2026-06-29).
//!
//! These tests express the spec for the principal-bundle section
//! obstruction:
//!
//!   - On a closed 2-manifold, every SU(N>=2) bundle is trivial, so
//!     the obstruction vacuously vanishes — `has_obstruction == false`,
//!     `class == 0`, `kind == "trivial_2d_su_n"`.
//!   - On a closed 4-manifold with SU(N), the obstruction class is
//!     the integrated `c_2` (a.k.a. the instanton number Q). The
//!     identity gauge field gives Q = 0; a synthetic single-instanton
//!     configuration gives Q = 1.
//!   - On a closed 2-manifold with U(1), the obstruction class is
//!     the integrated `c_1` (the monopole / first-Chern integer). A
//!     winding-2 U(1) configuration gives `class == 2`.
//!
//! RED expectation: every test in this file FAILS at runtime by
//! panicking inside the `unimplemented!()` body of
//! `gigi::obstruction::obstruction`. The point of the RED commit is
//! to lock the public-API surface (function name, argument list,
//! result shape) BEFORE filling in the math. GREEN commit replaces
//! the stub bodies once `crate::chern_weil::chern_class` ships.
//!
//! Run with:
//!   `cargo test --features halcyon --test obstruction_basic`

#![cfg(feature = "halcyon")]

use gigi::engine::Engine;
use gigi::obstruction::{obstruction, obstruction_with_default, ObstructionKind};
use gigi::types::{BundleSchema, FieldDef, Record, Value};

// ─── fixture helpers ─────────────────────────────────────────────────

/// Create a gauge-link bundle with `(vertex_a, vertex_b)` base + the
/// listed numeric fiber fields. Same shape as the
/// `spectral_gauge_basic.rs` harness so the OBSTRUCTION fixtures are
/// recognizable to anyone who's read that test file.
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
    engine.insert(name, &rec).expect("insert should succeed");
}

fn su2_field_names() -> &'static [&'static str] {
    &["q0", "q1", "q2", "q3"]
}

/// SU(2) identity quaternion `(1, 0, 0, 0)`.
fn su2_identity() -> [f64; 4] {
    [1.0, 0.0, 0.0, 0.0]
}

/// Seed a 2D buckyball-like SU(2) edge bundle that the executor will
/// associate with the buckyball lattice (topology hint S²). All links
/// are the SU(2) identity, so any sensible Chern-Weil reduction must
/// return `Q = 0`.
fn seed_buckyball_su2_identity(engine: &mut Engine, name: &str) {
    make_edge_bundle(engine, name, su2_field_names());
    // 90-edge buckyball; we only need a representative subset for the
    // RED phase. The GREEN executor reads E from the associated
    // lattice; the test asserts the spec contract, not a Monte-Carlo
    // distribution. Insert 30 edges as smoke (enough to be non-empty).
    let id = su2_identity();
    for e in 0..30_i64 {
        insert_edge(engine, name, e, e + 1, su2_field_names(), &id);
    }
}

/// Seed a 4D cubic SU(2) edge bundle (Halcyon §3.3 substrate shape,
/// scaled down to L=4 for test speed) with the SU(2) identity on every
/// link. `c_2 = 0` because the curvature vanishes identically.
fn seed_4d_cubic_su2_identity(engine: &mut Engine, name: &str) {
    make_edge_bundle(engine, name, su2_field_names());
    // L=4, D=4 → V=256, E=1024 in a real cubic lattice; we only need
    // enough edges to exercise the executor's loop. The exec layer
    // will look up the associated 4D cubic lattice for the face/cell
    // count when it ships.
    let id = su2_identity();
    for e in 0..64_i64 {
        insert_edge(engine, name, e, e + 1, su2_field_names(), &id);
    }
}

/// Seed a 4D cubic SU(2) edge bundle stamped with a *synthetic*
/// single-instanton configuration: a hand-tuned set of plaquette
/// values whose Chern-Weil integral evaluates to `Q = 1` within
/// quantization tolerance. The RED phase does not validate the
/// numerical seed; it only locks the contract that `class == 1` is
/// the expected output for a properly cooled BPST-style configuration.
fn seed_4d_cubic_su2_single_instanton(engine: &mut Engine, name: &str) {
    make_edge_bundle(engine, name, su2_field_names());
    // Stand-in fiber values — the GREEN test re-stamps this from a
    // recorded JSON/CBOR fixture (cf. design notes
    // `tests/fixtures/single_instanton_su2_l4d4.cbor`). For RED, any
    // distinguishable seed suffices because the assertion lives in
    // `unimplemented!()` land.
    let off_axis = [0.9, 0.4, 0.1, 0.05]; // non-identity SU(2) draw
    for e in 0..64_i64 {
        insert_edge(engine, name, e, e + 1, su2_field_names(), &off_axis);
    }
}

/// Seed a 2D flat-torus U(1) edge bundle with a winding-2
/// configuration so the integrated `c_1` should round to `class = 2`.
fn seed_2d_torus_u1_winding_two(engine: &mut Engine, name: &str) {
    let fiber_fields = ["theta"];
    make_edge_bundle(engine, name, &fiber_fields);
    // L=4, D=2 → V=16, E=32. Stamp angles that wind twice around the
    // homology generator; the executor will detect the integrated
    // first-Chern integer.
    let n_edges = 32;
    for e in 0..n_edges as i64 {
        let theta =
            4.0 * std::f64::consts::PI * (e as f64) / (n_edges as f64); // winding 2
        insert_edge(engine, name, e, e + 1, &fiber_fields, &[theta]);
    }
}

/// Seed a 2D buckyball SU(3) edge bundle with the SU(3) identity on
/// every link. On a 2-manifold every SU(N>=2) bundle is trivial, so
/// the OBSTRUCTION result must be `has_obstruction == false`,
/// `class == 0`, `kind == "trivial_2d_su_n"`.
fn seed_buckyball_su3_identity(engine: &mut Engine, name: &str) {
    // SU(3) fiber: 18 floats per link in interleaved real/imag
    // row-major order. Identity has real diagonal at indices 0, 8, 16.
    let fiber_field_names: Vec<String> = {
        let mut v = Vec::new();
        for r in 0..3 {
            for c in 0..3 {
                v.push(format!("re_{r}{c}"));
                v.push(format!("im_{r}{c}"));
            }
        }
        v
    };
    let fields_ref: Vec<&str> = fiber_field_names.iter().map(|s| s.as_str()).collect();
    make_edge_bundle(engine, name, &fields_ref);
    let mut id = [0.0_f64; 18];
    id[0] = 1.0;
    id[8] = 1.0;
    id[16] = 1.0;
    for e in 0..30_i64 {
        insert_edge(engine, name, e, e + 1, &fields_ref, &id);
    }
}

// ─── the 5 RED tests ─────────────────────────────────────────────────

/// (1) On the buckyball (closed 2-sphere) with SU(2), every bundle is
/// trivial. The identity field MUST yield `class == 0` and
/// `has_obstruction == false`.
///
/// RED: panics inside the stub body of
/// `gigi::obstruction::obstruction`. GREEN replaces the stub.
#[test]
fn test_obstruction_identity_buckyball_su2_is_zero() {
    let mut engine = Engine::open_memory().expect("memory engine");
    seed_buckyball_su2_identity(&mut engine, "bb_su2_id");

    let result = obstruction_with_default(&engine, "bb_su2_id")
        .expect("buckyball SU(2) identity must produce an OBSTRUCTION result");
    assert_eq!(
        result.class, 0,
        "SU(2) identity on the buckyball must have class = 0 (trivial bundle)"
    );
    assert!(
        !result.has_obstruction,
        "trivial bundle must report has_obstruction = false"
    );
    assert!(
        result.witness.abs() < 1e-6,
        "identity witness must be ~0, got {}",
        result.witness
    );
}

/// (2) On the 4D cubic lattice with SU(2), the identity field has
/// vanishing curvature so the integrated `c_2` (instanton number) is
/// exactly 0 — `class == 0`, `has_obstruction == false`.
///
/// RED: panics inside the stub body of
/// `gigi::obstruction::obstruction`. GREEN replaces the stub.
#[test]
fn test_obstruction_identity_4d_cubic_su2_is_zero() {
    let mut engine = Engine::open_memory().expect("memory engine");
    seed_4d_cubic_su2_identity(&mut engine, "cubic_su2_id");

    let result = obstruction_with_default(&engine, "cubic_su2_id")
        .expect("identity SU(2) on the 4D cubic lattice must produce a result");
    assert_eq!(
        result.class, 0,
        "identity SU(2) on T^4 has c_2 = 0 (Q = 0)"
    );
    assert!(
        !result.has_obstruction,
        "Q = 0 sector must report has_obstruction = false"
    );
    // Default kind is SectionExistence; the labelled kind string
    // should reflect that path for SU(N) on a 4D base.
    assert!(
        result.kind.contains("principal_bundle_section_obstruction")
            || result.kind.contains("instanton_number")
            || result.kind.contains("trivial"),
        "kind should be the section-existence / instanton-number label, got {:?}",
        result.kind
    );
}

/// (3) A synthetic single-instanton SU(2) configuration on the 4D
/// cubic lattice has `Q = 1`. The OBSTRUCTION verb must report
/// `class == 1`, `has_obstruction == true`, and a witness within
/// quantization tolerance of 1.0.
///
/// RED: panics inside the stub body of
/// `gigi::obstruction::obstruction`. GREEN replaces the stub.
#[test]
fn test_obstruction_synthetic_instanton_4d_cubic_su2_class_one() {
    let mut engine = Engine::open_memory().expect("memory engine");
    seed_4d_cubic_su2_single_instanton(&mut engine, "cubic_su2_inst1");

    let result = obstruction(
        &engine,
        "cubic_su2_inst1",
        ObstructionKind::InstantonNumber,
    )
    .expect("synthetic single-instanton must produce a result");
    assert_eq!(
        result.class, 1,
        "single-instanton configuration must report class = 1"
    );
    assert!(
        result.has_obstruction,
        "non-trivial principal-bundle sector must report has_obstruction = true"
    );
    assert!(
        (result.witness - 1.0).abs() < 0.25,
        "witness must lie within quantization tolerance of 1.0, got {}",
        result.witness
    );
    assert!(
        result.kind.contains("instanton_number"),
        "kind = InstantonNumber must surface the instanton_number label, got {:?}",
        result.kind
    );
}

/// (4) A U(1) configuration on the 2D flat torus with winding-2 first
/// Chern integer reports `class == 2`. Section existence: U(1) is
/// abelian and a non-trivial first Chern integer obstructs a global
/// section.
///
/// RED: panics inside the stub body of
/// `gigi::obstruction::obstruction`. GREEN replaces the stub.
#[test]
fn test_obstruction_u1_monopole_charge_two() {
    let mut engine = Engine::open_memory().expect("memory engine");
    seed_2d_torus_u1_winding_two(&mut engine, "t2_u1_w2");

    let result = obstruction_with_default(&engine, "t2_u1_w2")
        .expect("winding-2 U(1) on T^2 must produce a result");
    assert_eq!(
        result.class, 2,
        "winding-2 U(1) on T^2 must report c_1 = 2"
    );
    assert!(
        result.has_obstruction,
        "non-zero first Chern integer must report has_obstruction = true"
    );
    assert!(
        result.kind.contains("u1_section_obstruction")
            || result.kind.contains("u1_monopole_charge"),
        "kind must surface the U(1) section/monopole label, got {:?}",
        result.kind
    );
}

/// (5) Every SU(N>=2) bundle on a closed 2-manifold is trivial — the
/// OBSTRUCTION must report `class == 0`, `has_obstruction == false`,
/// and the dedicated `"trivial_2d_su_n"` kind label regardless of N.
/// We exercise both N=2 (buckyball SU(2)) and N=3 (buckyball SU(3)).
///
/// RED: panics inside the stub body of
/// `gigi::obstruction::obstruction`. GREEN replaces the stub.
#[test]
fn test_obstruction_2d_su_n_always_trivial() {
    let mut engine = Engine::open_memory().expect("memory engine");
    seed_buckyball_su2_identity(&mut engine, "bb_su2_triv");
    seed_buckyball_su3_identity(&mut engine, "bb_su3_triv");

    let r2 = obstruction_with_default(&engine, "bb_su2_triv")
        .expect("buckyball SU(2) must produce a vacuous-trivial result");
    let r3 = obstruction_with_default(&engine, "bb_su3_triv")
        .expect("buckyball SU(3) must produce a vacuous-trivial result");

    for (label, r) in [("SU(2)", &r2), ("SU(3)", &r3)] {
        assert_eq!(
            r.class, 0,
            "{label} on closed 2-manifold must have class = 0"
        );
        assert!(
            !r.has_obstruction,
            "{label} on closed 2-manifold has no obstruction"
        );
        assert!(
            r.kind.contains("trivial_2d_su_n"),
            "{label} on closed 2-manifold must surface the trivial_2d_su_n label, \
             got {:?}",
            r.kind
        );
    }
}
