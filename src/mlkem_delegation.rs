//! GIGI Encrypt v0.3.x — Sprint J.3: ML-KEM trusted-delegatee delegation.
//!
//! **Post-quantum key transport** for the trusted-delegatee threat model.
//! Uses FIPS 203 (ML-KEM, formerly CRYSTALS-Kyber) at security level 3
//! (ML-KEM-768; ~192-bit post-quantum security, NIST Level 3).
//!
//! ## Construction
//!
//! 1. **Bob** generates an ML-KEM-768 keypair `(dk_B, ek_B)`.
//!    Publishes `ek_B` (1184 bytes) as his recipient pubkey.
//! 2. **Alice** wants to delegate her gauge key (or any session-secret
//!    bytes) to Bob. She calls
//!    `encap_to_recipient(&ek_B, &alice_gauge_serialized)`:
//!    - Sample ephemeral randomness `m`.
//!    - Run ML-KEM encapsulation `(ct, ss) = Encaps(ek_B, m)`. `ss` is
//!      a 32-byte shared secret; `ct` is the 1088-byte ML-KEM
//!      ciphertext.
//!    - Derive AEAD key: `K_aead = HKDF-SHA256(ss, "gigi-mlkem-delegation-v1")`.
//!    - AEAD-encrypt the payload (gauge bytes) under `K_aead`:
//!      `aead_ct = AES-256-GCM-SIV(K_aead, payload)`.
//!    - Publish `(ct, aead_ct)` to the proxy.
//! 3. **Proxy** holds and forwards `(ct, aead_ct)` to Bob. Plays no
//!    cryptographic role.
//! 4. **Bob** receives `(ct, aead_ct)`:
//!    - `ss = Decaps(dk_B, ct)` (or any equivalent derivation).
//!    - `K_aead = HKDF-SHA256(ss, "gigi-mlkem-delegation-v1")`.
//!    - `payload = AES-256-GCM-SIV-decrypt(K_aead, aead_ct)`.
//!    - Bob now holds Alice's gauge key and can decrypt Alice's
//!      records.
//!
//! ## Trust model
//!
//! - **Trusted-delegatee**: Bob holds Alice's full key after
//!   delegation. Equivalent to Aff(ℝ) capability delegation in trust
//!   structure, but with PQ-safe key transport.
//! - **Not collusion-resistant**: a Bob-proxy coalition (which would
//!   require Bob to share his ML-KEM secret with the proxy) trivially
//!   recovers Alice's payload — Bob already has everything Alice sent
//!   him.
//! - For collusion-resistant delegation (Bob's key alone is
//!   insufficient even if leaked), use Sprint J.2 pairing delegation
//!   (BLS12-381, pre-quantum) or Sprint J.4 lattice-PRE (PQ +
//!   collusion-resistant, future).
//!
//! ## Cryptographic strength
//!
//! - **ML-KEM-768**: IND-CCA secure under the Module Learning With
//!   Errors (MLWE) hardness assumption. NIST Level 3 PQ security
//!   (corresponding to AES-192 classical strength).
//! - **AES-256-GCM-SIV**: IND-CCA secure; 256-bit symmetric strength
//!   (NIST Level 5 PQ acceptable under Grover bound).
//! - **HKDF-SHA256**: standard KDF (NIST Level 1 PQ acceptable).
//! - **Combined**: NIST Level 3 PQ secure under MLWE + standard
//!   symmetric assumptions. Quantum attacks (Shor, Grover) do not
//!   reduce security to below NIST Level 1.

use aes_gcm_siv::{
    aead::{Aead, KeyInit, Payload},
    Aes256GcmSiv, Nonce,
};
use hkdf::Hkdf;
use kem::{Decapsulate, Encapsulate};
use ml_kem::{
    kem::{DecapsulationKey, EncapsulationKey},
    EncodedSizeUser, KemCore, MlKem768, MlKem768Params,
};
use rand_core::OsRng;
use sha2::Sha256;

const KDF_INFO: &[u8] = b"gigi-mlkem-delegation-v1";
const AEAD_NONCE_LEN: usize = 12;

// ───────────────────────────────────────────────────────────────────
// Types
// ───────────────────────────────────────────────────────────────────

/// ML-KEM-768 public (encapsulation) key. Bob publishes this; Alice
/// uses it to encapsulate a shared secret.
#[derive(Clone)]
pub struct MlKemPubKey(pub EncapsulationKey<MlKem768Params>);

impl std::fmt::Debug for MlKemPubKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MlKemPubKey({} bytes)", self.0.as_bytes().len())
    }
}

/// ML-KEM-768 secret (decapsulation) key. Bob keeps this private.
pub struct MlKemPrivKey(pub DecapsulationKey<MlKem768Params>);

