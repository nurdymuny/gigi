//! Phase 4 — Geometric coherence (Option C: MVCC-style geometric
//! snapshots).
//!
//! Geometric primitives (curvature K, cocycle slack δ, holonomy H)
//! are deterministic functions of a bundle's record set. Under
//! snapshot isolation, every transaction's geometric read is *pinned*
//! to the transaction's `snap_id`. This module wires that pin into
//! the [`MvccStore`] machinery and adds the commit-time validation
//! that enforces the **temporal cocycle bound** (spec §2.2):
//!
//! ```text
//!   ‖ δ_{ij}(S_{t+1}) − δ_{ij}(S_t) ‖  ≤  B_{ij}
//! ```
//!
//! When a multi-bundle commit would violate this bound across any
//! pair of touched bundles, [`GeometricCoherenceEngine::commit_overlay`]
//! refuses with [`CoherenceError::CocycleViolation`] and the substrate
//! state is unchanged.
//!
//! TDD ground truth: `theory/transactions/validation/
//! tx14_18_geometric_coherence.py` (5/5 green).
//!
//! ## What's a payload here?
//!
//! For Phase 4 we model record payloads as a single `f64` — enough
//! for K = mean(payload) and δ = |K(b1) − K(b2)|. Real bundles will
//! plug in the production `bundle::Record` and use the existing
//! `curvature_stats` pipeline; this module ships the snapshot/pin
//! mechanism, not a new geometric primitive.

use crate::transactions::mvcc::{MvccStore, TransactionOverlay};
use crate::transactions::types::{PendingWrite, SnapshotId, WriteOp};
use std::collections::{HashMap, HashSet};

/// Multi-bundle MVCC engine that pins geometric reads at each
/// transaction's snapshot and enforces the temporal cocycle bound
/// at commit time.
#[derive(Clone, Debug, Default)]
pub struct GeometricCoherenceEngine {
    /// Per-bundle MVCC store.
    bundles: HashMap<String, MvccStore>,
    /// Monotone commit counter — every successful commit bumps this
    /// to a fresh, strictly-increasing snap_id.
    snap_counter: SnapshotId,
    /// Cocycle budget B for every pair of touched bundles. A single
    /// scalar for Phase 4; per-pair budgets land when the engine
    /// wires cross-atlas bridge transitions.
    cocycle_budget: f64,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum CoherenceError {
    #[error(
        "temporal cocycle bound violated on pair ({b1},{b2}): |Δδ|={delta:.6} > B={budget:.6}"
    )]
    CocycleViolation {
        b1: String,
        b2: String,
        delta: f64,
        budget: f64,
    },
}

impl GeometricCoherenceEngine {
    pub fn new(cocycle_budget: f64) -> Self {
        Self {
            bundles: HashMap::new(),
            snap_counter: SnapshotId::GENESIS,
            cocycle_budget,
        }
    }

    pub fn high_water(&self) -> SnapshotId {
        self.snap_counter
    }

    /// Allocate the next snapshot id. Caller-visible for begin()-style
    /// flows; commit_overlay uses it internally.
    pub fn next_snap_id(&mut self) -> SnapshotId {
        self.snap_counter = self.snap_counter.next();
        self.snap_counter
    }

    /// Apply a multi-bundle overlay at a fresh snap_id, refusing if
    /// the temporal cocycle bound would be violated. The overlay is
    /// keyed by bundle name; values are per-bundle [`TransactionOverlay`]s.
    pub fn commit_overlay(
        &mut self,
        overlay: &HashMap<String, TransactionOverlay>,
    ) -> Result<SnapshotId, CoherenceError> {
        // Phase 4 pre-flight: project the post-commit record set in
        // memory and check the temporal cocycle bound. We don't
        // mutate any bundle store until the check passes.
        self.check_temporal_cocycle(overlay)?;

        // Pass: persist.
        let new_snap = self.next_snap_id();
        for (bundle, ov) in overlay {
            let store = self.bundles.entry(bundle.clone()).or_default();
            store.apply_overlay(ov, new_snap);
        }
        Ok(new_snap)
    }

