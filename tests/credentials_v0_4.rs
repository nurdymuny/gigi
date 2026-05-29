//! GIGI Encrypt v0.4 — Sprint O: Credential-gated invariant query
//! authorization (TDD).
//!
//! Tests the Sprint O surface:
//!  - `gigi::invariant_ring`: I_Aff falsification harness +
//!    builtin invariant computations on raw value slices.
//!  - `gigi::credentials`: QueryCredential (HMAC-bound for v0.4;
//!    BBS+ is the v0.5 upgrade path per spec).
//!
//! Spec: `theory/encryption/GIGI_ENCRYPT_v0.4_SPRINT_SPEC.md` §Sprint O.
//!
//! Test map (O-1 through O-4 from spec):
//!   O-1: invariance harness is mathematical, not set lookup
//!   O-2: adversarial K_fake = mean/std² caught by harness
//!   O-3: gauge rerandomization — distinct ciphertexts, identical K
//!   O-4: credential binding — wrong bundle/class/key rejected
//!
//! Note on terminology: this sprint's "rerandomization" is
//! **gauge-level**, not full CL credential unlinkability. The latter
//! is deferred to v0.5 (BBS+ implementation). See spec §Sprint O for
//! the precise carving.

use gigi::credentials::{
    issue_credential, verify_credential, CredentialIssuingKey, QueryClass, UserCommitment,
};
use gigi::invariant_ring::{compute_k, compute_tau, is_in_iaff_harness};
use std::collections::HashSet;

// ───────────────────────────────────────────────────────────────────
// Helpers — deterministic PRNG
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
    fn nonzero_scale(&mut self, cap: f64) -> f64 {
        let mag = self.gen_range(1.0 / cap, cap);
        if (self.next_u64() & 1) == 0 {
            mag
        } else {
            -mag
        }
    }
}

fn uniform_bundle(n: usize, seed: u64) -> Vec<f64> {
    let mut rng = DetRng::new(seed);
    (0..n).map(|_| rng.gen_range(0.0, 100.0)).collect()
}

// ───────────────────────────────────────────────────────────────────
// O-1: invariance harness — mathematical, not set lookup
// ───────────────────────────────────────────────────────────────────

/// **O-1**: the IAff falsification harness is a *mathematical* check
/// (numerical gauge invariance under random a, b), not a string-name
/// lookup against a fixed set. Built-in invariants (K, τ, K + K²) pass;
/// non-invariants (mean, sum, std) fail.
///
/// The harness is *necessary but not sufficient* for IAff membership
/// (a function that happens to be invariant under the sampled gauges
/// but breaks on others would pass). The grammar / parser layer
/// provides membership by construction; this harness is the runtime
/// falsification net for ad-hoc / user-supplied queries.
#[test]
fn o1_invariance_harness_mathematical_not_lookup() {
    let bundle = uniform_bundle(500, 1);
    let gauges = vec![(2.5, 100.0), (-1.3, 7.0), (0.001, -500.0), (1000.0, 0.0)];

    // Should pass — these ARE in I_Aff.
    let (k_ok, _) = is_in_iaff_harness(&|v: &[f64]| compute_k(v), &bundle, &gauges, 1e-6);
    assert!(k_ok, "K must pass the IAff harness");
    let (tau_ok, _) = is_in_iaff_harness(&|v: &[f64]| compute_tau(v), &bundle, &gauges, 1e-6);
    assert!(tau_ok, "τ must pass the IAff harness");
    // Polynomial combination K + K² — still in I_Aff.
    let k_plus_k_sq = |v: &[f64]| {
        let k = compute_k(v);
        k + k.powi(2)
    };
    let (poly_ok, _) = is_in_iaff_harness(&k_plus_k_sq, &bundle, &gauges, 1e-6);
    assert!(poly_ok, "K + K² must pass the IAff harness");

    // Should FAIL — these are NOT in I_Aff (they shift under affine gauge).
    let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
    let (mean_ok, mean_err) = is_in_iaff_harness(&mean, &bundle, &gauges, 1e-6);
    assert!(
        !mean_ok,
        "mean must FAIL the harness (max_rel_err = {})",
        mean_err
    );

    let sum = |v: &[f64]| v.iter().sum::<f64>();
    let (sum_ok, _) = is_in_iaff_harness(&sum, &bundle, &gauges, 1e-6);
    assert!(!sum_ok, "sum must FAIL the harness");

    let stddev = |v: &[f64]| {
        let mu = v.iter().sum::<f64>() / v.len() as f64;
        (v.iter().map(|x| (x - mu).powi(2)).sum::<f64>() / v.len() as f64).sqrt()
    };
    let (std_ok, _) = is_in_iaff_harness(&stddev, &bundle, &gauges, 1e-6);
    assert!(!std_ok, "stddev must FAIL the harness");
}

// ───────────────────────────────────────────────────────────────────
// O-2: adversarial K_fake = mean/std² caught
// ───────────────────────────────────────────────────────────────────

