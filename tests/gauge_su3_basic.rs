//! Halcyon ITEM 3.1 Phase 1 — SU(3) basic-correctness gates.
//!
//! Integration-test cousin of the unit tests inside the gauge module.
//! Asserts:
//!
//! 1. SU(3) identity composes with itself to identity (FP64 exact).
//! 2. SU(3) inverse is conjugate transpose; `U · U† = I` within 1e-10.
//! 3. Haar-sampled SU(3) draws are unitary (`U U† = I` within 1e-12).
//! 4. Haar-sampled SU(3) draws have `|det(U) - 1| < 1e-10`.
//! 5. Cold-start (identity) SU(3) field on the buckyball gives
//!    plaquette = 1.0 exactly on every face.
//! 6. SU(3) buffer stride = 18 f64s per link (the locked Halcyon
//!    representation; 144 bytes per link).
//!
//! Run with:
//!   `cargo test --features halcyon --test gauge_su3_basic`

#![cfg(feature = "halcyon")]

use gigi::gauge::{
    plaquette_mean, plaquette_per_face,
    group_element::GroupElement,
    su3_gauge_field::SU3GaugeField,
    su2_gauge_field::GaugeFieldInit,
    DenseLinkBuffer, Group,
};
use gigi::gauge::registry::GaugeFieldHandle;
use gigi::gauge::registry as gauge_registry;
use gigi::lattice::registry as lattice_registry;
use gigi::lattice::topology::truncated_icosahedron::buckyball;

/// Each test calls this to lock the registry-test mutex and reset all
/// state — the gauge + lattice registries are process-singleton so
/// tests must serialize their setup/teardown.
fn fresh_env() {
    let _guard = gauge_registry::test_serial_lock();
    gauge_registry::clear();
    lattice_registry::clear();
}

/// (1) SU(3) identity composes with itself byte-identically.
#[test]
fn test_su3_identity_compose_yields_identity() {
    let i = GroupElement::su3_identity();
    let r = i.compose(&i);
    assert_eq!(r, i);
}

/// (2) U · U† = I to FP64 tolerance for a non-trivial diagonal SU(3)
/// element built from explicit phases (det = 1 by construction).
#[test]
fn test_su3_inverse_yields_identity() {
    // U = diag(e^{iα}, e^{iβ}, e^{-i(α+β)})  — det(U) = 1.
    let alpha = 0.55_f64;
    let beta = -0.27_f64;
    let gamma = -(alpha + beta);
    let mut m = [0.0_f64; 18];
    m[0] = alpha.cos();
    m[1] = alpha.sin();
    m[8] = beta.cos();
    m[9] = beta.sin();
    m[16] = gamma.cos();
    m[17] = gamma.sin();
    let u = GroupElement::SU3(m);
    let u_dag = u.inverse();
    let r = u.compose(&u_dag);
    let id = GroupElement::su3_identity();
    match (r, id) {
        (GroupElement::SU3(a), GroupElement::SU3(b)) => {
            for k in 0..18 {
                assert!(
                    (a[k] - b[k]).abs() < 1e-10,
                    "index {k}: |U U† − I| = {}",
                    (a[k] - b[k]).abs()
                );
            }
        }
        _ => panic!("expected SU3 variants"),
    }
}

/// (3) Three Haar samples — each U · U† ≈ I within 1e-12.
#[test]
fn test_su3_haar_sample_is_unitary() {
    use gigi::gauge::marsaglia_haar::{haar_random_su3, SmallRng};
    let mut rng = SmallRng::seed_from_u64(20260626);
    for sample in 0..3 {
        let m = haar_random_su3(&mut rng);
        let u = GroupElement::SU3(m);
        let u_dag = u.inverse();
        let r = u.compose(&u_dag);
        let id = GroupElement::su3_identity();
        match (r, id) {
            (GroupElement::SU3(a), GroupElement::SU3(b)) => {
                for k in 0..18 {
                    assert!(
                        (a[k] - b[k]).abs() < 1e-12,
                        "sample {sample}, idx {k}: |U U† − I| = {}",
                        (a[k] - b[k]).abs()
                    );
                }
            }
            _ => panic!("expected SU3 variants"),
        }
    }
}

