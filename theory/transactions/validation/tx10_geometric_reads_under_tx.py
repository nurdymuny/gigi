"""
TX10: Geometric reads under a transaction match the transaction's
      pinned snapshot.

================================================================================
CLAIM (ATOMIC_SHEAF_COMMIT_SPEC.md §4.4, §4.5 TX10):

    A geometric read primitive (CURVATURE, BETTI, HOLONOMY) is a
    deterministic function of the bundle's record set. Under snapshot
    isolation:

        Geometric(T) == Geometric( record_set_at(T.snap_id) )

    For TX10 we use CURVATURE as the canonical primitive, with
    K(records) := mean( f(r) ) for a stable, record-local feature f.
    The claim is:

      1. Out-of-tx reads of K reflect the latest committed state.
      2. Reads of K under transaction T reflect the snapshot at T.snap_id
         (visible committed records) plus T's own overlay shadowing.
      3. K under a tx that has staged no writes equals K computed
         directly against the snapshot's record set (no overlay = no
         shift).

    The same shape holds for any deterministic record-set functional:
    BETTI, HOLONOMY around a loop, etc. Phase 2 ships the *naive*
    (recompute from snapshot) implementation; Phase 4 ships caching.

GROUND TRUTH (independent):
    Compute K = mean(feature) by hand from the record set that's visible
    at the chosen snap_id, with the per-tx overlay applied. The reference
    engine builds the visible-record-set by scanning all versions and
    selecting the latest version per pk with commit_snap_id <= snap_id.

PASS CRITERION:

    TX10.1  K(no_open_tx)      = K(committed set after last commit)
    TX10.2  K(in tx T, T staged no writes) = K at T.snap_id (pre-T)
    TX10.3  K(in tx T after T staged writes) = K of (snapshot UNION
            overlay-overrides), NOT the latest committed K
    TX10.4  K after T commits = K on the post-T record set, and a
            tx T' that began BEFORE T's commit still reads pre-T K
            (snapshot pin holds for geometric reads too)
    TX10.5  K is a pure function of the (snapshot, overlay) pair --
            recomputing it twice gives the same value (no hidden
            mutable state)

CIRCULAR-LOGIC GUARDS:
    1. The reference K is computed by directly scanning the
       record set and averaging feature(r). No caching, no
       partial-state corner cases.
    2. The "snapshot record set" is rebuilt fresh on every call
       from the version chain.
    3. We compare K (a float) using exact equality when the inputs
       are integers, and within 1e-12 otherwise.
================================================================================
"""

from __future__ import annotations
import sys
from dataclasses import dataclass, field
from typing import Optional, Callable


# ============================================================================
# Reference MVCC engine with geometric read
# ============================================================================

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


def _feature_int(payload: bytes) -> float:
    """Stable record-local feature: interpret payload as a big-endian
    unsigned integer in bytes, scaled to [0, 1] over the payload's
    domain. Pure and deterministic."""
    if not payload:
        return 0.0
    n = int.from_bytes(payload, "big", signed=False)
    return float(n)


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

    def snapshot_records(self, snap_id: int) -> dict:
        """Build the visible record dict {pk: payload} at this snap."""
        out = {}
        for pk, vs in self.versions.items():
            latest: Optional[Version] = None
            for v in vs:
                if v.commit_snap_id <= snap_id:
                    if latest is None or v.commit_snap_id > latest.commit_snap_id:
                        latest = v
            if latest is not None and latest.payload is not None:
                out[pk] = latest.payload
        return out

    def tx_records(self, tx: Transaction) -> dict:
        """Snapshot records with overlay applied. Tombstones in overlay
        remove the pk from the result."""
        rs = self.snapshot_records(tx.snap_id)
        for pk, pw in tx.overlay.items():
            if pw.payload is None:
                rs.pop(pk, None)
            else:
                rs[pk] = pw.payload
        return rs

    # ---- Geometric reads -----------------------------------------------

    def curvature_committed(self) -> float:
        """K against the latest committed state (no-tx reader)."""
        rs = self.snapshot_records(self.snap_counter)
        if not rs:
            return 0.0
        return sum(_feature_int(v) for v in rs.values()) / len(rs)

    def curvature_under(self, tx: Transaction) -> float:
        """K under a tx: snapshot at tx.snap_id with overlay applied."""
        rs = self.tx_records(tx)
        if not rs:
            return 0.0
        return sum(_feature_int(v) for v in rs.values()) / len(rs)


# ============================================================================
# Cases
# ============================================================================

def tx10_1_no_open_tx_matches_committed() -> bool:
    print("[TX10.1] K(no_open_tx) = K(committed set)")
    eng = MVCCEngine()
    for pk, v in [(b"a", b"\x01"), (b"b", b"\x02"), (b"c", b"\x03")]:
        t = eng.begin(); eng.stage_write(t, pk, v); eng.commit(t)

    # Hand-computed K = (1+2+3)/3 = 2.0
    expected = 2.0
    got = eng.curvature_committed()
    if got != expected:
        print(f"  FAIL got {got}, expected {expected}")
        return False
    print(f"  ok K={got}")
    return True


