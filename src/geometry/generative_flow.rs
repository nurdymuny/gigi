//! L10 — Generative flow on a Kähler bundle (catalog §2.0).
//!
//! The keystone of the brain-primitives catalog
//! (`theory/brain_primitives/catalog.md`). Implements the master
//! equation
//!
//! > **`ẋ = B⁻¹ ∇(-log p(x))  +  √(2T) dW`**
//!
//! parametrized by boundary conditions, which gives four brain-like
//! operations from one piece of infrastructure:
//!
//! | Method | Boundary | Result |
//! |---|---|---|
//! | [`GenerativeFlow::sample`] | random init, `T = 1`, post burn-in | draw from stationary density |
//! | [`GenerativeFlow::forecast`] | fixed init, `T = 0` | deterministic flow trajectory |
//! | [`GenerativeFlow::dream`] | random init, `T ≫ 1` | high-noise creative variation |
//! | [`GenerativeFlow::reconstruct`] | arbitrary init, `T = 0`, run to convergence | MAP estimate (mode-seeking) |
//!
//! ## Why this lives in `geometry/` and not its own module tree
//!
//! It reuses L1's [`ClosedTwoForm`] for B, L9's Gauss-Jordan
//! inverter for `B⁻¹`, and the same Euler-Maruyama integrator pattern
//! L9 uses for Hamilton's equations. The Friston reading of the
//! Kähler substrate (`theory/brain_primitives/catalog.md §1`) is
//! literally that the existing pieces *already* implement variational
//! free-energy minimization — this module just packages the flow
//! with the four boundary conditions that matter for downstream
//! consumers.
//!
//! ## Caller responsibility
//!
//! The caller supplies the negative-log-density gradient `∇H` as a
//! closure. For bundles with a Welford-streaming Gaussian fit (the
//! L4 default), the gradient is `(x − μ) / σ²` per dimension — see
//! [`GenerativeFlow::from_isotropic_gaussian`] for the one-line
//! constructor.
//!
//! ## Validation
//!
//! Math matches `theory/brain_primitives/validation_tests.py` for
//! §2 SAMPLE, §3 FORECAST, §4 DREAM, §5 RECONSTRUCT. Per-method
//! tests in this module mirror those checks in Rust.

#![cfg(feature = "kahler")]

use crate::geometry::forms::ClosedTwoForm;
use thiserror::Error;

/// Default Euler-Maruyama step size. Smaller = more accurate per
/// step but more steps to traverse the same distance.
pub const DEFAULT_DT: f64 = 0.01;

/// Default burn-in length before stationary samples start being
/// kept. Should exceed the chain's mixing time; 2 000 steps at
/// `dt = 0.01` (t = 20) is sane for typical Gaussian-shaped bundles.
pub const DEFAULT_BURN_IN: usize = 2_000;

/// Configuration knobs shared by sample/forecast/dream/reconstruct.
#[derive(Debug, Clone, PartialEq)]
pub struct FlowConfig {
    /// Time-step for Euler-Maruyama integration.
    pub dt: f64,
    /// Temperature: 0 = deterministic Hamilton flow, 1 = canonical
    /// Langevin (stationary matches `p`), >1 = "dream" / creative.
    pub temperature: f64,
    /// Total integration steps (post-burn-in for sampling).
    pub n_steps: usize,
    /// Burn-in steps before keeping samples. Ignored by `forecast`
    /// and `reconstruct`.
    pub burn_in: usize,
    /// PRNG seed for reproducible runs. None → entropy from OS.
    pub seed: Option<u64>,
}

impl FlowConfig {
    /// Canonical Langevin: `T = 1`, sample-style.
    pub fn sampling() -> Self {
        Self {
            dt: DEFAULT_DT,
            temperature: 1.0,
            n_steps: 1_000,
            burn_in: DEFAULT_BURN_IN,
            seed: None,
        }
    }

