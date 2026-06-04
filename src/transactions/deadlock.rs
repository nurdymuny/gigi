//! Phase 3 — Deadlock detection via wait-for-graph cycle search.
//!
//! [`LockManager`] is a minimal exclusive-lock manager: each resource
//! has at most one holder and a FIFO waiter queue. The wait-for graph
//! [`LockManager::wait_for_graph`] has an edge `T_a -> T_b` whenever
//! `T_a` is waiting on a resource held by `T_b`. A directed cycle in
//! that graph is a deadlock.
//!
//! [`LockManager::find_cycle`] is plain three-color DFS. If a cycle
//! exists, [`LockManager::detect_and_abort`] picks the youngest
//! transaction in the cycle (largest `begin_ts`) and aborts it. This
//! is the §5.3 youngest-aborts heuristic; it is not starvation-free
//! but it is the canonical first ship.
//!
//! TDD ground truth: `theory/transactions/validation/
//! tx11_13_deadlock_detection.py` (3/3 green).
//!
//! ## Where this fits in Phase 1's protocol
//!
//! Phase 1's [`crate::transactions::Coordinator`] runs PREPARE / DECIDE /
//! NOTIFY against a fixed participant set, with no inter-transaction
//! locks. Phase 3 adds the lock layer: when two transactions touch
//! overlapping write sets, each takes a write-intent lock on the
//! affected bundle. The deadlock detector runs periodically (Spec §5.2
//! suggests every 100ms in production); when a cycle is found the
//! coordinator returns
//! [`crate::transactions::CommitRefusalKind::DeadlockAbort`] for the
//! victim.

use crate::transactions::types::TransactionId;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

/// Per-transaction info the lock manager needs: a monotone `begin_ts`
/// (so we can pick the youngest in a cycle) and final-state flags.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LockTxInfo {
    pub tx_id: TransactionId,
    pub begin_ts: u64,
    pub aborted: bool,
    pub committed: bool,
}

/// Trivial exclusive-lock manager. Each `Resource` is identified by a
/// bytes-y key (typically a bundle name encoded as bytes); the manager
/// records the current holder and a FIFO queue of waiters.
#[derive(Clone, Debug, Default)]
pub struct LockManager {
    /// resource -> holder tx_id
    holders: HashMap<Vec<u8>, TransactionId>,
    /// resource -> FIFO of waiting tx_ids
    waiters: HashMap<Vec<u8>, VecDeque<TransactionId>>,
    /// tx_id -> resources held
    held_by: HashMap<TransactionId, HashSet<Vec<u8>>>,
    /// tx_id -> info (begin_ts, aborted, committed)
    txs: HashMap<TransactionId, LockTxInfo>,
    /// Monotone begin counter.
    begin_counter: u64,
}

/// The outcome of attempting to acquire a lock.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LockOutcome {
    /// The lock is now held by this transaction.
    Acquired,
    /// The resource is held by another transaction; this transaction
    /// is queued and must wait.
    Waiting,
}

