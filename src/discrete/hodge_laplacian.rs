//! L6.3 — Hodge Laplacians `Δ_k = d† d + d d†` + Betti numbers via
//! eigendecomposition (catalog §2.9, IMPLEMENTATION_PLAN.md L6.3).
//!
//! Per the Hodge theorem (catalog §2.9): `dim H^k(M; ℝ) = dim
//! ker(Δ_k)`. We compute each kernel dimension by eigendecomposing
//! the Laplacian and counting eigenvalues below a tolerance.
//!
//! ### Formulas
//!
//! ```text
//! Δ_0 = d_0† d_0                       (V × V)
//! Δ_1 = d_0 d_0† + d_1† d_1            (E × E)
//! Δ_2 = d_1 d_1†                       (F × F)
//! ```
//!
//! The Laplacians are real symmetric positive semi-definite. We
//! use nalgebra's `SymmetricEigen` which gives real eigenvalues
//! sorted ascending.
//!
//! ### Validation
//!
//! `validation_tests_v3.py::test_11_hodge_torus` is the ground
//! truth: T² (6×6 grid) ⇒ Betti `(1, 2, 1)`; tetrahedron (= S²)
//! ⇒ Betti `(1, 0, 1)`. Our Rust must reproduce both to within
//! a `1e-8` eigenvalue tolerance (matching Python's `tol = 1e-8`).

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

/// Compute Betti numbers for a `HodgeComplex` with a given
/// eigenvalue tolerance.
///
/// The Python reference uses `tol = 1e-8`; we expose it so callers
/// can tighten or loosen per their context. Eigenvalues below
/// `tol` count as zero.
///
/// Time: `O(V³ + E³ + F³)` for the three eigendecompositions —
/// fine for the bundles + synthetic tests at L6 scale.
pub fn betti(hc: &HodgeComplex, tol: f64) -> BettiNumbers {
    let nv = hc.n_vertices;
    let ne = hc.n_edges();
    let nf = hc.n_faces();

    // Δ_0 = d_0† d_0  (V × V)
    let d0t = hc.d0.transpose();
    let l0 = &d0t * &hc.d0;
    let b0 = count_zeros(&l0, tol);

    // Δ_1 = d_0 d_0† + d_1† d_1  (E × E)
    let d1t = hc.d1.transpose();
    let l1 = &hc.d0 * &d0t + &d1t * &hc.d1;
    let b1 = count_zeros(&l1, tol);

    // Δ_2 = d_1 d_1†  (F × F). Skip eigendecomp when F = 0.
    let b2 = if nf == 0 {
        0
    } else {
        let l2 = &hc.d1 * &d1t;
        count_zeros(&l2, tol)
    };

    let _ = (nv, ne); // dimensions used only for sanity; eat the
                     // unused-binding lint.
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
}
