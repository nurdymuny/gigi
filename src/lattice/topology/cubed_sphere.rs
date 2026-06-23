//! Cubed-sphere constructor — six gnomonic panels stitched into a
//! topological S² with `LatticeWithMetric` returned.
//!
//! Phase 1 (AURORA A1) deliverable. The constructor parameterizes the
//! per-panel grid resolution C (`panel_size`) and returns a
//! [`LatticeWithMetric`] with:
//!
//! - F = 6 · C² quadrilateral faces (six panels, each a C×C grid of cells).
//! - V = 6 · C² + 2 vertices (8 cube corners + 12·(C−1) cube-edge interiors +
//!   6·(C−1)² panel interiors).
//! - E = 12 · C² edges (Euler V − E + F = 2 holds for all C ≥ 1).
//! - `cell_areas[f]` = spherical excess of face `f` on the unit sphere.
//! - `edge_lengths[e]` = great-circle arc length of edge `e` on the unit sphere.
//! - `dual_face_areas` = `None` (Phase 1 leaves the dual mesh to Phase 2's
//!   `lattice::dec` consumers).
//! - `topology = Some("S2")`.
//!
//! Vertex numbering convention (frozen as part of the A1 bit-identity
//! contract — do not reorder once tests anchor against it):
//!
//! 1. 8 cube-corner vertices first, indices 0..8, in the order
//!    `(s_x, s_y, s_z)` ∈ `{(−,−,−),(+,−,−),(−,+,−),(+,+,−),(−,−,+),
//!    (+,−,+),(−,+,+),(+,+,+)}` (mapped to the unit sphere by
//!    normalization).
//! 2. 12 cube-edge interior runs next, indices 8..8+12·(C−1), in fixed
//!    cube-edge order: 4 X-axis edges (z=−1,y=−1; z=−1,y=+1; z=+1,y=−1;
//!    z=+1,y=+1), then 4 Y-axis edges (z=−1,x=−1; z=−1,x=+1; z=+1,x=−1;
//!    z=+1,x=+1), then 4 Z-axis edges (y=−1,x=−1; y=−1,x=+1; y=+1,x=−1;
//!    y=+1,x=+1). Each run is ordered by increasing axis coordinate
//!    (t ∈ 1..C in panel local-coord units).
//! 3. 6 panel interiors last, indices 8 + 12·(C−1).., in panel order
//!    0..5 with the cube-face mapping `0=+X, 1=−X, 2=+Y, 3=−Y, 4=+Z,
//!    5=−Z`. Each panel's interior is a (C−1)·(C−1) block scanned in
//!    row-major (i then j) order, where `(i, j)` is the panel's local
//!    grid coordinate with i, j ∈ 1..C.
//!
//! Edge numbering is panel-major: for each panel 0..5, the constructor
//! emits horizontal-then-vertical edges in (i, j) row-major order,
//! skipping edges whose canonical owner panel is lower-indexed.
//!
//! Face numbering is panel-major: panel `p` contributes the C² faces
//! `p·C² + (j·C + i)` for (i, j) ∈ 0..C × 0..C.
//!
//! Face cycle orientation: each face's vertex cycle
//! `(i,j) → (i+1,j) → (i+1,j+1) → (i,j+1)` is emitted in the order that
//! produces an outward-pointing normal under the panel's gnomonic
//! mapping (the panel local-axis convention below is chosen to make this
//! the natural orientation). `Lattice::signed_face_orientations()` is
//! the single source of truth for per-edge signs — this module does NOT
//! duplicate face-cycle traversal logic.

use crate::lattice::metric::LatticeWithMetric;
use crate::lattice::{Lattice, VertexId};
use std::collections::HashMap;

/// Fallible thin wrapper over [`cubed_sphere`] for the AURORA Phase 1
/// test surface. Validates `panel_size` against the 1 ≤ C ≤ 256 envelope
/// from the AURORA design lock and returns a `Result` so call sites can
/// `.expect("...")` without panicking on bad input.
///
/// The lattice is named `"cubed_sphere"` (matches the registry default).
/// Use [`cubed_sphere`] directly if a custom name is required.
pub fn build(panel_size: usize) -> Result<LatticeWithMetric, String> {
    if panel_size < 1 {
        return Err(format!(
            "cubed_sphere::build: panel_size must be >= 1 (got {panel_size})"
        ));
    }
    if panel_size > 256 {
        return Err(format!(
            "cubed_sphere::build: panel_size must be <= 256 (got {panel_size})"
        ));
    }
    Ok(cubed_sphere("cubed_sphere", panel_size))
}

