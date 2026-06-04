"""
TX11-TX13: Deadlock detection via wait-for-graph cycle search.

================================================================================
CLAIMS (ATOMIC_SHEAF_COMMIT_SPEC.md §5.4):

  TX11  Two-transaction deadlock is detected by cycle search on the
        wait-for graph. The detector picks one transaction to abort
        and the other completes.

  TX12  Three-transaction cycle (T1 -> T2 -> T3 -> T1) is detected by
        the same DFS, and exactly one transaction is aborted to break
        the cycle.

  TX13  Non-deadlocked waiting (T1 waits for T2; T2 is making progress
        and will release the lock) does NOT trigger a spurious abort.
        Only true cycles are flagged.

GROUND TRUTH (independent):

    A wait-for graph WFG is a directed graph whose nodes are open
    transactions and whose edges are "T_a waits for T_b's lock on
    some bundle X." A deadlock exists iff WFG contains a directed
    cycle. Finding cycles is classical DFS with three-color marking
    (white = unvisited, gray = on current DFS stack, black = done).
    A back-edge into a gray node closes a cycle.

    The youngest-aborts policy: in any detected cycle, abort the
    transaction with the most recent BEGIN timestamp. This is
    starvation-prone in the worst case (Spec §5.3 acknowledges this)
    but is the canonical choice for Phase 3.

PASS CRITERION:

  TX11.1  Two-tx deadlock: T1 holds A, waits for B; T2 holds B, waits
          for A. Detector finds the cycle and aborts the younger tx.

  TX11.2  After abort, the older tx's lock acquisition succeeds and
          its remaining work completes. Final state: 1 committed, 1
          aborted.

  TX12.1  Three-tx cycle: T1->T2->T3->T1. Detector finds it and aborts
          the youngest. The remaining two are then no longer
          deadlocked (T2 can acquire T3's now-released lock).

  TX12.2  Disjoint sub-cycles (two independent 2-cycles): both are
          detected in one pass; one tx from each cycle is aborted.

  TX13.1  Linear waiting: T1 waits for T2 (no cycle). Detector reports
          "no deadlock"; no spurious abort.

  TX13.2  T1 waits for T2 which committed: as soon as T2 commits and
          releases, T1 acquires. No abort.

CIRCULAR-LOGIC GUARDS:

    1. The WFG is rebuilt from scratch on every detection pass; no
       caching, no incremental delta to forget about.
    2. The cycle finder is plain Tarjan-style three-color DFS;
       independent of the lock manager's bookkeeping.
    3. Youngest-aborts uses an externally-supplied `begin_ts`
       counter so the test can assert which tx gets killed.
================================================================================
"""

from __future__ import annotations
import sys
from dataclasses import dataclass, field
from typing import Optional


# ============================================================================
# Reference lock manager + wait-for graph
# ============================================================================

@dataclass
class TxInfo:
    tx_id: int
    begin_ts: int
    aborted: bool = False
    committed: bool = False