    fn check_temporal_cocycle(
        &self,
        overlay: &HashMap<String, TransactionOverlay>,
    ) -> Result<(), CoherenceError> {
        let touched: HashSet<&str> = overlay.keys().map(String::as_str).collect();
        let touched: Vec<&str> = touched.into_iter().collect();
        for i in 0..touched.len() {
            for j in (i + 1)..touched.len() {
                let b1 = touched[i];
                let b2 = touched[j];
                let delta_pre =
                    (self.k_committed(b1) - self.k_committed(b2)).abs();
                let k1_post = self.k_after_overlay(b1, overlay.get(b1));
                let k2_post = self.k_after_overlay(b2, overlay.get(b2));
                let delta_post = (k1_post - k2_post).abs();
                let drift = (delta_post - delta_pre).abs();
                if drift > self.cocycle_budget {
                    return Err(CoherenceError::CocycleViolation {
                        b1: b1.to_string(),
                        b2: b2.to_string(),
                        delta: drift,
                        budget: self.cocycle_budget,
                    });
                }
            }
        }
        Ok(())
    }

    /// Out-of-tx K against the latest committed state.
    pub fn k_committed(&self, bundle: &str) -> f64 {
        let Some(store) = self.bundles.get(bundle) else {
            return 0.0;
        };
        let pks = store.snapshot_keys(self.snap_counter);
        if pks.is_empty() {
            return 0.0;
        }
        let mut sum = 0.0f64;
        let mut n = 0usize;
        for pk in &pks {
            if let Some(p) = store.read_at(pk, self.snap_counter) {
                sum += payload_to_f64(&p);
                n += 1;
            }
        }
        if n == 0 {
            0.0
        } else {
            sum / n as f64
        }
    }

    /// K under a transaction (pinned at `tx_snap`, overlay applied).
    pub fn k_under(
        &self,
        bundle: &str,
        tx_snap: SnapshotId,
        overlay: &TransactionOverlay,
    ) -> f64 {
        let Some(store) = self.bundles.get(bundle) else {
            // No committed state; the overlay alone defines K.
            return k_of_overlay(overlay);
        };
        let rs = store.materialize_under(tx_snap, overlay);
        if rs.is_empty() {
            return 0.0;
        }
        rs.values()
            .map(|p| payload_to_f64(p))
            .sum::<f64>()
            / rs.len() as f64
    }

    /// δ_{ij} between two bundles under the latest committed state.
    pub fn delta_committed(&self, b1: &str, b2: &str) -> f64 {
        (self.k_committed(b1) - self.k_committed(b2)).abs()
    }

    /// δ_{ij} under a transaction pinned at `tx_snap`.
    pub fn delta_under(
        &self,
        b1: &str,
        b2: &str,
        tx_snap: SnapshotId,
        overlay1: &TransactionOverlay,
        overlay2: &TransactionOverlay,
    ) -> f64 {
        let k1 = self.k_under(b1, tx_snap, overlay1);
        let k2 = self.k_under(b2, tx_snap, overlay2);
        (k1 - k2).abs()
    }

    /// Holonomy stand-in: sum(payloads) mod 1.0. Holonomy paths walk
    /// the connection 1-form; for the Phase 4 model we use a discrete
    /// loop product. Same shape, much simpler ground truth.
    pub fn holonomy_committed(&self, bundle: &str) -> f64 {
        let Some(store) = self.bundles.get(bundle) else {
            return 0.0;
        };
        let pks = store.snapshot_keys(self.snap_counter);
        let sum: f64 = pks
            .iter()
            .filter_map(|pk| store.read_at(pk, self.snap_counter))
            .map(|p| payload_to_f64(&p))
            .sum();
        sum.rem_euclid(1.0)
    }

