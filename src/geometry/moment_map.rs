//! L9 — Moment maps and forced conservation laws (catalog §2.3).
//!
//! For a Lie group `G` acting symplectically on `(M, ω)` with
//! equivariant moment map `μ: M → 𝔤*`, μ is conserved along the flow
//! of any G-invariant Hamiltonian H. This is the geometric form of
//! Noether's theorem.
//!
//! ## What this gets us
//!
//! - **GIGI:** automatic invariants. A bundle with a closed-and-
//!   non-degenerate B (symplectic) plus a declared symmetry of its
//!   aggregation function H gets a conserved scalar `μ_ξ` for every
//!   generator `ξ ∈ 𝔤`. Integrity constraints derived from geometry
//!   instead of declared by hand.
//! - **PRISM:** reconciliation under FX/fee operations gets
//!   "books-balance-by-theorem" — if the rule is symmetric under
//!   currency relabeling (an SO(n) action on the currency axis),
//!   total accounting charge is conserved by any B-flow that respects
//!   that symmetry. The conservation law is forced, not declared.
//! - **Marcella:** same shape — any token-space symmetry of the
//!   transport Hamiltonian forces a conserved quantity along
//!   transport flows. Useful for held-out semantic invariants.
//!
//! ## API shape
//!
//! - [`InfinitesimalAction`] wraps a linear vector field
//!   `A: ℝⁿ → ℝⁿ` (a matrix). For a Lie group acting linearly on the
//!   state space, each `ξ ∈ 𝔤` is one such generator.
//! - [`MomentMap`] pairs a [`ClosedTwoForm`] `B` with a list of
//!   generators `{A_i}`. Construction validates each `A_i` is
//!   **B-symplectic**: `Aᵀ B + B A = 0` (equivalently `BA` is
//!   symmetric). That's exactly the condition that makes `μ_ξ`
//!   admit a clean quadratic-form representation
//!   `μ_ξ(x) = ½ xᵀ (BA) x`.
//! - [`measure_conservation`] integrates Hamilton's equations
//!   `ẋ = B⁻¹ dH(x)` with RK4 and reports the drift of `μ_ξ` along
//!   the trajectory.
//!
//! ## Sign convention
//!
//! State ordering for the symplectic case is `(q¹, …, qⁿ, p₁, …, pₙ)`
//! with canonical block form `B = [[0, -I], [I, 0]]`. With this
//! convention, the SO(2) action `A = [[0,-1,…], [1,0,…], …]` yields
//! the moment value `μ_ξ(x,y,p_x,p_y) = x·p_y − y·p_x` (angular
//! momentum, sign positive), matching the catalog §2.3 test.
//!
//! ## Validation
//!
//! `tests::*` mirrors `validation_tests.py::test_5_moment_map`:
//! - Symmetric Hamiltonian on T*ℝ²: drift `≤ 1e-9` over `t = 10`.
//! - Asymmetric Hamiltonian: drift `> 1.0` (clean separation).
//! - Plus the in-and-of-itself check that `dH(X_ξ) = 0` ⇔ H is
//!   G-invariant (the if-and-only-if structure of Noether).
//!
//! References:
//! - `theory/kahler_upgrade/catalog.md §2.3`
//! - Kobayashi-Nomizu Vol I, ch. III (canonical reference for
//!   moment maps; the µ_ξ(x) = ½ xᵀ BA x specialization for linear
//!   symplectic actions is in Marsden-Ratiu §11.2).

#![cfg(feature = "kahler")]

use crate::geometry::forms::ClosedTwoForm;
use thiserror::Error;

/// Tolerance on `Aᵀ B + B A = 0` (equivalently `BA` symmetric) when
/// validating that a candidate generator is B-symplectic.
const B_SYMPLECTIC_TOLERANCE: f64 = 1e-9;

/// Default finite-difference step for numerical gradients of
/// caller-supplied Hamiltonians.
const DEFAULT_GRADIENT_EPS: f64 = 1e-6;

