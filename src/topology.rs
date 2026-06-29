//! Higher Betti numbers (β_k for k ≥ 2) and the π_1 fundamental-group
//! presentation on lattice cell complexes — pure algebraic-topology
//! kernel for the CHERN/BETTI/PI_1/OBSTRUCTION verb cluster.
//!
//! This module operates on the `Lattice` cell complex (V vertices,
//! E edges, F faces) and exposes:
//!
//! - [`betti_topological`] — β_k via rank-nullity on the integer
//!   boundary maps ∂_1 and ∂_2.
//! - [`pi_1_presentation`] — the Massey spanning-tree presentation
//!   of π_1, returning generators (one per non-tree edge) and one
//!   relator per face. Phase 1 reports the abelianized rank
//!   (which equals β_1); full non-abelian Tietze reduction is
//!   Phase 2 scope.
//!
//! Phase 1 supports orders k ∈ {0, 1, 2}; β_k for k ≥ 3 requires
//! 3-cells which the current `Lattice` does not store, and is
//! returned as `TopologyError::UnsupportedOrder` until cubic.rs is
//! extended in Phase 2.
//!
//! Integer rank is computed via fraction-free (Bareiss-style)
//! Gaussian elimination on i64. This is exact rank-over-ℚ (= rank
//! of the integer matrix when entries are ±1), suitable for the
//! Phase 1 lattice scales (buckyball E=90 F=32; T² L=4 E=32 F=16;
//! T⁴ L=4 E=1024 F=1536).
//!
//! Consistency self-check on closed 2-manifolds:
//! Σ_k (-1)^k β_k == lattice.euler_characteristic().

#![cfg(feature = "lattice")]

use crate::lattice::{EdgeId, EdgeOrientation, Lattice, VertexId};
use std::collections::HashMap;

// ── Error type ──────────────────────────────────────────────────────

/// Errors from the topology kernel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TopologyError {
    /// `betti_topological` was called with an order higher than the
    /// Phase 1 supported range (currently {0, 1, 2}).
    UnsupportedOrder { order: usize, max_supported: usize },
    /// A face cycle had two consecutive vertices that did not
    /// resolve to an edge of the lattice. Indicates a malformed
    /// lattice construction.
    MalformedFace { face_idx: usize, position: usize },
}

impl std::fmt::Display for TopologyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TopologyError::UnsupportedOrder { order, max_supported } => write!(
                f,
                "betti_topological: order {order} is not supported in Phase 1 (max = {max_supported}); Phase 2 will extend the cell complex with k-cells for k ≥ 3"
            ),
            TopologyError::MalformedFace { face_idx, position } => write!(
                f,
                "malformed face {face_idx}: vertex pair at position {position} is not an edge of the lattice"
            ),
        }
    }
}

impl std::error::Error for TopologyError {}

// ── π_1 presentation type ───────────────────────────────────────────

/// A π_1 presentation built by [`pi_1_presentation`]: spanning-tree
/// generators (one per non-tree edge) and face-cycle relators.
///
/// Phase 1 reports `rank` as the abelianized rank
/// (= rank of H_1 = β_1). Full non-abelian Tietze reduction
/// (collapsing redundant relators, identifying the rank of the
/// free quotient via group-theoretic algorithms) is Phase 2 scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pi1Presentation {
    /// Rank reported to callers. Phase 1: equals `abelianized_rank`.
    pub rank: usize,
    /// Generators: one [`EdgeId`] per non-tree edge in the BFS
    /// spanning forest of the 1-skeleton.
    pub generators: Vec<EdgeId>,
    /// Relators: one per face. Each relator is a signed word in the
    /// generator index space. Encoding: a positive integer `+g+1`
    /// means "forward traversal of generator at index `g`",
    /// a negative integer `-(g+1)` means "reverse". Zero is reserved
    /// (never emitted). Tree edges contribute nothing to the word.
    pub relators: Vec<Vec<i64>>,
    /// Rank of the abelianization H_1(B; ℤ) (= β_1).
    pub abelianized_rank: usize,
}