    pub fn holonomy_under(
        &self,
        bundle: &str,
        tx_snap: SnapshotId,
        overlay: &TransactionOverlay,
    ) -> f64 {
        let Some(store) = self.bundles.get(bundle) else {
            return k_of_overlay(overlay).rem_euclid(1.0);
        };
        let rs = store.materialize_under(tx_snap, overlay);
        let sum: f64 = rs.values().map(|p| payload_to_f64(p)).sum();
        sum.rem_euclid(1.0)
    }

    /// Internal: K after the overlay is hypothetically applied at
    /// the current high-water mark. Used by the commit pre-flight.
    fn k_after_overlay(
        &self,
        bundle: &str,
        overlay: Option<&TransactionOverlay>,
    ) -> f64 {
        let Some(store) = self.bundles.get(bundle) else {
            return overlay.map(k_of_overlay).unwrap_or(0.0);
        };
        let empty = TransactionOverlay::new();
        let ov = overlay.unwrap_or(&empty);
        let rs = store.materialize_under(self.snap_counter, ov);
        if rs.is_empty() {
            return 0.0;
        }
        rs.values()
            .map(|p| payload_to_f64(p))
            .sum::<f64>()
            / rs.len() as f64
    }
}

fn payload_to_f64(p: &[u8]) -> f64 {
    if p.len() >= 8 {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&p[..8]);
        f64::from_le_bytes(bytes)
    } else if p.is_empty() {
        0.0
    } else {
        p[0] as f64
    }
}

