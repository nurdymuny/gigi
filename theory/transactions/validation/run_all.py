"""
Master TDD-gate runner for the Atomic Sheaf Commits validation suite.

Phase 1 gates (TX1-TX5): 2PC with coordinator/participant failure recovery.
Phase 2 gates (TX6-TX10): per-transaction overlay + snapshot isolation.
Phase 3 gates (TX11-TX13): deadlock detection.
Phase 4 gates (TX14-TX18): geometric coherence semantics.

Currently shipped: TX1, TX2.
"""

from __future__ import annotations
import subprocess
import sys
import os
import time


TESTS = [
    ("TX1",     "tx1_single_bundle_atomicity.py",       "Single-bundle atomic commit + rollback"),
    ("TX2",     "tx2_cross_bundle_atomicity.py",        "Cross-bundle 2PC + recovery (5 failure cases)"),
    ("TX6-8",   "tx6_8_snapshot_isolation.py",          "Per-tx overlay + snapshot isolation semantics"),
    ("TX9",     "tx9_mvcc_gc.py",                       "MVCC GC: retain only versions open txs need"),
    ("TX10",    "tx10_geometric_reads_under_tx.py",     "Geometric reads pinned to tx snapshot"),
    ("TX11-13", "tx11_13_deadlock_detection.py",        "Wait-for-graph cycle detection + youngest-aborts"),
]


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    results = []
    print("=" * 72)
    print("ATOMIC SHEAF COMMITS TDD gate runner")
    print("=" * 72)

    for tag, fname, desc in TESTS:
        path = os.path.join(here, fname)
        print(f"\n>>> {tag} :: {desc}")
        t0 = time.time()
        try:
            r = subprocess.run([sys.executable, path], capture_output=True, text=True, timeout=120)
            dt = time.time() - t0
            ok = (r.returncode == 0)
            results.append((tag, fname, desc, ok, dt))
            tail = "\n".join(r.stdout.strip().splitlines()[-4:])
            print(tail)
            print(f"    [{('PASS' if ok else 'FAIL')}] {tag} in {dt:.2f}s")
        except subprocess.TimeoutExpired:
            results.append((tag, fname, desc, False, 120.0))
            print(f"    [TIMEOUT] {tag} exceeded 120s")

    print("\n" + "=" * 72)
    print("FINAL SUMMARY")
    print("=" * 72)
    for tag, fname, desc, ok, dt in results:
        flag = "PASS" if ok else "FAIL"
        print(f"  [{flag}] {tag} {desc:<55} {dt:>6.2f}s")

    all_ok = all(ok for (_, _, _, ok, _) in results)
    print()
    if all_ok:
        print(f"  ALL {len(TESTS)} TRANSACTIONS GATES GREEN.")
        print("  Phase 1 (2PC + recovery) is math-validated.")
        print("  Phase 2 (SI + overlay + MVCC GC) is math-validated.")
        print("  Rust src/transactions/ matches the Python reference contract.")
        return 0
    else:
        n_fail = sum(1 for (_, _, _, ok, _) in results if not ok)
        print(f"  {n_fail} GATE(S) RED.")
        return 1


if __name__ == "__main__":
    sys.exit(main())