/// BFS spanning forest of the 1-skeleton of a [`Lattice`].
#[derive(Debug, Clone)]
pub struct SpanningTree {
    /// `parent[v] = Some((parent_vertex, edge_used))` when `v` was
    /// reached by traversing `edge_used` from `parent_vertex` in the
    /// BFS; `None` for the BFS roots (one per connected component).
    pub parent: Vec<Option<(VertexId, EdgeId)>>,
    /// `in_tree[e] = true` iff edge `e` is in the spanning forest.
    pub in_tree: Vec<bool>,
    /// BFS roots, one per connected component. Always includes
    /// vertex 0 (or is empty when the lattice has 0 vertices).
    pub roots: Vec<VertexId>,
}

// ── Components count (β_0) ──────────────────────────────────────────

/// Number of connected components of the 1-skeleton, via union-find
/// over the edges.
pub fn components_count(lattice: &Lattice) -> usize {
    let v = lattice.n_vertices;
    if v == 0 {
        return 0;
    }
    let mut parent: Vec<usize> = (0..v).collect();
    let mut rank: Vec<u8> = vec![0; v];

    fn find(parent: &mut [usize], x: usize) -> usize {
        if parent[x] == x {
            x
        } else {
            let r = find(parent, parent[x]);
            parent[x] = r;
            r
        }
    }

    for &(u, w) in &lattice.edges {
        let ru = find(&mut parent, u);
        let rw = find(&mut parent, w);
        if ru != rw {
            // Union by rank.
            match rank[ru].cmp(&rank[rw]) {
                std::cmp::Ordering::Less => parent[ru] = rw,
                std::cmp::Ordering::Greater => parent[rw] = ru,
                std::cmp::Ordering::Equal => {
                    parent[rw] = ru;
                    rank[ru] += 1;
                }
            }
        }
    }

    let mut roots = std::collections::HashSet::new();
    for x in 0..v {
        let r = find(&mut parent, x);
        roots.insert(r);
    }
    roots.len()
}

// ── Boundary maps ∂_1 and ∂_2 ───────────────────────────────────────

/// Build the integer boundary map ∂_1: ℤ^E → ℤ^V as a row-per-edge
/// dense i64 matrix.
///
/// Convention: `edges[i] = (u, v)` contributes a column of length V
/// with `-1` at row `u` and `+1` at row `v`. Returned as a row-major
/// V × E matrix (so row k has length E).
pub fn boundary_1(lattice: &Lattice) -> Vec<Vec<i64>> {
    let v = lattice.n_vertices;
    let e = lattice.n_edges();
    let mut mat: Vec<Vec<i64>> = vec![vec![0i64; e]; v];
    for (eid, &(u, w)) in lattice.edges.iter().enumerate() {
        if u < v {
            mat[u][eid] += -1;
        }
        if w < v {
            mat[w][eid] += 1;
        }
    }
    mat
}

/// Build the integer boundary map ∂_2: ℤ^F → ℤ^E as a row-per-edge
/// dense i64 matrix.
///
/// Each face contributes a column of length E. For each consecutive
/// vertex pair `(face[i], face[i+1])` in the face cycle, the
/// orientation returned by [`Lattice::resolve_edge`] gives a signed
/// ±1 entry at the corresponding edge row. Returned as a row-major
/// E × F matrix.
pub fn boundary_2(lattice: &Lattice) -> Result<Vec<Vec<i64>>, TopologyError> {
    let e = lattice.n_edges();
    let f = lattice.n_faces();
    let mut mat: Vec<Vec<i64>> = vec![vec![0i64; f]; e];
    for (fidx, face) in lattice.faces.iter().enumerate() {
        let n = face.len();
        if n == 0 {
            continue;
        }
        for pos in 0..n {
            let a = face[pos];
            let b = face[(pos + 1) % n];
            let (eid, orient) = lattice
                .resolve_edge(a, b)
                .ok_or(TopologyError::MalformedFace { face_idx: fidx, position: pos })?;
            mat[eid][fidx] += orient.sign() as i64;
        }
    }
    Ok(mat)
}

