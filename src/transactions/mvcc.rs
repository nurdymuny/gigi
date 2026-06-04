//! Phase 2 — Multi-version concurrency control + snapshot isolation.
//!
//! Ships the in-memory snapshot-isolation engine that backs the
//! atomic-sheaf-commit story:
//!
//! - [`VersionChain`] holds the linear history of one primary key as
//!   `[Version { payload, commit_snap_id }, ...]`.
//! - [`MvccStore`] is the per-bundle store: `pk -> VersionChain`.
//! - [`TransactionOverlay`] is the per-transaction staging area; reads
//!   under a transaction consult the overlay first, then the snapshot
//!   at the transaction's pinned [`SnapshotId`].
//! - [`MvccStore::gc`] implements the §4.3 garbage-collection contract:
//!   a non-latest version is collectable iff its *next* version's
//!   commit_snap_id is at or below the lowest open-transaction snap.
//!
//! The TDD ground truth lives in
//! `theory/transactions/validation/tx6_8_snapshot_isolation.py`,
//! `tx9_mvcc_gc.py`, and `tx10_geometric_reads_under_tx.py`. The Rust
//! tests below mirror those gates one-for-one.
//!
//! ## Where Phase 1 sits
//!
//! Phase 1's [`crate::transactions::Participant`] kept a single
//! `HashMap<Vec<u8>, Vec<u8>>` of committed state. Phase 2 supersedes
//! that with [`MvccStore`]. [`crate::transactions::Participant`] grows
//! a `mvcc: MvccStore` field; existing tests rebuild against the
//! single-version view (`MvccStore::read_at(pk, +inf)`).

use crate::transactions::types::{PendingWrite, SnapshotId, TransactionId, WriteOp};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// One committed value of a key, visible to readers whose
/// `snap_id >= commit_snap_id`.
///
/// A `payload == None` version is a *tombstone* — the key is deleted
/// as of `commit_snap_id`. Tombstones are versions in their own right;
/// they obey the same GC rules.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Version {
    pub payload: Option<Vec<u8>>,
    pub commit_snap_id: SnapshotId,
}

/// The append-only history of one primary key. Versions are stored
/// sorted by `commit_snap_id` ascending. [`MvccStore::commit_tx`]
/// preserves the invariant.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionChain {
    pub versions: Vec<Version>,
}

impl VersionChain {
    /// The latest version with `commit_snap_id <= snap`, or `None` when
    /// nothing in the chain is visible at this snapshot.
    pub fn read_at(&self, snap: SnapshotId) -> Option<&Version> {
        let mut latest: Option<&Version> = None;
        for v in &self.versions {
            if v.commit_snap_id <= snap {
                latest = Some(match latest {
                    Some(prev) if prev.commit_snap_id >= v.commit_snap_id => prev,
                    _ => v,
                });
            }
        }
        latest
    }
}

/// The pending writes a transaction has staged but not yet committed.
/// Keyed by primary key; the latest staged value wins (overlay
/// shadows itself).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TransactionOverlay {
    pub writes: HashMap<Vec<u8>, PendingWrite>,
}

impl TransactionOverlay {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn stage(&mut self, write: PendingWrite) {
        // The pending writes' `pk` is the canonical key.
        self.writes.insert(write.pk.clone(), write);
    }

    pub fn is_empty(&self) -> bool {
        self.writes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.writes.len()
    }

    /// Owned lookup. Returns:
    /// - `Some(Some(payload))` — overlay has an insert/update
    /// - `Some(None)` — overlay has a tombstone
    /// - `None` — no overlay entry; consult snapshot
    pub fn lookup_owned(&self, pk: &[u8]) -> Option<Option<Vec<u8>>> {
        self.writes.get(pk).map(|pw| match pw.op {
            WriteOp::Insert | WriteOp::Update => Some(pw.payload.clone()),
            WriteOp::Delete => None,
        })
    }
}

