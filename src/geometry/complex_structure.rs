//! Almost-complex structure `J: TₚM → TₚM` with `J² = -I`.
//!
//! On a Kähler manifold the structure additionally satisfies
//! `∇J = 0` (J is parallel under the Chern connection); that
//! condition is enforced at higher layers when J is paired with
//! a connection. Here in L1 we only validate the algebraic
//! identity `J² = -I` (the "almost-complex" condition).
//!
//! References:
//! - `theory/kahler_upgrade/catalog.md §1` (the generator 𝒢)
//! - `theory/kahler_upgrade/IMPLEMENTATION_PLAN.md` L1.2

use thiserror::Error;

/// Maximum element-wise deviation from `-I` allowed when verifying
/// `J² = -I` numerically. Slack accommodates floating-point error
/// in user-supplied matrices; tightened beyond this means the matrix
/// is genuinely not an almost-complex structure rather than just
/// computationally inexact.
const J_SQUARED_TOLERANCE: f64 = 1e-10;

/// Reasons a candidate matrix can't be promoted to a
/// `ComplexStructure`. Returned by `ComplexStructure::new`.
#[derive(Debug, Error, PartialEq)]
pub enum ComplexStructureError {
    /// The matrix is not square.
    #[error("complex structure must be a square matrix; got {rows}x{cols}")]
    NotSquare { rows: usize, cols: usize },

    /// `J²` differs from `-I` by more than the tolerance. The
    /// reported `max_dev` lets the caller decide whether to
    /// retighten user input or reject outright.
    #[error("J² ≠ -I: max element-wise deviation {max_dev:.3e} exceeds tolerance {tolerance:.3e}")]
    NotAlmostComplex { max_dev: f64, tolerance: f64 },

    /// Dimension is 0 — nothing to represent.
    #[error("complex structure must have positive dimension")]
    EmptyDimension,

    /// Real almost-complex structures only exist on even-dimensional
    /// tangent spaces. (Odd-dim → no global square root of -I in
    /// matrix form; `det(J)² = det(-I) = (-1)^n` forces n even for
    /// J real.) Reject early with a clear message rather than
    /// failing the J² check downstream.
    #[error("almost-complex structure requires even dimension; got {dim}")]
    OddDimension { dim: usize },
}

/// An almost-complex structure on a real vector space of even
/// dimension 2n. Stored row-major as a `2n × 2n` matrix of
/// `f64` (small dim — fiber tangent spaces are typically ≤ 8
/// in our application). For larger dims a sparse representation
/// would replace this; the public API hides the choice.
///
/// `PartialEq` compares the matrix element-wise. Fine for our use
/// case since J is either equal by construction (deterministic
/// inputs, including `standard(...)`) or different by non-trivial
/// amounts (different complex structures). Don't rely on `==` for
/// J matrices differing by ulp-level FP noise.
#[derive(Debug, Clone, PartialEq)]
pub struct ComplexStructure {
    dim: usize,
    /// Row-major `dim × dim` matrix.
    data: Vec<f64>,
}

impl ComplexStructure {
    /// Construct a complex structure from a row-major matrix.
    /// Validates squareness, even dimension, and the `J² = -I`
    /// identity (within tolerance). Returns
    /// [`ComplexStructureError`] for any failure so the caller can
    /// surface a meaningful diagnostic rather than a silent panic.
    pub fn new(data: Vec<f64>, dim: usize) -> Result<Self, ComplexStructureError> {
        if dim == 0 {
            return Err(ComplexStructureError::EmptyDimension);
        }
        if data.len() != dim * dim {
            return Err(ComplexStructureError::NotSquare {
                rows: if dim == 0 { 0 } else { data.len() / dim.max(1) },
                cols: dim,
            });
        }
        if dim % 2 != 0 {
            return Err(ComplexStructureError::OddDimension { dim });
        }

        // Compute J² and check it equals -I element-wise.
        let j_sq = matmul(&data, &data, dim);
        let mut max_dev = 0.0_f64;
        for i in 0..dim {
            for j in 0..dim {
                let expected = if i == j { -1.0 } else { 0.0 };
                let dev = (j_sq[i * dim + j] - expected).abs();
                if dev > max_dev {
                    max_dev = dev;
                }
            }
        }
        if max_dev > J_SQUARED_TOLERANCE {
            return Err(ComplexStructureError::NotAlmostComplex {
                max_dev,
                tolerance: J_SQUARED_TOLERANCE,
            });
        }

        Ok(Self { dim, data })
    }

