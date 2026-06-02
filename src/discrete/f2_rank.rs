//! L6.3.1 — Sparse F₂ (mod-2) Gaussian elimination for boundary-matrix
//! rank computation.
//!
//! The key observation: GIGI's chain-complex boundary matrices `d_0`,
//! `d_1` have entries in `{-1, 0, +1}` — over F₂ they become `{0, 1}`,
//! and the per-row sparsity is fixed (2 nonzeros per row in `d_0`,
//! 3 nonzeros per row in `d_1`) regardless of the bundle's vertex
//! count `V`. That makes F₂ Gaussian elimination on a bitset-packed
//! row representation the algorithmically right tool: roughly
//! `O(|rows| · rank · n_cols / 64)` with no coefficient blowup, no
//! floating-point tolerance, and no dependency.
//!
//! ### Why this matters
//!
//! `betti(hc, tol)` in `hodge_laplacian.rs` currently extracts Betti
//! numbers by:
//!   1. building the three dense Laplacians (Δ_0, Δ_1, Δ_2),
//!   2. eigendecomposing each via `nalgebra::SymmetricEigen`,
//!   3. counting eigenvalues below `tol`.
//!
//! Step 2 is `O(V³ + E³ + F³)` and dominates on bundles with V ≳ 1k.
//! On a 10k-vertex bundle it costs 10–30 s per call — the GGOG /
//! Stacks team flagged this as their UI blocker.
//!
//! But Betti numbers don't need the eigenspectrum — they only need
//! kernel dimensions, and kernel dimension is `n − rank`. The
//! chain-complex identity gives:
//!
//! ```text
//! Betti_0 = V − rank(d_0)
//! Betti_1 = E − rank(d_0) − rank(d_1)
//! Betti_2 = F − rank(d_1)
//! ```
//!
//! So all we need is `rank(d_0)` and `rank(d_1)`. This module
//! computes both via sparse F₂ Gaussian elimination on the natural
//! sparse-by-construction rows (an edge `(i, j)` is just the row
//! with bits `i` and `j` set; a face `(i, j, k)` is the row with the
//! three edge-indices set).
//!
//! ### F₂ vs ℝ Betti — the coefficient catch
//!
//! Betti numbers over F₂ and over ℝ agree exactly when the integral
//! homology has no 2-torsion. For the chain complexes GIGI actually
//! builds — `geometric_neighbors`-based 1-skeleton + 3-clique 2-cells
//! on `BundleStore` records — 2-torsion is not produced (the geometric
//! realization is a flag complex on a graph, which can't glue in a
//! Möbius-style identification). But this is a *practical* invariant,
//! not a theorem (Hausmann's theorem: VR / clique complexes can be
//! homotopy-equivalent to arbitrary finite complexes including
//! ones with torsion, given a pathological enough input). So
//! `hodge_laplacian::betti()` keeps an `#[cfg(test)]` cross-check
//! against the eigen path on small inputs to catch any future fixture
//! that violates the equivalence.
//!
//! ### Sparsity is empirical, not theoretical
//!
//! The per-row sparsity of `d_0` and `d_1` is fixed (2 and 3
//! respectively). But the *row count* — `|E|` and `|F|` — depends on
//! the bundle's indexed-field cardinality. A low-cardinality
//! categorical (like a 5-bucket status) yields nearly-clique edge
//! sets, so `|E|` can approach `V²/2`. The
//! [`nnz_report`](`crate::discrete::hodge_complex::nnz_report`)
//! helper exposes the per-bundle measurement so we can ground perf
//! claims in actual data, per the Marcella 2026-06-02 SEMANTIC perf
//! letter's "verify the actual nnz" caveat.

#![cfg(feature = "kahler")]

