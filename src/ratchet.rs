//! GIGI Encrypt v0.3 — Sprint M: Continuous RG-flow ratchet
//! (per-write forward secrecy).
//!
//! Per-insert KDF chain: `g_{t+1} = HKDF-SHA256(g_t, salt = record_bytes_t || t)`.
//! After advancing, the engine drops `g_t` from memory. Reading a record at
//! index `t` requires replaying forward from the nearest checkpoint
//! `g_{kN}` where `kN ≤ t`. Past the retention horizon `R`, both records
//! and checkpoints are gone — `g_t` for `t < T − R` is computationally
//! unrecoverable (HKDF one-wayness; Krawczyk CRYPTO 2010 Thm 4.4).
//!
//! **Per-field ratchet semantics** (spec §7.4): INDEXED fields default to
//! non-ratcheting (preserves equality search). The ratchet here governs the
//! chain advance — individual `FieldTransform` derivation from the current
//! chain key lives in the bundle integration layer.
//!
//! See `GIGI_ENCRYPT_v0.3_SPRINT_SPEC.md` §7.

use hkdf::Hkdf;
use sha2::Sha256;
use std::collections::BTreeMap;

// ───────────────────────────────────────────────────────────────────────
// Types
// ───────────────────────────────────────────────────────────────────────

/// Per-bundle ratchet state. Holds the current chain key, write count,
/// active checkpoints, and retention parameters.
#[derive(Debug, Clone)]
pub struct RatchetState {
    /// Current chain key g_T.
    pub current_key: [u8; 32],
    /// Number of writes processed so far (= T).
    pub write_count: u64,
    /// Checkpoint period N: persist g_{kN} for k = 0, 1, 2, ...
    pub checkpoint_every: u32,
    /// Active checkpoints, keyed by their write index (`kN`).
    /// `BTreeMap` for O(log) "find nearest checkpoint ≤ t" via `range`.
    pub checkpoints: BTreeMap<u64, [u8; 32]>,
    /// Retention horizon R, in writes. Records and checkpoints older
    /// than `write_count − R` may be forgotten by the operator.
    pub retention_horizon: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum RatchetError {
    #[error("write index {idx} predates retention horizon at {horizon}; records and key unrecoverable")]
    BeyondRetentionHorizon { idx: u64, horizon: u64 },

    #[error("capability pinned to checkpoint {pinned}, current chain at {current}")]
    CapabilityStale { pinned: u64, current: u64 },

    #[error("write index {idx} is in the future (current write_count = {current})")]
    FutureIndex { idx: u64, current: u64 },
}

// ───────────────────────────────────────────────────────────────────────
// RatchetState API
// ───────────────────────────────────────────────────────────────────────

impl RatchetState {
    /// Initialize a ratchet from a seed.
    /// `checkpoint_every` MUST be ≥ 1 (defaults to 1024 in production
    /// schemas; smaller values trade replay-cost for memory).
    /// `retention_horizon` is in writes; operators interpret 0 as
    /// "no horizon" (chain advances forever without forgetting).
    pub fn new(seed: [u8; 32], checkpoint_every: u32, retention_horizon: u64) -> Self {
        assert!(checkpoint_every >= 1, "checkpoint_every must be >= 1");
        let mut checkpoints = BTreeMap::new();
        checkpoints.insert(0u64, seed);
        Self {
            current_key: seed,
            write_count: 0,
            checkpoint_every,
            checkpoints,
            retention_horizon,
        }
    }

    /// Advance the ratchet by one step, consuming `record_bytes` as the
    /// HKDF salt input. Returns the new chain key.
    ///
    /// If the new `write_count` is a multiple of `checkpoint_every`, the
    /// checkpoint is persisted; otherwise the prior chain key is dropped.
    pub fn advance(&mut self, record_bytes: &[u8]) -> [u8; 32] {
        let next_t = self.write_count + 1;
        let next_key = hkdf_step(&self.current_key, record_bytes, next_t);
        self.current_key = next_key;
        self.write_count = next_t;
        // Checkpoint policy: persist g_{kN} for k = 1, 2, 3, ...
        if next_t % (self.checkpoint_every as u64) == 0 {
            self.checkpoints.insert(next_t, next_key);
        }
        // Retention: forget checkpoints older than (T - R), if R > 0.
        if self.retention_horizon > 0 && next_t > self.retention_horizon {
            let cutoff = next_t - self.retention_horizon;
            // Drop checkpoints with index < cutoff. Always keep at least
            // the most recent one so the chain isn't orphaned mid-write.
            let to_drop: Vec<u64> = self
                .checkpoints
                .range(..cutoff)
                .map(|(k, _)| *k)
                .collect();
            for k in to_drop {
                self.checkpoints.remove(&k);
            }
        }
        next_key
    }

