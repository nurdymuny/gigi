# Atomic Sheaf Commits — cross-bundle ACID for GIGI

**Status:** spec; ready to implement when prioritized
**Effort:** ~11 engineer-weeks across 4 phases
**Composes with:** sharding (T1–T10), IMAGINE/halo (T11–T13), Clean Finger Move resolver (already shipped)
**Authored:** 2026-06-03
**Owner:** GGOG engine team

---

## §0 — TL;DR

This spec defines **atomic sheaf commits**: cross-bundle write transactions that satisfy ACID + three GIGI-specific invariants that flat databases do not have.

The flat-DB framing is: *"all writes land together or none do."* The sheaf-aware framing is sharper: *"the cocycle bound holds across the transaction, curvature is consistent, holonomy paths don't cross unmapped seams."*

This is not ACID + decoration. It's a stronger primitive that subsumes ACID because the geometric substrate has more invariants to preserve than a row-bag does. We claim the stronger primitive and we test for it.

Four phases:
1. **Phase 1 (3 weeks):** 2-phase commit (2PC) protocol with coordinator-failure recovery.
2. **Phase 2 (3 weeks):** Per-transaction overlay — snapshot isolation against the committed state.
3. **Phase 3 (1 week):** Deadlock detection.
4. **Phase 4 (2 weeks):** Geometric coherence semantics.

Plus ~2 weeks of corner cases and integration. Total: ~11 weeks.

---

## §1 — Why this matters more than ACID

### §1.1 The flat-DB framing

In Postgres, a transaction touching tables `users` and `orders` either commits both or neither. Atomicity means "all rows land together." Isolation means "concurrent transactions don't see each other's pending writes." That's the contract.

### §1.2 The GIGI framing

In GIGI, a transaction touching bundles `users` and `orders` has more to preserve:

1. **Sheaf glue holds.** If the two bundles share a base-space region (e.g., overlap on `user_id`), the **cocycle bound** (Davis 2026b Def 21) must hold *across the commit*. A mid-transaction state that violates the cocycle bound cannot be visible to any concurrent reader.

2. **Curvature stays consistent.** The streaming K update (`bundle.curvature_stats.mean()`) is a function of the bundle's record set. A transaction inserting 10 records into bundle A and 5 records into bundle B updates K_A and K_B. Mid-transaction reads of K_A from a concurrent reader must see either *pre-transaction K_A* or *post-transaction K_A*, never an interleaving.

3. **Holonomy paths don't cross unmapped seams.** TRANSPORT and LOCAL_HOLONOMY traverse paths in the bundle's connection 1-form. A mid-transaction state can have inconsistent connection data on overlapping charts. Walking such a path produces undefined holonomy. Atomic sheaf commits guarantee the walk-target connection is fully committed before any walker enters it.

These three invariants are **the cocycle bound applied to time**. The same math that makes sharding work (T1–T10) makes atomic sheaf commits work. The Clean Finger Move resolver (T6, T10) is the write-conflict primitive. We're not building new math — we're applying the math we shipped today to a different dimension.

### §1.3 Marketing claim that survives Lysyanskaya review

> *GIGI provides atomic sheaf commits: cross-bundle transactions that preserve ACID plus three substrate invariants (cocycle bound, curvature consistency, holonomy well-definedness). The substrate invariants are not "added on top" of ACID — they are what ACID looks like when the data has shape.*

We don't say "we have ACID and three other things." We say "ACID is what our primitive degenerates to when you ignore the geometry."

---

## §2 — The math

### §2.1 Atomic sheaf commit, formally

A **transaction** T is a finite ordered set of writes `W = {(b_i, k_i, v_i)}` where `b_i` is a bundle, `k_i` is a base-space key, and `v_i` is a fiber value.

Let `S_t` denote the committed state of the engine at time `t` (the union of all per-bundle states, with their atlas structure).

A commit of T is **atomic-sheaf** if and only if:

1. **All-or-nothing (ACID atomicity):** Either every write in W lands or none do.
2. **Cocycle-preserving:** For every pair `(b_i, b_j)` in W whose atlases share a chart overlap, the cocycle slack `δ_{ij}` after commit satisfies `δ_{ij} ≤ B_{ij}` where `B_{ij}` is the committed cocycle budget for that overlap (Davis 2026b Def 21).
3. **K-monotone:** For each touched bundle `b_i`, the committed K update `ΔK_i` is computed against the *pre-transaction* candidate set, not against a partial-transaction interleaving.
4. **Connection-coherent:** No path that originates outside W's reach can hit a connection 1-form discontinuity introduced by W mid-commit.