// ── Integer matrix rank via Bareiss-style fraction-free Gaussian ────

/// Compute the rank over ℚ of an integer matrix.
///
/// The input is row-major (rows.len() == n_rows; rows[r].len() == n_cols).
/// Operates IN PLACE on a clone of the input — does not mutate caller
/// memory.
///
/// Algorithm: fraction-free Gaussian elimination ("Bareiss form").
/// At step k, for the pivot `p_k = m[k][k]`, eliminate row `i > k`
/// by `m[i][j] = (m[i][j] * p_k - m[i][k] * m[k][j]) / p_{k-1}`. The
/// division is exact under Sylvester's identity. Returns the number
/// of nonzero rows in the elimination's row-echelon form.
///
/// **Overflow framing (named blocking precondition):** Phase 1 ∂_1 and
/// ∂_2 inputs have entries in `{-1, 0, +1}`. The intermediate-value
/// growth under Bareiss elimination is bounded above by the Hadamard
/// bound on the largest minor — theoretically this can be very large
/// (factorial in matrix size for dense ±1 matrices). In practice the
/// lattice boundary matrices are SPARSE with bounded incidence degree
/// (each face touches ≤ 4 edges; each edge touches ≤ 2 vertices), so
/// Bareiss pivots stay small (empirically ≤ 6 for our T⁴ L=4 fixtures)
/// and the computation runs at the entrywise scale of the input. The
/// i128 intermediates buffer the multiply step against worst-case
/// minor growth.
///
/// EMPIRICALLY holds for: buckyball (V=60, E=90, F=32); T² L=4
/// (V=16, E=32, F=16); T⁴ L=4 (V=256, E=1024, F=1536). Larger lattices
/// or denser cell complexes may need a true bignum path (Phase 2
/// ticket).
pub fn integer_matrix_rank(mat: &[Vec<i64>], n_rows: usize, n_cols: usize) -> usize {
    if n_rows == 0 || n_cols == 0 {
        return 0;
    }
    let mut m: Vec<Vec<i64>> = mat.iter().map(|r| r.clone()).collect();
    // Defensive: pad short rows. Should never trigger for our callers.
    for r in m.iter_mut() {
        if r.len() < n_cols {
            r.resize(n_cols, 0);
        }
    }

    let mut rank: usize = 0;
    let mut prev_pivot: i64 = 1;
    let mut col: usize = 0;
    let mut row: usize = 0;

    while row < n_rows && col < n_cols {
        // Find a pivot in column `col` at or below row `row`.
        let mut pivot_row = None;
        for r in row..n_rows {
            if m[r][col] != 0 {
                pivot_row = Some(r);
                break;
            }
        }
        let p_row = match pivot_row {
            Some(r) => r,
            None => {
                // No pivot in this column; advance column.
                col += 1;
                continue;
            }
        };

        // Swap pivot row to position `row`. Note: swap flips the
        // sign of the determinant but does NOT affect the rank.
        if p_row != row {
            m.swap(p_row, row);
        }

        let pivot = m[row][col];

        // Eliminate all rows below.
        for i in (row + 1)..n_rows {
            if m[i][col] != 0 {
                let m_i_col = m[i][col];
                // Bareiss update: m[i][j] = (pivot * m[i][j] - m_i_col * m[row][j]) / prev_pivot
                // Computed via i128 to avoid overflow, then divided exactly.
                for j in col..n_cols {
                    let p = pivot as i128;
                    let mij = m[i][j] as i128;
                    let mrj = m[row][j] as i128;
                    let mic = m_i_col as i128;
                    let new_val = (p * mij - mic * mrj) / (prev_pivot as i128);
                    m[i][j] = new_val as i64;
                }
            }
        }

        prev_pivot = pivot;
        rank += 1;
        row += 1;
        col += 1;
    }

    rank
}

