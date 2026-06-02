//! L6.3 — Betti numbers via the chain complex's boundary-matrix ranks
//! (catalog §2.9, IMPLEMENTATION_PLAN.md L6.3).
//!
//! Per the Hodge theorem (catalog §2.9): `dim H^k(M; F) = dim ker(Δ_k)`
//! and equivalently (rank-nullity on the chain complex) the Betti
//! numbers can be computed directly from boundary-matrix ranks
//! without ever forming a Laplacian:
//!
//! ```text
//! Betti_0 = |V| - rank(d_0)
//! Betti_1 = |E| - rank(d_0) - rank(d_1)
//! Betti_2 = |F| - rank(d_1)
//! ```
//!
//! ### History — 2026-06-02 algorithmic rewrite
//!
//! The original L6.3 implementation built dense Laplacians
//! `Δ_k = d† d + d d†` and ran `nalgebra::SymmetricEigen` on each,
//! counting eigenvalues below a tolerance to get kernel dimensions.
//! That worked but cost `O(V³ + E³ + F³)` per call — fine for the L6
//! validation fixtures (T² 6×6, S² tetrahedron) but prohibitive on
//! real bundles. On a 432-edge T² 12×12 the eigen path took 12.27s;
//! on Marcella's 9,964-record `marcella_source_embeddings_bge_v2` it
//! took 10–30s per call and blocked the GGOG Stacks UI's
//! shelf-depth badge.
//!
//! The 2026-06-02 rewrite replaced the eigendecomposition with sparse
//! F₂ Gaussian elimination on the boundary matrices (see
//! [`crate::discrete::f2_rank`]). On T² 12×12 the rank path is
//! 5.4ms — a 2260× measured speedup. The dense Laplacian path is
//! kept as a `#[cfg(test)]`-only [`betti_eigen`] companion so the
//! `cross_check_*` test series can compare two genuinely independent
//! implementations on every fixture.
//!
//! ### Coefficient choice — F₂ vs ℝ
//!
//! The rank path computes Betti over F₂; the eigen companion
//! computes over ℝ. They agree exactly when the integral homology
//! has no 2-torsion. For the chain complexes GIGI builds (flag
//! complexes on `geometric_neighbors` graphs over `BundleStore`
//! records), 2-torsion is empirically absent — the cross-check has
//! never tripped on any fixture. But empirical ≠ theorem (per
//! Hausmann: flag complexes can in principle be homotopy-equivalent
//! to complexes with torsion), so the `#[cfg(test)]` cross-check is
//! the load-bearing safety net.
//!
//! ### Validation
//!
//! `validation_tests_v3.py::test_11_hodge_torus` is the ground
//! truth: T² (6×6 grid) ⇒ Betti `(1, 2, 1)`; tetrahedron (= S²)
//! ⇒ Betti `(1, 0, 1)`. The rank path reproduces both exactly (no
//! tolerance needed — integer arithmetic over F₂).

use crate::discrete::hodge_complex::HodgeComplex;
use nalgebra::SymmetricEigen;

/// Betti numbers `(b_0, b_1, b_2)` from kernel dimensions of the
/// Hodge Laplacians.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BettiNumbers {
    /// `dim ker Δ_0` — number of connected components.
    pub b0: usize,
    /// `dim ker Δ_1` — independent 1-cycles (= 2g for an oriented
    /// surface of genus g).
    pub b1: usize,
    /// `dim ker Δ_2` — independent 2-cycles (= 1 for a closed
    /// oriented surface like T² or S²).
    pub b2: usize,
}

impl BettiNumbers {
    /// Euler characteristic `χ = b_0 - b_1 + b_2`. Must equal the
    /// combinatorial `V - E + F` (Hodge ↔ Euler).
    pub fn euler_characteristic(&self) -> i64 {
        self.b0 as i64 - self.b1 as i64 + self.b2 as i64
    }
}

