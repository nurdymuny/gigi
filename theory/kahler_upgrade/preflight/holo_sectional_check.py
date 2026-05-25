"""
L8.4 — Holomorphic sectional curvature pre-flight (catalog §E.5 check 3).

PURPOSE
    Verify that Marcella's embedding manifold has holomorphic
    sectional curvature K_H in an expected range. Two regimes
    matter for catalog citations:

    - K_H ≤ HADAMARD_KB_THRESHOLD (= 0.5 in GIGI L5):
      practically-Hadamard ⇒ §1.4 + §1.5 theorems apply.
    - K_H near the constant-FS value (≈ 4 on disc-uniform data):
      Fano-like ⇒ §E.3 Einstein normalization holds, Ricci
      diversity bound is tight.

    Marcella's runtime cites different theorems in different
    regimes; this pre-flight tells her which regime applies on
    her actual manifold.

GROUND TRUTH
    K_H is computed from per-field Welford statistics via the
    L4 streaming recipe:

        K_H = 64 · (var(f_a) + var(f_b)) / (range(f_a)² + range(f_b)²)

    where (f_a, f_b) is a complex coordinate pair under J. This
    matches `BundleStore::kahler_curvature().holo_sectional`
    exactly (see src/curvature.rs L4 docstring).

USAGE FROM MARCELLA
    Replace `sample_complex_pair_stats()` with a call to your
    runtime's per-field statistics surface. The check then
    enumerates pairs and reports the K_H distribution.

CALL FROM CI
    PYTHONIOENCODING=utf-8 python -X utf8 holo_sectional_check.py
"""

import sys


HADAMARD_KB_THRESHOLD = 0.5  # matches src/geometry/hadamard.rs


def k_h_from_pair_stats(var_a, var_b, range_a, range_b):
    """
    L4 streaming recipe for K_H. Matches `compute_kahler_decomposition`
    exactly. Factor 64 is the Fubini-Study calibration.
    """
    range_sq_sum = range_a * range_a + range_b * range_b
    if range_sq_sum < 1e-300:
        return 0.0
    return 64.0 * (var_a + var_b) / range_sq_sum


def holo_sectional_check(
    pair_stats_iter,
    hadamard_threshold=HADAMARD_KB_THRESHOLD,
    fs_target=4.0,
    fs_tolerance=0.6,
):
    """
    Enumerate complex pairs, compute K_H per pair, classify the
    bundle into Hadamard / FS-like / mixed regimes.

    Returns dict with:
      - 'k_h_values': list of K_H per pair
      - 'min_k_h', 'max_k_h', 'mean_k_h'
      - 'all_below_threshold': True iff every pair has K_H ≤ threshold
        (⇒ §1.4 / §1.5 theorems apply globally)
      - 'all_near_fs': True iff every pair has |K_H - 4| < fs_tolerance
        (⇒ §E.3 Einstein normalization tight)
      - 'regime': "hadamard" | "fs_like" | "mixed" | "high_curvature"
    """
    k_h_values = [
        k_h_from_pair_stats(va, vb, ra, rb)
        for (va, vb, ra, rb) in pair_stats_iter
    ]
    if not k_h_values:
        return {
            "k_h_values": [],
            "min_k_h": 0.0,
            "max_k_h": 0.0,
            "mean_k_h": 0.0,
            "all_below_threshold": False,
            "all_near_fs": False,
            "regime": "no_pairs",
        }

    mn = min(k_h_values)
    mx = max(k_h_values)
    mean = sum(k_h_values) / len(k_h_values)
    all_below = all(k <= hadamard_threshold for k in k_h_values)
    all_near_fs = all(abs(k - fs_target) < fs_tolerance for k in k_h_values)

    if all_below:
        regime = "hadamard"
    elif all_near_fs:
        regime = "fs_like"
    elif mn <= hadamard_threshold and mx > hadamard_threshold:
        regime = "mixed"
    else:
        regime = "high_curvature"

    return {
        "k_h_values": k_h_values,
        "min_k_h": mn,
        "max_k_h": mx,
        "mean_k_h": mean,
        "all_below_threshold": all_below,
        "all_near_fs": all_near_fs,
        "regime": regime,
    }


# ============================================================
# Synthetic controls
# ============================================================

