//! 2-forms on a tangent space, with closedness verification.
//!
//! A 2-form `B ∈ Ω²(M)` is a smooth assignment of an antisymmetric
//! bilinear form `B_p: TₚM × TₚM → ℝ` at each point p. Locally, in
//! a coordinate chart with basis `{dx¹, …, dxⁿ}`, we represent
//! `B = ½ Σ_{i,j} B_{ij}(p) dxⁱ ∧ dxʲ` with `B_{ij} = -B_{ji}`.
//!
//! For the Kähler upgrade we need a *constant* 2-form per fiber
//! (the magnetic bias B in the generator 𝒢; see catalog §1.2),
//! so this module models point-wise constant coefficients
//! `B_{ij} ∈ ℝ` stored as an antisymmetric `n × n` matrix.
//!
//! **Closedness.** `B` is *closed* if `dB = 0`. For a constant
//! 2-form on flat coordinates, `dB = 0` automatically — there's
//! nothing to differentiate. The closedness check matters when B
//! is non-constant; here we keep the API ready for that case via
//! [`ClosedTwoForm::new_constant`] (trivially closed) and an
//! extensible `new_with_d_check` path that delegates to a
//! caller-supplied discrete exterior derivative (left for L6 when
//! we wire the Hodge complex in).
//!
//! References:
//! - `theory/kahler_upgrade/catalog.md §1.2` (magnetic 2-form bias)
//! - `theory/kahler_upgrade/IMPLEMENTATION_PLAN.md` L1.3

use thiserror::Error;

/// Maximum allowed antisymmetry deviation: `|B_{ij} + B_{ji}|` for
/// each (i, j) must be below this. Small floating-point asymmetry
/// from user input gets snapped to exact antisymmetry on
/// construction; gross asymmetry is rejected.
const ANTISYMMETRY_TOLERANCE: f64 = 1e-10;

/// Maximum allowed `‖dB‖` for the closedness check on a non-constant
/// 2-form. Cf. catalog §E.5 closedness threshold for Marcella
/// pre-flight: `ε < 1e-3 × ‖B‖`. We default to 1e-10 here for
/// internally-constructed forms; the L6 hodge complex check applies
/// a looser threshold for user-supplied learned forms.
const DEFAULT_CLOSEDNESS_TOLERANCE: f64 = 1e-10;

/// Reasons a candidate matrix isn't a valid 2-form, or a 2-form
/// isn't closed.
#[derive(Debug, Error, PartialEq)]
pub enum ClosednessError {
    /// Not square or wrong size for the declared dimension.
    #[error("2-form must be square; got {len} entries for dim={dim} (expected {})", dim * dim)]
    NotSquare { len: usize, dim: usize },

    /// Element-wise antisymmetry `B_{ij} = -B_{ji}` violated.
    #[error("2-form not antisymmetric: max |B[i,j] + B[j,i]| = {max_dev:.3e} exceeds tolerance {tolerance:.3e}")]
    NotAntisymmetric { max_dev: f64, tolerance: f64 },

    /// `dB` exceeded the closedness tolerance.
    #[error("2-form not closed: ‖dB‖ = {norm:.3e} exceeds tolerance {tolerance:.3e}")]
    NotClosed { norm: f64, tolerance: f64 },

    /// Dimension is 0.
    #[error("2-form must have positive dimension")]
    EmptyDimension,
}

/// A 2-form on `Rⁿ` with constant coefficients, stored as the
/// antisymmetric `n × n` matrix `B_{ij}`. Constructor enforces
/// antisymmetry within tolerance and snaps to exact antisymmetry
/// (averaging `½(B_{ij} - B_{ji})`) so downstream consumers can
/// rely on the invariant byte-for-byte.
///
/// `PartialEq` compares the post-symmetrization coefficient matrix
/// element-wise — fine for our use case where 2-forms are either
/// equal by construction (deterministic inputs) or different by
/// non-trivial amounts (different bias configurations). Don't
/// rely on `==` for forms differing by ulp-level FP noise.
#[derive(Debug, Clone, PartialEq)]
pub struct TwoForm {
    dim: usize,
    /// Row-major `dim × dim`, antisymmetric.
    data: Vec<f64>,
}

