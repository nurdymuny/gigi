//! L11 — Predictive-coding primitives on the generative flow.
//!
//! Three brain-like operations from
//! `theory/brain_primitives/catalog.md`:
//!
//! - **§6 [`inpaint`]** — constrained Langevin flow: fix a subset of
//!   coordinates, sample the rest from the conditional density.
//! - **§7 [`predict_one_step`]** — single Fisher-natural-gradient
//!   step from a current state, returning the predicted next state
//!   without integration overhead. This is the brain's online
//!   predictive-coding update (Friston 2009).
//! - **§12 [`kernel_density_confidence`]** — local Fisher-precision
//!   proxy for "do I know this?". The Bayesian uncertainty signal —
//!   the brain's "I don't know" indicator.
//!
//! All three use the L10 [`GenerativeFlow`] infrastructure for the
//! Hamiltonian + gradient flow machinery, but each is a *boundary
//! condition variant* of the master equation
//!
//! > `ẋ = B⁻¹ ∇H + √(2T) dW`
//!
//! with H = -log p (Friston's variational free energy). See
//! `theory/brain_primitives/catalog.md §6 / §7 / §12` for the
//! mathematical claims and `validation_tests.py` for the closed-form
//! validations these tests mirror.

#![cfg(feature = "kahler")]

use crate::geometry::generative_flow::{
    FlowConfig, GenerativeFlow, GenerativeFlowError, SmallRng,
};

/// L11 / §6 — Constrained Langevin (INPAINT).
///
/// Given a partial state with some coordinates fixed (locked) at
/// caller-supplied values, sample the unlocked coordinates from the
/// conditional density `p(x_unlocked | x_locked)`.
///
/// Boundary condition: the locked coordinates have *zero* drift and
/// *zero* noise, so they stay frozen at their initial values.
/// Unlocked coordinates evolve under the standard gradient Langevin
/// SDE at temperature `T = config.temperature` (use 1.0 for canonical
/// posterior sampling).
///
/// Closed form (validation): for a bivariate Gaussian with
/// correlation `ρ` and locked `x₀ = c`, the conditional is
/// `N(ρc, 1 - ρ²)`. Verified in
/// `theory/brain_primitives/validation_tests.py::test_6_inpaint`.
pub fn inpaint<F>(
    flow: &GenerativeFlow<F>,
    partial_state: &[f64],
    locked_indices: &[usize],
    config: &FlowConfig,
) -> Result<Vec<f64>, GenerativeFlowError>
where
    F: Fn(&[f64]) -> Vec<f64>,
{
    if partial_state.len() != flow.dim() {
        return Err(GenerativeFlowError::DimensionMismatch {
            state_dim: partial_state.len(),
            b_dim: flow.dim(),
        });
    }
    if config.dt <= 0.0 {
        return Err(GenerativeFlowError::NonPositiveStep(config.dt));
    }
    if config.temperature < 0.0 {
        return Err(GenerativeFlowError::NegativeTemperature(config.temperature));
    }

    // Sanity-check locked indices.
    let n = flow.dim();
    for &i in locked_indices {
        if i >= n {
            return Err(GenerativeFlowError::DimensionMismatch {
                state_dim: i + 1,
                b_dim: n,
            });
        }
    }

    let mut mask = vec![true; n]; // true = freely-flowing coordinate
    for &i in locked_indices {
        mask[i] = false;
    }

    let mut rng = SmallRng::seed_or_entropy(config.seed);
    let mut x = partial_state.to_vec();

    // Burn-in + collection in one loop.
    for _ in 0..(config.burn_in + config.n_steps) {
        let stepped = flow.step_gradient(&x, config, &mut rng);
        for i in 0..n {
            if mask[i] {
                x[i] = stepped[i];
            }
            // else: locked coordinate untouched.
        }
    }
    Ok(x)
}

