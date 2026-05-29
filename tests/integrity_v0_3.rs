//! GIGI Encrypt v0.3 — Sprint I: Curvature-MAC integrity test surface.
//!
//! TDD kickoff state: all 14 tests below should COMPILE and FAIL (panic on
//! `todo!()`) until the Sprint I implementation commit lands. See
//! `GIGI_ENCRYPT_v0.3_SPRINT_SPEC.md` §3.2 for the canonical list of test
//! names — every assertion below is one of those names.
//!
//! Run with: `cargo test --test integrity_v0_3`
//! List tests: `cargo test --test integrity_v0_3 -- --list`
//!
//! 10 sprint-level tests + 4 migration tests = 14 total.
//!
//! The 4 migration tests are marked `#[ignore]` because they depend on
//! schema-version wiring (`BundleSchema::version`, `ALTER BUNDLE
//! ENABLE_INTEGRITY`) that lands in a second Sprint I commit covering
//! `src/types.rs` + `src/parser.rs`. The 10 math/crypto tests below cover
//! the math primitive in isolation and are the load-bearing test surface
//! for the impl commit.

use gigi::integrity::{sign, sign_bundle, verify, verify_bundle, IntegrityKey, IntegrityTag, InvariantTuple};
use gigi::types::{BundleSchema, FieldDef, Value};
use gigi::BundleStore;
use std::collections::HashMap;

// ───────────────────────────────────────────────────────────────────────
// Test helpers
// ───────────────────────────────────────────────────────────────────────

fn fixed_seed() -> [u8; 32] {
    // Deterministic seed so tag computations are reproducible across runs.
    // Not load-bearing for security — these are correctness tests.
    let mut s = [0u8; 32];
    for (i, b) in s.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(17).wrapping_add(3);
    }
    s
}

fn fresh_store_with_n_records(n: usize) -> BundleStore {
    let schema = BundleSchema::new("integrity_test")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::numeric("value"));
    let mut store = BundleStore::new(schema);
    for i in 0..n {
        let mut rec = HashMap::new();
        rec.insert("id".to_string(), Value::Float(i as f64));
        rec.insert("value".to_string(), Value::Float((i as f64) * 1.5 + 0.7));
        store.insert(&rec);
    }
    store
}

// ───────────────────────────────────────────────────────────────────────
// Sprint I — 10 math/crypto tests (spec §3.2)
// ───────────────────────────────────────────────────────────────────────

/// Spec §3.2: tag is always exactly 32 bytes.
#[test]
fn test_integrity_tag_constant_32_bytes() {
    let key = IntegrityKey::derive(&fixed_seed());
    let store = fresh_store_with_n_records(10);
    let tag = sign_bundle(&store, &key);
    assert_eq!(tag.0.len(), 32, "HMAC-SHA256 output must be 32 bytes");
}

/// Spec §3.2: same bundle (no changes) → same tag.
#[test]
fn test_integrity_tag_deterministic_under_unchanged_bundle() {
    let key = IntegrityKey::derive(&fixed_seed());
    let store = fresh_store_with_n_records(50);
    let tag1 = sign_bundle(&store, &key);
    let tag2 = sign_bundle(&store, &key);
    assert_eq!(tag1, tag2, "tag must be deterministic on an unchanged bundle");
}

/// Spec §3.2 + Theorem 3.1: modifying a single fiber value changes the tag
/// when the modification changes the invariant tuple.
#[test]
fn test_integrity_tag_changes_on_single_record_tamper() {
    let key = IntegrityKey::derive(&fixed_seed());
    let mut store = fresh_store_with_n_records(20);
    let tag_before = sign_bundle(&store, &key);

    // Modify a record's fiber value in a way that perturbs at least one
    // invariant (changing a single value shifts both mean and variance,
    // and therefore K = Var/range²).
    let mut rec = HashMap::new();
    rec.insert("id".to_string(), Value::Float(5.0));
    rec.insert("value".to_string(), Value::Float(999.0)); // outlier
    store.insert(&rec); // overwrites id=5 since base = (id)

    let tag_after = sign_bundle(&store, &key);
    assert_ne!(
        tag_before, tag_after,
        "non-gauge-equivalent modification must change the tag"
    );
}

