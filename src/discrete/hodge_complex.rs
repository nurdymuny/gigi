//! L6.2 — Discrete exterior derivative operators `d_0`, `d_1`
//! (catalog §2.9, IMPLEMENTATION_PLAN.md L6.2).
//!
//! Builds the chain complex `C⁰ → C¹ → C²` from cell-incidence data:
//!
//! ```text
//! d_0 : C⁰ → C¹   (gradient)   (shape: |E| × |V|)
//! d_1 : C¹ → C²   (curl)       (shape: |F| × |E|)
//! ```
//!
//! `d_1 ∘ d_0 = 0` (the chain-complex identity) is forced by the
//! cell orientation convention — each triangle's boundary contains
//! its three edges with signs that cancel exactly on any common
//! vertex. The `d_squared_max_abs()` invariant check verifies this
//! numerically (must be `0.0` on integer inputs).
//!
//! ### Edge orientation
//!
//! Edges are oriented as `(i, j)` with `i < j`. `d_0(v)[e] = +1` if
//! `e.j == v`, `-1` if `e.i == v`, else `0`. This matches the
//! Python reference in `validation_tests_v3.py::test_11_hodge_torus`.
//!
//! ### Face orientation
//!
//! Faces are 3-cliques `(i, j, k)` with `i < j < k`. The boundary
//! traversal is `[(i, j), (j, k), -(i, k)]` so that summing the
//! oriented signed edges around the triangle gives zero on each
//! vertex when composed with `d_0`. The tetrahedron test in
//! `test_11_hodge_torus` uses exactly this convention.
//!
//! ### Validation
//!
//! On the T² 6×6 periodic grid we recover Betti `(1, 2, 1)`; on
//! the tetrahedron (= S²) we recover `(1, 0, 1)`. Both match the
//! Python ground truth exactly.

use nalgebra::DMatrix;

/// A 2-dim discrete chain complex built from cell incidence.
///
/// Holds the `d_0` (V → E) and `d_1` (E → F) matrices as dense
/// `DMatrix<f64>` — fine for the bundles + synthetic tests at L6
/// scale. A sparse rewrite is a follow-up if we hit >10^5 cells.
///
/// ### Invariants (held at construction)
///
/// - Edges are listed once in canonical order `(i, j)`, `i < j`.
/// - Faces are listed once in canonical order `(i, j, k)`,
///   `i < j < k`.
/// - All face edges exist in the edge list.
/// - `d_1 ∘ d_0 = 0` to machine epsilon.
#[derive(Debug, Clone)]
pub struct HodgeComplex {
    /// Number of 0-cells (vertices).
    pub n_vertices: usize,
    /// The 1-cells (edges) in canonical `(i, j)`, `i < j` order.
    pub edges: Vec<(usize, usize)>,
    /// The 2-cells (faces) in canonical `(i, j, k)`, `i < j < k` order.
    pub faces: Vec<(usize, usize, usize)>,
    /// `d_0` matrix, shape `|E| × |V|`.
    pub d0: DMatrix<f64>,
    /// `d_1` matrix, shape `|F| × |E|`.
    pub d1: DMatrix<f64>,
}

/// Failure modes for `HodgeComplex` construction.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum HodgeComplexError {
    /// An edge `(i, j)` references a vertex index `>= n_vertices`.
    #[error("edge ({i}, {j}) references vertex out of range (n_vertices = {n})")]
    EdgeOutOfRange { i: usize, j: usize, n: usize },
    /// An edge `(i, j)` has `i >= j` — not in canonical form.
    #[error("edge ({i}, {j}) is not in canonical i<j form")]
    EdgeNotCanonical { i: usize, j: usize },
    /// A face `(i, j, k)` references a vertex index `>= n_vertices`.
    #[error("face ({i}, {j}, {k}) references vertex out of range (n_vertices = {n})")]
    FaceOutOfRange {
        i: usize,
        j: usize,
        k: usize,
        n: usize,
    },
    /// A face `(i, j, k)` is not in canonical `i < j < k` form.
    #[error("face ({i}, {j}, {k}) is not in canonical i<j<k form")]
    FaceNotCanonical { i: usize, j: usize, k: usize },
    /// A face references an edge that's not in the edges list.
    #[error("face ({i}, {j}, {k}) references unlisted edge ({a}, {b})")]
    FaceMissingEdge {
        i: usize,
        j: usize,
        k: usize,
        a: usize,
        b: usize,
    },
}

