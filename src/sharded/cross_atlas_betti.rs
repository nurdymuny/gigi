//! T9 wiring: cross-atlas BETTI via fiber-product Mayer-Vietoris.
//!
//! Per T9 (`theory/poincare_to_sharding/validation/t9_cross_atlas_betti.py`):
//! for two simplicial atlases X_1, X_2 with a bridge identifying common
//! simplices, the Betti numbers of the fiber product X_1 ×_bridge X_2
//! are recoverable exactly from per-atlas chain data plus the bridge
//! identification — without constructing a global chain complex.
//!
//! ## Algorithm
//!
//! The assembly is the cross-atlas Mayer-Vietoris quotient:
//!
//! 1. Disjoint union: take X_1 and X_2 as separate complexes with
//!    disjoint vertex labels (atlas 1 vertices get tag `(1, v)`,
//!    atlas 2 vertices get `(2, v)`).
//! 2. Identification: collapse bridge-paired vertices via union-find
//!    (the bridge maps `(1, v_a) ↔ (2, v_b)` pairs).
//! 3. Rewrite all edges and faces under the identification.
//! 4. Compute Betti on the resulting complex via the existing
//!    `discrete::hodge_laplacian::betti_rank` (F₂ rank pipeline).
//!
//! The assembly **only consumes** per-atlas simplex lists + the bridge
//! dict. It never constructs the identified complex from external
//! ground truth.
//!
//! Phase 1 ships the vertex-only bridge (the most common case for
//! discrete topology gates); edge/face bridges follow the same pattern
//! and are queued as needed.

use crate::discrete::hodge_complex::HodgeComplex;
use crate::discrete::hodge_laplacian::{betti_rank, BettiNumbers};
use std::collections::HashMap;

/// A simplicial atlas: vertices + edges + faces.
///
/// Cells are referenced by their integer index. The atlas's internal
/// numbering is irrelevant — the cross-atlas assembly relabels them
/// under the union-find.
#[derive(Clone, Debug, PartialEq)]
pub struct SimplicialAtlas {
    /// Number of 0-cells (vertices).
    pub n_vertices: usize,
    /// 1-cells in canonical `(i, j)`, `i < j` form.
    pub edges: Vec<(usize, usize)>,
    /// 2-cells in canonical `(i, j, k)`, `i < j < k` form.
    pub faces: Vec<(usize, usize, usize)>,
}

impl SimplicialAtlas {
    pub fn new(
        n_vertices: usize,
        edges: Vec<(usize, usize)>,
        faces: Vec<(usize, usize, usize)>,
    ) -> Self {
        Self {
            n_vertices,
            edges,
            faces,
        }
    }
}

/// A bridge identification: maps vertex indices in atlas 1 to vertex
/// indices in atlas 2.
///
/// Bridge semantics: for every `(v_a -> v_b)` entry, the vertex `v_a`
/// in atlas 1 and the vertex `v_b` in atlas 2 are identified in the
/// fiber product. Multiple entries can target the same vertex in
/// either atlas — the union-find handles transitive identification.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CrossAtlasBridge {
    pub vertex_identifications: HashMap<usize, usize>,
}

impl CrossAtlasBridge {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a vertex identification `(atlas1_vertex, atlas2_vertex)`.
    /// Both vertices will be collapsed in the assembled complex.
    pub fn identify_vertices(&mut self, atlas1_vertex: usize, atlas2_vertex: usize) {
        self.vertex_identifications
            .insert(atlas1_vertex, atlas2_vertex);
    }
}

/// Errors from cross-atlas BETTI assembly.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum CrossAtlasBettiError {
    #[error("bridge identifies atlas1 vertex {atlas1_vertex} but atlas1 has only {n_vertices} vertices")]
    BridgeVertexOutOfRange1 {
        atlas1_vertex: usize,
        n_vertices: usize,
    },
    #[error("bridge identifies atlas2 vertex {atlas2_vertex} but atlas2 has only {n_vertices} vertices")]
    BridgeVertexOutOfRange2 {
        atlas2_vertex: usize,
        n_vertices: usize,
    },
    #[error("hodge complex error in assembled complex: {0}")]
    HodgeComplexError(String),
}