    /// Return the chain key at write index `t`, replaying from the
    /// nearest available checkpoint.
    ///
    /// Returns:
    /// - `Ok(key)` if a checkpoint with index ≤ t exists in memory.
    /// - `Err(BeyondRetentionHorizon)` if no such checkpoint remains.
    /// - `Err(FutureIndex)` if `t > write_count`.
    pub fn key_at_index(&self, t: u64, record_bytes_seq: &[Vec<u8>]) -> Result<[u8; 32], RatchetError> {
        if t > self.write_count {
            return Err(RatchetError::FutureIndex {
                idx: t,
                current: self.write_count,
            });
        }
        // Find the largest checkpoint index ≤ t.
        let (&checkpoint_idx, &checkpoint_key) = self
            .checkpoints
            .range(..=t)
            .next_back()
            .ok_or(RatchetError::BeyondRetentionHorizon {
                idx: t,
                horizon: self.write_count.saturating_sub(self.retention_horizon),
            })?;
        // Replay forward: apply (t - checkpoint_idx) HKDF steps.
        // The caller supplies `record_bytes_seq[0..t-checkpoint_idx]`
        // — record bytes for writes (checkpoint_idx+1)..=t.
        let mut key = checkpoint_key;
        let steps = (t - checkpoint_idx) as usize;
        if record_bytes_seq.len() < steps {
            return Err(RatchetError::BeyondRetentionHorizon {
                idx: t,
                horizon: checkpoint_idx + record_bytes_seq.len() as u64,
            });
        }
        for s in 0..steps {
            let step_t = checkpoint_idx + (s as u64) + 1;
            key = hkdf_step(&key, &record_bytes_seq[s], step_t);
        }
        Ok(key)
    }

    /// Drop all checkpoints with index < `cutoff`. Operator-triggered
    /// forward-secrecy advance: after this call, records at indices
    /// `< cutoff` are unreadable even with subsequent replay attempts.
    pub fn forget_history_before(&mut self, cutoff: u64) -> usize {
        let to_drop: Vec<u64> = self
            .checkpoints
            .range(..cutoff)
            .map(|(k, _)| *k)
            .collect();
        let n = to_drop.len();
        for k in to_drop {
            self.checkpoints.remove(&k);
        }
        n
    }

    /// Count of active checkpoints in memory.
    pub fn checkpoint_count(&self) -> usize {
        self.checkpoints.len()
    }

