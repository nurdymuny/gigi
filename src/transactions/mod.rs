//! Atomic Sheaf Commits — cross-bundle ACID transactions for GIGI.
//!
//! See [`theory/transactions/ATOMIC_SHEAF_COMMIT_SPEC.md`] for the
//! full 4-phase design.
//!
//! ## Phase 1 status
//!
//! This module ships Phase 1: 2-phase commit (2PC) with
//! coordinator/participant failure recovery. The TDD gates that pin
//! the contract are:
//!
//! - **TX1** — single-bundle transaction commits atomically
//!   ([`theory/transactions/validation/tx1_single_bundle_atomicity.py`]).
//!   GREEN: 11/11 cases.
//! - **TX2** — cross-bundle transaction commits atomically across N
//!   bundles, with coordinator/participant failure recovery
//!   ([`theory/transactions/validation/tx2_cross_bundle_atomicity.py`]).
//!   GREEN: 8/8 cases including all five failure-injection scenarios.
//!
//! TX3–TX5 land as this module fills in. The Python gates are the
//! reference contract; the Rust types here must satisfy them.
//!
//! ## Why "atomic sheaf commits" not "ACID"
//!
//! The flat-DB framing is *"all writes land together."* GIGI's framing
//! adds three substrate invariants per [`crate::sharded`]:
//!
//! 1. **Cocycle-preserving** — the cocycle bound (Davis 2026b Def 21)
//!    holds across the commit. Mid-transaction states cannot violate
//!    it.
//! 2. **K-monotone** — `bundle.curvature_stats().mean()` updates do
//!    not interleave with concurrent transactions.
//! 3. **Connection-coherent** — no walker entering a touched region
//!    sees a mid-commit connection 1-form discontinuity.
//!
//! These are the cocycle bound applied to *time* instead of *space*.
//! Same math (Davis 2026b/c, validated by T2/T8/T6/T10), different
//! dimension. Phase 1 ships the atomicity invariant (1) at the
//! engineering layer; Phase 4 ships (2) and (3) via MVCC-style
//! geometric snapshots.
//!
//! ## The marketing claim that survives review
//!
//! > GIGI provides atomic sheaf commits: cross-bundle transactions
//! > that preserve ACID plus three substrate invariants. The substrate
//! > invariants are not "added on top" of ACID — they are what ACID
//! > looks like when the data has shape.

#![cfg(feature = "transactions")]

pub mod coordinator;
pub mod global_log;
pub mod participant;
pub mod types;
pub mod wal_records;

pub use coordinator::{Coordinator, CoordinatorError};
pub use global_log::{GlobalLogEntry, GlobalTransactionLog};
pub use participant::{Participant, ParticipantError, PrepareVote};
pub use types::{
    CommitOutcome, CommitRefusalKind, IsolationLevel, PendingWrite, SnapshotId, Transaction,
    TransactionId, TransactionState,
};
pub use wal_records::{TxWalRecord, TxWalRecordKind};
