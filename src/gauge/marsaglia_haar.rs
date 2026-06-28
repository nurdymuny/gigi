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

/// Local complex-f64 micro-struct for the Haar SU(3) sampler.
///
/// Kept private so the SU(3) ingest path does not pull in `num_complex`
/// or `nalgebra::Complex<f64>` (optionality contract per Bee's locked
/// decision 7 — the `gauge` feature must not pull in extra crates for
/// a hot-loop sampler). Stack-allocated, `Copy`, mul/add/sub/conj
/// inline — same allocation discipline as `haar_random_su2`'s 4-tuple.
#[derive(Debug, Clone, Copy)]
struct C64 {
    re: f64,
    im: f64,
}

impl C64 {
    #[inline]
    fn zero() -> Self {
        Self { re: 0.0, im: 0.0 }
    }
    #[inline]
    fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }
    #[inline]
    fn add(self, other: Self) -> Self {
        Self {
            re: self.re + other.re,
            im: self.im + other.im,
        }
    }
    #[inline]
    fn sub(self, other: Self) -> Self {
        Self {
            re: self.re - other.re,
            im: self.im - other.im,
        }
    }
    #[inline]
    fn mul(self, other: Self) -> Self {
        Self {
            re: self.re * other.re - self.im * other.im,
            im: self.re * other.im + self.im * other.re,
        }
    }
    #[inline]
    fn conj(self) -> Self {
        Self {
            re: self.re,
            im: -self.im,
        }
    }
    /// `|z|² = re² + im²`.
    #[inline]
    fn norm_sq(self) -> f64 {
        self.re * self.re + self.im * self.im
    }
    /// `|z| = sqrt(re² + im²)`.
    #[inline]
    fn abs(self) -> f64 {
        self.norm_sq().sqrt()
    }
    #[inline]
    fn scale(self, s: f64) -> Self {
        Self {
            re: self.re * s,
            im: self.im * s,
        }
    }
}

/// Box–Muller pair: returns two independent standard normals from two
/// uniforms. Mirrors the pattern `maxwell_boltzmann_su2` uses
/// (clamp `u1` away from zero to avoid `ln(0)`).
#[inline]
fn box_muller_pair(rng: &mut SmallRng) -> (f64, f64) {
    let u1 = rng.uniform().max(f64::MIN_POSITIVE);
    let u2 = rng.uniform();
    let r = (-2.0 * u1.ln()).sqrt();
    let theta = 2.0 * std::f64::consts::PI * u2;
    (r * theta.cos(), r * theta.sin())
}

