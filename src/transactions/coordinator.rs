//! The 2PC coordinator.
//!
//! Holds the per-tx vote map and drives the protocol:
//!
//! 1. Phase A (PREPARE): ask each participant to vote. Collect votes.
//! 2. Decision: if all YES → COMMITTED, else ABORTED.
//! 3. Write decision to global log (the durability point).
//! 4. Phase B: send COMMIT or ABORT to each participant.
//!
//! Failure modes covered (matching TX2 Python gate):
//!
//! - Coordinator crash after PREPARE (no decision recorded) →
//!   presumed-abort recovery.
//! - Coordinator crash after DECISION write, mid-notify → log replay
//!   commits laggards.
//! - Participant crash during commit → coordinator records DECISION
//!   regardless; participant queries on restart.

use crate::transactions::global_log::{GlobalDecision, GlobalTransactionLog};
use crate::transactions::participant::{Participant, ParticipantError, ParticipantTxState, PrepareVote};
use crate::transactions::types::{CommitOutcome, CommitRefusalKind, SnapshotId, TransactionId};

/// Drives the 2PC protocol across a set of participants.
pub struct Coordinator<'a> {
    pub participants: Vec<&'a mut Participant>,
    /// Failure-injection: where to crash, in tests.
    pub crash_after: Option<CoordinatorCrashStep>,
    /// For partial-notify crash: stop after this many participants
    /// have been notified.
    pub crash_after_count: usize,
}

/// Which protocol step the coordinator should fail at, in tests.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoordinatorCrashStep {
    /// Crash after collecting votes, before writing decision to log.
    AfterPrepare,
    /// Crash after writing decision, before notifying any participant.
    AfterDecisionBeforeNotify,
    /// Crash after notifying `crash_after_count` participants.
    PartialNotify,
}

#[derive(Debug, thiserror::Error)]
pub enum CoordinatorError {
    #[error("coordinator crashed at {0:?}")]
    Crashed(CoordinatorCrashStep),