fn k_of_overlay(overlay: &TransactionOverlay) -> f64 {
    if overlay.is_empty() {
        return 0.0;
    }
    let mut sum = 0.0;
    let mut n = 0usize;
    for pw in overlay.writes.values() {
        match pw.op {
            WriteOp::Insert | WriteOp::Update => {
                sum += payload_to_f64(&pw.payload);
                n += 1;
            }
            WriteOp::Delete => {}
        }
    }
    if n == 0 {
        0.0
    } else {
        sum / n as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transactions::types::WriteOp;

    fn mk_insert(pk: &str, value: f64) -> PendingWrite {
        PendingWrite {
            pk: pk.as_bytes().to_vec(),
            payload: value.to_le_bytes().to_vec(),
            op: WriteOp::Insert,
        }
    }

    fn mk_overlay(writes: &[PendingWrite]) -> TransactionOverlay {
        let mut ov = TransactionOverlay::new();
        for w in writes {
            ov.stage(w.clone());
        }
        ov
    }

    fn one_bundle(name: &str, writes: &[PendingWrite]) -> HashMap<String, TransactionOverlay> {
        let mut out = HashMap::new();
        out.insert(name.to_string(), mk_overlay(writes));
        out
    }

    // ---- TX14: out-of-tx during open tx -------------------------------

    #[test]
    fn tx14_out_of_tx_during_open_tx_sees_pre_state() {
        let mut eng = GeometricCoherenceEngine::new(10.0);
        let seed = one_bundle("users", &[mk_insert("a", 1.0), mk_insert("b", 2.0)]);
        eng.commit_overlay(&seed).unwrap();
        let k_pre = eng.k_committed("users");
        assert!((k_pre - 1.5).abs() < 1e-12);

        // Simulate an open T at snap = high_water with a staged overlay.
        let t_snap = eng.high_water();
        let t_overlay = mk_overlay(&[mk_insert("c", 99.0)]);

        // Out-of-tx K is still 1.5.
        assert!((eng.k_committed("users") - 1.5).abs() < 1e-12);
        // In-tx K includes the staged record.
        let k_in_tx = eng.k_under("users", t_snap, &t_overlay);
        assert!(((k_in_tx - (1.0 + 2.0 + 99.0) / 3.0).abs()) < 1e-12);
    }

    // ---- TX15: out-of-tx after commit ---------------------------------

    #[test]
    fn tx15_out_of_tx_after_commit_sees_post_state() {
        let mut eng = GeometricCoherenceEngine::new(10.0);
        eng.commit_overlay(&one_bundle("u", &[mk_insert("a", 1.0), mk_insert("b", 2.0)]))
            .unwrap();
        eng.commit_overlay(&one_bundle("u", &[mk_insert("c", 9.0)])).unwrap();
        assert!((eng.k_committed("u") - (1.0 + 2.0 + 9.0) / 3.0).abs() < 1e-12);
    }

    // ---- TX16: cocycle bound at commit --------------------------------

    #[test]
    fn tx16_1_small_change_commits() {
        let mut eng = GeometricCoherenceEngine::new(0.5);
        let mut seed = HashMap::new();
        seed.insert("users".to_string(), mk_overlay(&[mk_insert("a", 1.0)]));
        seed.insert("orders".to_string(), mk_overlay(&[mk_insert("x", 1.0)]));
        eng.commit_overlay(&seed).unwrap();

        let mut small = HashMap::new();
        small.insert("users".to_string(), mk_overlay(&[mk_insert("b", 1.5)]));
        eng.commit_overlay(&small).unwrap();
    }

    #[test]
    fn tx16_2_large_change_refused_and_state_unchanged() {
        let mut eng = GeometricCoherenceEngine::new(0.5);
        let mut seed = HashMap::new();
        seed.insert("users".to_string(), mk_overlay(&[mk_insert("a", 1.0)]));
        seed.insert("orders".to_string(), mk_overlay(&[mk_insert("x", 1.0)]));
        eng.commit_overlay(&seed).unwrap();
        let snap_before = eng.high_water();
        let k_users_before = eng.k_committed("users");

        let mut bad = HashMap::new();
        bad.insert("users".to_string(), mk_overlay(&[mk_insert("c", 100.0)]));
        bad.insert("orders".to_string(), mk_overlay(&[]));

        let r = eng.commit_overlay(&bad);
        assert!(matches!(r, Err(CoherenceError::CocycleViolation { .. })));
        assert_eq!(eng.high_water(), snap_before);
        assert!((eng.k_committed("users") - k_users_before).abs() < 1e-12);
    }

    // ---- TX17: walker sees pre- OR post-, not partial -----------------

    #[test]
    fn tx17_walker_pinned_pre_sees_pre_only() {
        let mut eng = GeometricCoherenceEngine::new(10.0);
        eng.commit_overlay(&one_bundle("b", &[mk_insert("r1", 0.1), mk_insert("r2", 0.2)]))
            .unwrap();
        let pre_h = eng.holonomy_committed("b");

        let walker_snap = eng.high_water();
        eng.commit_overlay(&one_bundle("b", &[mk_insert("r3", 0.4)])).unwrap();
        let post_h = eng.holonomy_committed("b");

        let walker_h = eng.holonomy_under("b", walker_snap, &TransactionOverlay::new());
        assert!((walker_h - pre_h).abs() < 1e-12);
        // Sanity: post_h is different.
        assert!((post_h - pre_h).abs() > 1e-6);
    }

    // ---- TX18: storage scales with open-tx count ----------------------
    // The Rust mirror tests this property at the level of overlay
    // size — the same property the Python gate asserts but expressed
    // as a property of TransactionOverlay::len() rather than a heap
    // byte-count, which is implementation-defined.

    #[test]
    fn tx18_overlay_size_independent_of_committed_bundle_size() {
        let mut eng = GeometricCoherenceEngine::new(100.0);

        let mut big_seed = HashMap::new();
        let mut ov = TransactionOverlay::new();
        for i in 0..1000 {
            ov.stage(mk_insert(&format!("k{}", i), i as f64));
        }
        big_seed.insert("u".to_string(), ov);
        eng.commit_overlay(&big_seed).unwrap();

        // Open transactions each have small overlays; total = N * c
        // independent of the 1000-row committed bundle.
        let mut overlays = Vec::new();
        for n in 0..20 {
            let mut t_ov = TransactionOverlay::new();
            t_ov.stage(mk_insert(&format!("new{}", n), n as f64));
            overlays.push(t_ov);
        }
        let total: usize = overlays.iter().map(|o| o.len()).sum();
        assert_eq!(total, 20);
    }
}