/// Top-level constructor. Returns a fully-metric [`LatticeWithMetric`]
/// for the cubed-sphere with `panel_size` cells per panel side.
///
/// Panics if `panel_size < 1`. The 1 ≤ C ≤ 256 envelope from the
/// AURORA design lock is enforced by the registry's argument validator;
/// callers reaching the bare constructor are trusted.
pub fn cubed_sphere(name: &str, panel_size: usize) -> LatticeWithMetric {
    assert!(panel_size >= 1, "cubed_sphere: panel_size must be >= 1");
    let c = panel_size;

    // 1. Build the (panel, i, j) → VertexId table with canonical-owner
    //    deduplication for cube corners and cube-edge interiors.
    let (n_vertices, vertex_id, vertex_coords) = build_vertex_table(c);

    // 2. Emit faces in panel-major order. Each face is a 4-cycle in
    //    (i, j) order; the panel's local-axis mapping is chosen so this
    //    cycle is counter-clockwise viewed from outside the sphere.
    let mut faces: Vec<Vec<VertexId>> = Vec::with_capacity(6 * c * c);
    for p in 0..6usize {
        for j in 0..c {
            for i in 0..c {
                let v00 = vertex_id[&(p, i, j)];
                let v10 = vertex_id[&(p, i + 1, j)];
                let v11 = vertex_id[&(p, i + 1, j + 1)];
                let v01 = vertex_id[&(p, i, j + 1)];
                faces.push(vec![v00, v10, v11, v01]);
            }
        }
    }
    debug_assert_eq!(faces.len(), 6 * c * c);

    // 3. Emit edges in panel-major, horizontal-then-vertical order,
    //    skipping any edge whose canonical owner panel is lower-indexed.
    //    A panel owns an edge iff: (a) the edge is interior to the
    //    panel (both endpoints have at least one of i, j strictly
    //    between 0 and C), OR (b) the edge lies on a panel boundary
    //    whose canonical owner is this panel.
    let mut edges: Vec<(VertexId, VertexId)> = Vec::new();
    let mut emitted: HashMap<(VertexId, VertexId), usize> = HashMap::new();
    for p in 0..6usize {
        // Horizontal edges (i, j) → (i+1, j), i ∈ 0..C, j ∈ 0..=C.
        for j in 0..=c {
            for i in 0..c {
                let a = vertex_id[&(p, i, j)];
                let b = vertex_id[&(p, i + 1, j)];
                let key = canonical_pair(a, b);
                if emitted.contains_key(&key) {
                    continue;
                }
                emitted.insert(key, edges.len());
                edges.push((a, b));
            }
        }
        // Vertical edges (i, j) → (i, j+1), i ∈ 0..=C, j ∈ 0..C.
        for j in 0..c {
            for i in 0..=c {
                let a = vertex_id[&(p, i, j)];
                let b = vertex_id[&(p, i, j + 1)];
                let key = canonical_pair(a, b);
                if emitted.contains_key(&key) {
                    continue;
                }
                emitted.insert(key, edges.len());
                edges.push((a, b));
            }
        }
    }
    debug_assert_eq!(edges.len(), 12 * c * c);

    let lattice = Lattice::new(
        name.to_string(),
        n_vertices,
        edges,
        faces,
        Some("S2".to_string()),
    );

    // 4. Metric: per-cell spherical excess + per-edge great-circle arc.
    let cell_areas = compute_cell_areas(&lattice, &vertex_coords);
    let edge_lengths = compute_edge_lengths(&lattice, &vertex_coords);

    // 5. AURORA Reply 6: forward the gnomonic-projection unit-sphere
    //    positions out through the LatticeWithMetric so consumers
    //    (Williamson T2 IC projection, etc.) don't have to duplicate
    //    the projection table.
    let positions: Vec<(f64, f64, f64)> = vertex_coords
        .iter()
        .map(|c| (c[0], c[1], c[2]))
        .collect();

    LatticeWithMetric::from_lattice_and_metric(
        lattice,
        cell_areas,
        edge_lengths,
        None,
    )
    .with_vertex_positions(positions)
}

/// Canonical ordering for an undirected edge key. The forward direction
/// `(min, max)` is what we hash on; the actual stored `edges` entry
/// preserves the first-emission orientation (the lattice carries that
/// canonical orientation forward to `signed_face_orientations()`).
fn canonical_pair(a: VertexId, b: VertexId) -> (VertexId, VertexId) {
    if a < b { (a, b) } else { (b, a) }
}

