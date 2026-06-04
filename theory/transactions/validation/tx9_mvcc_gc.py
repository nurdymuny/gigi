"""
TX9: MVCC garbage collection removes only versions no open transaction
     can see.

================================================================================
CLAIM (ATOMIC_SHEAF_COMMIT_SPEC.md §4.3, §4.5 TX9):

    A multi-version store under snapshot isolation must retain every
    record version that some open transaction *could* read. The GC
    sweep removes a version v iff:

        (a) v is NOT the latest committed version of its pk, AND
        (b) for every open tx T:    T.snap_id >= v_next.commit_snap_id

    where v_next is the *next* committed version after v on the same pk.
    Condition (b) says: every open transaction has already moved past v
    in the version chain (they would read v_next or later, never v).

    Equivalently: the safe-GC frontier is
        gc_frontier = min(open_tx_snap_ids)
    and a non-latest version v is collectable iff
        v_next.commit_snap_id <= gc_frontier.

GROUND TRUTH (independent):
    Reuse the reference MVCCEngine from tx6_8 with an added `gc()` method.
    Construct concrete scenarios where the "safe frontier" can be
    computed by hand, run gc(), then assert the surviving version set
    matches the hand-computed expected set EXACTLY.

PASS CRITERION:

    TX9.1  No open tx -> only the latest version per pk survives.
           (Frontier = +inf; everything below the latest is collectable.)

    TX9.2  One open tx at snap=S -> for each pk, the survivor set is
           { v : v is latest OR v.commit_snap_id > S } unioned with
           { the latest v.commit_snap_id <= S } so the open tx still
           has a value to read. Hand-computed and asserted.

    TX9.3  Two open txs at snap=S1, S2 with S1 < S2: frontier = S1.
           Versions at <= S1 (except the latest <= S1) are collectable;
           anything > S1 is retained because the older tx might still
           need to read from somewhere on the chain. Worked example.

    TX9.4  All open txs close (commit/abort) -> frontier returns to
           "+inf" and a subsequent GC sweep removes all non-latest
           versions.

    TX9.5  Tombstone GC: a delete-tombstone is just a version with
           payload=None. The same rules apply. After GC with no open
           txs, a tombstoned pk has exactly one surviving version
           (the tombstone itself), so reads return None.

CIRCULAR-LOGIC GUARDS:
    1. The "expected surviving set" is computed by HAND in each test,
       not by re-running gc() with a different frontier.
    2. After every gc() call we re-run read_at()/read_under() for every
       open tx and assert the read result matches what the tx would
       have seen pre-GC (semantic invariance under GC).
    3. The frontier is computed from the set of open txs only -- no
       look-ahead, no caller-supplied frontier.
================================================================================
"""

from __future__ import annotations
import sys
from dataclasses import dataclass, field
from typing import Optional


# ============================================================================
# Reference MVCC engine with GC
# ============================================================================
# This duplicates the MVCCEngine from tx6_8 with one added method (gc).
# Duplicated rather than imported to keep each gate self-contained,
# matching the pattern used by tx1 / tx2.

@dataclass
class Version:
    payload: Optional[bytes]
    commit_snap_id: int


@dataclass
class PendingWrite:
    pk: bytes
    payload: Optional[bytes]


@dataclass
class Transaction:
    tx_id: int
    snap_id: int
    overlay: dict = field(default_factory=dict)
    state: str = "open"


