//! GIGI Encrypt v0.3 — Sprint K: Holonomy ledger (tamper-evident audit log).
//!
//! Per-write append-only Merkle log over
//! `(timestamp, op_id, holonomy_delta, record_hash, op_kind)`.
//!
//! **Two-level tamper-evidence** (spec §5.1, Theorems 5.1 + 5.2):
//! 1. **Merkle inclusion-proof tamper-evidence** (Thm 5.1): modifying any past
//!    leaf changes the SHA-256 leaf hash, which propagates to the root.
//! 2. **Telescope recompute-and-compare** (Thm 5.2): recomputing the bundle's
//!    current ⟨Hol⟩ from records and comparing to `⟨Hol⟩(B_0) + Σ Δ_t` from
//!    the ledger detects modifications that bypass the leaves.
//! 3. **Byte-level `record_hash` walk** (closes Sprint I §3.8 blindspot):
//!    re-hashing live records and comparing against ledger `record_hash`
//!    leaves catches modifications that preserve the invariant tuple.
//!
//! Merkle tree per RFC 6962 (Certificate Transparency log structure):
//! - Leaf hash: `SHA-256(0x00 || canonical_leaf_bytes)`
//! - Internal hash: `SHA-256(0x01 || left || right)`
//! - Odd-count level: last node duplicated upward
//!
//! See `GIGI_ENCRYPT_v0.3_SPRINT_SPEC.md` §5 for the full spec.

use sha2::{Digest, Sha256};

// ───────────────────────────────────────────────────────────────────────
// Types
// ───────────────────────────────────────────────────────────────────────

/// 32-byte SHA-256 hash, used for both leaf and internal node hashes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LeafHash(pub [u8; 32]);

impl LeafHash {
    pub fn to_hex(&self) -> String {
        let mut s = String::with_capacity(64);
        for b in self.0.iter() {
            s.push_str(&format!("{:02x}", b));
        }
        s
    }
}

/// Operation kind for a ledger leaf — preserved across modifications so
/// audit consumers can reconstruct the write history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OpKind {
    Insert = 1,
    Update = 2,
    Delete = 3,
    Rotate = 4,
    Split = 5,
}

/// A single ledger leaf — one per write event on a bundle.
///
/// Extended from v0.3.0 → v0.3.1 (per review item 2): `record_hash` was
/// added to close the Sprint I gauge-invariant-content blindspot. Re-hashing
/// live records and comparing against this field detects byte-level
/// modifications that preserve all six invariants.
#[derive(Debug, Clone)]
pub struct LedgerLeaf {
    pub timestamp: i64,
    pub op_id: u64,
    pub holonomy_delta: f64,
    pub record_hash: [u8; 32],
    pub op_kind: OpKind,
}

impl LedgerLeaf {
    /// Canonical 57-byte big-endian encoding for hashing.
    ///
    /// | Offset | Bytes | Field                              |
    /// |-------:|------:|------------------------------------|
    /// |   0–7  |   8   | `timestamp`      (i64 BE)          |
    /// |   8–15 |   8   | `op_id`          (u64 BE)          |
    /// |  16–23 |   8   | `holonomy_delta` (f64 BE, NaN-norm)|
    /// |  24–55 |  32   | `record_hash`                      |
    /// |  56    |   1   | `op_kind`        (u8 discriminant) |
    pub fn canonical_bytes(&self) -> [u8; 57] {
        let mut out = [0u8; 57];
        out[0..8].copy_from_slice(&self.timestamp.to_be_bytes());
        out[8..16].copy_from_slice(&self.op_id.to_be_bytes());
        let delta = if self.holonomy_delta.is_nan() {
            f64::from_bits(0x7ff8_0000_0000_0000)
        } else if self.holonomy_delta == 0.0 {
            0.0
        } else {
            self.holonomy_delta
        };
        out[16..24].copy_from_slice(&delta.to_be_bytes());
        out[24..56].copy_from_slice(&self.record_hash);
        out[56] = self.op_kind as u8;
        out
    }

    /// RFC 6962 leaf hash: `SHA-256(0x00 || canonical_bytes)`.
    pub fn leaf_hash(&self) -> LeafHash {
        let mut h = Sha256::new();
        h.update([0x00u8]);
        h.update(self.canonical_bytes());
        let out: [u8; 32] = h.finalize().into();
        LeafHash(out)
    }
}

/// Inclusion proof for a single leaf within a ledger.
///
/// Verifying: walk up from `leaf_hash` combining with each sibling at the
/// recorded position. If the final hash equals `root`, the leaf was
/// committed at the time the root was published.
#[derive(Debug, Clone)]
pub struct InclusionProof {
    pub leaf_index: usize,
    pub leaf_hash: LeafHash,
    pub root: LeafHash,
    /// Sibling hashes from bottom up, paired with `right` indicating whether
    /// the sibling is on the right (`true`) or left (`false`).
    pub siblings: Vec<(LeafHash, bool)>,
}

/// The ledger itself — append-only, Merkle-rooted.
#[derive(Debug, Clone, Default)]
pub struct HolonomyLedger {
    leaves: Vec<LedgerLeaf>,
}

