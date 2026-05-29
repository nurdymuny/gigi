//! GIGI Encrypt v0.4 — Sprint N: Invariant Consistency Verification (TDD).
//!
//! Tests the v0.4 `InvariantStatement` / `verify_invariant_statement` surface.
//! Spec: `theory/encryption/GIGI_ENCRYPT_v0.4_SPRINT_SPEC.md` §Sprint N.
//!
//! Run with: `cargo test --test invariant_verify_v0_4`
//!
//! TDD red-first: these tests are written BEFORE the implementation. They
//! exercise the public API surface as it is meant to be consumed by an
//! auditor / verifier holding only the encrypted bundle.

use gigi::crypto::GaugeKey;
use gigi::integrity::InvariantTuple;
use gigi::invariant_verify::{
    verify_invariant_statement, InvariantStatement, InvariantTolerances, VerifyResult,
};
use gigi::types::{BundleSchema, EncryptionMode, FieldDef, Value};
use gigi::BundleStore;
use std::collections::HashMap;

// ───────────────────────────────────────────────────────────────────────
// Test helpers — deterministic PRNG (SplitMix64), bundle builders, the
// adversarial same-K-different-λ₁ construction from spec §Sprint N N-4.
// ───────────────────────────────────────────────────────────────────────

/// SplitMix64 for deterministic, reproducible test data without pulling
/// in a dev-dep on `rand`.
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
    /// Uniform double in [0, 1).
    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0_f64 / ((1u64 << 53) as f64))
    }
    /// Uniform double in [lo, hi).
    fn gen_range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next_f64()
    }
    /// Signed non-zero scale factor in [-cap, -1/cap] ∪ [1/cap, cap].
    fn nonzero_scale(&mut self, cap: f64) -> f64 {
        let mag = self.gen_range(1.0 / cap, cap);
        if (self.next_u64() & 1) == 0 {
            mag
        } else {
            -mag
        }
    }
}

/// Build a fresh BundleStore with one fiber field "value", populated from
/// `vals` paired with sequential ids.
fn build_bundle(vals: &[f64], schema_name: &str) -> BundleStore {
    let schema = BundleSchema::new(schema_name)
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::numeric("value"));
    let mut store = BundleStore::new(schema);
    for (i, v) in vals.iter().enumerate() {
        let mut rec = HashMap::new();
        rec.insert("id".to_string(), Value::Float(i as f64));
        rec.insert("value".to_string(), Value::Float(*v));
        store.insert(&rec);
    }
    store
}

/// Apply an affine gauge to a value array: w = a * v + b. This simulates
/// what the prover does to encrypt before publishing the bundle to the
/// verifier — the resulting bundle's invariant tuple is identical to the
/// plaintext's by Theorem 3.5.
fn apply_affine_gauge(vals: &[f64], a: f64, b: f64) -> Vec<f64> {
    vals.iter().map(|v| a * v + b).collect()
}

/// Spec §Sprint N N-4 helper: build two bundles where K is nearly equal
/// (within O(1/n) sampling noise) but the *full* fingerprint differs on
/// record_count.
///
/// Construction: b2 = b1 plus one duplicate value at the mean of b1.
/// Adding a record at the existing mean preserves the mean exactly,
/// shifts **population** variance (Σ(x−μ)²/n) by factor n/(n+1) — the
/// numerator gains 0 from the new point but the denominator grows from
/// n to n+1 — and leaves range unchanged (the new point sits strictly
/// inside the existing range). The production code computes population
/// variance, so:
///
///   K_pop(b2) = (n / (n+1)) · K_pop(b1)
///
/// — a relative shift of 1/(n+1), e.g. ~0.5% for n = 200. (For
/// *sample* variance Σ(x−μ)²/(n−1) the factor would be (n−1)/n, also
/// O(1/n).) A K-only verifier with realistic tolerance (≥ 1%) would
/// accept b2 against b1's claim. The full tuple catches it because
/// record_count differs by 1.
///
/// (The spec's headline phrasing — "same K different λ_1" — applies
/// when the value field is INDEXED so its clustering structure drives
/// the field-index graph. We use record_count here because it's the
/// simplest and most realistic adversary scenario: tampering with the
/// existence of a single record is the highest-payoff modification, and
/// the fingerprint catches it exactly via the u64 record_count slot.)
fn adversarial_near_same_k_different_record_count(n: usize, seed: u64) -> (Vec<f64>, Vec<f64>) {
    let mut rng = DetRng::new(seed);
    let b1: Vec<f64> = (0..n).map(|_| rng.next_f64()).collect();
    let mean_b1 = b1.iter().sum::<f64>() / (n as f64);
    let mut b2 = b1.clone();
    b2.push(mean_b1); // duplicate-at-mean → minimal K perturbation
    (b1, b2)
}