The conjunction of (1)–(4) is what we mean by "atomic sheaf commit."

### §2.2 Cocycle bound at commit time

The cocycle budget `B_{ij}` is the engineering analog of Davis 2026b Lemma 4.7. Today it's checked at chart construction. Atomic sheaf commits extend it to *temporal* edges: between two committed states `S_t` and `S_{t+1}`, the cocycle slack across the transaction must satisfy `‖δ_{ij}(S_{t+1}) - δ_{ij}(S_t)‖ ≤ B_{ij}`. Otherwise the commit is refused with `CocycleViolation`.

### §2.3 Clean Finger Move as the conflict primitive

Concurrent transactions can produce write conflicts on overlapping charts. The Clean Finger Move resolver (T6/T10) terminates in N/2 steps, is density-invariant, and is ordering-invariant. It is **the right resolver** for atomic sheaf commits — not because we like reusing math, but because it is the resolver Davis 2026c Thm 5.3 gives us, and the math is already shipped.

Concretely: at commit, if T1 and T2 both touched overlapping base-space regions, the engine runs Clean Finger Move on the conflicting set. One transaction's writes win the region; the other's writes are rejected with `CleanFingerMoveLoss`. The losing transaction can be retried by the client.

---

## §3 — Phase 1: 2-phase commit (3 weeks)

### §3.1 Coordinator + participants

- **Coordinator:** one per transaction. Lives in the engine process. Holds the transaction's pending write set, the prepare votes, and the commit decision.
- **Participants:** one per touched bundle. Each bundle's existing WAL becomes a participant by adding a `prepare` log record type.

### §3.2 Protocol

```
BEGIN T
  -> coordinator created
  -> participant per touched bundle joined

WRITE (b_i, k_i, v_i) under T
  -> participant b_i appends a "pending" record (not yet visible)

COMMIT T
  Phase A: prepare
    coordinator -> each participant: PREPARE
    participant:
      - validate write set (uniqueness, fiber typing, cocycle bound)
      - append PREPARED record to WAL (fsync)
      - vote YES or NO
    coordinator collects votes. If any NO, decide ABORT.

  Phase B: commit/abort
    coordinator -> each participant: COMMIT or ABORT (the DECISION record)
    coordinator fsyncs the DECISION record to a global transaction log
      (single-file append-only, fsynced before the participants are told)
    participant:
      - if COMMIT: append COMMITTED record; pending writes become visible.
      - if ABORT: append ABORTED record; pending writes discarded.
    coordinator -> client: result
```

### §3.3 Coordinator failure recovery

If the coordinator crashes between Phase A and Phase B (the critical window):

- On restart, the engine scans the global transaction log for any transaction with PREPARE votes but no DECISION record. These are **in-doubt transactions**.
- For each in-doubt transaction, the engine queries the participants for their PREPARED state. If all voted YES, the engine retroactively decides COMMIT; if any voted NO or are missing, the engine decides ABORT.
- The DECISION is written to the global log and propagated to participants.

This is the standard "presumed abort" recovery from Mohan et al. 1986 (ARIES). Battle-tested.

### §3.4 Participant failure recovery

If a participant crashes after voting PREPARED but before applying COMMITTED:

- On restart, the participant's WAL contains a PREPARED but no COMMITTED/ABORTED for this transaction.
- The participant queries the coordinator (or the global transaction log) for the decision.
- The participant applies COMMITTED or ABORTED retroactively.

If the participant crashes *before* fsyncing PREPARED, the coordinator times out the vote and decides ABORT.

### §3.5 What ships in Phase 1

- New types: `TransactionId`, `Coordinator`, `Participant`, `PrepareVote`, `Decision`.
- New WAL record types: `Pending`, `Prepared`, `Committed`, `Aborted`.
- New global transaction log: `data/_global_tx.log`, append-only, fsync-on-write.
- New HTTP endpoints:
  - `POST /v1/transactions/begin` → `{ tx_id }`
  - `POST /v1/transactions/{tx_id}/write` → write set scoped to this transaction
  - `POST /v1/transactions/{tx_id}/commit` → COMMIT or `CocycleViolation`/`CleanFingerMoveLoss`/`PrepareFailed`
  - `POST /v1/transactions/{tx_id}/rollback` → unconditional ABORT