impl TwoForm {
    /// Construct a 2-form from row-major matrix data. Antisymmetry
    /// is verified and the matrix is symmetrized (averaged with its
    /// negated transpose) to remove floating-point asymmetry.
    pub fn new(raw: Vec<f64>, dim: usize) -> Result<Self, ClosednessError> {
        if dim == 0 {
            return Err(ClosednessError::EmptyDimension);
        }
        if raw.len() != dim * dim {
            return Err(ClosednessError::NotSquare {
                len: raw.len(),
                dim,
            });
        }

        // Antisymmetry check + symmetrization.
        let mut data = vec![0.0_f64; dim * dim];
        let mut max_dev = 0.0_f64;
        for i in 0..dim {
            for j in 0..dim {
                let bij = raw[i * dim + j];
                let bji = raw[j * dim + i];
                let dev = (bij + bji).abs();
                if dev > max_dev {
                    max_dev = dev;
                }
                // Symmetrize: B[i,j] := ½(B[i,j] - B[j,i]). Snaps
                // diagonal to zero exactly.
                data[i * dim + j] = 0.5 * (bij - bji);
            }
        }
        if max_dev > ANTISYMMETRY_TOLERANCE {
            return Err(ClosednessError::NotAntisymmetric {
                max_dev,
                tolerance: ANTISYMMETRY_TOLERANCE,
            });
        }

        Ok(Self { dim, data })
    }

    /// Dimension of the underlying vector space (`n` in `Rⁿ`).
    pub fn dim(&self) -> usize {
        self.dim
    }

    /// Borrow the antisymmetric coefficient matrix.
    pub fn matrix(&self) -> &[f64] {
        &self.data
    }

    /// Frobenius norm `‖B‖ = √Σ B_{ij}²`. Used as the denominator
    /// in relative closedness checks elsewhere (cf. catalog §E.5).
    pub fn frobenius_norm(&self) -> f64 {
        self.data.iter().map(|x| x * x).sum::<f64>().sqrt()
    }

    /// Evaluate `B(u, v) = Σᵢⱼ B_{ij} uⁱ vʲ` for tangent vectors
    /// u, v ∈ `Rⁿ`. Panics on length mismatch — programmer error.
    pub fn apply(&self, u: &[f64], v: &[f64]) -> f64 {
        assert_eq!(u.len(), self.dim, "u length mismatch");
        assert_eq!(v.len(), self.dim, "v length mismatch");
        let mut sum = 0.0_f64;
        for i in 0..self.dim {
            for j in 0..self.dim {
                sum += self.data[i * self.dim + j] * u[i] * v[j];
            }
        }
        sum
    }

    /// Zero 2-form of given dimension. Trivially antisymmetric and
    /// closed; useful as a default bias (no magnetic perturbation).
    pub fn zero(dim: usize) -> Self {
        Self::new(vec![0.0; dim * dim], dim).expect("zero 2-form construction can't fail")
    }
}

/// A 2-form known to be closed (`dB = 0`). Construction validates
/// the closedness condition via either a trivial route (constant
/// forms on flat charts) or a caller-supplied discrete exterior
/// derivative. Wrapping `TwoForm` rather than re-implementing
/// keeps the antisymmetry invariant inherited.
#[derive(Debug, Clone, PartialEq)]
pub struct ClosedTwoForm {
    inner: TwoForm,
}

impl ClosedTwoForm {
    /// Promote a constant-coefficient 2-form to a closed one
    /// **without** running a discrete-d check.
    ///
    /// Why this is sound: on a flat chart with constant
    /// coefficients `B_{ij}`, the exterior derivative
    /// `dB = Σ_{i<j<k} (∂_k B_{ij} − ∂_j B_{ik} + ∂_i B_{jk}) dx^i∧dx^j∧dx^k`
    /// vanishes identically because every partial derivative of a
    /// constant is zero. So a `TwoForm` (constant by storage) is
    /// closed by construction.
    ///
    /// This is the right constructor for L1's magnetic-bias use
    /// case — the bias B in 𝒢 is one constant antisymmetric matrix
    /// per fiber. Non-constant B's (learned positional forms in
    /// Marcella, for instance) take the `new_with_discrete_d` path
    /// once L6 ships the Hodge complex.
    pub fn new_constant(form: TwoForm) -> Self {
        Self { inner: form }
    }

