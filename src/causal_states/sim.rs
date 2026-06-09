//! Causal States — observation-sequence simulator.
//!
//! Companion to `mod.rs`. Generates trajectories from the paper's noisy
//! 2-state HMM. The `commutator()` orchestrator computes Ω from operator
//! formulas; this module lets us check whether Ω can also be **estimated**
//! from finite samples of the process's observation sequence, with the
//! expected ~1/√N convergence rate.
//!
//! Deterministic by construction: a fixed-seed LCG drives sampling, so
//! every Phase 4.2 experiment is byte-identical across machines and
//! across runs.

/// Deterministic 64-bit linear-congruential PRNG (Numerical Recipes constants).
///
/// Used by Phase 4.2 / 4.3 sampling experiments. Deterministic-by-construction
/// so the empirical-convergence study and the orthogonality scan reproduce
/// bit-identically — running them with the same seed gives the same CSV.
#[derive(Debug, Clone)]
pub struct Lcg(u64);

impl Lcg {
    /// New PRNG initialised at `seed`.
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// Advance state, return raw 64-bit value.
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    /// Uniform `f64` in `[0, 1)`.
    pub fn next_f64(&mut self) -> f64 {
        let u = self.next_u64();
        ((u >> 11) as f64) / ((1u64 << 53) as f64)
    }
}

/// One simulated observation pair + latent trajectory from a noisy 2-state HMM.
///
/// The empirical-commutator estimator (Phase 4.2) targets the **predictive
/// prior** `p(s_3 | x_1, x_2)` — that's what `U_b(U_a(μ))` computes (see
/// paper §6 and the worked check in mod.rs docs). Hence the simulator
/// generates three hidden states plus two observations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SamplePair {
    /// Hidden state at time 1 (sampled from the prior `mu`).
    pub s_1: u8,
    /// Hidden state at time 2 (after one Markov transition from `s_1`).
    pub s_2: u8,
    /// Hidden state at time 3 (after the second Markov transition).
    /// This is the variable the Bayesian operator's output describes.
    pub s_3: u8,
    /// Emission at time 1 (from `s_1`).
    pub x_1: u8,
    /// Emission at time 2 (from `s_2`).
    pub x_2: u8,
}

/// Simulate one observation pair `(X_1, X_2)` plus latent trajectory
/// `(s_1, s_2, s_3)` from the noisy 2-state HMM with prior `mu` over
/// `s_1`, transition probability `alpha`, and emission confusion `beta`.
///
/// `U_b(U_a(μ))` in the paper is the predictive prior on the **next**
/// hidden state given two observations, so this simulator generates
/// `s_3` as well.
///
/// # Panics
///
/// Panics if `mu.len() != 2` or `mu` is not a 2-vector probability
/// distribution (sums to 1, entries non-negative).
pub fn simulate_pair(
    mu: &[f64],
    alpha: f64,
    beta: f64,
    rng: &mut Lcg,
) -> SamplePair {
    assert_eq!(mu.len(), 2, "mu must be 2-state belief");
    let s_1 = sample_state(mu, rng);
    let s_2 = sample_state(&transition_row(s_1, alpha), rng);
    let s_3 = sample_state(&transition_row(s_2, alpha), rng);
    let x_1 = emit(s_1, beta, rng);
    let x_2 = emit(s_2, beta, rng);
    SamplePair { s_1, s_2, s_3, x_1, x_2 }
}

/// Estimate `P(s_3 | x_1 = a, x_2 = b)` from `n_samples` simulated pairs.
///
/// This is the **empirical commutator arm** for observation word `ab`:
/// since `U_b(U_a(μ))` is the predictive prior on `s_3` given `(x_1, x_2)`,
/// counting the empirical distribution of `s_3` over samples that match
/// `(x_1 = a, x_2 = b)` gives the empirical estimator with ~1/√N rate.
///
/// Returns `(distribution, n_matched)` where `n_matched` is the number of
/// samples whose observation pair equalled `(a, b)`. If `n_matched == 0`,
/// returns `(vec![0.5, 0.5], 0)` — caller should disregard.
pub fn empirical_belief_after_pair(
    mu: &[f64],
    alpha: f64,
    beta: f64,
    a: u8,
    b: u8,
    n_samples: u32,
    rng: &mut Lcg,
) -> (Vec<f64>, u32) {
    let mut counts = [0u32; 2];
    let mut matched = 0u32;
    for _ in 0..n_samples {
        let s = simulate_pair(mu, alpha, beta, rng);
        if s.x_1 == a && s.x_2 == b {
            matched += 1;
            counts[s.s_3 as usize] += 1;
        }
    }
    if matched == 0 {
        return (vec![0.5, 0.5], 0);
    }
    let total = f64::from(matched);
    (
        vec![f64::from(counts[0]) / total, f64::from(counts[1]) / total],
        matched,
    )
}

fn transition_row(from_state: u8, alpha: f64) -> [f64; 2] {
    if from_state == 0 {
        [1.0 - alpha, alpha]
    } else {
        [alpha, 1.0 - alpha]
    }
}

fn sample_state(probs: &[f64], rng: &mut Lcg) -> u8 {
    let u = rng.next_f64();
    if u < probs[0] {
        0
    } else {
        1
    }
}

fn emit(state: u8, beta: f64, rng: &mut Lcg) -> u8 {
    // P(x = state) = 1 - beta;  P(x ≠ state) = beta.
    let u = rng.next_f64();
    if u < 1.0 - beta {
        state
    } else {
        1 - state
    }
}
