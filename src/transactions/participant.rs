//! Per-bundle participant in the 2PC protocol.
//!
//! Each touched bundle becomes a participant. The participant owns the
//! per-tx pending writes and the per-tx state machine
//! (Open → Prepared → Committed / Aborted).
//!
//! Phase 1 ships an in-memory participant whose state mirrors what
//! the on-disk WAL would carry. Wiring the WAL fsync happens when this
//! integrates into the engine's storage stack.

use crate::transactions::types::{PendingWrite, TransactionId};
use crate::transactions::wal_records::TxWalRecord;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Per-transaction state a participant tracks. Matches the TxWalRecord
/// state machine on disk.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParticipantTxState {
    /// Pending writes are staged but PREPARE hasn't run.
    Open,
    /// Voted YES on PREPARE; awaiting decision from coordinator.
    /// This is the in-doubt state for recovery.
    Prepared,
    /// Decision was COMMITTED; writes have been applied to committed
    /// state.
    Committed,
    /// Decision was ABORTED; pending writes were discarded.
    Aborted,
}

/// The participant's vote during PREPARE.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrepareVote {
    Yes,
    No,
}

/// A reference participant implementation. The committed state is a
/// plain key-value store keyed by primary key (Vec<u8>).
#[derive(Clone, Debug)]
pub struct Participant {
    /// Bundle name this participant represents.
    pub bundle: String,
    /// Committed, visible state.
    pub committed: HashMap<Vec<u8>, Vec<u8>>,
    /// Per-tx pending writes.
    pending: HashMap<TransactionId, Vec<PendingWrite>>,
    /// Per-tx state machine.
    state: HashMap<TransactionId, ParticipantTxState>,
    /// WAL records appended in order. Phase 1: in-memory; later
    /// integrates with the bundle's on-disk WAL.
    wal: Vec<TxWalRecord>,
    /// Failure-injection hook for tests. Names the protocol step at
    /// which a synthetic crash should occur.
    pub crash_on: Option<CrashStep>,
}

/// Which protocol step the participant should fail at, in tests.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CrashStep {
    DuringPrepare,
    DuringCommit,
    DuringAbort,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ParticipantError {
    #[error("participant {bundle} crashed at {step:?}")]
    Crashed { bundle: String, step: CrashStep },

    #[error("cannot {action} tx {tx_id} in state {actual:?}")]
    WrongState {
        action: &'static str,
        tx_id: TransactionId,
        actual: ParticipantTxState,
    },

    #[error("tx {0} unknown to participant")]
    UnknownTransaction(TransactionId),
}

impl Participant {
    pub fn new(bundle: impl Into<String>) -> Self {
        Self {
            bundle: bundle.into(),
            committed: HashMap::new(),
            pending: HashMap::new(),
            state: HashMap::new(),
            wal: Vec::new(),
            crash_on: None,
        }
    }

    /// Stage a write under a transaction. The write is recorded in the
    /// pending map and a Pending WAL record is appended.
    pub fn stage_write(
        &mut self,
        tx_id: TransactionId,
        write: PendingWrite,
    ) -> Result<(), ParticipantError> {
        let current = self.state.get(&tx_id).copied().unwrap_or(ParticipantTxState::Open);
        if current != ParticipantTxState::Open {
            return Err(ParticipantError::WrongState {
                action: "stage",
                tx_id,
                actual: current,
            });
        }
        self.wal.push(TxWalRecord::pending(tx_id, write.clone()));
        self.pending.entry(tx_id).or_default().push(write);
        self.state.insert(tx_id, ParticipantTxState::Open);
        Ok(())
    }

    /// PREPARE phase: validate the write set and vote.
    pub fn prepare(&mut self, tx_id: TransactionId) -> Result<PrepareVote, ParticipantError> {
        if self.crash_on == Some(CrashStep::DuringPrepare) {
            return Err(ParticipantError::Crashed {
                bundle: self.bundle.clone(),
                step: CrashStep::DuringPrepare,
            });
        }
        let current = self.state.get(&tx_id).copied();
        match current {
            Some(ParticipantTxState::Open) => {
                // Validation hook (Phase 4 wires cocycle / fiber typing
                // checks here). Phase 1 accepts all syntactically valid
                // pending writes.
                self.wal.push(TxWalRecord::prepared(tx_id));
                self.state.insert(tx_id, ParticipantTxState::Prepared);
                Ok(PrepareVote::Yes)
            }
            Some(_other) => Ok(PrepareVote::No),
            None => Ok(PrepareVote::No),
        }
    }

