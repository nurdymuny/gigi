//! GIGI Encrypt v0.4 — Sprint Q: K-preserving transformation
//! characterization (TDD).
//!
//! Documents which linear / affine maps preserve the Davis dispersion
//! K = Var/range² on a per-field basis, and separates that geometric
//! question from lattice-based hiding. **This is not a shipped PQ
//! mode** — it is a roadmap with validated mathematical findings.
//!
//! Spec: `theory/encryption/GIGI_ENCRYPT_v0.4_SPRINT_SPEC.md` §Sprint Q.
//!
//! Test map (Q-1 through Q-5 from spec):
//!   Q-1: general GL shear breaks per-field K
//!   Q-2: diagonal affine (ℝ*)^k ⋉ ℝ^k preserves per-field K
//!   Q-3: O(k) rotation preserves tr(Cov) but NOT (max-min)²
//!   Q-4: scalar aI is a special case of diagonal affine
//!   Q-5: LWE illustration — K(As+e) ≈ K(random) ≠ K(s)
//!
//! Construction key result:
//!   G_AffK = (ℝ*)^k ⋉ ℝ^k  (diagonal affine; the K-preserving group)
//!   For Euclidean ball membership (Sprint P scalar gauge), further
//!   restrict to scalar conformal aI + b.

use gigi::invariant_ring::compute_k;

// ───────────────────────────────────────────────────────────────────
// Helpers — deterministic PRNG + small matrix-on-vector ops
// ───────────────────────────────────────────────────────────────────

struct DetRng {
    state: u64,
}

impl DetRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0_f64 / ((1u64 << 53) as f64))
    }
    fn gen_range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next_f64()
    }
    fn next_gauss(&mut self, sigma: f64) -> f64 {
        // Box-Muller from two uniforms.
        let u1 = self.next_f64().max(1e-300);
        let u2 = self.next_f64();
        let r = (-2.0 * u1.ln()).sqrt();
        sigma * r * (2.0 * std::f64::consts::PI * u2).cos()
    }
}

/// `n × k` value matrix: each column is a field. Stored as Vec of rows.
type Matrix = Vec<Vec<f64>>;

fn random_matrix(n: usize, k: usize, seed: u64) -> Matrix {
    let mut rng = DetRng::new(seed);
    (0..n).map(|_| (0..k).map(|_| rng.next_gauss(1.0)).collect()).collect()
}

fn column(m: &Matrix, j: usize) -> Vec<f64> {
    m.iter().map(|row| row[j]).collect()
}

/// Apply v -> a_i * v + b_i per field (column).
fn apply_diagonal_affine(m: &Matrix, a: &[f64], b: &[f64]) -> Matrix {
    m.iter()
        .map(|row| row.iter().zip(a).zip(b).map(|((v, ai), bi)| ai * v + bi).collect())
        .collect()
}

/// Apply v -> M @ v per row (k × k linear map).
fn apply_linear_map(m: &Matrix, transform: &[[f64; 3]; 3]) -> Matrix {
    m.iter()
        .map(|row| {
            (0..3)
                .map(|i| (0..3).map(|j| transform[i][j] * row[j]).sum::<f64>())
                .collect()
        })
        .collect()
}

/// 2-D rotation embedded in 3-D: rotates fields 0, 1 by θ; field 2 fixed.
fn rotation_3d(theta: f64) -> [[f64; 3]; 3] {
    let c = theta.cos();
    let s = theta.sin();
    [[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]]
}

/// tr(Cov) of an `n × k` data matrix — sum of per-field variances.
fn trace_cov(m: &Matrix) -> f64 {
    let k = m[0].len();
    (0..k)
        .map(|j| {
            let col = column(m, j);
            let mu = col.iter().sum::<f64>() / col.len() as f64;
            col.iter().map(|x| (x - mu).powi(2)).sum::<f64>() / col.len() as f64
        })
        .sum()
}

/// Pairwise-max-distance² over a subsample (rotation-invariant
/// approximation to true diameter²).
fn diameter_sq(m: &Matrix, sample: usize) -> f64 {
    let n = m.len();
    let sz = sample.min(n);
    let stride = (n / sz).max(1);
    let mut best = 0.0_f64;
    let pts: Vec<&Vec<f64>> = (0..sz).map(|i| &m[i * stride]).collect();
    for i in 0..pts.len() {
        for j in (i + 1)..pts.len() {
            let d2: f64 = pts[i]
                .iter()
                .zip(pts[j].iter())
                .map(|(a, b)| (a - b).powi(2))
                .sum();
            if d2 > best {
                best = d2;
            }
        }
    }
    best
}

// ───────────────────────────────────────────────────────────────────
// Q-1: general GL shear breaks per-field K
// ───────────────────────────────────────────────────────────────────