// ───────────────────────────────────────────────────────────────────────
// Sprint N — Tests (spec §Sprint N TDD section)
// ───────────────────────────────────────────────────────────────────────

/// **N-1**: the verifier computes K (and the full tuple) from the
/// bundle without ever holding the gauge key. Plain and gauge-encrypted
/// bundles produce the same invariant tuple (Theorem 3.5 in action).
#[test]
fn n1_verifier_needs_no_gauge_key() {
    let plain_vals: Vec<f64> = (0..200).map(|i| (i as f64) * 0.13 + 1.7).collect();
    let plain_bundle = build_bundle(&plain_vals, "n1_plain");
    let plain_tuple = InvariantTuple::compute(&plain_bundle);

    // Apply an affine gauge — these are the values the bank would publish.
    let (a, b) = (2.5_f64, -3.0_f64);
    let enc_vals = apply_affine_gauge(&plain_vals, a, b);
    let enc_bundle = build_bundle(&enc_vals, "n1_enc");
    let enc_tuple = InvariantTuple::compute(&enc_bundle);

    // Six-component agreement within tolerance — K is gauge-invariant by
    // construction.
    assert!(
        (plain_tuple.k - enc_tuple.k).abs() < 1e-9,
        "K must be gauge-invariant: plain={}, enc={}",
        plain_tuple.k,
        enc_tuple.k
    );
    assert!(
        (plain_tuple.lambda_1 - enc_tuple.lambda_1).abs() < 1e-9,
        "λ_1 must be gauge-invariant: plain={}, enc={}",
        plain_tuple.lambda_1,
        enc_tuple.lambda_1
    );
    assert_eq!(plain_tuple.record_count, enc_tuple.record_count);
    assert_eq!(plain_tuple.beta_0, enc_tuple.beta_0);
    assert_eq!(plain_tuple.beta_1, enc_tuple.beta_1);

    // The auditor's verifier accepts the prover's honest claim from the
    // encrypted bundle — no gauge key passed in.
    let statement = InvariantStatement::from_bundle(&plain_bundle, "n1");
    let result = verify_invariant_statement(
        &enc_bundle, // verifier holds the encrypted bundle ONLY
        "n1",        // and asserts it represents bundle "n1"
        &statement,
        InvariantTolerances::default(),
    );
    assert!(
        matches!(result, VerifyResult::Verified { .. }),
        "honest claim must verify on the encrypted bundle: {:?}",
        result
    );
}

/// **N-2**: soundness — a prover who submits a wrong K (or any wrong
/// component) is caught by recomputation. Returns `Rejected { field, .. }`
/// naming the disagreeing field.
#[test]
fn n2_soundness_wrong_k_caught() {
    let vals: Vec<f64> = (0..150).map(|i| ((i * 13) as f64).sin() * 10.0 + 50.0).collect();
    let bundle = build_bundle(&vals, "n2");
    let honest = InvariantStatement::from_bundle(&bundle, "n2");

    // Tamper with K — replace with an obviously wrong value.
    let mut tampered = honest.clone();
    tampered.claimed.k = honest.claimed.k + 0.1;

    let result =
        verify_invariant_statement(&bundle, "n2", &tampered, InvariantTolerances::default());
    match result {
        VerifyResult::Rejected { field, delta, .. } => {
            assert_eq!(field, "k", "rejection must name the K field");
            assert!(delta > 0.01, "delta must reflect the tampering magnitude");
        }
        VerifyResult::Verified { .. } => panic!("tampered K must be rejected, got Verified"),
        VerifyResult::BundleMismatch { .. } => {
            panic!("bundle_ids match in this test path — should not hit mismatch")
        }
    }
}