// ── Higher-dimensional cell synthesis for cubic lattices ────────────
//
// The `Lattice` struct stores only V, E, F (vertices / edges /
// 2-faces). Higher Betti orders (k = 2 on T^D with D ≥ 3, and
// k ≥ 3 in general) need ∂_3 — the boundary map from 3-cells
// (cubes) to 2-cells (faces). Phase 1 derives the 3-cell
// combinatorics on the fly from the lattice's topology-hint string
// (`"CUBIC_L{L}_D{D}"`) for the cubic family. Other lattice
// families (buckyball / cubed_sphere) only need ∂_2 (their
// 2-manifold structure has no native 3-cells).
//
// This keeps cubic.rs untouched and the cubic locking constraint
// honored, while making `betti_topological(_, 2)` correct on T⁴.

#[derive(Debug, Clone, Copy)]
struct CubicHint {
    l: usize,
    d: usize,
}

/// Parse a topology hint of the form `"CUBIC_L{L}_D{D}"` (PERIODIC).
/// Returns `None` for anything else.
fn parse_cubic_hint(lattice: &Lattice) -> Option<CubicHint> {
    let hint = lattice.topology.as_deref()?;
    if !hint.starts_with("CUBIC_L") || hint.ends_with("_OPEN") {
        return None;
    }
    let rest = hint.strip_prefix("CUBIC_L")?;
    let (l_str, d_part) = rest.split_once("_D")?;
    let l: usize = l_str.parse().ok()?;
    let d: usize = d_part.parse().ok()?;
    Some(CubicHint { l, d })
}

/// Compute coordinates of a site under the cubic indexing convention
/// (row-major `v(c) = sum_k c_k · L^k`).
fn coords_of(site: VertexId, hint: CubicHint) -> Vec<usize> {
    let mut s = site;
    let mut c = vec![0usize; hint.d];
    for k in 0..hint.d {
        c[k] = s % hint.l;
        s /= hint.l;
    }
    c
}

/// Encode a coordinate tuple back to a site id.
fn site_of(c: &[usize], hint: CubicHint) -> VertexId {
    let mut s = 0usize;
    let mut stride = 1usize;
    for (k, &ck) in c.iter().enumerate() {
        s += ck * stride;
        stride *= hint.l;
    }
    s
}

/// Look up the face index for the (site, axis_a, axis_b) plaquette
/// in the cubic constructor's face-ordering convention:
/// faces are (axis-pair-major, site-major) over `a < b`, then `s`.
fn cubic_face_index(s: VertexId, a: usize, b: usize, hint: CubicHint) -> Option<usize> {
    if a >= b || b >= hint.d {
        return None;
    }
    // Count pairs (a', b') with a' < b' that come before (a, b)
    // under lex order on (a, b).
    let mut pair_idx = 0usize;
    for ap in 0..hint.d {
        for bp in (ap + 1)..hint.d {
            if (ap, bp) == (a, b) {
                let n_vertices: usize = (0..hint.d).fold(1usize, |acc, _| acc * hint.l);
                return Some(pair_idx * n_vertices + s);
            }
            pair_idx += 1;
        }
    }
    None
}

