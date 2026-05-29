//! GIGI Encrypt v0.3 — Sprint K: Holonomy ledger tests.
//!
//! 9 tests per spec §5.2. Two are `#[ignore]` pending bundle-integration
//! follow-ups (concurrent WAL serialization, key-rotation event leaf
//! emission) — math primitive ships with this commit.
//!
//! Run with: `cargo test --test ledger_v0_3`

use gigi::ledger::{hash_record_bytes, HolonomyLedger, LedgerError, LedgerLeaf, OpKind};

// ───────────────────────────────────────────────────────────────────────
// Helpers
// ───────────────────────────────────────────────────────────────────────

fn synthetic_leaf(op_id: u64, holonomy_delta: f64, record_bytes: &[u8]) -> LedgerLeaf {
    LedgerLeaf {
        timestamp: 1_700_000_000 + op_id as i64,
        op_id,
        holonomy_delta,
        record_hash: hash_record_bytes(record_bytes),
        op_kind: OpKind::Insert,
    }
}

fn populate_ledger(n: u64) -> HolonomyLedger {
    let mut l = HolonomyLedger::new();
    for i in 0..n {
        let bytes = format!("record-{}", i).into_bytes();
        l.append(synthetic_leaf(i, 0.01 * (i as f64 + 1.0), &bytes));
    }
    l
}

// ───────────────────────────────────────────────────────────────────────
// Spec §5.2 — 9 tests
// ───────────────────────────────────────────────────────────────────────

/// Append-only property: API exposes no mutation path. After appending N
/// leaves, the only operations available are read (`leaf(i)`, `leaves()`),
/// extend (`append`), or root computation. Verified structurally — the
/// `HolonomyLedger` exposes no `set`, `remove`, `mutate_leaf`, or
/// `replace` methods (compile-time guarantee).
#[test]
fn test_holonomy_ledger_append_only() {
    let mut l = populate_ledger(5);
    // The append API returns an index, and the leaf is immutable from then
    // on. We confirm by re-reading and checking the stored value matches.
    let idx = l.append(synthetic_leaf(5, 0.5, b"new"));
    assert_eq!(idx, 5);
    let stored = l.leaf(5).unwrap();
    assert_eq!(stored.op_id, 5);
    assert_eq!(stored.holonomy_delta, 0.5);
    // No mutation API exists — this test passes by virtue of the type API
    // (you cannot call `l.replace_leaf(0, ...)` because that method does
    // not exist on `HolonomyLedger`).
}

/// Spec §5.2: same 100 writes in same order → same root.
#[test]
fn test_holonomy_ledger_merkle_root_deterministic() {
    let l1 = populate_ledger(100);
    let l2 = populate_ledger(100);
    assert_eq!(l1.root().unwrap(), l2.root().unwrap());
}

/// Spec §5.2: root before write ≠ root after.
#[test]
fn test_holonomy_ledger_root_changes_on_new_write() {
    let mut l = populate_ledger(10);
    let root_before = l.root().unwrap();
    l.append(synthetic_leaf(10, 0.0, b"extra"));
    let root_after = l.root().unwrap();
    assert_ne!(root_before, root_after);
}

/// Spec §5.2: for write t in a log of N=1000 entries, inclusion proof
/// verifies against root.
#[test]
fn test_holonomy_ledger_inclusion_proof_verifies() {
    let l = populate_ledger(1000);
    // Spot-check a sample of indices including boundaries.
    for &idx in &[0, 1, 7, 42, 256, 511, 512, 999] {
        let proof = l.inclusion_proof(idx).unwrap();
        assert!(
            HolonomyLedger::verify_proof(&proof),
            "inclusion proof failed at index {}",
            idx
        );
        // Proof size is O(log N) ~ ⌈log2 1000⌉ = 10.
        assert!(
            proof.siblings.len() <= 10,
            "proof has {} siblings, expected ≤10 for N=1000",
            proof.siblings.len()
        );
    }
}

/// Spec §5.2: flip one bit in leaf t → proof verification fails.
#[test]
fn test_holonomy_ledger_inclusion_proof_fails_on_tampered_leaf() {
    let l = populate_ledger(50);
    let mut proof = l.inclusion_proof(17).unwrap();
    proof.leaf_hash.0[0] ^= 0xFF;
    assert!(!HolonomyLedger::verify_proof(&proof));
}

