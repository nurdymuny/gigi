//! GIGI Encrypt v0.3.x — Sprint J.4: Lattice-based threshold delegation
//! (PQ + collusion-resistant).
//!
//! ## Construction overview
//!
//! Two-layer composition of audited primitives:
//!
//! 1. **Inner layer**: Shamir secret sharing over the secp256k1 base
//!    field $\mathbb{F}_p$ (Sprint L). Alice's payload (e.g.\ her
//!    gauge key bytes) is split into $K$-of-$N$ shares. Information-
//!    theoretic security: any $K-1$ shares reveal zero information.
//! 2. **Outer layer**: each share is delivered to a distinct
//!    shareholder via ML-KEM-768 KEM + AES-256-GCM-SIV AEAD (Sprint
//!    J.3). Post-quantum IND-CCA secure under MLWE.
//!
//! The construction sits on the security spectrum:
//!
//! | Mode | Trust model | Quantum | Collusion |
//! |---|---|---|---|
//! | Aff$(\mathbb{R})$ (Sprint J.1) | trusted delegatee | pre-quantum (algebraic) | $O(1)$ algebraic recovery |
//! | BLS12-381 (Sprint J.2) | proxy alone OK | pre-quantum (ECDLP via Shor) | DLP-hard recovery |
//! | ML-KEM trusted (Sprint J.3) | trusted delegatee | post-quantum | not applicable (single party) |
//! | **Threshold lattice (this module)** | $K$-of-$N$ shareholder cooperation | **post-quantum** | **info-theoretic on $\leq K-1$ subsets** |
//!
//! ## Why this is collusion-resistant in a meaningful sense
//!
//! "Collusion-resistant" classically means *delegatee + capability
//! cannot recover delegator's secret* (Ateniese--Hohenberger 2005).
//! For pairing-based PRE this is achieved via DLP hardness on $G_2$.
//! For our threshold construction it is achieved \emph{structurally}:
//!
//! - There is no "single delegatee" who holds enough material to
//!   recover Alice's secret. Any subset of size $\leq K - 1$ reveals
//!   information-theoretically zero — this is stronger than DLP
//!   hardness.
//! - The intended unlock path requires $K$ cooperating shareholders
//!   to assemble shares. This is a *separate* primitive from
//!   "delegate to a single Bob"; both can be useful and we ship both.
//!
//! For deployments where strict "Alice $\to$ single Bob" delegation
//! is required with collusion-resistance + post-quantum, a true
//! lattice-PRE construction (Kirshanova 2014, Aono--Hayashi 2017) is
//! the right primitive; implementation of those is a v0.4 sprint.
//! This module ships the *threshold-PQ* primitive today as the
//! collusion-resistant PQ option.
//!
//! ## Cryptographic strength summary
//!
//! - **Shamir over $\mathbb{F}_p$ ($p = 2^{256} - 2^{32} - 977$)**:
//!   information-theoretically secure against any classical or
//!   quantum adversary holding $\leq K - 1$ shares.
//! - **ML-KEM-768**: IND-CCA secure under MLWE (NIST Level 3 PQ).
//! - **AES-256-GCM-SIV**: IND-CCA AEAD (NIST Level 5 PQ acceptable
//!   under Grover).
//! - **HKDF-SHA256**: standard KDF (NIST Level 1 PQ acceptable).
//!
//! Combined: the threshold construction inherits the **weakest** of
//! the layers, which is ML-KEM-768 at NIST Level 3 PQ
//! (corresponding to $\sim 192$-bit post-quantum security). The
//! Shamir layer adds information-theoretic collusion-resistance
//! that does not degrade under PQ adversaries.

use crate::mlkem_delegation::{
    decap_recipient, encap_to_recipient, MlKemDelegation, MlKemDelegationError,
    MlKemPrivKey, MlKemPubKey,
};
use crate::threshold::{
    reconstruct as shamir_reconstruct, split as shamir_split, Holder, ShamirShare,
    ThresholdError, ThresholdScheme,
};

// ───────────────────────────────────────────────────────────────────
// Types
// ───────────────────────────────────────────────────────────────────