    /// COMMIT phase: apply pending writes to committed state.
    pub fn commit(&mut self, tx_id: TransactionId) -> Result<(), ParticipantError> {
        if self.crash_on == Some(CrashStep::DuringCommit) {
            return Err(ParticipantError::Crashed {
                bundle: self.bundle.clone(),
                step: CrashStep::DuringCommit,
            });
        }
        let current = self.state.get(&tx_id).copied();
        if current != Some(ParticipantTxState::Prepared) {
            return Err(ParticipantError::WrongState {
                action: "commit",
                tx_id,
                actual: current.unwrap_or(ParticipantTxState::Open),
            });
        }
        if let Some(writes) = self.pending.remove(&tx_id) {
            for w in writes {
                match w.op {
                    crate::transactions::types::WriteOp::Insert
                    | crate::transactions::types::WriteOp::Update => {
                        self.committed.insert(w.pk, w.payload);
                    }
                    crate::transactions::types::WriteOp::Delete => {
                        self.committed.remove(&w.pk);
                    }
                }
            }
        }
        self.wal.push(TxWalRecord::committed(tx_id));
        self.state.insert(tx_id, ParticipantTxState::Committed);
        Ok(())
    }

    /// ABORT phase: discard pending writes.
    pub fn abort(&mut self, tx_id: TransactionId) -> Result<(), ParticipantError> {
        if self.crash_on == Some(CrashStep::DuringAbort) {
            return Err(ParticipantError::Crashed {
                bundle: self.bundle.clone(),
                step: CrashStep::DuringAbort,
            });
        }
        self.pending.remove(&tx_id);
        self.wal.push(TxWalRecord::aborted(tx_id));
        self.state.insert(tx_id, ParticipantTxState::Aborted);
        Ok(())
    }

    /// Query this participant's view of the transaction's state.
    pub fn tx_state(&self, tx_id: TransactionId) -> ParticipantTxState {
        self.state.get(&tx_id).copied().unwrap_or(ParticipantTxState::Open)
    }

    /// Snapshot of the visible state. Out-of-tx reader.
    pub fn out_of_tx_read(&self) -> &HashMap<Vec<u8>, Vec<u8>> {
        &self.committed
    }

