//! GIGI Encrypt v0.4 — Sprint N: Invariant Consistency Verification.
//!
//! **Public deterministic verification**, not zero knowledge. A verifier
//! holding only the encrypted bundle independently recomputes the
//! invariant tuple π_inv(B) = (K, λ_1, ⟨Hol⟩, τ, β_0, β_1) and compares
//! it against the prover's claim. No gauge key is involved — every
//! component is gauge-invariant under the v0.2 modes (Affine, Opaque,
//! Indexed, Probabilistic, Isometric) by Theorem 3.5 of the encryption
//! paper.
//!
//! **Soundness**: a prover submitting a false claim is caught with
//! probability 1 in exact arithmetic; with the 10⁻¹⁰ f64 quantization
//! used by `InvariantTuple::canonical_bytes`, a real tampering ≥ 10⁻¹⁰
//! is detected.
//!
//! **Completeness**: an honest prover always passes (modulo
//! quantization edge cases at the noise floor).
//!
//! **Leakage scope**: the verifier learns π_inv(B) plus the distribution
//! shape of the encrypted bundle and all affine-invariant statistics.
//! This is *leakage-scoped invariant disclosure*, not zero knowledge.
//!
//! **Future ZK direction** (deferred to v0.5): a formal Sigma protocol
//! proving knowledge of witness w = (g, σ) satisfying the relation
//!
//!   R = { (C, y ; w) : C = Enc_g(σ), y = π_inv(σ), w = (g, σ) }
//!
//! with completeness, special soundness (extractor), and special
//! honest-verifier ZK. See `theory/encryption/GIGI_ENCRYPT_v0.4_SPRINT_SPEC.md`
//! §Sprint N for the protocol target and open problems.

use crate::bundle::BundleStore;
use crate::integrity::InvariantTuple;

/// A prover's claim about the invariant tuple of a bundle.
///
/// The prover sends `(bundle_id, claimed_tuple)`; the verifier recomputes
/// the tuple from the (encrypted) bundle and compares.
#[derive(Debug, Clone, PartialEq)]
pub struct InvariantStatement {
    /// The bundle identifier the claim is about. Verifier checks this
    /// matches the bundle they hold (out-of-band; this struct just
    /// carries the field).
    pub bundle_id: String,
    /// The full six-component tuple π_inv = (K, λ_1, ⟨Hol⟩, τ, β_0, β_1)
    /// that the prover claims for the bundle.
    pub claimed: InvariantTuple,
}

impl InvariantStatement {
    /// Honest prover constructs a statement from the bundle by computing
    /// the tuple. By gauge invariance, this is the same tuple any
    /// gauge-equivalent encryption of the same plaintext would yield —
    /// so the prover can construct the statement on the plaintext side
    /// before encrypting.
    pub fn from_bundle(store: &BundleStore, bundle_id: impl Into<String>) -> Self {
        Self {
            bundle_id: bundle_id.into(),
            claimed: InvariantTuple::compute(store),
        }
    }
}

/// Per-field tolerances for the f64 components of the invariant tuple.
/// `record_count`, `beta_0`, `beta_1` are u64 and checked for exact
/// equality (no tolerance).
#[derive(Debug, Clone, Copy)]
pub struct InvariantTolerances {
    pub k: f64,
    pub lambda_1: f64,
    pub holonomy_mean: f64,
}

impl Default for InvariantTolerances {
    /// Default tolerance: 10⁻¹⁰, matching `InvariantTuple::canonical_bytes`'s
    /// quantization grain. Real gauge drift across the v0.2 modes is
    /// measured at ~10⁻¹¹ relative (see `src/integrity.rs` docs), so this
    /// tolerance is one order of magnitude above the noise floor — gauge
    /// invariance holds, and any genuine modification ≥ 10⁻¹⁰ is caught.
    fn default() -> Self {
        Self {
            k: 1e-10,
            lambda_1: 1e-10,
            holonomy_mean: 1e-10,
        }
    }
}

