"""
Master TDD-gate runner for the IMAGINE / WALK validation suite.

Runs T11, T12, T13 in order; reports overall PASS/FAIL with a summary.
Exit code: 0 if all three gates green, 1 otherwise.

Usage:
    python theory/imagine/validation/run_all.py
"""

from __future__ import annotations
import subprocess
import sys
import os
import time


TESTS = [
    ("T11", "t11_geodesic_integrator.py",       "Geodesic integrator on S^2, T^2, CP^1"),
    ("T12", "t12_halo_partition_invariance.py", "Halo-as-IMAGINE makes K partition-invariant"),
    ("T13", "t13_double_cover_monodromy.py",    "Double cover monodromy (synthetic + discourse)"),
]


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    results = []
    print("=" * 72)
    print("IMAGINE / WALK TDD gate runner")
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
            tail = "\n".join(r.stdout.strip().splitlines()[-6:])
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
        print(f"  ALL {len(TESTS)} IMAGINE GATES GREEN.")
        print("  IMAGINE_AND_WALK.md spec is math-validated.")
        print("  Rust src/imagine/ scaffold is unblocked.")
        return 0
    else:
        n_fail = sum(1 for (_, _, _, ok, _) in results if not ok)
        print(f"  {n_fail} GATE(S) RED.")
        return 1


if __name__ == "__main__":
    sys.exit(main())