/// **N-2b**: tampering with λ_1, record_count, β₀, β₁ are each detected
/// (not just K). Verifier returns `Rejected { field: <the tampered one> }`.
#[test]
fn n2_soundness_each_field_independently_detected() {
    let vals: Vec<f64> = (0..120).map(|i| (i as f64) * 0.7 - 5.0).collect();
    let bundle = build_bundle(&vals, "n2b");
    let honest = InvariantStatement::from_bundle(&bundle, "n2b");

    for (name, mutator) in [
        ("lambda1", Box::new(|t: &mut InvariantTuple| t.lambda_1 += 0.5)
            as Box<dyn Fn(&mut InvariantTuple)>),
        ("holonomy_mean", Box::new(|t| t.holonomy_mean += 0.5)),
        ("record_count", Box::new(|t| t.record_count += 1)),
        ("beta_0", Box::new(|t| t.beta_0 += 1)),
        ("beta_1", Box::new(|t| t.beta_1 += 1)),
    ] {
        let mut tampered = honest.clone();
        mutator(&mut tampered.claimed);
        let result =
            verify_invariant_statement(&bundle, "n2b", &tampered, InvariantTolerances::default());
        match result {
            VerifyResult::Rejected { field, .. } => assert_eq!(
                field, name,
                "rejection must name the tampered field ({})",
                name
            ),
            VerifyResult::Verified { .. } => {
                panic!("tampering of {} must be rejected, got Verified", name)
            }
            VerifyResult::BundleMismatch { .. } => {
                panic!("bundle_ids match in this test path — should not hit mismatch")
            }
        }
    }
}

/// **N-3**: completeness — across 1000 random bundles each encrypted
/// under a random affine gauge, the honest prover succeeds with rate
/// ≥ 0.999. This matches the spec's headline number (was 100 in the
/// initial landing — bumped to 1000 per review follow-up #4).
#[test]
fn n3_completeness_random_bundles() {
    let mut rng = DetRng::new(42);
    let mut successes = 0;
    let n_trials = 1000;
    for trial in 0..n_trials {
        let n = 64 + (rng.next_u64() % 64) as usize; // 64..128 records
        let vals: Vec<f64> = (0..n)
            .map(|i| (i as f64) * rng.gen_range(0.1, 2.0) + rng.gen_range(-10.0, 10.0))
            .collect();
        let plain = build_bundle(&vals, &format!("n3_plain_{}", trial));
        let statement = InvariantStatement::from_bundle(&plain, format!("n3_{}", trial));

        // Encrypt under a random non-zero affine gauge.
        let a = rng.nonzero_scale(100.0);
        let b = rng.gen_range(-1000.0, 1000.0);
        let enc_vals = apply_affine_gauge(&vals, a, b);
        let enc = build_bundle(&enc_vals, &format!("n3_enc_{}", trial));

        let result =
            verify_invariant_statement(
                &enc,
                &format!("n3_{}", trial),
                &statement,
                InvariantTolerances::default(),
            );
        if matches!(result, VerifyResult::Verified { .. }) {
            successes += 1;
        }
    }
    // Completeness ≥ 0.999 (999 of 1000 trials) — matches the spec's
    // headline number. The 0.1% slack absorbs rare f64 quantization
    // edge cases at the 10⁻¹⁰ tolerance noise floor.
    assert!(
        (successes as f64) / (n_trials as f64) >= 0.999,
        "completeness too low: {}/{}",
        successes,
        n_trials
    );
}