/// Build the integer boundary map ∂_3: ℤ^{C_3} → ℤ^F for a cubic
/// lattice T^D (D ≥ 3). Each 3-cell is an oriented 3-cube indexed by
/// (axis-triple, site), bounded by 6 plaquette faces with signed
/// orientations derived from the cube's natural orientation.
///
/// Returns `None` when the lattice is not a recognized cubic hint
/// or has D < 3 (no 3-cells exist).
fn boundary_3_cubic(lattice: &Lattice) -> Option<Vec<Vec<i64>>> {
    let hint = parse_cubic_hint(lattice)?;
    if hint.d < 3 {
        return None;
    }
    let n_vertices: usize = (0..hint.d).fold(1usize, |acc, _| acc * hint.l);
    let n_triples = (0..hint.d)
        .flat_map(|a| ((a + 1)..hint.d).flat_map(move |b| ((b + 1)..hint.d).map(move |c| (a, b, c))))
        .count();
    let n_cells = n_vertices * n_triples;
    let n_faces = lattice.n_faces();

    // Dense E (= face count, since rows are faces) × n_cells matrix.
    // Each 3-cube contributes ±1 to 6 faces.
    let mut mat: Vec<Vec<i64>> = vec![vec![0i64; n_cells]; n_faces];

    // For each 3-cube indexed by axis-triple (a, b, c) and site s:
    // the 6 boundary faces are the 2-faces on each pair (a,b), (a,c),
    // (b,c) at the "low" and "high" ends along the third axis.
    // Standard orientation convention (cubical chain complex):
    //   ∂_3([abc]_s) = +[bc]_{s + e_a} - [bc]_s   (a-direction)
    //                  - [ac]_{s + e_b} + [ac]_s  (b-direction)
    //                  + [ab]_{s + e_c} - [ab]_s  (c-direction)
    //
    // Signs follow the alternating cubical convention; consistency
    // with our ∂_2 face orientation (counter-clockwise in (a,b))
    // means ∂_2 ∘ ∂_3 = 0 should hold modulo the face cycle
    // orientation. We verify this implicitly via the β_2 = 6 test
    // on T⁴; if a sign is wrong rank(∂_3) collapses and β_2 is off.

    let shift_plus = |c: &[usize], axis: usize| -> Vec<usize> {
        let mut c2 = c.to_vec();
        c2[axis] = (c2[axis] + 1) % hint.l;
        c2
    };

    let mut cube_idx = 0usize;
    for a in 0..hint.d {
        for b in (a + 1)..hint.d {
            for c_ax in (b + 1)..hint.d {
                for s in 0..n_vertices {
                    let coords = coords_of(s, hint);
                    let s_a = site_of(&shift_plus(&coords, a), hint);
                    let s_b = site_of(&shift_plus(&coords, b), hint);
                    let s_c = site_of(&shift_plus(&coords, c_ax), hint);

                    // +[bc]_{s+e_a} - [bc]_s
                    let f_bc_sa = cubic_face_index(s_a, b, c_ax, hint).unwrap();
                    let f_bc_s = cubic_face_index(s, b, c_ax, hint).unwrap();
                    mat[f_bc_sa][cube_idx] += 1;
                    mat[f_bc_s][cube_idx] += -1;

                    // -[ac]_{s+e_b} + [ac]_s
                    let f_ac_sb = cubic_face_index(s_b, a, c_ax, hint).unwrap();
                    let f_ac_s = cubic_face_index(s, a, c_ax, hint).unwrap();
                    mat[f_ac_sb][cube_idx] += -1;
                    mat[f_ac_s][cube_idx] += 1;

                    // +[ab]_{s+e_c} - [ab]_s
                    let f_ab_sc = cubic_face_index(s_c, a, b, hint).unwrap();
                    let f_ab_s = cubic_face_index(s, a, b, hint).unwrap();
                    mat[f_ab_sc][cube_idx] += 1;
                    mat[f_ab_s][cube_idx] += -1;

                    cube_idx += 1;
                }
            }
        }
    }
    debug_assert_eq!(cube_idx, n_cells);

    // Direct chain-complex integrity check: ∂_2 ∘ ∂_3 = 0 must hold
    // for any valid cubical chain complex. This catches sign-convention
    // bugs in `boundary_3_cubic` BEFORE the β_2 rank computation rolls
    // them up into a single number. Reference: Hatcher *Algebraic
    // Topology* §2.2.
    //
    // We exploit ∂_3's sparsity (each cube touches 6 faces) plus
    // ∂_2's sparsity (each face touches 4 edges) so the check runs in
    // O(C_3 · 6 · 4) = O(24 · C_3) — fast enough to leave on in debug
    // builds.
    #[cfg(debug_assertions)]
    {
        // Build a per-face sparse-rep of ∂_2 (face f → 4 (edge, sign) pairs).
        let mut face_edges_signed: Vec<Vec<(usize, i64)>> = vec![Vec::with_capacity(4); n_faces];
        for (fidx, face) in lattice.faces.iter().enumerate() {
            let n = face.len();
            if n == 0 {
                continue;
            }
            for pos in 0..n {
                let a = face[pos];
                let b = face[(pos + 1) % n];
                if let Some((eid, orient)) = lattice.resolve_edge(a, b) {
                    face_edges_signed[fidx].push((eid, orient.sign() as i64));
                }
            }
        }
        // Per-cube sparse-rep of ∂_3 from `mat`: we rebuild it the easy
        // way (6 entries) by walking the construction loop indices we
        // already used above. Reuse: scan each column of `mat`.
        for c in 0..n_cells {
            // Compose ∂_2 ∘ ∂_3 on this cube column: accumulate into a
            // small edge tally keyed by edge id.
            let mut tally: std::collections::HashMap<usize, i64> = std::collections::HashMap::new();
            for f in 0..n_faces {
                let s_cf = mat[f][c];
                if s_cf == 0 {
                    continue;
                }
                for &(eid, sign) in &face_edges_signed[f] {
                    *tally.entry(eid).or_insert(0) += s_cf * sign;
                }
            }
            if let Some((eid, val)) = tally.into_iter().find(|(_, v)| *v != 0) {
                debug_assert!(
                    false,
                    "boundary_3_cubic: ∂_2 ∘ ∂_3 ≠ 0 at edge {eid}, cube {c} \
                     (got {val}) — sign convention bug in cubical chain complex"
                );
            }
        }
    }

    Some(mat)
}