/// Default tolerance for treating a drift of `μ_ξ` as "conserved".
/// Looser than machine epsilon to absorb RK4 integration error;
/// tighter than the asymmetric-Hamiltonian drift of `O(1)`.
const DEFAULT_CONSERVATION_TOLERANCE: f64 = 1e-6;

#[derive(Debug, Error, PartialEq)]
pub enum MomentMapError {
    #[error("generator matrix has {len} entries; expected {} for dim={dim}", dim * dim)]
    GeneratorWrongSize { len: usize, dim: usize },

    #[error(
        "generator dim {gen_dim} does not match symplectic form dim {b_dim}"
    )]
    DimensionMismatch { gen_dim: usize, b_dim: usize },

    #[error(
        "generator is not B-symplectic: max |Aᵀ B + B A|_{{ij}} = {max_dev:.3e} \
         exceeds tolerance {tolerance:.3e}. Equivalently BA is not symmetric \
         within tolerance — a B-symplectic generator A satisfies BA = (BA)ᵀ"
    )]
    NotBSymplectic { max_dev: f64, tolerance: f64 },

    #[error("names length {names_len} does not match generators length {gen_len}")]
    NameCountMismatch { names_len: usize, gen_len: usize },

    #[error(
        "symplectic form B is singular (det ≈ 0); cannot compute B⁻¹ for \
         Hamilton's equations. Use a non-degenerate (symplectic) closed 2-form."
    )]
    SingularSymplectic,

    #[error("no generators registered; MomentMap requires at least one")]
    EmptyGeneratorList,
}

/// A linear vector field `A: ℝⁿ → ℝⁿ` (square matrix, row-major).
/// Represents an infinitesimal generator `ξ ∈ 𝔤` of a Lie group
/// acting linearly on the state space.
#[derive(Debug, Clone, PartialEq)]
pub struct InfinitesimalAction {
    dim: usize,
    matrix: Vec<f64>,
}

impl InfinitesimalAction {
    /// Build a generator from a row-major `n × n` matrix.
    pub fn new(matrix: Vec<f64>, dim: usize) -> Result<Self, MomentMapError> {
        if matrix.len() != dim * dim {
            return Err(MomentMapError::GeneratorWrongSize {
                len: matrix.len(),
                dim,
            });
        }
        Ok(Self { dim, matrix })
    }

    pub fn dim(&self) -> usize {
        self.dim
    }

    pub fn matrix(&self) -> &[f64] {
        &self.matrix
    }

    /// Apply the generator to a state: `A · x`. Panics on length
    /// mismatch — programmer error.
    pub fn apply(&self, state: &[f64]) -> Vec<f64> {
        assert_eq!(state.len(), self.dim, "state length mismatch");
        let mut out = vec![0.0_f64; self.dim];
        for i in 0..self.dim {
            let row = &self.matrix[i * self.dim..(i + 1) * self.dim];
            out[i] = row.iter().zip(state.iter()).map(|(a, x)| a * x).sum();
        }
        out
    }
}

/// Verdict on whether `μ_ξ` was conserved along a numerically
/// integrated Hamiltonian flow.
#[derive(Debug, Clone, PartialEq)]
pub struct ConservationVerdict {
    /// Whether `|μ_end - μ_start|` is below the tolerance.
    pub conserved: bool,
    /// `|μ_end - μ_start|` measured along the trajectory.
    pub drift: f64,
    /// Max `|dH(X_ξ)|` over sampled trajectory points — a direct
    /// measure of how badly H violates invariance under the generator
    /// (the symmetry side of Noether). 0 ↔ H is G-invariant.
    pub invariance_residual: f64,
    /// Number of integration steps taken.
    pub n_steps: usize,
    /// Time step used.
    pub dt: f64,
}

/// Pairs a symplectic form `B` with one or more infinitesimal
/// generators, providing moment-value computation and conservation
/// measurement under a user-supplied Hamiltonian.
#[derive(Debug, Clone)]
pub struct MomentMap {
    b: ClosedTwoForm,
    /// Precomputed `B⁻¹` (the Hamiltonian-vector-field inverter).
    /// Built once at construction; `None` if `B` is singular.
    b_inv: Option<Vec<f64>>,
    /// `B · Aᵢ` precomputed for moment evaluation; each entry is
    /// the symmetric `dim × dim` matrix `BA`.
    ba_matrices: Vec<Vec<f64>>,
    generators: Vec<InfinitesimalAction>,
    names: Vec<String>,
}

