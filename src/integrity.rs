//! GIGI Encrypt v0.3 — Sprint I: Curvature-MAC (bundle integrity).
//!
//! **Scope statement.** This module detects *gauge-invariant content drift* —
//! any modification of a bundle that changes its observable geometric output
//! (the invariant tuple `(K, λ₁, C, ⟨Hol⟩, β_0, β_1)`). It does **not** detect
//! arbitrary record-level tampering. Modifications that preserve all six
//! invariants exactly (e.g., record permutations on a set-semantic bundle,
//! eigenvalue-preserving graph automorphisms, trivial-position record
//! duplicates) evade this primitive by construction.
//!
//! Byte-level tamper-evidence is delivered by Sprint K's extended ledger leaves
//! (`record_hash`). The two primitives are **complementary**, and the
//! combined claim (v0.3.1 spec §3.8, Theorem 3.2) is that their union detects
//! all non-trivial modifications up to cryptographic collision bound.
//!
//! See `GIGI_ENCRYPT_v0.3_SPRINT_SPEC.md` §3 for the full spec.

use crate::parser::{InvariantExpr, InvariantOp};
use crate::BundleStore;

use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Domain-separation salt for the integrity-key derivation. Distinct from
/// any salt used by the v0.2 gauge KDF, so compromise of the gauge key does
/// not enable integrity-tag forgery (spec §3.4).
const INTEGRITY_KDF_SALT: &[u8] = b"gigi-integrity-v1";

/// Version magic placed at the front of the canonical-bytes encoding so
/// future tuple schemas can be discriminated without re-signing existing
/// bundles (spec §3.7).
const CANONICAL_MAGIC: &[u8; 4] = b"GIGI";

/// Quiet-NaN bit pattern used as the canonical NaN for f64 encoding
/// (spec §3.7). Any NaN component of an invariant tuple is normalized to
/// this exact pattern before encoding so `canonical_bytes` is deterministic
/// even when intermediate computations produce signalling NaNs or different
/// quiet-NaN payloads.
const CANONICAL_NAN_BITS: u64 = 0x7ff8_0000_0000_0000;

// ───────────────────────────────────────────────────────────────────────
// IntegrityTag
// ───────────────────────────────────────────────────────────────────────

/// HMAC-SHA256 output of the canonical-encoded invariant tuple.
///
/// Spec §3.1: `τ(B) = HMAC-SHA256(k_MAC, canonical(π_inv(B)))`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IntegrityTag(pub [u8; 32]);

impl IntegrityTag {
    /// Hex-encode the 32-byte tag for surfacing through GQL / API.
    pub fn to_hex(&self) -> String {
        let mut s = String::with_capacity(64);
        for b in self.0.iter() {
            s.push_str(&format!("{:02x}", b));
        }
        s
    }

    /// Parse a hex string into an `IntegrityTag`. Returns `None` if the input
    /// is not exactly 64 hex chars.
    pub fn from_hex(hex: &str) -> Option<Self> {
        if hex.len() != 64 {
            return None;
        }
        let mut out = [0u8; 32];
        for i in 0..32 {
            let byte = u8::from_str_radix(hex.get(i * 2..i * 2 + 2)?, 16).ok()?;
            out[i] = byte;
        }
        Some(IntegrityTag(out))
    }
}

// ───────────────────────────────────────────────────────────────────────
// IntegrityKey
// ───────────────────────────────────────────────────────────────────────

/// HMAC-SHA256 key for the integrity tag.
///
/// Derived from `BundleSchema::integrity_seed` via
/// `HKDF-SHA256(seed, salt = "gigi-integrity-v1")` (spec §3.4) — domain-
/// separated from the gauge KDF so compromise of the gauge key does not
/// enable tag forgery.
#[derive(Debug, Clone)]
pub struct IntegrityKey(pub [u8; 32]);

impl IntegrityKey {
    /// Derive an integrity key from a 32-byte seed via HKDF-SHA256 with the
    /// fixed salt `"gigi-integrity-v1"`.
    ///
    /// HKDF-Extract collects entropy from the seed under the salt; HKDF-Expand
    /// produces the 32-byte output key material. The `info` parameter is
    /// empty (we use the salt for domain separation instead, which is the
    /// idiomatic HKDF usage when there's only one downstream key per seed).
    pub fn derive(seed: &[u8; 32]) -> Self {
        let hk = Hkdf::<Sha256>::new(Some(INTEGRITY_KDF_SALT), seed);
        let mut okm = [0u8; 32];
        hk.expand(b"", &mut okm)
            .expect("HKDF expand cannot fail for 32-byte output (well under L_max)");
        IntegrityKey(okm)
    }
}

