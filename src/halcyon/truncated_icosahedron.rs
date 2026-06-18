//! Truncated-icosahedron (buckyball) constructor.
//!
//! Closes TDD-HAL-I.2. Returns a `Lattice` with V = 60, E = 90,
//! F = 32 (12 pentagons + 20 hexagons), χ = 2, topology `"S2"`.
//!
//! Incidence model: this implementation builds the buckyball as the
//! Goldberg polyhedron `G(1, 1)` — the fullerene C₆₀ — by first
//! constructing an icosahedron, then truncating each of its 12
//! vertices to a pentagon. The pentagonal faces sit at the original
//! icosahedron vertices; the hexagonal faces are the truncated
//! versions of the original 20 triangular faces.
//!
//! The construction is purely combinatorial — no floating-point
//! coordinates appear in the Part-I substrate. Bit-identity is
//! trivial because the integer incidence table is determined by
//! the icosahedron's adjacency and a deterministic truncation rule.
//!
//! Construction:
//!
//! 1. Start with the icosahedron: 12 vertices, 30 edges, 20
//!    triangular faces.
//! 2. At each icosahedron vertex `v` with neighbors `(n_1, …, n_5)`
//!    in cyclic order around the vertex, replace `v` with 5 new
//!    vertices `v_{i}` (one per adjacent edge), forming a pentagon.
//! 3. Each original edge `(v, w)` becomes a single edge in the
//!    buckyball, between the new vertex `v_{vw}` and `w_{wv}`.
//! 4. Each original triangular face becomes a hexagon (each of its
//!    3 original vertices contributes 2 new vertices to the face).
//!
//! Final counts: V = 12 × 5 = 60, E = 12 × 5 (pentagon edges) +
//! 30 (truncated original edges) = 90, F = 12 (pentagons) + 20
//! (hexagons) = 32. Euler χ = 60 − 90 + 32 = 2. ✓

use super::lattice::{Lattice, VertexId};

// ── Icosahedron adjacency table (12 vertices, 30 edges) ──
//
// Vertex layout: vertex 0 is the north pole, vertices 1-5 form the
// upper pentagon (in cyclic order), vertices 6-10 form the lower
// pentagon (in cyclic order, offset by one to alternate with the
// upper ring), vertex 11 is the south pole.
//
// Adjacency derived from the standard icosahedron incidence.

const ICO_N: usize = 12;

/// `ICO_NEIGHBORS[v]` is the cyclic list of icosahedron vertices
/// adjacent to vertex `v`, ordered counter-clockwise around `v` as
/// seen from outside the polyhedron. Length 5 for every vertex
/// (the icosahedron is 5-regular).
const ICO_NEIGHBORS: [[usize; 5]; ICO_N] = [
    // 0 (north pole): upper pentagon, ccw.
    [1, 2, 3, 4, 5],
    // 1: north + lower-pentagon neighbors + adjacent upper vertices.
    [0, 5, 10, 6, 2],
    [0, 1, 6, 7, 3],
    [0, 2, 7, 8, 4],
    [0, 3, 8, 9, 5],
    [0, 4, 9, 10, 1],
    // 6: lower-pentagon vertex.
    [1, 10, 11, 7, 2],
    [2, 6, 11, 8, 3],
    [3, 7, 11, 9, 4],
    [4, 8, 11, 10, 5],
    [5, 9, 11, 6, 1],
    // 11 (south pole): lower pentagon, ccw.
    [6, 7, 8, 9, 10],
];

/// `ICO_FACES` is the explicit list of 20 triangular faces, each
/// given as 3 vertex indices in counter-clockwise order (outward
/// normal). Used to assemble the buckyball's hexagonal faces.
const ICO_FACES: [[usize; 3]; 20] = [
    // Upper cap: 5 triangles around the north pole.
    [0, 1, 2],
    [0, 2, 3],
    [0, 3, 4],
    [0, 4, 5],
    [0, 5, 1],
    // Middle belt: 10 triangles alternating up / down.
    [1, 6, 2],
    [2, 6, 7],
    [2, 7, 3],
    [3, 7, 8],
    [3, 8, 4],
    [4, 8, 9],
    [4, 9, 5],
    [5, 9, 10],
    [5, 10, 1],
    [1, 10, 6],
    // Lower cap: 5 triangles around the south pole.
    [11, 7, 6],
    [11, 8, 7],
    [11, 9, 8],
    [11, 10, 9],
    [11, 6, 10],
];

/// New-vertex id for the buckyball: at icosahedron vertex `v`,
/// the new vertex on the edge `v → w` is `bb_vertex(v, w)`.
/// Implemented as `5*v + position_of_w_in_neighbors(v)`.
fn bb_vertex(v: usize, w: usize) -> VertexId {
    let neighbors = &ICO_NEIGHBORS[v];
    let pos = neighbors
        .iter()
        .position(|&n| n == w)
        .expect("bb_vertex: w must be a neighbor of v");
    5 * v + pos
}