def control_flat_data_is_hadamard():
    """All-zero variance ⇒ K_H = 0 ⇒ Hadamard regime."""
    print("\n=== Control: flat data (var = 0) ===")
    pairs = [(0.0, 0.0, 1.0, 1.0)] * 3
    result = holo_sectional_check(pairs)
    print(f"  regime = {result['regime']}, K_H = {result['k_h_values']}")
    assert result["regime"] == "hadamard"
    assert result["all_below_threshold"]
    print("  PASS")


def control_disc_uniform_is_fs_like():
    """
    Disc-uniform data has Var(x) ≈ 1/4, range = 2 (per L4
    docstring). K_H = 64 · 0.5 / 8 = 4. Matches CP¹ FS.
    """
    print("\n=== Control: disc-uniform data (FS-like) ===")
    # var = 1/4 each, range = 2 each, exactly.
    pairs = [(0.25, 0.25, 2.0, 2.0)] * 3
    result = holo_sectional_check(pairs)
    print(f"  regime = {result['regime']}, mean K_H = {result['mean_k_h']:.4f}")
    assert result["regime"] == "fs_like"
    assert result["all_near_fs"]
    print("  PASS")


def control_mixed_data_classified_as_mixed():
    """One Hadamard pair, one FS-like pair ⇒ "mixed" regime."""
    print("\n=== Control: mixed data (one Hadamard, one FS-like) ===")
    pairs = [
        (0.0, 0.0, 1.0, 1.0),       # K_H = 0 (Hadamard)
        (0.25, 0.25, 2.0, 2.0),     # K_H = 4 (FS-like)
    ]
    result = holo_sectional_check(pairs)
    print(f"  regime = {result['regime']}, K_H = {result['k_h_values']}")
    assert result["regime"] == "mixed"
    assert not result["all_below_threshold"]
    assert not result["all_near_fs"]
    print("  PASS")


def control_high_curvature_data_classified_as_high():
    """
    Data with K_H far above FS (e.g., concentrated near boundary)
    triggers the "high_curvature" diagnostic — Marcella should
    NOT cite §1.4 / §1.5 in that regime.
    """
    print("\n=== Control: high-curvature data (K_H ≫ 4) ===")
    # Var = range/4 (tight; close to limit). K_H = 64 · (range/2) / (2·range²) = 16/range.
    # For range = 1: K_H = 16. (= 4× FS).
    pairs = [(0.25, 0.25, 1.0, 1.0)] * 3
    result = holo_sectional_check(pairs)
    print(f"  regime = {result['regime']}, mean K_H = {result['mean_k_h']:.4f}")
    assert result["regime"] == "high_curvature"
    print("  PASS")


# ============================================================
# Entry point
# ============================================================

if __name__ == "__main__":
    print("L8.4 Holomorphic-sectional pre-flight — synthetic controls")
    print("=" * 60)
    failures = []
    for name, fn in [
        ("flat_is_hadamard", control_flat_data_is_hadamard),
        ("disc_uniform_is_fs", control_disc_uniform_is_fs_like),
        ("mixed_is_mixed", control_mixed_data_classified_as_mixed),
        ("high_curvature_is_high", control_high_curvature_data_classified_as_high),
    ]:
        try:
            fn()
        except AssertionError as e:
            failures.append((name, str(e)))

    if failures:
        print("\nFAILURES:")
        for n, e in failures:
            print(f"  {n}: {e}")
        sys.exit(1)
    print("\nAll synthetic controls passed.")
    print("\nNEXT: implement `pair_stats_iter` from your Marcella runtime")
    print("(yields (var_a, var_b, range_a, range_b) per complex pair) and")
    print("call `holo_sectional_check(your_iter)`. The returned `regime`")
    print("tells you which catalog theorems Marcella v3 can cite on your")
    print("substrate:")
    print("  - 'hadamard':       cite §1.4 + §1.5 (ideal boundary + invertibility)")
    print("  - 'fs_like':        cite §E.3 Einstein-normalization Ricci bound")
    print("  - 'mixed':          cite per-region; use is_hadamard_region(query)")
    print("  - 'high_curvature': avoid Hadamard citations; use streaming bounds only")
    sys.exit(0)