/// Per-bundle multi-version store. Holds the version chain for every
/// primary key that has ever been written, plus the rolling counter of
/// the highest committed snap_id (used as the out-of-tx read horizon).
#[derive(Clone, Debug, Default)]
pub struct MvccStore {
    /// `pk -> VersionChain`. New chains are inserted lazily on first
    /// commit; chains never get removed even after every version is
    /// GC'd (their absence is captured by `chain.versions.is_empty()`).
    chains: HashMap<Vec<u8>, VersionChain>,
    /// Highest committed snapshot in this store. Out-of-tx reads use
    /// this as their read horizon.
    high_water: SnapshotId,
}

impl MvccStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn high_water(&self) -> SnapshotId {
        self.high_water
    }

    /// Apply an overlay to the store at `commit_snap_id`. Overlay
    /// inserts/updates append `Version { Some(payload), snap }`;
    /// deletes append `Version { None, snap }` (tombstone).
    ///
    /// The caller is responsible for ensuring `commit_snap_id` is a
    /// fresh, strictly-increasing snapshot id.
    pub fn apply_overlay(&mut self, overlay: &TransactionOverlay, commit_snap_id: SnapshotId) {
        for pw in overlay.writes.values() {
            let payload = match pw.op {
                WriteOp::Insert | WriteOp::Update => Some(pw.payload.clone()),
                WriteOp::Delete => None,
            };
            let chain = self.chains.entry(pw.pk.clone()).or_default();
            chain.versions.push(Version {
                payload,
                commit_snap_id,
            });
        }
        if commit_snap_id > self.high_water {
            self.high_water = commit_snap_id;
        }
    }

    /// Out-of-tx read: the latest committed payload of `pk`, or `None`
    /// for a missing-or-tombstoned key.
    pub fn read_committed(&self, pk: &[u8]) -> Option<Vec<u8>> {
        let chain = self.chains.get(pk)?;
        chain
            .read_at(self.high_water)
            .and_then(|v| v.payload.clone())
    }

    /// Snapshot read at a specific `snap_id`. None means missing or
    /// tombstoned at this snapshot.
    pub fn read_at(&self, pk: &[u8], snap: SnapshotId) -> Option<Vec<u8>> {
        let chain = self.chains.get(pk)?;
        chain.read_at(snap).and_then(|v| v.payload.clone())
    }

    /// In-tx read: overlay shadows; otherwise read at `tx_snap`.
    /// Returns `None` for "missing or tombstoned at this view".
    pub fn read_under(
        &self,
        pk: &[u8],
        tx_snap: SnapshotId,
        overlay: &TransactionOverlay,
    ) -> Option<Vec<u8>> {
        match overlay.lookup_owned(pk) {
            Some(Some(payload)) => Some(payload),
            Some(None) => None,
            None => self.read_at(pk, tx_snap),
        }
    }

    /// The set of primary keys visible at `snap` (after tombstones).
    /// Used by geometric reads to materialize the snapshot's record
    /// set. Allocates; intended for reads, not hot paths.
    pub fn snapshot_keys(&self, snap: SnapshotId) -> Vec<Vec<u8>> {
        let mut out = Vec::new();
        for (pk, chain) in &self.chains {
            if let Some(v) = chain.read_at(snap) {
                if v.payload.is_some() {
                    out.push(pk.clone());
                }
            }
        }
        out
    }

    /// The materialized snapshot at `snap`, with the overlay applied.
    /// Tombstones in the overlay remove entries; insert/update overlay
    /// entries override the snapshot value. Used by geometric reads
    /// under a transaction (TX10).
    pub fn materialize_under(
        &self,
        snap: SnapshotId,
        overlay: &TransactionOverlay,
    ) -> HashMap<Vec<u8>, Vec<u8>> {
        let mut out = HashMap::new();
        for (pk, chain) in &self.chains {
            if let Some(v) = chain.read_at(snap) {
                if let Some(p) = &v.payload {
                    out.insert(pk.clone(), p.clone());
                }
            }
        }
        for (pk, pw) in &overlay.writes {
            match pw.op {
                WriteOp::Insert | WriteOp::Update => {
                    out.insert(pk.clone(), pw.payload.clone());
                }
                WriteOp::Delete => {
                    out.remove(pk);
                }
            }
        }
        out
    }

    /// Garbage-collect non-latest versions that no open transaction
    /// can see. Returns the number of versions removed. See §4.3 of
    /// the spec.
    ///
    /// `open_snaps` is the set of snap_ids of currently-open
    /// transactions. If empty, every non-latest version is collected.
    pub fn gc(&mut self, open_snaps: &[SnapshotId]) -> usize {
        let frontier: Option<SnapshotId> = open_snaps.iter().copied().min();
        let mut removed = 0usize;
        for chain in self.chains.values_mut() {
            if chain.versions.len() <= 1 {
                continue;
            }
            chain.versions.sort_by_key(|v| v.commit_snap_id);
            let mut kept: Vec<Version> = Vec::with_capacity(chain.versions.len());
            let n = chain.versions.len();
            for (i, v) in chain.versions.iter().enumerate() {
                let is_latest = i == n - 1;
                if is_latest {
                    kept.push(v.clone());
                    continue;
                }
                let v_next = &chain.versions[i + 1];
                let collectable = match frontier {
                    None => true,
                    Some(f) => v_next.commit_snap_id <= f,
                };
                if collectable {
                    removed += 1;
                } else {
                    kept.push(v.clone());
                }
            }
            chain.versions = kept;
        }
        removed
    }

    /// The set of distinct primary keys ever touched (live OR
    /// tombstoned, even if currently GC'd). Diagnostic helper.
    #[allow(dead_code)]
    pub fn pk_count(&self) -> usize {
        self.chains.len()
    }
}