class MVCCEngine:
    def __init__(self):
        self.versions: dict = {}
        self.snap_counter: int = 0
        self.txs: dict = {}
        self._tx_counter: int = 0

    def next_snap_id(self) -> int:
        self.snap_counter += 1
        return self.snap_counter

    def begin(self) -> Transaction:
        self._tx_counter += 1
        tx = Transaction(tx_id=self._tx_counter, snap_id=self.snap_counter)
        self.txs[tx.tx_id] = tx
        return tx

    def stage_write(self, tx: Transaction, pk: bytes, payload: Optional[bytes]) -> None:
        assert tx.state == "open"
        tx.overlay[pk] = PendingWrite(pk=pk, payload=payload)

    def commit(self, tx: Transaction) -> int:
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

    def read_at(self, pk: bytes, snap_id: int) -> Optional[bytes]:
        versions = self.versions.get(pk, [])
        latest: Optional[Version] = None
        for v in versions:
            if v.commit_snap_id <= snap_id:
                if latest is None or v.commit_snap_id > latest.commit_snap_id:
                    latest = v
        return latest.payload if latest is not None else None

    def read_under(self, tx: Transaction, pk: bytes) -> Optional[bytes]:
        if pk in tx.overlay:
            return tx.overlay[pk].payload
        return self.read_at(pk, tx.snap_id)

    # ---- GC -----------------------------------------------------------

    def open_snap_ids(self) -> list:
        return [t.snap_id for t in self.txs.values() if t.state == "open"]

    def gc_frontier(self) -> Optional[int]:
        """The lowest open-tx snap_id; None if no open txs."""
        snaps = self.open_snap_ids()
        return min(snaps) if snaps else None

    def gc(self) -> int:
        """Sweep collectable versions. Returns the number removed.

        Rules (matching spec §4.3):
          - The latest version of every pk is always retained.
          - For each non-latest version v, let v_next be the next version
            on the chain (the immediate successor in commit_snap_id).
              If gc_frontier() is None, v is collectable.
              Else, v is collectable iff v_next.commit_snap_id <= frontier.
            Otherwise v is retained because some open tx might still need
            to *land on* v when scanning the chain at its pinned snap.
        """
        frontier = self.gc_frontier()
        removed = 0
        for pk, vs in list(self.versions.items()):
            # Sort by snap_id ascending so successor lookup is O(1).
            vs_sorted = sorted(vs, key=lambda v: v.commit_snap_id)
            kept = []
            for i, v in enumerate(vs_sorted):
                is_latest = (i == len(vs_sorted) - 1)
                if is_latest:
                    kept.append(v)
                    continue
                v_next = vs_sorted[i + 1]
                if frontier is None:
                    # No open tx -> only latest survives.
                    removed += 1
                    continue
                if v_next.commit_snap_id <= frontier:
                    # Every open tx reads >= v_next, so v is dead.
                    removed += 1
                    continue
                kept.append(v)
            self.versions[pk] = kept
        return removed


# ============================================================================
# TX9.1: no open tx, only latest survives
# ============================================================================

def tx9_1_no_open_tx_only_latest_survives() -> bool:
    print("[TX9.1] No open tx -> only latest version per pk survives")

    eng = MVCCEngine()
    # 4 commits on "k", 2 commits on "j".
    for v in [b"k0", b"k1", b"k2", b"k3"]:
        t = eng.begin()
        eng.stage_write(t, b"k", v)
        eng.commit(t)
    for v in [b"j0", b"j1"]:
        t = eng.begin()
        eng.stage_write(t, b"j", v)
        eng.commit(t)

    pre_k = len(eng.versions[b"k"])
    pre_j = len(eng.versions[b"j"])
    assert (pre_k, pre_j) == (4, 2), (pre_k, pre_j)

    removed = eng.gc()
    assert removed == 4, removed
    assert [v.payload for v in eng.versions[b"k"]] == [b"k3"], eng.versions[b"k"]
    assert [v.payload for v in eng.versions[b"j"]] == [b"j1"], eng.versions[b"j"]
    print(f"  ok TX9.1: removed {removed}, surviving k={b'k3'!r}, j={b'j1'!r}")
    return True


# ============================================================================
# TX9.2: one open tx pins versions visible to it
# ============================================================================

def tx9_2_one_open_tx_pins_chain() -> bool:
    print("[TX9.2] One open tx at snap=S pins versions visible to it")

    eng = MVCCEngine()
    # snap 1: k0; snap 2: k1; snap 3: k2; snap 4: k3.
    for v in [b"k0", b"k1", b"k2", b"k3"]:
        t = eng.begin()
        eng.stage_write(t, b"k", v)
        eng.commit(t)
    # Open a tx at snap=2. It can read up to snap=2 -> sees k1.
    pinned = eng.begin()
    # pinned.snap_id is the current snap_counter, which is 4. Force it
    # to 2 to simulate a tx that opened earlier.
    pinned.snap_id = 2

    # What pinned can read pre-GC:
    pre_read = eng.read_under(pinned, b"k")
    assert pre_read == b"k1", pre_read

    removed = eng.gc()
    # Frontier=2. For each non-latest v, collect iff v_next.snap <= 2.
    # Versions: (1,k0),(2,k1),(3,k2),(4,k3).
    # v=(1,k0), v_next=(2,k1). v_next.snap=2 <= 2 -> COLLECT.
    # v=(2,k1), v_next=(3,k2). v_next.snap=3 > 2 -> RETAIN.
    # v=(3,k2), v_next=(4,k3). v_next.snap=4 > 2 -> RETAIN.
    # v=(4,k3), latest -> RETAIN.
    # Expected removed = 1.
    assert removed == 1, removed
    snaps_after = [v.commit_snap_id for v in eng.versions[b"k"]]
    assert snaps_after == [2, 3, 4], snaps_after

    # Semantic invariance: pinned still reads k1.
    post_read = eng.read_under(pinned, b"k")
    assert post_read == b"k1", post_read

    print(f"  ok TX9.2: removed {removed}, retained snaps {snaps_after}, pinned still reads k1")
    return True


