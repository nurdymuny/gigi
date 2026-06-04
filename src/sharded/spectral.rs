//! Distributed Lanczos for sharded SPECTRAL (T7 Rust port).
//!
//! The math is locked by T7 (`theory/poincare_to_sharding/validation/
//! t7_distributed_lanczos_spectral.py`). T7 validated that distributed
//! Lanczos converges to λ_1(L) for **all** graph classes — including
//! expanders where the T5 naive `min(per-shard λ_1)` bound fails
//! 5–7×. This is the UNIVERSAL sharded SPECTRAL primitive.
//!
//! ## Algorithm
//!
//! Block-matvec form of Lanczos iteration with twice-is-enough
//! Gram-Schmidt reorthogonalization, kernel projection against the
//! constant vector (the Laplacian null space), and adaptive
//! convergence detection on the smallest nonzero Ritz value.
//!
//! Per iteration:
//! 1. `w = block_matvec(A_S, A_T, B, v_k)` — the *sharded* step.
//!    Each shard contributes its block product; in a real distributed
//!    implementation this is one round-trip of communication.
//! 2. `α_k = ⟨v_k, w⟩`
//! 3. `w ← w − α_k v_k − β_k v_{k−1}`
//! 4. Full reorthogonalization (twice) against all previous basis
//!    vectors.
//! 5. Project out the all-ones direction (defense against kernel drift).
//! 6. `β_{k+1} = ‖w‖; v_{k+1} = w / β_{k+1}`
//! 7. Compute λ_1 of `T_{k+1}` (small K × K dense symmetric eigen).
//! 8. If λ_1 stable over `convergence_window` consecutive steps, stop.
//!
//! ## Communication cost
//!
//! K iterations = K round-trips. The test cases in this module converge
//! within K ≤ 30 for expanders (the case T5 missed) and K ≤ 120 for
//! slow-mixing graphs. The communication is per-iteration; total
//! latency is bounded by the smallest spectral gap times K.

use nalgebra::{DMatrix, SymmetricEigen};

/// Configuration for distributed Lanczos.
#[derive(Clone, Debug)]
pub struct DistributedLanczosConfig {
    /// Maximum iterations before terminating regardless of convergence.
    pub k_max: u32,
    /// Deterministic seed for the initial vector.
    pub seed: u64,
    /// Number of consecutive λ_1 values that must be stable for
    /// early termination.
    pub convergence_window: u32,
    /// Relative tolerance for λ_1 stability check.
    pub convergence_tol: f64,
}

impl Default for DistributedLanczosConfig {
    fn default() -> Self {
        Self {
            k_max: 120,
            seed: 1,
            convergence_window: 3,
            convergence_tol: 1e-10,
        }
    }
}

/// Result of a distributed Lanczos run.
#[derive(Clone, Debug)]
pub struct DistributedLanczosResult {
    /// Smallest non-zero Ritz value — the Lanczos approximation to
    /// λ_1(L).
    pub lambda_1: f64,
    /// Iterations actually performed (≤ `k_max`).
    pub iterations_used: u32,
    /// True iff the convergence-window check triggered termination.
    pub converged_by_window: bool,
}

/// Sharded block matrix-vector product `L · v` using only the per-shard
/// blocks `(A_S, A_T, B)`. The full Laplacian `L` is never materialized.
///
/// Block form:
/// ```text
///   L v = [A_S  B  ] [v_S]   = [A_S v_S + B v_T  ]
///         [B^T  A_T] [v_T]     [B^T v_S + A_T v_T]
/// ```
///
/// Each block product corresponds to one shard's local matvec plus
/// the boundary contribution from cut edges. In a real distributed
/// implementation this is one round-trip of communication.
pub fn block_matvec(
    a_s: &[Vec<f64>],
    a_t: &[Vec<f64>],
    b: &[Vec<f64>],
    v: &[f64],
    size_s: usize,
) -> Vec<f64> {
    let size_t = a_t.len();
    let n = size_s + size_t;
    debug_assert_eq!(v.len(), n);
    debug_assert_eq!(a_s.len(), size_s);
    debug_assert_eq!(b.len(), size_s);

    let (v_s, v_t) = v.split_at(size_s);
    let mut out = vec![0.0; n];

    // A_S v_S + B v_T
    for i in 0..size_s {
        let mut s = 0.0;
        for (j, &x) in v_s.iter().enumerate() {
            s += a_s[i][j] * x;
        }
        for (j, &x) in v_t.iter().enumerate() {
            s += b[i][j] * x;
        }
        out[i] = s;
    }
    // B^T v_S + A_T v_T
    for i in 0..size_t {
        let mut s = 0.0;
        for (j, &x) in v_s.iter().enumerate() {
            s += b[j][i] * x;
        }
        for (j, &x) in v_t.iter().enumerate() {
            s += a_t[i][j] * x;
        }
        out[size_s + i] = s;
    }
    out
}

