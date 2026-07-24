//! PK-2 / Information geometry — the Fisher metric of the 1D Gaussian family.
//!
//! The 2-parameter family `N(μ, σ²)` carries a natural Riemannian
//! metric, the Fisher information metric. In the `(μ, σ)` chart it is
//! the diagonal tensor
//!
//! > `g = diag(1/σ², 2/σ²)`,  `g_μσ = 0`.
//!
//! Derivation (analytic score functions of `log p(x | μ, σ)`):
//!
//! > `∂_μ log p = (x − μ)/σ²`  ⇒ `E[(∂_μ log p)²] = 1/σ²`
//! > `∂_σ log p = ((x − μ)² − σ²)/σ³`  ⇒ `E[(∂_σ log p)²] = 2/σ²`
//! > `E[∂_μ log p · ∂_σ log p] = 0`  (Gaussian symmetry).
//!
//! GIGI already tracks per-field mean and variance via L4 Welford
//! streaming statistics, so for any field typed as univariate Gaussian
//! the Fisher metric is available **for free** — no extra pass. This
//! module is the pure metric; the endpoint layer reads `σ²` from the
//! bundle's Welford stats and calls [`FisherGaussian::from_variance`].
//!
//! Ground truth: `theory/post_kahler_directions/validation_tests.py
//! ::test_2_information_geometry_fisher_on_gaussians`.

#![cfg(feature = "post_kahler_phase1")]

use std::fmt;

/// Error for degenerate Gaussian parameters.
#[derive(Debug, Clone, PartialEq)]
pub enum FisherError {
    /// `σ ≤ 0` (or non-finite): the Gaussian, and its metric, are undefined.
    NonPositiveScale,
}

impl fmt::Display for FisherError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FisherError::NonPositiveScale => {
                write!(f, "Fisher metric requires σ² > 0 (degenerate Gaussian).")
            }
        }
    }
}

impl std::error::Error for FisherError {}

/// The Fisher information metric of `N(μ, σ²)` in the `(μ, σ)` chart.
///
/// Diagonal by Gaussian symmetry: `g_μσ = 0`. The value is independent
/// of `μ` (translation invariance) and scales as `1/σ²`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FisherGaussian {
    /// `g_μμ = 1/σ²`.
    pub g_mu_mu: f64,
    /// `g_σσ = 2/σ²`.
    pub g_sigma_sigma: f64,
    /// `g_μσ = 0` (carried explicitly so callers see the full tensor).
    pub g_mu_sigma: f64,
}

impl FisherGaussian {
    /// Build the metric from the standard deviation `σ`.
    pub fn from_sigma(sigma: f64) -> Result<Self, FisherError> {
        if !(sigma > 0.0) || !sigma.is_finite() {
            return Err(FisherError::NonPositiveScale);
        }
        Self::from_variance(sigma * sigma)
    }

    /// Build the metric from the variance `σ²` — the quantity L4 Welford
    /// stats already hold, so this is the zero-cost bundle entry point.
    pub fn from_variance(variance: f64) -> Result<Self, FisherError> {
        if !(variance > 0.0) || !variance.is_finite() {
            return Err(FisherError::NonPositiveScale);
        }
        Ok(Self {
            g_mu_mu: 1.0 / variance,
            g_sigma_sigma: 2.0 / variance,
            g_mu_sigma: 0.0,
        })
    }

    /// The metric determinant `det g = 2/σ⁴` — the squared volume
    /// density (Jeffreys prior `∝ √det g = √2/σ²`).
    pub fn determinant(&self) -> f64 {
        self.g_mu_mu * self.g_sigma_sigma - self.g_mu_sigma * self.g_mu_sigma
    }

    /// The 2×2 metric as a row-major matrix.
    pub fn matrix(&self) -> [[f64; 2]; 2] {
        [
            [self.g_mu_mu, self.g_mu_sigma],
            [self.g_mu_sigma, self.g_sigma_sigma],
        ]
    }
}