impl std::fmt::Debug for MlKemPrivKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MlKemPrivKey(<redacted>)")
    }
}

/// A complete ML-KEM-delegated payload: the ML-KEM ciphertext
/// (encapsulating the shared secret) + the AEAD-encrypted payload
/// (the data Alice is transmitting to Bob).
#[derive(Debug, Clone)]
pub struct MlKemDelegation {
    /// ML-KEM-768 ciphertext (1088 bytes).
    pub kem_ciphertext: Vec<u8>,
    /// 12-byte AEAD nonce.
    pub aead_nonce: [u8; AEAD_NONCE_LEN],
    /// AES-256-GCM-SIV ciphertext of the payload (variable length).
    pub aead_ciphertext: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
pub enum MlKemDelegationError {
    #[error("ML-KEM-768 encapsulation failed")]
    EncapsulateFailed,
    #[error("ML-KEM-768 decapsulation failed (corrupted ciphertext or wrong key)")]
    DecapsulateFailed,
    #[error("AEAD encryption failed")]
    AeadEncryptFailed,
    #[error("AEAD decryption failed (tag mismatch, corrupted ciphertext, or wrong shared secret)")]
    AeadDecryptFailed,
}

// ───────────────────────────────────────────────────────────────────
// Key generation
// ───────────────────────────────────────────────────────────────────

/// Generate a fresh ML-KEM-768 keypair. Bob calls this once and
/// publishes the encapsulation (public) key.
pub fn keygen() -> (MlKemPrivKey, MlKemPubKey) {
    let mut rng = OsRng;
    let (dk, ek) = MlKem768::generate(&mut rng);
    (MlKemPrivKey(dk), MlKemPubKey(ek))
}

// ───────────────────────────────────────────────────────────────────
// Delegation
// ───────────────────────────────────────────────────────────────────

/// Alice's side: encapsulate a shared secret to Bob and AEAD-encrypt
/// `payload` under it. Returns the publishable `MlKemDelegation`.
///
/// **Use case**: `payload` can be Alice's serialized gauge key, a
/// session token, or any bytes Alice wishes to deliver to Bob without
/// the proxy seeing.
pub fn encap_to_recipient(
    recipient_pk: &MlKemPubKey,
    payload: &[u8],
) -> Result<MlKemDelegation, MlKemDelegationError> {
    let mut rng = OsRng;
    let (kem_ct, shared_secret) = recipient_pk
        .0
        .encapsulate(&mut rng)
        .map_err(|_| MlKemDelegationError::EncapsulateFailed)?;

    // Derive AEAD key from the 32-byte shared secret via HKDF.
    let hk = Hkdf::<Sha256>::new(None, shared_secret.as_slice());
    let mut aead_key_bytes = [0u8; 32];
    hk.expand(KDF_INFO, &mut aead_key_bytes)
        .expect("HKDF expand cannot fail for 32-byte output");

    // Generate a fresh AEAD nonce (12 bytes for AES-GCM-SIV).
    let mut nonce_bytes = [0u8; AEAD_NONCE_LEN];
    getrandom::getrandom(&mut nonce_bytes).expect("OS RNG must be available");

    let cipher = Aes256GcmSiv::new_from_slice(&aead_key_bytes)
        .expect("AES-256-GCM-SIV accepts 32-byte keys");
    let aead_ct = cipher
        .encrypt(
            Nonce::from_slice(&nonce_bytes),
            Payload {
                msg: payload,
                aad: KDF_INFO,
            },
        )
        .map_err(|_| MlKemDelegationError::AeadEncryptFailed)?;

    Ok(MlKemDelegation {
        kem_ciphertext: kem_ct.as_slice().to_vec(),
        aead_nonce: nonce_bytes,
        aead_ciphertext: aead_ct,
    })
}

/// Bob's side: decapsulate the ML-KEM ciphertext and AEAD-decrypt the
/// payload. Returns the original payload bytes.
pub fn decap_recipient(
    recipient_sk: &MlKemPrivKey,
    delegation: &MlKemDelegation,
) -> Result<Vec<u8>, MlKemDelegationError> {
    use hybrid_array::Array;

    // ML-KEM-768 ciphertext is exactly 1088 bytes; reject anything else.
    if delegation.kem_ciphertext.len() != 1088 {
        return Err(MlKemDelegationError::DecapsulateFailed);
    }

    let kem_ct: Array<u8, _> = Array::try_from(&delegation.kem_ciphertext[..])
        .map_err(|_| MlKemDelegationError::DecapsulateFailed)?;
    let shared_secret = recipient_sk
        .0
        .decapsulate(&kem_ct)
        .map_err(|_| MlKemDelegationError::DecapsulateFailed)?;

    let hk = Hkdf::<Sha256>::new(None, shared_secret.as_slice());
    let mut aead_key_bytes = [0u8; 32];
    hk.expand(KDF_INFO, &mut aead_key_bytes)
        .expect("HKDF expand cannot fail for 32-byte output");

    let cipher = Aes256GcmSiv::new_from_slice(&aead_key_bytes)
        .expect("AES-256-GCM-SIV accepts 32-byte keys");
    cipher
        .decrypt(
            Nonce::from_slice(&delegation.aead_nonce),
            Payload {
                msg: &delegation.aead_ciphertext,
                aad: KDF_INFO,
            },
        )
        .map_err(|_| MlKemDelegationError::AeadDecryptFailed)
}

// ───────────────────────────────────────────────────────────────────
// Tests
// ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keygen_produces_keypair_with_expected_sizes() {
        let (_sk, pk) = keygen();
        // ML-KEM-768 encapsulation key is 1184 bytes.
        assert_eq!(pk.0.as_bytes().len(), 1184);
    }

    #[test]
    fn encap_decap_roundtrip_short_payload() {
        let (sk_bob, pk_bob) = keygen();
        let payload = b"hello bob, this is alice's gauge key";
        let delegation = encap_to_recipient(&pk_bob, payload).unwrap();
        // ML-KEM-768 KEM ciphertext is exactly 1088 bytes.
        assert_eq!(delegation.kem_ciphertext.len(), 1088);
        let recovered = decap_recipient(&sk_bob, &delegation).unwrap();
        assert_eq!(recovered, payload);
    }

    #[test]
    fn encap_decap_roundtrip_long_payload() {
        let (sk_bob, pk_bob) = keygen();
        // Simulate a serialized gauge key with several fields.
        let payload: Vec<u8> = (0..2048).map(|i| (i & 0xFF) as u8).collect();
        let delegation = encap_to_recipient(&pk_bob, &payload).unwrap();
        let recovered = decap_recipient(&sk_bob, &delegation).unwrap();
        assert_eq!(recovered, payload);
    }

    #[test]
    fn delegation_is_distinct_under_repeated_encapsulation() {
        // ML-KEM uses fresh randomness; same payload to same Bob produces
        // different ciphertexts (IND-CCA freshness).
        let (sk_bob, pk_bob) = keygen();
        let payload = b"same plaintext twice";
        let d1 = encap_to_recipient(&pk_bob, payload).unwrap();
        let d2 = encap_to_recipient(&pk_bob, payload).unwrap();
        assert_ne!(d1.kem_ciphertext, d2.kem_ciphertext);
        assert_ne!(d1.aead_ciphertext, d2.aead_ciphertext);
        // Both still decapsulate to the same plaintext.
        assert_eq!(decap_recipient(&sk_bob, &d1).unwrap(), payload);
        assert_eq!(decap_recipient(&sk_bob, &d2).unwrap(), payload);
    }

    #[test]
    fn wrong_recipient_key_fails_decap() {
        let (_sk_bob, pk_bob) = keygen();
        let (sk_eve, _pk_eve) = keygen();
        let payload = b"only bob should be able to read this";
        let delegation = encap_to_recipient(&pk_bob, payload).unwrap();
        // Eve attempts to decap with her key — ML-KEM ciphertext was
        // encrypted to Bob's key, so Eve's decapsulation yields a
        // different shared secret, which derives a different AEAD key,
        // which fails the AEAD authentication tag check.
        let result = decap_recipient(&sk_eve, &delegation);
        assert!(matches!(
            result,
            Err(MlKemDelegationError::AeadDecryptFailed)
        ));
    }

    #[test]
    fn tampered_aead_ciphertext_fails_authentication() {
        let (sk_bob, pk_bob) = keygen();
        let payload = b"original payload";
        let mut delegation = encap_to_recipient(&pk_bob, payload).unwrap();
        // Tamper with one byte of the AEAD ciphertext:
        delegation.aead_ciphertext[0] ^= 0xFF;
        let result = decap_recipient(&sk_bob, &delegation);
        assert!(matches!(
            result,
            Err(MlKemDelegationError::AeadDecryptFailed)
        ));
    }

    #[test]
    fn tampered_kem_ciphertext_fails_decapsulation() {
        let (sk_bob, pk_bob) = keygen();
        let payload = b"original payload";
        let mut delegation = encap_to_recipient(&pk_bob, payload).unwrap();
        // Flip a bit in the KEM ciphertext. ML-KEM is IND-CCA: tampering
        // typically produces a different shared secret, which causes the
        // downstream AEAD to fail authentication.
        delegation.kem_ciphertext[100] ^= 0x42;
        let result = decap_recipient(&sk_bob, &delegation);
        assert!(result.is_err());
    }
}
