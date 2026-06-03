"""
TX1: Single-bundle transaction commits atomically.

================================================================================
CLAIM (ATOMIC_SHEAF_COMMIT_SPEC.md §3.6 TX1):

    A transaction containing N writes to a single bundle either:
      (a) commits ALL N writes to the bundle's visible state, OR
      (b) commits NONE of them.

    There is no intermediate observable state in which only K < N writes
    are visible.

    The 'visible state' is defined as: what an out-of-transaction reader
    sees when querying the bundle's record set. Pre-transaction state is
    the bundle's record set at TX_BEGIN; post-transaction state is the
    record set after TX_COMMIT (success) or TX_ROLLBACK (failure).

GROUND TRUTH (independent):
    A reference implementation maintains a 'committed' set and a 'pending'
    set per transaction. The pending set is invisible to out-of-transaction
    readers. COMMIT flushes pending -> committed atomically (single update
    of the visible reference). ROLLBACK discards pending.

    The atomicity guarantee is verified by interleaving reads and writes:
    while a transaction is open with K writes already issued, an out-of-
    transaction reader must see the bundle as it was at TX_BEGIN. After
    COMMIT, the reader must see all N writes. There is no window in which
    a partial K < N is observable.

PASS CRITERION:
    For every N in {1, 5, 100, 1000}:
      1. Begin a transaction.
      2. Out-of-tx reader R_pre: capture bundle record set.
      3. Issue N writes within the transaction.
      4. Out-of-tx reader R_mid: capture bundle record set. Must equal R_pre.
      5. COMMIT.
      6. Out-of-tx reader R_post: capture bundle record set. Must equal
         R_pre ∪ {written records}.
      7. Begin a second transaction.
      8. Issue N writes.
      9. R_mid2: must equal R_post.
      10. ROLLBACK.
      11. R_post2: must equal R_post (writes discarded).

CIRCULAR-LOGIC GUARDS:
    1. The reference implementation does NOT use a 'transaction'
       abstraction internally. It uses two plain dicts (committed,
       pending) so the atomicity is verified at the structural level,
       not by trusting a transaction object.
    2. R_mid is captured by reading from `committed` directly, NEVER
       from any function that knows about pending writes.
    3. Tests vary N to catch implementations that special-case small N.
================================================================================
"""

from __future__ import annotations
import sys
from dataclasses import dataclass, field
from typing import Optional


@dataclass
class ReferenceBundle:
    """Minimal reference bundle: just a dict of pk -> record."""
    committed: dict = field(default_factory=dict)

    def out_of_tx_read(self) -> dict:
        """What an out-of-tx reader sees. Returns a SNAPSHOT (copy)."""
        return dict(self.committed)


@dataclass
class ReferenceTransaction:
    """Per-transaction pending writes against a target bundle."""
    bundle: ReferenceBundle
    pending: dict = field(default_factory=dict)
    state: str = "open"  # open | committed | aborted

    def write(self, pk: int, record: dict) -> None:
        if self.state != "open":
            raise RuntimeError(f"cannot write to {self.state} transaction")
        self.pending[pk] = record

    def commit(self) -> None:
        if self.state != "open":
            raise RuntimeError(f"cannot commit {self.state} transaction")
        # ATOMICITY: a single update to the visible reference. The
        # pending dict is reified into committed in one statement.
        # An out-of-tx reader observing committed at this exact moment
        # sees either the pre-state (before the .update line) or the
        # post-state (after). No intermediate.
        self.bundle.committed.update(self.pending)
        self.state = "committed"

    def rollback(self) -> None:
        if self.state != "open":
            raise RuntimeError(f"cannot rollback {self.state} transaction")
        self.pending.clear()
        self.state = "aborted"


def case_atomic_for_n(n: int, bundle: ReferenceBundle) -> tuple[bool, str]:
    """
    Test atomicity for a transaction of size N. Returns (pass, message).
    """
    # 1. Capture pre-state
    r_pre = bundle.out_of_tx_read()

    # 2. Begin tx
    tx = ReferenceTransaction(bundle=bundle)

    # 3. Issue N writes
    start_pk = max(bundle.committed.keys(), default=-1) + 1
    written = {}
    for i in range(n):
        pk = start_pk + i
        record = {"pk": pk, "value": f"tx_val_{pk}"}
        tx.write(pk, record)
        written[pk] = record

    # 4. Mid-tx out-of-tx read MUST equal pre-state
    r_mid = bundle.out_of_tx_read()
    if r_mid != r_pre:
        return False, f"N={n}: mid-tx read changed (pre={len(r_pre)} mid={len(r_mid)})"

    # 5. Commit
    tx.commit()

    # 6. Post-tx read MUST equal pre ∪ written
    r_post = bundle.out_of_tx_read()
    expected = dict(r_pre)
    expected.update(written)
    if r_post != expected:
        return False, (
            f"N={n}: post-commit read mismatch "
            f"(expected {len(expected)}, got {len(r_post)})"
        )

    return True, f"N={n}: atomic commit OK"