/// A single shareholder's PQ-encrypted delivery: an ML-KEM-encrypted
/// Shamir share. The proxy holds and forwards these without
/// cryptographic involvement.
#[derive(Debug, Clone)]
pub struct LatticeShareEnvelope {
    /// The Shamir share's metadata (bundle_id, share_index, holder).
    /// We serialize this so the recipient knows their share's position
    /// when they decapsulate.
    pub share_index: u8,
    pub holder_pubkey: [u8; 32],
    /// ML-KEM ciphertext + AEAD-wrapped Shamir share bytes.
    pub mlkem_envelope: MlKemDelegation,
}

/// The full threshold-PQ delegation: scheme parameters + the $N$
/// encrypted envelopes (one per shareholder).
#[derive(Debug, Clone)]
pub struct LatticeThresholdDelegation {
    pub bundle_id: String,
    pub scheme: ThresholdScheme,
    pub envelopes: Vec<LatticeShareEnvelope>,
}

#[derive(Debug, thiserror::Error)]
pub enum LatticeDelegationError {
    #[error("Shamir layer error: {0}")]
    ShamirLayer(#[from] ThresholdError),

    #[error("ML-KEM layer error: {0}")]
    MlKemLayer(#[from] MlKemDelegationError),

    #[error("holder count {given} does not match scheme.n = {expected}")]
    HolderCountMismatch { given: usize, expected: u8 },

    #[error("envelope count {given} below threshold k = {required}")]
    InsufficientEnvelopes { given: usize, required: u8 },

    #[error("recipient secret-key vector ({sk_count}) does not match envelope subset ({env_count})")]
    SecretKeyCountMismatch { sk_count: usize, env_count: usize },
}

// ───────────────────────────────────────────────────────────────────
// Delegate
// ───────────────────────────────────────────────────────────────────

/// **Alice's side**: split a 32-byte payload (typically a gauge key
/// or other session secret) into $K$-of-$N$ Shamir shares, then
/// PQ-encrypt each share to its assigned shareholder via ML-KEM.
///
/// Returns the publishable delegation envelope set.
pub fn delegate(
    payload: &[u8; 32],
    scheme: ThresholdScheme,
    holders: &[Holder],
    holder_mlkem_pks: &[MlKemPubKey],
    auth_key: &[u8; 32],
    bundle_id: &str,
) -> Result<LatticeThresholdDelegation, LatticeDelegationError> {
    if holders.len() != scheme.n as usize {
        return Err(LatticeDelegationError::HolderCountMismatch {
            given: holders.len(),
            expected: scheme.n,
        });
    }
    if holder_mlkem_pks.len() != scheme.n as usize {
        return Err(LatticeDelegationError::HolderCountMismatch {
            given: holder_mlkem_pks.len(),
            expected: scheme.n,
        });
    }

    // 1. Shamir-split the payload.
    let shares = shamir_split(payload, scheme, holders, auth_key, bundle_id)?;

    // 2. ML-KEM-encrypt each share to its shareholder's PQ pubkey.
    let mut envelopes = Vec::with_capacity(shares.len());
    for (share, pk) in shares.iter().zip(holder_mlkem_pks.iter()) {
        let share_bytes = serialize_share(share);
        let mlkem_env = encap_to_recipient(pk, &share_bytes)?;
        envelopes.push(LatticeShareEnvelope {
            share_index: share.share_index,
            holder_pubkey: share.holder.pubkey,
            mlkem_envelope: mlkem_env,
        });
    }

    Ok(LatticeThresholdDelegation {
        bundle_id: bundle_id.to_string(),
        scheme,
        envelopes,
    })
}

// ───────────────────────────────────────────────────────────────────
// Reconstruct (K-of-N shareholders cooperating)
// ───────────────────────────────────────────────────────────────────

/// **Reconstruct path**: K shareholders cooperate. Each one
/// independently decapsulates their envelope (using their own ML-KEM
/// secret); the K resulting Shamir shares are then combined via
/// Lagrange interpolation to recover the original payload.
///
/// Inputs:
/// - `envelopes`: ≥ K envelopes from the publication. May include
///   more than K; only the first K are used (any K-subset works).
/// - `recipient_sks`: parallel array of each shareholder's ML-KEM
///   secret key, in the same order as `envelopes`.
/// - `auth_key`, `bundle_id`: same as at delegation time.
///
/// Output: the recovered 32-byte payload, or a typed error if any
/// step fails (envelope tamper, wrong key, insufficient envelopes,
/// share-auth failure).
pub fn reconstruct(
    delegation: &LatticeThresholdDelegation,
    envelopes: &[LatticeShareEnvelope],
    recipient_sks: &[MlKemPrivKey],
    auth_key: &[u8; 32],
) -> Result<[u8; 32], LatticeDelegationError> {
    if envelopes.len() != recipient_sks.len() {
        return Err(LatticeDelegationError::SecretKeyCountMismatch {
            sk_count: recipient_sks.len(),
            env_count: envelopes.len(),
        });
    }
    if envelopes.len() < delegation.scheme.k as usize {
        return Err(LatticeDelegationError::InsufficientEnvelopes {
            given: envelopes.len(),
            required: delegation.scheme.k,
        });
    }

    // 1. PQ-decapsulate each envelope to recover its Shamir share.
    let mut shares = Vec::with_capacity(envelopes.len());
    for (env, sk) in envelopes.iter().zip(recipient_sks.iter()) {
        let share_bytes = decap_recipient(sk, &env.mlkem_envelope)?;
        let share = deserialize_share(&share_bytes, &delegation.bundle_id, env.share_index)?;
        shares.push(share);
    }

    // 2. Shamir-reconstruct from the K shares.
    let payload = shamir_reconstruct(&shares, delegation.scheme, auth_key, &delegation.bundle_id)?;
    Ok(payload)
}

// ───────────────────────────────────────────────────────────────────
// Internal: share (de)serialization
// ───────────────────────────────────────────────────────────────────

/// Serialize a ShamirShare for ML-KEM transport. Layout:
///   1 byte share_index
///   32 bytes value (Fp scalar, big-endian)
///   32 bytes auth_tag
///   32 bytes holder.pubkey
///   2 bytes label_len (u16 big-endian)
///   N bytes label (UTF-8)
///   2 bytes bundle_id_len (u16 BE)
///   M bytes bundle_id (UTF-8)
fn serialize_share(s: &ShamirShare) -> Vec<u8> {
    let label_bytes = s.holder.label.as_bytes();
    let bundle_bytes = s.bundle_id.as_bytes();
    let mut out = Vec::with_capacity(1 + 32 + 32 + 32 + 2 + label_bytes.len() + 2 + bundle_bytes.len());
    out.push(s.share_index);
    out.extend_from_slice(&s.value);
    out.extend_from_slice(&s.auth_tag);
    out.extend_from_slice(&s.holder.pubkey);
    out.extend_from_slice(&(label_bytes.len() as u16).to_be_bytes());
    out.extend_from_slice(label_bytes);
    out.extend_from_slice(&(bundle_bytes.len() as u16).to_be_bytes());
    out.extend_from_slice(bundle_bytes);
    out
}

fn deserialize_share(
    bytes: &[u8],
    expected_bundle_id: &str,
    expected_index: u8,
) -> Result<ShamirShare, LatticeDelegationError> {
    if bytes.len() < 1 + 32 + 32 + 32 + 2 {
        return Err(LatticeDelegationError::MlKemLayer(
            MlKemDelegationError::AeadDecryptFailed,
        ));
    }
    let share_index = bytes[0];
    if share_index != expected_index {
        return Err(LatticeDelegationError::MlKemLayer(
            MlKemDelegationError::AeadDecryptFailed,
        ));
    }
    let mut value = [0u8; 32];
    value.copy_from_slice(&bytes[1..33]);
    let mut auth_tag = [0u8; 32];
    auth_tag.copy_from_slice(&bytes[33..65]);
    let mut holder_pubkey = [0u8; 32];
    holder_pubkey.copy_from_slice(&bytes[65..97]);
    let label_len = u16::from_be_bytes([bytes[97], bytes[98]]) as usize;
    let label_start = 99;
    let label_end = label_start + label_len;
    if bytes.len() < label_end + 2 {
        return Err(LatticeDelegationError::MlKemLayer(
            MlKemDelegationError::AeadDecryptFailed,
        ));
    }
    let label = String::from_utf8(bytes[label_start..label_end].to_vec())
        .map_err(|_| LatticeDelegationError::MlKemLayer(MlKemDelegationError::AeadDecryptFailed))?;
    let bundle_len_start = label_end;
    let bundle_len = u16::from_be_bytes([bytes[bundle_len_start], bytes[bundle_len_start + 1]]) as usize;
    let bundle_start = bundle_len_start + 2;
    let bundle_end = bundle_start + bundle_len;
    if bytes.len() < bundle_end {
        return Err(LatticeDelegationError::MlKemLayer(
            MlKemDelegationError::AeadDecryptFailed,
        ));
    }
    let bundle_id = String::from_utf8(bytes[bundle_start..bundle_end].to_vec())
        .map_err(|_| LatticeDelegationError::MlKemLayer(MlKemDelegationError::AeadDecryptFailed))?;
    // Bundle-id check (defense in depth; the reconstruct step also checks).
    if bundle_id != expected_bundle_id {
        return Err(LatticeDelegationError::ShamirLayer(
            ThresholdError::BundleIdMismatch {
                expected: expected_bundle_id.to_string(),
                got: bundle_id,
            },
        ));
    }
    Ok(ShamirShare {
        bundle_id,
        holder: Holder {
            pubkey: holder_pubkey,
            label,
        },
        share_index,
        value,
        auth_tag,
    })
}

// ───────────────────────────────────────────────────────────────────
// Tests
// ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mlkem_delegation::keygen as mlkem_keygen;

    fn fixed_payload(seed: u8) -> [u8; 32] {
        let mut p = [0u8; 32];
        for (i, b) in p.iter_mut().enumerate() {
            *b = ((i as u8).wrapping_mul(seed)).wrapping_add(seed);
        }
        p
    }

    fn fixed_auth_key() -> [u8; 32] {
        let mut k = [0u8; 32];
        for (i, b) in k.iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(53).wrapping_add(11);
        }
        k
    }

