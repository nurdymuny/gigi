//! `marsaglia_haar` — uniform-on-S^3 sampler for SU(2).
//!
//! Closes TDD-HAL-II.2. The free function `haar_random_su2` draws a
//! single SU(2) element uniformly under Haar measure using the
//! Marsaglia 4-uniforms-with-rejection algorithm. The RNG is the
//! same xorshift64* algorithm `SAMPLE_TRANSPORT` uses (mirrored from
//! `src/geometry/generative_flow.rs::SmallRng`), in-lined here to
//! avoid pulling the `kahler`-gated `geometry` module into the
//! `gauge` feature graph (optionality contract per Bee's locked
//! decision 7). Same algorithm + same seed → same state evolution
//! → byte-identical buffer (Bee's locked decision 1).
//!
//! Algorithm (mirrors the Python mock at
//! `gigi_client/mock.py::_haar_random_links`):
//!
//! 1. Draw `(x1, x2)` uniformly in `[-1, 1]^2`, rejecting until
//!    `s1 = x1^2 + x2^2 < 1`.
//! 2. Draw `(x3, x4)` uniformly in `[-1, 1]^2`, rejecting until
//!    `s2 = x3^2 + x4^2 < 1`.
//! 3. Emit the quaternion `(x1, x2, x3 * factor, x4 * factor)` where
//!    `factor = sqrt((1 - s1) / s2)`.
//!
//! Per-edge draw order is `x1, x2, x3, x4` (the rejection-failing
//! draws also consume RNG state — that's the bit-identity invariant).
//! Edge iteration order is `0..n_edges` (see `DenseLinkBuffer::new_haar`).
//!
//! Group erasure: this sampler is SU(2)-specific by construction
//! (uniform-on-S^3 lives only on SU(2)). It lives as a free function
//! so future U(1) / SU(3) / Z(N) samplers can land beside it as new
//! free functions without touching the buffer or the walker.

/// xorshift64* PRNG — byte-for-byte the same algorithm as
/// `src/geometry/generative_flow.rs::SmallRng`. Mirrored here so the
/// `gauge` feature does not need to pull in `kahler`/`geometry`
/// (optionality contract per Bee's locked decision 7). Same seed →
/// same state evolution → byte-identical Haar buffer (Bee's locked
/// decision 1).
#[derive(Debug, Clone)]
pub struct SmallRng {
    state: u64,
    /// Observational counter — incremented once per `uniform()` call.
    /// Does NOT participate in state evolution (byte-identity is
    /// preserved). Exposed via `draws()` so the III.4 KP draw-count
    /// test can assert exactly `4 · n_attempts` uniforms consumed per
    /// `sample_kp_x0` call.
    draws: u64,
}

impl SmallRng {
    /// Seed the PRNG. Mirrors
    /// `geometry::generative_flow::SmallRng::seed_or_entropy(Some(seed))`
    /// for the all-deterministic path (the entropy fallback is not
    /// needed in the sampler).
    pub fn seed_from_u64(seed: u64) -> Self {
        Self {
            state: seed.max(1),
            draws: 0,
        }
    }

    /// xorshift64* core. Identical to the canonical
    /// `geometry::generative_flow::SmallRng::next_u64`.
    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform in `[0, 1)`. Same conversion as the canonical impl.
    pub fn uniform(&mut self) -> f64 {
        self.draws = self.draws.wrapping_add(1);
        (self.next_u64() >> 11) as f64 / ((1u64 << 53) as f64)
    }

    /// Total `uniform()` calls observed since construction. Reset is
    /// not provided — tests should clone or capture a baseline and
    /// subtract.
    pub fn draws(&self) -> u64 {
        self.draws
    }
}