/// Helper that derives a [`PendingWrite`] from a `TransactionId` and a
/// staged key/value pair. Sugar for tests and direct callers.
#[allow(dead_code)]
pub(crate) fn insert_pending(_tx: TransactionId, pk: Vec<u8>, payload: Vec<u8>) -> PendingWrite {
    PendingWrite {
        pk,
        payload,
        op: WriteOp::Insert,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transactions::types::WriteOp;

    fn snap(n: u64) -> SnapshotId {
        SnapshotId(n)
    }

    fn pk(s: &str) -> Vec<u8> {
        s.as_bytes().to_vec()
    }

    fn insert(pk_s: &str, value: u8) -> PendingWrite {
        PendingWrite {
            pk: pk(pk_s),
            payload: vec![value],
            op: WriteOp::Insert,
        }
    }

    fn delete(pk_s: &str) -> PendingWrite {
        PendingWrite {
            pk: pk(pk_s),
            payload: vec![],
            op: WriteOp::Delete,
        }
    }

    // ---- VersionChain ---------------------------------------------------

    #[test]
    fn version_chain_read_at_picks_latest_le_snap() {
        let mut chain = VersionChain::default();
        chain.versions.push(Version {
            payload: Some(vec![1]),
            commit_snap_id: snap(1),
        });
        chain.versions.push(Version {
            payload: Some(vec![2]),
            commit_snap_id: snap(3),
        });
        chain.versions.push(Version {
            payload: Some(vec![3]),
            commit_snap_id: snap(5),
        });

        assert_eq!(chain.read_at(snap(0)), None);
        assert_eq!(chain.read_at(snap(1)).unwrap().payload, Some(vec![1]));
        assert_eq!(chain.read_at(snap(2)).unwrap().payload, Some(vec![1]));
        assert_eq!(chain.read_at(snap(3)).unwrap().payload, Some(vec![2]));
        assert_eq!(chain.read_at(snap(10)).unwrap().payload, Some(vec![3]));
    }

    // ---- TX6: read-your-own-writes -------------------------------------

    #[test]
    fn tx6_own_insert_visible_under_tx() {
        let mut store = MvccStore::new();
        let seed = TransactionOverlay {
            writes: [(pk("k1"), insert("k1", 0x55))].into_iter().collect(),
        };
        store.apply_overlay(&seed, snap(1));

        let mut overlay = TransactionOverlay::new();
        overlay.stage(insert("k_new", 0x77));

        assert_eq!(
            store.read_under(b"k_new", snap(1), &overlay),
            Some(vec![0x77])
        );
        assert_eq!(store.read_committed(b"k_new"), None);
    }

    #[test]
    fn tx6_own_update_visible_under_tx() {
        let mut store = MvccStore::new();
        let seed = TransactionOverlay {
            writes: [(pk("a"), insert("a", 1))].into_iter().collect(),
        };
        store.apply_overlay(&seed, snap(1));

        let mut overlay = TransactionOverlay::new();
        overlay.stage(insert("a", 99));

        assert_eq!(store.read_under(b"a", snap(1), &overlay), Some(vec![99]));
    }

    #[test]
    fn tx6_own_delete_tombstones_under_tx() {
        let mut store = MvccStore::new();
        let seed = TransactionOverlay {
            writes: [(pk("a"), insert("a", 1))].into_iter().collect(),
        };
        store.apply_overlay(&seed, snap(1));

        let mut overlay = TransactionOverlay::new();
        overlay.stage(delete("a"));

        assert_eq!(store.read_under(b"a", snap(1), &overlay), None);
        // Out-of-tx still sees the pre-tombstone value.
        assert_eq!(store.read_committed(b"a"), Some(vec![1]));
    }

    // ---- TX7: concurrent overlays are invisible ------------------------

    #[test]
    fn tx7_concurrent_overlay_invisible() {
        let mut store = MvccStore::new();
        let seed = TransactionOverlay {
            writes: [(pk("a"), insert("a", 1))].into_iter().collect(),
        };
        store.apply_overlay(&seed, snap(1));

        // T1 stages but does not commit.
        let mut t1 = TransactionOverlay::new();
        t1.stage(insert("a", 99));
        t1.stage(insert("new_from_t1", 200));

        // T2 has its own (empty) overlay.
        let t2 = TransactionOverlay::new();

        assert_eq!(store.read_under(b"a", snap(1), &t2), Some(vec![1]));
        assert_eq!(store.read_under(b"new_from_t1", snap(1), &t2), None);
    }

    // ---- TX8: snapshot pin survives a concurrent commit ----------------

    #[test]
    fn tx8_snapshot_pin_survives_concurrent_commit() {
        let mut store = MvccStore::new();
        let seed = TransactionOverlay {
            writes: [(pk("x"), insert("x", 1))].into_iter().collect(),
        };
        store.apply_overlay(&seed, snap(1));

        // T2's pinned snap is 1.
        let tx2_snap = snap(1);

        // T1 commits at snap=2: x = 99.
        let t1_overlay = TransactionOverlay {
            writes: [(pk("x"), insert("x", 99))].into_iter().collect(),
        };
        store.apply_overlay(&t1_overlay, snap(2));

        // T2 still reads x = 1 at its pinned snap.
        let t2_overlay = TransactionOverlay::new();
        assert_eq!(store.read_under(b"x", tx2_snap, &t2_overlay), Some(vec![1]));
        // Out-of-tx reads see 99.
        assert_eq!(store.read_committed(b"x"), Some(vec![99]));
    }

    // ---- TX9: GC correctness -------------------------------------------

    #[test]
    fn tx9_1_no_open_tx_collapses_chain() {
        let mut store = MvccStore::new();
        for (i, v) in [b"k0", b"k1", b"k2", b"k3"].iter().enumerate() {
            let ov = TransactionOverlay {
                writes: [(pk("k"), PendingWrite {
                    pk: pk("k"),
                    payload: v.to_vec(),
                    op: WriteOp::Insert,
                })].into_iter().collect(),
            };
            store.apply_overlay(&ov, snap(i as u64 + 1));
        }

        let removed = store.gc(&[]);
        assert_eq!(removed, 3);
        assert_eq!(store.read_committed(b"k"), Some(b"k3".to_vec()));
    }

    #[test]
    fn tx9_2_one_open_tx_pins_chain() {
        let mut store = MvccStore::new();
        for (i, v) in [b"k0", b"k1", b"k2", b"k3"].iter().enumerate() {
            let ov = TransactionOverlay {
                writes: [(pk("k"), PendingWrite {
                    pk: pk("k"),
                    payload: v.to_vec(),
                    op: WriteOp::Insert,
                })].into_iter().collect(),
            };
            store.apply_overlay(&ov, snap(i as u64 + 1));
        }
        // open tx at snap=2: reads k1.
        let pinned_snap = snap(2);
        assert_eq!(
            store.read_at(b"k", pinned_snap),
            Some(b"k1".to_vec())
        );

        let removed = store.gc(&[pinned_snap]);
        // v=(1,k0), v_next=(2,k1). 2<=2 -> collect.
        // v=(2,k1), v_next=(3,k2). 3>2 -> retain.
        // v=(3,k2), v_next=(4,k3). 4>2 -> retain.
        // v=(4,k3), latest -> retain.
        assert_eq!(removed, 1);
        // Pinned reader still sees k1.
        assert_eq!(store.read_at(b"k", pinned_snap), Some(b"k1".to_vec()));
        assert_eq!(store.read_committed(b"k"), Some(b"k3".to_vec()));
    }

    #[test]
    fn tx9_3_two_open_txs_frontier_is_min() {
        let mut store = MvccStore::new();
        for i in 0..6 {
            let v = format!("k{}", i).into_bytes();
            let ov = TransactionOverlay {
                writes: [(pk("k"), PendingWrite {
                    pk: pk("k"),
                    payload: v,
                    op: WriteOp::Insert,
                })].into_iter().collect(),
            };
            store.apply_overlay(&ov, snap(i + 1));
        }
        let s_early = snap(2);
        let s_late = snap(4);
        let removed = store.gc(&[s_early, s_late]);
        // Frontier=2. Only v=(1,k0) is collectable.
        assert_eq!(removed, 1);
        assert_eq!(store.read_at(b"k", s_early), Some(b"k1".to_vec()));
        assert_eq!(store.read_at(b"k", s_late), Some(b"k3".to_vec()));
    }

    #[test]
    fn tx9_4_close_all_collapses_chain() {
        let mut store = MvccStore::new();
        for (i, v) in [b"a", b"b", b"c", b"d"].iter().enumerate() {
            let ov = TransactionOverlay {
                writes: [(pk("k"), PendingWrite {
                    pk: pk("k"),
                    payload: v.to_vec(),
                    op: WriteOp::Insert,
                })].into_iter().collect(),
            };
            store.apply_overlay(&ov, snap(i as u64 + 1));
        }
        let _ = store.gc(&[snap(2)]); // one open tx
        // After closing, frontier collapses to None.
        let r2 = store.gc(&[]);
        // Remaining versions: snaps 2,3,4; remove 2,3.
        assert_eq!(r2, 2);
        assert_eq!(store.read_committed(b"k"), Some(b"d".to_vec()));
    }

    #[test]
    fn tx9_5_tombstones_survive_gc_as_latest() {
        let mut store = MvccStore::new();
        let ov_insert = TransactionOverlay {
            writes: [(pk("k"), insert("k", 0x42))].into_iter().collect(),
        };
        store.apply_overlay(&ov_insert, snap(1));
        let ov_delete = TransactionOverlay {
            writes: [(pk("k"), delete("k"))].into_iter().collect(),
        };
        store.apply_overlay(&ov_delete, snap(2));

        let removed = store.gc(&[]);
        assert_eq!(removed, 1);
        // The latest version is the tombstone.
        assert_eq!(store.read_committed(b"k"), None);
        // And only one version remains.
        assert_eq!(store.chains.get(&pk("k")).unwrap().versions.len(), 1);
    }

    // ---- TX10: geometric reads under SI --------------------------------
    //
    // A simple "K = mean(payload[0] as f64)" stand-in. The shape of the
    // claim — read-with-snapshot equals function-of-snapshot-records — is
    // what matters, not the geometric primitive chosen.

    fn k_committed(store: &MvccStore) -> f64 {
        let rs = store.materialize_under(store.high_water(), &TransactionOverlay::new());
        if rs.is_empty() {
            return 0.0;
        }
        rs.values().map(|p| p[0] as f64).sum::<f64>() / rs.len() as f64
    }

    fn k_under(store: &MvccStore, tx_snap: SnapshotId, overlay: &TransactionOverlay) -> f64 {
        let rs = store.materialize_under(tx_snap, overlay);
        if rs.is_empty() {
            return 0.0;
        }
        rs.values().map(|p| p[0] as f64).sum::<f64>() / rs.len() as f64
    }

    #[test]
    fn tx10_2_tx_no_writes_equals_snapshot_k() {
        let mut store = MvccStore::new();
        for (i, (k, v)) in [("a", 1u8), ("b", 2)].iter().enumerate() {
            let ov = TransactionOverlay {
                writes: [(pk(k), insert(k, *v))].into_iter().collect(),
            };
            store.apply_overlay(&ov, snap(i as u64 + 1));
        }
        // After snap=2: K_committed = 1.5
        assert!((k_committed(&store) - 1.5).abs() < 1e-12);

        let tx_snap = snap(2);
        let overlay = TransactionOverlay::new();
        assert!((k_under(&store, tx_snap, &overlay) - 1.5).abs() < 1e-12);

        // Now write c=9 at snap=3.
        let ov = TransactionOverlay {
            writes: [(pk("c"), insert("c", 9))].into_iter().collect(),
        };
        store.apply_overlay(&ov, snap(3));
        // Pinned tx still reads K=1.5.
        assert!((k_under(&store, tx_snap, &overlay) - 1.5).abs() < 1e-12);
        // Out-of-tx K reflects the new commit: (1+2+9)/3 = 4.0.
        assert!((k_committed(&store) - 4.0).abs() < 1e-12);
    }

    #[test]
    fn tx10_3_overlay_reflected_in_in_tx_k() {
        let mut store = MvccStore::new();
        for (i, (k, v)) in [("a", 1u8), ("b", 2)].iter().enumerate() {
            let ov = TransactionOverlay {
                writes: [(pk(k), insert(k, *v))].into_iter().collect(),
            };
            store.apply_overlay(&ov, snap(i as u64 + 1));
        }
        let tx_snap = snap(2);
        let mut overlay = TransactionOverlay::new();
        overlay.stage(insert("a", 10));
        overlay.stage(insert("c", 4));

        // (10+2+4)/3 = 16/3
        let got = k_under(&store, tx_snap, &overlay);
        assert!((got - (16.0 / 3.0)).abs() < 1e-12, "got {}", got);

        // Out-of-tx K untouched: (1+2)/2 = 1.5.
        assert!((k_committed(&store) - 1.5).abs() < 1e-12);
    }

    #[test]
    fn tx10_4_old_tx_pinned_past_commit() {
        let mut store = MvccStore::new();
        for (i, (k, v)) in [("a", 1u8), ("b", 2)].iter().enumerate() {
            let ov = TransactionOverlay {
                writes: [(pk(k), insert(k, *v))].into_iter().collect(),
            };
            store.apply_overlay(&ov, snap(i as u64 + 1));
        }
        let tx_prime_snap = snap(2);
        let tx_prime_overlay = TransactionOverlay::new();
        let pre = k_under(&store, tx_prime_snap, &tx_prime_overlay);
        assert!((pre - 1.5).abs() < 1e-12);

        let ov_t = TransactionOverlay {
            writes: [(pk("c"), insert("c", 9))].into_iter().collect(),
        };
        store.apply_overlay(&ov_t, snap(3));

        let still = k_under(&store, tx_prime_snap, &tx_prime_overlay);
        assert_eq!(still, pre);
    }

    #[test]
    fn tx10_5_k_is_a_pure_function() {
        let mut store = MvccStore::new();
        for (i, (k, v)) in [("a", 5u8), ("b", 7), ("c", 11)].iter().enumerate() {
            let ov = TransactionOverlay {
                writes: [(pk(k), insert(k, *v))].into_iter().collect(),
            };
            store.apply_overlay(&ov, snap(i as u64 + 1));
        }
        let tx_snap = snap(3);
        let mut overlay = TransactionOverlay::new();
        overlay.stage(insert("b", 2));
        overlay.stage(delete("d")); // no-op (d isn't there)

        let k1 = k_under(&store, tx_snap, &overlay);
        let k2 = k_under(&store, tx_snap, &overlay);
        let k3 = k_under(&store, tx_snap, &overlay);
        assert_eq!(k1, k2);
        assert_eq!(k2, k3);
        // (5+2+11)/3 = 6.0
        assert!((k1 - 6.0).abs() < 1e-12, "got {}", k1);
    }
}