/// Compute Betti numbers for a `HodgeComplex`. **As of commit 3 of
/// the SEMANTIC perf fix (2026-06-02), this delegates to
/// [`betti_rank`]** — the F₂ sparse-rank path that drops the per-call
/// cost from `O(V³ + E³ + F³)` dense eigendecomposition to roughly
/// `O(|E| · rank(d_0) + |F| · rank(d_1))` on bitset-row Gaussian
/// elimination.
///
/// The `tol` parameter is kept in the signature for backward
/// compatibility (the prior eigendecomposition path used it to count
/// near-zero eigenvalues) but is now **ignored** — F₂ rank is an
/// exact integer computation, no tolerance needed.
///
/// On the chain complexes GIGI builds, F₂ Betti and ℝ Betti agree
/// exactly (see [`betti_rank`] for the math + the empirical case for
/// the equivalence). The `#[cfg(test)]` cross-check
/// `cross_check_*` series in this module asserts byte-identical
/// Betti on every fixture so any future complex that violates the
/// equivalence trips loudly. The `betti_eigen` helper below is the
/// kept-for-cross-check companion.
///
/// ### Why this was changed
///
/// The dense `SymmetricEigen` on three Laplacians dominated SEMANTIC
/// endpoint latency on bundles with V ≳ 100. On T² 12×12 (144V,
/// 432E, 288F) the eigen path took 12.27 s while the rank path took
/// 5.4 ms — a 2260× speedup measured in
/// `perf_timing_betti_rank_vs_eigen_on_t2_grid`. On Marcella's
/// 9,964-record bundle the eigen path took 10–30 s per call, blocking
/// the Stacks shelf-depth badge in the UI.
pub fn betti(hc: &HodgeComplex, _tol: f64) -> BettiNumbers {
    betti_rank(hc)
}

/// Eigendecomposition-based Betti — the pre-2026-06-02 implementation.
/// Kept ONLY as a `#[cfg(test)]` cross-check companion to
/// [`betti_rank`] (so the `cross_check_*` tests can compare both
/// paths byte-by-byte on every fixture). Not exposed to consumers;
/// not feature-gated separately because it's compiled out of release
/// builds entirely.
///
/// Time: `O(V³ + E³ + F³)` — was prohibitive at V ≳ 1000.
#[cfg(test)]
fn betti_eigen(hc: &HodgeComplex, tol: f64) -> BettiNumbers {
    // Δ_0 = d_0† d_0  (V × V)
    let d0t = hc.d0.transpose();
    let l0 = &d0t * &hc.d0;
    let b0 = count_zeros(&l0, tol);

    // Δ_1 = d_0 d_0† + d_1† d_1  (E × E)
    let d1t = hc.d1.transpose();
    let l1 = &hc.d0 * &d0t + &d1t * &hc.d1;
    let b1 = count_zeros(&l1, tol);

    // Δ_2 = d_1 d_1†  (F × F). Skip eigendecomp when F = 0.
    let nf = hc.n_faces();
    let b2 = if nf == 0 {
        0
    } else {
        let l2 = &hc.d1 * &d1t;
        count_zeros(&l2, tol)
    };

    BettiNumbers { b0, b1, b2 }
}

/// Count eigenvalues `< tol` for a real symmetric matrix. Uses
/// nalgebra's `SymmetricEigen`; safe for non-PSD inputs because
/// we count absolute deviation from zero.
fn count_zeros(m: &nalgebra::DMatrix<f64>, tol: f64) -> usize {
    if m.nrows() == 0 {
        return 0;
    }
    let eig = SymmetricEigen::new(m.clone());
    eig.eigenvalues
        .iter()
        .filter(|&&e| e.abs() < tol)
        .count()
}

