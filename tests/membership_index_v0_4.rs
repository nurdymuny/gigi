//! GIGI Encrypt v0.4 — Sprint P: Geodesic-ball approximate membership
//! index (TDD).
//!
//! Tests the Sprint P surface:
//!  - `GeodesicBallIndex::new(members, alpha)` — accumulate centroid,
//!    isotropic σ², chi-square threshold χ²(k, 1−α).
//!  - `membership_check(v)` — primary Mahalanobis gate, returns verdict.
//!  - `encrypted_membership_scalar(v_enc, a, b)` — ball preserved under
//!    scalar isotropic gauge (Sprint Q Case 1).
//!
//! Spec: `theory/encryption/GIGI_ENCRYPT_v0.4_SPRINT_SPEC.md` §Sprint P.
//!
//! **Not a cryptographic accumulator.** This is a geometric approximate
//! membership filter — see spec for leakage scope (centroid +
//! covariance + count are public to anyone with read access).
//!
//! Test map (P-1 through P-5 from spec):
//!   P-1: threshold is dimension-aware (χ²(k, 1−α), not fixed 3σ)
//!   P-2: TPR matches the 1−α tail bound within O(1/√n) sampling noise
//!   P-3: adversarial boundary at 0.99·threshold — false-admit
//!        rate documented (open problem; K-consistency is diagnostic-
//!        only, not a security gate)
//!   P-4: encrypted membership preserved under scalar gauge;
//!        ellipsoidal condition required for field-wise gauge
//!   P-5: dynamic centroid drift — single deletion O(1/n), batch
//!        deletion O(|R|/n) (NOT O(1/n) as earlier drafts claimed)

use gigi::membership_index::{GeodesicBallIndex, MembershipResult};

// ───────────────────────────────────────────────────────────────────
// Helpers — deterministic PRNG + bundle builders
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
    fn next_gauss(&mut self, sigma: f64) -> f64 {
        let u1 = self.next_f64().max(1e-300);
        let u2 = self.next_f64();
        let r = (-2.0 * u1.ln()).sqrt();
        sigma * r * (2.0 * std::f64::consts::PI * u2).cos()
    }
}

fn random_members(n: usize, k: usize, seed: u64) -> Vec<Vec<f64>> {
    let mut rng = DetRng::new(seed);
    (0..n)
        .map(|_| (0..k).map(|_| rng.next_gauss(1.0)).collect())
        .collect()
}

// ───────────────────────────────────────────────────────────────────
// P-1: threshold is dimension-aware (χ²(k, 1−α), not fixed 3σ)
// ───────────────────────────────────────────────────────────────────

/// **P-1**: the threshold scales with dimension k. χ²(1, 0.95) ≈ 3.84,
/// χ²(4, 0.95) ≈ 9.49. A fixed 3σ threshold would treat k=1 and k=4
/// identically — that's the bug the dimension-aware threshold fixes.
#[test]
fn p1_threshold_is_dimension_aware() {
    let m1 = random_members(100, 1, 101);
    let m4 = random_members(100, 4, 104);
    let idx1 = GeodesicBallIndex::new(&m1, 0.05);
    let idx4 = GeodesicBallIndex::new(&m4, 0.05);

    // Thresholds must differ by dimension.
    assert!(
        (idx1.chi2_threshold() - idx4.chi2_threshold()).abs() > 1.0,
        "thresholds must be dimension-aware: k=1 → {}, k=4 → {}",
        idx1.chi2_threshold(),
        idx4.chi2_threshold()
    );

    // χ²(1, 0.95) ≈ 3.84 (Wilson-Hilferty accuracy: ~2-3%).
    assert!(
        (idx1.chi2_threshold() - 3.84).abs() < 0.15,
        "χ²(1, 0.95) should be ≈ 3.84, got {}",
        idx1.chi2_threshold()
    );
    // χ²(4, 0.95) ≈ 9.49.
    assert!(
        (idx4.chi2_threshold() - 9.49).abs() < 0.15,
        "χ²(4, 0.95) should be ≈ 9.49, got {}",
        idx4.chi2_threshold()
    );
}

// ───────────────────────────────────────────────────────────────────
// P-2: TPR matches the 1−α tail bound within O(1/√n) sampling noise
// ───────────────────────────────────────────────────────────────────

