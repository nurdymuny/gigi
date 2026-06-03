//! WAL record types for transactional writes.
//!
//! Each participant's WAL grows four new record types to support 2PC:
//! `Pending`, `Prepared`, `Committed`, `Aborted`. Together with the
//! pre-existing record types they form a state machine per transaction.
//!
//! ## State machine
//!
//! ```text
//!   <begin>          --> Pending(tx_id, write_1)
//!   <write>          --> Pending(tx_id, write_n)
//!   <PREPARE from coord> --> Prepared(tx_id)   [fsync before voting YES]
//!   <COMMIT from coord>  --> Committed(tx_id)  [writes become visible]
//!     OR
//!   <ABORT  from coord>  --> Aborted(tx_id)    [pending writes discarded]
//! ```
//!
//! On recovery, the participant scans the WAL for any tx_id whose last
//! record is `Prepared`. That transaction is in-doubt; the participant
//! queries the coordinator's global log for the decision.

use crate::transactions::types::{PendingWrite, TransactionId};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// A WAL record specifically for a transaction's lifecycle on this
/// participant.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TxWalRecord {
    pub tx_id: TransactionId,
    pub timestamp: SystemTime,
    pub kind: TxWalRecordKind,
}

impl TxWalRecord {
    pub fn pending(tx_id: TransactionId, write: PendingWrite) -> Self {
        Self {
            tx_id,
            timestamp: SystemTime::now(),
            kind: TxWalRecordKind::Pending { write },
        }
    }

    pub fn prepared(tx_id: TransactionId) -> Self {
        Self {
            tx_id,
            timestamp: SystemTime::now(),
            kind: TxWalRecordKind::Prepared,
        }
    }

    pub fn committed(tx_id: TransactionId) -> Self {
        Self {
            tx_id,
            timestamp: SystemTime::now(),
            kind: TxWalRecordKind::Committed,
        }
    }

    pub fn aborted(tx_id: TransactionId) -> Self {
        Self {
            tx_id,
            timestamp: SystemTime::now(),
            kind: TxWalRecordKind::Aborted,
        }
    }

    /// True when this record is a terminal state (Committed or Aborted).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.kind,
            TxWalRecordKind::Committed | TxWalRecordKind::Aborted
        )
    }
}

/// The shape of a transaction WAL record.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TxWalRecordKind {
    /// A staged write that has not yet been prepared or committed.
    /// Multiple per-write records may exist for the same tx_id.
    Pending { write: PendingWrite },
    /// The participant has voted YES; pending writes are durable but
    /// not yet visible. Exactly one Prepared per tx_id per participant.
    Prepared,
    /// The participant has applied the writes; they are now visible.
    /// Exactly one Committed per tx_id per participant.
    Committed,
    /// The transaction was aborted; pending writes are discarded.
    /// Exactly one Aborted per tx_id per participant.
    Aborted,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transactions::types::WriteOp;

    fn mk_write() -> PendingWrite {
        PendingWrite {
            pk: vec![1, 2, 3],
            payload: vec![4, 5, 6],
            op: WriteOp::Insert,
        }
    }

    #[test]
    fn pending_record_carries_write() {
        let tx_id = TransactionId::new();
        let r = TxWalRecord::pending(tx_id, mk_write());
        assert_eq!(r.tx_id, tx_id);
        assert!(matches!(r.kind, TxWalRecordKind::Pending { .. }));
        assert!(!r.is_terminal());
    }

    #[test]
    fn prepared_is_not_terminal() {
        let r = TxWalRecord::prepared(TransactionId::new());
        assert!(matches!(r.kind, TxWalRecordKind::Prepared));
        assert!(!r.is_terminal(), "Prepared is the in-doubt state, not terminal");
    }

    #[test]
    fn committed_is_terminal() {
        let r = TxWalRecord::committed(TransactionId::new());
        assert!(r.is_terminal());
    }

    #[test]
    fn aborted_is_terminal() {
        let r = TxWalRecord::aborted(TransactionId::new());
        assert!(r.is_terminal());
    }

    #[test]
    fn wal_record_serde_round_trips() {
        let tx_id = TransactionId::new();
        for r in [
            TxWalRecord::pending(tx_id, mk_write()),
            TxWalRecord::prepared(tx_id),
            TxWalRecord::committed(tx_id),
            TxWalRecord::aborted(tx_id),
        ] {
            let json = serde_json::to_string(&r).unwrap();
            let back: TxWalRecord = serde_json::from_str(&json).unwrap();
            assert_eq!(r, back);
        }
    }
}