    fn n_holders_with_pks(n: usize) -> (Vec<Holder>, Vec<MlKemPrivKey>, Vec<MlKemPubKey>) {
        let mut holders = Vec::with_capacity(n);
        let mut sks = Vec::with_capacity(n);
        let mut pks = Vec::with_capacity(n);
        for i in 0..n {
            let mut pk_bytes = [0u8; 32];
            pk_bytes[0] = (i + 1) as u8;
            holders.push(Holder {
                pubkey: pk_bytes,
                label: format!("holder-{}", i),
            });
            let (sk, pk) = mlkem_keygen();
            sks.push(sk);
            pks.push(pk);
        }
        (holders, sks, pks)
    }

    #[test]
    fn delegation_roundtrip_3_of_5() {
        let payload = fixed_payload(0xA5);
        let scheme = ThresholdScheme { k: 3, n: 5 };
        let (holders, sks, pks) = n_holders_with_pks(5);
        let auth = fixed_auth_key();

        let delegation = delegate(&payload, scheme, &holders, &pks, &auth, "test-bundle").unwrap();
        assert_eq!(delegation.envelopes.len(), 5);

        // 3 shareholders cooperate (first 3).
        let recovered = reconstruct(
            &delegation,
            &delegation.envelopes[..3],
            &sks[..3],
            &auth,
        )
        .unwrap();
        assert_eq!(recovered, payload);
    }