/// **N-4**: full-tuple-required — K alone is gameable. An adversary
/// constructs b2 with K(b2) within sampling noise of K(b1), but the
/// fingerprint differs on record_count. A K-only verifier with realistic
/// tolerance would accept; the full-tuple verifier catches it.
///
/// This test demonstrates *why* the spec mandates the full tuple as the
/// invariant fingerprint, not just K. See helper's docstring for the
/// "spec headline (same K different λ_1) vs operational (same K
/// different record_count)" framing.
#[test]
fn n4_full_tuple_required_to_catch_gameable_k() {
    let n = 200;
    let (b1_vals, b2_vals) = adversarial_near_same_k_different_record_count(n, 17);
    assert_eq!(b1_vals.len(), n);
    assert_eq!(b2_vals.len(), n + 1);

    let b1 = build_bundle(&b1_vals, "n4_b1");
    let b2 = build_bundle(&b2_vals, "n4_b2");

    let t1 = InvariantTuple::compute(&b1);
    let t2 = InvariantTuple::compute(&b2);

    // K shift is bounded by 1/(n+1) ≈ 0.5% (duplicate-at-mean perturbs
    // only the variance, by factor n/(n+1); range and mean unchanged).
    let k_rel = (t1.k - t2.k).abs() / t1.k.abs().max(1e-12);
    assert!(
        k_rel < 0.02,
        "K shift should be small: K1={}, K2={}, rel={}",
        t1.k,
        t2.k,
        k_rel
    );

    // record_count delta is exactly 1.
    assert_eq!(t2.record_count, t1.record_count + 1);

    // The auditor holds b2 (the adversary's bundle) but the prover claims
    // the tuple from b1. The full-tuple verifier MUST reject — record_count
    // is u64 and checked for exact equality.
    let statement = InvariantStatement::from_bundle(&b1, "n4");
    let result =
        verify_invariant_statement(&b2, "n4", &statement, InvariantTolerances::default());
    match result {
        VerifyResult::Rejected { field, .. } => {
            // K may fire first (the 0.5% shift exceeds the 1e-10 default
            // tolerance); record_count would fire if K matched within
            // tolerance. Either outcome confirms the full-tuple verifier
            // caught what a K-only check would have missed under loose K
            // tolerance.
            assert!(
                field == "k" || field == "record_count",
                "full-tuple verifier must reject on K or record_count, got: {}",
                field
            );
        }
        VerifyResult::Verified { .. } => panic!(
            "duplicate-record adversary must be caught by the full-tuple verifier"
        ),
        VerifyResult::BundleMismatch { .. } => {
            panic!("bundle_ids match in this test path — should not hit mismatch")
        }
    }

    // **The honest "full-tuple beats K-only" demonstration**: even if K
    // would have agreed (e.g., by tampering K to match b1's claim while
    // adding the duplicate record), the record_count field — which is
    // u64 and not subject to tolerance bypass — catches the substitution.
    // This is the operational meaning of "K alone is gameable."
    let mut k_tampered_to_match = statement.clone();
    k_tampered_to_match.claimed.k = t2.k; // adversary forges K to match b2
    let result2 = verify_invariant_statement(
        &b2,
        "n4",
        &k_tampered_to_match,
        InvariantTolerances::default(),
    );
    match result2 {
        VerifyResult::Rejected { field, .. } => assert_eq!(
            field, "record_count",
            "after K is tampered to match, record_count must catch the delta"
        ),
        VerifyResult::Verified { .. } => panic!(
            "record_count is u64 and must reject the delta even if K is forged"
        ),
        VerifyResult::BundleMismatch { .. } => panic!(
            "bundle_ids match in this test path — should not hit mismatch"
        ),
    }
}

/// **N-5** (v0.4 review follow-up Gap 1): bundle-identity check fires
/// *before* the tuple is computed. A claim about bundle A presented to
/// a verifier holding bundle B is rejected on identity grounds, even
/// when the tuples happen to coincide.
///
/// This closes the trust-handoff hole flagged in the encryption-team
/// review of Sprint N. Previously the `bundle_id` field on
/// `InvariantStatement` was decorative; the API now binds it.
#[test]
fn n5_bundle_mismatch_caught_before_tuple_check() {
    let vals_a: Vec<f64> = (0..50).map(|i| (i as f64) * 0.5 + 1.0).collect();
    let vals_b: Vec<f64> = (0..50).map(|i| (i as f64) * 0.5 + 1.0).collect(); // identical data
    let bundle_a = build_bundle(&vals_a, "n5_a");
    let bundle_b = build_bundle(&vals_b, "n5_b");

    // Tuples are identical because the data is identical — without
    // bundle_id binding, the verifier would accept the cross-claim.
    let tuple_a = InvariantTuple::compute(&bundle_a);
    let tuple_b = InvariantTuple::compute(&bundle_b);
    assert_eq!(tuple_a, tuple_b, "constructed identical tuples for the test");

    // Prover claims tuple for "bundle_A"; verifier holds "bundle_B".
    let statement = InvariantStatement::from_bundle(&bundle_a, "bundle_A");
    let result = verify_invariant_statement(
        &bundle_b,
        "bundle_B",
        &statement,
        InvariantTolerances::default(),
    );

    match result {
        VerifyResult::BundleMismatch { claimed, store_id } => {
            assert_eq!(claimed, "bundle_A");
            assert_eq!(store_id, "bundle_B");
        }
        VerifyResult::Verified { .. } => panic!(
            "without bundle_id binding, identical-tuple bundles would falsely verify"
        ),
        VerifyResult::Rejected { .. } => panic!(
            "tuples are identical — only identity check should fire"
        ),
    }
}