- New GQL verb: `BEGIN`, `COMMIT`, `ROLLBACK` as transaction control statements.

### §3.6 Phase 1 TDD gates

| Gate | Claim | Ground truth |
|---|---|---|
| **TX1** | Single-bundle transaction commits atomically | Pre/post state observation; assert all-or-nothing |
| **TX2** | Cross-bundle transaction commits atomically across 2 bundles | Sample 100 random write sets; assert atomicity |
| **TX3** | Coordinator crash after PREPARE, before DECISION: recovered correctly | Inject crash; restart; assert decision is consistent across participants |
| **TX4** | Participant crash after PREPARED: recovered to coordinator's decision | Inject crash on one participant; assert recovery |
| **TX5** | Concurrent transactions on non-overlapping bundles commit independently | 100 parallel non-overlapping transactions; assert all succeed |

---

## §4 — Phase 2: Per-transaction overlay + snapshot isolation (3 weeks)

### §4.1 The shape

Each open transaction gets a `TransactionOverlay`: an `OverlayBundle` extension that holds the transaction's pending writes for every touched bundle.

Reads under a transaction see:
- The transaction's overlay writes (if present), shadowing
- The committed snapshot as-of the transaction's BEGIN timestamp.

Reads outside any transaction see the latest committed snapshot.

This is **snapshot isolation** (SI). Postgres and Oracle ship SI; Marcella's runtime expects it.

### §4.2 Snapshot identification

At BEGIN, the engine records a monotonic transaction-snapshot ID (`snap_id`). All committed transactions with `commit_snap_id <= snap_id` are visible. All others are invisible.

Reads consult the per-bundle WAL for records with `commit_snap_id <= snap_id`. This requires:

- The WAL grows a `commit_snap_id` column on every committed record (already there as part of the COMMITTED record from Phase 1).
- A B-tree or similar index on `(record_pk, commit_snap_id)` so historical lookups don't full-scan.

### §4.3 Garbage collection

Snapshot isolation requires keeping old versions of records until no open transaction can see them. This is the classic MVCC garbage collection problem:

- A version `v` of record `r` is garbage-collectable when no open transaction has `snap_id < v.commit_snap_id`.
- The engine runs a periodic GC pass that walks the WAL backward, identifies collectable versions, and rewrites the WAL.

This is the same MVCC GC problem Postgres has — solved many times over, but the implementation IS work.

### §4.4 Geometric reads under SI

Curvature, holonomy, Betti — all derived from record sets. Phase 2 ships **read-time snapshot pinning**: a geometric read under transaction T sees the bundle's state as-of T's snapshot.

For curvature, this means `bundle.curvature_stats()` must accept a `snap_id` argument and return the stats computed against the record set visible at that snapshot. **This is potentially expensive** — naive implementation re-runs the streaming K update across the historical record set. Phase 2 ships the naive implementation; Phase 4 ships the caching layer.

### §4.5 Phase 2 TDD gates

| Gate | Claim | Ground truth |
|---|---|---|
| **TX6** | Transaction reads see its own pending writes | Insert under T, read under T; assert visible |
| **TX7** | Transaction reads do NOT see concurrent transactions' pending writes | T1 inserts, T2 reads; T2 sees pre-T1 state |
| **TX8** | Snapshot isolation: T2 reads consistent state even if T1 commits during T2 | Time T1's commit between T2's reads; assert no anomalies |
| **TX9** | MVCC GC removes only versions no open transaction can see | Construct version with open transaction; assert version retained |
| **TX10** | Geometric reads under transaction match the snapshot | Curvature under T = curvature at T.snap_id |

---

## §5 — Phase 3: Deadlock detection (1 week)

### §5.1 The problem

Two transactions hold locks on overlapping write sets and each waits for the other. Without detection, the engine hangs.

### §5.2 The shape