/// Draw one Haar-uniform SU(3) element using the Mezzadri 2007
/// algorithm: complex Ginibre matrix → modified Gram–Schmidt QR →
/// diagonal phase normalization → projection onto SU(3) by rotating
/// column 0 by `conj(det(Q))`.
///
/// Returns the 3×3 unitary matrix as 18 f64s in the row-major
/// interleaved-pairs layout pinned by `GroupElement::SU3`:
/// `[re_00, im_00, re_01, im_01, re_02, im_02,
///   re_10, im_10, re_11, im_11, re_12, im_12,
///   re_20, im_20, re_21, im_21, re_22, im_22]`.
///
/// **RNG cadence**: 18 uniforms per call (9 Box–Muller pairs = 18
/// uniforms; no rejection sampling). This is **fixed** — unlike
/// `haar_random_su2` which can consume a variable number of uniforms
/// due to (x1,x2) and (x3,x4) rejection loops, the Mezzadri SU(3)
/// algorithm is rejection-free. The per-edge advance count is
/// constant, which simplifies the bit-identity contract for Phase 2
/// heatbath integrations.
///
/// **Det normalization** (step 4): the projection `U(3) → SU(3)` is
/// done by multiplying column 0 by `conj(det(Q))`, NOT by dividing
/// every entry by `det(Q)^(1/3)`. Cube roots have three branches and
/// the branch choice is not bit-identical across compilers; the
/// single-column phase rotation is branch-free and preserves Haar
/// measure on SU(3) (Mezzadri 2007 §6, Diaconis–Forrester 2017 §2.1).
pub fn haar_random_su3(rng: &mut SmallRng) -> [f64; 18] {
    // STEP 1: 3×3 complex Ginibre matrix. 9 complex Gaussians = 18
    // standard normals = 9 Box–Muller pairs.
    let mut z: [[C64; 3]; 3] = [[C64::zero(); 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            let (re, im) = box_muller_pair(rng);
            z[i][j] = C64::new(re, im);
        }
    }

    // STEP 2: Modified Gram–Schmidt QR on the columns of Z.
    // After this, Q[*][k] is the k-th unitary basis column, and the
    // diagonal of R is real-positive by construction.
    let mut q: [[C64; 3]; 3] = [[C64::zero(); 3]; 3];
    for k in 0..3 {
        // v = Z[:, k]
        let mut v: [C64; 3] = [z[0][k], z[1][k], z[2][k]];
        // Subtract projections on prior Q columns.
        for j in 0..k {
            // r_jk = <Q[:, j], v> = Σ_i conj(Q[i, j]) · v[i]
            let mut r_jk = C64::zero();
            for i in 0..3 {
                r_jk = r_jk.add(q[i][j].conj().mul(v[i]));
            }
            // v -= r_jk · Q[:, j]
            for i in 0..3 {
                v[i] = v[i].sub(r_jk.mul(q[i][j]));
            }
        }
        // r_kk = sqrt(<v, v>.re) — real-positive by construction.
        let vnorm_sq = v[0].norm_sq() + v[1].norm_sq() + v[2].norm_sq();
        let r_kk = vnorm_sq.sqrt();
        // Q[:, k] = v / r_kk
        let inv = 1.0 / r_kk;
        for i in 0..3 {
            q[i][k] = v[i].scale(inv);
        }
    }

    // STEP 3: Mezzadri phase normalization. Modified Gram–Schmidt above
    // already yields R[k][k] real-positive, so Λ = I and this step is
    // a no-op. Kept as a comment-anchor so the algorithm reads as
    // canonical Mezzadri 2007 and a future swap-in (e.g. nalgebra QR
    // with arbitrary diagonal phases) is correct by construction.

    // STEP 4: Project U(3) → SU(3) by det(Q) ∈ U(1) on column 0.
    // det via Laplace expansion on row 0.
    let m00 = q[1][1].mul(q[2][2]).sub(q[1][2].mul(q[2][1]));
    let m01 = q[1][0].mul(q[2][2]).sub(q[1][2].mul(q[2][0]));
    let m02 = q[1][0].mul(q[2][1]).sub(q[1][1].mul(q[2][0]));
    let det = q[0][0]
        .mul(m00)
        .sub(q[0][1].mul(m01))
        .add(q[0][2].mul(m02));
    // |det(Q)| = 1 in exact arithmetic since Q is unitary; defensively
    // normalize so any sub-ULP drift in |det| does not propagate.
    let det_abs = det.abs().max(f64::MIN_POSITIVE);
    let phase = det.scale(1.0 / det_abs);
    let phase_conj = phase.conj();
    // Multiply column 0 by conj(det(Q)) so det(Q_new) = 1.
    for i in 0..3 {
        q[i][0] = q[i][0].mul(phase_conj);
    }

    // STEP 5: Pack into [f64; 18] interleaved row-major.
    let mut out = [0.0_f64; 18];
    for i in 0..3 {
        for j in 0..3 {
            out[6 * i + 2 * j] = q[i][j].re;
            out[6 * i + 2 * j + 1] = q[i][j].im;
        }
    }
    out
}