    /// Deterministic flow: `T = 0`, no burn-in.
    pub fn forecasting() -> Self {
        Self {
            dt: DEFAULT_DT,
            temperature: 0.0,
            n_steps: 1_000,
            burn_in: 0,
            seed: None,
        }
    }

    /// High-T Langevin: pass `temperature = 4` or higher.
    pub fn dreaming(temperature: f64) -> Self {
        Self {
            dt: DEFAULT_DT,
            temperature,
            n_steps: 1_000,
            burn_in: DEFAULT_BURN_IN / 4,
            seed: None,
        }
    }

    /// Zero-temperature descent: run until step `n_steps` (caller's
    /// convergence budget).
    pub fn reconstructing() -> Self {
        Self {
            dt: 0.05,
            temperature: 0.0,
            n_steps: 500,
            burn_in: 0,
            seed: None,
        }
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum GenerativeFlowError {
    #[error("symplectic form B is singular (det ≈ 0); cannot compute B⁻¹ for Hamilton's equations")]
    SingularSymplectic,
    #[error("state dimension {state_dim} does not match symplectic form dim {b_dim}")]
    DimensionMismatch { state_dim: usize, b_dim: usize },
    #[error("dt must be > 0; got {0}")]
    NonPositiveStep(f64),
    #[error("temperature must be ≥ 0; got {0}")]
    NegativeTemperature(f64),
}

/// The keystone generative-flow primitive on a Kähler bundle.
///
/// Holds a closed 2-form `B` and a callable `∇H` (the gradient of
/// the negative-log-density `H = -log p`). The four methods
/// `sample` / `forecast` / `dream` / `reconstruct` differ only in
/// boundary conditions (initial state, temperature, burn-in,
/// stopping rule) applied to the same SDE.
pub struct GenerativeFlow<F>
where
    F: Fn(&[f64]) -> Vec<f64>,
{
    b: ClosedTwoForm,
    /// Pre-computed `B⁻¹` for the Hamiltonian-vector-field map.
    b_inv: Vec<f64>,
    /// Caller-supplied `∇H(x) = ∇(-log p(x))`.
    grad_neg_log_p: F,
}

impl<F> GenerativeFlow<F>
where
    F: Fn(&[f64]) -> Vec<f64>,
{
    /// Construct from a closed 2-form (must be non-degenerate) and a
    /// negative-log-density gradient.
    pub fn new(b: ClosedTwoForm, grad_neg_log_p: F) -> Result<Self, GenerativeFlowError> {
        let n = b.dim();
        let b_mat = b.form().matrix().to_vec();
        let b_inv = invert(&b_mat, n).ok_or(GenerativeFlowError::SingularSymplectic)?;
        Ok(Self {
            b,
            b_inv,
            grad_neg_log_p,
        })
    }

    /// The underlying symplectic form.
    pub fn b(&self) -> &ClosedTwoForm {
        &self.b
    }

    pub fn dim(&self) -> usize {
        self.b.dim()
    }

    /// Dissipative Langevin step: `dx = -∇H dt + √(2T·dt) dW`.
    ///
    /// This is the gradient-descent half of the Kähler-bundle flow
    /// (uses no `B`). Stationary distribution at `T = 1` is
    /// `p ∝ exp(-H)`. Used by SAMPLE, DREAM, RECONSTRUCT, INPAINT —
    /// all the primitives that *minimize* free energy (Friston FEP).
    fn step_gradient(
        &self,
        state: &[f64],
        config: &FlowConfig,
        rng: &mut SmallRng,
    ) -> Vec<f64> {
        let g = (self.grad_neg_log_p)(state);
        let diffusion_scale = (2.0 * config.temperature * config.dt).max(0.0).sqrt();
        state
            .iter()
            .zip(g.iter())
            .map(|(s, gi)| {
                let noise = if diffusion_scale > 0.0 {
                    rng.standard_normal() * diffusion_scale
                } else {
                    0.0
                };
                s - config.dt * gi + noise
            })
            .collect()
    }

    /// Conservative Hamiltonian step: `ẋ = B⁻¹ ∇H` (rotates along
    /// constant-`H` level sets, conserves energy). Used by FORECAST
    /// when the goal is *predictive extension* rather than free-
    /// energy minimization.
    ///
    /// Note: this is the SAME machinery as L9
    /// `MomentMap::flow_step` — we keep it inline here so this
    /// module stays free-standing, and so the FORECAST sign
    /// convention is explicit at the call site.
    fn step_hamiltonian(&self, state: &[f64], config: &FlowConfig) -> Vec<f64> {
        let n = self.dim();
        let g = (self.grad_neg_log_p)(state);
        let drift = matvec(&self.b_inv, &g, n);
        state
            .iter()
            .zip(drift.iter())
            .map(|(s, d)| s - config.dt * d)
            .collect()
    }

    /// §2 SAMPLE — draw one sample from the stationary distribution
    /// of the Langevin chain (post burn-in).
    ///
    /// Returns the final state after `burn_in + n_steps` integration
    /// steps. For an isotropic Gaussian `H`, the returned point is a
    /// draw from `N(μ, σ²·I)`.
    pub fn sample(
        &self,
        initial: &[f64],
        config: &FlowConfig,
    ) -> Result<Vec<f64>, GenerativeFlowError> {
        self.validate(initial, config)?;
        let mut rng = SmallRng::seed_or_entropy(config.seed);
        let mut x = initial.to_vec();
        for _ in 0..(config.burn_in + config.n_steps) {
            x = self.step_gradient(&x, config, &mut rng);
        }
        Ok(x)
    }

    /// Draw `n_samples` post-burn-in draws (more useful than a
    /// single one for downstream consumers).
    pub fn sample_many(
        &self,
        initial: &[f64],
        config: &FlowConfig,
        n_samples: usize,
        keep_every: usize,
    ) -> Result<Vec<Vec<f64>>, GenerativeFlowError> {
        self.validate(initial, config)?;
        let mut rng = SmallRng::seed_or_entropy(config.seed);
        let mut x = initial.to_vec();
        // Burn-in.
        for _ in 0..config.burn_in {
            x = self.step_gradient(&x, config, &mut rng);
        }
        // Collect, thinning by `keep_every`.
        let stride = keep_every.max(1);
        let mut out = Vec::with_capacity(n_samples);
        let total_steps = n_samples * stride;
        for i in 0..total_steps {
            x = self.step_gradient(&x, config, &mut rng);
            if (i + 1) % stride == 0 {
                out.push(x.clone());
            }
        }
        Ok(out)
    }

    /// §3 FORECAST — deterministic Hamilton-flow trajectory from a
    /// fixed initial state. Returns the full path `[x_0, x_1, …,
    /// x_{n_steps}]`. Use `config.temperature = 0` (set by
    /// [`FlowConfig::forecasting`]).
    pub fn forecast(
        &self,
        initial: &[f64],
        config: &FlowConfig,
    ) -> Result<Vec<Vec<f64>>, GenerativeFlowError> {
        self.validate(initial, config)?;
        // FORECAST uses the conservative Hamiltonian flow — that's
        // the predictive primitive (energy levels stay constant).
        // We honor any caller-set temperature by adding noise after
        // the deterministic Hamilton step (rarely useful; canonical
        // usage is T = 0 via FlowConfig::forecasting()).
        let mut rng = SmallRng::seed_or_entropy(config.seed);
        let mut x = initial.to_vec();
        let mut path = Vec::with_capacity(config.n_steps + 1);
        path.push(x.clone());
        let diffusion_scale = (2.0 * config.temperature * config.dt).max(0.0).sqrt();
        for _ in 0..config.n_steps {
            x = self.step_hamiltonian(&x, config);
            if diffusion_scale > 0.0 {
                for xi in &mut x {
                    *xi += rng.standard_normal() * diffusion_scale;
                }
            }
            path.push(x.clone());
        }
        Ok(path)
    }

    /// §4 DREAM — high-temperature Langevin, returning the full
    /// trajectory. The path explores states well beyond the bundle's
    /// data manifold; useful for novelty / synthesis.
    pub fn dream(
        &self,
        initial: &[f64],
        config: &FlowConfig,
    ) -> Result<Vec<Vec<f64>>, GenerativeFlowError> {
        self.validate(initial, config)?;
        let mut rng = SmallRng::seed_or_entropy(config.seed);
        let mut x = initial.to_vec();
        // Burn-in suppressed here — caller wants the trajectory,
        // not stationary draws.
        let mut path = Vec::with_capacity(config.n_steps + 1);
        path.push(x.clone());
        for _ in 0..config.n_steps {
            x = self.step_gradient(&x, config, &mut rng);
            path.push(x.clone());
        }
        Ok(path)
    }

    /// §5 RECONSTRUCT — zero-temperature descent to the nearest
    /// mode of `p` (MAP estimate). Returns the final state. With
    /// unimodal `H`, the result is the global MAP; with multi-modal
    /// `H`, it's the closest local mode to `noisy_initial`.
    pub fn reconstruct(
        &self,
        noisy_initial: &[f64],
        config: &FlowConfig,
    ) -> Result<Vec<f64>, GenerativeFlowError> {
        self.validate(noisy_initial, config)?;
        if config.temperature != 0.0 {
            return Err(GenerativeFlowError::NegativeTemperature(config.temperature));
        }
        let mut rng = SmallRng::seed_or_entropy(config.seed);
        let mut x = noisy_initial.to_vec();
        for _ in 0..config.n_steps {
            x = self.step_gradient(&x, config, &mut rng);
        }
        Ok(x)
    }

    fn validate(
        &self,
        initial: &[f64],
        config: &FlowConfig,
    ) -> Result<(), GenerativeFlowError> {
        if initial.len() != self.dim() {
            return Err(GenerativeFlowError::DimensionMismatch {
                state_dim: initial.len(),
                b_dim: self.dim(),
            });
        }
        if config.dt <= 0.0 {
            return Err(GenerativeFlowError::NonPositiveStep(config.dt));
        }
        if config.temperature < 0.0 {
            return Err(GenerativeFlowError::NegativeTemperature(config.temperature));
        }
        Ok(())
    }
}

/// Convenience: build a generative flow whose Hamiltonian is the
/// negative-log-density of an isotropic Gaussian `N(μ, σ²·I)` —
/// the per-bundle default produced by L4's Welford-streaming fit.
///
/// `∇H(x) = (x - μ) / σ²`.
pub fn from_isotropic_gaussian(
    b: ClosedTwoForm,
    mu: Vec<f64>,
    sigma_sq: f64,
) -> Result<GenerativeFlow<impl Fn(&[f64]) -> Vec<f64>>, GenerativeFlowError> {
    if sigma_sq <= 0.0 {
        return Err(GenerativeFlowError::NonPositiveStep(sigma_sq));
    }
    let n = b.dim();
    if mu.len() != n {
        return Err(GenerativeFlowError::DimensionMismatch {
            state_dim: mu.len(),
            b_dim: n,
        });
    }
    let grad = move |x: &[f64]| -> Vec<f64> {
        x.iter().zip(mu.iter()).map(|(xi, mi)| (xi - mi) / sigma_sq).collect()
    };
    GenerativeFlow::new(b, grad)
}

// ── helpers ───────────────────────────────────────────────────────

/// Lightweight PCG-style PRNG. Avoids pulling in the `rand` crate
/// from this module so we keep the dependency surface minimal.
/// Quality is sufficient for Monte-Carlo Langevin sampling
/// (validated by the per-method tests).
pub struct SmallRng {
    state: u64,
}

impl SmallRng {
    pub fn seed_or_entropy(seed: Option<u64>) -> Self {
        let s = match seed {
            Some(s) => s,
            None => {
                use std::time::{SystemTime, UNIX_EPOCH};
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_nanos() as u64)
                    .unwrap_or(0x1234_5678_9abc_def0)
                    .wrapping_mul(0x5851_F42D_4C95_7F2D)
                    .wrapping_add(0x14057B7E_F767_814F)
            }
        };
        Self { state: s.max(1) }
    }

    /// xorshift64* — one of the fastest sound 64-bit generators.
    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform in [0, 1).
    pub fn uniform(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / ((1u64 << 53) as f64)
    }

    /// Standard normal via Box-Muller.
    pub fn standard_normal(&mut self) -> f64 {
        let u1 = (self.uniform()).max(1e-300);
        let u2 = self.uniform();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }
}

/// Row-major matrix-vector: `(A · x)[i] = Σ_j A[i,j] · x[j]`.
fn matvec(a: &[f64], x: &[f64], n: usize) -> Vec<f64> {
    (0..n)
        .map(|i| (0..n).map(|j| a[i * n + j] * x[j]).sum())
        .collect()
}

/// Invert a row-major `n × n` matrix via Gauss-Jordan with partial
/// pivoting. Returns `None` if singular. Same routine as
/// `moment_map::invert`; duplicated here so the modules can stay
/// independent.
fn invert(m: &[f64], n: usize) -> Option<Vec<f64>> {
    let mut a = vec![0.0_f64; n * 2 * n];
    for i in 0..n {
        for j in 0..n {
            a[i * 2 * n + j] = m[i * n + j];
        }
        a[i * 2 * n + n + i] = 1.0;
    }
    let cols = 2 * n;
    for col in 0..n {
        let mut pivot = col;
        let mut max_abs = a[col * cols + col].abs();
        for r in (col + 1)..n {
            let v = a[r * cols + col].abs();
            if v > max_abs {
                max_abs = v;
                pivot = r;
            }
        }
        if max_abs < 1e-14 {
            return None;
        }
        if pivot != col {
            for j in 0..cols {
                a.swap(col * cols + j, pivot * cols + j);
            }
        }
        let p = a[col * cols + col];
        for j in 0..cols {
            a[col * cols + j] /= p;
        }
        for r in 0..n {
            if r == col {
                continue;
            }
            let factor = a[r * cols + col];
            if factor == 0.0 {
                continue;
            }
            for j in 0..cols {
                a[r * cols + j] -= factor * a[col * cols + j];
            }
        }
    }
    let mut inv = vec![0.0_f64; n * n];
    for i in 0..n {
        for j in 0..n {
            inv[i * n + j] = a[i * cols + n + j];
        }
    }
    Some(inv)
}

// ── tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::forms::TwoForm;