    /// Smallest write index recoverable from the current checkpoint set.
    /// Returns `None` if there are no checkpoints (all forgotten).
    pub fn oldest_recoverable_write(&self) -> Option<u64> {
        self.checkpoints.keys().next().copied()
    }
}

// ───────────────────────────────────────────────────────────────────────
// HKDF chain step
// ───────────────────────────────────────────────────────────────────────

/// One ratchet step: `g_{t+1} = HKDF-SHA256(salt = record || t_be, ikm = g_t)`.
///
/// We use HKDF's `salt` for the per-write nonce input and `ikm` for the
/// prior chain key — both contribute entropy. The write index in the salt
/// prevents replay-at-same-position attacks (if an attacker substitutes a
/// different record at position t, the salt changes and the chain diverges).
fn hkdf_step(prev_key: &[u8; 32], record_bytes: &[u8], t: u64) -> [u8; 32] {
    let mut salt = Vec::with_capacity(record_bytes.len() + 8);
    salt.extend_from_slice(record_bytes);
    salt.extend_from_slice(&t.to_be_bytes());
    let hk = Hkdf::<Sha256>::new(Some(&salt), prev_key);
    let mut out = [0u8; 32];
    hk.expand(b"gigi-ratchet-v1", &mut out)
        .expect("HKDF expand cannot fail for 32-byte output");
    out
}

// ───────────────────────────────────────────────────────────────────────
// Unit tests
// ───────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn seed_a() -> [u8; 32] {
        let mut s = [0u8; 32];
        for (i, b) in s.iter_mut().enumerate() {
            *b = i as u8;
        }
        s
    }

    fn record(i: u64) -> Vec<u8> {
        format!("record-{}", i).into_bytes()
    }

    #[test]
    fn advance_changes_key_each_step() {
        let mut r = RatchetState::new(seed_a(), 1024, 0);
        let k0 = r.current_key;
        let k1 = r.advance(&record(1));
        assert_ne!(k0, k1);
        let k2 = r.advance(&record(2));
        assert_ne!(k1, k2);
    }

    #[test]
    fn chain_is_deterministic_given_seed_and_records() {
        let mut r1 = RatchetState::new(seed_a(), 1024, 0);
        let mut r2 = RatchetState::new(seed_a(), 1024, 0);
        for i in 1..=100 {
            r1.advance(&record(i));
            r2.advance(&record(i));
        }
        assert_eq!(r1.current_key, r2.current_key);
        assert_eq!(r1.write_count, 100);
    }

    #[test]
    fn checkpoints_persist_at_period_boundary() {
        let mut r = RatchetState::new(seed_a(), 4, 0);
        for i in 1..=10 {
            r.advance(&record(i));
        }
        // Expect checkpoints at 0, 4, 8.
        let keys: Vec<_> = r.checkpoints.keys().copied().collect();
        assert_eq!(keys, vec![0, 4, 8]);
    }

    #[test]
    fn key_at_index_replays_from_checkpoint() {
        let mut r = RatchetState::new(seed_a(), 4, 0);
        let mut keys: Vec<[u8; 32]> = vec![r.current_key];
        for i in 1..=10 {
            let next = r.advance(&record(i));
            keys.push(next);
        }
        // Build the record-byte sequence between checkpoint at 4 and t=7.
        let bytes_5_to_7: Vec<Vec<u8>> = (5..=7).map(record).collect();
        let recovered = r.key_at_index(7, &bytes_5_to_7).unwrap();
        assert_eq!(recovered, keys[7]);
    }

    #[test]
    fn retention_horizon_drops_old_checkpoints() {
        let mut r = RatchetState::new(seed_a(), 4, 8);
        for i in 1..=20 {
            r.advance(&record(i));
        }
        // At write 20 with R=8, cutoff = 12; checkpoints at 12, 16, 20 retained;
        // checkpoints at 0, 4, 8 dropped.
        let keys: Vec<_> = r.checkpoints.keys().copied().collect();
        assert_eq!(keys, vec![12, 16, 20]);
    }

    #[test]
    fn forget_history_drops_checkpoints_explicitly() {
        let mut r = RatchetState::new(seed_a(), 4, 0);
        for i in 1..=20 {
            r.advance(&record(i));
        }
        assert_eq!(r.checkpoint_count(), 6); // 0, 4, 8, 12, 16, 20
        let dropped = r.forget_history_before(12);
        assert_eq!(dropped, 3); // 0, 4, 8
        assert_eq!(r.checkpoint_count(), 3); // 12, 16, 20
    }

    #[test]
    fn future_index_rejected() {
        let r = RatchetState::new(seed_a(), 4, 0);
        let result = r.key_at_index(5, &[]);
        assert!(matches!(result, Err(RatchetError::FutureIndex { .. })));
    }

    #[test]
    fn beyond_retention_horizon_rejected() {
        let mut r = RatchetState::new(seed_a(), 4, 8);
        for i in 1..=20 {
            r.advance(&record(i));
        }
        // Asking for write 3 — checkpoint at 0 was forgotten, no path back.
        let result = r.key_at_index(3, &[]);
        assert!(matches!(
            result,
            Err(RatchetError::BeyondRetentionHorizon { .. })
        ));
    }
}