/// **N-5b**: when the bundle_id matches, the binding does not get in
/// the way of the normal tuple-checking path. Sanity test that the new
/// API surface doesn't introduce false rejections on the happy path.
#[test]
fn n5b_matching_bundle_id_does_not_block_verified() {
    let vals: Vec<f64> = (0..80).map(|i| (i as f64).sin() * 5.0 + 50.0).collect();
    let bundle = build_bundle(&vals, "matching_id");
    let statement = InvariantStatement::from_bundle(&bundle, "matching_id");
    let result = verify_invariant_statement(
        &bundle,
        "matching_id",
        &statement,
        InvariantTolerances::default(),
    );
    assert!(matches!(result, VerifyResult::Verified { .. }));
}

// ───────────────────────────────────────────────────────────────────────
// N-6: Stronger full-tuple adversary on an INDEXED value field
// (v0.4 review follow-up Gap 3 — restores the spec's headline
//  "same K different λ_1" construction by varying graph topology
//  on a field that is actually indexed).
// ───────────────────────────────────────────────────────────────────────

/// Build a bundle whose `category` field is INDEXED — the field-index
/// graph (used by spectral_gap and Betti) is sensitive to which
/// records share which category values. The `value` field carries the
/// numeric data K is computed from.
fn build_indexed_bundle(
    vals: &[f64],
    categories: &[&str],
    schema_name: &str,
) -> BundleStore {
    assert_eq!(vals.len(), categories.len());
    let schema = BundleSchema::new(schema_name)
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::numeric("value"))
        .fiber(FieldDef::categorical("category"))
        .index("category");
    let mut store = BundleStore::new(schema);
    for (i, (v, c)) in vals.iter().zip(categories.iter()).enumerate() {
        let mut rec = HashMap::new();
        rec.insert("id".to_string(), Value::Float(i as f64));
        rec.insert("value".to_string(), Value::Float(*v));
        rec.insert("category".to_string(), Value::Text(c.to_string()));
        store.insert(&rec);
    }
    store
}

/// **N-6**: stronger full-tuple adversary using INDEXED-field topology
/// variation — the spec's headline "same K different λ_1" construction.
///
/// Two bundles share identical numeric values (so K, mean, range, τ
/// all match) but differ on category topology:
///
///   - b_clique:   every record has category="shared" → field-index
///                 graph is the complete graph K_n → λ_1 = n/(n-1) ≈ 1.0
///                 and β_0 = 1 (single connected component).
///   - b_isolated: every record has a unique category → no shared
///                 values → n disconnected components → λ_1 = 0
///                 and β_0 = n.
///
/// K agrees bit-identically because the value field carries identical
/// data; record_count agrees because both bundles have the same n.
/// The full tuple catches the substitution on λ_1 (or β_0, whichever
/// the fingerprint-order check hits first).
///
/// This is what the spec's N-4 originally aimed at; we ship it as a
/// dedicated test now that the spec-correction motivation (production
/// λ_1 lives on the field-index graph, not sorted-diff variance) has a
/// clean test surface to demonstrate it on.
#[test]
fn n6_same_k_different_topology_caught_by_full_tuple() {
    let n = 80;
    let vals: Vec<f64> = (0..n).map(|i| (i as f64) * 0.7 + 1.3).collect();

    // b_clique: all same category → 1 component, clique graph.
    let cats_clique: Vec<&str> = vec!["shared"; n];
    let b_clique = build_indexed_bundle(&vals, &cats_clique, "n6_clique");

    // b_isolated: all unique categories → n components, no edges.
    let unique_cats: Vec<String> = (0..n).map(|i| format!("unique_{}", i)).collect();
    let cats_isolated: Vec<&str> = unique_cats.iter().map(|s| s.as_str()).collect();
    let b_isolated = build_indexed_bundle(&vals, &cats_isolated, "n6_isolated");

    let t_clique = InvariantTuple::compute(&b_clique);
    let t_isolated = InvariantTuple::compute(&b_isolated);

    // K agrees — value field is identical.
    assert!(
        (t_clique.k - t_isolated.k).abs() < 1e-9,
        "K must agree (same value data): clique={}, isolated={}",
        t_clique.k,
        t_isolated.k
    );
    // record_count agrees — same n.
    assert_eq!(t_clique.record_count, t_isolated.record_count);

    // λ_1 differs maximally: clique → n/(n-1), isolated → 0.
    assert!(
        (t_clique.lambda_1 - t_isolated.lambda_1).abs() > 0.5,
        "λ_1 must differ substantially: clique={}, isolated={}",
        t_clique.lambda_1,
        t_isolated.lambda_1
    );
    // β_0 also differs maximally: 1 vs n.
    assert!(
        t_clique.beta_0 != t_isolated.beta_0,
        "β_0 must differ (1 vs n): clique={}, isolated={}",
        t_clique.beta_0,
        t_isolated.beta_0
    );

    // Prover claims tuple for b_clique; verifier holds b_isolated under
    // the same bundle_id (an honest claim against the wrong bundle —
    // realistic spoofing scenario). The full-tuple verifier MUST reject
    // — λ_1 or β_0 will fire.
    let statement = InvariantStatement::from_bundle(&b_clique, "n6_target");
    let result = verify_invariant_statement(
        &b_isolated,
        "n6_target",
        &statement,
        InvariantTolerances::default(),
    );
    match result {
        VerifyResult::Rejected { field, .. } => {
            assert!(
                field == "lambda1" || field == "beta_0" || field == "holonomy_mean",
                "full-tuple verifier must reject on a topology field, got: {}",
                field
            );
        }
        VerifyResult::Verified { .. } => panic!(
            "same-K-different-topology adversary must be caught by the full tuple"
        ),
        VerifyResult::BundleMismatch { .. } => {
            panic!("bundle_ids match — should not hit mismatch path")
        }
    }
}