// ── tests ─────────────────────────────────────────────────────────
// Mirror validation_tests.py::test_2_information_geometry_fisher_on_gaussians:
// the closed form directly, plus a Monte-Carlo score cross-check that
// reproduces the python assertions (rel err < 5% at N = 2·10⁵).

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_form_diag_metric() {
        let sigma = 2.3_f64;
        let g = FisherGaussian::from_sigma(sigma).unwrap();
        assert!((g.g_mu_mu - 1.0 / (sigma * sigma)).abs() < 1e-15);
        assert!((g.g_sigma_sigma - 2.0 / (sigma * sigma)).abs() < 1e-15);
        assert_eq!(g.g_mu_sigma, 0.0);
        // det g = 2/σ⁴
        assert!((g.determinant() - 2.0 / sigma.powi(4)).abs() < 1e-15);
    }

    #[test]
    fn from_variance_matches_from_sigma() {
        let g1 = FisherGaussian::from_sigma(1.7).unwrap();
        let g2 = FisherGaussian::from_variance(1.7 * 1.7).unwrap();
        assert_eq!(g1, g2);
    }

    #[test]
    fn degenerate_scale_rejected() {
        assert_eq!(FisherGaussian::from_sigma(0.0), Err(FisherError::NonPositiveScale));
        assert_eq!(FisherGaussian::from_sigma(-1.0), Err(FisherError::NonPositiveScale));
        assert_eq!(FisherGaussian::from_variance(0.0), Err(FisherError::NonPositiveScale));
        assert!(FisherGaussian::from_sigma(f64::NAN).is_err());
    }

    /// Monte-Carlo score identity — the same estimator the python
    /// ground truth uses. Deterministic LCG + Box–Muller so the test is
    /// reproducible; N and tolerance mirror validation_tests.py.
    #[test]
    fn monte_carlo_scores_match_closed_form() {
        let (mu, sigma) = (1.7_f64, 2.3_f64);
        let n = 200_000usize;

        // deterministic uniforms → standard normals via Box–Muller
        let mut s: u64 = 0x5EED_1234_ABCD_0001;
        let mut uniform = || {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            // top 53 bits → (0,1)
            (((s >> 11) as f64) + 0.5) / (1u64 << 53) as f64
        };

        let (mut sum_mm, mut sum_ss, mut sum_ms) = (0.0f64, 0.0f64, 0.0f64);
        let s2 = sigma * sigma;
        let s3 = s2 * sigma;
        let mut i = 0;
        while i < n {
            let (u1, u2) = (uniform(), uniform());
            let r = (-2.0 * u1.ln()).sqrt();
            let z0 = r * (std::f64::consts::TAU * u2).cos();
            let z1 = r * (std::f64::consts::TAU * u2).sin();
            for z in [z0, z1] {
                let x = mu + sigma * z;
                let score_mu = (x - mu) / s2;
                let score_sigma = ((x - mu) * (x - mu) - s2) / s3;
                sum_mm += score_mu * score_mu;
                sum_ss += score_sigma * score_sigma;
                sum_ms += score_mu * score_sigma;
            }
            i += 2;
        }
        let (g_mm, g_ss, g_ms) = (sum_mm / n as f64, sum_ss / n as f64, sum_ms / n as f64);
        let g = FisherGaussian::from_sigma(sigma).unwrap();

        let err_mm = (g_mm - g.g_mu_mu).abs() / g.g_mu_mu;
        let err_ss = (g_ss - g.g_sigma_sigma).abs() / g.g_sigma_sigma;
        assert!(err_mm < 0.05, "g_μμ MC rel err {err_mm:.3} (empirical {g_mm}, closed {})", g.g_mu_mu);
        assert!(err_ss < 0.05, "g_σσ MC rel err {err_ss:.3} (empirical {g_ss}, closed {})", g.g_sigma_sigma);
        assert!(g_ms.abs() < 0.01, "g_μσ should vanish, got {g_ms}");
    }
}