// ── Vertex table ──────────────────────────────────────────────────────

/// Build the `(panel, i, j) → VertexId` lookup plus a parallel
/// `Vec<[f64; 3]>` of unit-sphere coordinates indexed by VertexId.
///
/// Returns `(n_vertices, table, coords)`. The table is keyed by every
/// (panel, i, j) for (i, j) ∈ 0..=C × 0..=C — boundary entries deref to
/// the canonical-owner panel's id.
fn build_vertex_table(
    c: usize,
) -> (
    usize,
    HashMap<(usize, usize, usize), VertexId>,
    Vec<[f64; 3]>,
) {
    let mut table: HashMap<(usize, usize, usize), VertexId> = HashMap::new();
    let mut coords: Vec<[f64; 3]> = Vec::new();

    // Tier 1: 8 cube corners. Order: (sx, sy, sz) ∈ {(−,−,−), (+,−,−),
    // (−,+,−), (+,+,−), (−,−,+), (+,−,+), (−,+,+), (+,+,+)}.
    let corner_signs: [[i8; 3]; 8] = [
        [-1, -1, -1],
        [1, -1, -1],
        [-1, 1, -1],
        [1, 1, -1],
        [-1, -1, 1],
        [1, -1, 1],
        [-1, 1, 1],
        [1, 1, 1],
    ];
    for (corner_id, signs) in corner_signs.iter().enumerate() {
        coords.push(normalize([signs[0] as f64, signs[1] as f64, signs[2] as f64]));
        // Register this corner under every (panel, i, j) it appears at.
        for p in 0..6usize {
            for (i, j) in corner_panel_positions(p, c, *signs) {
                table.insert((p, i, j), corner_id);
            }
        }
    }
    let mut next_id: VertexId = 8;

    // Tier 2: 12 cube-edge interior runs of length C−1 each.
    // The 12 edges in fixed order: 4 X-axis (varying x, fixed y,z), then
    // 4 Y-axis (varying y, fixed x,z), then 4 Z-axis (varying z, fixed x,y).
    // Each interior run is parameterized by t ∈ 1..C; the cube point is
    // located at axis-coord = 2·t/C − 1 and projected to the sphere.
    let cube_edges: [(usize, [i8; 3]); 12] = [
        // X-axis edges: axis index 0, fixed (y, z) signs.
        (0, [0, -1, -1]),
        (0, [0, 1, -1]),
        (0, [0, -1, 1]),
        (0, [0, 1, 1]),
        // Y-axis edges: axis index 1, fixed (x, z) signs.
        (1, [-1, 0, -1]),
        (1, [1, 0, -1]),
        (1, [-1, 0, 1]),
        (1, [1, 0, 1]),
        // Z-axis edges: axis index 2, fixed (x, y) signs.
        (2, [-1, -1, 0]),
        (2, [1, -1, 0]),
        (2, [-1, 1, 0]),
        (2, [1, 1, 0]),
    ];
    if c >= 2 {
        for (axis, signs) in cube_edges.iter() {
            for t in 1..c {
                // Cube coordinate of this interior vertex.
                let s = 2.0 * (t as f64) / (c as f64) - 1.0;
                let mut p3 = [signs[0] as f64, signs[1] as f64, signs[2] as f64];
                p3[*axis] = s;
                coords.push(normalize(p3));

                // Register under every (panel, i, j) that lands on
                // this cube-edge interior position.
                for p in 0..6usize {
                    for (i, j) in cube_edge_panel_positions(p, c, *axis, *signs, t) {
                        table.insert((p, i, j), next_id);
                    }
                }
                next_id += 1;
            }
        }
    }

    // Tier 3: 6 panel interiors, each (C−1) × (C−1), row-major (i then j).
    if c >= 2 {
        for p in 0..6usize {
            for j in 1..c {
                for i in 1..c {
                    let cube = panel_ij_to_cube(p, i, j, c);
                    coords.push(normalize(cube));
                    table.insert((p, i, j), next_id);
                    next_id += 1;
                }
            }
        }
    }

    let n_vertices = next_id;
    debug_assert_eq!(n_vertices, 6 * c * c + 2);
    (n_vertices, table, coords)
}