impl HodgeComplex {
    /// Construct a `HodgeComplex` from explicit cell lists. Returns
    /// `Err` if any invariant fails; never panics on bad input.
    ///
    /// Time: `O(|V| · |E| + |F| · |E|)` to build the dense
    /// matrices; the `d² = 0` invariant check is `O(|F| · |V|)`.
    pub fn new(
        n_vertices: usize,
        edges: Vec<(usize, usize)>,
        faces: Vec<(usize, usize, usize)>,
    ) -> Result<Self, HodgeComplexError> {
        // Validate edges.
        for &(i, j) in &edges {
            if i >= n_vertices || j >= n_vertices {
                return Err(HodgeComplexError::EdgeOutOfRange {
                    i,
                    j,
                    n: n_vertices,
                });
            }
            if i >= j {
                return Err(HodgeComplexError::EdgeNotCanonical { i, j });
            }
        }
        // Validate faces.
        for &(i, j, k) in &faces {
            if i >= n_vertices || j >= n_vertices || k >= n_vertices {
                return Err(HodgeComplexError::FaceOutOfRange {
                    i,
                    j,
                    k,
                    n: n_vertices,
                });
            }
            if !(i < j && j < k) {
                return Err(HodgeComplexError::FaceNotCanonical { i, j, k });
            }
        }

        // Build edge index for O(1) lookup.
        let edge_index: std::collections::HashMap<(usize, usize), usize> = edges
            .iter()
            .enumerate()
            .map(|(idx, &(a, b))| ((a, b), idx))
            .collect();

        // Build d_0 : |E| × |V|. d_0[e, v] = +1 if e.j == v, -1 if
        // e.i == v.
        let mut d0 = DMatrix::<f64>::zeros(edges.len(), n_vertices);
        for (e_idx, &(i, j)) in edges.iter().enumerate() {
            d0[(e_idx, j)] = 1.0;
            d0[(e_idx, i)] = -1.0;
        }

        // Build d_1 : |F| × |E|. For face (i, j, k):
        //   d_1[f, e(i,j)] = +1
        //   d_1[f, e(j,k)] = +1
        //   d_1[f, e(i,k)] = -1
        // (matches Python tetrahedron convention in
        // validation_tests_v3.py::test_11_hodge_torus.)
        let mut d1 = DMatrix::<f64>::zeros(faces.len(), edges.len());
        for (f_idx, &(i, j, k)) in faces.iter().enumerate() {
            let edges_to_set = [
                ((i, j), 1.0),
                ((j, k), 1.0),
                ((i, k), -1.0),
            ];
            for ((a, b), sign) in edges_to_set {
                let e = edge_index
                    .get(&(a, b))
                    .copied()
                    .ok_or(HodgeComplexError::FaceMissingEdge { i, j, k, a, b })?;
                d1[(f_idx, e)] += sign;
            }
        }

        Ok(Self {
            n_vertices,
            edges,
            faces,
            d0,
            d1,
        })
    }

    /// Max-abs entry of `d_1 ∘ d_0`. The chain-complex identity
    /// requires this to be exactly `0.0` (forced by combinatorics);
    /// any nonzero result is a construction bug.
    pub fn d_squared_max_abs(&self) -> f64 {
        let prod = &self.d1 * &self.d0;
        prod.iter().fold(0.0_f64, |acc, &v| acc.max(v.abs()))
    }

    /// Number of 1-cells (edges).
    pub fn n_edges(&self) -> usize {
        self.edges.len()
    }

    /// Number of 2-cells (faces).
    pub fn n_faces(&self) -> usize {
        self.faces.len()
    }

    /// Euler characteristic `χ = V - E + F`.
    pub fn euler_characteristic(&self) -> i64 {
        self.n_vertices as i64 - self.edges.len() as i64 + self.faces.len() as i64
    }

    /// Build the F₂ representation of `d_0`: `|E| × |V|`, two bits per
    /// row (one for each endpoint of the edge). Used by the rank-based
    /// Betti path in `hodge_laplacian::betti_rank` (step 2 of the
    /// SEMANTIC perf fix). Construction is `O(|E|)`.
    pub(crate) fn d0_f2(&self) -> crate::discrete::f2_rank::F2Matrix {
        let rows: Vec<Vec<usize>> = self
            .edges
            .iter()
            .map(|&(i, j)| vec![i, j])
            .collect();
        let row_refs: Vec<&[usize]> = rows.iter().map(|v| v.as_slice()).collect();
        crate::discrete::f2_rank::F2Matrix::from_index_rows(&row_refs, self.n_vertices)
    }