    #[test]
    fn delegation_roundtrip_from_arbitrary_k_subset() {
        let payload = fixed_payload(0xCC);
        let scheme = ThresholdScheme { k: 3, n: 5 };
        let (holders, sks, pks) = n_holders_with_pks(5);
        let auth = fixed_auth_key();

        let delegation = delegate(&payload, scheme, &holders, &pks, &auth, "b").unwrap();

        // Pick indices {0, 2, 4} — non-contiguous.
        let subset_envs: Vec<_> = [0usize, 2, 4]
            .iter()
            .map(|&i| delegation.envelopes[i].clone())
            .collect();
        let subset_sks: Vec<_> = [0usize, 2, 4]
            .iter()
            .map(|&i| MlKemPrivKey(sks[i].0.clone()))
            .collect();

        let recovered = reconstruct(&delegation, &subset_envs, &subset_sks, &auth).unwrap();
        assert_eq!(recovered, payload);
    }

    #[test]
    fn insufficient_envelopes_below_threshold_rejected() {
        let payload = fixed_payload(0x42);
        let scheme = ThresholdScheme { k: 3, n: 5 };
        let (holders, sks, pks) = n_holders_with_pks(5);
        let auth = fixed_auth_key();

        let delegation = delegate(&payload, scheme, &holders, &pks, &auth, "b").unwrap();

        let result = reconstruct(
            &delegation,
            &delegation.envelopes[..2],
            &sks[..2],
            &auth,
        );
        assert!(matches!(
            result,
            Err(LatticeDelegationError::InsufficientEnvelopes { .. })
        ));
    }