/// **P-2**: True positive rate should converge to 1−α as n→∞ with
/// sampling deviation O(1/√n). For α=0.05 and n=100 the 99% CI on TPR
/// is approximately 1−α ± 3·√(α(1−α)/n) = 0.95 ± 0.065. Assertion:
/// |TPR − (1−α)| < 3·sampling_sd. The headline number is 0.95 + O(1/√n).
#[test]
fn p2_tpr_matches_one_minus_alpha_tail_bound() {
    let members = random_members(200, 3, 17);
    let alpha = 0.05_f64;
    let idx = GeodesicBallIndex::new(&members, alpha);

    let tpr = members
        .iter()
        .filter(|m| idx.membership_check(m).is_member())
        .count() as f64
        / (members.len() as f64);

    let expected = 1.0 - alpha;
    let n = members.len() as f64;
    // 3 standard deviations of the binomial sample around 1−α.
    let sampling_sd = (expected * alpha / n).sqrt();
    let tolerance = 3.0 * sampling_sd;
    assert!(
        (tpr - expected).abs() < tolerance,
        "TPR = {} (expected ≈ {} ± {})",
        tpr,
        expected,
        tolerance
    );
}

// ───────────────────────────────────────────────────────────────────
// P-3: adversarial boundary — false-admit rate documented
// ───────────────────────────────────────────────────────────────────

/// **P-3**: an adversarial point placed at 0.99 × the Mahalanobis
/// boundary is admitted. This is the *expected* behavior — by
/// construction the index admits anything inside the χ²-quantile ball.
/// The test documents the false-admit rate (FAR) so it cannot be
/// hidden; spec acknowledges this as the open problem requiring a
/// formal membership witness in v0.5.
#[test]
fn p3_adversarial_boundary_false_admit_documented() {
    let members = random_members(100, 3, 23);
    let idx = GeodesicBallIndex::new(&members, 0.05);

    let mut rng = DetRng::new(101);
    let mut false_admits = 0;
    let n_trials = 500;
    for _ in 0..n_trials {
        // Random unit direction in ℝ³.
        let mut dir: Vec<f64> = (0..3).map(|_| rng.next_gauss(1.0)).collect();
        let norm: f64 = dir.iter().map(|d| d * d).sum::<f64>().sqrt();
        for d in dir.iter_mut() {
            *d /= norm;
        }
        // Place at 0.99 × the Mahalanobis radius along dir.
        let r = (idx.chi2_threshold() * idx.sigma_sq()).sqrt() * 0.99;
        let adversary: Vec<f64> = idx
            .centroid()
            .iter()
            .zip(dir.iter())
            .map(|(c, d)| c + r * d)
            .collect();
        if idx.membership_check(&adversary).is_member() {
            false_admits += 1;
        }
    }
    let far = false_admits as f64 / n_trials as f64;
    // Documented expectation: anything strictly inside the boundary IS
    // admitted; FAR should be near 1.0 by construction. This test pins
    // that behavior so any change is intentional.
    assert!(
        far > 0.95,
        "boundary points should be admitted (FAR = {} of {})",
        far,
        n_trials
    );
    // Spec follow-up: K-consistency is diagnostic only, not a security
    // gate; formal membership witness is the v0.5 open problem.
}

// ───────────────────────────────────────────────────────────────────
// P-4: encrypted membership preserved under scalar gauge
// ───────────────────────────────────────────────────────────────────

/// **P-4**: under scalar isotropic gauge g(v) = a·v + b (same a for all
/// fields), Euclidean ball membership is preserved. The encrypted
/// membership check on `g(v)` against the gauge-transformed index
/// agrees with the plaintext check on `v` against the original index.
#[test]
fn p4_scalar_gauge_preserves_ball_membership() {
    let members = random_members(100, 3, 29);
    let idx = GeodesicBallIndex::new(&members, 0.05);

    let a = 2.5_f64;
    let b = 100.0_f64;
    let enc_members: Vec<Vec<f64>> = members
        .iter()
        .map(|m| m.iter().map(|v| a * v + b).collect())
        .collect();
    let enc_idx = GeodesicBallIndex::new(&enc_members, 0.05);

    // Check 20 of the known members for membership consistency.
    for m in members.iter().take(20) {
        let plain_verdict = idx.membership_check(m).is_member();
        let enc_v: Vec<f64> = m.iter().map(|v| a * v + b).collect();
        let enc_verdict = enc_idx.membership_check(&enc_v).is_member();
        assert_eq!(
            plain_verdict, enc_verdict,
            "scalar gauge must preserve membership verdict"
        );
    }

    // Also test 20 random non-members (Gaussian draws at the far edge
    // of the data manifold).
    let mut rng = DetRng::new(99);
    for _ in 0..20 {
        let non_member: Vec<f64> = (0..3).map(|_| 5.0 + rng.next_gauss(0.5)).collect();
        let plain_verdict = idx.membership_check(&non_member).is_member();
        let enc_v: Vec<f64> = non_member.iter().map(|v| a * v + b).collect();
        let enc_verdict = enc_idx.membership_check(&enc_v).is_member();
        assert_eq!(
            plain_verdict, enc_verdict,
            "scalar gauge must preserve non-membership verdict"
        );
    }
}

