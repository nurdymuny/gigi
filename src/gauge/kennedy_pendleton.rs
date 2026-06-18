//! `kennedy_pendleton` — single-edge SU(2) heatbath kernel.
//!
//! Closes TDD-HAL-III.4. Implements the Kennedy-Pendleton 1985
//! rejection sampler for the marginal `ρ(x0) ∝ sqrt(1 - x0²) · exp(ξ · x0)`
//! plus the full SU(2) single-link update that conditions the gauge
//! field on its effective staple `V_eff`. Mirrors Halcyon Python
//! `inertia_damping/buckyball_heatbath.py` lines 95–171 byte-for-byte
//! at the algorithm level (constants `EPS_K = 1e-12`,
//! `MAX_KP_ITERS = 400`; sequential `for e in 0..n_edges` order is
//! enforced upstream at gate III.5).
//!
//! Group-erasure: this kernel is SU(2)-specific by construction (KP
//! IS the SU(2) heatbath algorithm — `Re Tr` of a unit quaternion in
//! the Wilson action collapses to `2·q0`, which is what the
//! exponential is conditioning on). It takes raw `[f64; 4]`
//! quaternions, NOT `GroupElement`. The group dispatch happens in
//! `GIBBS_SAMPLE` at gate III.5; a future SU(3) Cabibbo-Marinari
//! heatbath would land as a sibling module (sub-block pseudo-SU(2)
//! updates), not as a generic arm here.
//!
//! Fallback path: when `|V_eff| < EPS_K` the conditional distribution
//! is flat (no staple → uniform Haar), so we fall through to
//! `heatbath_haar::sample_haar_sqrt_rejection`. That is the SECOND
//! Haar sampler shipped this gate (locked decision D2) — the
//! Marsaglia 4-uniforms-with-rejection sampler in
//! `marsaglia_haar::haar_random_su2` stays as the INIT HAAR_RANDOM
//! path, and the two coexist on purpose.
//!
//! RNG: `SmallRng` from `marsaglia_haar` — the canonical xorshift64*
//! CSPRNG path Part II locked in for the whole gauge stack.
//!
//! Reference: Halcyon Python `buckyball_heatbath.py` lines 95–171
//! (`_sample_kp_x0` + `_sample_su2_link`).

use super::heatbath_haar::sample_haar_sqrt_rejection;
use super::marsaglia_haar::SmallRng;

/// `EPS_K = 1e-12` — staple-norm cutoff below which the KP kernel
/// falls back to Haar (`|V_eff| ≈ 0` ⇒ flat conditional). Matches
/// Halcyon Python `buckyball_heatbath.py::EPS_K` byte-for-byte.
pub const EPS_K: f64 = 1e-12;

/// `MAX_KP_ITERS = 400` — rejection-loop budget per KP draw. Matches
/// Halcyon Python `buckyball_heatbath.py::MAX_KP_ITERS` byte-for-byte.
/// For any realistic `ξ > 1e-6` the rejection acceptance probability
/// is high enough that 400 attempts effectively never exhausts — when
/// it does, return `None` so the caller can decide (the public
/// `sample_su2_link` falls through to Haar in that pathological case).
pub const MAX_KP_ITERS: usize = 400;

/// Sample `x0 ∈ [-1, 1]` from `ρ(x0) ∝ sqrt(1 - x0²) · exp(ξ · x0)`
/// using the Kennedy-Pendleton 1985 rejection sampler. `ξ := β · |V_eff|`
/// is the SU(2) effective coupling strength.
///
/// Each attempt consumes exactly 4 `uniform()` draws (`r1, r2, r3, r4`)
/// from `rng`. Returns `Some(x0)` on first acceptance, `None` if the
/// `MAX_KP_ITERS` budget exhausts without acceptance (essentially
/// never for `ξ > 1e-6`; the upstream `sample_su2_link` treats `None`
/// as the "fall through to Haar" signal).
///
/// Numerical floors: `r1` and `r3` are clamped at `1e-300` before the
/// `ln` to avoid `-inf` on the unlikely `uniform() == 0` case. Mirrors
/// Halcyon Python `_sample_kp_x0` byte-for-byte.
///
/// Reference: Halcyon Python `buckyball_heatbath.py::_sample_kp_x0`
/// (lines 95–116). Kennedy-Pendleton, Phys. Lett. B 156:393 (1985).
pub fn sample_kp_x0(xi: f64, rng: &mut SmallRng) -> Option<f64> {
    let xi_safe = xi.max(EPS_K);
    for _ in 0..MAX_KP_ITERS {
        let r1 = rng.uniform().max(1e-300);
        let r2 = rng.uniform();
        let r3 = rng.uniform().max(1e-300);
        let r4 = rng.uniform();
        let c = (2.0 * std::f64::consts::PI * r2).cos().powi(2);
        let delta = (-r1.ln() * c - r3.ln()) / xi_safe;
        if delta < 2.0 && (r4 * r4) <= (1.0 - 0.5 * delta) {
            return Some(1.0 - delta);
        }
    }
    None
}