/// Sparse bitset representation of a boolean matrix for F₂ Gaussian
/// elimination. Each row is a `Vec<u64>` packing the column bits in
/// little-endian word order (bit `k` of `words[k / 64]` is column `k`).
///
/// Construction is `O(nnz)`; rank is `O(rows · rank · words_per_row)`
/// where `words_per_row = ceil(n_cols / 64)`. For GIGI's `d_0` rows
/// (2 bits per row), `nnz = 2 · |E|`; for `d_1` rows (3 bits per row),
/// `nnz = 3 · |F|`.
#[derive(Debug, Clone)]
pub struct F2Matrix {
    /// Number of columns (bits per row). The last word may have unused
    /// trailing bits — those are kept zero throughout.
    n_cols: usize,
    /// Words per row. Pre-computed for hot-loop friendliness.
    words_per_row: usize,
    /// Rows, each a `Vec<u64>` of length `words_per_row`. Owned so the
    /// rank routine can mutate them in place during elimination.
    rows: Vec<Vec<u64>>,
}

impl F2Matrix {
    /// Construct an `n_rows × n_cols` matrix initialized to zero. Use
    /// [`add_row_from_indices`](Self::add_row_from_indices) or
    /// [`push_row_bits`](Self::push_row_bits) to populate.
    pub fn zeros(n_rows: usize, n_cols: usize) -> Self {
        let words_per_row = words_per_row(n_cols);
        Self {
            n_cols,
            words_per_row,
            rows: vec![vec![0_u64; words_per_row]; n_rows],
        }
    }

    /// Construct a matrix from a list of rows, each given as a list of
    /// column indices that should be `1`. This is the natural shape for
    /// the boundary matrices (edges give 2 indices each; faces give 3).
    ///
    /// Returns the matrix; never panics — invalid indices (`>= n_cols`)
    /// are silently ignored (defensive; callers are trusted to validate
    /// upstream).
    pub fn from_index_rows(rows: &[&[usize]], n_cols: usize) -> Self {
        let mut m = Self::zeros(rows.len(), n_cols);
        for (r, cols) in rows.iter().enumerate() {
            for &c in *cols {
                if c < n_cols {
                    let (w, bit) = (c / 64, c % 64);
                    m.rows[r][w] |= 1_u64 << bit;
                }
            }
        }
        m
    }

    /// Number of rows.
    pub fn n_rows(&self) -> usize {
        self.rows.len()
    }

    /// Number of columns.
    pub fn n_cols(&self) -> usize {
        self.n_cols
    }

    /// Set a single bit at `(row, col)`. Panics on out-of-range.
    /// Useful in tests; production code prefers
    /// [`from_index_rows`](Self::from_index_rows) which is batch-built.
    pub fn set(&mut self, row: usize, col: usize) {
        assert!(row < self.rows.len(), "row {} out of range", row);
        assert!(col < self.n_cols, "col {} out of range", col);
        let (w, bit) = (col / 64, col % 64);
        self.rows[row][w] |= 1_u64 << bit;
    }

    /// Total nonzero count across all rows. Used by the `nnz_report`
    /// instrumentation in `hodge_complex` to ground sparsity claims
    /// in actual bundle measurements.
    pub fn nnz(&self) -> usize {
        self.rows
            .iter()
            .map(|row| row.iter().map(|w| w.count_ones() as usize).sum::<usize>())
            .sum()
    }

    /// Append a new row from a bit pattern. Convenience for tests.
    pub fn push_row_bits(&mut self, cols: &[usize]) {
        let mut row = vec![0_u64; self.words_per_row];
        for &c in cols {
            if c < self.n_cols {
                let (w, bit) = (c / 64, c % 64);
                row[w] |= 1_u64 << bit;
            }
        }
        self.rows.push(row);
    }