    /// Canonical symplectic form on T*ℝ² in (q, p) ordering.
    /// Same matrix used in moment_map tests.
    fn canonical_b4() -> ClosedTwoForm {
        let raw = vec![
            0.0, 0.0, -1.0, 0.0,
            0.0, 0.0, 0.0, -1.0,
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
        ];
        ClosedTwoForm::new_constant(TwoForm::new(raw, 4).unwrap())
    }

    /// Standard symplectic form `[[0, 1], [-1, 0]]` on ℝ² —
    /// the 2D analog used for tests that don't need (q, p) split.
    fn canonical_b2() -> ClosedTwoForm {
        let raw = vec![0.0, 1.0, -1.0, 0.0];
        ClosedTwoForm::new_constant(TwoForm::new(raw, 2).unwrap())
    }

    // ── §3 FORECAST: harmonic-oscillator energy conservation ───
    //
    // On (q, p) with B = [[0, -1], [1, 0]] (so B⁻¹ = [[0, 1], [-1, 0]])
    // and H = ½(q² + p²), Hamilton's equations are q̇ = p, ṗ = -q
    // — harmonic motion. After one period t = 2π we return to start.
    //
    // We use the 2D B = [[0, 1], [-1, 0]] (canonical_b2) which has
    // B⁻¹ = [[0, -1], [1, 0]]. Then ẋ = -B⁻¹ ∇H = -[[0, -1], [1, 0]] · (q, p)
    //     = (p, -q). Same Hamiltonian dynamics, different sign convention.