// ───────────────────────────────────────────────────────────────────────
// InvariantTuple
// ───────────────────────────────────────────────────────────────────────

/// The six gauge-invariant scalars that summarize a bundle's geometric state.
///
/// **v0.3.1 design fix**: this tuple was previously
/// `(K, λ_1, capacity, ⟨Hol⟩, β_0, β_1)` with `capacity = τ / K` as an
/// f64 field. Capacity is **redundant** (fully determined by τ and K)
/// and its computation amplifies K's relative drift by τ — leading to
/// ~10⁻⁹ absolute drift on capacity even when K's drift is ~10⁻¹¹
/// relative. Replacing the f64 `capacity` slot with the u64
/// `record_count = τ` removes the amplification entirely and lets the
/// f64 quantization grain tighten from 10⁻⁶ to 10⁻¹⁰ — four orders of
/// magnitude stronger sensitivity to genuine modifications, while
/// preserving the same information content (anyone can recover
/// capacity = record_count as f64 / k on demand).
///
/// Each component is gauge-invariant under the structure group of the
/// v0.2 modes (Affine, Opaque, Indexed, Probabilistic, Isometric),
/// proved per-mode in the v0.1 / v0.2 specs.
#[derive(Debug, Clone, PartialEq)]
pub struct InvariantTuple {
    /// Scalar curvature K.
    pub k: f64,
    /// Spectral gap λ_1 (first non-zero Laplacian eigenvalue).
    pub lambda_1: f64,
    /// Mean holonomy ⟨Hol⟩ over closed loops in the base graph.
    pub holonomy_mean: f64,
    /// Record count τ. Pure u64; zero drift across gauge rotation.
    /// (Replaces the v0.3.0 `capacity` f64 field — see struct doc.)
    pub record_count: u64,
    /// 0th Betti number β_0 (connected components).
    pub beta_0: u64,
    /// 1st Betti number β_1 (independent 1-cycles).
    pub beta_1: u64,
}

impl InvariantTuple {
    /// Canonical big-endian byte encoding of the invariant tuple.
    ///
    /// Layout (52 bytes total, v0.3.1 layout):
    ///
    /// | Offset | Bytes | Field                                                  |
    /// |-------:|------:|--------------------------------------------------------|
    /// |   0–3  |   4   | version magic `b"GIGI"`                                |
    /// |   4–11 |   8   | `k`             (quantized i64 BE, 10¹⁰ precision)     |
    /// |  12–19 |   8   | `lambda_1`      (quantized i64 BE, 10¹⁰ precision)     |
    /// |  20–27 |   8   | `holonomy_mean` (quantized i64 BE, 10¹⁰ precision)     |
    /// |  28–35 |   8   | `record_count`  (u64 BE — exact, no drift)             |
    /// |  36–43 |   8   | `beta_0`        (u64 BE)                               |
    /// |  44–51 |   8   | `beta_1`        (u64 BE)                               |
    ///
    /// **f64 quantization** (v0.3.1 tightened): each f64 component is
    /// rounded to 10 decimal digits before encoding
    /// (`round(x · 10¹⁰)` stored as i64). The capacity-amplification
    /// problem of v0.3.0 has been removed by replacing the f64 capacity
    /// slot with a u64 record_count slot — capacity = record_count / K
    /// is recoverable on demand without storing it in the tag input.
    /// With capacity gone, the worst observed drift across gauge
    /// rotation is on K itself: ~7.5×10⁻¹³ absolute at K ≈ 0.087, or
    /// ~10⁻¹¹ relative. The 10⁻¹⁰ quantization grain sits one order of
    /// magnitude above this measured noise floor and 4 orders of
    /// magnitude tighter than the v0.3.0 6-dp workaround. Any genuine
    /// drift in the invariants ≥ 10⁻¹⁰ is detected.
    ///
    /// **NaN handling**: any NaN → `i64::MIN`. ±0.0 → 0. ±Inf clamps to
    /// ±i64::MAX (unreachable for realistic bundle invariants).
    pub fn canonical_bytes(&self) -> [u8; 52] {
        let mut out = [0u8; 52];
        out[0..4].copy_from_slice(CANONICAL_MAGIC);
        out[4..12].copy_from_slice(&quantize_f64(self.k).to_be_bytes());
        out[12..20].copy_from_slice(&quantize_f64(self.lambda_1).to_be_bytes());
        out[20..28].copy_from_slice(&quantize_f64(self.holonomy_mean).to_be_bytes());
        out[28..36].copy_from_slice(&self.record_count.to_be_bytes());
        out[36..44].copy_from_slice(&self.beta_0.to_be_bytes());
        out[44..52].copy_from_slice(&self.beta_1.to_be_bytes());
        out
    }