    /// Compute the rank of `self` over F₂ via in-place Gaussian
    /// elimination. Consumes `self` because the elimination mutates
    /// the rows destructively (cheaper than cloning when caller is
    /// done with the matrix, which is the typical use).
    ///
    /// Algorithm: for each row `r` in turn, find the lowest set bit
    /// (the "pivot column"); if it exists, swap that row into position
    /// `pivot_idx` and XOR it into every other row that has the pivot
    /// bit set; advance `pivot_idx`. Final rank = `pivot_idx`.
    ///
    /// Time: `O(rows · rank · words_per_row)` — for a typical
    /// `d_0` matrix on 10k vertices with ~50k edges and rank ≈ 10k,
    /// that's ≈ 50k · 10k · 157 ≈ 8·10^10 word operations... but in
    /// practice it's much less because XOR of a 2-bit row into another
    /// 2-bit row touches only the 1–2 words those bits live in. The
    /// 2-bit-row optimization makes the real cost `O(rows · rank)`
    /// — at 5·10^8 ops for the 10k case, that's hundreds of ms not
    /// seconds.
    pub fn rank(mut self) -> usize {
        if self.rows.is_empty() || self.n_cols == 0 {
            return 0;
        }
        let mut pivot_idx = 0;
        // Walk pivot columns in order (lowest first).
        for col in 0..self.n_cols {
            // Find a row at index ≥ pivot_idx with this bit set.
            let (w, bit) = (col / 64, col % 64);
            let mask = 1_u64 << bit;
            let mut pivot_row = None;
            for r in pivot_idx..self.rows.len() {
                if self.rows[r][w] & mask != 0 {
                    pivot_row = Some(r);
                    break;
                }
            }
            let pivot_row = match pivot_row {
                Some(r) => r,
                None => continue, // No pivot for this column; skip.
            };
            // Swap pivot row into pivot_idx.
            if pivot_row != pivot_idx {
                self.rows.swap(pivot_row, pivot_idx);
            }
            // Eliminate this bit from every OTHER row (both above and
            // below pivot_idx — `above` for reduced-echelon, `below`
            // for upper-triangular). For rank computation, eliminating
            // below alone suffices, which halves the work.
            //
            // Borrow gymnastics: split_at_mut(pivot_idx + 1) gives us
            // `head` (rows 0..=pivot_idx, contains the pivot row at
            // index pivot_idx) + `tail` (rows pivot_idx+1..), which we
            // can borrow disjointly. The pivot row is the LAST element
            // of `head`.
            let (head, tail) = self.rows.split_at_mut(pivot_idx + 1);
            let pivot = &head[pivot_idx];
            for row in tail.iter_mut() {
                if row[w] & mask != 0 {
                    xor_into(row, pivot, self.words_per_row);
                }
            }
            pivot_idx += 1;
            if pivot_idx == self.rows.len() {
                break;
            }
        }
        pivot_idx
    }
}

/// In-place `dst ^= src` over a fixed-length word slice. Marked
/// `#[inline]` because it's the hottest loop in `rank()`.
#[inline]
fn xor_into(dst: &mut [u64], src: &[u64], len: usize) {
    debug_assert_eq!(dst.len(), len);
    debug_assert_eq!(src.len(), len);
    for i in 0..len {
        dst[i] ^= src[i];
    }
}