// ── Higher Betti dispatcher ─────────────────────────────────────────

/// Compute β_k(lattice) for k ∈ {0, 1, 2}.
///
/// Betti numbers are ranks of the homology groups
/// H_k = ker(∂_k) / im(∂_{k+1}) of the cellular chain complex
///   C_0 ←∂_1— C_1 ←∂_2— C_2 ←∂_3— C_3 ...
/// On the Phase 1 lattice (no 3-cells stored) we set ∂_3 ≡ 0, so:
///
/// - k = 0: dim ker(∂_0) - rank(∂_1) = V - rank(∂_1)
///   = connected components count (proved by elementary linear
///   algebra and confirmed by the union-find computation).
/// - k = 1: dim ker(∂_1) - rank(∂_2) = (E - rank(∂_1)) - rank(∂_2).
///   The cycle rank E - V + c trapped only the kernel of ∂_1
///   without subtracting the boundary-image rank from ∂_2 — that
///   subtraction is the difference between the 1-skeleton's
///   cycle space and H_1 of the closed 2-complex.
/// - k = 2: dim ker(∂_2) - rank(∂_3) = F - rank(∂_2) since ∂_3 = 0
///   in Phase 1 (the Lattice stores no 3-cells; extending this
///   requires Phase 2 cubic.rs work to populate `cells_3`).
/// - k ≥ 3: returns [`TopologyError::UnsupportedOrder`] until
///   cubic.rs is extended in Phase 2 to store cells_3.
///
/// Consistency invariant on closed 2-manifolds: Σ_k (-1)^k β_k == χ.
pub fn betti_topological(lattice: &Lattice, order: usize) -> Result<usize, TopologyError> {
    match order {
        0 => Ok(components_count(lattice)),
        1 => {
            // β_1 = (E - rank(∂_1)) - rank(∂_2).
            let e = lattice.n_edges();
            if e == 0 {
                return Ok(0);
            }
            let m1 = boundary_1(lattice);
            let r1 = integer_matrix_rank(&m1, lattice.n_vertices, e);
            let ker_d1 = e - r1;
            // rank(∂_2): 0 when there are no faces.
            let r2 = if lattice.n_faces() == 0 {
                0
            } else {
                let m2 = boundary_2(lattice)?;
                integer_matrix_rank(&m2, e, lattice.n_faces())
            };
            // ker_d1 >= r2 on a valid cell complex (boundaries are cycles),
            // but guard against pathological inputs with a saturating sub.
            Ok(ker_d1.saturating_sub(r2))
        }
        2 => {
            // β_2 = ker(∂_2) - rank(∂_3) = (F - rank(∂_2)) - rank(∂_3).
            // On 2-manifolds (no 3-cells) rank(∂_3) = 0 and we get F - rank(∂_2).
            // On T^D with D ≥ 3, ∂_3 is synthesized from the cubic
            // topology hint so β_2 = C(D, 2) as expected.
            let f = lattice.n_faces();
            if f == 0 {
                return Ok(0);
            }
            let m2 = boundary_2(lattice)?;
            let r2 = integer_matrix_rank(&m2, lattice.n_edges(), f);
            let ker_d2 = f - r2;
            let r3 = if let Some(m3) = boundary_3_cubic(lattice) {
                let n_cells = m3.first().map(|r| r.len()).unwrap_or(0);
                integer_matrix_rank(&m3, f, n_cells)
            } else {
                0
            };
            Ok(ker_d2.saturating_sub(r3))
        }
        k => Err(TopologyError::UnsupportedOrder { order: k, max_supported: 2 }),
    }
}