// ───────────────────────────────────────────────────────────────────────
// N-7: True end-to-end through the production EncryptionMode::Affine
// write path (v0.4 review follow-up — final outstanding smaller note).
//
// Earlier tests simulated encryption by applying `a*v + b` directly to
// raw value arrays and inserting the transformed values into a plain
// bundle. That tests the math but bypasses the production encryption
// pipeline. N-7 routes through:
//
//   schema.gauge_key = Some(GaugeKey::derive(seed, fiber_fields))
//   schema.fiber("value").with_encryption(EncryptionMode::Affine)
//
// so `insert()` actually runs `gk.encrypt_fiber(...)` on each write
// and the bundle stores ciphertext. The Sprint N verifier should then
// reproduce the plaintext-bundle's invariant tuple from the encrypted
// bundle alone — Theorem 3.5 (gauge invariance) end-to-end through
// the real write path.
// ───────────────────────────────────────────────────────────────────────

/// Build a bundle whose `value` fiber field is encrypted under the
/// production Affine pipeline. `seed` deterministically derives the
/// gauge transform.
fn build_encrypted_bundle(vals: &[f64], schema_name: &str, seed: [u8; 32]) -> BundleStore {
    let mut schema = BundleSchema::new(schema_name)
        .base(FieldDef::numeric("id"))
        .fiber(
            FieldDef::numeric("value")
                .with_range(1000.0)
                .with_encryption(EncryptionMode::Affine),
        );
    schema.gauge_key = Some(GaugeKey::derive(&seed, &schema.fiber_fields));
    let mut store = BundleStore::new(schema);
    for (i, v) in vals.iter().enumerate() {
        let mut rec = HashMap::new();
        rec.insert("id".to_string(), Value::Float(i as f64));
        rec.insert("value".to_string(), Value::Float(*v));
        store.insert(&rec);
    }
    store
}