    /// Construct a closed 2-form, verifying closedness by a
    /// caller-supplied "exterior derivative" function.
    ///
    /// The caller passes `d_norm: impl FnOnce(&TwoForm) -> f64`
    /// that returns `‖dB‖` in whatever discrete sense applies
    /// (cell-incidence-based, FFT-based, finite-difference, etc.).
    /// This module doesn't know about cell complexes — that
    /// machinery is L6 — so we accept the norm as a callback.
    ///
    /// `tolerance` defaults to [`DEFAULT_CLOSEDNESS_TOLERANCE`]
    /// when `None`. Pass `Some(...)` for looser bounds (e.g.
    /// learned forms per catalog §E.5).
    pub fn new_with_discrete_d<F>(
        form: TwoForm,
        d_norm: F,
        tolerance: Option<f64>,
    ) -> Result<Self, ClosednessError>
    where
        F: FnOnce(&TwoForm) -> f64,
    {
        let tol = tolerance.unwrap_or(DEFAULT_CLOSEDNESS_TOLERANCE);
        let n = d_norm(&form);
        if n > tol {
            return Err(ClosednessError::NotClosed {
                norm: n,
                tolerance: tol,
            });
        }
        Ok(Self { inner: form })
    }

    /// Zero closed 2-form (trivial bias).
    pub fn zero(dim: usize) -> Self {
        Self::new_constant(TwoForm::zero(dim))
    }

    /// Borrow the underlying 2-form for evaluation / coefficient
    /// inspection.
    pub fn form(&self) -> &TwoForm {
        &self.inner
    }

    /// Dimension of the underlying vector space.
    pub fn dim(&self) -> usize {
        self.inner.dim()
    }

    /// Evaluate `B(u, v)`. Forwarded to the inner form.
    pub fn apply(&self, u: &[f64], v: &[f64]) -> f64 {
        self.inner.apply(u, v)
    }
}