    #[test]
    fn wrong_recipient_key_fails_envelope_decap() {
        let payload = fixed_payload(0xAB);
        let scheme = ThresholdScheme { k: 2, n: 3 };
        let (holders, sks, pks) = n_holders_with_pks(3);
        let auth = fixed_auth_key();

        let delegation = delegate(&payload, scheme, &holders, &pks, &auth, "b").unwrap();

        // Try to decap envelope 0 with shareholder 1's key.
        let mixed_sks = vec![
            MlKemPrivKey(sks[1].0.clone()),
            MlKemPrivKey(sks[1].0.clone()),
        ];
        let result = reconstruct(
            &delegation,
            &delegation.envelopes[..2],
            &mixed_sks,
            &auth,
        );
        assert!(result.is_err(), "wrong-key decap should fail");
    }

    #[test]
    fn tampered_envelope_aead_fails_authentication() {
        let payload = fixed_payload(0x99);
        let scheme = ThresholdScheme { k: 2, n: 3 };
        let (holders, sks, pks) = n_holders_with_pks(3);
        let auth = fixed_auth_key();

        let mut delegation = delegate(&payload, scheme, &holders, &pks, &auth, "b").unwrap();
        // Tamper with envelope 0's AEAD ciphertext.
        delegation.envelopes[0].mlkem_envelope.aead_ciphertext[0] ^= 0xFF;
        let result = reconstruct(
            &delegation,
            &delegation.envelopes[..2],
            &sks[..2],
            &auth,
        );
        assert!(result.is_err());
    }

    #[test]
    fn holder_count_mismatch_at_delegate_time() {
        let payload = fixed_payload(0x11);
        let scheme = ThresholdScheme { k: 3, n: 5 };
        let (holders, _sks, pks) = n_holders_with_pks(4); // wrong count
        let auth = fixed_auth_key();

        let result = delegate(&payload, scheme, &holders, &pks, &auth, "b");
        assert!(matches!(
            result,
            Err(LatticeDelegationError::HolderCountMismatch { .. })
        ));
    }

    /// Information-theoretic collusion-resistance under K-1 envelopes:
    /// k-1 shareholders cooperating get k-1 plaintext shares, which
    /// under Shamir reveal zero information about the secret. (We
    /// can't test "zero info" directly without a distinguisher; we
    /// test the operational property: reconstruction is refused with
    /// InsufficientEnvelopes, AND no algebraic shortcut exists in the
    /// shipped code path.)
    #[test]
    fn k_minus_1_envelopes_information_theoretically_safe() {
        let payload = fixed_payload(0x77);
        let scheme = ThresholdScheme { k: 4, n: 7 };
        let (holders, sks, pks) = n_holders_with_pks(7);
        let auth = fixed_auth_key();

        let delegation = delegate(&payload, scheme, &holders, &pks, &auth, "b").unwrap();

        // k-1 = 3 envelopes:
        let result = reconstruct(
            &delegation,
            &delegation.envelopes[..3],
            &sks[..3],
            &auth,
        );
        assert!(matches!(
            result,
            Err(LatticeDelegationError::InsufficientEnvelopes {
                given: 3,
                required: 4
            })
        ));

        // Adding the 4th unlocks (sanity).
        let recovered = reconstruct(
            &delegation,
            &delegation.envelopes[..4],
            &sks[..4],
            &auth,
        )
        .unwrap();
        assert_eq!(recovered, payload);
    }
}
