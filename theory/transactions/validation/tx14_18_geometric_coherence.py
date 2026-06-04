"""
TX14-TX18: Geometric coherence via Option C (MVCC-style geometric
snapshots).

================================================================================
CLAIMS (ATOMIC_SHEAF_COMMIT_SPEC.md §6 Option C, §6.3):

  TX14  Out-of-tx geometric read during an open tx returns the PRE-tx
        committed state (not the in-flight tx's modifications).

  TX15  Out-of-tx geometric read after the tx commits returns the
        POST-tx state.

  TX16  Cocycle bound preserved across commit: a transaction whose
        commit would violate the temporal cocycle bound
        ‖δ_{ij}(S_{t+1}) - δ_{ij}(S_t)‖ ≤ B_{ij} is REFUSED with
        CocycleViolation; the substrate state is unchanged.

  TX17  Holonomy walker entering a region during a concurrent tx
        sees either the PRE-tx or the POST-tx connection — never an
        interleaving. (Read-consistency for path-walking primitives.)

  TX18  Geometric snapshot storage scales linearly in the number of
        open transactions, NOT in the bundle size. (Storage = O(N),
        N = open-tx count.)

GROUND TRUTH (independent):

    The same MVCC machinery from Phase 2 is now used to pin GEOMETRIC
    quantities (curvature K, cocycle slack δ, holonomy H) at each
    transaction's snap_id. Reads under transaction T return
    Geometric(snapshot_records_at(T.snap_id) UNION T.overlay). Reads
    out of any tx return Geometric(high_water snapshot). The
    coherence comes for free from the snapshot pin already validated
    by TX10.

    The new ingredient is the COMMIT-TIME validation: before
    persisting an overlay at a fresh snap_id, the engine recomputes
    cocycle slack for every pair of touched bundles' overlapping
    charts and refuses the commit if the temporal-edge bound is
    exceeded. This is §2.2 + §6.3 TX16.

PASS CRITERION:

  TX14    Out-of-tx K during open T = K(committed_pre_T)
  TX15    Out-of-tx K after T.commit = K(committed_post_T)
  TX16.1  Commit with delta-slack inside budget -> Committed
  TX16.2  Commit with delta-slack exceeding budget -> CocycleViolation;
          substrate state unchanged; high-water did not advance
  TX17    Walker holonomy across a touched region during open T
          equals either pre-T or post-T holonomy, never partial
  TX18    snapshot_overhead = c1 + c2 * open_tx_count + O(1) in
          bundle size

CIRCULAR-LOGIC GUARDS:

    1. The PRE/POST geometry is computed by re-running K against the
       physical record set at the snap; no caching, no reuse of the
       under-T value to assert the out-of-T value.
    2. The temporal cocycle check uses the SAME slack formula as the
       Phase A T2 static check (Davis 2026b Def 21); we don't invent
       a new bound for time.
    3. The storage measurement counts per-tx overlay bytes only;
       bundle-wide structures are not double-counted.
================================================================================
"""

from __future__ import annotations
import math
import sys
from dataclasses import dataclass, field
from typing import Optional


# ============================================================================
# Reference: MVCC store + per-tx overlay (cut-down from tx6_8)
# ============================================================================

@dataclass
class Version:
    payload: Optional[float]
    commit_snap_id: int


@dataclass
class PendingWrite:
    pk: bytes
    payload: Optional[float]


@dataclass
class Transaction:
    tx_id: int
    snap_id: int
    overlay: dict = field(default_factory=dict)
    state: str = "open"


class CocycleViolation(Exception):
    pass


# ============================================================================
# Engine
# ============================================================================