/// Full SU(2) single-edge heatbath update: sample `U_new` from
/// `P(U) ∝ exp(β · q0(qmul(U, V_eff)))` conditioned on the effective
/// staple `V_eff` (a quaternion).
///
/// Algorithm (mirrors `_sample_su2_link`):
///
/// 1. Compute `k = |V_eff|`. If `k < EPS_K` the conditional is flat
///    → return `sample_haar_sqrt_rejection(rng)` (uniform Haar).
/// 2. Otherwise set `ξ = β · k`, draw `y0 = sample_kp_x0(ξ, rng)`.
/// 3. Place the remaining `(y1, y2, y3)` uniformly on the 2-sphere of
///    radius `r = sqrt(1 - y0²)` (latitude `cth` uniform on `[-1, 1]`,
///    azimuth `phi` uniform on `[0, 2π)`).
/// 4. Rotate into the lab frame:
///    `U_new = qmul(Y, qconj(V_hat))`, `V_hat = V_eff / k`.
/// 5. Project back to the unit 3-sphere (cheap insurance against
///    FP64 drift over a long heatbath run).
///
/// Pathological case: if `sample_kp_x0` returns `None` (budget
/// exhausted; never seen in practice for `ξ > 1e-6`), fall through to
/// the same Haar path as case 1.
///
/// Reference: Halcyon Python
/// `buckyball_heatbath.py::_sample_su2_link` (lines 137–171).
pub fn sample_su2_link(v_eff: [f64; 4], beta: f64, rng: &mut SmallRng) -> [f64; 4] {
    let k = (v_eff[0] * v_eff[0]
        + v_eff[1] * v_eff[1]
        + v_eff[2] * v_eff[2]
        + v_eff[3] * v_eff[3])
        .sqrt();
    if k < EPS_K {
        // Step 1 (fallback): flat conditional, return uniform Haar.
        return sample_haar_sqrt_rejection(rng);
    }

    let xi = beta * k;
    let y0 = match sample_kp_x0(xi, rng) {
        Some(y0) => y0,
        // Pathological exhaust (effectively never): fall to Haar.
        None => return sample_haar_sqrt_rejection(rng),
    };

    // Step 3: 2-sphere placement of (y1, y2, y3).
    let r = (1.0 - y0 * y0).max(0.0).sqrt();
    let cth = 2.0 * rng.uniform() - 1.0;
    let sth = (1.0 - cth * cth).max(0.0).sqrt();
    let phi = 2.0 * std::f64::consts::PI * rng.uniform();
    let y = [y0, r * sth * phi.cos(), r * sth * phi.sin(), r * cth];

    // Step 4: rotate into the lab frame via U_new = qmul(Y, qconj(V_hat)).
    let v_hat = [v_eff[0] / k, v_eff[1] / k, v_eff[2] / k, v_eff[3] / k];
    let v_hat_conj = [v_hat[0], -v_hat[1], -v_hat[2], -v_hat[3]];
    let u_new = qmul(y, v_hat_conj);

    // Step 5: re-normalize against FP64 drift.
    let nrm = (u_new[0] * u_new[0]
        + u_new[1] * u_new[1]
        + u_new[2] * u_new[2]
        + u_new[3] * u_new[3])
        .sqrt()
        .max(1e-15);
    [u_new[0] / nrm, u_new[1] / nrm, u_new[2] / nrm, u_new[3] / nrm]
}

