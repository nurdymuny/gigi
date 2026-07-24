//! PK-3 / Optimal transport — the 1D 2-Wasserstein distance `W₂`.
//!
//! For two 1D empirical distributions the optimal transport plan is the
//! **monotone rearrangement** (Hoeffding's lemma): sort both samples and
//! pair them in order. When both have `n` points of equal mass,
//!
//! > `W₂² = (1/n) · Σ_i (a_(i) − b_(i))²`
//!
//! over the order statistics — exact, no linear program. For unequal
//! sample sizes we integrate the quantile functions
//! `W₂² = ∫₀¹ (F_a⁻¹(u) − F_b⁻¹(u))² du` exactly by merging the two
//! step-function breakpoints (no fixed-grid approximation).
//!
//! Closed-form check (the validation ground truth): for two 1D Gaussians
//! `W₂²(N(μ₁,σ₁²), N(μ₂,σ₂²)) = (μ₁−μ₂)² + (σ₁−σ₂)²`
//! (Knott–Smith / Olkin–Pukelsheim). Random pairings cost strictly more
//! than the monotone plan — Hoeffding's bound.
//!
//! Ground truth: `theory/post_kahler_directions/validation_tests.py
//! ::test_3_optimal_transport_wasserstein_gaussians`.

#![cfg(feature = "post_kahler_phase1")]

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum WassersteinError {
    /// One or both distributions are empty.
    EmptyData,
    /// A sample contains a non-finite value (NaN / ±∞).
    InvalidDistribution,
}

impl fmt::Display for WassersteinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WassersteinError::EmptyData => {
                write!(f, "Cannot compute Wasserstein distance on empty distributions.")
            }
            WassersteinError::InvalidDistribution => {
                write!(f, "Invalid distribution provided (contains NaNs or Infs).")
            }
        }
    }
}

impl std::error::Error for WassersteinError {}

/// 1D 2-Wasserstein distance via monotone rearrangement.
pub struct Wasserstein1D;

impl Wasserstein1D {
    fn sorted_checked(v: &[f64]) -> Result<Vec<f64>, WassersteinError> {
        if v.is_empty() {
            return Err(WassersteinError::EmptyData);
        }
        if v.iter().any(|x| !x.is_finite()) {
            return Err(WassersteinError::InvalidDistribution);
        }
        let mut s = v.to_vec();
        s.sort_unstable_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
        Ok(s)
    }

    /// Exact squared 2-Wasserstein distance `W₂²`.
    pub fn compute_sq(a: &[f64], b: &[f64]) -> Result<f64, WassersteinError> {
        let a_sorted = Self::sorted_checked(a)?;
        let b_sorted = Self::sorted_checked(b)?;
        let (n, m) = (a_sorted.len(), b_sorted.len());

        if n == m {
            // Equal mass: pair order statistics directly (exact).
            let sum_sq: f64 = a_sorted
                .iter()
                .zip(b_sorted.iter())
                .map(|(x, y)| (x - y) * (x - y))
                .sum();
            return Ok(sum_sq / n as f64);
        }

        // Unequal sizes: integrate the quantile step functions exactly by
        // sweeping merged breakpoints at u = i/n and j/m. On each sub-
        // interval both quantile functions are constant; accumulate
        // (a_val − b_val)² × width. Total width = 1.
        let (mut i, mut j) = (0usize, 0usize); // #mass consumed from each
        let mut prev = 0.0f64;
        let mut acc = 0.0f64;
        while i < n && j < m {
            // right edge of the current a-step and b-step in [0,1]
            let ra = (i + 1) as f64 / n as f64;
            let rb = (j + 1) as f64 / m as f64;
            let cur = ra.min(rb);
            let width = cur - prev;
            let diff = a_sorted[i] - b_sorted[j];
            acc += diff * diff * width;
            // advance whichever step we've reached the end of (or both)
            if (ra - cur).abs() < 1e-15 {
                i += 1;
            }
            if (rb - cur).abs() < 1e-15 {
                j += 1;
            }
            prev = cur;
        }
        Ok(acc)
    }

    /// Exact 2-Wasserstein distance `W₂ = √(W₂²)`.
    pub fn compute(a: &[f64], b: &[f64]) -> Result<f64, WassersteinError> {
        Self::compute_sq(a, b).map(|sq| sq.sqrt())
    }
}

// ── tests ─────────────────────────────────────────────────────────
// Mirror validation_tests.py::test_3_optimal_transport_wasserstein_gaussians.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_length_shift_is_exact() {
        // a shifted by +1 → W₂ = 1 exactly.
        let a = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let b = vec![2.0, 3.0, 4.0, 5.0, 6.0];
        assert!((Wasserstein1D::compute(&a, &b).unwrap() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn identical_distributions_zero() {
        let a = vec![3.0, 1.0, 2.0];
        assert!(Wasserstein1D::compute(&a, &a).unwrap().abs() < 1e-12);
    }

    #[test]
    fn unequal_length_quantile_integral_partitions_unit() {
        // Two constant distributions offset by 2: every quantile differs
        // by 2 ⇒ W₂² = 4 regardless of the (unequal) sample sizes.
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![2.0, 2.0];
        assert!((Wasserstein1D::compute_sq(&a, &b).unwrap() - 4.0).abs() < 1e-12);
    }

    #[test]
    fn empty_and_nan_rejected() {
        assert_eq!(Wasserstein1D::compute(&[], &[1.0]), Err(WassersteinError::EmptyData));
        assert_eq!(
            Wasserstein1D::compute(&[f64::NAN], &[1.0]),
            Err(WassersteinError::InvalidDistribution)
        );
    }

    /// Closed-form Gaussian ground truth + Hoeffding negative control,
    /// reproducing the python validation with a deterministic sampler.
    #[test]
    fn gaussian_closed_form_and_hoeffding() {
        let (mu1, sigma1) = (0.0_f64, 1.0_f64);
        let (mu2, sigma2) = (3.0_f64, 2.0_f64);
        let n = 20_000usize;

        let mut s: u64 = 0xA11CE_5EED_0003;
        let mut u = || {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (((s >> 11) as f64) + 0.5) / (1u64 << 53) as f64
        };
        let mut normal = || {
            let (u1, u2) = (u(), u());
            (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
        };
        let a: Vec<f64> = (0..n).map(|_| mu1 + sigma1 * normal()).collect();
        let b: Vec<f64> = (0..n).map(|_| mu2 + sigma2 * normal()).collect();

        let w2_sq = Wasserstein1D::compute_sq(&a, &b).unwrap();
        let closed = (mu1 - mu2).powi(2) + (sigma1 - sigma2).powi(2); // = 9 + 1 = 10
        let rel = (w2_sq - closed).abs() / closed;
        assert!(rel < 0.05, "W₂² MC {w2_sq:.4} vs closed {closed:.4} (rel {rel:.3})");

        // Hoeffding: a random pairing costs strictly more than monotone.
        let mut a_sorted = a.clone();
        let mut b_shuf = b.clone();
        a_sorted.sort_by(|x, y| x.partial_cmp(y).unwrap());
        // Fisher–Yates with the same LCG
        for i in (1..b_shuf.len()).rev() {
            let j = (u() * (i as f64 + 1.0)) as usize;
            b_shuf.swap(i, j.min(i));
        }
        let random_cost: f64 =
            a_sorted.iter().zip(&b_shuf).map(|(x, y)| (x - y) * (x - y)).sum::<f64>() / n as f64;
        assert!(random_cost > 1.3 * w2_sq, "random {random_cost:.3} vs monotone {w2_sq:.3}");
    }
}