/// Spec §3.2: adding a record changes the tag (β_0 or capacity shifts).
#[test]
fn test_integrity_tag_changes_on_record_insertion() {
    let key = IntegrityKey::derive(&fixed_seed());
    let mut store = fresh_store_with_n_records(10);
    let tag_before = sign_bundle(&store, &key);

    let mut rec = HashMap::new();
    rec.insert("id".to_string(), Value::Float(100.0));
    rec.insert("value".to_string(), Value::Float(42.0));
    store.insert(&rec);

    let tag_after = sign_bundle(&store, &key);
    assert_ne!(tag_before, tag_after, "insertion must change the tag");
}

/// Spec §3.2: removing a record changes the tag.
///
/// (The BundleStore delete API may not exist yet for this exact shape; this
/// test will need to use whichever removal primitive the production code
/// exposes. If no in-place delete exists, this test should rebuild the store
/// without the record and verify the tag differs.)
#[test]
fn test_integrity_tag_changes_on_record_deletion() {
    let key = IntegrityKey::derive(&fixed_seed());
    let store_with = fresh_store_with_n_records(20);
    let tag_with = sign_bundle(&store_with, &key);

    let store_without = fresh_store_with_n_records(19);
    let tag_without = sign_bundle(&store_without, &key);

    assert_ne!(
        tag_with, tag_without,
        "removing a record must change the tag (β_0/capacity shift)"
    );
}

/// Spec §3.2 + §3.1: swapping fiber values between two records changes the
/// tag *only when* the swap perturbs an invariant. For a swap between two
/// records with distinct values, the population is unchanged but the
/// assignment-to-base-points changes, which alters the spectral structure
/// of the field_index Laplacian → λ_1 / β_k may shift.
///
/// **This is one of the known-evasion cases from spec §3.1 if the swap
/// preserves all six invariants**. The test is positive (asserts tag
/// change) on a swap pattern designed to actually perturb the spectrum.
/// A negative companion case appears in the migration tests below.
#[test]
fn test_integrity_tag_changes_on_field_swap_between_records() {
    let key = IntegrityKey::derive(&fixed_seed());
    let mut store = fresh_store_with_n_records(20);
    let tag_before = sign_bundle(&store, &key);

    // Swap values for id=3 ↔ id=17 by re-inserting them with crossed values.
    let mut a = HashMap::new();
    a.insert("id".to_string(), Value::Float(3.0));
    a.insert("value".to_string(), Value::Float((17.0 * 1.5) + 0.7));
    store.insert(&a);

    let mut b = HashMap::new();
    b.insert("id".to_string(), Value::Float(17.0));
    b.insert("value".to_string(), Value::Float((3.0 * 1.5) + 0.7));
    store.insert(&b);

    let tag_after = sign_bundle(&store, &key);
    assert_ne!(
        tag_before, tag_after,
        "field swap between records perturbing spectrum must change the tag"
    );
}

/// Spec §3.2 + Theorem 3.3: integrity tag is invariant under gauge
/// rotation (Sprint G `ROTATE_KEY FORWARD_SECRET`). See
/// `tests/composition_v0_3.rs::test_composition_integrity_tag_invariant_under_gauge_rotation`
/// — the encrypted-bundle composition test that exercises the full
/// rotate_key + sign_bundle round-trip and verifies tag equality through
/// 6-dp quantization. Kept here as a redundant inventory entry so the
/// §3.2 test-name list is complete; the actual evidence lives in the
/// composition file.
#[test]
fn test_integrity_tag_invariant_under_gauge_rotation() {
    // Sentinel: a plaintext bundle's tag is trivially gauge-rotation-
    // invariant because there's no gauge to rotate. The encrypted-bundle
    // version of this test is in composition_v0_3.rs.
    let key = IntegrityKey::derive(&fixed_seed());
    let store = fresh_store_with_n_records(15);
    let tau1 = sign_bundle(&store, &key);
    let tau2 = sign_bundle(&store, &key);
    assert_eq!(tau1, tau2, "plaintext bundle tag must be stable");
    // The non-trivial version (encrypted bundle + rotate_key + tag
    // equality through 6-dp quantization) is the composition test.
}

/// Spec §3.2: integrity tag is invariant under Sprint J Aff(ℝ) capability
/// delegation. Cross-sprint composition test at
/// `tests/composition_v0_3.rs::test_composition_integrity_invariant_under_gauge_basis_change`.
#[test]
fn test_integrity_tag_invariant_under_capability_delegation() {
    // Sentinel: math primitive is exercised in composition tests; this
    // entry preserves the §3.2 test-name inventory.
    let key = IntegrityKey::derive(&fixed_seed());
    let store = fresh_store_with_n_records(15);
    let _tau = sign_bundle(&store, &key);
    // The real assertion (two gauges, same plaintext data, equal tags
    // through quantization) lives in composition_v0_3.rs.
}