// ───────────────────────────────────────────────────────────────────
// P-5: dynamic centroid drift — single deletion O(1/n), batch O(|R|/n)
// ───────────────────────────────────────────────────────────────────

/// **P-5**: deleting one member shifts the centroid by O(1/n); deleting
/// a batch of size |R| shifts the centroid by O(|R|/n) — NOT O(1/n).
/// This test confirms the corrected complexity claim from the spec
/// (earlier drafts incorrectly claimed batch removal was O(1/n)).
///
/// Averaged across multiple random seeds + larger n to suppress
/// per-seed variance in the drift ratio.
#[test]
fn p5_dynamic_centroid_drift_single_vs_batch() {
    let n = 300;
    let batch_size = 60; // 20% removal
    let n_seeds = 8;
    let mut total_single = 0.0_f64;
    let mut total_batch = 0.0_f64;

    for seed in 0..n_seeds {
        let members = random_members(n, 3, 1000 + seed);
        let idx_full = GeodesicBallIndex::new(&members, 0.05);
        let idx_minus1 = GeodesicBallIndex::new(&members[..n - 1], 0.05);
        let idx_minus_batch = GeodesicBallIndex::new(&members[..n - batch_size], 0.05);

        let d_single = euclidean_distance(idx_full.centroid(), idx_minus1.centroid());
        let d_batch = euclidean_distance(idx_full.centroid(), idx_minus_batch.centroid());

        // Per-seed: batch must exceed single (this is structural — the
        // batch is a superset of "remove last 1", so its drift is
        // dominated by a sum of ≥|R| terms).
        assert!(
            d_batch > d_single,
            "seed {}: batch drift ({}) must exceed single ({})",
            seed,
            d_batch,
            d_single
        );

        // Single deletion drift bound: ≤ max||x − μ||/(n−1).
        // For n=300 with unit-Gaussian data, single drift is < 0.05
        // typically.
        assert!(
            d_single < 0.1,
            "seed {}: single-deletion drift too large for n=300: {}",
            seed,
            d_single
        );

        total_single += d_single;
        total_batch += d_batch;
    }

    let avg_single = total_single / (n_seeds as f64);
    let avg_batch = total_batch / (n_seeds as f64);

    // Expected ratio for random i.i.d. data: batch_drift ≈ √(|R|)·σ /
    // (n−|R|) and single_drift ≈ √k·σ / (n−1). For k=3, n=300, |R|=60:
    //   batch ≈ √60 · σ / 240 ≈ 0.0322·σ
    //   single ≈ √3 · σ / 299 ≈ 0.00579·σ
    //   ratio ≈ 5.6
    // With 8 seeds, the SE on the ratio is reduced enough to assert > 3.
    let ratio = avg_batch / avg_single.max(1e-12);
    assert!(
        ratio > 3.0,
        "averaged batch/single ratio ({:.2}, single={:.4}, batch={:.4}) inconsistent with O(|R|/n) scaling",
        ratio, avg_single, avg_batch
    );
}

fn euclidean_distance(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| (x - y).powi(2)).sum::<f64>().sqrt()
}

// ───────────────────────────────────────────────────────────────────
// Smoke test for non-member rejection
// ───────────────────────────────────────────────────────────────────

#[test]
fn smoke_non_member_far_from_centroid_rejected() {
    let members = random_members(100, 3, 37);
    let idx = GeodesicBallIndex::new(&members, 0.05);
    // Place at distance 100·σ from centroid — definitely outside any
    // reasonable χ² ball.
    let far: Vec<f64> = idx.centroid().iter().map(|c| c + 100.0).collect();
    assert!(
        !idx.membership_check(&far).is_member(),
        "point 100σ away must be rejected"
    );
}