    /// The canonical complex structure on R^{2n}: rotation by 90°
    /// in each (x_k, y_k) plane. In block form,
    /// `J = diag([[0, -1], [1, 0]] × n)`. Useful as a default and
    /// as a test anchor.
    pub fn standard(half_dim: usize) -> Self {
        let dim = 2 * half_dim;
        let mut data = vec![0.0_f64; dim * dim];
        for k in 0..half_dim {
            // J(e_{2k}) = e_{2k+1}, J(e_{2k+1}) = -e_{2k}.
            // Row 2k+1, col 2k = 1; row 2k, col 2k+1 = -1.
            data[(2 * k + 1) * dim + 2 * k] = 1.0;
            data[2 * k * dim + (2 * k + 1)] = -1.0;
        }
        // SAFETY: We constructed this matrix to satisfy J² = -I
        // exactly; new() should never fail. Unwrap is the right
        // signal — if it ever fails, the construction is buggy.
        Self::new(data, dim).expect("standard J should satisfy J² = -I exactly")
    }

    /// Dimension `dim = 2n` of the underlying real vector space.
    pub fn dim(&self) -> usize {
        self.dim
    }

    /// Borrow the row-major matrix data. The structure stays
    /// immutable post-construction so callers can rely on the
    /// invariant.
    pub fn matrix(&self) -> &[f64] {
        &self.data
    }

    /// Apply `J` to a tangent vector `v ∈ R^dim`. Returns `J·v`.
    /// Panics if `v.len() != dim`; this is a programmer error
    /// rather than a runtime data error, so panic is appropriate.
    pub fn apply(&self, v: &[f64]) -> Vec<f64> {
        assert_eq!(
            v.len(),
            self.dim,
            "vector length {} doesn't match J's dimension {}",
            v.len(),
            self.dim
        );
        let mut out = vec![0.0_f64; self.dim];
        for i in 0..self.dim {
            let mut sum = 0.0_f64;
            for j in 0..self.dim {
                sum += self.data[i * self.dim + j] * v[j];
            }
            out[i] = sum;
        }
        out
    }
}

/// Naive O(n³) matrix multiply for square row-major matrices.
/// Fine at our dimensions (≤ 8 typical); replace with a BLAS call
/// if we ever need to handle large tangent spaces.
fn matmul(a: &[f64], b: &[f64], dim: usize) -> Vec<f64> {
    let mut out = vec![0.0_f64; dim * dim];
    for i in 0..dim {
        for j in 0..dim {
            let mut sum = 0.0_f64;
            for k in 0..dim {
                sum += a[i * dim + k] * b[k * dim + j];
            }
            out[i * dim + j] = sum;
        }
    }
    out
}