/// Deterministic pseudo-random normal samples for the initial Lanczos
/// vector. Xorshift64 + Box-Muller. Standalone (no rand dep needed).
fn random_normal(state: &mut u64) -> f64 {
    fn step(s: &mut u64) -> u64 {
        *s ^= *s << 13;
        *s ^= *s >> 7;
        *s ^= *s << 17;
        *s
    }
    let u1 = (step(state) as f64).abs() / (u64::MAX as f64);
    let u2 = (step(state) as f64).abs() / (u64::MAX as f64);
    let r = (-2.0_f64 * u1.max(1e-15).ln()).sqrt();
    let theta = 2.0_f64 * std::f64::consts::PI * u2;
    r * theta.cos()
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

fn axpy(out: &mut [f64], a: f64, x: &[f64]) {
    for (o, &xi) in out.iter_mut().zip(x.iter()) {
        *o += a * xi;
    }
}

fn norm(a: &[f64]) -> f64 {
    a.iter().map(|x| x * x).sum::<f64>().sqrt()
}

/// Smallest non-zero eigenvalue of a K × K symmetric tridiagonal
/// matrix `T_k`, built from `alphas` (diagonal) and `betas` (off-
/// diagonal, length k+1 with `betas[0]` unused).
fn smallest_nonzero_eigenvalue(alphas: &[f64], betas: &[f64], eps: f64) -> f64 {
    let k = alphas.len();
    if k == 0 {
        return 0.0;
    }
    // Build a dense K x K symmetric matrix
    let mut t = DMatrix::<f64>::zeros(k, k);
    for i in 0..k {
        t[(i, i)] = alphas[i];
        if i + 1 < k {
            t[(i, i + 1)] = betas[i + 1];
            t[(i + 1, i)] = betas[i + 1];
        }
    }
    let eigen = SymmetricEigen::new(t);
    let mut eigs: Vec<f64> = eigen.eigenvalues.iter().copied().collect();
    eigs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    for &e in &eigs {
        if e > eps {
            return e;
        }
    }
    0.0
}

/// Distributed Lanczos with full reorthogonalization, kernel projection,
/// and adaptive convergence detection on the smallest nonzero Ritz value.
///
/// Inputs are the per-shard Laplacian blocks `(A_S, A_T, B)` and the
/// size of partition S. The full Laplacian `L` is never reconstructed
/// inside the algorithm.
///
/// Returns the smallest non-zero eigenvalue of L (the algebraic
/// connectivity / spectral gap), the number of iterations used, and
/// whether convergence triggered.
pub fn distributed_lanczos(
    a_s: &[Vec<f64>],
    a_t: &[Vec<f64>],
    b: &[Vec<f64>],
    size_s: usize,
    config: &DistributedLanczosConfig,
) -> DistributedLanczosResult {
    let size_t = a_t.len();
    let n = size_s + size_t;
    if n == 0 {
        return DistributedLanczosResult {
            lambda_1: 0.0,
            iterations_used: 0,
            converged_by_window: false,
        };
    }

    // Initial vector orthogonal to the all-ones direction, normalized.
    let mut state = config.seed.max(1);
    let mut v: Vec<f64> = (0..n).map(|_| random_normal(&mut state)).collect();
    let one = vec![1.0 / (n as f64).sqrt(); n];
    let dot_one = dot(&v, &one);
    for (vi, oi) in v.iter_mut().zip(one.iter()) {
        *vi -= dot_one * oi;
    }
    let nv = norm(&v);
    if nv < 1e-12 {
        return DistributedLanczosResult {
            lambda_1: 0.0,
            iterations_used: 0,
            converged_by_window: false,
        };
    }
    for vi in v.iter_mut() {
        *vi /= nv;
    }

    let mut v_history: Vec<Vec<f64>> = vec![v];
    let mut alphas: Vec<f64> = Vec::new();
    let mut betas: Vec<f64> = vec![0.0];
    let mut w_prev: Vec<f64> = vec![0.0; n];
    let mut lambda_1_history: Vec<f64> = Vec::new();
    let mut converged_by_window = false;

    for _k in 0..config.k_max {
        let v_k = v_history.last().unwrap().clone();
        let mut w = block_matvec(a_s, a_t, b, &v_k, size_s);
        let alpha = dot(&v_k, &w);
        alphas.push(alpha);

        // w ← w − α v_k − β w_prev
        axpy(&mut w, -alpha, &v_k);
        let last_beta = *betas.last().unwrap();
        axpy(&mut w, -last_beta, &w_prev);

        // Twice-is-enough Gram-Schmidt against all previous vectors.
        for _pass in 0..2 {
            for vp in &v_history {
                let d = dot(&w, vp);
                axpy(&mut w, -d, vp);
            }
        }
        // Project out the all-ones kernel direction
        let d_one = dot(&w, &one);
        axpy(&mut w, -d_one, &one);

        let beta = norm(&w);
        betas.push(beta);
        if beta < 1e-12 {
            break;
        }
        w_prev = v_k;
        for wi in w.iter_mut() {
            *wi /= beta;
        }
        v_history.push(w);

        // Convergence check
        let lam = smallest_nonzero_eigenvalue(&alphas, &betas, 1e-9);
        lambda_1_history.push(lam);
        if lambda_1_history.len() >= config.convergence_window as usize {
            let recent = &lambda_1_history[lambda_1_history.len()
                - config.convergence_window as usize..];
            let max_v = recent.iter().cloned().fold(f64::MIN, f64::max);
            let min_v = recent.iter().cloned().fold(f64::MAX, f64::min);
            if max_v - min_v
                < config.convergence_tol * (recent.last().unwrap().abs() + 1e-12)
            {
                converged_by_window = true;
                break;
            }
        }
    }

    let k_used = alphas.len();
    let lambda_1 = smallest_nonzero_eigenvalue(&alphas, &betas, 1e-9);
    DistributedLanczosResult {
        lambda_1,
        iterations_used: k_used as u32,
        converged_by_window,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Combinatorial Laplacian of the path graph P_n.
    fn path_laplacian(n: usize) -> Vec<Vec<f64>> {
        let mut l = vec![vec![0.0; n]; n];
        for i in 0..(n - 1) {
            l[i][i] += 1.0;
            l[i + 1][i + 1] += 1.0;
            l[i][i + 1] -= 1.0;
            l[i + 1][i] -= 1.0;
        }
        l
    }

    /// Combinatorial Laplacian of the cycle graph C_n.
    fn cycle_laplacian(n: usize) -> Vec<Vec<f64>> {
        let mut l = vec![vec![0.0; n]; n];
        for i in 0..n {
            let j = (i + 1) % n;
            l[i][i] += 1.0;
            l[j][j] += 1.0;
            l[i][j] -= 1.0;
            l[j][i] -= 1.0;
        }
        l
    }

    /// Complete bipartite graph K_{a,b} as a Laplacian. This is a
    /// well-known expander when a == b.
    fn complete_bipartite_laplacian(a: usize, b: usize) -> Vec<Vec<f64>> {
        let n = a + b;
        let mut l = vec![vec![0.0; n]; n];
        for i in 0..a {
            for j in a..(a + b) {
                l[i][i] += 1.0;
                l[j][j] += 1.0;
                l[i][j] -= 1.0;
                l[j][i] -= 1.0;
            }
        }
        l
    }

    /// Split a full Laplacian L into (A_S, A_T, B) by the index set S.
    fn split(l: &[Vec<f64>], s_indices: &[usize]) -> (Vec<Vec<f64>>, Vec<Vec<f64>>, Vec<Vec<f64>>) {
        let n = l.len();
        let s_set: std::collections::HashSet<usize> = s_indices.iter().copied().collect();
        let mut s_ord: Vec<usize> = s_indices.to_vec();
        s_ord.sort();
        let t_ord: Vec<usize> = (0..n).filter(|i| !s_set.contains(i)).collect();
        let a_s: Vec<Vec<f64>> = s_ord
            .iter()
            .map(|&i| s_ord.iter().map(|&j| l[i][j]).collect())
            .collect();
        let a_t: Vec<Vec<f64>> = t_ord
            .iter()
            .map(|&i| t_ord.iter().map(|&j| l[i][j]).collect())
            .collect();
        let b: Vec<Vec<f64>> = s_ord
            .iter()
            .map(|&i| t_ord.iter().map(|&j| l[i][j]).collect())
            .collect();
        (a_s, a_t, b)
    }

    /// Direct ground truth λ_1 via SymmetricEigen on the full L.
    fn direct_lambda_1(l: &[Vec<f64>]) -> f64 {
        let n = l.len();
        let mut m = DMatrix::<f64>::zeros(n, n);
        for i in 0..n {
            for j in 0..n {
                m[(i, j)] = l[i][j];
            }
        }
        let eigen = SymmetricEigen::new(m);
        let mut eigs: Vec<f64> = eigen.eigenvalues.iter().copied().collect();
        eigs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        for &e in &eigs {
            if e > 1e-9 {
                return e;
            }
        }
        0.0
    }

    #[test]
    fn block_matvec_reconstructs_full_matvec() {
        let l = path_laplacian(6);
        let v: Vec<f64> = vec![1.0, 2.0, -1.0, 0.5, -0.3, 0.7];
        // Full matvec
        let mut expected = vec![0.0; 6];
        for i in 0..6 {
            for j in 0..6 {
                expected[i] += l[i][j] * v[j];
            }
        }
        // Block matvec with S = {0, 1, 2}, T = {3, 4, 5}
        let s_indices = vec![0, 1, 2];
        let (a_s, a_t, b) = split(&l, &s_indices);
        let got = block_matvec(&a_s, &a_t, &b, &v, 3);
        for i in 0..6 {
            assert!(
                (got[i] - expected[i]).abs() < 1e-12,
                "block_matvec mismatch at {}: expected {}, got {}",
                i, expected[i], got[i]
            );
        }
    }

    #[test]
    fn lanczos_recovers_lambda_1_on_path_p20() {
        // Closed form: λ_1(P_n) = 2 * (1 - cos(π/n))
        let n = 20;
        let l = path_laplacian(n);
        let expected = 2.0 * (1.0 - (std::f64::consts::PI / n as f64).cos());
        let direct = direct_lambda_1(&l);
        // sanity: closed form matches direct
        assert!((direct - expected).abs() < 1e-10);

        let s_indices: Vec<usize> = (0..n / 2).collect();
        let (a_s, a_t, b) = split(&l, &s_indices);
        let config = DistributedLanczosConfig {
            k_max: 120,
            ..Default::default()
        };
        let result = distributed_lanczos(&a_s, &a_t, &b, n / 2, &config);
        let rel_err = (result.lambda_1 - direct).abs() / direct;
        assert!(
            rel_err < 1e-4,
            "P_20: expected λ_1 = {}, got {} (rel err {:.2e}), K used = {}",
            direct,
            result.lambda_1,
            rel_err,
            result.iterations_used
        );
    }

    #[test]
    fn lanczos_recovers_lambda_1_on_cycle_c20() {
        // Closed form: λ_1(C_n) = 2 * (1 - cos(2π/n))
        let n = 20;
        let l = cycle_laplacian(n);
        let direct = direct_lambda_1(&l);

        let s_indices: Vec<usize> = (0..n / 2).collect();
        let (a_s, a_t, b) = split(&l, &s_indices);
        let config = DistributedLanczosConfig::default();
        let result = distributed_lanczos(&a_s, &a_t, &b, n / 2, &config);
        let rel_err = (result.lambda_1 - direct).abs() / direct;
        assert!(
            rel_err < 1e-4,
            "C_20: expected λ_1 = {}, got {} (rel err {:.2e})",
            direct, result.lambda_1, rel_err
        );
    }

    #[test]
    fn lanczos_recovers_lambda_1_on_expander_k_5_5() {
        // K_{5,5}: complete bipartite, lambda_1 = 5 (well-known).
        // This is the case T5's naive bound FAILS on (expander gap).
        let l = complete_bipartite_laplacian(5, 5);
        let direct = direct_lambda_1(&l);
        // Sanity
        assert!((direct - 5.0).abs() < 1e-9, "K_5,5 λ_1 should be 5.0");

        // Partition by side (S = first 5, T = second 5)
        let s_indices: Vec<usize> = (0..5).collect();
        let (a_s, a_t, b) = split(&l, &s_indices);
        let config = DistributedLanczosConfig::default();
        let result = distributed_lanczos(&a_s, &a_t, &b, 5, &config);
        let rel_err = (result.lambda_1 - direct).abs() / direct;
        assert!(
            rel_err < 1e-6,
            "K_5,5: expected λ_1 = {}, got {} (rel err {:.2e}), K used = {}",
            direct,
            result.lambda_1,
            rel_err,
            result.iterations_used
        );
    }

    #[test]
    fn lanczos_terminates_within_k_max() {
        let l = path_laplacian(10);
        let s_indices: Vec<usize> = (0..5).collect();
        let (a_s, a_t, b) = split(&l, &s_indices);
        let config = DistributedLanczosConfig {
            k_max: 50,
            ..Default::default()
        };
        let result = distributed_lanczos(&a_s, &a_t, &b, 5, &config);
        assert!(result.iterations_used <= 50);
    }

    #[test]
    fn lanczos_is_deterministic_for_fixed_seed() {
        let l = path_laplacian(15);
        let s_indices: Vec<usize> = (0..7).collect();
        let (a_s, a_t, b) = split(&l, &s_indices);
        let config = DistributedLanczosConfig {
            seed: 42,
            ..Default::default()
        };
        let r1 = distributed_lanczos(&a_s, &a_t, &b, 7, &config);
        let r2 = distributed_lanczos(&a_s, &a_t, &b, 7, &config);
        assert_eq!(r1.iterations_used, r2.iterations_used);
        assert!((r1.lambda_1 - r2.lambda_1).abs() < 1e-15);
    }
}