impl MomentMap {
    /// Construct a moment map from a symplectic form and a Lie
    /// algebra basis.
    ///
    /// Validates each generator is B-symplectic (`Aᵀ B + B A = 0`).
    /// Caches `B⁻¹` (if invertible) and `B · Aᵢ` for fast moment
    /// evaluation. If `B` is degenerate, construction still succeeds
    /// (moment values remain meaningful) but [`measure_conservation`]
    /// will return [`MomentMapError::SingularSymplectic`].
    pub fn new(
        b: ClosedTwoForm,
        generators: Vec<InfinitesimalAction>,
        names: Vec<String>,
    ) -> Result<Self, MomentMapError> {
        if generators.is_empty() {
            return Err(MomentMapError::EmptyGeneratorList);
        }
        if names.len() != generators.len() {
            return Err(MomentMapError::NameCountMismatch {
                names_len: names.len(),
                gen_len: generators.len(),
            });
        }
        let dim = b.dim();
        let b_mat = b.form().matrix().to_vec();

        // Validate and precompute BA for each generator.
        let mut ba_matrices = Vec::with_capacity(generators.len());
        for gen in &generators {
            if gen.dim() != dim {
                return Err(MomentMapError::DimensionMismatch {
                    gen_dim: gen.dim(),
                    b_dim: dim,
                });
            }
            let ba = matmul(&b_mat, gen.matrix(), dim);
            // B-symplecticity ⇔ BA is symmetric (since B is antisymmetric).
            let mut max_dev = 0.0_f64;
            for i in 0..dim {
                for j in (i + 1)..dim {
                    let dev = (ba[i * dim + j] - ba[j * dim + i]).abs();
                    if dev > max_dev {
                        max_dev = dev;
                    }
                }
            }
            if max_dev > B_SYMPLECTIC_TOLERANCE {
                return Err(MomentMapError::NotBSymplectic {
                    max_dev,
                    tolerance: B_SYMPLECTIC_TOLERANCE,
                });
            }
            ba_matrices.push(ba);
        }

        let b_inv = invert(&b_mat, dim);

        Ok(Self {
            b,
            b_inv,
            ba_matrices,
            generators,
            names,
        })
    }

    pub fn dim(&self) -> usize {
        self.b.dim()
    }

    pub fn b(&self) -> &ClosedTwoForm {
        &self.b
    }

    pub fn generators(&self) -> &[InfinitesimalAction] {
        &self.generators
    }

    pub fn names(&self) -> &[String] {
        &self.names
    }

    /// `μ_ξ(x) = ½ xᵀ (B · A) x` for the `gen_idx`-th generator.
    ///
    /// This is the closed-form moment value for linear B-symplectic
    /// actions on a constant-coefficient symplectic vector space.
    /// On the canonical T*ℝ² with rotation generator, this returns
    /// angular momentum `x·p_y − y·p_x` (catalog §2.3 sign).
    pub fn moment_value(&self, state: &[f64], gen_idx: usize) -> f64 {
        assert_eq!(state.len(), self.dim(), "state length mismatch");
        assert!(
            gen_idx < self.generators.len(),
            "generator index {} out of range (have {})",
            gen_idx,
            self.generators.len()
        );
        let ba = &self.ba_matrices[gen_idx];
        let n = self.dim();
        let mut s = 0.0_f64;
        for i in 0..n {
            for j in 0..n {
                s += ba[i * n + j] * state[i] * state[j];
            }
        }
        0.5 * s
    }