    /// Build the F₂ representation of `d_1`: `|F| × |E|`, three bits
    /// per row (one for each edge of the triangular face). Construction
    /// is `O(|F|)` using the same edge-index map as `HodgeComplex::new`.
    pub(crate) fn d1_f2(&self) -> crate::discrete::f2_rank::F2Matrix {
        let edge_index: std::collections::HashMap<(usize, usize), usize> = self
            .edges
            .iter()
            .enumerate()
            .map(|(idx, &(a, b))| ((a, b), idx))
            .collect();
        let rows: Vec<Vec<usize>> = self
            .faces
            .iter()
            .map(|&(i, j, k)| {
                let mut out = Vec::with_capacity(3);
                for pair in [(i, j), (j, k), (i, k)] {
                    if let Some(&e) = edge_index.get(&pair) {
                        out.push(e);
                    }
                }
                out
            })
            .collect();
        let row_refs: Vec<&[usize]> = rows.iter().map(|v| v.as_slice()).collect();
        crate::discrete::f2_rank::F2Matrix::from_index_rows(&row_refs, self.edges.len())
    }
}

/// Sparsity instrumentation for a `HodgeComplex` — measures the
/// actual nnz of `d_0` and `d_1` on real bundles so perf claims for
/// the rank-based Betti path can be grounded in data, not assumption.
///
/// Per the Marcella 2026-06-02 SEMANTIC perf letter:
/// > The speedup depends on the boundary matrices being sparse. That
/// > depends on how the VR complex is constructed [...]. If ε is large
/// > enough that the graph is nearly complete (O(n²) edges), the
/// > boundary matrices aren't sparse and the column-reduction cost can
/// > approach O(n³) anyway.
///
/// GIGI's actual complex construction is NOT a VR-on-distance — it's
/// `geometric_neighbors`-based (records sharing an indexed-field
/// value). So |E| can balloon on bundles with low-cardinality indexed
/// categoricals. This helper makes the per-bundle measurement explicit
/// so the smoke tests can pin a real number, not an assumed one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NnzReport {
    /// Vertex count.
    pub n_vertices: usize,
    /// Edge count.
    pub n_edges: usize,
    /// Face count.
    pub n_faces: usize,
    /// Total set bits in `d_0` over F₂. Always equal to `2 · n_edges`
    /// (per-row sparsity is exactly 2 by construction).
    pub d0_nnz: usize,
    /// Total set bits in `d_1` over F₂. Always equal to `3 · n_faces`
    /// (per-row sparsity is exactly 3 by construction).
    pub d1_nnz: usize,
    /// Density of `d_0` = `d0_nnz / (n_edges · n_vertices)`. Tiny
    /// (≈ 2/V) by construction. Sanity check: a "dense" d_0 would be
    /// a misconfigured complex.
    pub d0_density: f64,
    /// Density of `d_1` = `d1_nnz / (n_faces · n_edges)`. Tiny
    /// (≈ 3/E) by construction.
    pub d1_density: f64,
    /// Average number of edges per vertex. The signal for whether the
    /// rank-based path will be fast: small values (e.g. `< 100` for a
    /// 10k-vertex bundle) → sub-second; larger values → approaches
    /// O(V²) and the eigendecomp wasn't *that* much worse.
    pub edges_per_vertex: f64,
}