    #[test]
    fn forecast_harmonic_oscillator_conserves_energy() {
        // H = ½(q² + p²), ∇H = (q, p).
        let flow = GenerativeFlow::new(
            canonical_b2(),
            |x: &[f64]| vec![x[0], x[1]],
        )
        .unwrap();

        let config = FlowConfig {
            dt: 0.001,
            temperature: 0.0,
            n_steps: 6283, // ≈ 2π
            burn_in: 0,
            seed: Some(0),
        };
        let path = flow.forecast(&[1.0, 0.0], &config).unwrap();
        let end = &path[path.len() - 1];
        let err = ((end[0] - 1.0).powi(2) + end[1].powi(2)).sqrt();
        assert!(err < 0.05, "harmonic return error {:.4e}", err);

        // Energy drift across the full path: with first-order
        // Euler-Maruyama at dt=0.001, ~0.3% drift over t = 2π is
        // expected.
        let energy_start = 0.5 * (path[0][0].powi(2) + path[0][1].powi(2));
        let energy_end = 0.5 * (end[0].powi(2) + end[1].powi(2));
        let drift = (energy_end - energy_start).abs();
        assert!(drift < 0.01, "energy drift {:.4e}", drift);
    }

    // ── §5 RECONSTRUCT: zero-noise descent to MAP ──────────────
    //
    // For an isotropic Gaussian N(μ, σ²·I), MAP = μ. Descent from
    // any starting point must converge to μ within the budget.