- Each transaction maintains a list of bundles it has acquired write-intent on.
- The engine maintains a **wait-for graph**: nodes are transactions, edges are "T_a waits for T_b's lock on bundle X."
- A periodic detector (every 100ms) scans the wait-for graph for cycles. Cycles are deadlocks.
- The detector picks the youngest transaction in the cycle and aborts it (`DeadlockAbort`). The client retries.

### §5.3 Why this is cheap

The wait-for graph is small (proportional to open transactions, not records). Cycle detection is O(V + E) DFS. 100ms is fast enough for human-time clients; we can tune down to 10ms if needed.

The hard problem is **avoiding starvation** — the youngest-aborts heuristic can starve a single retrying client. Phase 3 ships the naive heuristic; if production observes starvation, we add randomized retry backoff + fairness.

### §5.4 Phase 3 TDD gates

| Gate | Claim | Ground truth |
|---|---|---|
| **TX11** | Two-transaction deadlock detected within 200ms | Construct deadlock; assert detection time |
| **TX12** | Three-transaction cycle detected | Construct 3-cycle; assert detection |
| **TX13** | Non-deadlocked waiting transactions are NOT aborted | T1 waits for committing T2; assert no false abort |

---

## §6 — Phase 4: Geometric coherence semantics (2 weeks)

### §6.1 The choice

Mid-transaction, what do **out-of-transaction** geometric reads see? Three options:

**Option A: Freeze geometric reads.**
While any open transaction touches bundle B, out-of-transaction reads of `B.curvature_stats()` block until all touching transactions complete.
- ✓ Always-consistent reads.
- ✗ Long transactions stall analytics workloads.
- ✗ Doesn't compose with sharding (cross-shard transactions stall geometric reads everywhere).

**Option B: Stream updates as records land; flag responses.**
Curvature/holonomy/Betti update on every write (including pending). Geometric read responses include a `transactional_consistency: "stable" | "transient"` flag.
- ✓ No stalling.
- ✗ Mid-transaction reads see partial state. Caller must check the flag.
- ✗ The geometric values can oscillate as transactions roll forward/back.

**Option C: MVCC-style geometric snapshots.**
Every transaction's geometric read is pinned to its snapshot ID. The engine caches per-snapshot curvature/holonomy and updates them on commit.
- ✓ Best of both: always-consistent, never-blocking.
- ✓ Geometry IS data — this is the GIGI-native solution.
- ✗ Storage cost: a snapshot per open transaction.
- ✗ Implementation complexity: ~1.5 weeks of the Phase 4 budget.

### §6.2 Recommendation

**Phase 4 ships Option C.**

Rationale:
- Option A and B don't compose with sharding. Once we have cross-shard transactions (post-Phase F of the sharding plan), A and B become untenable.
- Option C reuses the Phase 2 snapshot machinery — geometric snapshots ride on the same `snap_id` axis as record snapshots. So Option C's "complexity" is mostly already paid for by Phase 2.
- The storage cost is bounded by the number of open transactions, not the database size. For typical workloads (< 100 open transactions at any time), this is negligible.

If the implementation work runs over budget, Phase 4 ships Option B with the flag as a fallback and Option C lands in a follow-up sprint. **Both options preserve the §2.1 atomic-sheaf-commit invariants** — the difference is only in what the *reader* sees mid-transaction.

### §6.3 Phase 4 TDD gates

| Gate | Claim | Ground truth |
|---|---|---|
| **TX14** | Out-of-tx geometric read during open tx returns pre-tx state | Open T1, insert records, read curvature out-of-tx; assert pre-T1 value |
| **TX15** | Out-of-tx geometric read after commit returns post-tx state | Commit T1; read curvature; assert post-T1 value |
| **TX16** | Cocycle bound preserved across commit | Construct tx whose commit would violate cocycle; assert refused |
| **TX17** | Holonomy walker entering committed region sees consistent connection | Start walker during tx; commit tx mid-walk; assert walker sees pre-OR-post but not interleaved |
| **TX18** | Geometric snapshot storage scales linearly with open tx count | Open N transactions; assert snapshot storage = O(N) |

---

## §7 — API surface

### §7.1 HTTP

