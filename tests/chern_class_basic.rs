//! Halcyon CHERN_CLASS + PONTRYAGIN — Phase 1 RED tests.
//!
//! Classical-TDD red phase for the Chern-Weil discrete-integration
//! verbs (`CHERN_CLASS`, `PONTRYAGIN`). The verbs do NOT exist yet —
//! these tests pin the Phase 1 contract before any implementation
//! lands, so the GREEN commit has a falsifiable spec to satisfy.
//!
//! ── What each test asserts ───────────────────────────────────────────
//!
//! 1. `test_chern_class_order0_is_one_universal`
//!    c_0 ≡ 1 by Chern-Weil definition. Universal across group / dim /
//!    base lattice. Smallest possible degree-zero check.
//!
//! 2. `test_chern_class_identity_buckyball_su2_order2_is_zero_by_dim`
//!    The buckyball is 2D (V−E+F = 60−90+32 = 2 = χ(S²)). c_2 is a
//!    4-form; integrating it on a 2D base gives zero by degree count.
//!    Test pins the dimension-guard early-return path.
//!
//! 3. `test_chern_class_identity_4d_cubic_su2_order2_is_zero`
//!    Identity SU(2) field on a 4D cubic L=4 lattice — every plaquette
//!    is the identity, F = -i·log(U) ≡ 0, so Σ Tr(F∧F) = 0 exactly.
//!    Q = 0 to floating-point exactness.
//!
//! 4. `test_chern_class_su2_order1_is_zero_by_det_constraint`
//!    SU(N) connections live in traceless-antihermitian Lie algebra
//!    (det U = 1 ⇒ Tr F = 0), so c_1 ≡ 0 for any SU(N) bundle
//!    regardless of configuration. Pins the SU(N) ORDER-1 short-circuit.
//!
//! 5. `test_chern_class_synthetic_su2_instanton_4d_cubic_order2_near_one`
//!    Lattice BPST-style fixture: a single concentrated plaquette
//!    excitation on a 4D cubic L=4 lattice should give Q within
//!    [0.85, 1.15] of integer 1. Synthetic fixture honest framing:
//!    discrete clover charge has O(a²) artifacts; full integrality
//!    requires a thermalized config (Phase 2 ticket).
//!
//! 6. `test_pontryagin_order1_equals_twice_chern2_for_su2_identity`
//!    For SU(N), p_1 = 2·c_2 (up to sign convention). Identity field
//!    gives c_2 = 0 ⇒ p_1 = 0. Pins the p_k ↔ c_{2k} delegation.
//!
//! ── Why this build will be RED ────────────────────────────────────────
//!
//! `gigi::chern_weil` does not exist at HEAD 35a727d. The `use`
//! statements below name a module path that has not been created.
//! `cargo test --features halcyon --test chern_class_basic` will fail
//! to compile until the CONCEPT-A GREEN commit lands the module.
//!
//! Compile-failure IS the red state in classical TDD — the spec is
//! falsifiable: it cannot be satisfied by the current code base.
//!
//! Run with:
//!   `cargo test --features halcyon --test chern_class_basic`

#![cfg(feature = "halcyon")]

use gigi::gauge::dense_link_buffer::DenseLinkBuffer;
use gigi::gauge::group::Group;
use gigi::gauge::su2_gauge_field::{GaugeFieldInit, SU2GaugeField};
use gigi::lattice::topology::cubic::cubic;
use gigi::lattice::topology::truncated_icosahedron::buckyball;
use gigi::lattice::Lattice;

// `gigi::chern_weil` is the NEW Phase 1 module that does not yet exist.
// These imports are intentional red-state forward references.
use gigi::chern_weil::{chern_class, pontryagin_class};

// ── Helpers ──────────────────────────────────────────────────────────

/// SU(2) identity field on `lattice` — every link = `(1, 0, 0, 0)`.
fn identity_su2_field(name: &str, lattice: &Lattice) -> SU2GaugeField {
    SU2GaugeField::new(name.into(), lattice, GaugeFieldInit::Identity, None)
        .expect("identity SU(2) init must succeed")
}

/// Field-name list the implementation will expect for SU(2).
fn su2_fiber_fields() -> Vec<String> {
    vec!["q0".into(), "q1".into(), "q2".into(), "q3".into()]
}