/// Return every `(i, j)` on panel `p` that the cube corner with the
/// given sign tuple occupies. Corners belong to exactly 3 of the 6
/// panels — the three whose fixed axis matches one of the three nonzero
/// signs.
fn corner_panel_positions(
    panel: usize,
    c: usize,
    signs: [i8; 3],
) -> Vec<(usize, usize)> {
    let (axis, fixed_sign) = panel_axis_and_sign(panel);
    if signs[axis] != fixed_sign {
        return Vec::new();
    }
    let (ui, uj) = panel_local_axes(panel);
    let i = if signs[ui] == 1 { c } else { 0 };
    let j = if signs[uj] == 1 { c } else { 0 };
    vec![(i, j)]
}

/// Return every `(i, j)` on panel `p` that the given cube-edge interior
/// vertex (axis, signs, t) occupies. Cube-edge interiors belong to
/// exactly 2 panels — the two whose fixed axis equals one of the two
/// nonzero signs on this cube edge.
fn cube_edge_panel_positions(
    panel: usize,
    c: usize,
    cube_edge_axis: usize,
    edge_signs: [i8; 3],
    t: usize,
) -> Vec<(usize, usize)> {
    let (panel_axis, panel_sign) = panel_axis_and_sign(panel);
    // Panel's fixed axis must be one of the two fixed (nonzero) axes
    // of this cube edge, with matching sign.
    if panel_axis == cube_edge_axis {
        return Vec::new();
    }
    if edge_signs[panel_axis] != panel_sign {
        return Vec::new();
    }
    let (ui, uj) = panel_local_axes(panel);
    // Determine which of (ui, uj) is the varying-axis direction.
    // The varying axis is `cube_edge_axis`; the other fixed axis sets
    // the other local coord.
    let other_fixed_axis = (0..3usize)
        .find(|&a| a != panel_axis && a != cube_edge_axis)
        .unwrap();
    let varying_coord = t; // 1..C
    let other_fixed_coord = if edge_signs[other_fixed_axis] == 1 { c } else { 0 };

    let (i, j) = if ui == cube_edge_axis {
        (varying_coord, other_fixed_coord)
    } else if uj == cube_edge_axis {
        (other_fixed_coord, varying_coord)
    } else {
        unreachable!("panel local axes must cover the two non-fixed axes")
    };
    vec![(i, j)]
}

/// Map panel index to (fixed_axis, fixed_sign).
/// `0=+X, 1=−X, 2=+Y, 3=−Y, 4=+Z, 5=−Z`.
fn panel_axis_and_sign(panel: usize) -> (usize, i8) {
    match panel {
        0 => (0, 1),
        1 => (0, -1),
        2 => (1, 1),
        3 => (1, -1),
        4 => (2, 1),
        5 => (2, -1),
        _ => panic!("panel index out of range"),
    }
}

/// Map panel index to (local_i_axis, local_j_axis) — the two
/// non-fixed cube axes that the (i, j) grid coordinates live on.
///
/// The choices are FROZEN as part of the bit-identity contract. They are
/// picked so the natural face cycle (i,j)→(i+1,j)→(i+1,j+1)→(i,j+1)
/// produces an outward-pointing normal under gnomonic projection. For
/// every panel, the convention is:
///   - local_i_axis × local_j_axis = +fixed_axis_sign × fixed_axis_unit
///     (right-hand rule, outward).
///
/// Verified table:
///   panel 0 (+X): i=Y, j=Z → Y × Z = +X ✓ outward
///   panel 1 (−X): i=Z, j=Y → Z × Y = −X ✓ outward
///   panel 2 (+Y): i=Z, j=X → Z × X = +Y ✓ outward
///   panel 3 (−Y): i=X, j=Z → X × Z = −Y ✓ outward
///   panel 4 (+Z): i=X, j=Y → X × Y = +Z ✓ outward
///   panel 5 (−Z): i=Y, j=X → Y × X = −Z ✓ outward
fn panel_local_axes(panel: usize) -> (usize, usize) {
    match panel {
        0 => (1, 2), // +X: i=Y, j=Z
        1 => (2, 1), // −X: i=Z, j=Y
        2 => (2, 0), // +Y: i=Z, j=X
        3 => (0, 2), // −Y: i=X, j=Z
        4 => (0, 1), // +Z: i=X, j=Y
        5 => (1, 0), // −Z: i=Y, j=X
        _ => panic!("panel index out of range"),
    }
}

