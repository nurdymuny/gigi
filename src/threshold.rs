//! GIGI Encrypt v0.3 — Sprint L: Čech threshold sharing (k-of-n decrypt).
//!
//! Shamir secret sharing (Shamir 1979) over the **secp256k1 base field**
//! `F_p` with `p = 2^256 − 2^32 − 977`, framed as Čech reconstruction on a
//! presheaf over the share-holder cover.
//!
//! **Security**:
//! - Theorem 6.1: information-theoretic privacy against any (k − 1) coalition.
//! - Theorem 6.2: Čech-cocycle authentication binds each share to
//!   `(bundle_id, share_index, holder_pubkey, value)` via HMAC-SHA256.
//!   Substituting a forged share at reconstruct time fails with probability
//!   `1 - 2^{-256}`.
//!
//! See `GIGI_ENCRYPT_v0.3_SPRINT_SPEC.md` §6.

use hmac::{Hmac, Mac};
use num_bigint::BigUint;
use num_traits::{One, Zero};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

// ───────────────────────────────────────────────────────────────────────
// secp256k1 base field prime
// p = 2^256 − 2^32 − 977
// = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEFFFFFC2F
// ───────────────────────────────────────────────────────────────────────

/// secp256k1 base field prime, hex-encoded.
const SECP256K1_BASE_PRIME_HEX: &str =
    "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEFFFFFC2F";

fn field_prime() -> BigUint {
    BigUint::parse_bytes(SECP256K1_BASE_PRIME_HEX.as_bytes(), 16)
        .expect("secp256k1 base prime is a valid 64-char hex string")
}

// ───────────────────────────────────────────────────────────────────────
// Types
// ───────────────────────────────────────────────────────────────────────

/// A share-holder identity. Pubkey is locked into the share's auth tag
/// at SPLIT time (spec §6.4) — substituting a forged share with a
/// different pubkey will fail authentication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Holder {
    pub pubkey: [u8; 32],
    pub label: String,
}

/// A single Shamir share with its Čech-cocycle authentication tag.
#[derive(Debug, Clone)]
pub struct ShamirShare {
    pub bundle_id: String,
    pub holder: Holder,
    pub share_index: u8, // 1..n; the x-coordinate (0 is reserved for the secret)
    pub value: [u8; 32], // y-coordinate, big-endian encoding of an Fp element
    pub auth_tag: [u8; 32], // HMAC-SHA256 binding to (bundle_id, share_index, pubkey, value)
}

/// A (k, n) threshold scheme parameterization.
#[derive(Debug, Clone, Copy)]
pub struct ThresholdScheme {
    pub k: u8,
    pub n: u8,
}

#[derive(Debug, thiserror::Error)]
pub enum ThresholdError {
    #[error("threshold parameters invalid: k={k} must satisfy 1 <= k <= n <= 255, got n={n}")]
    InvalidScheme { k: u8, n: u8 },
    #[error("holder count {given} does not match n={n}")]
    HolderCountMismatch { given: usize, n: u8 },
    #[error("share count {given} below threshold k={k}")]
    InsufficientShares { given: usize, k: u8 },
    #[error("share authentication failed at index {share_index} (holder {holder_label})")]
    ShareAuthenticationFailed {
        share_index: u8,
        holder_label: String,
    },
    #[error("duplicate share index {0} among supplied shares")]
    DuplicateShareIndex(u8),
    #[error("share carries unexpected bundle_id (expected '{expected}', got '{got}')")]
    BundleIdMismatch { expected: String, got: String },
}

// ───────────────────────────────────────────────────────────────────────
// Public API
// ───────────────────────────────────────────────────────────────────────