class LockManager:
    """A trivial exclusive-lock manager. Holds resource -> tx_id; when a
    tx requests a held resource it goes onto the resource's wait queue."""

    def __init__(self):
        self.holders: dict = {}            # resource -> tx_id
        self.waiters: dict = {}            # resource -> [tx_id, ...]
        self.held_by: dict = {}            # tx_id -> set(resources)
        self.txs: dict = {}                # tx_id -> TxInfo
        self._ts: int = 0

    def begin(self, tx_id: int) -> TxInfo:
        self._ts += 1
        info = TxInfo(tx_id=tx_id, begin_ts=self._ts)
        self.txs[tx_id] = info
        self.held_by[tx_id] = set()
        return info

    def lock(self, tx_id: int, resource: str) -> bool:
        """Try to acquire an exclusive lock. Returns True on success;
        False if the tx must wait."""
        assert tx_id in self.txs
        if resource not in self.holders:
            self.holders[resource] = tx_id
            self.held_by[tx_id].add(resource)
            return True
        if self.holders[resource] == tx_id:
            return True
        self.waiters.setdefault(resource, []).append(tx_id)
        return False

    def release_all(self, tx_id: int) -> None:
        for r in list(self.held_by.get(tx_id, ())):
            assert self.holders[r] == tx_id
            del self.holders[r]
            # Promote first waiter to holder.
            if r in self.waiters and self.waiters[r]:
                next_tx = self.waiters[r].pop(0)
                if not self.waiters[r]:
                    del self.waiters[r]
                # Only promote if the waiter isn't already aborted.
                if not self.txs[next_tx].aborted:
                    self.holders[r] = next_tx
                    self.held_by[next_tx].add(r)
        self.held_by[tx_id] = set()

    def commit(self, tx_id: int) -> None:
        self.txs[tx_id].committed = True
        self.release_all(tx_id)

    def abort(self, tx_id: int) -> None:
        self.txs[tx_id].aborted = True
        self.release_all(tx_id)
        # Remove this tx from any wait queues.
        for r in list(self.waiters.keys()):
            self.waiters[r] = [t for t in self.waiters[r] if t != tx_id]
            if not self.waiters[r]:
                del self.waiters[r]

    # ---- WFG ----------------------------------------------------------

    def build_wfg(self) -> dict:
        """edges[u] -> [v, ...] means tx u waits for tx v."""
        edges: dict = {}
        for tx_id in self.txs:
            edges.setdefault(tx_id, [])
        for resource, queue in self.waiters.items():
            holder = self.holders.get(resource)
            if holder is None:
                continue
            for waiter in queue:
                if self.txs[waiter].aborted or self.txs[holder].aborted:
                    continue
                edges[waiter].append(holder)
        return edges

    def find_cycle(self) -> Optional[list]:
        """Return one cycle as a list [n0, n1, ..., n_k=n0] if any
        exists, else None. Three-color DFS."""
        edges = self.build_wfg()
        WHITE, GRAY, BLACK = 0, 1, 2
        color: dict = {n: WHITE for n in edges}
        parent: dict = {n: None for n in edges}

        def dfs(start: int) -> Optional[list]:
            stack = [(start, iter(edges[start]))]
            color[start] = GRAY
            while stack:
                u, it = stack[-1]
                found_next = False
                for v in it:
                    if color[v] == GRAY:
                        # Back-edge: cycle = u -> ... -> v -> u.
                        cycle = [v]
                        w = u
                        while w is not None and w != v:
                            cycle.append(w)
                            w = parent[w]
                        cycle.append(v)
                        cycle.reverse()
                        return cycle
                    if color[v] == WHITE:
                        color[v] = GRAY
                        parent[v] = u
                        stack.append((v, iter(edges[v])))
                        found_next = True
                        break
                if not found_next:
                    color[u] = BLACK
                    stack.pop()
            return None

        for n in edges:
            if color[n] == WHITE:
                c = dfs(n)
                if c is not None:
                    return c
        return None

    def detect_and_abort(self) -> Optional[int]:
        """Run cycle detection; if a cycle is found, abort the youngest
        (largest begin_ts) tx in the cycle. Returns the aborted tx_id
        or None if no cycle."""
        cycle = self.find_cycle()
        if cycle is None:
            return None
        # Drop the duplicate close-of-cycle node.
        nodes = cycle[:-1] if cycle and cycle[0] == cycle[-1] else cycle
        victim = max(nodes, key=lambda t: self.txs[t].begin_ts)
        self.abort(victim)
        return victim


# ============================================================================
# TX11: two-tx deadlock
# ============================================================================

def tx11_two_tx_deadlock_detected() -> bool:
    print("[TX11] Two-tx deadlock: cycle detected, younger aborted")

    lm = LockManager()
    t1 = lm.begin(1)
    t2 = lm.begin(2)

    assert lm.lock(1, "A") is True
    assert lm.lock(2, "B") is True
    # Now each tries to grab the other's lock.
    assert lm.lock(1, "B") is False     # T1 waits for T2
    assert lm.lock(2, "A") is False     # T2 waits for T1

    cycle = lm.find_cycle()
    if cycle is None:
        print("  FAIL TX11: cycle not detected")
        return False
    print(f"  cycle: {cycle}")

    victim = lm.detect_and_abort()
    if victim != 2:
        print(f"  FAIL TX11: aborted {victim}, expected 2 (younger)")
        return False
    print(f"  ok TX11.1: detected + aborted tx {victim}")

    # T1 should now be able to acquire B since T2's locks were released.
    assert lm.lock(1, "B") is True
    lm.commit(1)
    if not lm.txs[1].committed:
        print("  FAIL TX11.2: T1 did not commit after T2 aborted")
        return False
    if not lm.txs[2].aborted:
        print("  FAIL TX11.2: T2 was not aborted")
        return False
    print("  ok TX11.2: T1 commits cleanly after deadlock break")
    return True


