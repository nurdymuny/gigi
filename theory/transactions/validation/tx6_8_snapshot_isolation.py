"""
TX6-TX8: Snapshot isolation under per-transaction overlays.

================================================================================
CLAIMS (ATOMIC_SHEAF_COMMIT_SPEC.md §4.5):

  TX6  Transaction reads see its own pending writes.
       (Read-your-writes within a transaction.)

  TX7  Transaction reads do NOT see concurrent transactions' pending writes.
       (Pre-commit writes from other txs are invisible.)

  TX8  Snapshot isolation: T2's reads remain consistent across its lifetime
       even if T1 commits during T2.
       (T2's snapshot is pinned at BEGIN; later commits don't leak in.)

GROUND TRUTH (independent):
    A minimal multi-version key-value store that:
      - records every committed write with a monotone `commit_snap_id`
      - exposes a `read_at(pk, snap_id)` that returns the version v with
        the largest `v.commit_snap_id <= snap_id` (or None)
      - exposes a per-tx `read_under(tx, pk)` that consults the tx's
        pending overlay first, then falls back to `read_at(pk, tx.snap_id)`

    This is the classical MVCC snapshot model (Postgres, Oracle).

PASS CRITERION:
    TX6.1  Insert under T, read under T -> visible (own pending write).
    TX6.2  Update under T, read under T -> sees the staged update.
    TX6.3  Delete under T, read under T -> returns None (tombstone visible
           inside the transaction).

    TX7.1  T1 stages an insert; T2 reads -> does not see T1's pending.
    TX7.2  T1 stages an update; T2 reads -> sees pre-T1 committed value.
    TX7.3  T1 stages a delete; T2 reads -> still sees the old value.

    TX8.1  T2 begins at snap=10. T1 commits at snap=11 while T2 still open.
           T2's later read returns the snap=10 view (the value T2 saw at
           its first read), NOT the snap=11 update.
    TX8.2  Repeat with 3 concurrent txs at different snap ids; each sees
           its own pinned view.

CIRCULAR-LOGIC GUARDS:
    1. The committed store is a list of versions per pk. read_at scans it
       linearly; no caching, no snapshot reuse.
    2. The overlay is a per-tx dict separate from the committed store.
       Mutation only happens through stage_write / commit / abort.
    3. Snap ids are monotone integers assigned by the engine, not by the
       transaction. The transaction observes them, never sets them.
================================================================================
"""

from __future__ import annotations
import sys
from dataclasses import dataclass, field
from typing import Optional


# ============================================================================
# Reference MVCC engine
# ============================================================================

@dataclass
class Version:
    """A single committed value of a key, with the snap_id at which it
    became visible."""
    payload: Optional[bytes]    # None means "tombstone" (delete)
    commit_snap_id: int


@dataclass
class PendingWrite:
    pk: bytes
    payload: Optional[bytes]    # None means "stage a delete"


@dataclass
class Transaction:
    tx_id: int
    snap_id: int                                # pinned at BEGIN
    overlay: dict = field(default_factory=dict) # pk -> PendingWrite
    state: str = "open"                         # open / committed / aborted


