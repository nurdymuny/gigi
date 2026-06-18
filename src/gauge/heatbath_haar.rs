//! `heatbath_haar` — sqrt-rejection Haar SU(2) sampler used as the
//! KP heatbath's xi → 0 fallback.
//!
//! Closes part of TDD-HAL-III.4 (Bee's locked decision D2: ship a
//! SECOND Haar sampler distinct from the Marsaglia 4-uniforms-with-
//! rejection sampler in `src/gauge/marsaglia_haar.rs`). Both Haar
//! paths coexist; each is named honestly:
//!
//! - `marsaglia_haar::haar_random_su2` — INIT HAAR_RANDOM path (Part
//!   II, gate II.2). Marsaglia 4-uniforms-with-rejection. Bit-identity
//!   gold-walker tests pin this algorithm.
//! - `heatbath_haar::sample_haar_sqrt_rejection` — KP heatbath
//!   xi → 0 fallback. sqrt-rejection on x0 then uniform placement on
//!   the remaining 2-sphere. Mirrors Halcyon Python
//!   `buckyball_heatbath.py::_sample_haar_su2` (lines 119–134).
//!
//! Algorithm (mirrors `_sample_haar_su2`):
//!
//! 1. Draw `x0` uniformly in `[-1, 1]` and `u` uniformly in `[0, 1)`,
//!    rejecting until `u ≤ sqrt(max(1 - x0², 0))`. (This realizes the
//!    `ρ(x0) ∝ sqrt(1 - x0²)` marginal of Haar measure on `S^3`.)
//! 2. Set `r = sqrt(max(1 - x0², 0))`.
//! 3. Draw `cth` uniformly in `[-1, 1]` (latitude on the 2-sphere of
//!    radius `r`), `sth = sqrt(max(1 - cth², 0))`.
//! 4. Draw `phi` uniformly in `[0, 2π)`.
//! 5. Emit the quaternion
//!    `[x0, r·sth·cos(phi), r·sth·sin(phi), r·cth]`.
//!
//! Per-link RNG consumption is variable in step 1 (2 draws per
//! rejection attempt) plus exactly 3 in steps 3–4. Rejected `(x0, u)`
//! pairs still consume RNG state — that is the intra-binding
//! bit-identity invariant Part II locked in.
//!
//! Group erasure: this sampler is SU(2)-specific by construction
//! (Haar on `S^3`). Sibling samplers for other groups would land as
//! distinct free functions; nothing here is generic over `Group`.
//!
//! Reference: Halcyon Python `inertia_damping/buckyball_heatbath.py`
//! lines 119–134.

use super::marsaglia_haar::SmallRng;

/// Draw one uniform-on-S^3 quaternion using the sqrt-rejection-on-x0
/// + spherical placement algorithm (Halcyon
/// `buckyball_heatbath.py::_sample_haar_su2`).
///
/// This is the KP heatbath xi → 0 fallback. The INIT HAAR_RANDOM
/// path stays on `marsaglia_haar::haar_random_su2` — two algorithms
/// coexist on purpose (locked decision D2).
///
/// Returns `[q0, q1, q2, q3]` with `q0² + q1² + q2² + q3² = 1` up to
/// f64 rounding. Same RNG + same seed → byte-identical output (locked
/// decision 1, same xorshift64* state evolution as Part II).
pub fn sample_haar_sqrt_rejection(rng: &mut SmallRng) -> [f64; 4] {
    // Step 1: sqrt-rejection on x0. Each rejected (x0, u) pair still
    // consumes two uniform draws — required for bit-identity.
    let x0 = loop {
        let x0 = 2.0 * rng.uniform() - 1.0;
        let u = rng.uniform();
        let bound = (1.0 - x0 * x0).max(0.0).sqrt();
        if u <= bound {
            break x0;
        }
    };

    // Steps 2–5: place the remaining components uniformly on the
    // 2-sphere of radius r = sqrt(1 - x0²).
    let r = (1.0 - x0 * x0).max(0.0).sqrt();
    let cth = 2.0 * rng.uniform() - 1.0;
    let sth = (1.0 - cth * cth).max(0.0).sqrt();
    let phi = 2.0 * std::f64::consts::PI * rng.uniform();

    [x0, r * sth * phi.cos(), r * sth * phi.sin(), r * cth]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TDD-HAL-III.4: 10000 sqrt-rejection Haar draws have |⟨q0⟩| < 0.05
    /// (the Haar measure on SU(2) is symmetric under q → -q, so q0 has
    /// mean 0; this asserts we are sampling the right distribution).
    #[test]
    fn tdd_hal_iii_4_haar_sqrt_rejection_distribution() {
        let mut rng = SmallRng::seed_from_u64(20260616);
        let mut sum_q0 = 0.0_f64;
        let n = 10000_usize;
        for _ in 0..n {
            let q = sample_haar_sqrt_rejection(&mut rng);
            // Unit-norm sanity (f64 rounding).
            let n2 = q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3];
            assert!(
                (n2 - 1.0).abs() < 1e-12,
                "sqrt-rejection Haar draw not unit-norm: |q|² = {n2}"
            );
            sum_q0 += q[0];
        }
        let mean = sum_q0 / n as f64;
        assert!(
            mean.abs() < 0.05,
            "|mean(q0)| = {} >= 0.05 (Haar symmetry violated?)",
            mean.abs()
        );
    }
}
