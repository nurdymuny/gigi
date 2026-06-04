"""
Master TDD-gate runner for poincare_to_sharding validation.

Runs T1..T6 in order; reports overall PASS/FAIL with a summary table.
Exit code: 0 if all gates green, 1 otherwise.

Usage:
    python theory/poincare_to_sharding/validation/run_all.py
"""

from __future__ import annotations
import subprocess
import sys
import os
import time


TESTS = [
    # Intra-atlas gates (poincare_to_sharding.md §3.1-§3.6)
    ("T1",  "t1_mayer_vietoris_betti.py",            "Mayer-Vietoris BETTI assembly"),
    ("T2",  "t2_cocycle_bound.py",                   "Cocycle bound on 3-chart S^2 atlas"),
    ("T3",  "t3_sharded_curvature.py",               "Sharded CURVATURE on CP^1 FS"),
    ("T4",  "t4_sharded_holonomy.py",                "Sharded HOLONOMY w/ gauge transition"),
    ("T5",  "t5_cauchy_interlacing_lambda1.py",      "Honest sharded lambda_1 bounds"),
    ("T6",  "t6_clean_finger_move.py",               "Clean Finger Move resolver analog"),
    ("T7",  "t7_distributed_lanczos_spectral.py",    "Distributed Lanczos -- universal SPECTRAL"),
    # Cross-atlas gates (CROSS_ATLAS_JOINS.md §5)
    ("T8",  "t8_bridge_cocycle.py",                  "Cross-atlas bridge cocycle bound"),
    ("T9",  "t9_cross_atlas_betti.py",               "Cross-atlas BETTI via fiber-product M-V"),
    ("T10", "t10_cross_atlas_resolver.py",           "Cross-atlas Clean Finger Move resolver"),
    # Topology-aware partitioning (SHARDING_SPEC.md §9 Phase D)
    ("TFP1", "tfp1_fiedler_preserves_curvature.py",  "Fiedler partition preserves K aggregation"),
]


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    results = []
    print("=" * 72)
    print("poincare_to_sharding TDD gate runner")
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
            tail = "\n".join(r.stdout.strip().splitlines()[-8:])
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
        print(f"  [{flag}] {tag} {desc:<48} {dt:>6.2f}s")

    all_ok = all(ok for (_, _, _, ok, _) in results)
    print()
    if all_ok:
        print(f"  ALL {len(TESTS)} TDD GATES GREEN.")
        print("  poincare_to_sharding.md theory + CROSS_ATLAS_JOINS.md unblocked.")
        return 0
    else:
        n_fail = sum(1 for (_, _, _, ok, _) in results if not ok)
        print(f"  {n_fail} GATE(S) RED.")
        return 1


if __name__ == "__main__":
    sys.exit(main())