class GeoEngine:
    """Multi-bundle MVCC engine that pins geometric reads (curvature K
    and cocycle slack δ) at each transaction's snap_id."""

    def __init__(self, cocycle_budget: float = 0.5):
        # bundle -> {pk -> [Version, ...]}
        self.bundles: dict = {}
        self.snap_counter: int = 0
        self.txs: dict = {}
        self._tx_counter: int = 0
        self.cocycle_budget = cocycle_budget

    # ---- snap / tx lifecycle ------------------------------------------

    def next_snap_id(self) -> int:
        self.snap_counter += 1
        return self.snap_counter

    def begin(self) -> Transaction:
        self._tx_counter += 1
        tx = Transaction(tx_id=self._tx_counter, snap_id=self.snap_counter)
        self.txs[tx.tx_id] = tx
        return tx

    def stage(self, tx: Transaction, bundle: str, pk: bytes, payload: Optional[float]) -> None:
        assert tx.state == "open"
        tx.overlay.setdefault(bundle, {})[pk] = PendingWrite(pk=pk, payload=payload)

    def commit(self, tx: Transaction) -> int:
        """Apply overlay at a fresh snap. Refuses if the temporal
        cocycle bound is violated."""
        assert tx.state == "open"
        # Cocycle pre-flight against the NEXT-snap state.
        post_state = self._project_post(tx)
        self._check_temporal_cocycle(post_state)
        new_snap = self.next_snap_id()
        for bundle, writes in tx.overlay.items():
            self.bundles.setdefault(bundle, {})
            for pk, pw in writes.items():
                self.bundles[bundle].setdefault(pk, []).append(
                    Version(payload=pw.payload, commit_snap_id=new_snap)
                )
        tx.state = "committed"
        return new_snap

    def abort(self, tx: Transaction) -> None:
        assert tx.state == "open"
        tx.overlay.clear()
        tx.state = "aborted"

    # ---- read paths ---------------------------------------------------

    def _records_at(self, bundle: str, snap_id: int) -> dict:
        out: dict = {}
        for pk, vs in self.bundles.get(bundle, {}).items():
            latest = None
            for v in vs:
                if v.commit_snap_id <= snap_id:
                    if latest is None or v.commit_snap_id > latest.commit_snap_id:
                        latest = v
            if latest is not None and latest.payload is not None:
                out[pk] = latest.payload
        return out

    def _records_under(self, tx: Transaction, bundle: str) -> dict:
        rs = self._records_at(bundle, tx.snap_id)
        for pk, pw in tx.overlay.get(bundle, {}).items():
            if pw.payload is None:
                rs.pop(pk, None)
            else:
                rs[pk] = pw.payload
        return rs

    # ---- geometric primitives -----------------------------------------

    def k_committed(self, bundle: str) -> float:
        rs = self._records_at(bundle, self.snap_counter)
        return (sum(rs.values()) / len(rs)) if rs else 0.0

    def k_under(self, tx: Transaction, bundle: str) -> float:
        rs = self._records_under(tx, bundle)
        return (sum(rs.values()) / len(rs)) if rs else 0.0

    def delta_committed(self, b1: str, b2: str) -> float:
        """Cocycle slack: |K(b1) - K(b2)| against committed state."""
        return abs(self.k_committed(b1) - self.k_committed(b2))

    def delta_under(self, tx: Transaction, b1: str, b2: str) -> float:
        return abs(self.k_under(tx, b1) - self.k_under(tx, b2))

    # ---- holonomy stand-in --------------------------------------------
    # A "holonomy" here is the discrete-loop product around a chosen
    # base loop. We model it as sum(records on the loop) mod 1.0.

    def holonomy_committed(self, bundle: str) -> float:
        rs = self._records_at(bundle, self.snap_counter)
        return math.fmod(sum(rs.values()), 1.0)

    def holonomy_under(self, tx: Transaction, bundle: str) -> float:
        rs = self._records_under(tx, bundle)
        return math.fmod(sum(rs.values()), 1.0)

    # ---- cocycle pre-flight -------------------------------------------

    def _project_post(self, tx: Transaction) -> dict:
        """Return the materialized record dicts the commit would
        produce, without mutating bundle storage."""
        post: dict = {}
        for bundle in set(list(self.bundles.keys()) + list(tx.overlay.keys())):
            rs = self._records_at(bundle, self.snap_counter)
            for pk, pw in tx.overlay.get(bundle, {}).items():
                if pw.payload is None:
                    rs.pop(pk, None)
                else:
                    rs[pk] = pw.payload
            post[bundle] = rs
        return post

    def _check_temporal_cocycle(self, post: dict) -> None:
        """For every bundle pair, ensure |δ_post - δ_pre| <= B."""
        bundles = sorted(post.keys())
        def k_of(rs):
            return (sum(rs.values()) / len(rs)) if rs else 0.0
        for i in range(len(bundles)):
            for j in range(i + 1, len(bundles)):
                b1, b2 = bundles[i], bundles[j]
                delta_pre = abs(self.k_committed(b1) - self.k_committed(b2))
                delta_post = abs(k_of(post[b1]) - k_of(post[b2]))
                if abs(delta_post - delta_pre) > self.cocycle_budget:
                    raise CocycleViolation(
                        f"pair ({b1},{b2}): |Δδ|={abs(delta_post - delta_pre):.3f} > B={self.cocycle_budget}"
                    )

    # ---- storage measurement ------------------------------------------

    def snapshot_storage_bytes(self) -> int:
        """Crude approximation: sum of overlay sizes across open txs.
        Each PendingWrite is counted as len(pk) + 8 bytes."""
        total = 0
        for tx in self.txs.values():
            if tx.state != "open":
                continue
            for bundle_overlay in tx.overlay.values():
                for pk, pw in bundle_overlay.items():
                    total += len(pk) + 8
        return total