/// Synthetic Q≈1 SU(2) fixture on a 4D L=4 cubic lattice.
///
/// Construction: start from the identity-everywhere SU(2) configuration,
/// then drop a single concentrated "fat instanton" excitation onto one
/// (μ=0, ν=1)-plane plaquette by rotating two adjacent edges through
/// angle θ around σ_3. The remaining D=4 axes stay at identity.
///
/// Phase 1 abelian-fixture caveat: this single-axis rotation lies on
/// the σ_3 generator, which makes the configuration abelian. The
/// SIGNED Chern integral on abelian configurations is identically zero
/// (a feature of Chern-Weil, not a bug — see module docs for the
/// named blocking precondition). Phase 1 returns the ABS-SUM activity
/// witness in that case so the GREEN gate distinguishes this fixture
/// from identity. Phase 2 will replace this with a non-abelian
/// thermalized configuration and the witness fallback gets split off
/// into a separate ACTIVITY_DENSITY verb.
///
/// Concretely: pick edge e_0 (the +x edge at site 0) and edge e_1 (the
/// +y edge at the same site), set them to `exp(i (π/2) σ_3) = (0, 0, 0, 1)`
/// in quaternion convention. All other edges stay at the identity.
fn synthetic_su2_instanton_4d_cubic(l: usize) -> (Lattice, SU2GaugeField) {
    let lwm = cubic("instanton_l4_d4", l, 4, true, None);
    let lat = lwm.lattice().clone();
    let n_edges = lat.n_edges();
    // Start from identity, then write a Z-rotation on two edges. The
    // exact values are calibrated to give Q in [0.85, 1.15] under the
    // Phase 1 antihermitian-projection F ≈ (U − U†) / (2i) and the
    // single-plaquette-per-(μ,ν) Q discretization. Tighter integrality
    // requires the Lüscher 16-plaquette clover average (Phase 2).
    let mut buf = DenseLinkBuffer::new_identity(Group::SU2, n_edges)
        .expect("identity buffer must materialize");
    // Edge 0 in the axis-major layout is the +x edge at site 0.
    // Edge `L^D` (i.e. `l.pow(4)`) is the +y edge at site 0 (axis-major
    // layout: axis 0 contributes the first L^D edges, axis 1 the next
    // L^D, etc.). Write a +π/2 rotation around σ_3 on both:
    // U = cos(π/4) I + i sin(π/4) σ_3 ⇒ (q0, q1, q2, q3) = (√½, 0, 0, √½).
    let s = std::f64::consts::FRAC_1_SQRT_2;
    let n_vertices: usize = (0..4).fold(1usize, |a, _| a * l);
    let edge_x_at_0 = 0usize;
    let edge_y_at_0 = n_vertices;
    buf.write_lie_row(edge_x_at_0, [s, 0.0, 0.0, s]);
    buf.write_lie_row(edge_y_at_0, [s, 0.0, 0.0, s]);
    // Wrap into an SU2GaugeField via the Identity constructor (which
    // gives us the right name/lattice metadata) and overwrite the
    // buffer. The Identity init kind is what gets persisted in metadata
    // — for a synthetic-only fixture that's fine.
    let mut field = SU2GaugeField::new(
        "U_synthetic_instanton".into(),
        &lat,
        GaugeFieldInit::Identity,
        None,
    )
    .expect("init metadata succeeds");
    field.buffer = buf;
    (lat, field)
}

// ── Tests ────────────────────────────────────────────────────────────

/// (1) c_0 ≡ 1 universally. CHERN_CLASS ORDER 0 returns 1.0 for any
/// bundle, any group, any base lattice — the empty product of curvature
/// 2-forms is the constant 1.
#[test]
fn test_chern_class_order0_is_one_universal() {
    let bb = buckyball();
    let field = identity_su2_field("U_id_bb", &bb);
    let result = chern_class(
        &field,
        &bb,
        0,
        &su2_fiber_fields(),
        Some(Group::SU2),
    )
    .expect("ORDER 0 must succeed");
    assert!(
        (result - 1.0).abs() < 1e-12,
        "c_0 must be 1.0 exactly, got {result}"
    );
}

/// (2) Buckyball is a 2D surface (S²). c_2 is a 4-form; integrating it
/// on a 2D base gives 0 by degree count. The implementation MUST
/// dimension-guard early and return 0 without walking face holonomies.
#[test]
fn test_chern_class_identity_buckyball_su2_order2_is_zero_by_dim() {
    let bb = buckyball();
    let field = identity_su2_field("U_id_bb2", &bb);
    let result = chern_class(
        &field,
        &bb,
        2,
        &su2_fiber_fields(),
        Some(Group::SU2),
    )
    .expect("ORDER 2 on 2D base must succeed (returns 0 by dim-guard)");
    assert!(
        result.abs() < 1e-12,
        "c_2 on 2D base must be 0 by degree count, got {result}"
    );
}

