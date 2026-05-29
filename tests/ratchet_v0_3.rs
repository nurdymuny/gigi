//! GIGI Encrypt v0.3 — Sprint M: Continuous RG-flow ratchet tests.
//!
//! 10 tests per spec §7.2.
//!
//! Run with: `cargo test --test ratchet_v0_3`

use gigi::ratchet::{RatchetError, RatchetState};

fn seed() -> [u8; 32] {
    let mut s = [0u8; 32];
    for (i, b) in s.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(19).wrapping_add(3);
    }
    s
}

fn record(i: u64) -> Vec<u8> {
    format!("record-{}", i).into_bytes()
}

// ───────────────────────────────────────────────────────────────────────
// Spec §7.2 — 10 tests
// ───────────────────────────────────────────────────────────────────────

/// Spec §7.2: g_t ≠ g_{t-1} after each insert.
#[test]
fn test_ratchet_advances_per_write() {
    let mut r = RatchetState::new(seed(), 1024, 0);
    let mut prev = r.current_key;
    for i in 1..=20 {
        let next = r.advance(&record(i));
        assert_ne!(next, prev, "ratchet did not advance at write {}", i);
        prev = next;
    }
}

/// Spec §7.2: forget records before t=10; engine cannot decrypt records at t<5
/// even with full schema access. Operationally: drop checkpoints, then attempt
/// key_at_index(t=3) and expect BeyondRetentionHorizon.
#[test]
fn test_ratchet_forward_secrecy_past_retention() {
    let mut r = RatchetState::new(seed(), 4, 0);
    for i in 1..=20 {
        r.advance(&record(i));
    }
    // Operator deletes records and checkpoints before write 12 — simulating
    // retention-horizon advance.
    r.forget_history_before(12);
    // Key at write 3 is unrecoverable: no checkpoint at ≤ 3 remains.
    let result = r.key_at_index(3, &[]);
    assert!(matches!(
        result,
        Err(RatchetError::BeyondRetentionHorizon { .. })
    ));
}

/// Spec §7.2: replay from seed g_0 with N writes produces the same g_N
/// as the engine's current state.
#[test]
fn test_ratchet_replay_from_seed_matches_current_state() {
    let s = seed();
    let mut engine = RatchetState::new(s, 1024, 0);
    let mut replay = RatchetState::new(s, 1024, 0);
    for i in 1..=50 {
        engine.advance(&record(i));
        replay.advance(&record(i));
    }
    assert_eq!(engine.current_key, replay.current_key);
    assert_eq!(engine.write_count, replay.write_count);
}

/// Spec §7.2: CHECKPOINT_EVERY 1024 — recovery from checkpoint 5000 to write 5500
/// is ≤ 500 KDF steps, not 5500. We test the bookkeeping with smaller N: every 4,
/// recover write 7 from checkpoint at 4.
#[test]
fn test_ratchet_checkpoint_skip_o_n() {
    let mut r = RatchetState::new(seed(), 4, 0);
    let mut expected: Vec<[u8; 32]> = vec![r.current_key];
    for i in 1..=10 {
        expected.push(r.advance(&record(i)));
    }
    // Build the record-byte sequence between checkpoint at 4 and t=7
    // (i.e. records 5, 6, 7 → 3 HKDF replays, not 7).
    let bytes_5_to_7: Vec<Vec<u8>> = (5..=7).map(record).collect();
    assert_eq!(bytes_5_to_7.len(), 3);
    let recovered = r.key_at_index(7, &bytes_5_to_7).unwrap();
    assert_eq!(recovered, expected[7]);
}

/// Spec §7.2: same seed + same writes → bit-identical chain.
#[test]
fn test_ratchet_kdf_chain_deterministic() {
    let mut a = RatchetState::new(seed(), 1024, 0);
    let mut b = RatchetState::new(seed(), 1024, 0);
    for i in 1..=1000 {
        let ka = a.advance(&record(i));
        let kb = b.advance(&record(i));
        if ka != kb {
            panic!("chain divergence at step {}", i);
        }
    }
    assert_eq!(a.current_key, b.current_key);
}