/// Split a 32-byte secret into `n` Shamir shares with reconstruction
/// threshold `k`.
///
/// The polynomial `P(x) = s + c_1·x + c_2·x^2 + ... + c_{k-1}·x^{k-1}` has
/// degree `k-1`. The shares are `(i, P(i)) mod p` for `i = 1, 2, ..., n`.
///
/// Each share is authenticated with HMAC-SHA256 binding to the
/// `(bundle_id, share_index, holder.pubkey, value)` tuple under `auth_key`.
pub fn split(
    seed: &[u8; 32],
    scheme: ThresholdScheme,
    holders: &[Holder],
    auth_key: &[u8; 32],
    bundle_id: &str,
) -> Result<Vec<ShamirShare>, ThresholdError> {
    validate_scheme(scheme)?;
    if holders.len() != scheme.n as usize {
        return Err(ThresholdError::HolderCountMismatch {
            given: holders.len(),
            n: scheme.n,
        });
    }
    let p = field_prime();
    let secret = BigUint::from_bytes_be(seed) % &p;

    // Sample (k − 1) random coefficients for the polynomial.
    let mut coeffs: Vec<BigUint> = vec![secret];
    for _ in 1..scheme.k {
        let c = gen_field_element(&p);
        coeffs.push(c);
    }

    // Evaluate at x = 1, 2, ..., n.
    let mut shares = Vec::with_capacity(scheme.n as usize);
    for (i, holder) in holders.iter().enumerate() {
        let share_index = (i + 1) as u8; // 1-indexed
        let x = BigUint::from(share_index as u64);
        let y = poly_eval(&coeffs, &x, &p);
        let value = biguint_to_32_be(&y);
        let auth_tag = compute_auth_tag(auth_key, bundle_id, share_index, &holder.pubkey, &value);
        shares.push(ShamirShare {
            bundle_id: bundle_id.to_string(),
            holder: holder.clone(),
            share_index,
            value,
            auth_tag,
        });
    }
    Ok(shares)
}

/// Reconstruct the 32-byte secret from `≥ k` authenticated shares.
///
/// Verifies each share's auth tag, then runs Lagrange interpolation at
/// `x = 0` to recover the constant term of the polynomial (= the secret).
pub fn reconstruct(
    shares: &[ShamirShare],
    scheme: ThresholdScheme,
    auth_key: &[u8; 32],
    bundle_id: &str,
) -> Result<[u8; 32], ThresholdError> {
    validate_scheme(scheme)?;
    if shares.len() < scheme.k as usize {
        return Err(ThresholdError::InsufficientShares {
            given: shares.len(),
            k: scheme.k,
        });
    }
    // Authenticate every share + check duplicates + bundle_id consistency.
    let mut seen_indices = std::collections::HashSet::new();
    for share in shares {
        if share.bundle_id != bundle_id {
            return Err(ThresholdError::BundleIdMismatch {
                expected: bundle_id.to_string(),
                got: share.bundle_id.clone(),
            });
        }
        let expected = compute_auth_tag(
            auth_key,
            bundle_id,
            share.share_index,
            &share.holder.pubkey,
            &share.value,
        );
        if !constant_time_eq(&expected, &share.auth_tag) {
            return Err(ThresholdError::ShareAuthenticationFailed {
                share_index: share.share_index,
                holder_label: share.holder.label.clone(),
            });
        }
        if !seen_indices.insert(share.share_index) {
            return Err(ThresholdError::DuplicateShareIndex(share.share_index));
        }
    }
    // Lagrange interpolation at x=0 over the first k shares.
    let p = field_prime();
    let points: Vec<(BigUint, BigUint)> = shares
        .iter()
        .take(scheme.k as usize)
        .map(|s| {
            (
                BigUint::from(s.share_index as u64),
                BigUint::from_bytes_be(&s.value),
            )
        })
        .collect();
    let secret = lagrange_at_zero(&points, &p);
    Ok(biguint_to_32_be(&secret))
}

// ───────────────────────────────────────────────────────────────────────
// Internals
// ───────────────────────────────────────────────────────────────────────

fn validate_scheme(scheme: ThresholdScheme) -> Result<(), ThresholdError> {
    if scheme.k == 0 || scheme.n == 0 || scheme.k > scheme.n {
        return Err(ThresholdError::InvalidScheme {
            k: scheme.k,
            n: scheme.n,
        });
    }
    Ok(())
}

fn poly_eval(coeffs: &[BigUint], x: &BigUint, p: &BigUint) -> BigUint {
    // Horner's method: ((c_{k-1} * x + c_{k-2}) * x + ... + c_1) * x + c_0
    let mut acc = BigUint::zero();
    for c in coeffs.iter().rev() {
        acc = (&acc * x + c) % p;
    }
    acc
}