impl LockManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Begin a transaction in the lock layer. Assigns a fresh
    /// `begin_ts` and records the tx in the table.
    pub fn begin(&mut self, tx_id: TransactionId) -> LockTxInfo {
        self.begin_counter = self.begin_counter.saturating_add(1);
        let info = LockTxInfo {
            tx_id,
            begin_ts: self.begin_counter,
            aborted: false,
            committed: false,
        };
        self.txs.insert(tx_id, info);
        self.held_by.entry(tx_id).or_default();
        info
    }

    /// Try to acquire an exclusive lock on `resource`.
    pub fn lock(&mut self, tx_id: TransactionId, resource: &[u8]) -> LockOutcome {
        assert!(self.txs.contains_key(&tx_id), "lock() called for unknown tx");
        match self.holders.get(resource) {
            None => {
                self.holders.insert(resource.to_vec(), tx_id);
                self.held_by
                    .entry(tx_id)
                    .or_default()
                    .insert(resource.to_vec());
                LockOutcome::Acquired
            }
            Some(&h) if h == tx_id => LockOutcome::Acquired,
            Some(_) => {
                self.waiters
                    .entry(resource.to_vec())
                    .or_default()
                    .push_back(tx_id);
                LockOutcome::Waiting
            }
        }
    }

    /// Release all locks held by `tx_id`, promoting the head waiter on
    /// each released resource to be the new holder (skipping aborted
    /// waiters).
    fn release_all(&mut self, tx_id: TransactionId) {
        let resources: Vec<Vec<u8>> = self
            .held_by
            .get(&tx_id)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default();
        for r in resources {
            // Drop the holder if it's this tx (always true here).
            if self.holders.get(&r) == Some(&tx_id) {
                self.holders.remove(&r);
            }
            // Promote next non-aborted waiter to holder.
            while let Some(q) = self.waiters.get_mut(&r) {
                let Some(next_tx) = q.pop_front() else {
                    break;
                };
                if q.is_empty() {
                    self.waiters.remove(&r);
                }
                let next_aborted = self
                    .txs
                    .get(&next_tx)
                    .map(|i| i.aborted)
                    .unwrap_or(true);
                if next_aborted {
                    continue;
                }
                self.holders.insert(r.clone(), next_tx);
                self.held_by.entry(next_tx).or_default().insert(r.clone());
                break;
            }
        }
        if let Some(s) = self.held_by.get_mut(&tx_id) {
            s.clear();
        }
    }

    /// Mark a transaction committed and release its locks.
    pub fn commit(&mut self, tx_id: TransactionId) {
        if let Some(i) = self.txs.get_mut(&tx_id) {
            i.committed = true;
        }
        self.release_all(tx_id);
    }

    /// Mark a transaction aborted, release its locks, and pluck it out
    /// of any wait queues it was in.
    pub fn abort(&mut self, tx_id: TransactionId) {
        if let Some(i) = self.txs.get_mut(&tx_id) {
            i.aborted = true;
        }
        self.release_all(tx_id);
        // Remove from any wait queues.
        let resources: Vec<Vec<u8>> = self.waiters.keys().cloned().collect();
        for r in resources {
            if let Some(q) = self.waiters.get_mut(&r) {
                q.retain(|&t| t != tx_id);
                if q.is_empty() {
                    self.waiters.remove(&r);
                }
            }
        }
    }

    /// Read accessor.
    pub fn info(&self, tx_id: TransactionId) -> Option<LockTxInfo> {
        self.txs.get(&tx_id).copied()
    }

    /// Read accessor: who currently holds `resource`, if anyone.
    pub fn holder(&self, resource: &[u8]) -> Option<TransactionId> {
        self.holders.get(resource).copied()
    }

    // ---- Wait-for graph ---------------------------------------------------

    /// Build the wait-for graph as `tx -> [txs it waits on]`.
    pub fn wait_for_graph(&self) -> HashMap<TransactionId, Vec<TransactionId>> {
        let mut edges: HashMap<TransactionId, Vec<TransactionId>> = HashMap::new();
        for &tx in self.txs.keys() {
            edges.entry(tx).or_default();
        }
        for (resource, queue) in &self.waiters {
            let Some(&holder) = self.holders.get(resource) else {
                continue;
            };
            let holder_aborted = self.txs.get(&holder).map(|i| i.aborted).unwrap_or(true);
            if holder_aborted {
                continue;
            }
            for &waiter in queue {
                let w_aborted = self.txs.get(&waiter).map(|i| i.aborted).unwrap_or(true);
                if w_aborted {
                    continue;
                }
                edges.entry(waiter).or_default().push(holder);
            }
        }
        edges
    }

    /// Find one cycle in the wait-for graph and return the nodes that
    /// form it (without the closing duplicate); `None` if acyclic.
    /// Three-color DFS — same primitive the Python reference uses.
    pub fn find_cycle(&self) -> Option<Vec<TransactionId>> {
        let edges = self.wait_for_graph();
        let mut color: HashMap<TransactionId, u8> = edges.keys().map(|&k| (k, 0)).collect();
        let mut parent: HashMap<TransactionId, Option<TransactionId>> =
            edges.keys().map(|&k| (k, None)).collect();

        // Stable iteration order: nodes sorted by begin_ts so the
        // detector deterministically prefers older roots.
        let mut roots: Vec<TransactionId> = edges.keys().copied().collect();
        roots.sort_by_key(|t| self.txs.get(t).map(|i| i.begin_ts).unwrap_or(0));

        const WHITE: u8 = 0;
        const GRAY: u8 = 1;
        const BLACK: u8 = 2;

        for &start in &roots {
            if color[&start] != WHITE {
                continue;
            }
            // Iterative DFS to avoid recursion limits.
            let mut stack: Vec<(TransactionId, std::vec::IntoIter<TransactionId>)> = Vec::new();
            color.insert(start, GRAY);
            stack.push((start, edges[&start].clone().into_iter()));
            while let Some((u, _)) = stack.last() {
                let u_id = *u;
                let mut advanced = false;
                while let Some(v) = stack.last_mut().unwrap().1.next() {
                    match color.get(&v).copied().unwrap_or(WHITE) {
                        GRAY => {
                            // Back-edge -> cycle from v to u to v.
                            let mut cycle = vec![v];
                            let mut w = Some(u_id);
                            while let Some(node) = w {
                                if node == v {
                                    break;
                                }
                                cycle.push(node);
                                w = parent.get(&node).copied().flatten();
                            }
                            cycle.reverse();
                            return Some(cycle);
                        }
                        WHITE => {
                            color.insert(v, GRAY);
                            parent.insert(v, Some(u_id));
                            stack.push((v, edges[&v].clone().into_iter()));
                            advanced = true;
                            break;
                        }
                        _ => {}
                    }
                }
                if !advanced {
                    color.insert(u_id, BLACK);
                    stack.pop();
                }
            }
        }
        None
    }

    /// One detection pass. If a cycle is found, abort the youngest
    /// transaction in the cycle (largest `begin_ts`) and return its
    /// `tx_id`. Returns `None` when no deadlock exists.
    pub fn detect_and_abort(&mut self) -> Option<TransactionId> {
        let cycle = self.find_cycle()?;
        let victim = cycle
            .iter()
            .copied()
            .max_by_key(|t| self.txs.get(t).map(|i| i.begin_ts).unwrap_or(0))?;
        self.abort(victim);
        Some(victim)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_tx(id: u128) -> TransactionId {
        TransactionId(uuid::Uuid::from_u128(id))
    }

    // ---- TX11: two-tx deadlock -----------------------------------------

    #[test]
    fn tx11_two_tx_deadlock_detected_and_younger_aborted() {
        let mut lm = LockManager::new();
        let t1 = mk_tx(1);
        let t2 = mk_tx(2);
        let i1 = lm.begin(t1);
        let i2 = lm.begin(t2);
        assert!(i2.begin_ts > i1.begin_ts);

        assert_eq!(lm.lock(t1, b"A"), LockOutcome::Acquired);
        assert_eq!(lm.lock(t2, b"B"), LockOutcome::Acquired);
        assert_eq!(lm.lock(t1, b"B"), LockOutcome::Waiting);
        assert_eq!(lm.lock(t2, b"A"), LockOutcome::Waiting);

        let cycle = lm.find_cycle().expect("two-cycle must be present");
        assert_eq!(cycle.len(), 2);

        let victim = lm.detect_and_abort().expect("must abort");
        assert_eq!(victim, t2, "younger tx must be the victim");
        assert!(lm.info(t2).unwrap().aborted);

        // After T2's abort, T1 should be able to acquire B.
        assert_eq!(lm.lock(t1, b"B"), LockOutcome::Acquired);
        lm.commit(t1);
        assert!(lm.info(t1).unwrap().committed);
    }

    // ---- TX12: three-tx cycle ------------------------------------------

    #[test]
    fn tx12_three_tx_cycle_detected_and_youngest_aborted() {
        let mut lm = LockManager::new();
        let txs: Vec<TransactionId> = (1u128..=3).map(mk_tx).collect();
        for &t in &txs {
            lm.begin(t);
        }
        assert_eq!(lm.lock(txs[0], b"A"), LockOutcome::Acquired);
        assert_eq!(lm.lock(txs[1], b"B"), LockOutcome::Acquired);
        assert_eq!(lm.lock(txs[2], b"C"), LockOutcome::Acquired);
        assert_eq!(lm.lock(txs[0], b"B"), LockOutcome::Waiting);
        assert_eq!(lm.lock(txs[1], b"C"), LockOutcome::Waiting);
        assert_eq!(lm.lock(txs[2], b"A"), LockOutcome::Waiting);

        let victim = lm.detect_and_abort().expect("3-cycle must be detected");
        assert_eq!(victim, txs[2], "youngest of the 3-cycle must abort");
    }

    #[test]
    fn tx12_disjoint_cycles_both_broken_in_one_pass() {
        let mut lm = LockManager::new();
        let txs: Vec<TransactionId> = (10u128..=40).step_by(10).map(mk_tx).collect();
        for &t in &txs {
            lm.begin(t);
        }
        assert_eq!(lm.lock(txs[0], b"X"), LockOutcome::Acquired);
        assert_eq!(lm.lock(txs[1], b"Y"), LockOutcome::Acquired);
        assert_eq!(lm.lock(txs[2], b"Z"), LockOutcome::Acquired);
        assert_eq!(lm.lock(txs[3], b"W"), LockOutcome::Acquired);
        // Cycle A: 0 <-> 1 on X,Y
        assert_eq!(lm.lock(txs[0], b"Y"), LockOutcome::Waiting);
        assert_eq!(lm.lock(txs[1], b"X"), LockOutcome::Waiting);
        // Cycle B: 2 <-> 3 on Z,W
        assert_eq!(lm.lock(txs[2], b"W"), LockOutcome::Waiting);
        assert_eq!(lm.lock(txs[3], b"Z"), LockOutcome::Waiting);

        let mut aborted: Vec<TransactionId> = Vec::new();
        while let Some(v) = lm.detect_and_abort() {
            aborted.push(v);
        }
        aborted.sort_by_key(|t| t.0);
        let expected: Vec<TransactionId> = vec![txs[1], txs[3]];
        let mut expected_sorted = expected.clone();
        expected_sorted.sort_by_key(|t| t.0);
        assert_eq!(aborted, expected_sorted);
    }

    // ---- TX13: linear waiting is not spuriously aborted -----------------

    #[test]
    fn tx13_linear_wait_no_cycle_no_abort() {
        let mut lm = LockManager::new();
        let t1 = mk_tx(1);
        let t2 = mk_tx(2);
        lm.begin(t1);
        lm.begin(t2);
        assert_eq!(lm.lock(t1, b"A"), LockOutcome::Acquired);
        assert_eq!(lm.lock(t2, b"A"), LockOutcome::Waiting);

        assert!(lm.find_cycle().is_none());
        assert!(lm.detect_and_abort().is_none());
        assert!(!lm.info(t1).unwrap().aborted);
        assert!(!lm.info(t2).unwrap().aborted);
    }

    #[test]
    fn tx13_after_holder_commit_waiter_acquires() {
        let mut lm = LockManager::new();
        let t1 = mk_tx(1);
        let t2 = mk_tx(2);
        lm.begin(t1);
        lm.begin(t2);
        assert_eq!(lm.lock(t1, b"A"), LockOutcome::Acquired);
        assert_eq!(lm.lock(t2, b"A"), LockOutcome::Waiting);

        lm.commit(t1);
        assert_eq!(lm.holder(b"A"), Some(t2));
        assert!(!lm.info(t2).unwrap().aborted);
    }
}