class MVCCEngine:
    """Reference snapshot-isolation engine. NOT geometry-aware (that's TX10).
    Tracks one bundle's worth of versions and per-tx overlays."""

    def __init__(self):
        # pk -> [Version, Version, ...]  (append-only, oldest first)
        self.versions: dict = {}
        # Monotone snap_id counter.
        self.snap_counter: int = 0
        # Open transactions, by tx_id.
        self.txs: dict = {}
        # Next tx_id.
        self._tx_counter: int = 0

    # ---- Snapshot bookkeeping ----------------------------------------

    def next_snap_id(self) -> int:
        self.snap_counter += 1
        return self.snap_counter

    # ---- Transaction lifecycle ---------------------------------------

    def begin(self) -> Transaction:
        """BEGIN: pin the current snap_id as the transaction's read view."""
        self._tx_counter += 1
        tx = Transaction(tx_id=self._tx_counter, snap_id=self.snap_counter)
        self.txs[tx.tx_id] = tx
        return tx

    def stage_write(self, tx: Transaction, pk: bytes, payload: Optional[bytes]) -> None:
        """Stage a write in the transaction's overlay."""
        assert tx.state == "open", "cannot stage on non-open tx"
        tx.overlay[pk] = PendingWrite(pk=pk, payload=payload)

    def commit(self, tx: Transaction) -> int:
        """Commit: apply overlay as new versions at a fresh snap_id."""
        assert tx.state == "open"
        new_snap = self.next_snap_id()
        for pk, pw in tx.overlay.items():
            self.versions.setdefault(pk, []).append(
                Version(payload=pw.payload, commit_snap_id=new_snap)
            )
        tx.state = "committed"
        return new_snap

    def abort(self, tx: Transaction) -> None:
        assert tx.state == "open"
        tx.overlay.clear()
        tx.state = "aborted"

    # ---- Reads --------------------------------------------------------

    def read_at(self, pk: bytes, snap_id: int) -> Optional[bytes]:
        """Out-of-tx read at a specific snapshot. Returns the payload of
        the latest version v with v.commit_snap_id <= snap_id, or None
        (which means either "no such key" or "tombstoned at this snap")."""
        versions = self.versions.get(pk, [])
        latest: Optional[Version] = None
        for v in versions:
            if v.commit_snap_id <= snap_id:
                if latest is None or v.commit_snap_id > latest.commit_snap_id:
                    latest = v
        return latest.payload if latest is not None else None

    def read_under(self, tx: Transaction, pk: bytes) -> Optional[bytes]:
        """In-tx read: overlay shadows; otherwise read at tx.snap_id."""
        if pk in tx.overlay:
            return tx.overlay[pk].payload
        return self.read_at(pk, tx.snap_id)


# ============================================================================
# TX6: read-your-own-writes
# ============================================================================

def tx6_own_writes_visible_to_self() -> bool:
    print("[TX6] Read-your-own-writes inside a transaction")

    eng = MVCCEngine()
    # Seed committed state with one key.
    seed = eng.begin()
    eng.stage_write(seed, b"k1", b"v_pre")
    eng.commit(seed)

    # TX6.1: insert under T -> visible to T.
    t = eng.begin()
    eng.stage_write(t, b"k_new", b"v_new")
    if eng.read_under(t, b"k_new") != b"v_new":
        print("  FAIL TX6.1: own insert not visible")
        return False
    print("  ok TX6.1: own insert visible inside tx")

    # TX6.2: update under T -> sees staged.
    eng.stage_write(t, b"k1", b"v_t_updated")
    if eng.read_under(t, b"k1") != b"v_t_updated":
        print("  FAIL TX6.2: own update not visible")
        return False
    print("  ok TX6.2: own update visible inside tx")

    # TX6.3: delete under T -> tombstone visible to T.
    eng.stage_write(t, b"k1", None)
    if eng.read_under(t, b"k1") is not None:
        print("  FAIL TX6.3: own delete not visible")
        return False
    print("  ok TX6.3: own delete tombstones inside tx")

    return True


# ============================================================================
# TX7: tx isolation from concurrent overlays
# ============================================================================

def tx7_concurrent_pending_writes_invisible() -> bool:
    print("[TX7] Concurrent pending writes are invisible to other txs")

    eng = MVCCEngine()
    # Seed.
    seed = eng.begin()
    eng.stage_write(seed, b"a", b"a_pre")
    eng.commit(seed)

    # T1 and T2 begin concurrently.
    t1 = eng.begin()
    t2 = eng.begin()

    # TX7.1: T1 inserts a NEW key; T2 must not see it.
    eng.stage_write(t1, b"new_from_t1", b"hidden")
    if eng.read_under(t2, b"new_from_t1") is not None:
        print("  FAIL TX7.1: T2 leaked T1's pending insert")
        return False
    print("  ok TX7.1: T2 cannot see T1's pending insert")

    # TX7.2: T1 updates "a"; T2 must still see the old committed "a_pre".
    eng.stage_write(t1, b"a", b"t1_update")
    if eng.read_under(t2, b"a") != b"a_pre":
        print(f"  FAIL TX7.2: T2 saw {eng.read_under(t2, b'a')!r}, expected b'a_pre'")
        return False
    print("  ok TX7.2: T2 sees pre-T1 committed value")

    # TX7.3: T1 stages a delete of "a"; T2 must still see "a_pre".
    eng.stage_write(t1, b"a", None)
    if eng.read_under(t2, b"a") != b"a_pre":
        print("  FAIL TX7.3: T2 saw T1's tombstone")
        return False
    print("  ok TX7.3: T2 unaffected by T1's pending tombstone")

    return True