/// Spec §5.2 (Theorem 5.2 recompute-and-compare): modify a record in
/// the bundle simulation; the ledger's stored deltas no longer match
/// the recomputed holonomy — telescope_check returns false.
#[test]
fn test_holonomy_ledger_recompute_and_compare_detects_holonomy_tamper() {
    let mut l = HolonomyLedger::new();
    // Simulate a sequence of writes with known deltas.
    let baseline_holonomy = 0.0;
    let deltas = [0.10, 0.20, -0.05, 0.30, 0.15];
    for (i, &d) in deltas.iter().enumerate() {
        l.append(synthetic_leaf(i as u64, d, format!("rec-{}", i).as_bytes()));
    }
    let expected_recomputed = baseline_holonomy + deltas.iter().sum::<f64>();
    assert!(l.telescope_check(baseline_holonomy, expected_recomputed));
    // Now suppose an attacker tampered with a record so the live bundle's
    // recomputed holonomy is different — telescope check detects.
    let tampered_recomputed = expected_recomputed + 0.05;
    assert!(!l.telescope_check(baseline_holonomy, tampered_recomputed));
}

/// Spec §5.2 (extended record_hash closes Sprint I blindspot per §3.8):
/// modify a record's bytes; re-hashing and comparing against the
/// ledger's stored `record_hash` detects the tamper.
#[test]
fn test_holonomy_ledger_record_hash_walk_detects_byte_tamper() {
    let mut l = HolonomyLedger::new();
    let original_bytes = b"alice,42,active";
    let hash_at_write = hash_record_bytes(original_bytes);
    l.append(LedgerLeaf {
        timestamp: 1,
        op_id: 1,
        holonomy_delta: 0.0,
        record_hash: hash_at_write,
        op_kind: OpKind::Insert,
    });
    // Live read sees the record bytes; re-hash and compare.
    let recomputed_unchanged = hash_record_bytes(original_bytes);
    assert_eq!(recomputed_unchanged, l.leaf(0).unwrap().record_hash);
    // Tampered: a single byte flip changes the hash with overwhelming
    // probability (SHA-256 preimage / collision resistance).
    let tampered_bytes = b"alice,42,banned";
    let recomputed_tampered = hash_record_bytes(tampered_bytes);
    assert_ne!(recomputed_tampered, l.leaf(0).unwrap().record_hash);
}

/// Spec §5.2: concurrent WAL writes serialize. **Deferred** — requires
/// bundle/WAL integration commit. Math primitive (the ledger's
/// append-only Vec) is itself serial; concurrent integration is the
/// follow-up.
#[test]
#[ignore = "deferred to bundle/WAL integration commit (Sprint K follow-up)"]
fn test_holonomy_ledger_concurrent_appends_serialize() {
    todo!("requires bundle.rs + wal.rs hooks to emit ledger leaves on insert");
}

/// Spec §5.2: Sprint G key rotation emits a rotation event leaf.
/// Composition test at
/// `tests/composition_v0_3.rs::test_composition_ledger_x_rotation_event`
/// covers the math: append a sequence of insert leaves, then a Rotate
/// leaf with holonomy_delta = 0, verify the root and inclusion proofs
/// for pre-rotation entries still hold under the new root.
#[test]
fn test_holonomy_ledger_rotation_compatible() {
    let mut l = HolonomyLedger::new();
    for i in 0..3u64 {
        l.append(synthetic_leaf(i, 0.01, format!("rec-{}", i).as_bytes()));
    }
    let root_pre = l.root().unwrap();
    // Append a rotate leaf (delta = 0):
    l.append(LedgerLeaf {
        timestamp: 999,
        op_id: 3,
        holonomy_delta: 0.0,
        record_hash: [0u8; 32],
        op_kind: OpKind::Rotate,
    });
    let root_post = l.root().unwrap();
    assert_ne!(root_pre, root_post);
    // Pre-rotation inclusion proofs still verify under the new root:
    for idx in 0..3 {
        let proof = l.inclusion_proof(idx).unwrap();
        assert!(HolonomyLedger::verify_proof(&proof));
        assert_eq!(proof.root, root_post);
    }
}
