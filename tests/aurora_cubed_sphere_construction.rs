//! AURORA Phase 1 — A1 CUBED_SPHERE constructor (RED-first tests).
//!
//! Receipt: theory/aurora/GIGI_TO_AURORA_2026-06-19_v0_1_REPLY_2.md
//! § A1 (commit ad306ec, Q4/Q5/Q6 refined at c06f073 + 358ede4).
//!
//! Locked combinatorics (verified at C=1,2,3 by Euler V−E+F=2):
//!
//!     F = 6 · C²
//!     E = 12 · C²
//!     V = 6 · C² + 2
//!
//! C=1 reproduces the inscribed cube exactly (8 corners, 12 edges,
//! 6 quad faces). The constructor returns a `LatticeWithMetric`
//! (the minimal Phase-1 wrapper from src/lattice/metric.rs — DEC
//! operators stay in Phase 2). The lattice half MUST consume
//! `Lattice::signed_face_orientations()` (Phase-0 receipt at ca589eb)
//! rather than duplicate face-cycle traversal.
//!
//! Per the AURORA constraint sheet:
//!   - combinatorial counts are exact integer assertions (no tolerances);
//!   - f64 metric values use a documented 1e-10 tolerance only where
//!     spherical-excess / arc-length accumulation requires it;
//!   - bit-identity for the existing TRUNCATED_ICOSAHEDRON surface is
//!     a separate test file (aurora_lattice_registry_dispatch.rs).

#![cfg(feature = "lattice")]

use gigi::lattice::topology::cubed_sphere;

/// Smallest non-degenerate face count: C=1 → 6 quadrilateral faces
/// (the inscribed cube).
#[test]
fn test_cubed_sphere_c1_face_count_is_6() {
    let lwm = cubed_sphere::build(1).expect("C=1 build");
    assert_eq!(lwm.lattice().n_faces(), 6, "C=1 face count = 6·1² = 6");
}

/// C=2 → 24 faces (6 panels × 2×2 grid).
#[test]
fn test_cubed_sphere_c2_face_count_is_24() {
    let lwm = cubed_sphere::build(2).expect("C=2 build");
    assert_eq!(lwm.lattice().n_faces(), 24, "C=2 face count = 6·2² = 24");
}

/// C=4 → 96 faces. This is the Phase-1 reference resolution for the
/// 4π surface-area test below.
#[test]
fn test_cubed_sphere_c4_face_count_is_96() {
    let lwm = cubed_sphere::build(4).expect("C=4 build");
    assert_eq!(lwm.lattice().n_faces(), 96, "C=4 face count = 6·4² = 96");
}

/// Euler characteristic on the sphere is 2 for every C ≥ 1.
/// Anchors the combinatorial contract — if V, E, F drift,
/// V − E + F = 2 is the canary.
#[test]
fn test_cubed_sphere_euler_characteristic_is_2() {
    for c in [1usize, 2, 3, 4] {
        let lwm = cubed_sphere::build(c).expect("build");
        let lat = lwm.lattice();
        let v = lat.n_vertices as i64;
        let e = lat.n_edges() as i64;
        let f = lat.n_faces() as i64;
        assert_eq!(
            v - e + f,
            2,
            "C={c}: V−E+F must be 2 on the sphere (got V={v}, E={e}, F={f})"
        );
    }
}

/// Every face on a cubed sphere is a quadrilateral (4-cycle), by
/// construction. C=3 is the smallest C with both edge-shared and
/// fully-interior cells per panel.
#[test]
fn test_cubed_sphere_all_faces_are_quads() {
    let lwm = cubed_sphere::build(3).expect("C=3 build");
    let lat = lwm.lattice();
    for (fidx, face) in lat.faces.iter().enumerate() {
        assert_eq!(
            face.len(),
            4,
            "face {fidx} must be a quadrilateral (got len {})",
            face.len()
        );
    }
}

/// The CUBED_SPHERE constructor MUST consume the promoted
/// `Lattice::signed_face_orientations()` (Phase-0 receipt at ca589eb)
/// rather than duplicate face-cycle traversal. This test exercises
/// that surface end-to-end: outer length = n_faces, every cycle has
/// length 4 (quad).
#[test]
fn test_cubed_sphere_signed_face_orientations_round_trip() {
    let lwm = cubed_sphere::build(2).expect("C=2 build");
    let lat = lwm.lattice();
    let signed = lat.signed_face_orientations();
    assert_eq!(signed.len(), 24, "outer length = n_faces = 24 for C=2");
    for (fidx, cycle) in signed.iter().enumerate() {
        assert_eq!(
            cycle.len(),
            4,
            "signed cycle {fidx} must be length 4 (quad)"
        );
    }
}

/// The constructor stamps the topology hint as Some("S2") by default.
/// This is the metadata half of the A1 surface contract — every
/// cubed-sphere lattice is an S² triangulation (well, quadrangulation).
#[test]
fn test_cubed_sphere_topology_hint_is_s2() {
    let lwm = cubed_sphere::build(2).expect("C=2 build");
    assert_eq!(
        lwm.lattice().topology.as_deref(),
        Some("S2"),
        "CUBED_SPHERE constructor must set topology hint = S2"
    );
}

/// The Phase-1 metric stub assigns cell areas via gnomonic projection
/// + spherical excess. The sum of every cell area on the unit sphere
/// must equal 4π (the sphere's surface area). 1e-10 tolerance honors
/// the AURORA "documented f64 tolerance" rule for accumulation of
/// spherical-excess arithmetic.
#[test]
fn test_cubed_sphere_metric_cell_areas_sum_to_4pi() {
    let lwm = cubed_sphere::build(4).expect("C=4 build");
    let sum: f64 = lwm.cell_areas().iter().sum();
    let expected = 4.0 * std::f64::consts::PI;
    let diff = (sum - expected).abs();
    assert!(
        diff < 1e-10,
        "sum of cell areas must equal 4π on the unit sphere \
         (got {sum}, expected {expected}, |diff|={diff})"
    );
}