/// Scalar-first quaternion product:
///   `c0 = a0·b0 − a·b`
///   `c_vec = a0·b_vec + b0·a_vec + a × b`
///
/// Matches the convention pinned in `src/gauge/mod.rs` (scalar-first,
/// `A = q0·I + i·(q1·σ_x + q2·σ_y + q3·σ_z)`). Inlined here so
/// `sample_su2_link` does not need to reach into the lattice walker
/// — the kernel speaks raw `[f64; 4]` arrays at the boundary.
#[inline]
fn qmul(a: [f64; 4], b: [f64; 4]) -> [f64; 4] {
    let (a0, a1, a2, a3) = (a[0], a[1], a[2], a[3]);
    let (b0, b1, b2, b3) = (b[0], b[1], b[2], b[3]);
    [
        a0 * b0 - a1 * b1 - a2 * b2 - a3 * b3,
        a0 * b1 + b0 * a1 + a2 * b3 - a3 * b2,
        a0 * b2 + b0 * a2 + a3 * b1 - a1 * b3,
        a0 * b3 + b0 * a3 + a1 * b2 - a2 * b1,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TDD-HAL-III.4: when `|V_eff| < EPS_K`, `sample_su2_link` falls
    /// through to the Haar branch and returns a unit quaternion.
    #[test]
    fn tdd_hal_iii_4_kp_xi_zero_falls_to_haar() {
        let mut rng = SmallRng::seed_from_u64(20260616);
        // V_eff with norm well below EPS_K (1e-12).
        let v_eff = [1e-15, 0.0, 0.0, 0.0];
        let q = sample_su2_link(v_eff, 2.5, &mut rng);
        let n2 = q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3];
        // FP64 epsilon · sqrt(4) ≈ 4.44e-16; budget 1e-12 for the
        // sqrt-rejection placement's intermediate rounding.
        assert!(
            (n2 - 1.0).abs() < 1e-12,
            "Haar fallback draw not unit-norm: |q|² = {n2}"
        );
    }

    /// TDD-HAL-III.4: for large `ξ = β · |V_eff|` (here 100) the KP
    /// kernel concentrates the new link near `V_hat^†` in the lab
    /// frame — i.e. q0 in the V_hat frame is sharply peaked near +1.
    /// Average q0 across 10000 samples should exceed 0.95.
    ///
    /// We pick `V_eff = (1, 0, 0, 0)` so `V_hat = (1, 0, 0, 0)`,
    /// `qconj(V_hat) = (1, 0, 0, 0)`, and `qmul(Y, V_hat^†) = Y`.
    /// Under that choice the lab-frame q0 IS the KP-sampled y0, so
    /// concentration of y0 near +1 shows directly.
    #[test]
    fn tdd_hal_iii_4_kp_xi_large_concentrates_near_v_hat() {
        let mut rng = SmallRng::seed_from_u64(20260617);
        let v_eff = [1.0, 0.0, 0.0, 0.0];
        let beta = 100.0; // ξ = 100 · 1 = 100.
        let n = 10000_usize;
        let mut sum_q0 = 0.0_f64;
        for _ in 0..n {
            let q = sample_su2_link(v_eff, beta, &mut rng);
            sum_q0 += q[0];
        }
        let mean_q0 = sum_q0 / n as f64;
        assert!(
            mean_q0 > 0.95,
            "KP at ξ=100 should concentrate ⟨q0⟩ > 0.95 near V_hat; got {mean_q0}"
        );
    }

    /// TDD-HAL-III.4: each `sample_kp_x0` attempt consumes exactly 4
    /// `uniform()` draws (r1, r2, r3, r4). Total draws after a call
    /// at ξ = 1.0 must be `4 · n_attempts` where `n_attempts` is the
    /// number of rejection rounds taken to acceptance.
    ///
    /// We run two parallel RNGs at the same seed: one drives
    /// `sample_kp_x0` and reports its draw count; the other manually
    /// replays uniforms until we observe the same acceptance, giving
    /// the ground-truth attempt count. The two must agree at
    /// `draws == 4 · attempts`.
    #[test]
    fn tdd_hal_iii_4_kp_consumes_4_rngs_per_attempt() {
        let xi = 1.0_f64;
        let mut rng_a = SmallRng::seed_from_u64(20260618);
        let draws_before = rng_a.draws();
        let result_a = sample_kp_x0(xi, &mut rng_a);
        let draws_consumed = rng_a.draws() - draws_before;
        assert!(result_a.is_some(), "ξ=1 should accept inside budget");

        // Replay manually with a second RNG at the same seed.
        let mut rng_b = SmallRng::seed_from_u64(20260618);
        let xi_safe = xi.max(EPS_K);
        let mut attempts = 0_u64;
        loop {
            attempts += 1;
            let r1 = rng_b.uniform().max(1e-300);
            let r2 = rng_b.uniform();
            let r3 = rng_b.uniform().max(1e-300);
            let r4 = rng_b.uniform();
            let c = (2.0 * std::f64::consts::PI * r2).cos().powi(2);
            let delta = (-r1.ln() * c - r3.ln()) / xi_safe;
            if delta < 2.0 && (r4 * r4) <= (1.0 - 0.5 * delta) {
                break;
            }
            assert!(attempts < MAX_KP_ITERS as u64);
        }
        assert_eq!(
            draws_consumed,
            4 * attempts,
            "sample_kp_x0 must consume exactly 4 uniforms per attempt; got {draws_consumed} consumed across {attempts} attempts"
        );
    }

    /// TDD-HAL-III.4: lock the KP constants to the Halcyon Python
    /// reference byte-for-byte (`EPS_K = 1e-12`, `MAX_KP_ITERS = 400`).
    /// Drifting either constant breaks intra-binding bit-identity
    /// against the mock and is a SHIP-BLOCKER per the sprint spec.
    #[test]
    fn tdd_hal_iii_4_kp_constants_match_halcyon() {
        assert_eq!(EPS_K, 1e-12, "EPS_K must match Halcyon Python verbatim");
        assert_eq!(
            MAX_KP_ITERS, 400,
            "MAX_KP_ITERS must match Halcyon Python verbatim"
        );
    }
}