# ============================================================================
# TX8: SI consistency across T1's commit during T2
# ============================================================================

def tx8_snapshot_pinned_across_concurrent_commit() -> bool:
    print("[TX8] T2's read view stays pinned even if T1 commits during T2")

    eng = MVCCEngine()
    # Seed.
    seed = eng.begin()
    eng.stage_write(seed, b"x", b"x_pre")
    eng.commit(seed)
    # snap_counter is now 1; bumping it to 10 by stamping spacer commits.
    while eng.snap_counter < 10:
        t = eng.begin()
        eng.stage_write(t, b"_spacer", b"")
        eng.commit(t)

    # T2 begins at snap=10.
    t2 = eng.begin()
    assert t2.snap_id == 10, t2.snap_id

    # T2's first read of "x".
    r0 = eng.read_under(t2, b"x")
    assert r0 == b"x_pre", r0

    # T1 begins and commits between T2's reads. T1 writes x = "x_post".
    t1 = eng.begin()
    eng.stage_write(t1, b"x", b"x_post")
    new_snap = eng.commit(t1)
    if new_snap <= t2.snap_id:
        print(f"  FAIL TX8 setup: t1 committed at snap={new_snap}, t2.snap={t2.snap_id}")
        return False

    # T2's second read of "x". Must still see x_pre, not x_post.
    r1 = eng.read_under(t2, b"x")
    if r1 != b"x_pre":
        print(f"  FAIL TX8.1: T2 second read saw {r1!r}, expected b'x_pre'")
        return False
    print("  ok TX8.1: T2 second read still sees pre-T1 snapshot")

    # TX8.2: three concurrent txs at distinct snap ids each see their own.
    # Reset state.
    eng2 = MVCCEngine()
    a = eng2.begin()
    eng2.stage_write(a, b"y", b"v0")
    eng2.commit(a)  # snap=1, y=v0

    rA = eng2.begin()                       # snap=1
    b = eng2.begin()
    eng2.stage_write(b, b"y", b"v1")
    eng2.commit(b)                          # snap=2, y=v1
    rB = eng2.begin()                       # snap=2

    c = eng2.begin()
    eng2.stage_write(c, b"y", b"v2")
    eng2.commit(c)                          # snap=3, y=v2
    rC = eng2.begin()                       # snap=3

    # After a 4th commit, the three pinned readers must each see their
    # own snapshot value.
    d = eng2.begin()
    eng2.stage_write(d, b"y", b"v3")
    eng2.commit(d)                          # snap=4

    seen_a = eng2.read_under(rA, b"y")
    seen_b = eng2.read_under(rB, b"y")
    seen_c = eng2.read_under(rC, b"y")
    if (seen_a, seen_b, seen_c) != (b"v0", b"v1", b"v2"):
        print(f"  FAIL TX8.2: got {(seen_a, seen_b, seen_c)}, expected (v0,v1,v2)")
        return False
    print("  ok TX8.2: three concurrent readers each see their pinned snapshot")

    return True


# ============================================================================
# main
# ============================================================================

def main():
    print("=" * 72)
    print("TX6-TX8: Snapshot isolation under per-transaction overlays")
    print("=" * 72)

    cases = [
        ("TX6 own writes visible to self",          tx6_own_writes_visible_to_self),
        ("TX7 concurrent pending writes invisible", tx7_concurrent_pending_writes_invisible),
        ("TX8 snapshot pinned across concurrent",   tx8_snapshot_pinned_across_concurrent_commit),
    ]

    results = []
    for name, fn in cases:
        try:
            ok = fn()
        except AssertionError as e:
            print(f"  assertion: {e}")
            ok = False
        results.append((name, ok))

    print("\n" + "=" * 72)
    print("SUMMARY")
    print("=" * 72)
    all_ok = True
    for name, ok in results:
        flag = "PASS" if ok else "FAIL"
        print(f"  [{flag}] {name}")
        all_ok = all_ok and ok

    if all_ok:
        print(f"\n  ALL {len(results)} SI-SEMANTICS CASES GREEN.")
        return 0
    return 1


if __name__ == "__main__":
    sys.exit(main())