/// Union-find for vertex identification.
struct UnionFind {
    parent: Vec<usize>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
        }
    }

    fn find(&mut self, mut i: usize) -> usize {
        while self.parent[i] != i {
            self.parent[i] = self.parent[self.parent[i]];
            i = self.parent[i];
        }
        i
    }

    fn union(&mut self, i: usize, j: usize) {
        let ri = self.find(i);
        let rj = self.find(j);
        if ri != rj {
            // Lower index wins for deterministic numbering
            if ri < rj {
                self.parent[rj] = ri;
            } else {
                self.parent[ri] = rj;
            }
        }
    }
}

/// Cross-atlas BETTI report.
#[derive(Clone, Debug)]
pub struct CrossAtlasBettiReport {
    /// The assembled global Betti numbers (b0, b1, b2).
    pub assembled_betti: BettiNumbers,
    /// Number of vertices in the assembled complex (after identification).
    pub n_vertices_assembled: usize,
    /// Number of distinct edges in the assembled complex (after
    /// identification + de-duplication).
    pub n_edges_assembled: usize,
    /// Number of distinct faces in the assembled complex.
    pub n_faces_assembled: usize,
}

/// Assemble the cross-atlas BETTI from two atlases + a bridge.
///
/// The fiber-product Mayer-Vietoris assembly:
/// - Take the disjoint union of X_1 and X_2 (atlas 1 vertices get
///   indices `0..n1`, atlas 2 vertices get `n1..n1+n2`).
/// - Apply union-find to collapse bridge-paired vertices.
/// - Rewrite all edges/faces under the new vertex labels, removing
///   duplicates and degenerate simplices (edges with collapsed
///   endpoints, faces with two identified vertices).
/// - Compute Betti on the resulting `HodgeComplex` via the existing
///   F₂ rank pipeline.
///
/// Per T9: this assembly's output equals the direct simplicial Betti
/// of the identified union — verified against SymPy in the Python gate.
pub fn cross_atlas_betti_via_fiber_product(
    atlas_a: &SimplicialAtlas,
    atlas_b: &SimplicialAtlas,
    bridge: &CrossAtlasBridge,
) -> Result<CrossAtlasBettiReport, CrossAtlasBettiError> {
    // Validate bridge bounds
    for (&v_a, &v_b) in &bridge.vertex_identifications {
        if v_a >= atlas_a.n_vertices {
            return Err(CrossAtlasBettiError::BridgeVertexOutOfRange1 {
                atlas1_vertex: v_a,
                n_vertices: atlas_a.n_vertices,
            });
        }
        if v_b >= atlas_b.n_vertices {
            return Err(CrossAtlasBettiError::BridgeVertexOutOfRange2 {
                atlas2_vertex: v_b,
                n_vertices: atlas_b.n_vertices,
            });
        }
    }

    // Disjoint-union labeling: atlas A uses [0, n_a), atlas B uses
    // [n_a, n_a + n_b).
    let n_a = atlas_a.n_vertices;
    let n_b = atlas_b.n_vertices;
    let n_total = n_a + n_b;
    let mut uf = UnionFind::new(n_total);
    for (&v_a, &v_b) in &bridge.vertex_identifications {
        uf.union(v_a, n_a + v_b);
    }

    // Canonical label = union-find root. Remap roots to a dense
    // [0, n_canonical) numbering in the order they first appear.
    let mut canonical_label: HashMap<usize, usize> = HashMap::new();
    let mut next_idx = 0usize;
    for v in 0..n_total {
        let r = uf.find(v);
        if !canonical_label.contains_key(&r) {
            canonical_label.insert(r, next_idx);
            next_idx += 1;
        }
    }
    let label_of = |v: usize| -> usize {
        let mut uf_clone = UnionFind {
            parent: uf.parent.clone(),
        };
        let r = uf_clone.find(v);
        *canonical_label.get(&r).expect("every vertex has a canonical label")
    };

    let n_canonical = next_idx;

    // Rewrite edges: collect into a dedup set with canonical form.
    let mut edge_set: std::collections::HashSet<(usize, usize)> =
        std::collections::HashSet::new();
    let mut add_edge = |a: usize, b: usize, edge_set: &mut std::collections::HashSet<(usize, usize)>| {
        if a == b {
            // Degenerate edge — collapsed under identification. Drop.
            return;
        }
        let canon = if a < b { (a, b) } else { (b, a) };
        edge_set.insert(canon);
    };
    for &(i, j) in &atlas_a.edges {
        add_edge(label_of(i), label_of(j), &mut edge_set);
    }
    for &(i, j) in &atlas_b.edges {
        add_edge(label_of(n_a + i), label_of(n_a + j), &mut edge_set);
    }
    let mut edges: Vec<(usize, usize)> = edge_set.into_iter().collect();
    edges.sort();

    // Rewrite faces: collect into a dedup set with canonical sorted form.
    let mut face_set: std::collections::HashSet<(usize, usize, usize)> =
        std::collections::HashSet::new();
    let mut add_face = |a: usize, b: usize, c: usize, fs: &mut std::collections::HashSet<(usize, usize, usize)>| {
        // Degenerate face — two vertices coincide.
        if a == b || b == c || a == c {
            return;
        }
        let mut t = [a, b, c];
        t.sort();
        fs.insert((t[0], t[1], t[2]));
    };
    for &(i, j, k) in &atlas_a.faces {
        add_face(label_of(i), label_of(j), label_of(k), &mut face_set);
    }
    for &(i, j, k) in &atlas_b.faces {
        add_face(
            label_of(n_a + i),
            label_of(n_a + j),
            label_of(n_a + k),
            &mut face_set,
        );
    }
    let mut faces: Vec<(usize, usize, usize)> = face_set.into_iter().collect();
    faces.sort();

    // Build the assembled HodgeComplex and compute Betti via existing
    // F₂ rank pipeline.
    let hc = HodgeComplex::new(n_canonical, edges.clone(), faces.clone())
        .map_err(|e| CrossAtlasBettiError::HodgeComplexError(format!("{}", e)))?;
    let betti = betti_rank(&hc);

    Ok(CrossAtlasBettiReport {
        assembled_betti: betti,
        n_vertices_assembled: n_canonical,
        n_edges_assembled: edges.len(),
        n_faces_assembled: faces.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// T9 Part (a) reframed: S² from two tetrahedral hemispheres
    /// joined at a triangular equator.
    ///
    /// Atlas A = upper hemisphere: 1 apex + 3 equator vertices, 3
    /// triangular faces meeting at the apex.
    /// Atlas B = lower hemisphere: same topology, different apex.
    /// Bridge: identify the 3 equator vertices.
    ///
    /// Assembled complex is a bipyramid surface (combinatorial S²):
    /// 5 vertices, 9 edges, 6 distinct faces — every face has a
    /// distinct vertex set so no face-deduplication artifacts. Betti
    /// numbers are (1, 0, 1).
    ///
    /// NOTE: An earlier framing using two flat triangles glued at
    /// their full boundary triangle was degenerate — both atlases
    /// produced the SAME `(0,1,2)` face triple, which the HodgeComplex
    /// representation collapses (it indexes 2-cells by sorted vertex
    /// tuples, so it can't distinguish two different 2-cells sharing
    /// a vertex set). The bipyramid construction avoids that by
    /// ensuring each assembled face has a unique vertex triple.
    #[test]
    fn s2_bipyramid_assembly_yields_b_1_0_1() {
        // Upper hemisphere: apex = vertex 0, equator = vertices 1, 2, 3.
        let atlas_a = SimplicialAtlas::new(
            4,
            vec![(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)],
            vec![(0, 1, 2), (0, 1, 3), (0, 2, 3)],
        );
        // Lower hemisphere: apex = vertex 0, equator = vertices 1, 2, 3.
        // Same per-atlas structure; the bridge collapses the equator.
        let atlas_b = atlas_a.clone();
        let mut bridge = CrossAtlasBridge::new();
        // Identify only the 3 equator vertices, NOT the apex.
        bridge.identify_vertices(1, 1);
        bridge.identify_vertices(2, 2);
        bridge.identify_vertices(3, 3);

        let report =
            cross_atlas_betti_via_fiber_product(&atlas_a, &atlas_b, &bridge).unwrap();
        assert_eq!(report.assembled_betti.b0, 1, "S² should have b0 = 1 (connected)");
        assert_eq!(report.assembled_betti.b1, 0, "S² should have b1 = 0");
        assert_eq!(report.assembled_betti.b2, 1, "S² should have b2 = 1 (one 2-hole)");
        // 5 vertices: 2 apexes + 3 equator
        assert_eq!(report.n_vertices_assembled, 5);
        // 9 edges: 3 upper apex-equator + 3 lower apex-equator + 3 equator
        assert_eq!(report.n_edges_assembled, 9);
        // 6 faces: 3 upper + 3 lower, all distinct vertex triples
        assert_eq!(report.n_faces_assembled, 6);
    }

    /// Identical to the above but with NO bridge: the disjoint union
    /// of two triangles. b0 = 2 (two components), b1 = 0, b2 = 0.
    ///
    /// Substantive non-triviality: confirms the bridge IS doing work —
    /// without it, b0 differs from S²'s b0 = 1.
    #[test]
    fn s2_double_triangle_without_bridge_disjoint_union() {
        let atlas_a = SimplicialAtlas::new(
            3,
            vec![(0, 1), (0, 2), (1, 2)],
            vec![(0, 1, 2)],
        );
        let atlas_b = atlas_a.clone();
        let bridge = CrossAtlasBridge::new();

        let report =
            cross_atlas_betti_via_fiber_product(&atlas_a, &atlas_b, &bridge).unwrap();
        // Two disjoint triangles -> b0 = 2 (NOT 1 as in S²).
        // Each triangle is a disk (b1 = b2 = 0).
        assert_eq!(report.assembled_betti.b0, 2);
        assert_eq!(report.assembled_betti.b1, 0);
        assert_eq!(report.assembled_betti.b2, 0);
        assert_eq!(report.n_vertices_assembled, 6);
    }

    /// Partial bridge: identify only ONE vertex pair. The two
    /// triangles meet at a single point. b0 = 1 (connected via that
    /// point), b1 = 0, b2 = 0.
    #[test]
    fn s2_double_triangle_with_single_vertex_bridge() {
        let atlas_a = SimplicialAtlas::new(
            3,
            vec![(0, 1), (0, 2), (1, 2)],
            vec![(0, 1, 2)],
        );
        let atlas_b = atlas_a.clone();
        let mut bridge = CrossAtlasBridge::new();
        bridge.identify_vertices(0, 0); // only one shared vertex

        let report =
            cross_atlas_betti_via_fiber_product(&atlas_a, &atlas_b, &bridge).unwrap();
        // Wedge of two disks at one point — still contractible from
        // each side, but connected via the wedge point.
        assert_eq!(report.assembled_betti.b0, 1);
        assert_eq!(report.assembled_betti.b1, 0);
        assert_eq!(report.assembled_betti.b2, 0);
        assert_eq!(report.n_vertices_assembled, 5);
    }

    /// Verify the bridge identification machinery: assembling an atlas
    /// against ITSELF with the identity bridge gives the same Betti
    /// numbers as the original atlas (verified via betti_rank).
    #[test]
    fn identity_bridge_against_self_yields_original_betti() {
        // A tetrahedron boundary (S^2) as a single atlas.
        let atlas = SimplicialAtlas::new(
            4,
            vec![(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)],
            vec![(0, 1, 2), (0, 1, 3), (0, 2, 3), (1, 2, 3)],
        );
        // Direct Betti via the existing pipeline
        let direct_hc = HodgeComplex::new(
            atlas.n_vertices,
            atlas.edges.clone(),
            atlas.faces.clone(),
        )
        .unwrap();
        let direct_betti = betti_rank(&direct_hc);

        // Now assemble the atlas against itself with identity bridge.
        let mut bridge = CrossAtlasBridge::new();
        for v in 0..atlas.n_vertices {
            bridge.identify_vertices(v, v);
        }
        let report =
            cross_atlas_betti_via_fiber_product(&atlas, &atlas, &bridge).unwrap();
        // Identity bridge -> assembled = original.
        assert_eq!(report.assembled_betti.b0, direct_betti.b0);
        assert_eq!(report.assembled_betti.b1, direct_betti.b1);
        assert_eq!(report.assembled_betti.b2, direct_betti.b2);
        // S^2 = (1, 0, 1)
        assert_eq!(report.assembled_betti.b0, 1);
        assert_eq!(report.assembled_betti.b1, 0);
        assert_eq!(report.assembled_betti.b2, 1);
        assert_eq!(report.n_vertices_assembled, 4);
    }

    /// Two triangles joined at a shared edge. Bridge identifies vertex
    /// 0 of atlas A with vertex 0 of atlas B, and vertex 1 of A with 1
    /// of B. The result has 4 vertices, 5 edges (edge (0,1) appears in
    /// both and dedups), and 2 faces. Topologically a disk: b = (1, 0, 0).
    #[test]
    fn two_triangles_sharing_an_edge_form_a_disk() {
        let atlas_a = SimplicialAtlas::new(
            3,
            vec![(0, 1), (0, 2), (1, 2)],
            vec![(0, 1, 2)],
        );
        let atlas_b = atlas_a.clone();
        let mut bridge = CrossAtlasBridge::new();
        bridge.identify_vertices(0, 0);
        bridge.identify_vertices(1, 1);
        // Vertex 2 of each atlas stays distinct -> two distinct faces
        // sharing an edge.

        let report =
            cross_atlas_betti_via_fiber_product(&atlas_a, &atlas_b, &bridge).unwrap();
        assert_eq!(report.assembled_betti.b0, 1);
        assert_eq!(report.assembled_betti.b1, 0);
        assert_eq!(report.assembled_betti.b2, 0);
        assert_eq!(report.n_vertices_assembled, 4); // 0, 1 shared; 2_a, 2_b distinct
        assert_eq!(report.n_edges_assembled, 5);    // (0,1) shared; (0,2_a), (1,2_a), (0,2_b), (1,2_b)
        assert_eq!(report.n_faces_assembled, 2);
    }

    /// Bridge bound validation: identifying a non-existent vertex
    /// fails cleanly.
    #[test]
    fn out_of_range_bridge_vertex_returns_error() {
        let atlas_a = SimplicialAtlas::new(3, vec![(0, 1)], vec![]);
        let atlas_b = SimplicialAtlas::new(3, vec![(0, 1)], vec![]);
        let mut bridge = CrossAtlasBridge::new();
        bridge.identify_vertices(99, 0);
        let r = cross_atlas_betti_via_fiber_product(&atlas_a, &atlas_b, &bridge);
        assert!(matches!(
            r,
            Err(CrossAtlasBettiError::BridgeVertexOutOfRange1 { atlas1_vertex: 99, .. })
        ));
    }

    #[test]
    fn out_of_range_bridge_vertex_atlas_b_returns_error() {
        let atlas_a = SimplicialAtlas::new(3, vec![(0, 1)], vec![]);
        let atlas_b = SimplicialAtlas::new(3, vec![(0, 1)], vec![]);
        let mut bridge = CrossAtlasBridge::new();
        bridge.identify_vertices(0, 99);
        let r = cross_atlas_betti_via_fiber_product(&atlas_a, &atlas_b, &bridge);
        assert!(matches!(
            r,
            Err(CrossAtlasBettiError::BridgeVertexOutOfRange2 { atlas2_vertex: 99, .. })
        ));
    }

    /// Empty atlases + empty bridge: no vertices, b = (0, 0, 0).
    #[test]
    fn empty_atlases_yield_zero_betti() {
        let atlas_a = SimplicialAtlas::new(0, vec![], vec![]);
        let atlas_b = SimplicialAtlas::new(0, vec![], vec![]);
        let bridge = CrossAtlasBridge::new();
        let report =
            cross_atlas_betti_via_fiber_product(&atlas_a, &atlas_b, &bridge).unwrap();
        assert_eq!(report.assembled_betti.b0, 0);
        assert_eq!(report.assembled_betti.b1, 0);
        assert_eq!(report.assembled_betti.b2, 0);
        assert_eq!(report.n_vertices_assembled, 0);
    }
}