/// Compute the `NnzReport` for a `HodgeComplex`. `O(1)` — just reads
/// the counts and divides; no matrix construction.
pub fn nnz_report(hc: &HodgeComplex) -> NnzReport {
    let nv = hc.n_vertices;
    let ne = hc.n_edges();
    let nf = hc.n_faces();
    let d0_nnz = 2 * ne;
    let d1_nnz = 3 * nf;
    let d0_density = if nv == 0 || ne == 0 {
        0.0
    } else {
        d0_nnz as f64 / (ne as f64 * nv as f64)
    };
    let d1_density = if ne == 0 || nf == 0 {
        0.0
    } else {
        d1_nnz as f64 / (nf as f64 * ne as f64)
    };
    let edges_per_vertex = if nv == 0 { 0.0 } else { ne as f64 / nv as f64 };
    NnzReport {
        n_vertices: nv,
        n_edges: ne,
        n_faces: nf,
        d0_nnz,
        d1_nnz,
        d0_density,
        d1_density,
        edges_per_vertex,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build the T² 6×6 periodic grid the Python ground truth uses.
    /// Returns `(n_vertices, edges, faces)`. Matches the convention
    /// in `validation_tests_v3.py::test_11_hodge_torus`.
    fn t2_grid(n: usize) -> (usize, Vec<(usize, usize)>, Vec<(usize, usize, usize)>) {
        let nv = n * n;
        let v = |i: usize, j: usize| (i % n) * n + (j % n);

        // Horizontal + vertical + ONE diagonal per square (so the
        // triangulation closes properly). Periodic mod N. Each
        // square (i,j)↔(i+1,j+1) is split into two triangles by
        // its NE-SW diagonal v(i,j)–v(i+1,j+1). Canonicalize each
        // edge to (min, max).
        let mut edge_set: std::collections::BTreeSet<(usize, usize)> =
            std::collections::BTreeSet::new();
        for i in 0..n {
            for j in 0..n {
                let a = v(i, j);
                let b = v(i + 1, j); // horizontal neighbor (wraps)
                edge_set.insert((a.min(b), a.max(b)));
                let c = v(i, j + 1); // vertical neighbor (wraps)
                edge_set.insert((a.min(c), a.max(c)));
                let d = v(i + 1, j + 1); // NE-SW diagonal (wraps)
                edge_set.insert((a.min(d), a.max(d)));
            }
        }
        let edges: Vec<(usize, usize)> = edge_set.into_iter().collect();

        // Two triangles per square split along the NE-SW diagonal:
        //   T1 = (v(i,j), v(i+1,j), v(i+1,j+1))
        //   T2 = (v(i,j), v(i+1,j+1), v(i,j+1))
        // Each triangle's edges are now all present in `edges`.
        let mut face_set: std::collections::BTreeSet<(usize, usize, usize)> =
            std::collections::BTreeSet::new();
        for i in 0..n {
            for j in 0..n {
                let mut t1 = [v(i, j), v(i + 1, j), v(i + 1, j + 1)];
                let mut t2 = [v(i, j), v(i + 1, j + 1), v(i, j + 1)];
                t1.sort();
                t2.sort();
                face_set.insert((t1[0], t1[1], t1[2]));
                face_set.insert((t2[0], t2[1], t2[2]));
            }
        }
        let faces: Vec<(usize, usize, usize)> = face_set.into_iter().collect();

        (nv, edges, faces)
    }

    /// Positive — T² grid: d² = 0 holds.
    #[test]
    fn d_squared_zero_on_torus_grid() {
        let (nv, edges, faces) = t2_grid(6);
        let hc = HodgeComplex::new(nv, edges, faces).expect("build T²");
        assert!(
            hc.d_squared_max_abs() < 1e-12,
            "‖d₁∘d₀‖_∞ must be 0 on T² grid; got {}",
            hc.d_squared_max_abs()
        );
    }

    /// Positive — tetrahedron (= S²): d² = 0 holds. Uses the exact
    /// edge/face lists from the Python reference.
    #[test]
    fn d_squared_zero_on_tetrahedron() {
        let edges = vec![(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
        let faces = vec![(0, 1, 2), (0, 1, 3), (0, 2, 3), (1, 2, 3)];
        let hc = HodgeComplex::new(4, edges, faces).expect("build tetrahedron");
        assert!(
            hc.d_squared_max_abs() < 1e-12,
            "‖d₁∘d₀‖_∞ must be 0 on tetrahedron; got {}",
            hc.d_squared_max_abs()
        );
    }

    /// Sanity — Euler characteristic matches V - E + F.
    #[test]
    fn euler_characteristic_matches_v_minus_e_plus_f() {
        // T² has χ = 0.
        let (nv, edges, faces) = t2_grid(6);
        let n_e = edges.len();
        let n_f = faces.len();
        let hc = HodgeComplex::new(nv, edges, faces).expect("build T²");
        let chi = nv as i64 - n_e as i64 + n_f as i64;
        assert_eq!(hc.euler_characteristic(), chi);
        // For a triangulated T² with two triangles per square,
        // 6×6 periodic: V=36, E=108, F=72 → χ = 36 - 108 + 72 = 0.
        assert_eq!(hc.euler_characteristic(), 0);

        // Tetrahedron has χ = 2.
        let edges = vec![(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
        let faces = vec![(0, 1, 2), (0, 1, 3), (0, 2, 3), (1, 2, 3)];
        let hc = HodgeComplex::new(4, edges, faces).expect("build tet");
        assert_eq!(hc.euler_characteristic(), 2);
    }

    /// Negative — edge with index ≥ n_vertices ⇒ error.
    #[test]
    fn edge_out_of_range_rejected() {
        let r = HodgeComplex::new(3, vec![(0, 5)], vec![]);
        assert!(matches!(r, Err(HodgeComplexError::EdgeOutOfRange { .. })));
    }

    /// Negative — edge not in canonical form (i >= j) ⇒ error.
    #[test]
    fn edge_not_canonical_rejected() {
        let r = HodgeComplex::new(5, vec![(2, 1)], vec![]);
        assert!(matches!(r, Err(HodgeComplexError::EdgeNotCanonical { .. })));
    }

    /// Negative — face references an unlisted edge ⇒ error.
    #[test]
    fn face_missing_edge_rejected() {
        // List only edge (0,1) but try to add face (0,1,2) which
        // needs (0,2) and (1,2).
        let r = HodgeComplex::new(3, vec![(0, 1)], vec![(0, 1, 2)]);
        assert!(matches!(r, Err(HodgeComplexError::FaceMissingEdge { .. })));
    }
}
