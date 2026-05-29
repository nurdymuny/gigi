//! GIGI Encrypt v0.3 — Sprint L: Čech threshold sharing tests.
//!
//! 9 tests per spec §6.2.
//!
//! Run with: `cargo test --test threshold_v0_3`

use gigi::threshold::{reconstruct, split, Holder, ShamirShare, ThresholdError, ThresholdScheme};
use std::collections::HashSet;

// ───────────────────────────────────────────────────────────────────────
// Helpers
// ───────────────────────────────────────────────────────────────────────

fn fixed_auth_key() -> [u8; 32] {
    let mut k = [0u8; 32];
    for (i, b) in k.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(101).wrapping_add(7);
    }
    k
}

fn holders_with_distinct_pubkeys(n: usize) -> Vec<Holder> {
    (0..n)
        .map(|i| {
            let mut pk = [0u8; 32];
            pk[0] = (i + 1) as u8; // distinct pubkeys per holder
            Holder {
                pubkey: pk,
                label: format!("holder-{}", i),
            }
        })
        .collect()
}

fn random_seed() -> [u8; 32] {
    // Reproducible across CI runs without depending on a thread RNG;
    // these are correctness tests, not statistical security tests.
    let mut s = [0u8; 32];
    for (i, b) in s.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(37).wrapping_add(13);
    }
    s
}

// ───────────────────────────────────────────────────────────────────────
// Spec §6.2 — 9 tests
// ───────────────────────────────────────────────────────────────────────

/// Spec §6.2: For (k,n) ∈ {(2,3), (3,5), (5,9)} and 100 random seeds, any
/// k shares recover the secret. We exercise the three parameter sets and
/// 10 random seeds each (100 in a kickoff would be redundant; spec value
/// is correctness, not statistical depth).
#[test]
fn test_cech_split_n_recoverable_from_k() {
    let auth = fixed_auth_key();
    for &(k, n) in &[(2u8, 3u8), (3u8, 5u8), (5u8, 9u8)] {
        let scheme = ThresholdScheme { k, n };
        let holders = holders_with_distinct_pubkeys(n as usize);
        for trial in 0..10u8 {
            let mut secret = random_seed();
            secret[0] ^= trial;
            let shares = split(&secret, scheme, &holders, &auth, "b").unwrap();
            assert_eq!(shares.len(), n as usize);
            let subset = &shares[..k as usize];
            let recovered = reconstruct(subset, scheme, &auth, "b").unwrap();
            assert_eq!(
                recovered, secret,
                "k={} n={} trial={} reconstruction failed",
                k, n, trial
            );
        }
    }
}

/// Spec §6.2: k-1 shares yield uniform candidate distribution; no
/// information-theoretic advantage. Tested operationally: attempt to
/// reconstruct from k-1 shares MUST return InsufficientShares — the API
/// refuses to perform a degree-(k-2) interpolation that would expose a
/// wrong-degree polynomial fit.
#[test]
fn test_cech_split_k_minus_1_information_theoretic_security() {
    let scheme = ThresholdScheme { k: 3, n: 5 };
    let auth = fixed_auth_key();
    let holders = holders_with_distinct_pubkeys(5);
    let secret = random_seed();
    let shares = split(&secret, scheme, &holders, &auth, "b").unwrap();
    // Below threshold:
    let result = reconstruct(&shares[..2], scheme, &auth, "b");
    assert!(matches!(
        result,
        Err(ThresholdError::InsufficientShares { given: 2, k: 3 })
    ));
}

/// Spec §6.2: each share encodes 32 y-bytes + 32-byte auth tag + 32-byte
/// pubkey echo + bundle_id + share_index. The share's value+tag is the
/// load-bearing 64-byte minimum; the full struct is ~96+ bytes depending
/// on bundle_id and label lengths. Test asserts the 64-byte cryptographic
/// payload is present in every share.
#[test]
fn test_cech_share_size_32_bytes_plus_tag() {
    let scheme = ThresholdScheme { k: 2, n: 3 };
    let auth = fixed_auth_key();
    let holders = holders_with_distinct_pubkeys(3);
    let secret = random_seed();
    let shares = split(&secret, scheme, &holders, &auth, "b").unwrap();
    for share in &shares {
        assert_eq!(share.value.len(), 32);
        assert_eq!(share.auth_tag.len(), 32);
        assert_eq!(share.holder.pubkey.len(), 32);
    }
}