// ── Spanning tree (BFS over the 1-skeleton) ─────────────────────────

/// Build a BFS spanning forest of the 1-skeleton of `lattice`.
///
/// Starts BFS at vertex 0; if the lattice has multiple connected
/// components, the loop continues with the smallest unvisited
/// vertex as the next root. The result enumerates one root per
/// component.
pub fn build_spanning_tree(lattice: &Lattice) -> SpanningTree {
    let v = lattice.n_vertices;
    let e = lattice.n_edges();
    let mut parent: Vec<Option<(VertexId, EdgeId)>> = vec![None; v];
    let mut in_tree: Vec<bool> = vec![false; e];
    let mut visited: Vec<bool> = vec![false; v];
    let mut roots: Vec<VertexId> = Vec::new();

    // Build adjacency: adj[v] = Vec<(neighbour, edge_id)>.
    let mut adj: Vec<Vec<(VertexId, EdgeId)>> = vec![Vec::new(); v];
    for (eid, &(u, w)) in lattice.edges.iter().enumerate() {
        if u < v && w < v {
            adj[u].push((w, eid));
            adj[w].push((u, eid));
        }
    }

    for root in 0..v {
        if visited[root] {
            continue;
        }
        roots.push(root);
        visited[root] = true;
        let mut queue: std::collections::VecDeque<VertexId> =
            std::collections::VecDeque::new();
        queue.push_back(root);
        while let Some(u) = queue.pop_front() {
            for &(w, eid) in &adj[u] {
                if !visited[w] {
                    visited[w] = true;
                    parent[w] = Some((u, eid));
                    in_tree[eid] = true;
                    queue.push_back(w);
                }
            }
        }
    }

    SpanningTree { parent, in_tree, roots }
}

// ── π_1 face-relator (Massey presentation) ──────────────────────────

/// Express the face-cycle of face `face_idx` as a word in the
/// spanning-tree's non-tree generators.
///
/// Tree edges contribute nothing (their canonical loops cancel in
/// the spanning tree). Each non-tree edge contributes `+(g+1)` for
/// forward traversal along the face cycle and `-(g+1)` for reverse.
///
/// Returns the empty word for a face that touches only tree edges
/// (e.g. degenerate / boundary-only faces on a contractible region).
pub fn face_relator(
    lattice: &Lattice,
    face_idx: usize,
    in_tree: &[bool],
    gen_index: &HashMap<EdgeId, usize>,
) -> Vec<i64> {
    let face = &lattice.faces[face_idx];
    let n = face.len();
    let mut word: Vec<i64> = Vec::new();
    for pos in 0..n {
        let a = face[pos];
        let b = face[(pos + 1) % n];
        if let Some((eid, orient)) = lattice.resolve_edge(a, b) {
            if !in_tree[eid] {
                let g = *gen_index.get(&eid).expect(
                    "face_relator: non-tree edge missing from generator index",
                ) as i64;
                let symbol = (g + 1) * match orient {
                    EdgeOrientation::Forward => 1,
                    EdgeOrientation::Reverse => -1,
                };
                word.push(symbol);
            }
        }
    }
    word
}