```
POST /v1/transactions/begin
  Response: { tx_id: "tx_01H...", snap_id: 12345, opened_at: "..." }

POST /v1/transactions/{tx_id}/write
  Body: { bundle: "users", records: [...] }
  Response: { staged: 5, total_in_tx: 23 }

POST /v1/transactions/{tx_id}/commit
  Response (success): { committed_at: "...", new_snap_id: 12346 }
  Response (refused): {
    error_kind: "CocycleViolation" | "CleanFingerMoveLoss" | "PrepareFailed" | "DeadlockAbort",
    detail: "..."
  }

POST /v1/transactions/{tx_id}/rollback
  Response: { aborted: true }

GET /v1/transactions/{tx_id}
  Response: tx status, write count, age, isolation level
```

### §7.2 GQL

```sql
BEGIN TRANSACTION;
  INSERT INTO users (id, name, ...) VALUES (...);
  INSERT INTO orders (user_id, ...) VALUES (...);
  -- geometric verbs work too:
  SELECT CURVATURE FROM users;  -- pinned to tx snapshot
COMMIT;
-- or:
ROLLBACK;
```

`BEGIN TRANSACTION WITH ISOLATION READ_COMMITTED` for callers that want weaker isolation (and lower commit-conflict rates). SI is the default.

### §7.3 Rust types

```rust
pub struct TransactionId(pub Uuid);

pub struct Transaction {
    pub id: TransactionId,
    pub snap_id: SnapshotId,
    pub opened_at: SystemTime,
    pub isolation: IsolationLevel,
    pub touched_bundles: HashSet<BundleName>,
    pub pending_writes: HashMap<BundleName, Vec<PendingWrite>>,
}

pub enum CommitOutcome {
    Committed { new_snap_id: SnapshotId },
    Refused { kind: CommitRefusalKind, detail: String },
}

pub enum CommitRefusalKind {
    CocycleViolation,
    CleanFingerMoveLoss,
    PrepareFailed,
    DeadlockAbort,
    ParticipantUnavailable,
}

pub enum IsolationLevel {
    SnapshotIsolation,    // default
    ReadCommitted,        // weaker, fewer conflicts
}
```

### §7.4 Backwards compatibility

Existing single-bundle endpoints (POST `/v1/bundles/{name}/insert`, etc.) keep working unchanged. They're shorthand for `BEGIN; ...; COMMIT;` — single-statement transactions. No client migration required.

---

## §8 — Failure modes

### §8.1 Coordinator failure

Covered in §3.3. Standard 2PC + presumed abort.

### §8.2 Participant failure during PREPARED

Covered in §3.4. Participant queries coordinator on restart.

### §8.3 Network partition (multi-node future)

Phase 1–4 ship for single-node. When the multi-node engine lands (post-sharding Phase F), the coordinator + participants may live on different nodes. In that case:

- **Partition during Phase A:** PREPARE messages don't reach all participants. Coordinator times out and decides ABORT.
- **Partition during Phase B:** DECISION reaches some participants. Other participants are in-doubt on restart and query the global transaction log (which must be highly available — at minimum, replicated to a quorum).

This is the same problem Spanner, CockroachDB, and FoundationDB solve. We adopt their playbook: Raft consensus on the global transaction log. **Out of scope for Phase 1–4** — multi-node partitions land with the multi-node engine.

### §8.4 Global transaction log loss

If `data/_global_tx.log` is corrupted, in-doubt transactions can't be recovered. Mitigations:

- Replicate the global log to a second disk + Tigris on every fsync.
- Periodically checkpoint the global log and prune entries older than the last checkpoint.

The math here is identical to the existing `cow_snapshot` durability story we already ship for per-bundle data.

### §8.5 Long-running transactions

A transaction held open indefinitely pins MVCC versions and blocks GC. Phase 2 ships a **default max-tx-lifetime** (15 minutes) after which the transaction is forcibly aborted with `TransactionExpired`. Configurable via `WalkConfig`-style settings.

---

## §9 — Open questions

1. **Should isolation level be settable per-write or per-transaction?** Postgres ships per-transaction. We follow.

2. **Should the global transaction log be sharded?** For single-node, no. For multi-node, yes, via Raft + per-shard logs. **Recommendation:** spec the single-node version now; defer multi-node sharding of the log to the multi-node engine sprint.

3. **What is the cocycle budget for *temporal* edges?** §2.2 says it's the same as the static cocycle budget. But two committed snapshots an hour apart might legitimately accumulate slack. Recommendation: ship with the static budget; observe; tune if production hits false `CocycleViolation`.