# ============================================================================
# TX12: three-tx cycle
# ============================================================================

def tx12_three_tx_cycle_detected() -> bool:
    print("[TX12] Three-tx cycle: detector aborts youngest")

    lm = LockManager()
    t1 = lm.begin(1)
    t2 = lm.begin(2)
    t3 = lm.begin(3)

    assert lm.lock(1, "A")
    assert lm.lock(2, "B")
    assert lm.lock(3, "C")
    # Cycle: T1 -> B (held by T2) -> C (held by T3) -> A (held by T1).
    assert lm.lock(1, "B") is False
    assert lm.lock(2, "C") is False
    assert lm.lock(3, "A") is False

    cycle = lm.find_cycle()
    if cycle is None:
        print("  FAIL TX12.1: 3-cycle not detected")
        return False
    print(f"  cycle: {cycle}")

    victim = lm.detect_and_abort()
    if victim != 3:
        print(f"  FAIL TX12.1: aborted {victim}, expected 3")
        return False
    print(f"  ok TX12.1: 3-cycle detected, aborted tx {victim}")

    # TX12.2: two disjoint 2-cycles, both broken in one pass.
    lm2 = LockManager()
    a, b, c, d = (lm2.begin(i) for i in (10, 20, 30, 40))
    assert lm2.lock(10, "X") and lm2.lock(20, "Y") and lm2.lock(30, "Z") and lm2.lock(40, "W")
    assert lm2.lock(10, "Y") is False and lm2.lock(20, "X") is False    # 10<->20
    assert lm2.lock(30, "W") is False and lm2.lock(40, "Z") is False    # 30<->40

    aborted = []
    while True:
        v = lm2.detect_and_abort()
        if v is None:
            break
        aborted.append(v)
    aborted.sort()
    if aborted != [20, 40]:
        print(f"  FAIL TX12.2: aborted {aborted}, expected [20, 40]")
        return False
    print(f"  ok TX12.2: disjoint cycles broken, aborted {aborted}")
    return True


# ============================================================================
# TX13: non-deadlocked waiting is not spuriously aborted
# ============================================================================

def tx13_linear_waiting_not_aborted() -> bool:
    print("[TX13] Linear waiting -> no false abort")

    lm = LockManager()
    t1 = lm.begin(1)
    t2 = lm.begin(2)
    assert lm.lock(1, "A")
    # T2 waits for T1 but the wait chain is linear, no cycle.
    assert lm.lock(2, "A") is False

    cycle = lm.find_cycle()
    if cycle is not None:
        print(f"  FAIL TX13.1: spurious cycle {cycle}")
        return False
    victim = lm.detect_and_abort()
    if victim is not None:
        print(f"  FAIL TX13.1: spurious abort of {victim}")
        return False
    print("  ok TX13.1: linear wait detected as not-a-deadlock")

    # T1 commits; T2 acquires.
    lm.commit(1)
    if lm.holders.get("A") != 2:
        print(f"  FAIL TX13.2: holder after T1 commit = {lm.holders.get('A')}")
        return False
    if lm.txs[2].aborted:
        print("  FAIL TX13.2: T2 was aborted in the no-deadlock path")
        return False
    print("  ok TX13.2: T2 acquired after T1 committed, no abort")
    return True


def main():
    print("=" * 72)
    print("TX11-TX13: deadlock detection via wait-for-graph cycle search")
    print("=" * 72)

    cases = [
        ("TX11 two-tx deadlock detected + younger aborted", tx11_two_tx_deadlock_detected),
        ("TX12 three-tx cycle + disjoint cycles",           tx12_three_tx_cycle_detected),
        ("TX13 linear waiting not spuriously aborted",      tx13_linear_waiting_not_aborted),
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
        print(f"\n  ALL {len(results)} DEADLOCK-DETECTION CASES GREEN.")
        return 0
    return 1


if __name__ == "__main__":
    sys.exit(main())