// ── Tests (red-first per IMPLEMENTATION_PLAN §0) ─────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── TwoForm constructor ──

    /// Positive: a clean antisymmetric `[[0, 1.5], [-1.5, 0]]`
    /// constructs and `B(e_0, e_1) = 1.5`.
    #[test]
    fn flat_two_form_on_r2_constructs_and_evaluates() {
        let raw = vec![0.0, 1.5, -1.5, 0.0];
        let b = TwoForm::new(raw, 2).expect("antisymmetric form should construct");

        assert_eq!(b.dim(), 2);
        assert_eq!(b.apply(&[1.0, 0.0], &[0.0, 1.0]), 1.5);
        // Antisymmetry: B(v, u) = -B(u, v).
        assert_eq!(b.apply(&[0.0, 1.0], &[1.0, 0.0]), -1.5);
        // Diagonal vanishes: B(v, v) = 0.
        assert_eq!(b.apply(&[1.0, 0.0], &[1.0, 0.0]), 0.0);
        assert_eq!(b.apply(&[3.0, 7.0], &[3.0, 7.0]), 0.0);
    }

    /// Positive: tiny asymmetry within tolerance is symmetrized
    /// silently; the resulting form is exactly antisymmetric.
    /// Uses `1e-13` deviation — comfortably below the `1e-10`
    /// production tolerance even after FP-representation noise.
    /// (An earlier draft used `1e-10` and tripped over ulp drift
    /// in `1.500_000_000_1`, which was the right kind of failure
    /// to catch in development.)
    #[test]
    fn small_asymmetry_is_symmetrized_silently() {
        let asymmetry = 1e-13;
        let raw = vec![0.0, 1.5, -(1.5 + asymmetry), 0.0];
        let b = TwoForm::new(raw, 2).expect("within-tolerance asymmetry should be accepted");
        let m = b.matrix();
        // After symmetrization, B[0,1] + B[1,0] should be exactly 0.
        assert!(
            (m[1] + m[2]).abs() < 1e-18,
            "expected exact antisymmetry post-symmetrization, got B[0,1]+B[1,0] = {}",
            m[1] + m[2]
        );
    }

    /// Negative: a gross asymmetry (`[[0, 1], [1, 0]]` — symmetric,
    /// not antisymmetric) is rejected.
    #[test]
    fn rejects_symmetric_matrix() {
        let raw = vec![0.0, 1.0, 1.0, 0.0];
        let result = TwoForm::new(raw, 2);
        match result {
            Err(ClosednessError::NotAntisymmetric { max_dev, .. }) => {
                assert!(
                    (max_dev - 2.0).abs() < 1e-12,
                    "expected max_dev ≈ 2.0 (1 + 1 = 2), got {}",
                    max_dev
                );
            }
            other => panic!("expected NotAntisymmetric, got {:?}", other),
        }
    }

    /// Negative: empty dimension rejected outright.
    #[test]
    fn rejects_empty_dim() {
        assert_eq!(
            TwoForm::new(vec![], 0),
            Err(ClosednessError::EmptyDimension)
        );
    }

    /// Negative: data-length / dim mismatch rejected.
    #[test]
    fn rejects_data_dim_mismatch() {
        let result = TwoForm::new(vec![0.0; 5], 2); // 2*2=4 ≠ 5
        assert!(
            matches!(result, Err(ClosednessError::NotSquare { .. })),
            "expected NotSquare, got {:?}",
            result
        );
    }

    /// Positive: zero 2-form construction is infallible and the
    /// resulting form has zero Frobenius norm.
    #[test]
    fn zero_two_form_has_zero_norm() {
        let z = TwoForm::zero(4);
        assert_eq!(z.dim(), 4);
        assert_eq!(z.frobenius_norm(), 0.0);
        // Apply to nonzero vectors gives zero.
        assert_eq!(z.apply(&[1.0, 2.0, 3.0, 4.0], &[5.0, 6.0, 7.0, 8.0]), 0.0);
    }

    // ── ClosedTwoForm constructor ──

    /// Positive: a constant 2-form on a flat chart is closed by
    /// construction (dB = 0 because every coefficient is constant
    /// and `∂_k constant = 0`).
    #[test]
    fn closed_form_constructor_accepts_constant_form() {
        let b = TwoForm::new(vec![0.0, 1.5, -1.5, 0.0], 2).unwrap();
        let closed = ClosedTwoForm::new_constant(b);
        assert_eq!(closed.dim(), 2);
        assert_eq!(closed.apply(&[1.0, 0.0], &[0.0, 1.0]), 1.5);
    }

    /// Positive: `new_with_discrete_d` accepts a form whose
    /// supplied dB-norm is below tolerance.
    #[test]
    fn closed_form_constructor_accepts_dB_zero() {
        let b = TwoForm::new(vec![0.0, 0.7, -0.7, 0.0], 2).unwrap();
        let result = ClosedTwoForm::new_with_discrete_d(b, |_| 1e-15, None);
        assert!(
            result.is_ok(),
            "form with dB ≈ 0 should construct, got {:?}",
            result.err()
        );
    }

    /// Negative: `new_with_discrete_d` rejects a form whose
    /// supplied dB-norm exceeds tolerance. This is the gate that
    /// catches a learned positional form that drifted from
    /// closedness during training.
    #[test]
    fn closed_form_constructor_rejects_dB_nonzero() {
        let b = TwoForm::new(vec![0.0, 0.7, -0.7, 0.0], 2).unwrap();
        let result = ClosedTwoForm::new_with_discrete_d(b, |_| 1e-3, None);
        match result {
            Err(ClosednessError::NotClosed { norm, tolerance }) => {
                assert!((norm - 1e-3).abs() < 1e-18, "reported norm {}", norm);
                assert!(tolerance < norm, "tolerance {} should be < norm {}", tolerance, norm);
            }
            other => panic!("expected NotClosed, got {:?}", other),
        }
    }

    /// Positive: caller-supplied tolerance is honored (looser
    /// bound for learned forms per catalog §E.5).
    #[test]
    fn closed_form_constructor_honors_caller_tolerance() {
        let b = TwoForm::new(vec![0.0, 0.7, -0.7, 0.0], 2).unwrap();
        // dB-norm of 1e-3, tolerance of 1e-2 — should accept.
        let result = ClosedTwoForm::new_with_discrete_d(b, |_| 1e-3, Some(1e-2));
        assert!(result.is_ok());
    }
}