    /// Compute the invariant tuple from a live `BundleStore`.
    ///
    /// Calls into the existing v0.2 invariant-ring evaluators
    /// (`crate::curvature::scalar_curvature`, `crate::spectral::spectral_gap`,
    /// `crate::spectral::betti_numbers`, `crate::invariant::evaluate` for
    /// `HolonomyAvg`). The computation is O(1) in record count once the
    /// Welford-streaming field stats are populated.
    pub fn compute(store: &BundleStore) -> Self {
        let k = crate::curvature::scalar_curvature(store);
        let lambda_1 = crate::spectral::spectral_gap(store);
        let (b0, b1) = crate::spectral::betti_numbers(store);
        let holonomy_mean =
            crate::invariant::evaluate(store, &InvariantExpr::Op(InvariantOp::HolonomyAvg));
        Self {
            k,
            lambda_1,
            holonomy_mean,
            record_count: store.len() as u64,
            beta_0: b0 as u64,
            beta_1: b1 as u64,
        }
    }
}

/// Quantize an f64 to a 10-decimal-precision i64 representation.
///
/// `quantize_f64(x) = round(x · 10¹⁰)` clamped to i64 range. NaN maps to
/// `i64::MIN`.
///
/// **Why 10 decimal digits** (v0.3.1 tightened from v0.3.0's 6 dp):
/// removing capacity from the invariant tuple eliminated the τ/K
/// amplification that drove v0.3.0's 10⁻⁹ drift on capacity. The
/// remaining worst-case drift is on K itself — measured ~7.5×10⁻¹³
/// absolute at K ≈ 0.087 across an Aff(ℝ) gauge rotation on a 40-record
/// bundle, or ~10⁻¹¹ relative. The 10⁻¹⁰ quantization grain sits
/// one order of magnitude above this measured noise floor: gauge
/// invariance holds bit-identically through the tag in practice while
/// detection sensitivity is 4 orders of magnitude tighter than v0.3.0.
///
/// Saturation: i64::MAX corresponds to ~9.22 × 10⁸ at 10¹⁰ scale. The
/// invariant components are realistically bounded well below this (K ≤
/// ~10², λ_1 ≤ ~10², ⟨Hol⟩ ∈ ℝ but bounded by per-graph topology), so
/// no real bundle saturates.
#[inline]
fn quantize_f64(x: f64) -> i64 {
    if x.is_nan() {
        return i64::MIN;
    }
    let scaled = x * 1e10;
    if scaled >= i64::MAX as f64 {
        i64::MAX
    } else if scaled <= i64::MIN as f64 {
        i64::MIN
    } else {
        scaled.round() as i64
    }
}

#[allow(dead_code)] // kept for reference; superseded by quantize_f64
fn canonicalize_f64(x: f64) -> f64 {
    if x.is_nan() {
        f64::from_bits(CANONICAL_NAN_BITS)
    } else if x == 0.0 {
        0.0_f64
    } else {
        x
    }
}

// ───────────────────────────────────────────────────────────────────────
// sign / verify
// ───────────────────────────────────────────────────────────────────────

/// Mint an `IntegrityTag` for the given invariant tuple under a particular
/// integrity key.
///
/// `τ(B) = HMAC-SHA256(k_MAC, canonical(π_inv(B)))`.
pub fn sign(key: &IntegrityKey, tuple: &InvariantTuple) -> IntegrityTag {
    let bytes = tuple.canonical_bytes();
    let mut mac =
        <HmacSha256 as Mac>::new_from_slice(&key.0).expect("HMAC-SHA256 accepts any key length");
    mac.update(&bytes);
    let result = mac.finalize().into_bytes();
    let mut tag = [0u8; 32];
    tag.copy_from_slice(&result);
    IntegrityTag(tag)
}