fn lagrange_at_zero(points: &[(BigUint, BigUint)], p: &BigUint) -> BigUint {
    // L(0) = Σ y_i · Π (−x_j / (x_i − x_j))
    let mut total = BigUint::zero();
    for i in 0..points.len() {
        let (xi, yi) = &points[i];
        let mut numer = BigUint::one();
        let mut denom = BigUint::one();
        for j in 0..points.len() {
            if i == j {
                continue;
            }
            let xj = &points[j].0;
            // numer *= (0 - xj) mod p  =  (p - xj)
            let neg_xj = if xj.is_zero() {
                BigUint::zero()
            } else {
                p - (xj % p)
            };
            numer = (&numer * &neg_xj) % p;
            // denom *= (xi - xj) mod p
            let diff = mod_sub(xi, xj, p);
            denom = (&denom * &diff) % p;
        }
        let denom_inv = mod_inverse(&denom, p);
        let term = (yi * numer % p * denom_inv) % p;
        total = (&total + &term) % p;
    }
    total
}

fn mod_sub(a: &BigUint, b: &BigUint, p: &BigUint) -> BigUint {
    let a_mod = a % p;
    let b_mod = b % p;
    if a_mod >= b_mod {
        a_mod - b_mod
    } else {
        p - (b_mod - a_mod)
    }
}

/// Modular inverse via Fermat's little theorem: a^(p-2) mod p.
/// Valid for prime p and nonzero a.
fn mod_inverse(a: &BigUint, p: &BigUint) -> BigUint {
    let exp = p - BigUint::from(2u64);
    a.modpow(&exp, p)
}

fn biguint_to_32_be(x: &BigUint) -> [u8; 32] {
    let bytes = x.to_bytes_be();
    let mut out = [0u8; 32];
    // Left-pad with zeros if shorter than 32 bytes.
    let offset = 32usize.saturating_sub(bytes.len());
    out[offset..].copy_from_slice(&bytes[bytes.len().saturating_sub(32)..]);
    out
}

fn compute_auth_tag(
    auth_key: &[u8; 32],
    bundle_id: &str,
    share_index: u8,
    pubkey: &[u8; 32],
    value: &[u8; 32],
) -> [u8; 32] {
    let mut mac = <HmacSha256 as Mac>::new_from_slice(auth_key).expect("HMAC accepts any key");
    mac.update(bundle_id.as_bytes());
    mac.update(&[0u8]); // separator
    mac.update(&[share_index]);
    mac.update(pubkey);
    mac.update(value);
    let result = mac.finalize().into_bytes();
    let mut tag = [0u8; 32];
    tag.copy_from_slice(&result);
    tag
}

fn constant_time_eq(a: &[u8; 32], b: &[u8; 32]) -> bool {
    let mut diff = 0u8;
    for i in 0..32 {
        diff |= a[i] ^ b[i];
    }
    diff == 0
}

/// Sample a uniformly random Fp element via OS entropy + rejection.
///
/// For secp256k1's prime p (very close to 2^256), the rejection
/// probability per draw is ~2^-32 → on average <1.0000001 retries.
/// Constant-time would matter for ECDSA signing; for Shamir secret
/// sharing the timing channel is irrelevant (sampled coefficients
/// are themselves secrets, not key material exercised in a public
/// algorithm).
fn gen_field_element(p: &BigUint) -> BigUint {
    loop {
        let mut buf = [0u8; 32];
        getrandom::getrandom(&mut buf).expect("OS RNG must be available");
        let candidate = BigUint::from_bytes_be(&buf);
        if candidate < *p {
            return candidate;
        }
    }
}

