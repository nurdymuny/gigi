//! AURORA Reply 6 — `LatticeWithMetric::vertex_positions()` accessor.
//!
//! Receipt: theory/aurora/AURORA_TO_GIGI_REPLY6_2026-06-22.md
//! ("one new ask: pub fn vertex_positions(&self) -> &[(f64, f64, f64)]").
//!
//! Contract:
//!
//! - Cubed-sphere constructors populate the slice with the normalized
//!   gnomonic-projection unit-sphere coordinates that `build_vertex_table`
//!   already computes internally — same vertex-id order, length = V.
//! - Truncated-icosahedron (buckyball) constructors populate the slice
//!   with the 60 fullerene-cage coordinates normalized to the unit
//!   sphere — same vertex-id order, length = 60.
//! - Constructors that don't compute 3D positions return an empty
//!   slice; consumers (Williamson T2 IC projection, etc.) check
//!   `.is_empty()` and refuse gracefully.
//!
//! This file is the AURORA-side acceptance gate for that ask.

#![cfg(feature = "lattice")]

use gigi::lattice::registry;
use gigi::lattice::topology::cubed_sphere::cubed_sphere;
use gigi::lattice::Lattice;
use gigi::lattice::LatticeWithMetric;

/// Tolerance for "on the unit sphere" — every position vector's
/// squared L2 norm must round-trip to 1.0 within 1e-12. The
/// cubed-sphere `normalize()` helper is a straight L2 division so
/// 1e-12 is comfortably above the worst-case ulp error.
const UNIT_SPHERE_TOL: f64 = 1e-12;

/// C=1 cubed-sphere: 8 cube corners. The vertex_positions slice
/// must have length 8.
#[test]
fn test_cubed_sphere_c1_vertex_positions_length_8() {
    let lwm = cubed_sphere("cs1", 1);
    let pos = lwm.vertex_positions();
    assert_eq!(
        pos.len(),
        8,
        "C=1 cubed-sphere has 8 cube-corner vertices; \
         vertex_positions() must surface them all"
    );
    assert_eq!(
        pos.len(),
        lwm.lattice().n_vertices,
        "vertex_positions().len() must equal lattice().n_vertices"
    );
}

/// C=1 cubed-sphere: every vertex position is a unit-sphere point.
#[test]
fn test_cubed_sphere_c1_vertex_positions_on_unit_sphere() {
    let lwm = cubed_sphere("cs1", 1);
    for (id, &(x, y, z)) in lwm.vertex_positions().iter().enumerate() {
        let norm_sq = x * x + y * y + z * z;
        assert!(
            (norm_sq - 1.0).abs() < UNIT_SPHERE_TOL,
            "C=1 vertex {id} = ({x}, {y}, {z}) not on unit sphere: |v|² = {norm_sq}"
        );
    }
}

/// C=4 cubed-sphere: V = 6·C² + 2 = 98 vertices.
#[test]
fn test_cubed_sphere_c4_vertex_positions_length_98() {
    let lwm = cubed_sphere("cs4", 4);
    let pos = lwm.vertex_positions();
    assert_eq!(
        pos.len(),
        98,
        "C=4 cubed-sphere: V = 6·4² + 2 = 98; vertex_positions length must match"
    );
    assert_eq!(pos.len(), lwm.lattice().n_vertices);
    // And every one must still be unit-sphere.
    for (id, &(x, y, z)) in pos.iter().enumerate() {
        let norm_sq = x * x + y * y + z * z;
        assert!(
            (norm_sq - 1.0).abs() < UNIT_SPHERE_TOL,
            "C=4 vertex {id} not on unit sphere: |v|² = {norm_sq}"
        );
    }
}

/// Buckyball: 60 fullerene-cage vertices. The registry-dispatched
/// truncated-icosahedron LatticeWithMetric must carry all 60.
#[test]
fn test_truncated_icosahedron_vertex_positions_length_60() {
    let ctor = registry::get_constructor("TRUNCATED_ICOSAHEDRON")
        .expect("TRUNCATED_ICOSAHEDRON must be registered");
    let lwm = ctor(&registry::ConstructorArgs::default())
        .expect("buckyball constructor must succeed");
    let pos = lwm.vertex_positions();
    assert_eq!(
        pos.len(),
        60,
        "buckyball: 60 fullerene-cage vertices; \
         vertex_positions length must match"
    );
    assert_eq!(pos.len(), lwm.lattice().n_vertices);
}

/// Buckyball: every cage vertex normalizes onto the unit sphere.
#[test]
fn test_truncated_icosahedron_vertex_positions_on_unit_sphere() {
    let ctor = registry::get_constructor("TRUNCATED_ICOSAHEDRON")
        .expect("TRUNCATED_ICOSAHEDRON must be registered");
    let lwm = ctor(&registry::ConstructorArgs::default())
        .expect("buckyball constructor must succeed");
    for (id, &(x, y, z)) in lwm.vertex_positions().iter().enumerate() {
        let norm_sq = x * x + y * y + z * z;
        assert!(
            (norm_sq - 1.0).abs() < UNIT_SPHERE_TOL,
            "buckyball vertex {id} = ({x}, {y}, {z}) not on unit sphere: |v|² = {norm_sq}"
        );
    }
}

/// Constructors that don't compute 3D positions return an empty
/// slice. The explicit `LATTICE name VERTICES n EDGES … FACES …`
/// declaration path goes through `from_lattice_and_metric` directly
/// without supplying positions; that path must surface an empty
/// vertex_positions slice so consumers can check `.is_empty()` and
/// refuse gracefully.
#[test]
fn test_vertex_positions_empty_for_constructors_without_positions() {
    // Hand-built non-spherical lattice with no 3D position table.
    let lat = Lattice::new(
        "no_positions_smoke",
        4,
        vec![(0, 1), (1, 2), (2, 3), (3, 0)],
        vec![vec![0, 1, 2, 3]],
        Some("R2".to_string()),
    );
    let lwm = LatticeWithMetric::from_lattice_and_metric(
        lat,
        Vec::new(),
        Vec::new(),
        None,
    );
    assert!(
        lwm.vertex_positions().is_empty(),
        "lattices built without 3D positions must surface empty vertex_positions"
    );
}

/// Consumer-side smoke test: lat/lon derivation. AURORA's
/// Williamson T2 IC needs phi = arcsin(z), lambda = atan2(y, x).
/// Verify the +Z cube corner of C=1 has phi = +pi/2 (north pole-ish:
/// actually arcsin(1/sqrt(3)) since corners are diagonal — the
/// point is that the consumer can call the formula at all).
#[test]
fn test_vertex_positions_supports_latlon_derivation() {
    let lwm = cubed_sphere("cs1", 1);
    let pos = lwm.vertex_positions();
    // For each corner, phi should be ±arcsin(1/sqrt(3)).
    let expected_abs_phi = (1.0_f64 / 3.0_f64.sqrt()).asin();
    for (id, &(_x, _y, z)) in pos.iter().enumerate() {
        let phi = z.asin();
        assert!(
            (phi.abs() - expected_abs_phi).abs() < 1e-12,
            "C=1 corner {id}: |phi| = {} != arcsin(1/sqrt(3)) = {expected_abs_phi}",
            phi.abs()
        );
    }
}