// ── Tests (red-first per IMPLEMENTATION_PLAN §0) ─────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Positive: the canonical J on R² is a 90° rotation matrix
    /// `[[0, -1], [1, 0]]`; its square is `-I`. Should construct
    /// cleanly via `standard(1)` AND `new(...)`.
    #[test]
    fn standard_j_on_r2_is_rotation_by_90() {
        let j = ComplexStructure::standard(1);
        assert_eq!(j.dim(), 2);

        // J · e_0 = e_1
        assert_eq!(j.apply(&[1.0, 0.0]), vec![0.0, 1.0]);
        // J · e_1 = -e_0
        assert_eq!(j.apply(&[0.0, 1.0]), vec![-1.0, 0.0]);
        // J² · e_0 = -e_0
        let je0 = j.apply(&[1.0, 0.0]);
        assert_eq!(j.apply(&je0), vec![-1.0, 0.0]);
    }

    /// Positive: the algebraic identity `J² = -I` holds at machine
    /// precision for the standard J at multiple dimensions.
    #[test]
    fn j_squared_is_neg_identity() {
        for half_dim in 1..=4 {
            let j = ComplexStructure::standard(half_dim);
            let dim = j.dim();
            let j_sq = matmul(j.matrix(), j.matrix(), dim);
            for i in 0..dim {
                for k in 0..dim {
                    let expected = if i == k { -1.0 } else { 0.0 };
                    assert!(
                        (j_sq[i * dim + k] - expected).abs() < 1e-15,
                        "(J²)[{},{}] = {} for half_dim={}, expected {}",
                        i,
                        k,
                        j_sq[i * dim + k],
                        half_dim,
                        expected
                    );
                }
            }
        }
    }

    /// Negative: a random non-almost-complex matrix must be
    /// rejected. The identity matrix is a clean negative — its
    /// square is `+I`, not `-I`, so the constructor must refuse
    /// rather than silently accept.
    #[test]
    fn rejects_non_almost_complex() {
        // The 2×2 identity. I² = I, not -I.
        let identity_2 = vec![1.0, 0.0, 0.0, 1.0];
        match ComplexStructure::new(identity_2, 2) {
            Err(ComplexStructureError::NotAlmostComplex { max_dev, .. }) => {
                // Deviation should be ≈ 2 (each diagonal entry is
                // off by 2: got +1, expected -1).
                assert!(
                    (max_dev - 2.0).abs() < 1e-12,
                    "expected max_dev ≈ 2.0 for identity² = I, got {}",
                    max_dev
                );
            }
            other => panic!("expected NotAlmostComplex error, got {:?}", other),
        }

        // A random asymmetric junk matrix — also not almost-complex.
        let junk = vec![1.0, 2.0, 3.0, 4.0];
        match ComplexStructure::new(junk, 2) {
            Err(ComplexStructureError::NotAlmostComplex { .. }) => { /* ok */ }
            other => panic!("expected NotAlmostComplex error for junk, got {:?}", other),
        }
    }

    /// Negative: odd dimension is impossible for a real
    /// almost-complex structure.
    #[test]
    fn rejects_odd_dimension() {
        // 3×3 anything — odd dim is rejected before J² check fires.
        let junk_3 = vec![0.0; 9];
        assert_eq!(
            ComplexStructure::new(junk_3, 3),
            Err(ComplexStructureError::OddDimension { dim: 3 })
        );
    }

    /// Negative: dimension 0 is rejected outright.
    #[test]
    fn rejects_empty_dimension() {
        assert_eq!(
            ComplexStructure::new(vec![], 0),
            Err(ComplexStructureError::EmptyDimension)
        );
    }

    /// Negative: dimension mismatch (data length doesn't match
    /// dim × dim) is rejected. Catches the common bug of passing
    /// a 2×3 matrix and claiming dim=2.
    #[test]
    fn rejects_non_square_data() {
        let bad = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]; // length 6
        let result = ComplexStructure::new(bad, 2); // 2*2 = 4, not 6
        assert!(
            matches!(result, Err(ComplexStructureError::NotSquare { .. })),
            "expected NotSquare error, got {:?}",
            result
        );
    }

    /// Positive: `apply` is consistent with the matrix
    /// representation. `J · J · v = -v` for any v in the standard
    /// basis (and by linearity, for any v).
    #[test]
    fn apply_applied_twice_negates_input() {
        let j = ComplexStructure::standard(2); // 4-dim
        for k in 0..4 {
            let mut e_k = vec![0.0; 4];
            e_k[k] = 1.0;
            let je = j.apply(&e_k);
            let j_je = j.apply(&je);
            let expected: Vec<f64> = e_k.iter().map(|x| -x).collect();
            for i in 0..4 {
                assert!(
                    (j_je[i] - expected[i]).abs() < 1e-15,
                    "J²(e_{}) component {} = {}, expected {}",
                    k,
                    i,
                    j_je[i],
                    expected[i]
                );
            }
        }
    }
}