/// Compute the cube-face coordinate corresponding to panel-local (i, j)
/// at resolution C. Returns the 3D point on the inscribed cube (not yet
/// normalized to the sphere).
fn panel_ij_to_cube(panel: usize, i: usize, j: usize, c: usize) -> [f64; 3] {
    let (axis, sign) = panel_axis_and_sign(panel);
    let (ui, uj) = panel_local_axes(panel);
    let mut p = [0.0_f64; 3];
    p[axis] = sign as f64;
    p[ui] = 2.0 * (i as f64) / (c as f64) - 1.0;
    p[uj] = 2.0 * (j as f64) / (c as f64) - 1.0;
    p
}

/// L2-normalize a 3-vector to the unit sphere.
fn normalize(v: [f64; 3]) -> [f64; 3] {
    let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    [v[0] / n, v[1] / n, v[2] / n]
}

// ── Metric (areas + arc lengths) ─────────────────────────────────────

/// Per-cell spherical excess on the unit sphere. For each quadrilateral
/// face, we triangulate into two spherical triangles and sum their
/// excesses via L'Huilier's theorem.
fn compute_cell_areas(lattice: &Lattice, coords: &[[f64; 3]]) -> Vec<f64> {
    lattice
        .faces
        .iter()
        .map(|face| {
            // Triangulate quad (v0, v1, v2, v3) as (v0, v1, v2) + (v0, v2, v3).
            let v0 = coords[face[0]];
            let v1 = coords[face[1]];
            let v2 = coords[face[2]];
            let v3 = coords[face[3]];
            spherical_triangle_area(v0, v1, v2) + spherical_triangle_area(v0, v2, v3)
        })
        .collect()
}

/// Spherical triangle area via L'Huilier's theorem. Inputs are
/// unit-sphere points; output is the area in steradians.
fn spherical_triangle_area(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
    let side_a = great_circle_arc(b, c);
    let side_b = great_circle_arc(a, c);
    let side_c = great_circle_arc(a, b);
    let s = 0.5 * (side_a + side_b + side_c);
    // L'Huilier: tan(E/4) = √(tan(s/2) tan((s−a)/2) tan((s−b)/2) tan((s−c)/2)).
    let t = (s * 0.5).tan()
        * ((s - side_a) * 0.5).tan()
        * ((s - side_b) * 0.5).tan()
        * ((s - side_c) * 0.5).tan();
    if t <= 0.0 {
        return 0.0;
    }
    4.0 * t.sqrt().atan()
}

/// Per-edge great-circle arc length on the unit sphere.
fn compute_edge_lengths(lattice: &Lattice, coords: &[[f64; 3]]) -> Vec<f64> {
    lattice
        .edges
        .iter()
        .map(|&(u, v)| great_circle_arc(coords[u], coords[v]))
        .collect()
}