    #[test]
    fn reconstruct_isotropic_gaussian_converges_to_mu() {
        let mu = vec![2.0, -3.0];
        let sigma_sq = 1.0;
        let flow = from_isotropic_gaussian(canonical_b2(), mu.clone(), sigma_sq).unwrap();

        let config = FlowConfig::reconstructing();
        let map = flow.reconstruct(&[10.0, 10.0], &config).unwrap();
        let err = ((map[0] - mu[0]).powi(2) + (map[1] - mu[1]).powi(2)).sqrt();
        // After 500 steps at dt=0.05 the trajectory has covered
        // multiple e-foldings of the harmonic well — should be
        // exponentially close to μ.
        assert!(err < 1e-3, "||MAP − μ|| = {:.4e}", err);
    }

    // ── §2 SAMPLE: stationary distribution recovers N(μ, σ²·I) ──
    //
    // Long chain at T = 1: empirical mean and variance should
    // converge to (μ, σ²·I) within Monte-Carlo error.

    #[test]
    fn sample_recovers_isotropic_gaussian() {
        let mu = vec![1.5, -0.7];
        let sigma_sq = 0.64; // σ = 0.8
        let flow = from_isotropic_gaussian(canonical_b2(), mu.clone(), sigma_sq).unwrap();

        let config = FlowConfig {
            dt: 0.01,
            temperature: 1.0,
            n_steps: 1, // single-step after burn-in inside sample_many
            burn_in: 2_000,
            seed: Some(42),
        };
        let samples = flow
            .sample_many(&[5.0, 5.0], &config, 10_000, 1)
            .unwrap();

        let n = samples.len() as f64;
        let mean_x: f64 = samples.iter().map(|s| s[0]).sum::<f64>() / n;
        let mean_y: f64 = samples.iter().map(|s| s[1]).sum::<f64>() / n;
        let var_x: f64 =
            samples.iter().map(|s| (s[0] - mean_x).powi(2)).sum::<f64>() / n;
        let var_y: f64 =
            samples.iter().map(|s| (s[1] - mean_y).powi(2)).sum::<f64>() / n;

        let mean_err = ((mean_x - mu[0]).powi(2) + (mean_y - mu[1]).powi(2)).sqrt();
        let var_err = ((var_x - sigma_sq).powi(2) + (var_y - sigma_sq).powi(2)).sqrt();

        // 10 000 samples → Monte-Carlo error on mean ~ σ/√N ≈ 0.008,
        // on variance ~ σ²·√(2/N) ≈ 0.009. Allow generous tolerance
        // for finite-step discretization bias.
        assert!(mean_err < 0.1, "mean err {:.4e}", mean_err);
        assert!(var_err < 0.2, "var err {:.4e}", var_err);
    }