/// Draw one Maxwell–Boltzmann SU(2) tangent vector from `rng` packed
/// as a quaternion with `q0 = 0`.
///
/// E ∈ su(2) lives in the imaginary-quaternion tangent direction, so
/// the canonical packing is `(0, g_1·σ, g_2·σ, g_3·σ)` where the `g_k`
/// are independent standard normals and `σ = sqrt(1 / (β · dim/2))`.
/// For SU(2) `dim = 3`, so `dim/2 = 1.5` and `σ = sqrt(1 / (β · 1.5))`
/// — the Halcyon canonical_sigma packing (mirrored from the Python
/// reference at `davis-wilson-lattice/inertia_damping/`).
///
/// Standard normals are produced via Box–Muller from two uniforms per
/// pair: `g = sqrt(-2·ln u1) · cos(2π·u2)` and the same with `sin`
/// for the second. We draw two pairs per call (four uniforms) and
/// use the first three results — the fourth is consumed to keep the
/// per-edge RNG-state advance fixed (bit-identity invariant: every
/// edge consumes the same number of uniforms regardless of which g_k
/// we keep).
pub fn maxwell_boltzmann_su2(rng: &mut SmallRng, beta: f64) -> [f64; 4] {
    let sigma = (1.0 / (beta * 1.5)).sqrt();
    // Pair 1: yields g1, g2.
    let u1 = rng.uniform().max(f64::MIN_POSITIVE);
    let u2 = rng.uniform();
    let r1 = (-2.0 * u1.ln()).sqrt();
    let theta1 = 2.0 * std::f64::consts::PI * u2;
    let g1 = r1 * theta1.cos();
    let g2 = r1 * theta1.sin();
    // Pair 2: yields g3; the second standard normal is consumed but
    // discarded — fixed RNG-state advance per edge so different β
    // values still share the same draw cadence and the bit-identity
    // contract (A2 row 1) holds.
    let u3 = rng.uniform().max(f64::MIN_POSITIVE);
    let u4 = rng.uniform();
    let r2 = (-2.0 * u3.ln()).sqrt();
    let theta2 = 2.0 * std::f64::consts::PI * u4;
    let g3 = r2 * theta2.cos();
    let _g4_discarded = r2 * theta2.sin();

    [0.0, sigma * g1, sigma * g2, sigma * g3]
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

    // ───────────────────── Haar SU(3) tests ─────────────────────

    /// Halcyon ITEM 3.1: two `SmallRng`s seeded identically produce
    /// byte-identical Haar SU(3) streams (intra-binding bit-identity
    /// — same contract as `tdd_hal_ii_2_haar_same_seed_byte_equal`).
    #[test]
    fn haar_su3_same_seed_byte_equal() {
        let mut a = SmallRng::seed_from_u64(20260626);
        let mut b = SmallRng::seed_from_u64(20260626);
        for _ in 0..50 {
            let ma = haar_random_su3(&mut a);
            let mb = haar_random_su3(&mut b);
            assert_eq!(ma, mb, "Haar SU(3) draws must be byte-identical under same seed");
        }
    }

    /// Halcyon ITEM 3.1: every Haar SU(3) draw is unitary
    /// (`U U† = I` to FP64 tolerance ~1e-12).
    #[test]
    fn haar_su3_unitary() {
        let mut rng = SmallRng::seed_from_u64(20260626);
        for sample in 0..100 {
            let m = haar_random_su3(&mut rng);
            // Compute U · U† using the same SU(3) primitives the engine
            // dispatches on (compose + inverse) to keep this test
            // honest about the production code path.
            let u = crate::gauge::group_element::GroupElement::SU3(m);
            let u_dag = u.inverse();
            let prod = u.compose(&u_dag);
            let id = crate::gauge::group_element::GroupElement::su3_identity();
            match (prod, id) {
                (
                    crate::gauge::group_element::GroupElement::SU3(p),
                    crate::gauge::group_element::GroupElement::SU3(i),
                ) => {
                    for k in 0..18 {
                        assert!(
                            (p[k] - i[k]).abs() < 1e-12,
                            "sample {sample}, index {k}: |U U† − I| = {} >= 1e-12",
                            (p[k] - i[k]).abs()
                        );
                    }
                }
                _ => panic!("expected SU3 variants"),
            }
        }
    }

    /// Halcyon ITEM 3.1: every Haar SU(3) draw has `|det(U) − 1| < 1e-10`.
    /// Det normalization via the Mezzadri column-0 rotation must land
    /// in this tolerance; cube-root would not (branch ambiguity).
    #[test]
    fn haar_su3_special_unitary() {
        let mut rng = SmallRng::seed_from_u64(20260626);
        for sample in 0..100 {
            let m = haar_random_su3(&mut rng);
            // Inline det via Laplace expansion on row 0 (same shape as
            // the projector inside haar_random_su3).
            // Row 0 entries:
            let a00 = (m[0], m[1]);
            let a01 = (m[2], m[3]);
            let a02 = (m[4], m[5]);
            // Row 1:
            let a10 = (m[6], m[7]);
            let a11 = (m[8], m[9]);
            let a12 = (m[10], m[11]);
            // Row 2:
            let a20 = (m[12], m[13]);
            let a21 = (m[14], m[15]);
            let a22 = (m[16], m[17]);
            // Complex multiply helpers (inline).
            let cmul = |x: (f64, f64), y: (f64, f64)| -> (f64, f64) {
                (x.0 * y.0 - x.1 * y.1, x.0 * y.1 + x.1 * y.0)
            };
            let csub = |x: (f64, f64), y: (f64, f64)| -> (f64, f64) {
                (x.0 - y.0, x.1 - y.1)
            };
            let cadd = |x: (f64, f64), y: (f64, f64)| -> (f64, f64) {
                (x.0 + y.0, x.1 + y.1)
            };
            let m00 = csub(cmul(a11, a22), cmul(a12, a21));
            let m01 = csub(cmul(a10, a22), cmul(a12, a20));
            let m02 = csub(cmul(a10, a21), cmul(a11, a20));
            let det = cadd(csub(cmul(a00, m00), cmul(a01, m01)), cmul(a02, m02));
            let drift = ((det.0 - 1.0).powi(2) + det.1.powi(2)).sqrt();
            assert!(
                drift < 1e-10,
                "sample {sample}: |det(U) - 1| = {drift} >= 1e-10 (det = {:?})",
                det
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
