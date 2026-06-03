//! Global transaction log — the ground truth for 2PC recovery.
//!
//! Lives at `data/_global_tx.log` (append-only, fsync-on-write). On
//! coordinator restart, the engine scans this log to determine which
//! transactions are in-doubt (PREPARED on participants but no decision
//! recorded) and which have decisions to replay to participants that
//! haven't applied them yet.
//!
//! Per the ARIES presumed-abort discipline: any transaction with
//! PREPARE votes but no decision in the log is presumed-aborted on
//! recovery. The log is written BEFORE participants are notified, so
//! a decision in the log is the canonical commit/abort record.

use crate::transactions::types::TransactionId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

/// A single entry in the global transaction log.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GlobalLogEntry {
    pub tx_id: TransactionId,
    pub timestamp: SystemTime,
    pub decision: GlobalDecision,
    /// The set of participants this decision applies to.
    pub participants: Vec<String>,
}

/// The coordinator's binding decision for a transaction. Once an entry
/// with a decision is in the log, that decision is canonical — no
/// recovery path can override it.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GlobalDecision {
    Committed,
    Aborted,
}

/// In-memory view of the global transaction log. Phase 1 ships a
/// memory-backed implementation that mirrors the on-disk format; the
/// fsync layer is added when this wires into the engine's storage
/// stack.
#[derive(Clone, Debug, Default)]
pub struct GlobalTransactionLog {
    entries: Vec<GlobalLogEntry>,
    /// Index: tx_id -> position of its decision in `entries`.
    index: HashMap<TransactionId, usize>,
}

impl GlobalTransactionLog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a decision. The append is the durability point.
    /// Returns an error if a decision for this tx_id is already
    /// recorded (decisions are final once written).
    pub fn record_decision(
        &mut self,
        tx_id: TransactionId,
        decision: GlobalDecision,
        participants: Vec<String>,
    ) -> Result<(), GlobalLogError> {
        if self.index.contains_key(&tx_id) {
            return Err(GlobalLogError::DecisionAlreadyRecorded(tx_id));
        }
        let entry = GlobalLogEntry {
            tx_id,
            timestamp: SystemTime::now(),
            decision,
            participants,
        };
        let idx = self.entries.len();
        self.entries.push(entry);
        self.index.insert(tx_id, idx);
        Ok(())
    }

    /// Look up a transaction's recorded decision, if any.
    pub fn decision_for(&self, tx_id: TransactionId) -> Option<GlobalDecision> {
        self.index
            .get(&tx_id)
            .map(|&i| self.entries[i].decision)
    }

    /// All entries, in append order. Used for replay during recovery.
    pub fn entries(&self) -> &[GlobalLogEntry] {
        &self.entries
    }

    /// Number of recorded decisions.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum GlobalLogError {
    #[error("decision for tx {0} is already recorded")]
    DecisionAlreadyRecorded(TransactionId),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_log_is_empty() {
        let log = GlobalTransactionLog::new();
        assert!(log.is_empty());
        assert_eq!(log.len(), 0);
    }

    #[test]
    fn record_decision_then_lookup() {
        let mut log = GlobalTransactionLog::new();
        let tx_id = TransactionId::new();
        log.record_decision(tx_id, GlobalDecision::Committed, vec!["users".into()])
            .unwrap();
        assert_eq!(log.decision_for(tx_id), Some(GlobalDecision::Committed));
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn duplicate_decision_is_refused() {
        // Decisions are FINAL once recorded. This is load-bearing for
        // recovery — a participant restart cannot retroactively change
        // a committed transaction.
        let mut log = GlobalTransactionLog::new();
        let tx_id = TransactionId::new();
        log.record_decision(tx_id, GlobalDecision::Committed, vec![]).unwrap();
        let r = log.record_decision(tx_id, GlobalDecision::Aborted, vec![]);
        assert_eq!(r, Err(GlobalLogError::DecisionAlreadyRecorded(tx_id)));
    }

    #[test]
    fn missing_tx_returns_none() {
        let log = GlobalTransactionLog::new();
        assert_eq!(log.decision_for(TransactionId::new()), None);
    }

    #[test]
    fn entries_are_append_order() {
        let mut log = GlobalTransactionLog::new();
        let tx_a = TransactionId::new();
        let tx_b = TransactionId::new();
        let tx_c = TransactionId::new();
        log.record_decision(tx_a, GlobalDecision::Committed, vec![]).unwrap();
        log.record_decision(tx_b, GlobalDecision::Aborted, vec![]).unwrap();
        log.record_decision(tx_c, GlobalDecision::Committed, vec![]).unwrap();
        let ids: Vec<_> = log.entries().iter().map(|e| e.tx_id).collect();
        assert_eq!(ids, vec![tx_a, tx_b, tx_c]);
    }

    #[test]
    fn global_decision_serde_round_trips() {
        for d in [GlobalDecision::Committed, GlobalDecision::Aborted] {
            let s = serde_json::to_string(&d).unwrap();
            let back: GlobalDecision = serde_json::from_str(&s).unwrap();
            assert_eq!(d, back);
        }
    }

    #[test]
    fn global_log_entry_serde_round_trips() {
        let entry = GlobalLogEntry {
            tx_id: TransactionId::new(),
            timestamp: SystemTime::now(),
            decision: GlobalDecision::Committed,
            participants: vec!["users".into(), "orders".into()],
        };
        let s = serde_json::to_string(&entry).unwrap();
        let back: GlobalLogEntry = serde_json::from_str(&s).unwrap();
        assert_eq!(entry.tx_id, back.tx_id);
        assert_eq!(entry.decision, back.decision);
        assert_eq!(entry.participants, back.participants);
    }
}
