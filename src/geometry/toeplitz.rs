//! L7.6 — Berezin-Toeplitz operators on toy manifolds
//! (catalog §2.8, IMPLEMENTATION_PLAN.md L7.6).
//!
//! Per Bordemann-Meinrenken-Schlichenmaier (catalog §2.8 ground
//! truth, tested by `validation_tests_v3.py::test_10`):
//!
//! ```text
//! [T_f, T_g] = -i ℏ T_{f, g} + O(ℏ²)
//! T_f T_g - T_{fg} = O(ℏ)
//! ```
//!
//! where `T_f` is the Toeplitz operator associated with a smooth
//! `f: M → ℝ` at quantization parameter `ℏ = 1/k`.
//!
//! ### Scope
//!
//! L7.6 ships the API surface + the toy-manifold path (CP^n, T^n,
//! S²) where coherent states have closed form. General Kähler
//! Toeplitz operators need the spectral expansion of the Bergman
//! kernel — research-grade. We refuse with
//! `ToeplitzError::UnsupportedManifold` on those.
//!
//! ### Production ℏ safety bound (catalog §E.5 / reply Q6)
//!
//! `ℏ ≥ 4 / embedding_dim` is the safe deployment bound. Below
//! that threshold the truncation error dominates the O(ℏ³)
//! correction; the operator is constructed but the
//! `truncation_dominates_correction` flag fires so the caller
//! knows reliability is compromised.

#![cfg(feature = "kahler")]

use crate::geometry::quantum_cohomology::QuantumCohomology;

/// Toeplitz operator on a toy Kähler manifold at quantization
/// parameter `hbar = 1/k`. The operator's matrix entries live in
/// a coherent-state basis of dimension `k + 1` for CP^1 (the
/// dimension is `(k+n choose n)` for CP^n).
///
/// Matrix is stored as a flat row-major `Vec<f64>` of length
/// `dim²` for L7.6's toy case. A sparse rewrite is a follow-up.
#[derive(Debug, Clone, PartialEq)]
pub struct ToeplitzOperator {
    /// Coherent-state basis dimension `dim H⁰(M, L^k)`.
    pub dim: usize,
    /// Row-major matrix entries `[a_{0,0}, a_{0,1}, ..., a_{n,n}]`.
    pub matrix: Vec<f64>,
    /// Quantization parameter `ℏ = 1/k` at which this operator
    /// was constructed.
    pub hbar: f64,
    /// True iff `hbar < 4 / embedding_dim` — operator was built
    /// outside the safe deployment bound and the caller opted in
    /// via the `allow_below_safe_hbar` path.
    pub truncation_dominates_correction: bool,
}

/// Safety-gate verdict returned by `toeplitz_operator` when the
/// caller does not opt in to below-safe-bound construction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToeplitzSafetyGate {
    /// `hbar >= 4 / embedding_dim`: safe to deploy.
    Safe,
    /// `hbar < 4 / embedding_dim`: would build, but caller hasn't
    /// opted in. Construction refused with `ToeplitzError::HbarBelowSafeBound`.
    BelowSafeBound,
}

/// Errors from `toeplitz_operator`.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ToeplitzError {
    /// Manifold class doesn't support closed-form coherent states
    /// (general Kähler with no Bergman kernel expansion).
    #[error("Toeplitz operator unsupported on this manifold class")]
    UnsupportedManifold,
    /// `hbar < 4 / embedding_dim` and the caller didn't opt in.
    /// `minimum` is the safe bound; `supplied` is what we received.
    #[error(
        "hbar = {supplied} below safe bound {minimum} for embedding_dim = {embedding_dim}; \
         pass allow_below_safe_hbar = true to opt in"
    )]
    HbarBelowSafeBound {
        supplied: f64,
        minimum: f64,
        embedding_dim: usize,
    },
    /// `hbar <= 0` is nonsensical (1 / 0 ⇒ infinite k).
    #[error("hbar must be > 0; got {0}")]
    NonPositiveHbar(f64),
}

/// L7.6.2 — construct the Toeplitz operator for `f: M → ℝ` at
/// quantization `ℏ = 1/k`.
///
/// For L7.6 the `f` argument is the constant function ≡ `f_value`.
/// `T_{const} = const · I` is the trivial case but it lets us
/// exercise the full API surface + the safety gate. Non-constant
/// `f` requires sampling on coherent states (research-grade).
///
/// `embedding_dim` is the host space dimension Marcella consumes
/// the operator in; sets the safe-bound `4 / embedding_dim`.
pub fn toeplitz_operator(
    qh: &QuantumCohomology,
    f_value: f64,
    hbar: f64,
    embedding_dim: usize,
    allow_below_safe_hbar: bool,
) -> Result<ToeplitzOperator, ToeplitzError> {
    if hbar <= 0.0 {
        return Err(ToeplitzError::NonPositiveHbar(hbar));
    }

    let safe_min = 4.0 / embedding_dim.max(1) as f64;
    let below_bound = hbar < safe_min;
    if below_bound && !allow_below_safe_hbar {
        return Err(ToeplitzError::HbarBelowSafeBound {
            supplied: hbar,
            minimum: safe_min,
            embedding_dim,
        });
    }

    // Dimension of the coherent-state basis depends on the manifold.
    let dim = match qh {
        QuantumCohomology::Cpn { n, .. } => {
            // dim H⁰(CP^n, L^k) = binomial(k + n, n). For L7.6 we
            // truncate at k = 1/hbar.
            let k = (1.0 / hbar).round() as usize;
            binomial(k + n, *n)
        }
        QuantumCohomology::Sphere2 => {
            let k = (1.0 / hbar).round() as usize;
            k + 1
        }
        QuantumCohomology::TorusTn { n } => 2_usize.pow(*n as u32),
        QuantumCohomology::NonToy => {
            return Err(ToeplitzError::UnsupportedManifold);
        }
    };

    // T_{const f_value} = f_value · I (the trivial case).
    let mut matrix = vec![0.0_f64; dim * dim];
    for i in 0..dim {
        matrix[i * dim + i] = f_value;
    }

    Ok(ToeplitzOperator {
        dim,
        matrix,
        hbar,
        truncation_dominates_correction: below_bound,
    })
}