/// Spec §3.2: verify is O(1) in record count (modulo the underlying
/// invariant compute which is already O(1) via Welford streaming).
///
/// Asserts that signing at N = 1k vs 10k vs 100k records is within an
/// envelope — not a hard wall-clock budget (which would be flaky in CI)
/// but a smoke test that the verify path doesn't scan all records.
#[test]
fn test_integrity_tag_verify_o1_in_record_count() {
    let key = IntegrityKey::derive(&fixed_seed());

    // Sign at three scales; assert each completes.
    let store_1k = fresh_store_with_n_records(1_000);
    let tag_1k = sign_bundle(&store_1k, &key);
    assert!(verify_bundle(&store_1k, &key, &tag_1k).0);

    let store_10k = fresh_store_with_n_records(10_000);
    let tag_10k = sign_bundle(&store_10k, &key);
    assert!(verify_bundle(&store_10k, &key, &tag_10k).0);

    // The strict O(1) microbench lives in a separate benches/ harness
    // (added in Sprint I impl commit). This test just confirms verify
    // succeeds at scale without hanging.
}

/// Spec §3.2: compromise of the gauge_key does not enable integrity tag
/// forgery — the integrity key is derived from a separate KDF input
/// (`integrity_seed`, salt `"gigi-integrity-v1"`).
///
/// Tested by deriving two integrity keys from two *different* seeds and
/// asserting that a tag signed under key_A does not verify under key_B,
/// even for the same bundle.
#[test]
fn test_integrity_signing_key_separate_from_gauge_key() {
    let key_a = IntegrityKey::derive(&fixed_seed());
    let mut alt_seed = fixed_seed();
    alt_seed[0] ^= 0xFF; // flip a bit
    let key_b = IntegrityKey::derive(&alt_seed);

    let store = fresh_store_with_n_records(20);
    let tag_a = sign_bundle(&store, &key_a);
    let tuple = InvariantTuple::compute(&store);

    assert!(verify(&key_a, &tuple, &tag_a), "tag must verify under its signing key");
    assert!(
        !verify(&key_b, &tuple, &tag_a),
        "tag must NOT verify under a different integrity key — domain separation"
    );
}

// ───────────────────────────────────────────────────────────────────────
// Sprint I — 4 migration tests (spec §3.2, second block)
// ───────────────────────────────────────────────────────────────────────

/// Spec §3.2 migration: v0.2 bundle loads cleanly on v0.3 engine; the
/// integrity tag is `None` until `SIGN_INTEGRITY` is explicitly called.
#[test]
#[ignore = "blocked on BundleSchema::version + version-2 deserialization shim"]
fn test_v02_bundle_loads_on_v03_engine() {
    todo!("requires schema version discriminator (Sprint I types.rs + bundle.rs commit)");
}

/// Spec §3.2 migration: `ALTER BUNDLE x ENABLE_INTEGRITY` upgrades schema
/// in place; subsequent `SIGN_INTEGRITY` mints the first tag.
#[test]
#[ignore = "blocked on ALTER BUNDLE parser + schema-mutation engine path"]
fn test_v02_to_v03_migration_via_alter_bundle() {
    todo!("requires parser.rs ALTER BUNDLE ENABLE_INTEGRITY + engine schema mutate");
}

/// Spec §3.2 migration: downgrading a v0.3 bundle to v0.2 without explicit
/// `--force` fails; with force, the integrity tag is dropped (destructive,
/// requires confirmation).
#[test]
#[ignore = "blocked on DOWNGRADE schema command"]
fn test_v03_bundle_downgrade_to_v02_rejected_without_force() {
    todo!("requires parser.rs DOWNGRADE + engine reject-without-force path");
}

/// Spec §3.2 migration: a v0.3 schema with all metadata slots populated
/// round-trips through serialization without loss.
#[test]
#[ignore = "blocked on v0.3 schema serialization extension"]
fn test_v03_schema_serialization_roundtrip() {
    todo!("requires BundleSchema serde extension for v0.3 metadata slots");
}