/// **N-7**: end-to-end through the production write path.
///
/// Plain bundle: schema with no gauge_key; raw values stored.
/// Encrypted bundle: same schema name + `with_encryption(Affine)` +
/// `gauge_key = GaugeKey::derive(seed, ...)`. The SAME plaintext
/// records are inserted into both; the encrypted bundle stores
/// ciphertext (insert() routes through encrypt_fiber).
///
/// Asserts:
///  (a) the two `InvariantTuple::compute` outputs agree on all 6
///      components to within the 1e-10 tolerance — Theorem 3.5 in
///      action through the production pipeline.
///  (b) the Sprint N verifier accepts an honest claim from the plain
///      side against the encrypted store, given matching bundle_id.
fn n7_end_to_end_inner(seed: [u8; 32]) {
    let vals: Vec<f64> = (0..150).map(|i| ((i * 7 + 3) as f64) * 0.13 + 1.7).collect();

    let plain = build_bundle(&vals, "n7_e2e");
    let encrypted = build_encrypted_bundle(&vals, "n7_e2e", seed);

    let t_plain = InvariantTuple::compute(&plain);
    let t_enc = InvariantTuple::compute(&encrypted);

    // (a) Tuples agree on all 6 components — gauge invariance holds
    //     through the production write path.
    assert!(
        (t_plain.k - t_enc.k).abs() < 1e-9,
        "K must be gauge-invariant through production pipeline: plain={}, enc={}",
        t_plain.k,
        t_enc.k
    );
    assert!(
        (t_plain.lambda_1 - t_enc.lambda_1).abs() < 1e-9,
        "λ_1 must be gauge-invariant: plain={}, enc={}",
        t_plain.lambda_1,
        t_enc.lambda_1
    );
    assert!(
        (t_plain.holonomy_mean - t_enc.holonomy_mean).abs() < 1e-9,
        "⟨Hol⟩ must be gauge-invariant: plain={}, enc={}",
        t_plain.holonomy_mean,
        t_enc.holonomy_mean
    );
    assert_eq!(t_plain.record_count, t_enc.record_count, "record_count must match");
    assert_eq!(t_plain.beta_0, t_enc.beta_0, "β_0 must match");
    assert_eq!(t_plain.beta_1, t_enc.beta_1, "β_1 must match");

    // (b) End-to-end verifier acceptance: prover constructs a statement
    //     from the plaintext bundle (which they own); verifier holds
    //     only the encrypted bundle and accepts the claim.
    let statement = InvariantStatement::from_bundle(&plain, "n7_e2e");
    let result = verify_invariant_statement(
        &encrypted,
        "n7_e2e",
        &statement,
        InvariantTolerances::default(),
    );
    assert!(
        matches!(result, VerifyResult::Verified { .. }),
        "verifier must accept honest claim on encrypted bundle: {:?}",
        result
    );
}

// ───────────────────────────────────────────────────────────────────────
// N-4-INDEXED: spec-headline "same K different λ_1" via Indexed-mode
// encryption on the topology-bearing field (cross-team review Flag 2).
//
// The original N-4 swapped the headline construction for a
// record_count delta because production λ_1 doesn't see sorted-diff
// variance. This test restores the headline construction by routing
// the topology axis through EncryptionMode::Indexed: the PRF mode
// preserves value-equivalence classes on ciphertext, so the
// field-index graph topology is well-defined on the encrypted side
// AND the test exercises the real encryption write path.
// ───────────────────────────────────────────────────────────────────────

/// Build a bundle whose `category` field is INDEXED-encrypted under
/// the production PRF pipeline. `value` is plain numeric (drives K);
/// `category` drives the field-index graph topology.
fn build_indexed_encrypted_bundle(
    vals: &[f64],
    categories: &[&str],
    schema_name: &str,
    seed: [u8; 32],
) -> BundleStore {
    assert_eq!(vals.len(), categories.len());
    let mut schema = BundleSchema::new(schema_name)
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::numeric("value").with_range(1000.0))
        .fiber(
            FieldDef::categorical("category").with_encryption(EncryptionMode::Indexed),
        )
        .index("category");
    schema.gauge_key = Some(GaugeKey::derive(&seed, &schema.fiber_fields));
    let mut store = BundleStore::new(schema);
    for (i, (v, c)) in vals.iter().zip(categories.iter()).enumerate() {
        let mut rec = HashMap::new();
        rec.insert("id".to_string(), Value::Float(i as f64));
        rec.insert("value".to_string(), Value::Float(*v));
        rec.insert("category".to_string(), Value::Text(c.to_string()));
        store.insert(&rec);
    }
    store
}