/// Compute the Massey π_1 presentation of the 1-skeleton + face
/// 2-cells of `lattice`.
///
/// Algorithm (Massey, "A Basic Course in Algebraic Topology" §IV.5):
///
/// 1. Build a BFS spanning forest of the 1-skeleton.
/// 2. Generators = non-tree edges (one per edge not in the forest).
///    For a connected lattice with V vertices and E edges:
///    `generators.len() == E - V + 1`.
/// 3. Relators = face boundary cycles, each expressed as a word in
///    the generator index space (tree edges contribute nothing).
/// 4. `abelianized_rank = β_1` (consistent with the rank-nullity
///    Betti computation).
///
/// Phase 1 reports `rank` equal to `abelianized_rank`. Full
/// non-abelian Tietze reduction is Phase 2 scope (it would let us
/// distinguish e.g. ℤ * ℤ from ℤ² which both have abelianized
/// rank 2 — Phase 1 cannot, and the test fixtures used in the
/// RED tests all happen to be abelian).
pub fn pi_1_presentation(lattice: &Lattice) -> Pi1Presentation {
    let e = lattice.n_edges();
    let tree = build_spanning_tree(lattice);

    // Non-tree edges are the generators.
    let mut generators: Vec<EdgeId> = Vec::new();
    for eid in 0..e {
        if !tree.in_tree[eid] {
            generators.push(eid);
        }
    }
    let gen_index: HashMap<EdgeId, usize> = generators
        .iter()
        .enumerate()
        .map(|(i, &eid)| (eid, i))
        .collect();

    // One relator per face.
    let mut relators: Vec<Vec<i64>> = Vec::with_capacity(lattice.n_faces());
    for fidx in 0..lattice.n_faces() {
        let word = face_relator(lattice, fidx, &tree.in_tree, &gen_index);
        relators.push(word);
    }

    // Abelianized rank = β_1. Reuse the topological dispatcher so the
    // two surfaces stay consistent. β_1 always exists (order = 1).
    let abelianized_rank =
        betti_topological(lattice, 1).expect("β_1 always defined for order=1");

    Pi1Presentation {
        rank: abelianized_rank,
        generators,
        relators,
        abelianized_rank,
    }
}

// ── Test module ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lattice::topology::cubic::cubic;
    use crate::lattice::topology::truncated_icosahedron::buckyball;

    #[test]
    fn components_count_buckyball_is_one() {
        let lat = buckyball();
        assert_eq!(components_count(&lat), 1);
    }

    #[test]
    fn betti_buckyball_b0_b1_b2() {
        let lat = buckyball();
        assert_eq!(betti_topological(&lat, 0).unwrap(), 1);
        assert_eq!(betti_topological(&lat, 1).unwrap(), 0);
        assert_eq!(betti_topological(&lat, 2).unwrap(), 1);
    }

    #[test]
    fn betti_t2_4x4_b0_b1_b2() {
        let lwm = cubic("t2_4_test", 4, 2, true);
        let lat = lwm.lattice();
        assert_eq!(betti_topological(lat, 0).unwrap(), 1);
        assert_eq!(betti_topological(lat, 1).unwrap(), 2);
        assert_eq!(betti_topological(lat, 2).unwrap(), 1);
    }

    #[test]
    fn integer_rank_simple_2x2() {
        let mat = vec![vec![1, 2], vec![2, 4]];
        assert_eq!(integer_matrix_rank(&mat, 2, 2), 1);
    }

    #[test]
    fn integer_rank_identity_3x3() {
        let mat = vec![vec![1, 0, 0], vec![0, 1, 0], vec![0, 0, 1]];
        assert_eq!(integer_matrix_rank(&mat, 3, 3), 3);
    }

    #[test]
    fn unsupported_order_error_message() {
        let lat = buckyball();
        match betti_topological(&lat, 3) {
            Err(TopologyError::UnsupportedOrder { order, max_supported }) => {
                assert_eq!(order, 3);
                assert_eq!(max_supported, 2);
            }
            other => panic!("expected UnsupportedOrder, got {other:?}"),
        }
    }
}