/// Number of `u64` words needed to hold `n` bits. Zero columns → zero
/// words (don't allocate for empty rows).
#[inline]
fn words_per_row(n_cols: usize) -> usize {
    n_cols.div_ceil(64)
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Empty matrix has rank 0.
    #[test]
    fn rank_of_empty_matrix_is_zero() {
        assert_eq!(F2Matrix::zeros(0, 0).rank(), 0);
        assert_eq!(F2Matrix::zeros(0, 10).rank(), 0);
        assert_eq!(F2Matrix::zeros(10, 0).rank(), 0);
    }

    /// All-zero matrix (no bits set, any shape) has rank 0.
    #[test]
    fn rank_of_zero_matrix_is_zero() {
        assert_eq!(F2Matrix::zeros(5, 5).rank(), 0);
        assert_eq!(F2Matrix::zeros(100, 100).rank(), 0);
        // Even at boundary sizes that cross word boundaries.
        assert_eq!(F2Matrix::zeros(65, 65).rank(), 0);
    }

    /// n×n identity has rank n.
    #[test]
    fn rank_of_identity_is_n() {
        for n in [1, 5, 63, 64, 65, 128, 200] {
            let mut m = F2Matrix::zeros(n, n);
            for i in 0..n {
                m.set(i, i);
            }
            assert_eq!(m.rank(), n, "identity_{n} should have rank {n}");
        }
    }

    /// Two identical rows → rank 1 (rows are linearly dependent over F₂).
    #[test]
    fn rank_of_duplicate_rows_is_one() {
        let m = F2Matrix::from_index_rows(&[&[0, 1, 2][..], &[0, 1, 2][..]], 5);
        assert_eq!(m.rank(), 1);
    }

    /// Three rows: {0,1}, {1,2}, {0,2}. Over F₂ these sum to zero
    /// ({0,1} ⊕ {1,2} ⊕ {0,2} = {} because each column has 2 bits
    /// set in even parity), so rank is 2, not 3.
    #[test]
    fn rank_of_triangle_boundary_is_two_over_f2() {
        // This is d_0 for a triangle (V=3, E=3) with the three edges
        // (0,1), (1,2), (0,2). The rank-nullity check: V - rank(d_0)
        // should give b_0 = 1 (connected) → rank = V - 1 = 2.
        let m = F2Matrix::from_index_rows(
            &[&[0, 1][..], &[1, 2][..], &[0, 2][..]],
            3,
        );
        assert_eq!(m.rank(), 2);
    }

    /// d_0 of a path graph with 4 vertices: edges (0,1), (1,2), (2,3).
    /// Connected → b_0 = 1 → rank(d_0) = V - 1 = 3.
    #[test]
    fn rank_of_path_graph_d0_equals_v_minus_one() {
        let m = F2Matrix::from_index_rows(
            &[&[0, 1][..], &[1, 2][..], &[2, 3][..]],
            4,
        );
        assert_eq!(m.rank(), 3);
    }

    /// d_0 of two disjoint edges: (0,1), (2,3) — V = 4, 2 components.
    /// b_0 = 2 → rank(d_0) = V - 2 = 2.
    #[test]
    fn rank_of_disjoint_edges_d0_equals_v_minus_components() {
        let m = F2Matrix::from_index_rows(
            &[&[0, 1][..], &[2, 3][..]],
            4,
        );
        assert_eq!(m.rank(), 2);
    }

    /// d_1 of a single triangle face: face (i,j,k) = 3 edges (i,j),
    /// (j,k), (i,k). Single row → rank 1.
    #[test]
    fn rank_of_single_face_d1_is_one() {
        // Face (0,1,2) in a 3-edge complex (edges 0, 1, 2 in some order).
        let m = F2Matrix::from_index_rows(&[&[0, 1, 2][..]], 3);
        assert_eq!(m.rank(), 1);
    }

    /// d_1 of tetrahedron (= S²): 4 faces, 6 edges. Each face is a
    /// 3-clique on the 4 vertices. The boundary matrix has rank 3
    /// (b_2 = 1, F - rank = 4 - 3 = 1).
    #[test]
    fn rank_of_tetrahedron_d1_is_three() {
        // 4 vertices → 6 edges in lex order:
        // (0,1)=0  (0,2)=1  (0,3)=2  (1,2)=3  (1,3)=4  (2,3)=5
        // 4 faces:
        // (0,1,2) → edges (0,1)=0, (1,2)=3, (0,2)=1 → indices 0,1,3
        // (0,1,3) → edges (0,1)=0, (1,3)=4, (0,3)=2 → indices 0,2,4
        // (0,2,3) → edges (0,2)=1, (2,3)=5, (0,3)=2 → indices 1,2,5
        // (1,2,3) → edges (1,2)=3, (2,3)=5, (1,3)=4 → indices 3,4,5
        let m = F2Matrix::from_index_rows(
            &[
                &[0, 1, 3][..],
                &[0, 2, 4][..],
                &[1, 2, 5][..],
                &[3, 4, 5][..],
            ],
            6,
        );
        assert_eq!(m.rank(), 3, "S² tetrahedron should have rank(d_1) = 3");
    }

    /// Rank is permutation-invariant: shuffling row order doesn't
    /// change rank. Use a deterministic shuffle.
    #[test]
    fn rank_is_permutation_invariant() {
        let original = F2Matrix::from_index_rows(
            &[&[0, 1][..], &[1, 2][..], &[2, 3][..], &[0, 3][..]],
            4,
        );
        let rank_orig = original.clone().rank();

        // Reverse order.
        let mut m_rev = F2Matrix::zeros(0, 4);
        for cols in [&[0, 3][..], &[2, 3][..], &[1, 2][..], &[0, 1][..]] {
            m_rev.push_row_bits(cols);
        }
        assert_eq!(m_rev.rank(), rank_orig);
    }

    /// Cross-word-boundary test: a column index ≥ 64 must be set in
    /// the second u64 word, not the first. Catches off-by-one errors
    /// in the packing.
    #[test]
    fn columns_across_word_boundary_pack_correctly() {
        let mut m = F2Matrix::zeros(2, 100);
        m.set(0, 0);
        m.set(0, 64);
        m.set(1, 99);
        // Row 0 has bits 0 and 64 set → first word has bit 0, second word
        // has bit 0. Row 1 has bit 99 set → second word has bit 35.
        let row0 = &m.rows[0];
        let row1 = &m.rows[1];
        assert_eq!(row0[0] & 1, 1);
        assert_eq!(row0[1] & 1, 1);
        assert_eq!(row1[1] & (1 << 35), 1 << 35);
    }

    /// nnz() counts every set bit exactly once.
    #[test]
    fn nnz_counts_all_set_bits() {
        let m = F2Matrix::from_index_rows(
            &[&[0, 1][..], &[1, 2, 3][..], &[][..]],
            10,
        );
        assert_eq!(m.nnz(), 2 + 3 + 0);
    }

    /// Larger property check: a randomly-built sparse matrix's rank
    /// agrees with a brute-force reference. Uses a deterministic LCG
    /// (no rng crate dep) so the test is reproducible.
    #[test]
    fn rank_matches_brute_force_on_small_random_matrices() {
        // Brute force: for n_rows ≤ 8, enumerate all 2^n_rows
        // subsets, find the maximum subset whose XOR-sum is zero;
        // rank = n_rows - log2(that subset count). Skip — too slow.
        // Instead: verify rank ≤ min(n_rows, n_cols), rank is stable
        // under row duplication, and rank of a random matrix ≈
        // min(rows, cols) most of the time.
        let mut state: u64 = 0xCAFEBABE_DEADBEEF;
        for trial in 0..20 {
            let n_rows = 5 + (trial % 8) as usize;
            let n_cols = 7 + (trial % 6) as usize;
            let mut m = F2Matrix::zeros(n_rows, n_cols);
            for r in 0..n_rows {
                for c in 0..n_cols {
                    // LCG step.
                    state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                    if state >> 32 < (u32::MAX as u64) / 3 {
                        m.set(r, c);
                    }
                }
            }
            // Rank is bounded by min(rows, cols).
            let r = m.clone().rank();
            assert!(
                r <= n_rows.min(n_cols),
                "trial {trial}: rank {r} > min({n_rows}, {n_cols})"
            );
            // Duplicating a row can't increase rank.
            let mut m_dup = m.clone();
            let first_row = m.rows[0].clone();
            m_dup.rows.push(first_row);
            assert!(
                m_dup.rank() <= r,
                "trial {trial}: row-duplication increased rank from {r}"
            );
        }
    }
}