/// Build the truncated-icosahedron Lattice.
///
/// 60 vertices, 90 edges, 32 faces (12 pentagons + 20 hexagons),
/// Euler χ = 2, topology `"S2"`.
pub fn buckyball() -> Lattice {
    let n_vertices = 5 * ICO_N; // 60

    // ── Edges ──
    //
    // Two kinds:
    //
    // 1. Pentagon edges — at each icosahedron vertex v, the 5 new
    //    vertices form a pentagon: edges (5v+0 → 5v+1), …,
    //    (5v+4 → 5v+0). 12 × 5 = 60 edges.
    //
    // 2. Hexagon-bridging edges — for each original icosahedron
    //    edge (v, w), we have the buckyball edge
    //    (bb_vertex(v, w) → bb_vertex(w, v)). 30 edges.
    //
    // Total: 90. ✓
    let mut edges: Vec<(VertexId, VertexId)> = Vec::with_capacity(90);

    // Pentagon edges.
    for v in 0..ICO_N {
        for i in 0..5 {
            let a = 5 * v + i;
            let b = 5 * v + (i + 1) % 5;
            edges.push((a, b));
        }
    }

    // Hexagon-bridging edges. Each ICO edge (v, w) with v < w gets
    // exactly one buckyball edge.
    for v in 0..ICO_N {
        for &w in ICO_NEIGHBORS[v].iter() {
            if v < w {
                let a = bb_vertex(v, w);
                let b = bb_vertex(w, v);
                edges.push((a, b));
            }
        }
    }
    debug_assert_eq!(edges.len(), 90);

    // ── Faces ──
    //
    // 12 pentagons — one per icosahedron vertex.
    // 20 hexagons  — one per icosahedron triangular face.
    let mut faces: Vec<Vec<VertexId>> = Vec::with_capacity(32);

    // Pentagons.
    for v in 0..ICO_N {
        faces.push((0..5).map(|i| 5 * v + i).collect());
    }

    // Hexagons. For triangle (a, b, c) (ccw), the hexagonal face
    // visits the 6 new vertices in the order:
    //
    //   bb(a, b), bb(b, a), bb(b, c), bb(c, b), bb(c, a), bb(a, c)
    //
    // i.e. for each ordered pair (x → y) along the triangle's
    // boundary (a→b, b→c, c→a), emit (bb(x, y), bb(y, x)) — the
    // two new vertices on that ICO edge in the direction of
    // traversal. This yields a closed cycle around the hexagon.
    for &face in ICO_FACES.iter() {
        let [a, b, c] = face;
        let hexagon = vec![
            bb_vertex(a, b),
            bb_vertex(b, a),
            bb_vertex(b, c),
            bb_vertex(c, b),
            bb_vertex(c, a),
            bb_vertex(a, c),
        ];
        faces.push(hexagon);
    }
    debug_assert_eq!(faces.len(), 32);

    Lattice::new(
        "buckyball",
        n_vertices,
        edges,
        faces,
        Some("S2".to_string()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// TDD-HAL-I.2 — buckyball topology.
    /// V = 60, E = 90, F = 32; χ = 2; face-size histogram = {12
    /// pentagons, 20 hexagons}; every edge is shared by exactly 2
    /// faces.
    #[test]
    fn tdd_hal_i_2_buckyball_topology() {
        let lat = buckyball();

        // Counts.
        assert_eq!(lat.n_vertices, 60, "V");
        assert_eq!(lat.n_edges(), 90, "E");
        assert_eq!(lat.n_faces(), 32, "F");

        // Euler χ = V − E + F = 2.
        assert_eq!(lat.euler_characteristic(), 2, "Euler characteristic");

        // Face-size histogram: {5: 12, 6: 20}.
        let mut hist: HashMap<usize, usize> = HashMap::new();
        for face in &lat.faces {
            *hist.entry(face.len()).or_insert(0) += 1;
        }
        assert_eq!(hist.get(&5).copied().unwrap_or(0), 12, "12 pentagons");
        assert_eq!(hist.get(&6).copied().unwrap_or(0), 20, "20 hexagons");
        assert_eq!(hist.len(), 2, "only pentagons and hexagons");

        // Every edge is shared by exactly 2 faces. Resolve each
        // consecutive vertex pair in every face to an `edge_id`,
        // count occurrences across all faces; every edge must
        // appear exactly twice.
        let mut edge_counts: HashMap<usize, usize> =
            (0..lat.n_edges()).map(|i| (i, 0)).collect();
        for face in &lat.faces {
            let n = face.len();
            for i in 0..n {
                let a = face[i];
                let b = face[(i + 1) % n];
                let (edge_id, _orient) = lat
                    .resolve_edge(a, b)
                    .unwrap_or_else(|| panic!("face edge ({a},{b}) not in incidence"));
                *edge_counts.entry(edge_id).or_insert(0) += 1;
            }
        }
        for (eid, count) in &edge_counts {
            assert_eq!(*count, 2, "edge {eid} appears in {count} faces, expected 2");
        }

        // Topology hint is preserved.
        assert_eq!(lat.topology.as_deref(), Some("S2"));
    }
}