/// (4) Three Haar samples — each |det(U) - 1| < 1e-10.
#[test]
fn test_su3_haar_sample_has_det_one() {
    use gigi::gauge::marsaglia_haar::{haar_random_su3, SmallRng};
    let mut rng = SmallRng::seed_from_u64(20260626);
    for sample in 0..3 {
        let m = haar_random_su3(&mut rng);
        // Inline det via Laplace expansion on row 0.
        let cmul = |x: (f64, f64), y: (f64, f64)| -> (f64, f64) {
            (x.0 * y.0 - x.1 * y.1, x.0 * y.1 + x.1 * y.0)
        };
        let csub = |x: (f64, f64), y: (f64, f64)| -> (f64, f64) {
            (x.0 - y.0, x.1 - y.1)
        };
        let cadd = |x: (f64, f64), y: (f64, f64)| -> (f64, f64) {
            (x.0 + y.0, x.1 + y.1)
        };
        let a00 = (m[0], m[1]);
        let a01 = (m[2], m[3]);
        let a02 = (m[4], m[5]);
        let a10 = (m[6], m[7]);
        let a11 = (m[8], m[9]);
        let a12 = (m[10], m[11]);
        let a20 = (m[12], m[13]);
        let a21 = (m[14], m[15]);
        let a22 = (m[16], m[17]);
        let m00 = csub(cmul(a11, a22), cmul(a12, a21));
        let m01 = csub(cmul(a10, a22), cmul(a12, a20));
        let m02 = csub(cmul(a10, a21), cmul(a11, a20));
        let det = cadd(csub(cmul(a00, m00), cmul(a01, m01)), cmul(a02, m02));
        let drift = ((det.0 - 1.0).powi(2) + det.1.powi(2)).sqrt();
        assert!(
            drift < 1e-10,
            "sample {sample}: |det(U) - 1| = {drift}"
        );
    }
}

/// (5) Cold-start (identity) SU(3) field on the buckyball: every face
/// plaquette is exactly 1.0 — composition of identity 3×3 complex
/// matrices is FP64-exact and Re Tr(I) / 3 = 1.0 exactly.
#[test]
fn test_su3_plaquette_identity_returns_one() {
    fresh_env();
    let bb = buckyball();
    lattice_registry::register(bb.clone());
    let field = SU3GaugeField::new(
        "U_su3_id_pl".into(),
        &bb,
        GaugeFieldInit::Identity,
        None,
    )
    .expect("identity init must succeed");
    gauge_registry::register_su3(field);
    let handle = gauge_registry::get("U_su3_id_pl").expect("just registered");
    let per = plaquette_per_face(handle.as_ref(), &bb).expect("SU(3) reduction");
    assert_eq!(per.len(), 32, "buckyball has F=32 faces");
    for (i, q) in per.iter().enumerate() {
        assert_eq!(*q, 1.0, "face {i}: expected 1.0 exactly, got {q}");
    }
    let mean = plaquette_mean(handle.as_ref(), &bb).expect("mean");
    assert_eq!(mean, 1.0, "mean over identity SU(3) field = 1.0 exactly");
}

/// (6) Buffer stride is 18 f64 per link (locked Halcyon ITEM 3.1
/// representation: 144 bytes per link).
#[test]
fn test_su3_buffer_stride_is_18_f64() {
    let n_edges = 90; // buckyball
    let buf = DenseLinkBuffer::new_identity(Group::SU3, n_edges).unwrap();
    assert_eq!(buf.group, Group::SU3);
    assert_eq!(buf.n_edges, n_edges);
    assert_eq!(buf.repr_dim, 18);
    assert_eq!(buf.data.len(), n_edges * 18);
    assert_eq!(buf.data.len() * 8, n_edges * 144); // 144 B per link
}