/// **Q-1**: a non-diagonal shear matrix mixes fields, so per-field
/// variance over per-field range changes. Per-field K is NOT preserved.
/// This is the negative result confirming that the K-preserving group
/// is strictly inside GL(ℝᵏ).
#[test]
fn q1_general_gl_shear_breaks_per_field_k() {
    let m = random_matrix(500, 3, 11);
    let k_orig: Vec<f64> = (0..3).map(|j| compute_k(&column(&m, j))).collect();

    // Shear: M[0,1] = 2.0 (mixes field 1 into field 0).
    let shear = [[1.0, 2.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let m_sheared = apply_linear_map(&m, &shear);
    let k_sheared: Vec<f64> = (0..3).map(|j| compute_k(&column(&m_sheared, j))).collect();

    // At least one field's K must have changed substantially.
    let max_drift: f64 = (0..3)
        .map(|j| (k_orig[j] - k_sheared[j]).abs() / k_orig[j].abs().max(1e-12))
        .fold(0.0_f64, f64::max);
    assert!(
        max_drift > 0.05,
        "shear must break per-field K (max drift = {:.4})",
        max_drift
    );
}

// ───────────────────────────────────────────────────────────────────
// Q-2: diagonal affine preserves per-field K
// ───────────────────────────────────────────────────────────────────

/// **Q-2**: independent per-field affine rescalings preserve per-field
/// K bit-identically (to f64 precision). This validates
/// G_AffK = (ℝ*)^k ⋉ ℝ^k as the K-preserving group.
#[test]
fn q2_diagonal_affine_preserves_per_field_k() {
    let m = random_matrix(500, 3, 13);
    let k_orig: Vec<f64> = (0..3).map(|j| compute_k(&column(&m, j))).collect();

    let a = [2.5_f64, -1.3, 0.7];
    let b = [100.0_f64, -50.0, 300.0];
    let m_enc = apply_diagonal_affine(&m, &a, &b);
    let k_enc: Vec<f64> = (0..3).map(|j| compute_k(&column(&m_enc, j))).collect();

    for j in 0..3 {
        let delta = (k_orig[j] - k_enc[j]).abs();
        assert!(
            delta < 1e-10,
            "diagonal affine must preserve K bit-identically on field {} (delta = {:.2e})",
            j,
            delta
        );
    }
}

// ───────────────────────────────────────────────────────────────────
// Q-3: rotation preserves tr(Cov) but NOT (max-min)²
// ───────────────────────────────────────────────────────────────────

/// **Q-3**: O(k) rotation is tr(Cov)-invariant (covariance trace is the
/// sum of eigenvalues, rotation-invariant) but **does NOT** preserve
/// (max−min)² — range is taken componentwise after rotation and depends
/// on the basis. This was the bug in earlier Isometric-mode drafts.
/// Rotation-invariant trace-K must use diameter² (max pairwise
/// distance²) as the denominator, not (max−min)².
#[test]
fn q3_rotation_preserves_trcov_but_not_range_sq() {
    let m = random_matrix(500, 3, 17);
    let theta = std::f64::consts::PI / 5.0;
    let rot = rotation_3d(theta);
    let m_rot = apply_linear_map(&m, &rot);

    // tr(Cov) IS rotation-invariant.
    let trcov_orig = trace_cov(&m);
    let trcov_rot = trace_cov(&m_rot);
    assert!(
        (trcov_orig - trcov_rot).abs() < 1e-9,
        "tr(Cov) must be rotation-invariant: orig={}, rot={}",
        trcov_orig,
        trcov_rot
    );

    // (max−min)² is NOT rotation-invariant — compute on the flattened
    // value array (the way the earlier buggy draft did it).
    let flat_orig: Vec<f64> = m.iter().flatten().copied().collect();
    let flat_rot: Vec<f64> = m_rot.iter().flatten().copied().collect();
    let range_sq_orig = (flat_orig.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
        - flat_orig.iter().cloned().fold(f64::INFINITY, f64::min))
    .powi(2);
    let range_sq_rot = (flat_rot.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
        - flat_rot.iter().cloned().fold(f64::INFINITY, f64::min))
    .powi(2);
    let range_sq_drift = (range_sq_orig - range_sq_rot).abs() / range_sq_orig;
    assert!(
        range_sq_drift > 1e-3,
        "(max-min)² must NOT be rotation-invariant (drift = {:.4})",
        range_sq_drift
    );

    // tr(Cov)/diam² IS rotation-invariant (use the same sample budget for
    // both to bound the random-pair approximation error symmetrically).
    let d2_orig = diameter_sq(&m, 40);
    let d2_rot = diameter_sq(&m_rot, 40);
    let trk_orig = trcov_orig / d2_orig;
    let trk_rot = trcov_rot / d2_rot;
    let trk_drift = (trk_orig - trk_rot).abs() / trk_orig.abs().max(1e-12);
    assert!(
        trk_drift < 0.05,
        "tr(Cov)/diam² should be approximately rotation-invariant (drift = {:.4})",
        trk_drift
    );
}

// ───────────────────────────────────────────────────────────────────
// Q-4: scalar aI is a special case of diagonal affine
// ───────────────────────────────────────────────────────────────────

/// **Q-4**: scalar isotropic gauge (a_i = a for all i) is the special
/// case of diagonal affine where all entries are equal. K is preserved
/// per-field, AND Euclidean ball membership is preserved (this is the
/// Sprint P Case 1 condition).
#[test]
fn q4_scalar_isotropic_preserves_k_and_euclidean_ball() {
    let m = random_matrix(500, 3, 19);
    let k_orig: Vec<f64> = (0..3).map(|j| compute_k(&column(&m, j))).collect();

    let a_scalar = 2.5_f64;
    let b_isotropic = [10.0_f64; 3];
    let a_vec = [a_scalar; 3];
    let m_enc = apply_diagonal_affine(&m, &a_vec, &b_isotropic);
    let k_enc: Vec<f64> = (0..3).map(|j| compute_k(&column(&m_enc, j))).collect();

    for j in 0..3 {
        assert!(
            (k_orig[j] - k_enc[j]).abs() < 1e-10,
            "scalar isotropic gauge must preserve per-field K on field {}",
            j
        );
    }

    // Euclidean ball membership preservation: for two points u, v,
    // ||u - v|| under scalar gauge equals |a| * ||u_orig - v_orig||.
    // So membership in a ball of radius r maps to membership in a ball
    // of radius |a|·r. The ratio is preserved.
    let u = &m[0];
    let v = &m[1];
    let d_orig: f64 = u.iter().zip(v.iter()).map(|(a, b)| (a - b).powi(2)).sum::<f64>().sqrt();
    let u_enc = &m_enc[0];
    let v_enc = &m_enc[1];
    let d_enc: f64 = u_enc
        .iter()
        .zip(v_enc.iter())
        .map(|(a, b)| (a - b).powi(2))
        .sum::<f64>()
        .sqrt();
    let expected = a_scalar.abs() * d_orig;
    assert!(
        (d_enc - expected).abs() / expected < 1e-10,
        "scalar gauge must scale Euclidean distance by |a|: expected {}, got {}",
        expected,
        d_enc
    );
}

// ───────────────────────────────────────────────────────────────────
// Q-5: LWE illustration — K(As+e) ≈ K(random) ≠ K(s)
// ───────────────────────────────────────────────────────────────────

/// **Q-5**: LWE samples (A·s + e mod q) statistically resemble uniform
/// random samples in their K value, NOT the secret s's K. This is an
/// *illustration*, not a security proof — LWE pseudorandomness is a
/// computational hardness assumption, and K is a single statistic so
/// this only sketches the "LWE is a hiding layer, separate from the
/// gauge" framing in the spec.
///
/// The test asserts |K(lwe) − K(random)| < |K(lwe) − K(secret)|, i.e.
/// the LWE distribution's K is closer to uniform-random's K than to
/// the secret's K.
#[test]
fn q5_lwe_samples_look_more_like_random_than_secret_in_k() {
    let mut rng = DetRng::new(23);
    let n_secret = 128;
    let m_samples = 500;
    let q = (1u64 << 16) + 1; // small modulus for the toy LWE
    let sigma_e = 3.0;

    // Binary secret.
    let secret: Vec<f64> = (0..n_secret)
        .map(|_| (rng.next_u64() & 1) as f64)
        .collect();

    // A: m_samples × n_secret with uniform entries in [0, q).
    let a_matrix: Vec<Vec<f64>> = (0..m_samples)
        .map(|_| (0..n_secret).map(|_| (rng.next_u64() % q) as f64).collect())
        .collect();
    let errors: Vec<f64> = (0..m_samples).map(|_| rng.next_gauss(sigma_e)).collect();

    // LWE = (A·s + e) mod q
    let lwe: Vec<f64> = a_matrix
        .iter()
        .zip(errors.iter())
        .map(|(row, &e)| {
            let dot: f64 = row.iter().zip(secret.iter()).map(|(a, s)| a * s).sum();
            ((dot + e) % q as f64 + q as f64) % q as f64
        })
        .collect();

    // Reference uniform-random samples in [0, q).
    let random_samples: Vec<f64> =
        (0..m_samples).map(|_| (rng.next_u64() % q) as f64).collect();

    let k_secret = compute_k(&secret);
    let k_lwe = compute_k(&lwe);
    let k_rand = compute_k(&random_samples);

    let dist_to_rand = (k_lwe - k_rand).abs();
    let dist_to_secret = (k_lwe - k_secret).abs();

    // The headline assertion: LWE is closer to uniform-random than to
    // secret in K-space. (Illustration only — LWE pseudorandomness is
    // the underlying computational assumption.)
    assert!(
        dist_to_rand < dist_to_secret,
        "K(lwe) closer to K(random)={} than to K(secret)={}: K(lwe)={}",
        k_rand,
        k_secret,
        k_lwe
    );
}