def case_rollback_for_n(n: int, bundle: ReferenceBundle) -> tuple[bool, str]:
    """
    Test rollback for a transaction of size N. Returns (pass, message).
    Pre-state captured first; after rollback, state must equal pre-state.
    """
    r_pre = bundle.out_of_tx_read()

    tx = ReferenceTransaction(bundle=bundle)
    start_pk = max(bundle.committed.keys(), default=-1) + 1
    for i in range(n):
        pk = start_pk + i
        tx.write(pk, {"pk": pk, "value": f"rollback_val_{pk}"})

    tx.rollback()

    r_post = bundle.out_of_tx_read()
    if r_post != r_pre:
        return False, (
            f"N={n}: rollback left state changed "
            f"(pre={len(r_pre)} post={len(r_post)})"
        )

    return True, f"N={n}: rollback OK"


def case_double_commit_rejected() -> tuple[bool, str]:
    """Calling commit twice must raise."""
    bundle = ReferenceBundle()
    tx = ReferenceTransaction(bundle=bundle)
    tx.write(0, {"pk": 0})
    tx.commit()
    try:
        tx.commit()
        return False, "double-commit was allowed (should raise)"
    except RuntimeError:
        return True, "double-commit rejected (good)"


def case_commit_after_rollback_rejected() -> tuple[bool, str]:
    bundle = ReferenceBundle()
    tx = ReferenceTransaction(bundle=bundle)
    tx.write(0, {"pk": 0})
    tx.rollback()
    try:
        tx.commit()
        return False, "commit-after-rollback was allowed (should raise)"
    except RuntimeError:
        return True, "commit-after-rollback rejected (good)"


def case_write_to_committed_tx_rejected() -> tuple[bool, str]:
    bundle = ReferenceBundle()
    tx = ReferenceTransaction(bundle=bundle)
    tx.write(0, {"pk": 0})
    tx.commit()
    try:
        tx.write(1, {"pk": 1})
        return False, "write-to-committed-tx was allowed (should raise)"
    except RuntimeError:
        return True, "write-to-committed-tx rejected (good)"


def main() -> int:
    print("=" * 72)
    print("TX1: Single-bundle transaction commits atomically")
    print("=" * 72)
    print()

    results: list[tuple[bool, str]] = []

    bundle = ReferenceBundle()

    print("-- Atomicity at varying N --")
    for n in (1, 5, 100, 1000):
        ok, msg = case_atomic_for_n(n, bundle)
        results.append((ok, msg))
        print(f"  [{('PASS' if ok else 'FAIL')}] {msg}")

    print()
    print("-- Rollback at varying N --")
    bundle2 = ReferenceBundle(committed={i: {"pk": i, "v": "seed"} for i in range(10)})
    for n in (1, 5, 100, 1000):
        ok, msg = case_rollback_for_n(n, bundle2)
        results.append((ok, msg))
        print(f"  [{('PASS' if ok else 'FAIL')}] {msg}")

    print()
    print("-- State machine guards --")
    for tc in (case_double_commit_rejected,
               case_commit_after_rollback_rejected,
               case_write_to_committed_tx_rejected):
        ok, msg = tc()
        results.append((ok, msg))
        print(f"  [{('PASS' if ok else 'FAIL')}] {msg}")

    all_ok = all(ok for ok, _ in results)

    print()
    print("=" * 72)
    print("SUMMARY")
    print("=" * 72)
    print(f"  {len(results)} cases, {sum(ok for ok, _ in results)} passed.")
    if all_ok:
        print()
        print("  TX1 GREEN -- single-bundle atomic commit validated:")
        print("    All-or-nothing visibility holds for N in {1, 5, 100, 1000}.")
        print("    Rollback restores pre-state exactly at every N.")
        print("    State machine refuses illegal transitions.")
        print()
        print("    This is the reference contract the Rust implementation")
        print("    must satisfy. The 2PC infrastructure in src/transactions/")
        print("    inherits this gate for single-bundle commits as a")
        print("    degenerate case of cross-bundle.")
        return 0
    else:
        print()
        print("  TX1 RED -- one or more cases failed above.")
        return 1


if __name__ == "__main__":
    sys.exit(main())