def tx10_2_tx_no_writes_equals_snapshot_k() -> bool:
    print("[TX10.2] K(in tx T, no writes) = K at T.snap_id")
    eng = MVCCEngine()
    for pk, v in [(b"a", b"\x01"), (b"b", b"\x02")]:
        t = eng.begin(); eng.stage_write(t, pk, v); eng.commit(t)

    t_read = eng.begin()        # snap=2, sees {a:1, b:2}
    expected = 1.5
    got = eng.curvature_under(t_read)
    if got != expected:
        print(f"  FAIL got {got}, expected {expected}")
        return False

    # Now write more in another tx; t_read must still see K=1.5.
    t_other = eng.begin()
    eng.stage_write(t_other, b"c", b"\x09")
    eng.commit(t_other)         # snap=3, latest {a:1,b:2,c:9} K=4.0

    got2 = eng.curvature_under(t_read)
    if got2 != expected:
        print(f"  FAIL t_read drifted: got {got2}, expected {expected}")
        return False
    print(f"  ok t_read K={got2} pinned at snap={t_read.snap_id}")
    return True


def tx10_3_tx_with_overlay_includes_overlay() -> bool:
    print("[TX10.3] K under T with overlay includes overlay overrides")
    eng = MVCCEngine()
    for pk, v in [(b"a", b"\x01"), (b"b", b"\x02")]:
        t = eng.begin(); eng.stage_write(t, pk, v); eng.commit(t)

    t = eng.begin()
    # Stage: override a -> 10, insert c -> 4.
    eng.stage_write(t, b"a", b"\x0a")
    eng.stage_write(t, b"c", b"\x04")

    # Visible set under T: {a:10, b:2, c:4}. K = (10+2+4)/3 = 16/3.
    expected = 16.0 / 3.0
    got = eng.curvature_under(t)
    if abs(got - expected) > 1e-12:
        print(f"  FAIL got {got}, expected {expected}")
        return False

    # Out-of-tx K must NOT reflect T's overlay.
    out_of_tx = eng.curvature_committed()
    expected_out = 1.5
    if out_of_tx != expected_out:
        print(f"  FAIL out-of-tx leaked overlay: got {out_of_tx}, expected {expected_out}")
        return False
    print(f"  ok K_under_T={got:.6f}, K_committed={out_of_tx}")
    return True


def tx10_4_post_commit_old_tx_still_pinned() -> bool:
    print("[TX10.4] After T commits, an earlier-open T' still sees pre-T K")
    eng = MVCCEngine()
    for pk, v in [(b"a", b"\x01"), (b"b", b"\x02")]:
        t = eng.begin(); eng.stage_write(t, pk, v); eng.commit(t)

    # T' opens at snap=2, sees K=1.5.
    t_prime = eng.begin()
    pre = eng.curvature_under(t_prime)
    assert pre == 1.5

    # T opens, inserts, commits. New committed K reflects new set.
    t = eng.begin()
    eng.stage_write(t, b"c", b"\x09")
    eng.commit(t)
    post_committed = eng.curvature_committed()
    assert post_committed == (1 + 2 + 9) / 3

    # T' still sees pre = 1.5.
    still = eng.curvature_under(t_prime)
    if still != pre:
        print(f"  FAIL t_prime drifted: pre={pre}, now={still}")
        return False
    print(f"  ok pre={pre}, post_committed={post_committed:.6f}, t_prime still={still}")
    return True


def tx10_5_pure_function_recompute_stable() -> bool:
    print("[TX10.5] K is a pure function of (snapshot, overlay)")
    eng = MVCCEngine()
    for pk, v in [(b"a", b"\x05"), (b"b", b"\x07"), (b"c", b"\x0b")]:
        t = eng.begin(); eng.stage_write(t, pk, v); eng.commit(t)

    t = eng.begin()
    eng.stage_write(t, b"b", b"\x02")   # override b -> 2
    eng.stage_write(t, b"d", None)      # tombstone for non-existent d (no-op)

    k1 = eng.curvature_under(t)
    k2 = eng.curvature_under(t)
    k3 = eng.curvature_under(t)
    if not (k1 == k2 == k3):
        print(f"  FAIL non-pure: {k1}, {k2}, {k3}")
        return False

    # Hand: visible set = {a:5, b:2, c:11}; K = 18/3 = 6.0.
    if k1 != 6.0:
        print(f"  FAIL got {k1}, expected 6.0")
        return False
    print(f"  ok K={k1} stable across 3 recomputes")
    return True


def main():
    print("=" * 72)
    print("TX10: Geometric reads under SI match the pinned snapshot")
    print("=" * 72)

    cases = [
        ("TX10.1 no-tx matches committed",        tx10_1_no_open_tx_matches_committed),
        ("TX10.2 in-tx no-writes = snapshot K",   tx10_2_tx_no_writes_equals_snapshot_k),
        ("TX10.3 overlay reflected in in-tx K",   tx10_3_tx_with_overlay_includes_overlay),
        ("TX10.4 old tx pinned past commit",      tx10_4_post_commit_old_tx_still_pinned),
        ("TX10.5 K is a pure function",           tx10_5_pure_function_recompute_stable),
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
        print(f"\n  ALL {len(results)} GEOMETRIC-READ CASES GREEN.")
        return 0
    return 1


if __name__ == "__main__":
    sys.exit(main())
