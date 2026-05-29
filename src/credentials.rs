//! GIGI Encrypt v0.4 — Sprint O.B: Credential-gated invariant query
//! authorization.
//!
//! A `QueryCredential` proves that the holder is authorized to execute
//! a specific query class on a specific bundle. The verifier checks the
//! credential without seeing the holder's identity (only a commitment).
//!
//! ## v0.4 construction: HMAC-bound credential
//!
//! The credential is an HMAC-SHA256 tag over
//! `(user_commitment, query_class, bundle_id)` under a per-issuer key.
//! Verification recomputes the tag and constant-time-compares. This is
//! a deterministic-issuance, replayable credential — every issuance of
//! the same (commitment, class, bundle) tuple produces the same tag.
//!
//! **What v0.4 provides:**
//!  - The issuer's signing key authorizes the (commitment, class,
//!    bundle) tuple.
//!  - The verifier learns the commitment but not the underlying user
//!    identity (commitment is opaque to the verifier).
//!  - Tampering with any of (key, class, bundle) is detected with
//!    HMAC-SHA256 security (≈ 2¹²⁸ work to forge).
//!
//! **What v0.4 does NOT provide (deferred to v0.5):**
//!  - **CL credential unlinkability**: two presentations of the same
//!    credential by the same holder are linkable (same tag → same
//!    presentation). True unlinkability requires randomized
//!    presentations of a CL/BBS+ signature; that's the v0.5 upgrade.
//!  - **Issuer commitment**: the issuing key is symmetric; the v0.5
//!    upgrade to BBS+ replaces it with a public verification key.
//!
//! See `theory/encryption/GIGI_ENCRYPT_v0.4_SPRINT_SPEC.md` §Sprint O
//! for the BBS+ vs CL04 flavor pinning and the lattice-BBS path to
//! post-quantum credentials.

use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// The issuer's symmetric signing key. v0.4 uses HMAC-SHA256, so
/// "signing" is symmetric — the verifier must hold the same key. v0.5
/// upgrades to BBS+ public-key signatures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CredentialIssuingKey(pub [u8; 32]);

/// An opaque 32-byte commitment to the holder's identity. The verifier
/// sees the commitment but not the underlying user_id; the commitment
/// scheme (Pedersen, hash, etc.) is the caller's responsibility for
/// v0.4. v0.5 will pin a specific commitment via the BBS+ blind-sign
/// protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserCommitment(pub [u8; 32]);

/// The class of query the credential authorizes. v0.4 enumerates the
/// builtin invariant-ring generators; v0.5 will widen this to arbitrary
/// parser-admitted query ASTs hashed into a class identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryClass {
    /// Davis dispersion K = Var / range².
    K,
    /// Record count τ.
    Tau,
    /// Spectral gap λ_1.
    Lambda1,
    /// Mean holonomy ⟨Hol⟩.
    HolonomyMean,
    /// Polynomial combination of invariant-ring generators
    /// (`c_0 + c_1·K + c_2·K² + …`, etc.). Distinguished by a 16-byte
    /// content hash committed at issuance time so that two distinct
    /// polynomials yield distinct credentials.
    Polynomial { content_hash: [u8; 16] },
}

impl QueryClass {
    /// Stable byte encoding for the credential's HMAC input. The
    /// encoding is unambiguous (length-prefixed tag bytes) so that no
    /// two distinct `QueryClass` values produce the same encoding.
    fn encode(&self) -> Vec<u8> {
        match self {
            QueryClass::K => vec![0x01],
            QueryClass::Tau => vec![0x02],
            QueryClass::Lambda1 => vec![0x03],
            QueryClass::HolonomyMean => vec![0x04],
            QueryClass::Polynomial { content_hash } => {
                let mut out = Vec::with_capacity(1 + 16);
                out.push(0x05);
                out.extend_from_slice(content_hash);
                out
            }
        }
    }
}

/// A query-authorization credential. The HMAC tag binds
/// `(user_commitment, query_class, bundle_id)` under the issuer's key.
#[derive(Debug, Clone)]
pub struct QueryCredential {
    pub user_commitment: UserCommitment,
    pub query_class: QueryClass,
    pub bundle_id: String,
    pub tag: [u8; 32],
}

/// Compute the HMAC binding tag. The domain separator is versioned
/// (`"GIGI_v0.4_credential_v1"`) so a future protocol revision is
/// trivially incompatible with v0.4 tags — prevents cross-version
/// replay.
fn compute_tag(
    issuing_key: &CredentialIssuingKey,
    user_commitment: &UserCommitment,
    query_class: &QueryClass,
    bundle_id: &str,
) -> [u8; 32] {
    let mut mac =
        <HmacSha256 as Mac>::new_from_slice(&issuing_key.0).expect("HMAC accepts any key");
    mac.update(b"GIGI_v0.4_credential_v1");
    mac.update(&[0u8]); // separator
    mac.update(&user_commitment.0);
    mac.update(&[0u8]); // separator
    let class_bytes = query_class.encode();
    mac.update(&(class_bytes.len() as u32).to_be_bytes());
    mac.update(&class_bytes);
    mac.update(&[0u8]); // separator
    let bid_bytes = bundle_id.as_bytes();
    mac.update(&(bid_bytes.len() as u32).to_be_bytes());
    mac.update(bid_bytes);
    let result = mac.finalize().into_bytes();
    let mut tag = [0u8; 32];
    tag.copy_from_slice(&result);
    tag
}