/// Verify that `tag` is the integrity tag for `tuple` under `key`.
///
/// Constant-time equality comparison via `hmac::Mac::verify_slice` to avoid
/// timing-channel leaks on the tag bytes.
pub fn verify(key: &IntegrityKey, tuple: &InvariantTuple, tag: &IntegrityTag) -> bool {
    let bytes = tuple.canonical_bytes();
    let mut mac =
        <HmacSha256 as Mac>::new_from_slice(&key.0).expect("HMAC-SHA256 accepts any key length");
    mac.update(&bytes);
    mac.verify_slice(&tag.0).is_ok()
}

/// Convenience: compute the integrity tuple + tag for a bundle in one call.
///
/// Equivalent to `sign(&key, &InvariantTuple::compute(store))`.
pub fn sign_bundle(store: &BundleStore, key: &IntegrityKey) -> IntegrityTag {
    let tuple = InvariantTuple::compute(store);
    sign(key, &tuple)
}

/// Convenience: verify a stored tag against the current bundle state.
///
/// Equivalent to `verify(&key, &InvariantTuple::compute(store), tag)`.
/// Returns `(verified, computed_tuple)` — the computed tuple is surfaced
/// for the `VERIFY_INTEGRITY` GQL response (spec §3.5) so callers can see
/// the drift dimensions when verification fails.
pub fn verify_bundle(
    store: &BundleStore,
    key: &IntegrityKey,
    tag: &IntegrityTag,
) -> (bool, InvariantTuple) {
    let tuple = InvariantTuple::compute(store);
    let ok = verify(key, &tuple, tag);
    (ok, tuple)
}

// ───────────────────────────────────────────────────────────────────────
// Module-level unit tests (canonical encoding + KDF + HMAC behavior).
// Integration-level tests with a live BundleStore live in
// `tests/integrity_v0_3.rs`.
// ───────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_bytes_is_52_bytes_with_magic_at_front() {
        let t = InvariantTuple {
            k: 1.0,
            lambda_1: 2.0,
            holonomy_mean: 3.0,
            record_count: 4,
            beta_0: 5,
            beta_1: 6,
        };
        let bytes = t.canonical_bytes();
        assert_eq!(bytes.len(), 52);
        assert_eq!(&bytes[0..4], CANONICAL_MAGIC);
    }

    #[test]
    fn canonical_bytes_normalizes_negative_zero_and_nan() {
        // Distinct NaN bit patterns and +0.0 vs -0.0 must collapse to
        // identical bytes after quantization (NaN → i64::MIN; ±0 → 0).
        let t1 = InvariantTuple {
            k: -0.0,
            lambda_1: f64::NAN,
            holonomy_mean: 0.0,
            record_count: 0,
            beta_0: 0,
            beta_1: 0,
        };
        let t2 = InvariantTuple {
            k: 0.0,
            lambda_1: f64::from_bits(0x7ff8_0000_0000_dead), // alternate quiet-NaN payload
            holonomy_mean: -0.0,
            record_count: 0,
            beta_0: 0,
            beta_1: 0,
        };
        assert_eq!(t1.canonical_bytes(), t2.canonical_bytes());
    }

    #[test]
    fn derive_then_sign_then_verify_roundtrip() {
        let seed = [0xABu8; 32];
        let key = IntegrityKey::derive(&seed);
        let t = InvariantTuple {
            k: 0.0341,
            lambda_1: 0.71,
            holonomy_mean: 0.0,
            record_count: 100,
            beta_0: 1,
            beta_1: 0,
        };
        let tag = sign(&key, &t);
        assert!(verify(&key, &t, &tag));
        // Modify one field and expect verification to fail.
        let t2 = InvariantTuple {
            k: 0.0342,
            ..t.clone()
        };
        assert!(!verify(&key, &t2, &tag));
    }

    #[test]
    fn hex_roundtrip() {
        let tag = IntegrityTag([0x5Au8; 32]);
        let hex = tag.to_hex();
        assert_eq!(hex.len(), 64);
        assert_eq!(IntegrityTag::from_hex(&hex), Some(tag));
        assert_eq!(IntegrityTag::from_hex("too short"), None);
        // Non-hex chars
        assert_eq!(IntegrityTag::from_hex(&"z".repeat(64)), None);
    }

    #[test]
    fn derive_with_different_seeds_produces_different_keys() {
        let mut seed_a = [0u8; 32];
        let mut seed_b = [0u8; 32];
        seed_a[0] = 1;
        seed_b[0] = 2;
        let key_a = IntegrityKey::derive(&seed_a);
        let key_b = IntegrityKey::derive(&seed_b);
        assert_ne!(key_a.0, key_b.0);
    }
}