/// Result of verifying a prover's claim against the bundle.
#[derive(Debug, Clone, PartialEq)]
pub enum VerifyResult {
    /// All six components agree within tolerance AND the bundle_id
    /// in the prover's statement matches the bundle the verifier
    /// holds. The full computed tuple is surfaced for the caller's
    /// audit log.
    Verified { computed: InvariantTuple },
    /// The bundle_id in the prover's statement does not match the
    /// bundle the verifier holds. Checked **first** — a context
    /// mismatch is detected before any tuple computation is wasted on
    /// the wrong bundle.
    ///
    /// This prevents the trust-handoff hole flagged by the v0.4 review
    /// (Sprint N follow-up Gap 1): without this check, a prover could
    /// submit a claim about bundle A and the verifier could accept it
    /// against bundle B if the tuples happened to coincide.
    BundleMismatch {
        claimed: String,
        store_id: String,
    },
    /// At least one tuple component disagrees. The verifier returns
    /// the FIRST failure encountered, in fingerprint order:
    /// K → λ_1 → ⟨Hol⟩ → record_count → β_0 → β_1.
    ///
    /// For u64 fields (`record_count`, `beta_0`, `beta_1`), `claimed` and
    /// `computed` are cast to f64 for the report; `delta` is the absolute
    /// integer difference.
    Rejected {
        /// One of: `"k"`, `"lambda1"`, `"holonomy_mean"`,
        /// `"record_count"`, `"beta_0"`, `"beta_1"`.
        field: &'static str,
        claimed: f64,
        computed: f64,
        delta: f64,
    },
}

impl VerifyResult {
    /// `true` iff the claim was accepted (bundle bound AND all
    /// components within tolerance).
    pub fn is_verified(&self) -> bool {
        matches!(self, VerifyResult::Verified { .. })
    }
}

/// **Sprint N core**: verify a prover's invariant claim against the
/// bundle. The verifier holds the (possibly encrypted) `store` and the
/// `statement` from the prover; it recomputes π_inv(store) and checks
/// each component against the claim.
///
/// **Bundle-id binding** (v0.4 review follow-up Gap 1): the caller
/// passes `store_bundle_id` — the identifier they assert the `store`
/// represents. If this does not match `statement.bundle_id`, the
/// function returns `BundleMismatch` *before* computing the tuple. This
/// closes the trust-handoff hole where the API previously accepted any
/// statement whose tuple coincidentally matched the verifier's bundle.
///
/// **Verifier never receives the gauge key.** If the caller is tempted
/// to pass one in, the test is invalid — π_inv is by construction
/// computable from the encrypted bundle alone.
///
/// Returns one of:
///  - `BundleMismatch { claimed, store_id }` — bundle identity disagrees.
///  - `Verified { computed }` — bundle and all 6 tuple components agree.
///  - `Rejected { field, claimed, computed, delta }` — first tuple
///    failure in fingerprint order (K → λ_1 → ⟨Hol⟩ → record_count →
///    β_0 → β_1).
pub fn verify_invariant_statement(
    store: &BundleStore,
    store_bundle_id: &str,
    statement: &InvariantStatement,
    tolerances: InvariantTolerances,
) -> VerifyResult {
    if store_bundle_id != statement.bundle_id {
        return VerifyResult::BundleMismatch {
            claimed: statement.bundle_id.clone(),
            store_id: store_bundle_id.to_string(),
        };
    }
    let computed = InvariantTuple::compute(store);

    // f64 checks in fingerprint order.
    if let Some(r) = check_f64("k", statement.claimed.k, computed.k, tolerances.k) {
        return r;
    }
    if let Some(r) = check_f64(
        "lambda1",
        statement.claimed.lambda_1,
        computed.lambda_1,
        tolerances.lambda_1,
    ) {
        return r;
    }
    if let Some(r) = check_f64(
        "holonomy_mean",
        statement.claimed.holonomy_mean,
        computed.holonomy_mean,
        tolerances.holonomy_mean,
    ) {
        return r;
    }
    // u64 checks (exact equality).
    if let Some(r) = check_u64(
        "record_count",
        statement.claimed.record_count,
        computed.record_count,
    ) {
        return r;
    }
    if let Some(r) = check_u64("beta_0", statement.claimed.beta_0, computed.beta_0) {
        return r;
    }
    if let Some(r) = check_u64("beta_1", statement.claimed.beta_1, computed.beta_1) {
        return r;
    }

    VerifyResult::Verified { computed }
}