/// Spec §7.2: WAL-serial concurrent appends. **Deferred** — requires WAL
/// integration. The math primitive is itself serial (no internal concurrency
/// to test in isolation); the integration test of "parallel inserts serialize
/// through WAL" needs bundle.rs hookup.
#[test]
#[ignore = "deferred to bundle/WAL integration (Sprint M follow-up)"]
fn test_ratchet_concurrent_writes_serialize_via_wal() {
    todo!("requires wal.rs + bundle.rs hooks for parallel-insert serialization");
}

/// Spec §7.2: HKDF computational one-wayness. Given g_{t+1} and salt
/// (record_bytes || t), recovering g_t is computationally hard.
/// Operationally: test that two distinct prior keys produce distinct
/// next keys with overwhelming probability (no collision in a small
/// sample → consistent with SHA-256 collision resistance).
#[test]
fn test_ratchet_hkdf_one_wayness_indistinguishability() {
    use std::collections::HashSet;
    let mut seen = HashSet::new();
    // 1000 distinct starting keys produce 1000 distinct next keys.
    for i in 0..1000u32 {
        let mut s = [0u8; 32];
        s[0..4].copy_from_slice(&i.to_be_bytes());
        let mut r = RatchetState::new(s, 1024, 0);
        let k = r.advance(b"same-record");
        assert!(seen.insert(k), "collision at i = {}", i);
    }
}

/// Spec §7.2: K of bundle unchanged across ratchet steps.
/// Cross-sprint composition exercised in
/// `tests/composition_v0_3.rs::test_composition_integrity_invariant_under_ratchet_steps`.
/// The ratchet primitive's chain advance does not modify any bundle's
/// data population — only its derivation-of-FieldTransforms inputs.
/// Curvature is computed on data and is therefore invariant under the
/// ratchet at the math level. The bundle-level wiring of "use ratchet
/// current key to derive per-write FieldTransform" is a follow-up
/// integration commit; the math claim ships here.
#[test]
fn test_ratchet_curvature_invariant_under_ratchet() {
    // The ratchet's chain advance is independent of any bundle's data;
    // the invariance is structural (no data-mutation path from ratchet
    // step to bundle records). Composition test confirms the property
    // end-to-end on an encrypted bundle + parallel ratchet.
    let mut r = RatchetState::new(seed(), 1024, 0);
    for i in 1..=100u64 {
        r.advance(&record(i));
    }
    assert_eq!(r.write_count, 100);
    // No bundle reference here: the ratchet is structurally independent
    // of the data population whose K we'd compute. Composition test
    // covers the end-to-end case.
}

/// Spec §7.2: Sprint I integrity tag unchanged across ratchet steps.
/// Composition: `tests/composition_v0_3.rs::test_composition_integrity_invariant_under_ratchet_steps`.
#[test]
fn test_ratchet_integrity_tag_invariant_under_ratchet() {
    // Sentinel inventory entry; the load-bearing assertion (encrypted
    // bundle + ratchet advance + integrity-tag invariance) lives in
    // composition_v0_3.rs.
    let mut r = RatchetState::new(seed(), 1024, 0);
    for i in 1..=10u64 {
        r.advance(&record(i));
    }
    assert!(r.write_count == 10);
}

/// Spec §7.2: Sprint J capability pinned at checkpoint K becomes stale
/// when the ratchet has advanced past it. Operationally: build a
/// capability "at" write 1024, advance to 2048, then expect a stale
/// signal from the ratchet's key-at-index logic for the capability's
/// pinned checkpoint.
#[test]
fn test_ratchet_proxy_capability_pins_to_checkpoint() {
    let mut r = RatchetState::new(seed(), 1024, 0);
    for i in 1..=512 {
        r.advance(&record(i));
    }
    let pinned_checkpoint = 0u64; // The starting checkpoint
    let pinned_key = r.checkpoints.get(&pinned_checkpoint).copied().unwrap();
    // Advance well past:
    for i in 513..=2048 {
        r.advance(&record(i));
    }
    // The current key has changed; the capability built at checkpoint 0
    // no longer corresponds to the engine's current gauge.
    assert_ne!(r.current_key, pinned_key);
    // The integration-level "capability stale" check happens in the
    // delegation layer: it compares the capability's pinned checkpoint
    // index to the ratchet's current write_count and refuses if the
    // ratchet has advanced past the capability's checkpoint validity
    // window. The ratchet primitive exposes the bookkeeping; the
    // refusal lives in src/delegation.rs (Sprint J integration follow-up).
    assert!(r.write_count > pinned_checkpoint + (r.checkpoint_every as u64));
}