4. **Should we expose `BEGIN TRANSACTION WITH SHEAF_INVARIANTS_OFF`?** Some workloads (bulk data load, schema migration) may want to suspend the §2.1 invariants and just get atomicity. Recommendation: yes, but log every use to the audit log, and refuse in production unless the bundle is flagged `allow_unsafe_loads`.

5. **What happens to IMAGINE_COHERENCE during an open transaction touching the bundle?** Under Option C (§6.2 recommended), the trajectory is computed against the transaction's snapshot. Out-of-transaction callers see the pre-transaction substrate. Both are mathematically consistent. We ship that semantics.

6. **Read-only transactions: special case?** Yes — they don't need 2PC. They just pin a snap_id and read. Phase 2 ships read-only as a fast path (no coordinator, no PREPARE).

7. **Cross-atlas transactions (post-Phase F sharding):** when bundles live on different atlases, the transaction crosses a bridge. The bridge transition's Lipschitz constant gates whether the cocycle bound holds across the bridge at commit. **Math primitive already shipped (T8).** Engineering ports straightforwardly. Don't redo this work — wire it.

---

## §10 — Effort summary + sequence

| Phase | What | Weeks | TDD gates |
|---|---|---|---|
| 1 | 2PC + coordinator/participant recovery | 3 | TX1–TX5 |
| 2 | Per-tx overlay + snapshot isolation + MVCC GC | 3 | TX6–TX10 |
| 3 | Deadlock detection | 1 | TX11–TX13 |
| 4 | Geometric coherence (Option C) | 2 | TX14–TX18 |
| + | Integration + corner cases + load testing | 2 | (no new gates; existing 18 pass at scale) |

**Total: ~11 weeks.**

**Suggested sequencing:**

1. Phase 1 first — atomic multi-bundle commits are the visible win and unblock the §1.3 marketing claim.
2. Phase 2 second — snapshot isolation is what enables Phase 4. Build the snapshot infrastructure once.
3. Phase 3 third — deadlock detection is small and cheap; ships independently.
4. Phase 4 last — geometric coherence is the GIGI-specific win and benefits from Phase 2's snapshot work being battle-tested first.

Each phase is independently shippable. Phase 1 alone gives "atomic multi-bundle commits" — a real win, just below full SI semantics. We can publish Phase 1, observe production usage, then decide whether to push through to Phase 4 or stop.

---

## §11 — Composition with existing work

This spec composes with the work already shipped:

- **Sharding (T1–T10, src/sharded/):** Cross-shard transactions are the post-Phase-F future. When the multi-node engine ships, the global transaction log goes through Raft, and atomic sheaf commits extend to cross-node atomicity. The math (cocycle bound at commit time = Davis 2026b Def 21 applied to time) is already validated.
- **IMAGINE/halo (T11–T13, src/imagine/):** §9 question 5 — IMAGINE_COHERENCE under an open transaction pins the trajectory to the transaction's snapshot. No code change required, just the snapshot pinning mechanism from Phase 2.
- **Clean Finger Move resolver (T6, T10):** the write-conflict resolver. Reused as-is.
- **Cocycle budget check (T2, T8):** the prepare-phase validation. Reused as-is.
- **OverlayBundle, WAL, mmap snapshots:** the existing single-bundle infrastructure. Phase 1 extends with new record types; Phase 2 extends with snapshot pinning.

**We are not building this from scratch.** The math is mostly done, the substrate is mostly built. Phase 1–4 is the **wiring** of math we already proved into a transaction abstraction.

---

## §12 — When to start

This spec is ready to implement when:

1. A concrete consumer asks for it (Marcella, PRISM, ICARUS, or paying customer).
2. Phase 2 dim lift (IMAGINE) is shipped (unblocks Marcella's bge_v2 verification).
3. Sharding Phase C–E is shipped or queued (so atomic sheaf commits don't conflict with multi-node design).

Until then, this spec lives here, ready. The §10 effort estimate is honest — 11 weeks with one engineer focused. Slip estimate: +30% if the Option C snapshot machinery hits unexpected corner cases.

The marketing claim from §1.3 — "atomic sheaf commits, not ACID" — is unlocked the moment Phase 1 ships, and gets stronger with each subsequent phase.

— Spec authored 2026-06-03 (GGOG engine team)
