//! Core types for atomic sheaf commits.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::SystemTime;
use uuid::Uuid;

/// Per-transaction unique identifier. Generated at BEGIN.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct TransactionId(pub Uuid);

impl TransactionId {
    pub fn new() -> Self {
        TransactionId(Uuid::new_v4())
    }
}

impl Default for TransactionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TransactionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "tx_{}", self.0)
    }
}

/// Monotonic snapshot identifier. Each committed transaction gets a
/// fresh, strictly-increasing snap_id. Reads under a transaction see
/// committed state as-of their BEGIN snap_id (snapshot isolation —
/// Phase 2).
#[derive(
    Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize,
)]
pub struct SnapshotId(pub u64);

impl SnapshotId {
    /// The "no snapshot yet" sentinel. Used at engine init.
    pub const GENESIS: SnapshotId = SnapshotId(0);

    pub fn next(self) -> Self {
        SnapshotId(self.0.saturating_add(1))
    }
}

/// Isolation level for a transaction. Default is `SnapshotIsolation`;
/// `ReadCommitted` is a weaker mode with fewer commit-time conflicts
/// (Phase 2).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IsolationLevel {
    /// Reads see a consistent snapshot taken at BEGIN. Writes are
    /// validated at COMMIT against the snapshot. The default.
    SnapshotIsolation,
    /// Each read sees the latest committed state at the time of the
    /// read. Lower conflict rate; weaker consistency.
    ReadCommitted,
}

impl Default for IsolationLevel {
    fn default() -> Self {
        IsolationLevel::SnapshotIsolation
    }
}

/// State of a transaction within its lifecycle.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionState {
    /// Transaction is open; writes can be staged.
    Open,
    /// PREPARE phase has been initiated; awaiting participant votes.
    Preparing,
    /// All participants voted YES; commit decision is recorded.
    /// Final state.
    Committed,
    /// At least one participant voted NO, OR an explicit ROLLBACK
    /// was issued, OR recovery decided ABORT. Final state.
    Aborted,
}

/// A single pending write within a transaction's staging area.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PendingWrite {
    /// Primary key of the record being written.
    pub pk: Vec<u8>,
    /// Serialized fiber payload (engine-internal format).
    pub payload: Vec<u8>,
    /// Operation kind — Insert / Update / Delete.
    pub op: WriteOp,
}

/// The kind of write operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WriteOp {
    Insert,
    Update,
    Delete,
}

/// A transaction's runtime state, held by the engine while open.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transaction {
    pub id: TransactionId,
    /// Snapshot taken at BEGIN. Reads under this tx see committed
    /// state with `commit_snap_id <= snap_id`.
    pub snap_id: SnapshotId,
    pub opened_at: SystemTime,
    pub isolation: IsolationLevel,
    pub state: TransactionState,
    /// Bundles this transaction has touched (acquired write-intent on).
    pub touched_bundles: HashSet<String>,
    /// Pending writes, scoped by bundle name.
    pub pending_writes: HashMap<String, Vec<PendingWrite>>,
}

impl Transaction {
    pub fn new(snap_id: SnapshotId, isolation: IsolationLevel) -> Self {
        Self {
            id: TransactionId::new(),
            snap_id,
            opened_at: SystemTime::now(),
            isolation,
            state: TransactionState::Open,
            touched_bundles: HashSet::new(),
            pending_writes: HashMap::new(),
        }
    }

    pub fn stage_write(&mut self, bundle: &str, write: PendingWrite) -> Result<(), &'static str> {
        if self.state != TransactionState::Open {
            return Err("cannot stage write on non-open transaction");
        }
        self.touched_bundles.insert(bundle.to_string());
        self.pending_writes
            .entry(bundle.to_string())
            .or_default()
            .push(write);
        Ok(())
    }

    pub fn total_pending(&self) -> usize {
        self.pending_writes.values().map(|v| v.len()).sum()
    }
}

/// Result of a `Coordinator::commit` call.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CommitOutcome {
    /// All participants voted YES, decision was recorded, all writes
    /// landed.
    Committed { new_snap_id: SnapshotId },
    /// Commit was refused. The kind names why; the detail string is
    /// for the client.
    Refused {
        kind: CommitRefusalKind,
        detail: String,
    },
}

/// Why a commit was refused. Sibling of `OverCurvatureRefused` in
/// `imagine` — refusal kinds are first-class so consumers can branch
/// rather than parse error strings.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommitRefusalKind {
    /// Phase 4 invariant (1): the cocycle bound was violated at commit.
    CocycleViolation,
    /// Concurrent transaction won the Clean Finger Move resolution.
    /// The client should retry.
    CleanFingerMoveLoss,
    /// At least one participant voted NO during PREPARE.
    PrepareFailed,
    /// Deadlock detector aborted this transaction. Client retries.
    DeadlockAbort,
    /// A participant was unreachable; presumed-abort recovery fired.
    ParticipantUnavailable,
    /// Transaction lifetime expired (Phase 2 default 15 minutes).
    TransactionExpired,
}