/// **N-4-INDEXED**: same K, same record_count, different λ_1.
///
/// Two bundles share identical numeric value distributions (so K and
/// record_count match bit-identically) but differ on the PRF-
/// encrypted `category` field's equivalence-class structure:
///
///   - b_clique:   every record has category = "shared" → after PRF,
///                 every ciphertext is identical → 1 clique component
///                 in the field-index graph → λ_1 = n/(n-1).
///   - b_isolated: every record has a unique category → after PRF,
///                 every ciphertext is distinct → n isolated
///                 components → λ_1 = 0.
///
/// The PRF (Indexed mode) preserves the value-equivalence relation
/// on encrypted data, so the production spectral_gap (which reads
/// `field_index`) sees the topology change end-to-end through the
/// real encryption pipeline.
///
/// **Assertion target** (per cross-team review Flag 2): the verifier
/// rejects on `field == "lambda1"` specifically — not record_count or
/// K. This proves the full tuple catches the spec's headline "K is
/// gameable; λ_1 catches it" attack on the production codepath.
fn n4_indexed_inner(seed: [u8; 32]) {
    let n = 80;
    let vals: Vec<f64> = (0..n).map(|i| (i as f64) * 0.7 + 1.3).collect();

    // Clique: shared category → encrypted to a single PRF output →
    // single connected component → clique structure → λ_1 = n/(n-1).
    let cats_clique: Vec<&str> = vec!["shared"; n];
    let b_clique =
        build_indexed_encrypted_bundle(&vals, &cats_clique, "n4_idx", seed);

    // Isolated: unique categories → encrypted to n distinct PRF outputs
    // → n components → λ_1 = 0.
    let unique_cats: Vec<String> = (0..n).map(|i| format!("u_{}", i)).collect();
    let cats_iso: Vec<&str> = unique_cats.iter().map(|s| s.as_str()).collect();
    let b_isolated =
        build_indexed_encrypted_bundle(&vals, &cats_iso, "n4_idx", seed);

    let t_clique = InvariantTuple::compute(&b_clique);
    let t_isolated = InvariantTuple::compute(&b_isolated);

    // K agrees (same value distribution).
    let k_diff = (t_clique.k - t_isolated.k).abs();
    assert!(
        k_diff < 1e-9,
        "K must match bit-identically (same numeric values): clique={}, isolated={}",
        t_clique.k,
        t_isolated.k
    );

    // record_count agrees (same n).
    assert_eq!(
        t_clique.record_count, t_isolated.record_count,
        "record_count must match (same n)"
    );

    // λ_1 differs maximally: clique → n/(n-1) ≈ 1.0, isolated → 0.
    assert!(
        (t_clique.lambda_1 - t_isolated.lambda_1).abs() > 0.5,
        "λ_1 must differ on the encrypted topology: clique={}, isolated={}",
        t_clique.lambda_1,
        t_isolated.lambda_1
    );

    // Adversary substitution: prover claims b_clique's tuple; verifier
    // holds b_isolated. Because K and record_count are bit-identical
    // and ⟨Hol⟩ is 0 in both (untwisted bundle), the FIRST disagreement
    // in fingerprint order (K → λ_1 → ⟨Hol⟩ → record_count → β_0 → β_1)
    // MUST be λ_1.
    let statement = InvariantStatement::from_bundle(&b_clique, "n4_idx");
    let result = verify_invariant_statement(
        &b_isolated,
        "n4_idx",
        &statement,
        InvariantTolerances::default(),
    );
    match result {
        VerifyResult::Rejected { field, .. } => {
            assert_eq!(
                field, "lambda1",
                "spec-headline assertion: rejection field must be lambda1 \
                 (K and record_count agree by construction; ⟨Hol⟩ is 0 for \
                 both; lambda1 is the FIRST disagreement in fingerprint order). \
                 Got: {}",
                field
            );
        }
        VerifyResult::Verified { .. } => panic!(
            "same-K-different-λ_1 adversary must be caught by the full tuple \
             on the lambda_1 field"
        ),
        VerifyResult::BundleMismatch { .. } => {
            panic!("bundle_ids match — should not hit mismatch path")
        }
    }
}

#[test]
fn n4_indexed_field_lambda1_differs_seed_a() {
    let mut seed = [0u8; 32];
    for (i, b) in seed.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(13).wrapping_add(37);
    }
    n4_indexed_inner(seed);
}

#[test]
fn n4_indexed_field_lambda1_differs_seed_b() {
    let mut seed = [0u8; 32];
    for (i, b) in seed.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(53).wrapping_add(101);
    }
    n4_indexed_inner(seed);
}

#[test]
fn n7_end_to_end_via_production_encryption_pipeline_seed_a() {
    // Deterministic seed → deterministic gauge transform → deterministic test.
    let mut seed = [0u8; 32];
    for (i, b) in seed.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(7).wrapping_add(11);
    }
    n7_end_to_end_inner(seed);
}

#[test]
fn n7_end_to_end_via_production_encryption_pipeline_seed_b() {
    // A second seed exercises a different derived (a, b) — proving the
    // gauge invariance holds across the parameter space, not just for
    // one lucky seed.
    let mut seed = [0u8; 32];
    for (i, b) in seed.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(31).wrapping_add(91);
    }
    n7_end_to_end_inner(seed);
}