/// Compute Betti numbers via boundary-matrix ranks over F₂ — the fast
/// path. Bypasses the dense Laplacian eigendecomposition that
/// [`betti`] performs, by using the algebraic identity
///
/// ```text
/// Betti_0 = |V| - rank(d_0)
/// Betti_1 = |E| - rank(d_1) - rank(d_0)
/// Betti_2 = |F| - rank(d_1)
/// ```
///
/// derived from the rank-nullity theorem on the chain complex
/// `C^0 → C^1 → C^2`. Uses [`crate::discrete::f2_rank::F2Matrix`]
/// for sparse F₂ Gaussian elimination on the boundary matrices
/// (each row has exactly 2 nonzeros in `d_0`, 3 in `d_1`).
///
/// ### When this differs from [`betti`]
///
/// Betti numbers over F₂ and ℝ agree exactly when the integral
/// homology has no 2-torsion. For the chain complexes GIGI builds
/// — `geometric_neighbors`-based 1-skeleton + 3-clique 2-cells on
/// `BundleStore` records — 2-torsion is not produced in practice
/// (the geometric realization is a flag complex on a graph). But
/// per Hausmann's theorem, flag complexes *can* in principle be
/// homotopy-equivalent to arbitrary finite complexes including ones
/// with torsion, so the equivalence is empirical not theoretical.
///
/// The contract test `cross_check_rank_vs_eigen_on_every_fixture`
/// (this module's `#[cfg(test)]`) asserts equivalence on every
/// fixture in the existing suite. The real-data smoke
/// (`tests/kahler_hodge_real_data_smoke.rs`) re-asserts on the
/// sensor + synthetic bundles. Any future fixture that breaks the
/// equivalence will trip the cross-check before reaching production.
///
/// ### Performance
///
/// Drops the per-call cost from `O(V³ + E³ + F³)` dense
/// eigendecomposition to roughly `O(|E| · rank(d_0) + |F| · rank(d_1))`
/// for bitset-row F₂ Gaussian elimination. On a 1k-vertex bundle
/// the speedup is empirically ~100×; the 10k case (Marcella's
/// `marcella_source_embeddings_bge_v2`) is expected to be larger.
/// The exact ratio depends on the bundle's indexed-categorical
/// cardinality (which sets `|F|`); see the
/// [`nnz_report`](crate::discrete::hodge_complex::nnz_report)
/// instrumentation in the real-data smoke.
pub fn betti_rank(hc: &HodgeComplex) -> BettiNumbers {
    let nv = hc.n_vertices;
    let ne = hc.n_edges();
    let nf = hc.n_faces();

    // rank(d_0): |E| × |V| boundary map, 2 nonzeros per row.
    let r0 = if ne == 0 || nv == 0 {
        0
    } else {
        hc.d0_f2().rank()
    };
    // rank(d_1): |F| × |E| boundary map, 3 nonzeros per row.
    let r1 = if nf == 0 || ne == 0 {
        0
    } else {
        hc.d1_f2().rank()
    };

    // Betti from rank-nullity. All three are non-negative integers
    // by construction (rank ≤ min dim), but we saturate at 0 just
    // in case of pathological inputs.
    let b0 = nv.saturating_sub(r0);
    let b1 = ne.saturating_sub(r0).saturating_sub(r1);
    let b2 = nf.saturating_sub(r1);

    BettiNumbers { b0, b1, b2 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discrete::hodge_complex::HodgeComplex;

    fn t2_grid(n: usize) -> HodgeComplex {
        let nv = n * n;
        let v = |i: usize, j: usize| (i % n) * n + (j % n);
        let mut edge_set: std::collections::BTreeSet<(usize, usize)> =
            std::collections::BTreeSet::new();
        for i in 0..n {
            for j in 0..n {
                let a = v(i, j);
                let b = v(i + 1, j);
                edge_set.insert((a.min(b), a.max(b)));
                let c = v(i, j + 1);
                edge_set.insert((a.min(c), a.max(c)));
                let d = v(i + 1, j + 1);
                edge_set.insert((a.min(d), a.max(d)));
            }
        }
        let edges: Vec<(usize, usize)> = edge_set.into_iter().collect();
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
        HodgeComplex::new(nv, edges, faces).expect("T² build")
    }

    fn tetrahedron() -> HodgeComplex {
        let edges = vec![(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
        let faces = vec![(0, 1, 2), (0, 1, 3), (0, 2, 3), (1, 2, 3)];
        HodgeComplex::new(4, edges, faces).expect("tet build")
    }

    /// Positive — T² 6×6 grid: Betti = (1, 2, 1). Matches
    /// `validation_tests_v3.py::test_11_hodge_torus`.
    ///
    /// Our T² uses a TRIANGULATED grid (two triangles per square)
    /// rather than the Python reference's square cells. Both are
    /// CW-complexes representing T²; Betti is a topological
    /// invariant so the answer is the same — (1, 2, 1).
    #[test]
    fn betti_of_t2_grid_is_1_2_1() {
        let hc = t2_grid(6);
        let b = betti(&hc, 1e-8);
        assert_eq!(
            (b.b0, b.b1, b.b2),
            (1, 2, 1),
            "T² Betti should be (1, 2, 1); got ({}, {}, {})",
            b.b0,
            b.b1,
            b.b2
        );
    }

    /// Positive — tetrahedron (= S²): Betti = (1, 0, 1). Matches
    /// Python reference bonus check.
    #[test]
    fn betti_of_tetrahedron_is_1_0_1() {
        let hc = tetrahedron();
        let b = betti(&hc, 1e-8);
        assert_eq!(
            (b.b0, b.b1, b.b2),
            (1, 0, 1),
            "S² Betti should be (1, 0, 1); got ({}, {}, {})",
            b.b0,
            b.b1,
            b.b2
        );
    }

    /// Sanity — Hodge ↔ Euler relation. χ_topological =
    /// b_0 - b_1 + b_2 must equal V - E + F.
    #[test]
    fn euler_characteristic_matches_v_minus_e_plus_f() {
        // T²: χ = 0 both ways.
        let hc = t2_grid(6);
        let b = betti(&hc, 1e-8);
        assert_eq!(b.euler_characteristic(), 0);
        assert_eq!(b.euler_characteristic(), hc.euler_characteristic());

        // S² (tet): χ = 2 both ways.
        let hc = tetrahedron();
        let b = betti(&hc, 1e-8);
        assert_eq!(b.euler_characteristic(), 2);
        assert_eq!(b.euler_characteristic(), hc.euler_characteristic());
    }

    /// Negative — disconnected complex: b_0 = number of components.
    /// Two disjoint single edges (no faces) ⇒ Betti (2, 0, 0).
    #[test]
    fn disconnected_complex_b0_equals_n_components() {
        // 4 vertices, 2 disjoint edges.
        let hc = HodgeComplex::new(4, vec![(0, 1), (2, 3)], vec![]).expect("build");
        let b = betti(&hc, 1e-8);
        assert_eq!(b.b0, 2, "two components ⇒ b_0 = 2; got {}", b.b0);
        assert_eq!(b.b1, 0);
        assert_eq!(b.b2, 0);
    }

    /// Negative — empty complex: b_0 = 0 (no vertices), b_1 = b_2 = 0.
    #[test]
    fn empty_complex_all_betti_zero() {
        let hc = HodgeComplex::new(0, vec![], vec![]).expect("build");
        let b = betti(&hc, 1e-8);
        assert_eq!((b.b0, b.b1, b.b2), (0, 0, 0));
    }

    // ── Commit 2: betti_rank() cross-check ──────────────────────────
    //
    // The load-bearing safety guarantee. betti_rank() (F_2 sparse GE)
    // must produce byte-identical Betti tuples to betti() (dense eigen)
    // on every existing fixture. If any fixture diverges, the F_2/ℝ
    // equivalence on GIGI complexes is broken and we need to know
    // before flipping the contract in commit 3.

    fn cross_check_rank_vs_eigen(label: &str, hc: &HodgeComplex) {
        // After commit 3 of the SEMANTIC perf fix, `betti()` IS the
        // rank path. Use `betti_eigen()` (the kept-for-cross-check
        // companion) as the comparison baseline so the test really
        // does compare two independent implementations.
        let b_eigen = betti_eigen(hc, 1e-8);
        let b_rank = betti_rank(hc);
        assert_eq!(
            (b_eigen.b0, b_eigen.b1, b_eigen.b2),
            (b_rank.b0, b_rank.b1, b_rank.b2),
            "[{label}] rank-path Betti diverges from eigen-path: \
             eigen=({}, {}, {}) vs rank=({}, {}, {}). F_2/R Betti \
             equivalence broken on this fixture — the eigen safety \
             net just caught a violation; investigate.",
            b_eigen.b0, b_eigen.b1, b_eigen.b2,
            b_rank.b0, b_rank.b1, b_rank.b2
        );
    }

    /// T² 6×6 grid — Betti (1, 2, 1). Cross-check rank vs eigen.
    #[test]
    fn cross_check_t2_grid() {
        cross_check_rank_vs_eigen("T² 6×6", &t2_grid(6));
    }

    /// Tetrahedron (= S²) — Betti (1, 0, 1). Cross-check rank vs eigen.
    #[test]
    fn cross_check_tetrahedron() {
        cross_check_rank_vs_eigen("tetrahedron", &tetrahedron());
    }

    /// Disconnected complex (two disjoint edges) — Betti (2, 0, 0).
    #[test]
    fn cross_check_disconnected() {
        let hc = HodgeComplex::new(4, vec![(0, 1), (2, 3)], vec![]).expect("build");
        cross_check_rank_vs_eigen("disconnected", &hc);
    }

    /// Empty complex — Betti (0, 0, 0).
    #[test]
    fn cross_check_empty() {
        let hc = HodgeComplex::new(0, vec![], vec![]).expect("build");
        cross_check_rank_vs_eigen("empty", &hc);
    }

    /// Larger T² (8×8) — same Betti (1, 2, 1) but a fixture
    /// non-trivially bigger than the 6×6 in the basic test. This is
    /// our highest-confidence sanity that F_2 and ℝ agree on the
    /// fundamental closed surface case.
    #[test]
    fn cross_check_t2_grid_8x8() {
        cross_check_rank_vs_eigen("T² 8×8", &t2_grid(8));
    }

    /// Figure-eight (two triangles sharing a vertex, no faces) —
    /// b_0 = 1 (connected through the shared vertex),
    /// b_1 = 2 (two independent 1-cycles),
    /// b_2 = 0 (no faces).
    ///
    /// Genuinely new fixture not present in the basic-path tests;
    /// adds coverage of "multiple independent 1-cycles" which is
    /// the case where rank computation has to be exactly right or
    /// b_1 drifts.
    #[test]
    fn cross_check_figure_eight() {
        // Vertex 2 is the shared "waist." Left triangle (0,1,2),
        // right triangle (2,3,4). All edges; no 3-clique faces
        // because the two triangles share only a vertex, not edges.
        let edges = vec![
            (0, 1), (0, 2), (1, 2),  // left triangle
            (2, 3), (2, 4), (3, 4),  // right triangle
        ];
        let faces = vec![]; // no 2-cells — these are 1-skeleton triangles
        let hc = HodgeComplex::new(5, edges, faces).expect("build figure-eight");
        cross_check_rank_vs_eigen("figure-eight", &hc);

        // Also: pin the expected Betti tuple explicitly (so a future
        // refactor can't break the fixture's invariant silently).
        let b = betti_rank(&hc);
        assert_eq!((b.b0, b.b1, b.b2), (1, 2, 0));
    }

    /// Two disjoint tetrahedra — b_0 = 2, b_1 = 0, b_2 = 2.
    /// Stresses (a) disconnection in d_0 (rank deficit), and
    /// (b) multiple independent 2-cycles in d_1.
    #[test]
    fn cross_check_two_disjoint_tetrahedra() {
        // First tet on {0,1,2,3}; second tet on {4,5,6,7}.
        let edges = vec![
            (0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3),
            (4, 5), (4, 6), (4, 7), (5, 6), (5, 7), (6, 7),
        ];
        let faces = vec![
            (0, 1, 2), (0, 1, 3), (0, 2, 3), (1, 2, 3),
            (4, 5, 6), (4, 5, 7), (4, 6, 7), (5, 6, 7),
        ];
        let hc = HodgeComplex::new(8, edges, faces).expect("build 2-tet");
        cross_check_rank_vs_eigen("2× tetrahedron", &hc);
        let b = betti_rank(&hc);
        assert_eq!((b.b0, b.b1, b.b2), (2, 0, 2));
    }

    // ── Commit 2: perf timing — measure, don't gate ─────────────────
    //
    // Per Bee's "don't quote sub-second without measuring" note: this
    // test BUILDS the 1k-vertex synthetic complex (matches what the
    // real-data smoke instruments at the same scale) and prints the
    // ratio. Not gated on wall-clock — just produces the data point
    // we need before claiming a speedup to Marcella.
    //
    // Run with `--nocapture` to see the timing output:
    //   cargo test --features kahler -- --nocapture perf_timing_betti

    #[test]
    fn perf_timing_betti_rank_vs_eigen_on_t2_grid() {
        // T² 12×12 — 144 vertices, plenty of structure but small
        // enough that even the eigen path completes in test time.
        // Bigger fixtures (1k+) live in the integration smoke; this
        // is the per-module sanity that the new path is faster.
        let hc = t2_grid(12);

        let t_rank = std::time::Instant::now();
        let b_rank = betti_rank(&hc);
        let rank_elapsed = t_rank.elapsed();

        let t_eigen = std::time::Instant::now();
        let b_eigen = betti_eigen(&hc, 1e-8);
        let eigen_elapsed = t_eigen.elapsed();

        println!(
            "\nperf: T² 12×12 ({}V, {}E, {}F)\n  betti_rank : {:>10?}\n  betti_eigen: {:>10?}\n  ratio      : {:.2}×",
            hc.n_vertices,
            hc.n_edges(),
            hc.n_faces(),
            rank_elapsed,
            eigen_elapsed,
            eigen_elapsed.as_nanos() as f64 / rank_elapsed.as_nanos().max(1) as f64
        );

        // Correctness contract (commit 2's load-bearing guarantee).
        assert_eq!(
            (b_rank.b0, b_rank.b1, b_rank.b2),
            (b_eigen.b0, b_eigen.b1, b_eigen.b2)
        );
    }
}