    #[error("participant error: {0}")]
    ParticipantError(#[from] ParticipantError),

    #[error("global log error: {0}")]
    GlobalLogError(#[from] crate::transactions::global_log::GlobalLogError),
}

impl<'a> Coordinator<'a> {
    pub fn new(participants: Vec<&'a mut Participant>) -> Self {
        Self {
            participants,
            crash_after: None,
            crash_after_count: 0,
        }
    }

    /// Run the 2PC protocol. On success, returns the new snapshot id.
    /// On a NO vote, returns `CommitOutcome::Refused`. On failure
    /// injection, returns `CoordinatorError::Crashed`.
    pub fn commit(
        &mut self,
        tx_id: TransactionId,
        new_snap_id: SnapshotId,
        global_log: &mut GlobalTransactionLog,
    ) -> Result<CommitOutcome, CoordinatorError> {
        // -- Phase A: PREPARE --
        let mut votes: Vec<PrepareVote> = Vec::with_capacity(self.participants.len());
        for p in self.participants.iter_mut() {
            let v = match p.prepare(tx_id) {
                Ok(v) => v,
                Err(ParticipantError::Crashed { .. }) => PrepareVote::No,
                Err(other) => return Err(CoordinatorError::ParticipantError(other)),
            };
            votes.push(v);
        }

        if self.crash_after == Some(CoordinatorCrashStep::AfterPrepare) {
            return Err(CoordinatorError::Crashed(CoordinatorCrashStep::AfterPrepare));
        }

        // -- Decision --
        let all_yes = votes.iter().all(|v| *v == PrepareVote::Yes);
        let decision = if all_yes {
            GlobalDecision::Committed
        } else {
            GlobalDecision::Aborted
        };

        let participant_names: Vec<String> =
            self.participants.iter().map(|p| p.bundle.clone()).collect();
        global_log.record_decision(tx_id, decision, participant_names)?;

        if self.crash_after == Some(CoordinatorCrashStep::AfterDecisionBeforeNotify) {
            return Err(CoordinatorError::Crashed(
                CoordinatorCrashStep::AfterDecisionBeforeNotify,
            ));
        }

        // -- Phase B: notify participants --
        let mut notified = 0;
        for p in self.participants.iter_mut() {
            if self.crash_after == Some(CoordinatorCrashStep::PartialNotify)
                && notified >= self.crash_after_count
            {
                return Err(CoordinatorError::Crashed(
                    CoordinatorCrashStep::PartialNotify,
                ));
            }
            let res = match decision {
                GlobalDecision::Committed => p.commit(tx_id),
                GlobalDecision::Aborted => p.abort(tx_id),
            };
            // Participant crashes mid-Phase-B don't abort the
            // transaction: the global log already has the decision.
            // The participant will catch up on recovery.
            let _ = res;
            notified += 1;
        }

        Ok(match decision {
            GlobalDecision::Committed => CommitOutcome::Committed { new_snap_id },
            GlobalDecision::Aborted => CommitOutcome::Refused {
                kind: CommitRefusalKind::PrepareFailed,
                detail: format!("at least one participant voted NO during PREPARE"),
            },
        })
    }

    /// Recovery: replay the global log to bring participants in line
    /// with each recorded decision.
    pub fn recover(&mut self, global_log: &GlobalTransactionLog) {
        for entry in global_log.entries() {
            for p in self.participants.iter_mut() {
                // Skip participants that aren't ours OR have already
                // applied this decision.
                if !entry.participants.contains(&p.bundle) {
                    continue;
                }
                if !matches!(p.tx_state(entry.tx_id), ParticipantTxState::Prepared) {
                    continue;
                }
                let _ = match entry.decision {
                    GlobalDecision::Committed => p.commit(entry.tx_id),
                    GlobalDecision::Aborted => p.abort(entry.tx_id),
                };
            }
        }
    }

    /// Recovery for the coordinator-crashed-after-PREPARE case.
    /// Presumed-abort: any in-doubt tx becomes Aborted.
    pub fn recover_presumed_abort(
        &mut self,
        tx_id: TransactionId,
        global_log: &mut GlobalTransactionLog,
    ) -> Result<(), CoordinatorError> {
        if global_log.decision_for(tx_id).is_some() {
            // Already decided; nothing to do.
            return Ok(());
        }
        let participant_names: Vec<String> =
            self.participants.iter().map(|p| p.bundle.clone()).collect();
        global_log.record_decision(tx_id, GlobalDecision::Aborted, participant_names)?;
        for p in self.participants.iter_mut() {
            if matches!(p.tx_state(tx_id), ParticipantTxState::Prepared) {
                let _ = p.abort(tx_id);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transactions::participant::CrashStep;
    use crate::transactions::types::{PendingWrite, WriteOp};

    fn mk_write(pk: u32, val: u8) -> PendingWrite {
        PendingWrite {
            pk: pk.to_be_bytes().to_vec(),
            payload: vec![val],
            op: WriteOp::Insert,
        }
    }

    /// All participants vote YES → all commit. TX2 case 1.
    #[test]
    fn n_bundles_atomic_commit() {
        for n in [2usize, 3, 5] {
            let mut parts: Vec<Participant> =
                (0..n).map(|i| Participant::new(format!("b{}", i))).collect();
            let tx_id = TransactionId::new();

            for (i, p) in parts.iter_mut().enumerate() {
                p.stage_write(tx_id, mk_write(100 + i as u32, 0)).unwrap();
            }

            let mut log = GlobalTransactionLog::new();
            {
                let refs: Vec<&mut Participant> = parts.iter_mut().collect();
                let mut coord = Coordinator::new(refs);
                let outcome = coord.commit(tx_id, SnapshotId(1), &mut log).unwrap();
                assert!(matches!(outcome, CommitOutcome::Committed { .. }));
            }

            for (i, p) in parts.iter().enumerate() {
                assert_eq!(
                    p.tx_state(tx_id),
                    ParticipantTxState::Committed,
                    "n={}, participant {} not committed",
                    n,
                    i
                );
                assert!(
                    p.committed.contains_key(&(100 + i as u32).to_be_bytes().to_vec()),
                    "n={}, participant {} missing write",
                    n,
                    i
                );
            }
        }
    }

    /// One participant votes NO → all abort. TX2 case 2.
    #[test]
    fn no_vote_aborts_all() {
        let mut parts: Vec<Participant> =
            (0..3).map(|i| Participant::new(format!("b{}", i))).collect();
        let tx_id = TransactionId::new();

        for (i, p) in parts.iter_mut().enumerate() {
            p.stage_write(tx_id, mk_write(200 + i as u32, 0)).unwrap();
        }
        // Inject NO vote on b1
        parts[1].crash_on = Some(CrashStep::DuringPrepare);

        let mut log = GlobalTransactionLog::new();
        {
            let refs: Vec<&mut Participant> = parts.iter_mut().collect();
            let mut coord = Coordinator::new(refs);
            let outcome = coord.commit(tx_id, SnapshotId(1), &mut log).unwrap();
            match outcome {
                CommitOutcome::Refused { kind, .. } => {
                    assert_eq!(kind, CommitRefusalKind::PrepareFailed);
                }
                _ => panic!("expected Refused, got Committed"),
            }
        }

        for p in parts.iter() {
            // Either Aborted or still Open (in the case of the participant
            // that crashed during prepare; it stayed Open because prepare
            // failed before mutating state).
            let s = p.tx_state(tx_id);
            assert!(
                matches!(s, ParticipantTxState::Aborted | ParticipantTxState::Open),
                "participant {} in unexpected state {:?}",
                p.bundle,
                s
            );
            assert!(p.committed.is_empty(), "participant {} kept writes", p.bundle);
        }
    }

    /// Coordinator crash after PREPARE, before decision. Recovery via
    /// presumed-abort. TX2 case 3.
    #[test]
    fn coordinator_crash_after_prepare_presumed_abort() {
        let mut parts: Vec<Participant> =
            (0..3).map(|i| Participant::new(format!("b{}", i))).collect();
        let tx_id = TransactionId::new();

        for (i, p) in parts.iter_mut().enumerate() {
            p.stage_write(tx_id, mk_write(300 + i as u32, 0)).unwrap();
        }

        let mut log = GlobalTransactionLog::new();

        // 2PC run that crashes after prepare.
        {
            let refs: Vec<&mut Participant> = parts.iter_mut().collect();
            let mut coord = Coordinator::new(refs);
            coord.crash_after = Some(CoordinatorCrashStep::AfterPrepare);
            let r = coord.commit(tx_id, SnapshotId(1), &mut log);
            assert!(matches!(r, Err(CoordinatorError::Crashed(_))));
        }
        // Pre-recovery: participants are Prepared, log is empty.
        assert!(log.is_empty());
        for p in parts.iter() {
            assert_eq!(p.tx_state(tx_id), ParticipantTxState::Prepared);
        }

        // Recovery: presumed-abort.
        {
            let refs: Vec<&mut Participant> = parts.iter_mut().collect();
            let mut coord = Coordinator::new(refs);
            coord.recover_presumed_abort(tx_id, &mut log).unwrap();
        }
        for p in parts.iter() {
            assert_eq!(p.tx_state(tx_id), ParticipantTxState::Aborted);
            assert!(p.committed.is_empty());
        }
        assert_eq!(log.decision_for(tx_id), Some(GlobalDecision::Aborted));
    }

    /// Coordinator crash after DECISION write, before notifying anyone.
    /// Recovery via log replay → all commit. TX2 case 4.
    #[test]
    fn coordinator_crash_after_decision_log_replay() {
        let mut parts: Vec<Participant> =
            (0..3).map(|i| Participant::new(format!("b{}", i))).collect();
        let tx_id = TransactionId::new();

        for (i, p) in parts.iter_mut().enumerate() {
            p.stage_write(tx_id, mk_write(400 + i as u32, 0)).unwrap();
        }

        let mut log = GlobalTransactionLog::new();

        {
            let refs: Vec<&mut Participant> = parts.iter_mut().collect();
            let mut coord = Coordinator::new(refs);
            coord.crash_after = Some(CoordinatorCrashStep::AfterDecisionBeforeNotify);
            let r = coord.commit(tx_id, SnapshotId(1), &mut log);
            assert!(matches!(r, Err(CoordinatorError::Crashed(_))));
        }

        // Decision is in the log, no participant applied yet.
        assert_eq!(log.decision_for(tx_id), Some(GlobalDecision::Committed));
        for p in parts.iter() {
            assert_eq!(p.tx_state(tx_id), ParticipantTxState::Prepared);
        }

        // Recovery: replay.
        {
            let refs: Vec<&mut Participant> = parts.iter_mut().collect();
            let mut coord = Coordinator::new(refs);
            coord.recover(&log);
        }
        for p in parts.iter() {
            assert_eq!(p.tx_state(tx_id), ParticipantTxState::Committed);
        }
    }

    /// Coordinator crash after partial notify. Recovery via log replay
    /// for the laggards. TX2 case 5.
    #[test]
    fn coordinator_crash_partial_notify_log_replay() {
        let mut parts: Vec<Participant> =
            (0..3).map(|i| Participant::new(format!("b{}", i))).collect();
        let tx_id = TransactionId::new();

        for (i, p) in parts.iter_mut().enumerate() {
            p.stage_write(tx_id, mk_write(500 + i as u32, 0)).unwrap();
        }

        let mut log = GlobalTransactionLog::new();

        {
            let refs: Vec<&mut Participant> = parts.iter_mut().collect();
            let mut coord = Coordinator::new(refs);
            coord.crash_after = Some(CoordinatorCrashStep::PartialNotify);
            coord.crash_after_count = 1;
            let r = coord.commit(tx_id, SnapshotId(1), &mut log);
            assert!(matches!(r, Err(CoordinatorError::Crashed(_))));
        }
        // Pre-recovery: 1 committed, 2 still prepared.
        let committed_count = parts
            .iter()
            .filter(|p| matches!(p.tx_state(tx_id), ParticipantTxState::Committed))
            .count();
        assert_eq!(committed_count, 1);

        // Recovery
        {
            let refs: Vec<&mut Participant> = parts.iter_mut().collect();
            let mut coord = Coordinator::new(refs);
            coord.recover(&log);
        }
        for p in parts.iter() {
            assert_eq!(p.tx_state(tx_id), ParticipantTxState::Committed);
        }
    }

    /// Participant crash after voting YES, before applying COMMIT.
    /// Recovery makes states consistent.
    #[test]
    fn participant_crash_after_yes_vote() {
        let mut parts: Vec<Participant> =
            (0..3).map(|i| Participant::new(format!("b{}", i))).collect();
        let tx_id = TransactionId::new();

        for (i, p) in parts.iter_mut().enumerate() {
            p.stage_write(tx_id, mk_write(600 + i as u32, 0)).unwrap();
        }
        // Participant 1 crashes during commit step
        parts[1].crash_on = Some(CrashStep::DuringCommit);

        let mut log = GlobalTransactionLog::new();
        {
            let refs: Vec<&mut Participant> = parts.iter_mut().collect();
            let mut coord = Coordinator::new(refs);
            let _ = coord.commit(tx_id, SnapshotId(1), &mut log);
        }
        // Decision is COMMITTED in the log (PREPARE all voted YES).
        assert_eq!(log.decision_for(tx_id), Some(GlobalDecision::Committed));

        // Simulate participant restart: clear its crash flag.
        parts[1].crash_on = None;

        {
            let refs: Vec<&mut Participant> = parts.iter_mut().collect();
            let mut coord = Coordinator::new(refs);
            coord.recover(&log);
        }
        // All participants now Committed.
        for p in parts.iter() {
            assert_eq!(p.tx_state(tx_id), ParticipantTxState::Committed);
        }
    }
}