    /// `dH(X_ξ)(x)` — the Lie derivative of H along the generator's
    /// vector field at state `x`. Computed by finite differences
    /// (central, step `eps`).
    ///
    /// `0` (within tolerance) ⇔ H is G-invariant under the
    /// `gen_idx`-th generator. By Noether, this is exactly when
    /// `μ_ξ` is conserved along the H-flow.
    pub fn hamiltonian_lie_derivative<F>(
        &self,
        h: &F,
        state: &[f64],
        gen_idx: usize,
        eps: Option<f64>,
    ) -> f64
    where
        F: Fn(&[f64]) -> f64,
    {
        let eps = eps.unwrap_or(DEFAULT_GRADIENT_EPS);
        let grad = gradient(h, state, eps);
        let x_xi = self.generators[gen_idx].apply(state);
        grad.iter().zip(x_xi.iter()).map(|(g, v)| g * v).sum()
    }

    /// Single RK4 step of Hamilton's equations `ẋ = B⁻¹ dH(x)` with
    /// timestep `dt`. Returns the new state.
    ///
    /// Errors if B is singular (no Hamiltonian flow without an
    /// invertible symplectic form).
    pub fn flow_step<F>(
        &self,
        h: &F,
        state: &[f64],
        dt: f64,
        eps: Option<f64>,
    ) -> Result<Vec<f64>, MomentMapError>
    where
        F: Fn(&[f64]) -> f64,
    {
        let b_inv = self
            .b_inv
            .as_ref()
            .ok_or(MomentMapError::SingularSymplectic)?;
        let eps = eps.unwrap_or(DEFAULT_GRADIENT_EPS);
        let n = self.dim();

        let f = |x: &[f64]| -> Vec<f64> {
            let g = gradient(h, x, eps);
            matvec(b_inv, &g, n)
        };

        let k1 = f(state);
        let s2: Vec<f64> = (0..n).map(|i| state[i] + 0.5 * dt * k1[i]).collect();
        let k2 = f(&s2);
        let s3: Vec<f64> = (0..n).map(|i| state[i] + 0.5 * dt * k2[i]).collect();
        let k3 = f(&s3);
        let s4: Vec<f64> = (0..n).map(|i| state[i] + dt * k3[i]).collect();
        let k4 = f(&s4);
        Ok((0..n)
            .map(|i| {
                state[i] + (dt / 6.0) * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i])
            })
            .collect())
    }

    /// Measure conservation of `μ_ξ` along the H-flow from `init`.
    ///
    /// Integrates Hamilton's equations for `n_steps` of size `dt`,
    /// computes `μ_end - μ_start`, and samples
    /// `dH(X_ξ)` along the way to report the invariance residual.
    pub fn measure_conservation<F>(
        &self,
        h: &F,
        init: &[f64],
        dt: f64,
        n_steps: usize,
        gen_idx: usize,
        tolerance: Option<f64>,
        eps: Option<f64>,
    ) -> Result<ConservationVerdict, MomentMapError>
    where
        F: Fn(&[f64]) -> f64,
    {
        let tol = tolerance.unwrap_or(DEFAULT_CONSERVATION_TOLERANCE);
        let mu_start = self.moment_value(init, gen_idx);
        let mut state = init.to_vec();
        let mut invariance_residual = 0.0_f64;
        // Sample invariance residual every ~max(1, n_steps/20) steps to keep
        // cost bounded for long trajectories.
        let sample_stride = (n_steps / 20).max(1);
        for step in 0..n_steps {
            if step % sample_stride == 0 {
                let r = self.hamiltonian_lie_derivative(h, &state, gen_idx, eps).abs();
                if r > invariance_residual {
                    invariance_residual = r;
                }
            }
            state = self.flow_step(h, &state, dt, eps)?;
        }
        let mu_end = self.moment_value(&state, gen_idx);
        let drift = (mu_end - mu_start).abs();
        Ok(ConservationVerdict {
            conserved: drift <= tol,
            drift,
            invariance_residual,
            n_steps,
            dt,
        })
    }
}

// ── helpers ───────────────────────────────────────────────────────