/// Binomial coefficient `C(n, k)`. Used for CP^n basis dim.
fn binomial(n: usize, k: usize) -> usize {
    if k > n {
        return 0;
    }
    let k = k.min(n - k);
    let mut c = 1_usize;
    for i in 0..k {
        c = c * (n - i) / (i + 1);
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Positive — Bohr correspondence: at small ℏ, `T_f` ≈ scalar
    /// multiplication by `f` on the coherent-state basis. For the
    /// constant `f = c` we test, the operator is exactly `c · I`
    /// at every ℏ (the BCH correction vanishes on constants —
    /// `{const, g} = 0`).
    #[test]
    fn toeplitz_bohr_correspondence_holds_at_small_hbar() {
        // CP² with embedding_dim 1024 ⇒ safe ℏ ≥ 4/1024 ≈ 0.0039.
        let qh = QuantumCohomology::cpn(2);
        let f_value = 1.5;
        for hbar in [1.0, 0.5, 0.25, 0.125, 0.01] {
            let op = toeplitz_operator(&qh, f_value, hbar, 1024, false)
                .expect("safe ℏ on dim=1024");
            // Constant f ⇒ scalar multiple of identity.
            for i in 0..op.dim {
                for j in 0..op.dim {
                    let entry = op.matrix[i * op.dim + j];
                    let expected = if i == j { f_value } else { 0.0 };
                    assert!(
                        (entry - expected).abs() < 1e-12,
                        "T_const should be c·I; got ({i},{j}) = {entry} at ℏ = {hbar}"
                    );
                }
            }
            assert!(
                !op.truncation_dominates_correction,
                "ℏ = {hbar} ≥ safe bound; flag should be false"
            );
        }
    }

    /// Negative — Toeplitz on NonToy manifold returns
    /// `UnsupportedManifold` per the L7.5 scoping caveat.
    #[test]
    fn toeplitz_on_general_manifold_returns_unimplemented() {
        let qh = QuantumCohomology::NonToy;
        let err = toeplitz_operator(&qh, 1.0, 0.1, 64, true)
            .expect_err("NonToy: must refuse");
        assert!(matches!(err, ToeplitzError::UnsupportedManifold));
    }

    /// Safety gate — ℏ below safe bound without opt-in returns
    /// `HbarBelowSafeBound`. Per IMPLEMENTATION_PLAN L7.6
    /// `ℏ ≥ 4 / embedding_dim`.
    #[test]
    fn hbar_below_safe_bound_refuses_without_opt_in() {
        let qh = QuantumCohomology::cpn(1);
        // embedding_dim = 100 ⇒ safe bound = 0.04. Pass ℏ = 0.01.
        let err = toeplitz_operator(&qh, 1.0, 0.01, 100, false)
            .expect_err("below bound w/o opt-in: must refuse");
        match err {
            ToeplitzError::HbarBelowSafeBound {
                supplied,
                minimum,
                embedding_dim,
            } => {
                assert!((supplied - 0.01).abs() < 1e-12);
                assert!((minimum - 0.04).abs() < 1e-12);
                assert_eq!(embedding_dim, 100);
            }
            _ => panic!("expected HbarBelowSafeBound; got {:?}", err),
        }
    }

    /// Safety gate — opt-in via `allow_below_safe_hbar = true`
    /// builds the operator with `truncation_dominates_correction
    /// = true` so the caller knows reliability is compromised.
    #[test]
    fn hbar_below_safe_bound_with_opt_in_flags_truncation() {
        let qh = QuantumCohomology::cpn(1);
        let op = toeplitz_operator(&qh, 1.0, 0.01, 100, true)
            .expect("opt-in: must build");
        assert!(op.truncation_dominates_correction);
        // Constant-f operator still hits scalar identity.
        for i in 0..op.dim {
            for j in 0..op.dim {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!((op.matrix[i * op.dim + j] - expected).abs() < 1e-12);
            }
        }
    }

    /// Negative — hbar ≤ 0 is nonsensical.
    #[test]
    fn non_positive_hbar_rejected() {
        let qh = QuantumCohomology::cpn(1);
        assert!(matches!(
            toeplitz_operator(&qh, 1.0, 0.0, 100, true),
            Err(ToeplitzError::NonPositiveHbar(_))
        ));
        assert!(matches!(
            toeplitz_operator(&qh, 1.0, -0.5, 100, true),
            Err(ToeplitzError::NonPositiveHbar(_))
        ));
    }
}