/// Sanity: registry round-trips an SU(3) field through the dyn read
/// surface — Arc<dyn GaugeFieldHandle> exposes the right group tag
/// and lattice binding.
#[test]
fn test_su3_registry_round_trip() {
    fresh_env();
    let bb = buckyball();
    lattice_registry::register(bb.clone());
    let field = SU3GaugeField::new(
        "U_su3_reg".into(),
        &bb,
        GaugeFieldInit::HaarRandom,
        Some(20260626),
    )
    .expect("haar init must succeed");
    gauge_registry::register_su3(field);
    let got = gauge_registry::get("U_su3_reg").expect("just registered");
    assert_eq!(got.name(), "U_su3_reg");
    assert_eq!(got.lattice_name(), bb.name);
    assert_eq!(got.group(), Group::SU3);
    let (kind, seed) = got.init_metadata();
    assert_eq!(kind, GaugeFieldInit::HaarRandom);
    assert_eq!(seed, Some(20260626));
    let buf = got.as_dense_buffer();
    assert_eq!(buf.n_edges, bb.n_edges());
    assert_eq!(buf.repr_dim, 18);

    // get_su3_mut also resolves.
    let mut_handle =
        gauge_registry::get_su3_mut("U_su3_reg").expect("SU(3)-mut handle present");
    let guard = mut_handle.lock().expect("lock");
    assert_eq!(guard.name, "U_su3_reg");
}

/// Parser ergonomics #4 (2026-06-28): the GAUGE_FIELD parser accepts
/// bare group synonyms (`SU2`, `SU3`, `U1`) as equivalent to their
/// canonical parenthesized forms (`SU(2)`, `SU(3)`, `U(1)`). This test
/// pins both forms produce the same `Group` variant in the parsed
/// `Statement::GaugeField` AST node, which is the surface programmatic
/// callers (and the GQL surface) observe.
#[test]
fn test_parser_accepts_su3_synonym() {
    use gigi::parser::{parse, Statement};

    let stmt_paren =
        parse("GAUGE_FIELD U ON LATTICE bb GROUP SU(3) INIT IDENTITY;")
            .expect("SU(3) parses");
    let stmt_bare =
        parse("GAUGE_FIELD U ON LATTICE bb GROUP SU3 INIT IDENTITY;")
            .expect("SU3 (bare) parses");

    let group_paren = match &stmt_paren {
        Statement::GaugeField { group, .. } => *group,
        other => panic!("expected GaugeField, got {other:?}"),
    };
    let group_bare = match &stmt_bare {
        Statement::GaugeField { group, .. } => *group,
        other => panic!("expected GaugeField, got {other:?}"),
    };
    assert_eq!(group_paren, Group::SU3, "SU(3) → Group::SU3");
    assert_eq!(group_bare, Group::SU3, "SU3 → Group::SU3");
    assert_eq!(group_paren, group_bare, "both forms yield the same group");
}

/// Parser ergonomics #4 (2026-06-28): SU2 synonym mirror of the SU3
/// test above. Pins that the synonym path doesn't accidentally collapse
/// every short token onto the same group.
#[test]
fn test_parser_accepts_su2_synonym() {
    use gigi::parser::{parse, Statement};

    let stmt_paren =
        parse("GAUGE_FIELD U ON LATTICE bb GROUP SU(2) INIT IDENTITY;")
            .expect("SU(2) parses");
    let stmt_bare =
        parse("GAUGE_FIELD U ON LATTICE bb GROUP SU2 INIT IDENTITY;")
            .expect("SU2 (bare) parses");

    let group_paren = match &stmt_paren {
        Statement::GaugeField { group, .. } => *group,
        _ => panic!("expected GaugeField"),
    };
    let group_bare = match &stmt_bare {
        Statement::GaugeField { group, .. } => *group,
        _ => panic!("expected GaugeField"),
    };
    assert_eq!(group_paren, Group::SU2);
    assert_eq!(group_bare, Group::SU2);
}

/// Parser ergonomics #4 (2026-06-28): bare `ZN` without a modulus is
/// rejected with a clear error directing the user to `Z(<n>)`. The
/// canonical `Z(2)` path still parses fine.
#[test]
fn test_parser_rejects_bare_zn_without_modulus() {
    use gigi::parser::parse;

    let err = parse("GAUGE_FIELD U ON LATTICE bb GROUP ZN INIT IDENTITY;")
        .expect_err("bare ZN must be rejected");
    assert!(
        err.contains("ZN") && err.contains("Z(") && err.contains("modulus"),
        "expected ZN-needs-modulus message, got: {err}"
    );

    // Z(2) still works.
    let ok = parse("GAUGE_FIELD U ON LATTICE bb GROUP Z(2) INIT IDENTITY;")
        .expect("Z(2) parses");
    match ok {
        gigi::parser::Statement::GaugeField { group, .. } => {
            assert_eq!(group, Group::ZN { n: 2 });
        }
        _ => panic!("expected GaugeField"),
    }
}