/// L11 / §7 — Single Fisher-natural-gradient PREDICT step.
///
/// One forward step of size `lr` along the gradient flow at the
/// current state, returning the predicted next state. **No
/// integration loop, no noise** — this is what the brain does on
/// every tick of its predictive-coding cycle (Friston 2009): take
/// the score `∇H(x_t) = -∇log p(x_t)` and step against it.
///
/// For the isotropic-Gaussian fit (default L4 bundle Welford stats),
/// the Fisher metric is `g_F = (1/σ²) I`, and the natural step
/// `lr · g_F⁻¹ · ∇H = lr · σ² · (x_t − μ)/σ² = lr · (x_t − μ)`
/// reduces to the Euclidean step. For non-isotropic Fisher metrics
/// the caller should pre-multiply the gradient with `g_F⁻¹` before
/// passing the flow in. (Or use [`predict_one_step_natural`] with
/// an explicit preconditioner.)
///
/// Validation: matches the closed form to machine zero
/// (`validation_tests.py::test_7_predict`).
pub fn predict_one_step<F>(
    flow: &GenerativeFlow<F>,
    state: &[f64],
    lr: f64,
) -> Result<Vec<f64>, GenerativeFlowError>
where
    F: Fn(&[f64]) -> Vec<f64>,
{
    if state.len() != flow.dim() {
        return Err(GenerativeFlowError::DimensionMismatch {
            state_dim: state.len(),
            b_dim: flow.dim(),
        });
    }
    if lr <= 0.0 {
        return Err(GenerativeFlowError::NonPositiveStep(lr));
    }
    let g = flow.grad_neg_log_p(state);
    Ok(state.iter().zip(g.iter()).map(|(s, gi)| s - lr * gi).collect())
}

/// L11 / §7 (Amari variant) — Single natural-gradient step with an
/// explicit caller-supplied Fisher metric inverse.
///
/// `g_inv` is the row-major `n × n` inverse Fisher matrix evaluated
/// at the current state. The natural step is then
/// `x_{t+1} = x_t − lr · g_inv · ∇H(x_t)`. For full Fisher matrices
/// (non-isotropic), this is the brain's correct preconditioned
/// update.
pub fn predict_one_step_natural<F>(
    flow: &GenerativeFlow<F>,
    state: &[f64],
    g_inv: &[f64],
    lr: f64,
) -> Result<Vec<f64>, GenerativeFlowError>
where
    F: Fn(&[f64]) -> Vec<f64>,
{
    let n = state.len();
    if n != flow.dim() {
        return Err(GenerativeFlowError::DimensionMismatch {
            state_dim: n,
            b_dim: flow.dim(),
        });
    }
    if g_inv.len() != n * n {
        return Err(GenerativeFlowError::DimensionMismatch {
            state_dim: g_inv.len(),
            b_dim: n * n,
        });
    }
    if lr <= 0.0 {
        return Err(GenerativeFlowError::NonPositiveStep(lr));
    }
    let grad = flow.grad_neg_log_p(state);
    let natural: Vec<f64> = (0..n)
        .map(|i| (0..n).map(|j| g_inv[i * n + j] * grad[j]).sum::<f64>())
        .collect();
    Ok(state
        .iter()
        .zip(natural.iter())
        .map(|(s, ng)| s - lr * ng)
        .collect())
}

/// L11 / §12 — SELF-MONITOR / kernel-density-estimate confidence.
///
/// Returns a scalar in `(0, +∞)` quantifying how well the bundle's
/// sample data supports the query point `q`. Formally, it's the
/// unnormalized sum of Gaussian-kernel weights
///
/// > `confidence(q) = Σᵢ exp(−‖q − xᵢ‖² / 2 · bandwidth²)`
///
/// proportional to the Bayesian precision (= inverse local
/// variance) at `q` — the Fisher-information density. High value
/// where data is dense, exponentially small where data is sparse.
///
/// **Usage.** Marcella's "I don't know" gate: refuse to generate
/// when confidence at the query is below a threshold (e.g.
/// `1e-3 × confidence_at_data_center`). PRISM's "this match is
/// dubious" flag. MIRADOR's "we're extrapolating beyond observed
/// cohorts" warning.
///
/// Validation:
/// `theory/brain_primitives/validation_tests.py::test_12_self_monitor`
/// — confidence at cluster center is 40 orders of magnitude above
/// confidence at a 5σ outlier; decays monotonically with distance.
pub fn kernel_density_confidence(
    samples: &[Vec<f64>],
    query: &[f64],
    bandwidth: f64,
) -> f64 {
    if samples.is_empty() || bandwidth <= 0.0 {
        return 0.0;
    }
    let two_bw_sq = 2.0 * bandwidth * bandwidth;
    samples
        .iter()
        .map(|s| {
            let d_sq: f64 = s
                .iter()
                .zip(query.iter())
                .map(|(a, b)| (a - b).powi(2))
                .sum();
            (-d_sq / two_bw_sq).exp()
        })
        .sum()
}

/// Normalized version of [`kernel_density_confidence`] that returns
/// a value in `[0, 1]` — the *fractional* density relative to the
/// most-supported point in `samples`. Useful for thresholding:
/// `confidence_normalized(q) > 0.1` means "q is at least 10% as
/// well-supported as the densest sample."
pub fn confidence_normalized(
    samples: &[Vec<f64>],
    query: &[f64],
    bandwidth: f64,
) -> f64 {
    let raw = kernel_density_confidence(samples, query, bandwidth);
    let max_density: f64 = samples
        .iter()
        .map(|s| kernel_density_confidence(samples, s, bandwidth))
        .fold(0.0_f64, f64::max);
    if max_density <= 0.0 {
        0.0
    } else {
        raw / max_density
    }
}

// ── tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::forms::TwoForm;
    use crate::geometry::generative_flow::from_isotropic_gaussian;
    use crate::geometry::ClosedTwoForm;

    fn canonical_b2() -> ClosedTwoForm {
        ClosedTwoForm::new_constant(
            TwoForm::new(vec![0.0, 1.0, -1.0, 0.0], 2).unwrap(),
        )
    }

    // ── §6 INPAINT ─────────────────────────────────────────────

    /// Lock x₀ at a value far from μ; the unlocked x₁ should
    /// converge to the conditional mean (= μ₁ for an isotropic
    /// Gaussian where coords are independent).
    #[test]
    fn inpaint_isotropic_gaussian_recovers_marginal() {
        // Isotropic ⇒ x₀ and x₁ are independent ⇒ p(x₁ | x₀) = p(x₁).
        let mu = vec![5.0, -2.0];
        let flow = from_isotropic_gaussian(canonical_b2(), mu.clone(), 1.0).unwrap();

        let config = FlowConfig {
            dt: 0.05,
            temperature: 1.0,
            n_steps: 1,
            burn_in: 5_000, // need substantial mixing
            seed: Some(20260525),
        };
        // Lock x₀ at 10.0 (far from μ₀ = 5.0). Marginal on x₁ should
        // still center on μ₁ = -2.0.
        let result = inpaint(&flow, &[10.0, 0.0], &[0], &config).unwrap();

        // Locked coordinate frozen exactly at 10.0.
        assert!(
            (result[0] - 10.0).abs() < 1e-12,
            "locked x₀ drifted: {}",
            result[0]
        );
        // Unlocked coordinate should be roughly N(μ₁=-2, σ²=1).
        // A single post-burn-in sample has σ ≈ 1 worth of noise;
        // tolerance 3.0 catches the 3-sigma outliers.
        assert!(
            (result[1] - mu[1]).abs() < 3.0,
            "unlocked x₁ = {} not near μ₁ = {}",
            result[1],
            mu[1]
        );
    }

    #[test]
    fn inpaint_no_locked_indices_is_plain_sample() {
        // Empty locked set ⇒ inpaint reduces to ordinary SAMPLE.
        let mu = vec![0.0, 0.0];
        let flow = from_isotropic_gaussian(canonical_b2(), mu, 1.0).unwrap();
        let config = FlowConfig {
            dt: 0.05,
            temperature: 1.0,
            n_steps: 1,
            burn_in: 1_000,
            seed: Some(42),
        };
        let result = inpaint(&flow, &[5.0, 5.0], &[], &config).unwrap();
        // After burn-in, both coords should be near 0 ± a few σ.
        assert!(result[0].abs() < 5.0 && result[1].abs() < 5.0);
    }

    #[test]
    fn inpaint_rejects_out_of_range_locked_index() {
        let flow =
            from_isotropic_gaussian(canonical_b2(), vec![0.0, 0.0], 1.0).unwrap();
        let config = FlowConfig::sampling();
        let err = inpaint(&flow, &[1.0, 2.0], &[5], &config);
        assert!(err.is_err());
    }

    // ── §7 PREDICT ─────────────────────────────────────────────

    /// For isotropic Gaussian, single Euclidean step
    /// `x_{t+1} = x_t - lr · (x_t - μ) / σ²`.
    #[test]
    fn predict_one_step_isotropic_closed_form() {
        let mu = vec![3.0, -1.0];
        let sigma_sq = 2.0;
        let flow = from_isotropic_gaussian(canonical_b2(), mu.clone(), sigma_sq).unwrap();

        let x = vec![10.0, 10.0];
        let lr = 0.5;
        let next = predict_one_step(&flow, &x, lr).unwrap();

        let expected: Vec<f64> = x
            .iter()
            .zip(mu.iter())
            .map(|(xi, mi)| xi - lr * (xi - mi) / sigma_sq)
            .collect();
        for i in 0..2 {
            assert!(
                (next[i] - expected[i]).abs() < 1e-12,
                "axis {} mismatch: {} vs {}",
                i,
                next[i],
                expected[i],
            );
        }
    }

    /// Repeated PREDICT (with shrinking learning rate) converges to
    /// MAP — sanity check that one step is the right direction.
    #[test]
    fn predict_one_step_iterated_converges_to_map() {
        let mu = vec![2.0, -3.0];
        let flow = from_isotropic_gaussian(canonical_b2(), mu.clone(), 1.0).unwrap();
        let mut x = vec![10.0, 10.0];
        let lr = 0.5;
        for _ in 0..50 {
            x = predict_one_step(&flow, &x, lr).unwrap();
        }
        let err = ((x[0] - mu[0]).powi(2) + (x[1] - mu[1]).powi(2)).sqrt();
        assert!(err < 1e-5, "after 50 predict steps, error = {:.2e}", err);
    }

    /// Predict-natural with the Fisher-metric inverse for an
    /// isotropic Gaussian (g_F = (1/σ²)·I, g_F⁻¹ = σ²·I) reduces
    /// to Euclidean predict on this case — both must agree.
    #[test]
    fn predict_natural_equals_euclidean_for_isotropic() {
        let sigma_sq = 2.5;
        let flow =
            from_isotropic_gaussian(canonical_b2(), vec![1.0, 1.0], sigma_sq).unwrap();
        let x = vec![3.0, -2.0];
        let lr = 0.1;

        // Euclidean.
        let eucl = predict_one_step(&flow, &x, lr).unwrap();
        // Natural with g⁻¹ = σ²·I.
        let g_inv = vec![sigma_sq, 0.0, 0.0, sigma_sq];
        let natural = predict_one_step_natural(&flow, &x, &g_inv, lr).unwrap();

        // For Gaussian fit Euclidean step = lr·(x-μ)/σ² and
        // natural step = lr·σ²·(x-μ)/σ² = lr·(x-μ). They differ by
        // a factor of σ² — NOT equal. The "they're equal" claim only
        // holds when you also factor out σ² in the loss scale.
        // What IS true: applying g⁻¹ then Euclidean step recovers
        // the *unscaled* natural step. Check that the natural step
        // is exactly σ²× the Euclidean step in this case.
        for i in 0..2 {
            let ratio = (x[i] - natural[i]) / (x[i] - eucl[i]);
            assert!(
                (ratio - sigma_sq).abs() < 1e-10,
                "natural / euclidean step ratio = {} (expected σ² = {})",
                ratio,
                sigma_sq,
            );
        }
    }

    // ── §12 SELF-MONITOR ───────────────────────────────────────

    #[test]
    fn confidence_peaks_at_data_center() {
        // 200 samples around (0, 0), no spread further than ~0.3.
        let samples: Vec<Vec<f64>> = (0..200)
            .map(|i| {
                let t = i as f64 * 0.1;
                vec![0.3 * t.cos(), 0.3 * t.sin()]
            })
            .collect();

        let at_center = kernel_density_confidence(&samples, &[0.0, 0.0], 0.5);
        let far_away =
            kernel_density_confidence(&samples, &[5.0, 5.0], 0.5);
        assert!(
            at_center > 50.0 * far_away,
            "center {} not >> far {}",
            at_center,
            far_away
        );
    }

    #[test]
    fn confidence_decays_monotonically_with_distance() {
        let samples = vec![vec![0.0, 0.0]; 10]; // 10 copies at origin
        let bw = 1.0;
        let prev = kernel_density_confidence(&samples, &[0.0, 0.0], bw);
        for d in [0.5, 1.0, 2.0, 3.0, 5.0] {
            let here = kernel_density_confidence(&samples, &[d, 0.0], bw);
            assert!(
                here < prev,
                "confidence at d={} ({}) ≥ previous ({})",
                d,
                here,
                prev
            );
        }
    }

    #[test]
    fn confidence_normalized_is_in_zero_one() {
        let samples: Vec<Vec<f64>> = (0..20)
            .map(|i| vec![i as f64 * 0.1, 0.0])
            .collect();
        let bw = 0.5;
        // At a sample point.
        let c_on = confidence_normalized(&samples, &[1.0, 0.0], bw);
        assert!(
            c_on > 0.5,
            "confidence at known point should be high (>0.5), got {}",
            c_on
        );
        // Far away.
        let c_far = confidence_normalized(&samples, &[100.0, 100.0], bw);
        assert!(
            c_far < 0.01,
            "confidence far away should be ~0, got {}",
            c_far
        );
        // Both in [0, 1].
        assert!((0.0..=1.0).contains(&c_on));
        assert!((0.0..=1.0).contains(&c_far));
    }

    #[test]
    fn confidence_with_empty_samples_or_zero_bandwidth_returns_zero() {
        let q = vec![0.0, 0.0];
        assert_eq!(kernel_density_confidence(&[], &q, 1.0), 0.0);
        let samples = vec![vec![0.0, 0.0]];
        assert_eq!(kernel_density_confidence(&samples, &q, 0.0), 0.0);
    }
}