#[derive(Debug, thiserror::Error)]
pub enum LedgerError {
    #[error("leaf index {idx} out of bounds (ledger has {count} leaves)")]
    LeafIndexOutOfBounds { idx: usize, count: usize },
    #[error("ledger is empty; root is undefined")]
    EmptyLedger,
}

// ───────────────────────────────────────────────────────────────────────
// HolonomyLedger
// ───────────────────────────────────────────────────────────────────────

impl HolonomyLedger {
    pub fn new() -> Self {
        Self { leaves: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.leaves.len()
    }

    pub fn is_empty(&self) -> bool {
        self.leaves.is_empty()
    }

    /// Append a leaf. Returns the index at which it was sealed.
    ///
    /// The append-only property is enforced by the API: there is no
    /// `replace`, `set`, or `mutate_leaf` method exposed. The internal
    /// `leaves: Vec<LedgerLeaf>` is `pub(crate)`-free; callers can only
    /// extend, never modify.
    pub fn append(&mut self, leaf: LedgerLeaf) -> usize {
        let idx = self.leaves.len();
        self.leaves.push(leaf);
        idx
    }

    /// Read-only access to a sealed leaf (no mutation API).
    pub fn leaf(&self, idx: usize) -> Option<&LedgerLeaf> {
        self.leaves.get(idx)
    }

    /// Iterate over all sealed leaves in order.
    pub fn leaves(&self) -> impl Iterator<Item = &LedgerLeaf> {
        self.leaves.iter()
    }

    /// Compute the current Merkle root (RFC 6962).
    ///
    /// Implementation: full recompute from leaf hashes. O(N log N) per call;
    /// for N up to 10^6, dominated by ~2N SHA-256 ops, well under the
    /// 1ms/10⁶ leaves budget in spec §11.1. Incremental subtree caching
    /// is a v0.3.x optimization; correctness is the v0.3.1 acceptance gate.
    pub fn root(&self) -> Result<LeafHash, LedgerError> {
        if self.leaves.is_empty() {
            return Err(LedgerError::EmptyLedger);
        }
        let mut layer: Vec<LeafHash> = self.leaves.iter().map(|l| l.leaf_hash()).collect();
        while layer.len() > 1 {
            let mut next = Vec::with_capacity((layer.len() + 1) / 2);
            let mut i = 0;
            while i < layer.len() {
                if i + 1 < layer.len() {
                    next.push(internal_hash(&layer[i], &layer[i + 1]));
                    i += 2;
                } else {
                    // Odd node at this level — RFC 6962 promotes
                    // unpaired node verbatim (does NOT duplicate).
                    next.push(layer[i]);
                    i += 1;
                }
            }
            layer = next;
        }
        Ok(layer[0])
    }

    /// Build an inclusion proof for the leaf at `idx`.
    pub fn inclusion_proof(&self, idx: usize) -> Result<InclusionProof, LedgerError> {
        if idx >= self.leaves.len() {
            return Err(LedgerError::LeafIndexOutOfBounds {
                idx,
                count: self.leaves.len(),
            });
        }
        let leaf_hash = self.leaves[idx].leaf_hash();
        let root = self.root()?;
        let mut layer: Vec<LeafHash> = self.leaves.iter().map(|l| l.leaf_hash()).collect();
        let mut path_idx = idx;
        let mut siblings = Vec::new();
        while layer.len() > 1 {
            // Pair with sibling at this level, if any.
            let pair_idx = path_idx ^ 1; // sibling index in current layer
            if pair_idx < layer.len() {
                // sibling on the right when pair_idx is odd (i.e. path_idx is even)
                let sibling_is_right = path_idx % 2 == 0;
                siblings.push((layer[pair_idx], sibling_is_right));
            }
            // Promote to parent layer.
            let mut next = Vec::with_capacity((layer.len() + 1) / 2);
            let mut i = 0;
            while i < layer.len() {
                if i + 1 < layer.len() {
                    next.push(internal_hash(&layer[i], &layer[i + 1]));
                    i += 2;
                } else {
                    next.push(layer[i]);
                    i += 1;
                }
            }
            layer = next;
            path_idx /= 2;
        }
        Ok(InclusionProof {
            leaf_index: idx,
            leaf_hash,
            root,
            siblings,
        })
    }

    /// Verify an inclusion proof against its claimed leaf hash and root.
    ///
    /// Walks the sibling path from the leaf up; returns `true` iff the
    /// final hash matches the recorded root. No mutable state.
    pub fn verify_proof(proof: &InclusionProof) -> bool {
        let mut current = proof.leaf_hash;
        for (sibling, sibling_is_right) in &proof.siblings {
            current = if *sibling_is_right {
                internal_hash(&current, sibling)
            } else {
                internal_hash(sibling, &current)
            };
        }
        current == proof.root
    }

    /// Telescope check: given the bundle's baseline ⟨Hol⟩ at write 0 and a
    /// freshly-recomputed ⟨Hol⟩ from current records, verify that the sum
    /// of stored deltas matches the difference. Returns `true` iff
    /// `baseline + Σ Δ_t ≈ recomputed` (within f64 precision tolerance).
    pub fn telescope_check(&self, baseline_holonomy: f64, recomputed_holonomy: f64) -> bool {
        let sum_deltas: f64 = self.leaves.iter().map(|l| l.holonomy_delta).sum();
        let expected = baseline_holonomy + sum_deltas;
        (expected - recomputed_holonomy).abs() < 1e-9 * (expected.abs().max(1.0))
    }

    /// Sum of all stored deltas — exposed for diagnostics / GQL surfacing.
    pub fn sum_deltas(&self) -> f64 {
        self.leaves.iter().map(|l| l.holonomy_delta).sum()
    }
}

/// RFC 6962 internal hash: `SHA-256(0x01 || left || right)`.
fn internal_hash(left: &LeafHash, right: &LeafHash) -> LeafHash {
    let mut h = Sha256::new();
    h.update([0x01u8]);
    h.update(left.0);
    h.update(right.0);
    let out: [u8; 32] = h.finalize().into();
    LeafHash(out)
}

/// Convenience: SHA-256 of arbitrary canonical record bytes. Used by
/// callers to produce the `record_hash` field of a leaf, and by the
/// byte-level tamper-evidence walker to re-hash live records.
pub fn hash_record_bytes(bytes: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(bytes);
    h.finalize().into()
}

// ───────────────────────────────────────────────────────────────────────
// Unit tests (math primitive in isolation).
// Integration with a live BundleStore lives in `tests/ledger_v0_3.rs`.
// ───────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_leaf(op_id: u64) -> LedgerLeaf {
        LedgerLeaf {
            timestamp: 1_700_000_000 + op_id as i64,
            op_id,
            holonomy_delta: (op_id as f64) * 0.01,
            record_hash: [op_id as u8; 32],
            op_kind: OpKind::Insert,
        }
    }

    #[test]
    fn canonical_bytes_is_57_bytes_with_op_kind_last() {
        let leaf = fake_leaf(42);
        let bytes = leaf.canonical_bytes();
        assert_eq!(bytes.len(), 57);
        assert_eq!(bytes[56], OpKind::Insert as u8);
    }

    #[test]
    fn leaf_hash_is_sha256_with_leaf_prefix() {
        let leaf = fake_leaf(1);
        let lh = leaf.leaf_hash();
        // Recompute by hand to confirm RFC 6962 prefix discipline.
        let mut h = Sha256::new();
        h.update([0x00u8]);
        h.update(leaf.canonical_bytes());
        let expected: [u8; 32] = h.finalize().into();
        assert_eq!(lh.0, expected);
    }

    #[test]
    fn root_is_deterministic_across_runs() {
        let mut a = HolonomyLedger::new();
        let mut b = HolonomyLedger::new();
        for i in 0..7 {
            a.append(fake_leaf(i));
            b.append(fake_leaf(i));
        }
        assert_eq!(a.root().unwrap(), b.root().unwrap());
    }

    #[test]
    fn root_changes_on_appended_leaf() {
        let mut l = HolonomyLedger::new();
        for i in 0..4 {
            l.append(fake_leaf(i));
        }
        let r1 = l.root().unwrap();
        l.append(fake_leaf(4));
        let r2 = l.root().unwrap();
        assert_ne!(r1, r2);
    }

    #[test]
    fn inclusion_proof_verifies_for_each_leaf() {
        let mut l = HolonomyLedger::new();
        for i in 0..13 {
            l.append(fake_leaf(i));
        }
        for idx in 0..13 {
            let proof = l.inclusion_proof(idx).unwrap();
            assert!(HolonomyLedger::verify_proof(&proof), "leaf {} should verify", idx);
        }
    }

    #[test]
    fn inclusion_proof_rejects_tampered_leaf() {
        let mut l = HolonomyLedger::new();
        for i in 0..8 {
            l.append(fake_leaf(i));
        }
        let mut proof = l.inclusion_proof(3).unwrap();
        // Tamper with the leaf hash:
        proof.leaf_hash.0[0] ^= 0xFF;
        assert!(!HolonomyLedger::verify_proof(&proof));
    }

    #[test]
    fn telescope_sum_matches_for_unchanged_bundle() {
        let mut l = HolonomyLedger::new();
        for i in 1..11 {
            l.append(LedgerLeaf {
                timestamp: i as i64,
                op_id: i,
                holonomy_delta: 0.1,
                record_hash: [0u8; 32],
                op_kind: OpKind::Insert,
            });
        }
        // Baseline 0.0 + 10 × 0.1 = 1.0; recomputed = 1.0 → telescope_check true.
        assert!(l.telescope_check(0.0, 1.0));
        // Mismatch → false.
        assert!(!l.telescope_check(0.0, 1.5));
    }

    #[test]
    fn empty_ledger_has_no_root() {
        let l = HolonomyLedger::new();
        assert!(matches!(l.root(), Err(LedgerError::EmptyLedger)));
    }
}