impl CommitRefusalKind {
    /// Stable audit string; do not change across versions.
    pub fn as_str(self) -> &'static str {
        match self {
            CommitRefusalKind::CocycleViolation => "cocycle_violation",
            CommitRefusalKind::CleanFingerMoveLoss => "clean_finger_move_loss",
            CommitRefusalKind::PrepareFailed => "prepare_failed",
            CommitRefusalKind::DeadlockAbort => "deadlock_abort",
            CommitRefusalKind::ParticipantUnavailable => "participant_unavailable",
            CommitRefusalKind::TransactionExpired => "transaction_expired",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transaction_id_is_unique() {
        let a = TransactionId::new();
        let b = TransactionId::new();
        assert_ne!(a, b, "transaction ids must be unique per call");
    }

    #[test]
    fn snapshot_id_genesis_is_zero() {
        assert_eq!(SnapshotId::GENESIS.0, 0);
    }

    #[test]
    fn snapshot_id_next_is_monotone() {
        let a = SnapshotId(5);
        let b = a.next();
        assert!(b > a, "snap_id next must be strictly increasing");
    }

    #[test]
    fn snapshot_id_next_does_not_overflow() {
        let max = SnapshotId(u64::MAX);
        let next = max.next();
        // saturating_add: at u64::MAX, next stays at u64::MAX
        assert_eq!(next.0, u64::MAX, "saturating overflow at u64::MAX");
    }

    #[test]
    fn isolation_level_default_is_snapshot_isolation() {
        assert_eq!(IsolationLevel::default(), IsolationLevel::SnapshotIsolation);
    }

    #[test]
    fn new_transaction_is_open_with_no_writes() {
        let tx = Transaction::new(SnapshotId(1), IsolationLevel::SnapshotIsolation);
        assert_eq!(tx.state, TransactionState::Open);
        assert!(tx.touched_bundles.is_empty());
        assert_eq!(tx.total_pending(), 0);
    }

    #[test]
    fn stage_write_updates_pending_and_touched() {
        let mut tx = Transaction::new(SnapshotId(1), IsolationLevel::default());
        let w = PendingWrite {
            pk: vec![1, 2, 3],
            payload: vec![4, 5, 6],
            op: WriteOp::Insert,
        };
        tx.stage_write("users", w).unwrap();
        assert!(tx.touched_bundles.contains("users"));
        assert_eq!(tx.total_pending(), 1);
    }

    #[test]
    fn stage_write_rejected_after_commit() {
        let mut tx = Transaction::new(SnapshotId(1), IsolationLevel::default());
        tx.state = TransactionState::Committed;
        let w = PendingWrite {
            pk: vec![],
            payload: vec![],
            op: WriteOp::Insert,
        };
        assert!(tx.stage_write("x", w).is_err());
    }

    #[test]
    fn refusal_kind_as_str_is_stable() {
        // These strings ship in audit logs and HTTP responses. If they
        // drift, downstream parsers break.
        assert_eq!(
            CommitRefusalKind::CocycleViolation.as_str(),
            "cocycle_violation"
        );
        assert_eq!(
            CommitRefusalKind::CleanFingerMoveLoss.as_str(),
            "clean_finger_move_loss"
        );
        assert_eq!(
            CommitRefusalKind::PrepareFailed.as_str(),
            "prepare_failed"
        );
        assert_eq!(
            CommitRefusalKind::DeadlockAbort.as_str(),
            "deadlock_abort"
        );
        assert_eq!(
            CommitRefusalKind::ParticipantUnavailable.as_str(),
            "participant_unavailable"
        );
        assert_eq!(
            CommitRefusalKind::TransactionExpired.as_str(),
            "transaction_expired"
        );
    }

    #[test]
    fn commit_outcome_serde_round_trips() {
        let committed = CommitOutcome::Committed {
            new_snap_id: SnapshotId(42),
        };
        let refused = CommitOutcome::Refused {
            kind: CommitRefusalKind::CocycleViolation,
            detail: "δ=0.7 exceeds budget B=0.5".to_string(),
        };
        for outcome in [committed, refused] {
            let json = serde_json::to_string(&outcome).unwrap();
            let back: CommitOutcome = serde_json::from_str(&json).unwrap();
            assert_eq!(outcome, back);
        }
    }

    #[test]
    fn transaction_id_display_has_tx_prefix() {
        let id = TransactionId::new();
        let s = format!("{}", id);
        assert!(s.starts_with("tx_"), "tx_id display must prefix 'tx_'");
    }
}