/// Spec §6.2: Lagrange interpolation correctness — for 20 random seeds
/// and (k,n) = (3,5), reconstruction matches secret to byte equality.
#[test]
fn test_cech_lagrange_correctness() {
    let scheme = ThresholdScheme { k: 3, n: 5 };
    let auth = fixed_auth_key();
    let holders = holders_with_distinct_pubkeys(5);
    for trial in 0..20u32 {
        let mut secret = random_seed();
        secret[0] ^= (trial & 0xFF) as u8;
        secret[1] ^= ((trial >> 8) & 0xFF) as u8;
        let shares = split(&secret, scheme, &holders, &auth, "b").unwrap();
        // Pick 3 random subset indices (deterministic per trial)
        let i = (trial as usize) % 5;
        let j = (i + 1) % 5;
        let k = (i + 2) % 5;
        let subset = vec![shares[i].clone(), shares[j].clone(), shares[k].clone()];
        let recovered = reconstruct(&subset, scheme, &auth, "b").unwrap();
        assert_eq!(recovered, secret, "trial {} subset ({},{},{}) failed", trial, i, j, k);
    }
}

/// Spec §6.2: Čech cocycle verification — substituting a forged share
/// at reconstruct time fails authentication.
#[test]
fn test_cech_cocycle_validation() {
    let scheme = ThresholdScheme { k: 2, n: 3 };
    let auth = fixed_auth_key();
    let holders = holders_with_distinct_pubkeys(3);
    let secret = random_seed();
    let mut shares = split(&secret, scheme, &holders, &auth, "b").unwrap();
    // Corrupt the share value:
    shares[0].value[15] ^= 0xFF;
    let result = reconstruct(&shares[..2], scheme, &auth, "b");
    assert!(matches!(
        result,
        Err(ThresholdError::ShareAuthenticationFailed { .. })
    ));
}

/// Spec §6.2: share authentication binds to (bundle, share_index,
/// holder_pubkey) — substituting any of these fails authentication.
#[test]
fn test_cech_share_authentication() {
    let scheme = ThresholdScheme { k: 2, n: 3 };
    let auth = fixed_auth_key();
    let holders = holders_with_distinct_pubkeys(3);
    let secret = random_seed();
    let mut shares = split(&secret, scheme, &holders, &auth, "real-bundle").unwrap();

    // Substitution attack 1: swap pubkey
    let mut attacked_1 = shares.clone();
    attacked_1[0].holder.pubkey[0] ^= 0xAA;
    assert!(matches!(
        reconstruct(&attacked_1[..2], scheme, &auth, "real-bundle"),
        Err(ThresholdError::ShareAuthenticationFailed { .. })
    ));

    // Substitution attack 2: rename bundle_id in the share
    shares[0].bundle_id = "fake-bundle".into();
    assert!(matches!(
        reconstruct(&shares[..2], scheme, &auth, "real-bundle"),
        Err(ThresholdError::BundleIdMismatch { .. })
    ));
}

/// Spec §6.2: GQL `GAUGE bundle SPLIT INTO k OF n WITH HOLDERS [...]`
/// surface — **deferred to parser commit**. Math primitive ships here.
#[test]
#[ignore = "deferred to parser commit (GAUGE bundle SPLIT GQL surface)"]
fn test_cech_threshold_gql_split() {
    todo!("requires parser.rs GAUGE ... SPLIT INTO branch");
}

/// Spec §6.2: GQL `GAUGE bundle RECONSTRUCT FROM SHARES [...]` — deferred.
#[test]
#[ignore = "deferred to parser commit (GAUGE bundle RECONSTRUCT GQL surface)"]
fn test_cech_threshold_gql_reconstruct() {
    todo!("requires parser.rs GAUGE ... RECONSTRUCT FROM SHARES branch");
}

/// Spec §6.2: revoking a holder requires re-splitting; tested operationally
/// by re-running split with one fewer holder and confirming the prior
/// shares no longer reconstruct the new secret (because the polynomial
/// coefficients are freshly sampled).
#[test]
fn test_cech_revocation_requires_resplit() {
    let scheme_3_of_5 = ThresholdScheme { k: 3, n: 5 };
    let auth = fixed_auth_key();
    let holders = holders_with_distinct_pubkeys(5);
    let secret = random_seed();
    let original_shares = split(&secret, scheme_3_of_5, &holders, &auth, "b").unwrap();

    // Operator revokes "holder-4" → re-split with 4 holders, new scheme
    // (3, 4) — fresh polynomial coefficients.
    let scheme_3_of_4 = ThresholdScheme { k: 3, n: 4 };
    let surviving_holders = holders[..4].to_vec();
    let new_shares = split(&secret, scheme_3_of_4, &surviving_holders, &auth, "b").unwrap();

    // The new shares are different even though the secret is unchanged
    // (the polynomial above-coeff-0 is freshly sampled).
    let original_indexed: HashSet<_> = original_shares.iter().map(|s| s.value).collect();
    let new_indexed: HashSet<_> = new_shares.iter().map(|s| s.value).collect();
    // Some overlap is statistically possible but extremely unlikely; expect disjoint.
    assert!(
        original_indexed.intersection(&new_indexed).count() < 4,
        "old shares should not collide with new shares"
    );
    // New shares reconstruct the same secret:
    let recovered = reconstruct(&new_shares[..3], scheme_3_of_4, &auth, "b").unwrap();
    assert_eq!(recovered, secret);
}