    /// Walk the WAL backward to find in-doubt transactions (last record
    /// is Prepared with no subsequent Committed/Aborted).
    pub fn in_doubt_transactions(&self) -> Vec<TransactionId> {
        self.state
            .iter()
            .filter_map(|(tx_id, state)| {
                if *state == ParticipantTxState::Prepared {
                    Some(*tx_id)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Test-only access to the WAL.
    #[cfg(test)]
    pub fn wal_len(&self) -> usize {
        self.wal.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transactions::types::WriteOp;

    fn mk_write(pk: u8, val: u8) -> PendingWrite {
        PendingWrite {
            pk: vec![pk],
            payload: vec![val],
            op: WriteOp::Insert,
        }
    }

    #[test]
    fn new_participant_has_empty_state() {
        let p = Participant::new("users");
        assert_eq!(p.bundle, "users");
        assert!(p.committed.is_empty());
        assert!(p.in_doubt_transactions().is_empty());
    }

    #[test]
    fn stage_write_appends_pending_wal() {
        let mut p = Participant::new("users");
        let tx_id = TransactionId::new();
        p.stage_write(tx_id, mk_write(1, 100)).unwrap();
        assert_eq!(p.wal_len(), 1);
        assert_eq!(p.tx_state(tx_id), ParticipantTxState::Open);
    }

    #[test]
    fn prepare_after_writes_votes_yes() {
        let mut p = Participant::new("users");
        let tx_id = TransactionId::new();
        p.stage_write(tx_id, mk_write(1, 100)).unwrap();
        let vote = p.prepare(tx_id).unwrap();
        assert_eq!(vote, PrepareVote::Yes);
        assert_eq!(p.tx_state(tx_id), ParticipantTxState::Prepared);
    }

    #[test]
    fn prepare_for_unknown_tx_votes_no() {
        let mut p = Participant::new("users");
        let vote = p.prepare(TransactionId::new()).unwrap();
        assert_eq!(vote, PrepareVote::No);
    }

    #[test]
    fn commit_applies_pending_writes_to_committed() {
        let mut p = Participant::new("users");
        let tx_id = TransactionId::new();
        p.stage_write(tx_id, mk_write(1, 100)).unwrap();
        p.stage_write(tx_id, mk_write(2, 200)).unwrap();
        p.prepare(tx_id).unwrap();
        p.commit(tx_id).unwrap();
        assert_eq!(p.tx_state(tx_id), ParticipantTxState::Committed);
        assert_eq!(p.committed.get(&vec![1]), Some(&vec![100]));
        assert_eq!(p.committed.get(&vec![2]), Some(&vec![200]));
    }

    #[test]
    fn commit_without_prepare_is_rejected() {
        let mut p = Participant::new("users");
        let tx_id = TransactionId::new();
        p.stage_write(tx_id, mk_write(1, 100)).unwrap();
        let r = p.commit(tx_id);
        assert!(r.is_err());
        // Bundle is intact.
        assert!(p.committed.is_empty());
    }

    #[test]
    fn abort_discards_pending() {
        let mut p = Participant::new("users");
        let tx_id = TransactionId::new();
        p.stage_write(tx_id, mk_write(1, 100)).unwrap();
        p.prepare(tx_id).unwrap();
        p.abort(tx_id).unwrap();
        assert_eq!(p.tx_state(tx_id), ParticipantTxState::Aborted);
        assert!(p.committed.is_empty());
    }

    #[test]
    fn delete_op_removes_from_committed() {
        let mut p = Participant::new("users");
        p.committed.insert(vec![1], vec![100]);

        let tx_id = TransactionId::new();
        p.stage_write(
            tx_id,
            PendingWrite {
                pk: vec![1],
                payload: vec![],
                op: WriteOp::Delete,
            },
        )
        .unwrap();
        p.prepare(tx_id).unwrap();
        p.commit(tx_id).unwrap();

        assert!(p.committed.get(&vec![1]).is_none());
    }

    #[test]
    fn crash_during_prepare_returns_error() {
        let mut p = Participant::new("users");
        p.crash_on = Some(CrashStep::DuringPrepare);
        let tx_id = TransactionId::new();
        p.stage_write(tx_id, mk_write(1, 100)).unwrap();
        let r = p.prepare(tx_id);
        assert!(matches!(r, Err(ParticipantError::Crashed { .. })));
    }

    #[test]
    fn crash_during_commit_returns_error() {
        let mut p = Participant::new("users");
        let tx_id = TransactionId::new();
        p.stage_write(tx_id, mk_write(1, 100)).unwrap();
        p.prepare(tx_id).unwrap();
        p.crash_on = Some(CrashStep::DuringCommit);
        let r = p.commit(tx_id);
        assert!(matches!(r, Err(ParticipantError::Crashed { .. })));
        // Bundle state did NOT change because the crash fired before
        // the apply step.
        assert!(p.committed.is_empty());
    }

    #[test]
    fn in_doubt_transactions_lists_prepared() {
        let mut p = Participant::new("users");
        let tx_a = TransactionId::new();
        let tx_b = TransactionId::new();
        p.stage_write(tx_a, mk_write(1, 10)).unwrap();
        p.stage_write(tx_b, mk_write(2, 20)).unwrap();
        p.prepare(tx_a).unwrap();
        // tx_b not yet prepared.
        let mut indoubt = p.in_doubt_transactions();
        indoubt.sort_by_key(|id| id.0);
        assert_eq!(indoubt.len(), 1);
        assert_eq!(indoubt[0], tx_a);
    }

    #[test]
    fn stage_after_prepare_is_rejected() {
        let mut p = Participant::new("users");
        let tx_id = TransactionId::new();
        p.stage_write(tx_id, mk_write(1, 100)).unwrap();
        p.prepare(tx_id).unwrap();
        // No more writes allowed.
        let r = p.stage_write(tx_id, mk_write(2, 200));
        assert!(r.is_err());
    }
}