/// Great-circle arc length between two unit-sphere points. Uses the
/// numerically stable atan2(|a×b|, a·b) form.
fn great_circle_arc(a: [f64; 3], b: [f64; 3]) -> f64 {
    let dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let cross = [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ];
    let cross_norm = (cross[0] * cross[0]
        + cross[1] * cross[1]
        + cross[2] * cross[2])
        .sqrt();
    cross_norm.atan2(dot)
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lattice::EdgeOrientation;

    /// Combinatorial counts for C=1 (degenerate cube).
    #[test]
    fn combinatorics_c1() {
        let lwm = cubed_sphere("cs1", 1);
        let lat = lwm.lattice();
        assert_eq!(lat.n_vertices, 8, "C=1: 8 cube corners");
        assert_eq!(lat.n_edges(), 12, "C=1: 12 cube edges");
        assert_eq!(lat.n_faces(), 6, "C=1: 6 cube faces");
        assert_eq!(lat.euler_characteristic(), 2);
        assert_eq!(lat.topology.as_deref(), Some("S2"));
    }

    /// Combinatorial counts for C=2.
    #[test]
    fn combinatorics_c2() {
        let lwm = cubed_sphere("cs2", 2);
        let lat = lwm.lattice();
        assert_eq!(lat.n_vertices, 6 * 4 + 2);
        assert_eq!(lat.n_edges(), 12 * 4);
        assert_eq!(lat.n_faces(), 6 * 4);
        assert_eq!(lat.euler_characteristic(), 2);
    }

    /// Combinatorial counts for C=3.
    #[test]
    fn combinatorics_c3() {
        let lwm = cubed_sphere("cs3", 3);
        let lat = lwm.lattice();
        assert_eq!(lat.n_vertices, 6 * 9 + 2);
        assert_eq!(lat.n_edges(), 12 * 9);
        assert_eq!(lat.n_faces(), 6 * 9);
        assert_eq!(lat.euler_characteristic(), 2);
    }

    /// Every face is a quadrilateral (4-cycle).
    #[test]
    fn all_faces_are_quads() {
        let lwm = cubed_sphere("cs4", 4);
        for face in &lwm.lattice().faces {
            assert_eq!(face.len(), 4, "every cubed-sphere face is a quad");
        }
    }

    /// `signed_face_orientations()` consumes our face cycles and
    /// returns one signed-edge tuple per face position. Every edge of
    /// a closed S² surface appears in exactly two faces, with one
    /// Forward and one Reverse traversal — verify this directly.
    #[test]
    fn signed_face_orientations_consume_clean() {
        let lwm = cubed_sphere("cs3o", 3);
        let lat = lwm.lattice();
        let sfo = lat.signed_face_orientations();
        assert_eq!(sfo.len(), lat.n_faces());
        for cycle in &sfo {
            assert_eq!(cycle.len(), 4);
        }
        // Tally per-edge (forward_count, reverse_count). Closed S²:
        // every edge has (1, 1).
        let mut tally: Vec<(u32, u32)> = vec![(0, 0); lat.n_edges()];
        for cycle in &sfo {
            for &(eid, orient) in cycle {
                match orient {
                    EdgeOrientation::Forward => tally[eid].0 += 1,
                    EdgeOrientation::Reverse => tally[eid].1 += 1,
                }
            }
        }
        for (eid, (fwd, rev)) in tally.iter().enumerate() {
            assert_eq!(
                (*fwd, *rev),
                (1, 1),
                "edge {eid} should appear once Forward and once Reverse on S²"
            );
        }
    }

    /// Metric stub presence: cell_areas and edge_lengths populated to
    /// the right cardinalities; dual_face_areas left None.
    #[test]
    fn metric_present_and_sized() {
        let lwm = cubed_sphere("cs2m", 2);
        let lat = lwm.lattice();
        assert_eq!(lwm.cell_areas().len(), lat.n_faces());
        assert_eq!(lwm.edge_lengths().len(), lat.n_edges());
        assert!(lwm.dual_face_areas().is_none());
        for &a in lwm.cell_areas() {
            assert!(a > 0.0, "every cell area should be strictly positive");
        }
        for &l in lwm.edge_lengths() {
            assert!(l > 0.0, "every edge length should be strictly positive");
        }
    }

    /// Total spherical area on the unit sphere equals 4π (to f64 tol).
    #[test]
    fn total_cell_area_equals_four_pi() {
        let lwm = cubed_sphere("csarea", 4);
        let total: f64 = lwm.cell_areas().iter().sum();
        let four_pi = 4.0 * std::f64::consts::PI;
        assert!(
            (total - four_pi).abs() < 1e-10,
            "total area {total} should equal 4π = {four_pi}"
        );
    }

    /// Topology hint round-trips as "S2".
    #[test]
    fn topology_hint_is_s2() {
        let lwm = cubed_sphere("cshint", 2);
        assert_eq!(lwm.lattice().topology.as_deref(), Some("S2"));
    }

    /// First eight vertex coordinates are the cube corners (after
    /// normalization, ±1/√3 in each axis) in the frozen order.
    #[test]
    fn first_eight_vertices_are_cube_corners_in_frozen_order() {
        // We rebuild the coord table internally to verify ordering;
        // since the constructor doesn't expose coords, we re-derive
        // and check against the table directly.
        let c = 2;
        let (_, _, coords) = build_vertex_table(c);
        let r3 = 1.0_f64 / 3.0_f64.sqrt();
        let expected: [[f64; 3]; 8] = [
            [-r3, -r3, -r3],
            [r3, -r3, -r3],
            [-r3, r3, -r3],
            [r3, r3, -r3],
            [-r3, -r3, r3],
            [r3, -r3, r3],
            [-r3, r3, r3],
            [r3, r3, r3],
        ];
        for (k, exp) in expected.iter().enumerate() {
            for axis in 0..3 {
                assert!(
                    (coords[k][axis] - exp[axis]).abs() < 1e-12,
                    "corner {k} axis {axis}: got {} want {}",
                    coords[k][axis],
                    exp[axis]
                );
            }
        }
    }
}