    // ── §4 DREAM: variance scales with temperature ────────────

    #[test]
    fn dream_variance_scales_with_temperature() {
        let mu = vec![0.0, 0.0];
        let sigma_sq = 1.0;
        let flow = from_isotropic_gaussian(canonical_b2(), mu, sigma_sq).unwrap();

        fn chain_var<F: Fn(&[f64]) -> Vec<f64>>(
            flow: &GenerativeFlow<F>,
            temperature: f64,
            seed: u64,
        ) -> f64 {
            let config = FlowConfig {
                dt: 0.01,
                temperature,
                n_steps: 1,
                burn_in: 1_000,
                seed: Some(seed),
            };
            let samples = flow.sample_many(&[0.0, 0.0], &config, 5_000, 1).unwrap();
            let n = samples.len() as f64;
            let m: f64 = samples.iter().map(|s| s[0]).sum::<f64>() / n;
            samples.iter().map(|s| (s[0] - m).powi(2)).sum::<f64>() / n
        }

        let cold = chain_var(&flow, 0.5, 1);
        let warm = chain_var(&flow, 1.0, 2);
        let hot = chain_var(&flow, 4.0, 3);

        assert!(cold < warm, "cold {:.3} >= warm {:.3}", cold, warm);
        assert!(warm < hot, "warm {:.3} >= hot {:.3}", warm, hot);
        assert!(
            hot > 3.0 * cold,
            "hot / cold = {:.2}× should be ≥ 3",
            hot / cold
        );
    }