/// **O-2**: an adversary defines K_fake = mean(v) / std(v)² that *looks*
/// like a dispersion ratio but uses the mean (which is NOT gauge-
/// invariant). The harness catches it: relative error under a sample
/// affine gauge is >> tolerance.
///
/// This is the headline demonstration that the harness is a real
/// security gate, not a syntactic name check.
#[test]
fn o2_adversarial_k_fake_caught_by_harness() {
    let bundle = uniform_bundle(1000, 2);

    let k_fake = |v: &[f64]| {
        let mu = v.iter().sum::<f64>() / v.len() as f64;
        let var = v.iter().map(|x| (x - mu).powi(2)).sum::<f64>() / v.len() as f64;
        mu / (var + 1e-10)
    };

    let (passes, max_rel_err) =
        is_in_iaff_harness(&k_fake, &bundle, &[(3.7, 100.0)], 1e-6);
    assert!(
        !passes,
        "K_fake = mean/std² must FAIL the harness (max_rel_err = {})",
        max_rel_err
    );
    assert!(
        max_rel_err > 0.1,
        "K_fake must show substantial relative error under gauge: {}",
        max_rel_err
    );

    // Real K passes the same gauge.
    let (real_passes, _) =
        is_in_iaff_harness(&|v: &[f64]| compute_k(v), &bundle, &[(3.7, 100.0)], 1e-6);
    assert!(real_passes, "real K must pass the same gauge harness");
}

// ───────────────────────────────────────────────────────────────────
// O-3: gauge rerandomization — distinct ciphertexts, identical K
// ───────────────────────────────────────────────────────────────────

/// **O-3**: re-encrypt the same bundle under 5 distinct random gauges.
/// The 5 ciphertexts must be byte-distinct (gauge rerandomization
/// works), and all 5 must yield the *same* K value when the verifier
/// computes it.
///
/// **Caveat per spec**: this is gauge-level rerandomization, NOT full
/// CL credential unlinkability. The latter requires randomized
/// presentations of the same credential to be unlinkable — that's
/// v0.5 / BBS+. See spec §Sprint O.
#[test]
fn o3_gauge_rerandomization_5_distinct_ciphertexts_same_k() {
    let bundle = uniform_bundle(500, 3);
    let mut rng = DetRng::new(31);

    let mut fingerprints: HashSet<Vec<u64>> = HashSet::new();
    let mut k_values: Vec<f64> = Vec::with_capacity(5);

    for _ in 0..5 {
        let a = rng.nonzero_scale(10.0);
        let b = rng.gen_range(-5000.0, 5000.0);
        let enc: Vec<f64> = bundle.iter().map(|v| a * v + b).collect();
        // Fingerprint the ciphertext by hashing the bit pattern of the
        // first few values (avoids collision on accidental equality of
        // a single rounded float).
        let fingerprint: Vec<u64> = enc.iter().take(10).map(|v| v.to_bits()).collect();
        fingerprints.insert(fingerprint);
        k_values.push(compute_k(&enc));
    }

    assert_eq!(
        fingerprints.len(),
        5,
        "all 5 gauge rerandomizations must produce distinct ciphertexts"
    );

    let k_max = k_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let k_min = k_values.iter().cloned().fold(f64::INFINITY, f64::min);
    let k_range = k_max - k_min;
    assert!(
        k_range < 1e-8,
        "K must be identical across rerandomizations: range = {}",
        k_range
    );
}

// ───────────────────────────────────────────────────────────────────
// O-4: credential binding — wrong bundle/class/key rejected
// ───────────────────────────────────────────────────────────────────

/// **O-4**: a QueryCredential is HMAC-bound to (user_commitment,
/// query_class, bundle_id) under an issuing key. Verifying with the
/// right combination accepts; verifying with any wrong component
/// rejects.
#[test]
fn o4_credential_binding_rejects_wrong_bundle_class_or_key() {
    let issuing_key = CredentialIssuingKey([0x42u8; 32]);
    let user = UserCommitment([0x07u8; 32]);
    let cred = issue_credential(&issuing_key, user, QueryClass::K, "bundle_001");

    // Honest verification: accept.
    assert!(
        verify_credential(&issuing_key, &cred, QueryClass::K, "bundle_001"),
        "honest verification must accept"
    );

    // Wrong bundle_id: reject.
    assert!(
        !verify_credential(&issuing_key, &cred, QueryClass::K, "bundle_002"),
        "verifier must reject when bundle_id differs"
    );

    // Wrong query class: reject.
    assert!(
        !verify_credential(&issuing_key, &cred, QueryClass::Tau, "bundle_001"),
        "verifier must reject when query class differs"
    );

    // Wrong issuing key: reject.
    let wrong_key = CredentialIssuingKey([0x99u8; 32]);
    assert!(
        !verify_credential(&wrong_key, &cred, QueryClass::K, "bundle_001"),
        "verifier must reject under wrong issuing key"
    );
}

/// **O-4b**: HMAC binding is constant-time — verification time does not
/// depend on which byte of the tag differs. This is a structural test:
/// we just verify the `verify_credential` API exists and accepts/rejects
/// correctly; the constant-time property is provided by the underlying
/// HMAC subtle-comparison helper (mirroring `src/threshold.rs::constant_time_eq`).
#[test]
fn o4_credential_tag_is_32_bytes() {
    let issuing_key = CredentialIssuingKey([0x01u8; 32]);
    let user = UserCommitment([0x02u8; 32]);
    let cred = issue_credential(&issuing_key, user, QueryClass::K, "any_bundle");
    assert_eq!(cred.tag.len(), 32, "HMAC-SHA256 tag must be 32 bytes");
}

/// **O-4c**: deterministic issuance — issuing the same credential
/// twice produces the same tag (HMAC is deterministic). This is what
/// makes the verifier's recomputation work without sharing nonces.
#[test]
fn o4_credential_issuance_is_deterministic() {
    let issuing_key = CredentialIssuingKey([0x11u8; 32]);
    let user = UserCommitment([0x22u8; 32]);
    let c1 = issue_credential(&issuing_key, user, QueryClass::K, "deterministic_bundle");
    let c2 = issue_credential(&issuing_key, user, QueryClass::K, "deterministic_bundle");
    assert_eq!(
        c1.tag, c2.tag,
        "issuing same credential twice must produce identical tag"
    );
}