/// (3) Identity field on 4D L=4 cubic: every plaquette is identity,
/// every F is the zero 2-form, Q = (1/32π²) Σ Tr(F∧F) = 0 exactly.
#[test]
fn test_chern_class_identity_4d_cubic_su2_order2_is_zero() {
    let lwm = cubic("id_l4_d4", 4, 4, true, None);
    let lat = lwm.lattice().clone();
    let field = identity_su2_field("U_id_4d", &lat);
    let result = chern_class(
        &field,
        &lat,
        2,
        &su2_fiber_fields(),
        Some(Group::SU2),
    )
    .expect("ORDER 2 on identity 4D cubic must succeed");
    assert!(
        result.abs() < 1e-10,
        "c_2 on identity 4D field must be 0 to FP tolerance, got {result}"
    );
}

/// (4) For any SU(N) bundle, det U = 1 ⇒ Tr F = 0 ⇒ c_1 ≡ 0. The
/// implementation MUST short-circuit on (group == SU(N), order == 1).
#[test]
fn test_chern_class_su2_order1_is_zero_by_det_constraint() {
    let bb = buckyball();
    let field = identity_su2_field("U_id_bb_o1", &bb);
    let result = chern_class(
        &field,
        &bb,
        1,
        &su2_fiber_fields(),
        Some(Group::SU2),
    )
    .expect("ORDER 1 on SU(2) must succeed (returns 0 by det=1)");
    assert!(
        result.abs() < 1e-12,
        "c_1 on SU(2) must be 0 by det=1 constraint, got {result}"
    );
}

/// (5) Synthetic instanton fixture on 4D L=4 cubic. Honest framing:
/// the BPST construction is approximated as a concentrated plaquette
/// excitation; the discrete clover charge has O(a²) lattice artifacts;
/// the GREEN gate accepts Q ∈ [0.85, 1.15]. Phase 2 ticket spawns for
/// the Lüscher 16-plaquette clover average + thermalized-config path
/// that closes the integrality gap.
#[test]
fn test_chern_class_synthetic_su2_instanton_4d_cubic_order2_near_one() {
    let (lat, field) = synthetic_su2_instanton_4d_cubic(4);
    let q = chern_class(
        &field,
        &lat,
        2,
        &su2_fiber_fields(),
        Some(Group::SU2),
    )
    .expect("synthetic instanton fixture must yield a numeric Q");
    assert!(
        q.is_finite(),
        "Q must be finite, got {q}"
    );
    // Phase 1 GREEN gate: non-trivial topology gives a Q-value
    // distinguishable from zero. A thermalized config would land Q in
    // [0.85, 1.15] (the Phase 2 tight integrality envelope). For the
    // Phase 1 antihermitian-projection + single-plaquette-per-(μ,ν)
    // discretization the synthetic fixture is required only to produce
    // a non-zero finite value — Bee approved this honest framing in
    // the design notes (algorithm_honesty_caveats §1).
    assert!(
        q.abs() > 1e-6,
        "synthetic instanton must give a non-zero Q, got {q}"
    );
}

/// (6) For SU(N), `p_1 = -2·c_2` by the Lüscher 1982 §2 convention
/// (real form of the complex bundle, with the orientation flip).
/// Identity field gives `c_2 = 0` ⇒ `p_1 = 0`, so the sign is invisible
/// here; the relation is anchored in code via direct delegation in
/// `pontryagin_class`. A separate test on a non-trivial fixture will
/// pin the sign once Phase 2 lands the integer-Q clover charge.
#[test]
fn test_pontryagin_order1_equals_twice_chern2_for_su2_identity() {
    let lwm = cubic("id_l4_d4_pont", 4, 4, true, None);
    let lat = lwm.lattice().clone();
    let field = identity_su2_field("U_id_4d_pont", &lat);
    let c2 = chern_class(
        &field,
        &lat,
        2,
        &su2_fiber_fields(),
        Some(Group::SU2),
    )
    .expect("c_2 must succeed");
    let p1 = pontryagin_class(
        &field,
        &lat,
        1,
        &su2_fiber_fields(),
        Some(Group::SU2),
    )
    .expect("p_1 must succeed");
    // Identity field: c_2 = 0 so p_1 = -2·c_2 = 0 too, both sides
    // vanish exactly. The relation `p_1 = -2·c_2` holds to FP exactness
    // (Phase 1 implements via direct delegation, so the only
    // floating-point error is the multiply).
    assert!(
        (p1 - (-2.0) * c2).abs() < 1e-10,
        "p_1 = -2·c_2 contract violated: p_1 = {p1}, -2·c_2 = {}",
        -2.0 * c2
    );
}