    // ── error paths ────────────────────────────────────────────

    #[test]
    fn dimension_mismatch_rejected() {
        let flow = GenerativeFlow::new(
            canonical_b2(),
            |_: &[f64]| vec![0.0, 0.0],
        )
        .unwrap();
        let config = FlowConfig::sampling();
        let err = flow.sample(&[1.0, 2.0, 3.0], &config).unwrap_err();
        assert!(matches!(
            err,
            GenerativeFlowError::DimensionMismatch { .. }
        ));
    }

    #[test]
    fn singular_b_rejected() {
        // 4×4 with one all-zero row/column (singular antisymmetric matrix).
        let raw = vec![
            0.0, 1.0, 0.0, 0.0,
            -1.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 0.0,
        ];
        let b = ClosedTwoForm::new_constant(TwoForm::new(raw, 4).unwrap());
        // GenerativeFlow is generic on F (no Debug bound) so we
        // pattern-match the error explicitly instead of unwrap_err.
        match GenerativeFlow::new(b, |x: &[f64]| x.to_vec()) {
            Ok(_) => panic!("singular B should be rejected"),
            Err(e) => assert_eq!(e, GenerativeFlowError::SingularSymplectic),
        }
    }

    #[test]
    fn reconstruct_rejects_nonzero_temperature() {
        let flow = from_isotropic_gaussian(canonical_b2(), vec![0.0, 0.0], 1.0).unwrap();
        let mut bad = FlowConfig::reconstructing();
        bad.temperature = 0.5;
        let err = flow.reconstruct(&[1.0, 1.0], &bad).unwrap_err();
        assert!(matches!(err, GenerativeFlowError::NegativeTemperature(_)));
    }

    // ── SmallRng sanity ────────────────────────────────────────

    #[test]
    fn small_rng_normal_has_correct_moments() {
        let mut rng = SmallRng::seed_or_entropy(Some(12345));
        let n = 100_000;
        let samples: Vec<f64> = (0..n).map(|_| rng.standard_normal()).collect();
        let mean: f64 = samples.iter().sum::<f64>() / n as f64;
        let var: f64 =
            samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;
        // ~0.003 standard error on mean, ~0.0045 on variance.
        assert!(mean.abs() < 0.02, "mean = {}", mean);
        assert!((var - 1.0).abs() < 0.05, "var = {}", var);
    }
}