/// Draw one uniform-on-S^3 quaternion from `rng` using Marsaglia's
/// 4-uniforms-with-rejection algorithm.
///
/// Returns `[q0, q1, q2, q3] = [x1, x2, x3 * factor, x4 * factor]`
/// with `q0^2 + q1^2 + q2^2 + q3^2 = 1` (up to f64 rounding — the
/// `tdd_hal_ii_2_haar_unit_norm` test asserts within 1e-12).
///
/// The function consumes RNG state for every draw including rejected
/// ones; reproducibility across runs (same seed → same output) is
/// the bit-identity contract that the Part-II gold gate pins.
pub fn haar_random_su2(rng: &mut SmallRng) -> [f64; 4] {
    // Reject (x1, x2) until s1 < 1. Each rejected pair still
    // consumes two uniform draws — that's required for bit-identity.
    let (x1, x2, s1) = loop {
        let x1 = 2.0 * rng.uniform() - 1.0;
        let x2 = 2.0 * rng.uniform() - 1.0;
        let s1 = x1 * x1 + x2 * x2;
        if s1 < 1.0 {
            break (x1, x2, s1);
        }
    };

    // Reject (x3, x4) until s2 < 1, same RNG-consumption rule.
    let (x3, x4, s2) = loop {
        let x3 = 2.0 * rng.uniform() - 1.0;
        let x4 = 2.0 * rng.uniform() - 1.0;
        let s2 = x3 * x3 + x4 * x4;
        if s2 < 1.0 {
            break (x3, x4, s2);
        }
    };

    let factor: f64 = ((1.0 - s1) / s2).sqrt();
    [x1, x2, x3 * factor, x4 * factor]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TDD-HAL-II.2: two `SmallRng`s seeded identically produce
    /// byte-identical Haar quaternion streams. This is the
    /// intra-binding bit-identity contract (Bee's locked decision 1).
    #[test]
    fn tdd_hal_ii_2_haar_same_seed_byte_equal() {
        let mut a = SmallRng::seed_from_u64(20260616);
        let mut b = SmallRng::seed_from_u64(20260616);
        for _ in 0..100 {
            let qa = haar_random_su2(&mut a);
            let qb = haar_random_su2(&mut b);
            assert_eq!(qa, qb, "Haar draws must be byte-identical under same seed");
        }
    }

    /// TDD-HAL-II.2: every Haar draw lands on S^3 (unit norm within
    /// 1e-12 — f64 rounding only; the algorithm is exact in real
    /// arithmetic).
    #[test]
    fn tdd_hal_ii_2_haar_unit_norm() {
        let mut rng = SmallRng::seed_from_u64(20260616);
        for _ in 0..1000 {
            let q = haar_random_su2(&mut rng);
            let n2 = q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3];
            assert!(
                (n2 - 1.0).abs() < 1e-12,
                "Haar draw not unit-norm: q = {:?}, |q|^2 = {}",
                q,
                n2
            );
        }
    }

    /// TDD-HAL-II.2: the marginal of `q0` over Haar measure has mean
    /// 0 (symmetry) and mean(q0^2) = 1/4 (uniform-on-S^3
    /// component variance). Across 20 seeds × 90 edges = 1800 samples
    /// we budget |mean(q0)| < 0.08 and |mean(q0^2) - 0.25| < 0.05.
    /// Loose bounds because the test is a "did you implement the
    /// right distribution" guard, not a precision PRNG audit.
    #[test]
    fn tdd_hal_ii_2_haar_marginal_stats() {
        let mut q0_sum = 0.0_f64;
        let mut q0_sq_sum = 0.0_f64;
        let mut n = 0_usize;
        for seed in 0..20_u64 {
            let mut rng = SmallRng::seed_from_u64(seed + 1);
            for _ in 0..90 {
                let q = haar_random_su2(&mut rng);
                q0_sum += q[0];
                q0_sq_sum += q[0] * q[0];
                n += 1;
            }
        }
        let mean = q0_sum / n as f64;
        let mean_sq = q0_sq_sum / n as f64;
        assert!(
            mean.abs() < 0.08,
            "|mean(q0)| = {} >= 0.08 (Haar symmetry violated?)",
            mean.abs()
        );
        assert!(
            (mean_sq - 0.25).abs() < 0.05,
            "|mean(q0^2) - 0.25| = {} >= 0.05 (Haar variance violated?)",
            (mean_sq - 0.25).abs()
        );
    }
}