# ============================================================================
# Cases
# ============================================================================

def tx14_out_of_tx_during_open_tx_sees_pre_state() -> bool:
    print("[TX14] Out-of-tx read during open T returns pre-T committed K")
    eng = GeoEngine()
    seed = eng.begin()
    eng.stage(seed, "users", b"a", 1.0)
    eng.stage(seed, "users", b"b", 2.0)
    eng.commit(seed)        # snap=1, K=1.5

    t = eng.begin()
    eng.stage(t, "users", b"c", 99.0)
    # While T is open, out-of-tx K reflects only snap=1.
    out = eng.k_committed("users")
    if abs(out - 1.5) > 1e-12:
        print(f"  FAIL: out-of-tx saw {out}, expected 1.5")
        return False
    # T's own view includes the staged record.
    seen_inside = eng.k_under(t, "users")
    if abs(seen_inside - (1+2+99)/3) > 1e-12:
        print(f"  FAIL: in-tx K {seen_inside} not the overlay-augmented value")
        return False
    print(f"  ok K_committed={out}, K_inside_T={seen_inside:.3f}")
    return True


def tx15_post_commit_out_of_tx_sees_post_state() -> bool:
    print("[TX15] After commit, out-of-tx K reflects new state")
    eng = GeoEngine()
    seed = eng.begin()
    eng.stage(seed, "users", b"a", 1.0)
    eng.stage(seed, "users", b"b", 2.0)
    eng.commit(seed)

    t = eng.begin()
    eng.stage(t, "users", b"c", 9.0)
    eng.commit(t)
    out = eng.k_committed("users")
    if abs(out - (1+2+9)/3) > 1e-12:
        print(f"  FAIL: post-commit K {out}, expected {(1+2+9)/3}")
        return False
    print(f"  ok K_committed={out}")
    return True


def tx16_cocycle_bound_at_commit() -> bool:
    print("[TX16] Cocycle bound is checked at commit time")
    # Budget B=0.5. Pre: K_u=1.0, K_o=1.0 -> δ_pre=0.
    # Post under bad tx pushes K_u to 10 -> δ_post=9, ΔΔ=9 > 0.5 -> refuse.
    eng = GeoEngine(cocycle_budget=0.5)
    s = eng.begin()
    eng.stage(s, "users",  b"a", 1.0)
    eng.stage(s, "orders", b"x", 1.0)
    eng.commit(s)
    snap_pre = eng.snap_counter

    # TX16.1: small change, inside budget.
    t1 = eng.begin()
    eng.stage(t1, "users", b"b", 1.5)
    try:
        eng.commit(t1)
        ok1 = True
    except CocycleViolation as e:
        print(f"  FAIL TX16.1: spurious refusal: {e}")
        return False
    if not ok1:
        print("  FAIL TX16.1")
        return False
    print("  ok TX16.1 small change committed")

    # TX16.2: large change, refused.
    t2 = eng.begin()
    eng.stage(t2, "users", b"c", 100.0)
    snap_before_bad = eng.snap_counter
    try:
        eng.commit(t2)
        print("  FAIL TX16.2: out-of-budget commit was allowed")
        return False
    except CocycleViolation as e:
        pass
    if eng.snap_counter != snap_before_bad:
        print(f"  FAIL TX16.2: snap advanced from {snap_before_bad} to {eng.snap_counter}")
        return False
    print(f"  ok TX16.2 out-of-budget commit refused, snap held at {eng.snap_counter}")
    return True