fn check_f64(field: &'static str, claimed: f64, computed: f64, tol: f64) -> Option<VerifyResult> {
    let delta = (claimed - computed).abs();
    if delta > tol {
        Some(VerifyResult::Rejected {
            field,
            claimed,
            computed,
            delta,
        })
    } else {
        None
    }
}

fn check_u64(field: &'static str, claimed: u64, computed: u64) -> Option<VerifyResult> {
    if claimed != computed {
        let delta = (claimed as i128 - computed as i128).unsigned_abs() as f64;
        Some(VerifyResult::Rejected {
            field,
            claimed: claimed as f64,
            computed: computed as f64,
            delta,
        })
    } else {
        None
    }
}

// ───────────────────────────────────────────────────────────────────────
// Unit tests — minimal smoke. The end-to-end TDD surface lives in
// tests/invariant_verify_v0_4.rs.
// ───────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{BundleSchema, FieldDef, Value};
    use std::collections::HashMap;

    fn tiny_bundle() -> BundleStore {
        let schema = BundleSchema::new("invariant_verify_smoke")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("value"));
        let mut store = BundleStore::new(schema);
        for i in 0..20 {
            let mut rec = HashMap::new();
            rec.insert("id".to_string(), Value::Float(i as f64));
            rec.insert("value".to_string(), Value::Float((i as f64) * 1.5 + 0.7));
            store.insert(&rec);
        }
        store
    }

    #[test]
    fn honest_claim_verifies() {
        let store = tiny_bundle();
        let stmt = InvariantStatement::from_bundle(&store, "smoke");
        let r =
            verify_invariant_statement(&store, "smoke", &stmt, InvariantTolerances::default());
        assert!(r.is_verified(), "honest claim must verify: {:?}", r);
    }

    #[test]
    fn tampered_k_is_rejected() {
        let store = tiny_bundle();
        let mut stmt = InvariantStatement::from_bundle(&store, "smoke");
        stmt.claimed.k += 0.5;
        let r =
            verify_invariant_statement(&store, "smoke", &stmt, InvariantTolerances::default());
        match r {
            VerifyResult::Rejected { field, .. } => assert_eq!(field, "k"),
            _ => panic!("expected Rejected, got {:?}", r),
        }
    }

    #[test]
    fn tampered_record_count_is_rejected() {
        let store = tiny_bundle();
        let mut stmt = InvariantStatement::from_bundle(&store, "smoke");
        stmt.claimed.record_count += 1;
        let r =
            verify_invariant_statement(&store, "smoke", &stmt, InvariantTolerances::default());
        match r {
            VerifyResult::Rejected { field, delta, .. } => {
                assert_eq!(field, "record_count");
                assert_eq!(delta, 1.0);
            }
            _ => panic!("expected Rejected, got {:?}", r),
        }
    }

    #[test]
    fn bundle_mismatch_caught_before_tuple_check() {
        // Closes Gap 1 from the v0.4 review: a prover's claim about
        // bundle "A" presented against a verifier holding bundle "B"
        // must be rejected on identity grounds even if the tuples
        // happened to coincide.
        let store = tiny_bundle();
        let stmt = InvariantStatement::from_bundle(&store, "different_bundle_id");
        let r = verify_invariant_statement(
            &store,
            "verifier_holds_this",
            &stmt,
            InvariantTolerances::default(),
        );
        match r {
            VerifyResult::BundleMismatch { claimed, store_id } => {
                assert_eq!(claimed, "different_bundle_id");
                assert_eq!(store_id, "verifier_holds_this");
            }
            _ => panic!("expected BundleMismatch, got {:?}", r),
        }
    }
}