// ───────────────────────────────────────────────────────────────────────
// Unit tests — math primitive in isolation.
// ───────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed_auth_key() -> [u8; 32] {
        let mut k = [0u8; 32];
        for (i, b) in k.iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(31).wrapping_add(7);
        }
        k
    }

    fn fixed_seed(byte: u8) -> [u8; 32] {
        [byte; 32]
    }

    fn n_holders(n: usize) -> Vec<Holder> {
        (0..n)
            .map(|i| Holder {
                pubkey: [(i as u8).wrapping_mul(13); 32],
                label: format!("holder-{}", i),
            })
            .collect()
    }

    #[test]
    fn split_then_reconstruct_3_of_5() {
        let secret = fixed_seed(0xAA);
        let scheme = ThresholdScheme { k: 3, n: 5 };
        let auth = fixed_auth_key();
        let holders = n_holders(5);
        let shares = split(&secret, scheme, &holders, &auth, "test-bundle").unwrap();
        // Use the first 3 to reconstruct.
        let recovered = reconstruct(&shares[..3], scheme, &auth, "test-bundle").unwrap();
        assert_eq!(recovered, secret);
    }

    #[test]
    fn split_then_reconstruct_with_different_share_subsets() {
        let secret = fixed_seed(0x42);
        let scheme = ThresholdScheme { k: 3, n: 5 };
        let auth = fixed_auth_key();
        let holders = n_holders(5);
        let shares = split(&secret, scheme, &holders, &auth, "b").unwrap();
        // Subset {0, 1, 4}:
        let subset_a: Vec<_> = [&shares[0], &shares[1], &shares[4]].iter().map(|s| (*s).clone()).collect();
        let r_a = reconstruct(&subset_a, scheme, &auth, "b").unwrap();
        // Subset {2, 3, 4}:
        let subset_b: Vec<_> = [&shares[2], &shares[3], &shares[4]].iter().map(|s| (*s).clone()).collect();
        let r_b = reconstruct(&subset_b, scheme, &auth, "b").unwrap();
        // Both must equal the original secret.
        assert_eq!(r_a, secret);
        assert_eq!(r_b, secret);
    }

    #[test]
    fn forged_share_fails_authentication() {
        let secret = fixed_seed(0x10);
        let scheme = ThresholdScheme { k: 2, n: 3 };
        let auth = fixed_auth_key();
        let holders = n_holders(3);
        let mut shares = split(&secret, scheme, &holders, &auth, "b").unwrap();
        // Tamper with one share's value (but leave auth_tag intact):
        shares[0].value[0] ^= 0xFF;
        let result = reconstruct(&shares[..2], scheme, &auth, "b");
        assert!(matches!(
            result,
            Err(ThresholdError::ShareAuthenticationFailed { .. })
        ));
    }

    #[test]
    fn share_substitution_with_different_pubkey_fails() {
        let secret = fixed_seed(0x10);
        let scheme = ThresholdScheme { k: 2, n: 3 };
        let auth = fixed_auth_key();
        let holders = n_holders(3);
        let mut shares = split(&secret, scheme, &holders, &auth, "b").unwrap();
        // Tamper with the holder pubkey:
        shares[0].holder.pubkey[0] ^= 0xFF;
        let result = reconstruct(&shares[..2], scheme, &auth, "b");
        assert!(matches!(
            result,
            Err(ThresholdError::ShareAuthenticationFailed { .. })
        ));
    }

    #[test]
    fn insufficient_shares_below_threshold_rejected() {
        let secret = fixed_seed(0x99);
        let scheme = ThresholdScheme { k: 3, n: 5 };
        let auth = fixed_auth_key();
        let holders = n_holders(5);
        let shares = split(&secret, scheme, &holders, &auth, "b").unwrap();
        let result = reconstruct(&shares[..2], scheme, &auth, "b");
        assert!(matches!(
            result,
            Err(ThresholdError::InsufficientShares { given: 2, k: 3 })
        ));
    }

    #[test]
    fn cross_bundle_share_rejected() {
        let secret = fixed_seed(0xCC);
        let scheme = ThresholdScheme { k: 2, n: 3 };
        let auth = fixed_auth_key();
        let holders = n_holders(3);
        let shares = split(&secret, scheme, &holders, &auth, "bundle-A").unwrap();
        // Try to reconstruct claiming the shares are for a different bundle:
        let result = reconstruct(&shares[..2], scheme, &auth, "bundle-B");
        assert!(matches!(
            result,
            Err(ThresholdError::BundleIdMismatch { .. })
        ));
    }

    #[test]
    fn invalid_scheme_rejected() {
        let secret = fixed_seed(0x00);
        let auth = fixed_auth_key();
        let holders = n_holders(2);
        // k > n
        let r = split(
            &secret,
            ThresholdScheme { k: 3, n: 2 },
            &holders,
            &auth,
            "b",
        );
        assert!(matches!(r, Err(ThresholdError::InvalidScheme { .. })));
    }

    #[test]
    fn modular_inverse_correct_for_known_values() {
        // Quick sanity on a small prime: 3 * 5 ≡ 1 mod 7 → inv(3) = 5
        let p = BigUint::from(7u64);
        let a = BigUint::from(3u64);
        let inv = mod_inverse(&a, &p);
        assert_eq!(inv, BigUint::from(5u64));
        // 3 * 5 mod 7
        assert_eq!((&a * &inv) % &p, BigUint::one());
    }
}