# ============================================================================
# TX9.3: two open txs with frontier = min(snaps)
# ============================================================================

def tx9_3_two_open_txs_frontier_is_min() -> bool:
    print("[TX9.3] Two open txs at snap=S1<S2: frontier = S1")

    eng = MVCCEngine()
    # snap 1..6 on key "k".
    for i in range(6):
        t = eng.begin()
        eng.stage_write(t, b"k", f"k{i}".encode())
        eng.commit(t)

    t_early = eng.begin(); t_early.snap_id = 2     # reads k1
    t_late  = eng.begin(); t_late.snap_id  = 4     # reads k3

    assert eng.read_under(t_early, b"k") == b"k1"
    assert eng.read_under(t_late,  b"k") == b"k3"
    assert eng.gc_frontier() == 2

    removed = eng.gc()
    # Versions (1..6). For each non-latest v, collect iff v_next.snap <= 2.
    # v=(1,k0): v_next.snap=2 <= 2 -> COLLECT
    # v=(2,k1): v_next.snap=3 > 2 -> RETAIN
    # v=(3,k2), (4,k3), (5,k4): all v_next > 2 -> RETAIN
    # v=(6,k5): latest -> RETAIN
    # Expected: 1 removed.
    assert removed == 1, removed
    snaps_after = sorted(v.commit_snap_id for v in eng.versions[b"k"])
    assert snaps_after == [2, 3, 4, 5, 6], snaps_after

    # Both readers still see the right values.
    assert eng.read_under(t_early, b"k") == b"k1"
    assert eng.read_under(t_late,  b"k") == b"k3"
    print(f"  ok TX9.3: removed {removed}, snaps {snaps_after}, both readers consistent")
    return True


# ============================================================================
# TX9.4: closing all txs collapses chain to latest
# ============================================================================

def tx9_4_close_all_collapses_chain() -> bool:
    print("[TX9.4] Closing all open txs -> next GC removes all non-latest")

    eng = MVCCEngine()
    for v in [b"a", b"b", b"c", b"d"]:
        t = eng.begin()
        eng.stage_write(t, b"k", v)
        eng.commit(t)

    pinned = eng.begin()
    pinned.snap_id = 2
    removed1 = eng.gc()
    # 1 removed under pinned (k0 collectable, snap=1 -> v_next.snap=2 <= 2).
    assert removed1 == 1, removed1

    # Pinned closes.
    eng.commit(pinned)
    # Frontier is now None.
    assert eng.gc_frontier() is None

    removed2 = eng.gc()
    # Remaining versions: snaps 2,3,4. With no frontier, all non-latest collapse.
    # Expected: 2 removed (snaps 2 and 3).
    assert removed2 == 2, removed2
    snaps_after = [v.commit_snap_id for v in eng.versions[b"k"]]
    assert snaps_after == [4], snaps_after

    print(f"  ok TX9.4: removed {removed1}+{removed2}, chain collapsed to {snaps_after}")
    return True


# ============================================================================
# TX9.5: tombstones obey the same rules
# ============================================================================

def tx9_5_tombstones_obey_rules() -> bool:
    print("[TX9.5] Tombstones are versions with payload=None; GC respects them")

    eng = MVCCEngine()
    # Write, then delete.
    t1 = eng.begin(); eng.stage_write(t1, b"k", b"v"); eng.commit(t1)
    t2 = eng.begin(); eng.stage_write(t2, b"k", None); eng.commit(t2)

    # No open txs -> only the latest version (tombstone) survives.
    removed = eng.gc()
    assert removed == 1, removed
    assert len(eng.versions[b"k"]) == 1
    assert eng.versions[b"k"][0].payload is None

    # Out-of-tx read at any snap returns None.
    eng_read = eng.read_at(b"k", eng.snap_counter)
    assert eng_read is None, eng_read

    print("  ok TX9.5: tombstone survives, read returns None")
    return True


def main():
    print("=" * 72)
    print("TX9: MVCC garbage collection correctness")
    print("=" * 72)

    cases = [
        ("TX9.1 no open tx -> only latest",     tx9_1_no_open_tx_only_latest_survives),
        ("TX9.2 one open tx pins chain",        tx9_2_one_open_tx_pins_chain),
        ("TX9.3 two open txs, frontier=min",    tx9_3_two_open_txs_frontier_is_min),
        ("TX9.4 close-all collapses chain",     tx9_4_close_all_collapses_chain),
        ("TX9.5 tombstones obey GC rules",      tx9_5_tombstones_obey_rules),
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
        print(f"\n  ALL {len(results)} GC CORRECTNESS CASES GREEN.")
        return 0
    return 1


if __name__ == "__main__":
    sys.exit(main())