/// **Issuer side**: produce a `QueryCredential` authorizing
/// `user_commitment` to run `query_class` on the bundle identified by
/// `bundle_id`. Deterministic — issuing the same tuple twice produces
/// the same tag.
pub fn issue_credential(
    issuing_key: &CredentialIssuingKey,
    user_commitment: UserCommitment,
    query_class: QueryClass,
    bundle_id: impl Into<String>,
) -> QueryCredential {
    let bundle_id = bundle_id.into();
    let tag = compute_tag(issuing_key, &user_commitment, &query_class, &bundle_id);
    QueryCredential {
        user_commitment,
        query_class,
        bundle_id,
        tag,
    }
}

/// **Verifier side**: check that `credential` was issued for
/// `expected_query_class` on `expected_bundle_id` under
/// `expected_issuing_key`. Returns `true` iff all components match the
/// credential's binding and the HMAC tag is valid.
///
/// Constant-time tag comparison (no early-return on tag-byte
/// mismatch) — mirrors `crate::threshold::constant_time_eq`. The
/// pre-tag checks (bundle_id, query class) are NOT constant-time;
/// those metadata fields are public per the credential's definition.
pub fn verify_credential(
    issuing_key: &CredentialIssuingKey,
    credential: &QueryCredential,
    expected_query_class: QueryClass,
    expected_bundle_id: &str,
) -> bool {
    if credential.bundle_id != expected_bundle_id {
        return false;
    }
    if credential.query_class != expected_query_class {
        return false;
    }
    let recomputed = compute_tag(
        issuing_key,
        &credential.user_commitment,
        &credential.query_class,
        &credential.bundle_id,
    );
    constant_time_eq(&credential.tag, &recomputed)
}

/// Constant-time 32-byte equality. Mirrors
/// `crate::threshold::constant_time_eq`.
fn constant_time_eq(a: &[u8; 32], b: &[u8; 32]) -> bool {
    let mut diff = 0u8;
    for i in 0..32 {
        diff |= a[i] ^ b[i];
    }
    diff == 0
}

// ───────────────────────────────────────────────────────────────────
// Unit tests
// ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed_key() -> CredentialIssuingKey {
        CredentialIssuingKey([0x42u8; 32])
    }
    fn fixed_user() -> UserCommitment {
        UserCommitment([0x07u8; 32])
    }

    #[test]
    fn issued_credential_verifies_under_correct_inputs() {
        let cred = issue_credential(&fixed_key(), fixed_user(), QueryClass::K, "bundle_001");
        assert!(verify_credential(
            &fixed_key(),
            &cred,
            QueryClass::K,
            "bundle_001"
        ));
    }

    #[test]
    fn wrong_bundle_id_rejected() {
        let cred = issue_credential(&fixed_key(), fixed_user(), QueryClass::K, "bundle_001");
        assert!(!verify_credential(
            &fixed_key(),
            &cred,
            QueryClass::K,
            "bundle_002"
        ));
    }

    #[test]
    fn wrong_query_class_rejected() {
        let cred = issue_credential(&fixed_key(), fixed_user(), QueryClass::K, "bundle_001");
        assert!(!verify_credential(
            &fixed_key(),
            &cred,
            QueryClass::Tau,
            "bundle_001"
        ));
    }

    #[test]
    fn wrong_issuing_key_rejected() {
        let cred = issue_credential(&fixed_key(), fixed_user(), QueryClass::K, "bundle_001");
        let wrong = CredentialIssuingKey([0x99u8; 32]);
        assert!(!verify_credential(&wrong, &cred, QueryClass::K, "bundle_001"));
    }

    #[test]
    fn deterministic_issuance() {
        let c1 = issue_credential(&fixed_key(), fixed_user(), QueryClass::K, "x");
        let c2 = issue_credential(&fixed_key(), fixed_user(), QueryClass::K, "x");
        assert_eq!(c1.tag, c2.tag);
    }

    #[test]
    fn polynomial_class_distinguished_by_content_hash() {
        let cred_p1 = issue_credential(
            &fixed_key(),
            fixed_user(),
            QueryClass::Polynomial {
                content_hash: [0x01; 16],
            },
            "bundle",
        );
        let cred_p2 = issue_credential(
            &fixed_key(),
            fixed_user(),
            QueryClass::Polynomial {
                content_hash: [0x02; 16],
            },
            "bundle",
        );
        // Different content hash → distinct credentials, and neither
        // verifies against the other's class.
        assert_ne!(cred_p1.tag, cred_p2.tag);
        assert!(!verify_credential(
            &fixed_key(),
            &cred_p1,
            QueryClass::Polynomial {
                content_hash: [0x02; 16]
            },
            "bundle"
        ));
    }
}