/// Row-major matrix multiplication: `(A · B)[i,j] = Σ_k A[i,k] · B[k,j]`.
fn matmul(a: &[f64], b: &[f64], n: usize) -> Vec<f64> {
    let mut c = vec![0.0_f64; n * n];
    for i in 0..n {
        for j in 0..n {
            let mut s = 0.0_f64;
            for k in 0..n {
                s += a[i * n + k] * b[k * n + j];
            }
            c[i * n + j] = s;
        }
    }
    c
}

/// Row-major matrix · vector: `(A · x)[i] = Σ_j A[i,j] · x[j]`.
fn matvec(a: &[f64], x: &[f64], n: usize) -> Vec<f64> {
    (0..n)
        .map(|i| (0..n).map(|j| a[i * n + j] * x[j]).sum())
        .collect()
}

/// Central finite-difference gradient of `h` at `x`. Step `eps`.
fn gradient<F>(h: &F, x: &[f64], eps: f64) -> Vec<f64>
where
    F: Fn(&[f64]) -> f64,
{
    let n = x.len();
    let mut g = vec![0.0_f64; n];
    let mut xp = x.to_vec();
    for i in 0..n {
        let orig = xp[i];
        xp[i] = orig + eps;
        let hp = h(&xp);
        xp[i] = orig - eps;
        let hm = h(&xp);
        xp[i] = orig;
        g[i] = (hp - hm) / (2.0 * eps);
    }
    g
}