def tx17_holonomy_walker_sees_pre_or_post_not_partial() -> bool:
    print("[TX17] Holonomy walker entering touched region sees pre- OR post-, not both")
    eng = GeoEngine(cocycle_budget=10.0)
    s = eng.begin()
    eng.stage(s, "bundleA", b"r1", 0.1)
    eng.stage(s, "bundleA", b"r2", 0.2)
    eng.commit(s)
    pre_h = eng.holonomy_committed("bundleA")

    # Walker opens its tx at the pre snap.
    walker = eng.begin()

    # Other tx writes mid-walk.
    other = eng.begin()
    eng.stage(other, "bundleA", b"r3", 0.4)
    eng.commit(other)
    post_h = eng.holonomy_committed("bundleA")

    walker_view = eng.holonomy_under(walker, "bundleA")
    if not (math.isclose(walker_view, pre_h) or math.isclose(walker_view, post_h)):
        print(f"  FAIL: walker saw {walker_view}, not in (pre={pre_h}, post={post_h})")
        return False
    # In Option C the walker pinned at the pre-snap MUST see pre_h.
    if not math.isclose(walker_view, pre_h):
        print(f"  FAIL: pinned walker should see pre_h={pre_h}, saw {walker_view}")
        return False
    print(f"  ok walker pinned at pre snap sees {walker_view}, not post {post_h}")
    return True


def tx18_snapshot_storage_scales_with_open_tx_count() -> bool:
    print("[TX18] Snapshot storage is O(open_tx_count), not O(bundle_size)")
    eng = GeoEngine(cocycle_budget=100.0)
    # Seed a moderately-sized bundle.
    seed = eng.begin()
    for i in range(1000):
        eng.stage(seed, "u", f"k{i}".encode(), float(i))
    eng.commit(seed)

    # No open txs -> snapshot overhead is zero.
    if eng.snapshot_storage_bytes() != 0:
        print(f"  FAIL: no-tx overhead = {eng.snapshot_storage_bytes()}")
        return False

    # Open N txs, each writes 1 record.
    txs = []
    for n in range(1, 21):
        t = eng.begin()
        eng.stage(t, "u", f"new{n}".encode(), float(n))
        txs.append(t)
        per = eng.snapshot_storage_bytes() / len(txs)
        # Each overlay entry: ~12 bytes (pk+8). Linear, bounded.
        if not (8 <= per <= 100):
            print(f"  FAIL: per-tx overhead {per} bytes outside expected range")
            return False

    final = eng.snapshot_storage_bytes()
    if final < 200 or final > 2000:
        print(f"  FAIL: final overhead {final} bytes, expected ~20*~12 = ~240")
        return False
    print(f"  ok storage = {final} bytes across {len(txs)} txs on 1000-row bundle")
    return True


def main():
    print("=" * 72)
    print("TX14-TX18: Geometric coherence (Option C — MVCC geometric snapshots)")
    print("=" * 72)
    cases = [
        ("TX14 out-of-tx during open tx = pre state",      tx14_out_of_tx_during_open_tx_sees_pre_state),
        ("TX15 out-of-tx after commit = post state",       tx15_post_commit_out_of_tx_sees_post_state),
        ("TX16 cocycle bound checked at commit",           tx16_cocycle_bound_at_commit),
        ("TX17 walker sees pre- or post-, not partial",    tx17_holonomy_walker_sees_pre_or_post_not_partial),
        ("TX18 snapshot storage = O(open_tx_count)",        tx18_snapshot_storage_scales_with_open_tx_count),
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
        print(f"\n  ALL {len(results)} GEOMETRIC-COHERENCE CASES GREEN.")
        return 0
    return 1


if __name__ == "__main__":
    sys.exit(main())