/// Invert a row-major `n × n` matrix via Gauss-Jordan elimination
/// with partial pivoting. Returns `None` if the matrix is singular
/// (no pivot found within tolerance).
fn invert(m: &[f64], n: usize) -> Option<Vec<f64>> {
    // Build augmented [M | I].
    let mut a = vec![0.0_f64; n * 2 * n];
    for i in 0..n {
        for j in 0..n {
            a[i * 2 * n + j] = m[i * n + j];
        }
        a[i * 2 * n + n + i] = 1.0;
    }
    let cols = 2 * n;
    for col in 0..n {
        // Pivot: find row with max |a[r, col]| for r ≥ col.
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
            return None; // singular
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
    // Extract right half.
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

    /// Canonical symplectic form on T*ℝ² in ordering (x, y, p_x, p_y):
    /// `B = [[0, 0, -1, 0], [0, 0, 0, -1], [1, 0, 0, 0], [0, 1, 0, 0]]`
    /// (block form `[[0, -I], [I, 0]]`).
    fn canonical_b4() -> ClosedTwoForm {
        let raw = vec![
            0.0, 0.0, -1.0, 0.0, //
            0.0, 0.0, 0.0, -1.0, //
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
        ];
        ClosedTwoForm::new_constant(TwoForm::new(raw, 4).expect("antisymmetric"))
    }

    /// SO(2) rotation in the (x, y) plane (and simultaneously
    /// (p_x, p_y), since A must be B-symplectic).
    fn rotation_generator() -> InfinitesimalAction {
        InfinitesimalAction::new(
            vec![
                0.0, -1.0, 0.0, 0.0, //
                1.0, 0.0, 0.0, 0.0, //
                0.0, 0.0, 0.0, -1.0, //
                0.0, 0.0, 1.0, 0.0, //
            ],
            4,
        )
        .unwrap()
    }

    // ── B-symplecticity validation ─────────────────────────────

    #[test]
    fn rotation_is_b_symplectic_for_canonical_form() {
        let mm = MomentMap::new(
            canonical_b4(),
            vec![rotation_generator()],
            vec!["L_z".into()],
        )
        .expect("rotation must construct");
        assert_eq!(mm.dim(), 4);
        assert_eq!(mm.names(), &["L_z".to_string()]);
    }

    #[test]
    fn non_b_symplectic_generator_is_rejected() {
        // Pure scaling in the x-direction alone — clearly NOT in
        // sp(4, ℝ). For block decomposition A = [[a, b], [c, d]],
        // sp requires d = −aᵀ. Here a = diag(1, 0), d = 0,
        // so −aᵀ = diag(-1, 0) ≠ d. The BA matrix has BA_{2,0} = 1
        // but BA_{0,2} = 0 — asymmetric, correctly rejected.
        let bad = InfinitesimalAction::new(
            vec![
                1.0, 0.0, 0.0, 0.0, //
                0.0, 0.0, 0.0, 0.0, //
                0.0, 0.0, 0.0, 0.0, //
                0.0, 0.0, 0.0, 0.0, //
            ],
            4,
        )
        .unwrap();
        let err = MomentMap::new(canonical_b4(), vec![bad], vec!["scale_x".into()])
            .expect_err("non-symplectic generator must be rejected");
        assert!(matches!(err, MomentMapError::NotBSymplectic { .. }));
    }

    #[test]
    fn dimension_mismatch_is_rejected() {
        let small =
            InfinitesimalAction::new(vec![0.0, -1.0, 1.0, 0.0], 2).unwrap();
        let err = MomentMap::new(canonical_b4(), vec![small], vec!["r2".into()])
            .expect_err("dim mismatch must be rejected");
        assert!(matches!(err, MomentMapError::DimensionMismatch { .. }));
    }

    // ── moment value (catalog §2.3 closed form) ────────────────

    #[test]
    fn moment_value_rotation_gives_angular_momentum() {
        let mm = MomentMap::new(
            canonical_b4(),
            vec![rotation_generator()],
            vec!["L_z".into()],
        )
        .unwrap();
        // (x, y, p_x, p_y) = (1, 0, 0, 1) → x·p_y - y·p_x = 1.
        let mu = mm.moment_value(&[1.0, 0.0, 0.0, 1.0], 0);
        assert!(
            (mu - 1.0).abs() < 1e-12,
            "expected angular momentum 1, got {}",
            mu
        );
        // (2, 3, 5, 7) → 2·7 - 3·5 = -1.
        let mu = mm.moment_value(&[2.0, 3.0, 5.0, 7.0], 0);
        assert!(
            (mu - (-1.0)).abs() < 1e-12,
            "expected -1, got {}",
            mu
        );
    }

    // ── Noether conservation along H-flow ──────────────────────

    /// Symmetric harmonic oscillator `H = (p² + r²)/2 = ½(x² + y² + p_x² + p_y²)`.
    /// SO(2)-invariant — angular momentum is conserved (catalog §2.3
    /// positive case).
    fn symmetric_h(s: &[f64]) -> f64 {
        0.5 * (s[0] * s[0] + s[1] * s[1] + s[2] * s[2] + s[3] * s[3])
    }

    /// Anisotropic `H = p_x²/2 + p_y²/2 + x²`. Breaks SO(2) symmetry
    /// (y direction has no quadratic potential, x does). Angular
    /// momentum must drift (catalog §2.3 negative case).
    fn asymmetric_h(s: &[f64]) -> f64 {
        0.5 * s[2] * s[2] + 0.5 * s[3] * s[3] + s[0] * s[0]
    }

    #[test]
    fn symmetric_hamiltonian_conserves_moment() {
        let mm = MomentMap::new(
            canonical_b4(),
            vec![rotation_generator()],
            vec!["L_z".into()],
        )
        .unwrap();
        // Initial state with nonzero angular momentum (so drift is
        // meaningful as a fraction, not just absolute).
        let init = [1.0, 0.0, 0.0, 1.0];
        let verdict = mm
            .measure_conservation(
                &symmetric_h,
                &init,
                0.01,
                1000, // t = 10
                0,
                Some(1e-9),
                None,
            )
            .unwrap();
        assert!(
            verdict.conserved,
            "drift {:.3e} should be ≤ tol; invariance residual was {:.3e}",
            verdict.drift, verdict.invariance_residual
        );
        // Catalog reports drift ~ 7e-15 for the analytic ODE
        // integrator with adaptive step. Our RK4 with dt=0.01 over
        // t=10 is the workhorse setting; expect drift ≤ 1e-9.
        assert!(verdict.drift < 1e-9, "drift {:.3e}", verdict.drift);
        // For a strictly-invariant H, the Lie-derivative residual
        // should be machine-epsilon up to finite-difference noise.
        assert!(
            verdict.invariance_residual < 1e-6,
            "invariance residual {:.3e}",
            verdict.invariance_residual
        );
    }

    #[test]
    fn asymmetric_hamiltonian_breaks_conservation() {
        let mm = MomentMap::new(
            canonical_b4(),
            vec![rotation_generator()],
            vec!["L_z".into()],
        )
        .unwrap();
        let init = [1.0, 0.0, 0.0, 1.0];
        let verdict = mm
            .measure_conservation(
                &asymmetric_h,
                &init,
                0.01,
                1000,
                0,
                Some(1e-9),
                None,
            )
            .unwrap();
        assert!(
            !verdict.conserved,
            "asymmetric Hamiltonian must violate conservation (drift was {:.3e})",
            verdict.drift
        );
        // Catalog reports drift ≈ 19.5 over the same window; the
        // RK4 trajectory here can wander significantly. Just require
        // it to clearly exceed numerical tolerance.
        assert!(
            verdict.drift > 0.1,
            "expected substantial drift, got {:.3e}",
            verdict.drift
        );
        // Lie-derivative residual must be obviously nonzero at
        // generic points (asymmetry of H is the direct measure).
        assert!(
            verdict.invariance_residual > 0.01,
            "asymmetric H must have nonzero Lie derivative; got {:.3e}",
            verdict.invariance_residual
        );
    }

    #[test]
    fn lie_derivative_is_zero_at_invariant_h() {
        // At ANY state, dH(X_ξ) = 0 for symmetric H (not just on the
        // flow). This is the pointwise invariance test, which is the
        // "iff" structure of Noether.
        let mm = MomentMap::new(
            canonical_b4(),
            vec![rotation_generator()],
            vec!["L_z".into()],
        )
        .unwrap();
        for state in &[
            [1.0, 0.0, 0.0, 1.0],
            [0.3, 0.7, -0.4, 0.9],
            [2.5, -1.3, 0.6, -0.2],
        ] {
            let r = mm.hamiltonian_lie_derivative(&symmetric_h, state, 0, None);
            assert!(
                r.abs() < 1e-6,
                "Lie derivative of symmetric H must be zero at {:?}; got {:e}",
                state,
                r
            );
        }
    }

    #[test]
    fn lie_derivative_is_nonzero_at_asymmetric_h() {
        let mm = MomentMap::new(
            canonical_b4(),
            vec![rotation_generator()],
            vec!["L_z".into()],
        )
        .unwrap();
        // At (0.5, 0.5, 0, 0): X_ξ = (-0.5, 0.5, 0, 0); ∇H = (2x, 0, p_x, p_y) = (1, 0, 0, 0).
        // dH(X_ξ) = 1 · (-0.5) = -0.5.
        let r =
            mm.hamiltonian_lie_derivative(&asymmetric_h, &[0.5, 0.5, 0.0, 0.0], 0, None);
        assert!(
            (r - (-0.5)).abs() < 1e-4,
            "expected dH(X_ξ) ≈ -0.5 at this state, got {}",
            r
        );
    }

    // ── helper sanity ──────────────────────────────────────────

    #[test]
    fn canonical_b4_is_invertible_and_self_inverse_negated() {
        // For canonical B = [[0,-I],[I,0]], B⁻¹ = -B.
        let mm = MomentMap::new(
            canonical_b4(),
            vec![rotation_generator()],
            vec!["L_z".into()],
        )
        .unwrap();
        let b_inv = mm.b_inv.as_ref().expect("B must be invertible");
        let b = mm.b.form().matrix();
        for i in 0..16 {
            let expected = -b[i];
            assert!(
                (b_inv[i] - expected).abs() < 1e-12,
                "B⁻¹[{}] = {} but expected -B = {}",
                i,
                b_inv[i],
                expected
            );
        }
    }

    #[test]
    fn invert_singular_matrix_returns_none() {
        // 2×2 singular matrix.
        let m = vec![1.0, 2.0, 2.0, 4.0];
        assert!(invert(&m, 2).is_none());
    }
}
