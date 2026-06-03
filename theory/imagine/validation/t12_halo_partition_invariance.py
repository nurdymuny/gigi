"""
T12: Halo-as-IMAGINE makes sharded CURVATURE partition-invariant.

================================================================================
CLAIM (IMAGINE_AND_WALK.md §7 T12):
    The Phase D learning -- hash sharding fragments the k-NN
    neighborhood graph, so per-chart K depends on partition shape --
    is solved by populating each chart with halo records via
    `imagine_halo`. With halos present, the aggregate K_sum across
    charts is IDENTICAL to the direct single-shard K_sum, AND
    invariant across multiple partition shapes.

    This validates the gauge-equivariance claim: the per-chart
    CURVATURE recipe COMMUTES with the hash-partition operation
    when halos provide the missing boundary data.

DEEP CONNECTION (gauge-equivariance, encrypt parallel):
    Encrypt v0.3-v0.4 designed `rho_inv` such that for SUM/AVG/VAR:
        rho_inv(sigma(rho(x_1), ..., rho(x_n))) = sigma(x_1, ..., x_n)
    i.e., aggregate-in-ciphertext-then-decrypt = aggregate-on-plaintext.
    SUM-under-Affine works; MIN-under-Probabilistic-sigma>0 was refused
    because no closed-form rho_inv existed.

    Sharding under hash partition has the same structure. We want:
        sheafify(K_chart_1(records_1, halo_1), ...,
                 K_chart_N(records_N, halo_N))
            = K_global(union(records_1, ..., records_N))

    Without halos, the k-NN graph fragments and the equality fails
    (Phase D finding: 60 records / 2 vs 8 charts -> k_sum 15.4 vs 12.9).
    With halos populated via imagine_halo, the equality holds.
    THIS is the gauge-equivariance proof.

REFERENCES:
    - IMAGINE_AND_WALK.md §7 T12 (this test's spec entry, including
      the explicit n_charts in {2, 4, 8} requirement from Marcella's
      feedback round 2).
    - poincare_to_sharding.md §3.3 / T3 (sharded CURVATURE math claim,
      which the Rust Phase D implementation cannot satisfy without
      halos).
    - SHARDING_SPEC.md §5.1 (the per-verb sharded execution recipe
      that IMAGINE/halo refactor will make exact).

GROUND TRUTH (independent):
    Direct K computation on the FULL 60-record dataset (no partition).
    Each record's K is computed from its k=8 nearest neighbors in the
    full set. Sum across all records is the baseline.

TEST DESIGN:
    1. Generate 60 synthetic records as 3D points on a noisy unit
       sphere -- a structured manifold where k-NN curvature is
       meaningful.

    2. Compute K_baseline[i] for each record using k-NN over the
       FULL 60-record set. Sum is k_sum_baseline.

    3. Hash-partition the records into n_charts in {2, 4, 8} via
       hash(record_id) mod n_charts. Three independent partitions
       of the SAME dataset.

    4. For each partition:

       (a) WITHOUT halo (Phase D current behavior):
           Compute K[i] for each record using k-NN restricted to its
           own chart's records.
           Sum k_sum_no_halo for this partition.
           Expected: k_sum_no_halo != k_sum_baseline (partition
           fragments k-NN).

       (b) WITH halo (IMAGINE pivot):
           For each chart c, compute its halo: the records in OTHER
           charts that would be in c's records' k-NN if we computed
           globally. This is the imagine_halo primitive in its
           simplest form (intra-bundle, real records projected via
           identity transition).
           Compute K[i] for each record in c using k-NN over
           c's records UNION c's halo.
           Sum k_sum_with_halo for this partition.
           Expected: k_sum_with_halo == k_sum_baseline exactly.

    5. Cross-partition invariance check:
       k_sum_with_halo[n=2] == k_sum_with_halo[n=4]
                            == k_sum_with_halo[n=8]
                            == k_sum_baseline
       (all to machine precision)

PASS CRITERION:
    Three parts, all must hold:

    A) WITHOUT-halo Phase D finding reproduced:
       k_sum_no_halo differs from baseline for AT LEAST ONE n_charts
       in {2, 4, 8}, by at least 5% relative -- documenting that the
       fragmentation is real and the test is exercising the right
       failure mode.

    B) WITH-halo partition-invariance:
       For all n_charts in {2, 4, 8}:
         |k_sum_with_halo[n] - k_sum_baseline| < 1e-10

    C) WITH-halo cross-partition agreement:
       max(k_sum_with_halo) - min(k_sum_with_halo) < 1e-10
       across n_charts in {2, 4, 8}.

CIRCULAR-LOGIC GUARDS:
    1. The halo construction routine takes ONLY (records, partition,
       k) and returns halos. It does NOT consult the baseline K.
    2. The K function is deterministic and identical in all three
       paths (baseline, no-halo, with-halo). The only thing that
       changes is the NEIGHBORHOOD SET passed to K.
    3. The synthetic records are generated with a fixed seed for
       reproducibility. No data-dependent tuning of K or halos.
    4. The hash partition is a deterministic function of record_id;
       no shuffling between WITHOUT and WITH halo runs.
================================================================================
"""

from __future__ import annotations
import math
import sys
from typing import Callable
import numpy as np


# ============================================================================
# Synthetic substrate: 3D points on a noisy unit sphere
# ============================================================================


def generate_records(n: int, seed: int = 42) -> np.ndarray:
    """
    Generate `n` 3D points on a noisy unit sphere. Each row is a record.

    Structured enough that k-NN curvature is well-defined; noisy enough
    that the k-NN graph has interesting structure.
    """
    rng = np.random.RandomState(seed)
    # Uniform on sphere via normalized Gaussian
    raw = rng.normal(size=(n, 3))
    raw = raw / np.linalg.norm(raw, axis=1, keepdims=True)
    # Slight radial noise so the sphere isn't perfectly singular
    noise = 0.02 * rng.normal(size=(n, 3))
    return raw + noise


# ============================================================================
# K function -- deterministic, neighborhood-dependent
# ============================================================================


def k_at_record(
    record_i: int,
    candidate_indices: list[int],
    records: np.ndarray,
    k: int = 8,
) -> float:
    """
    Synthetic K(record_i) computed from the k nearest neighbors among
    `candidate_indices`. The K value is the MEAN of squared distances
    to the k nearest -- a deterministic monotone function of the
    local neighborhood structure.

    This stand-in models GIGI's `compute_record_k` in the property
    that matters: K depends on which records are in `candidate_indices`.
    If you change the candidate set, you change K.
    """
    if record_i not in candidate_indices:
        # The record must be in its own candidate set; the k-NN
        # search EXCLUDES self.
        raise ValueError(
            f"record {record_i} not in candidate_indices for K computation"
        )
    others = [j for j in candidate_indices if j != record_i]
    if len(others) == 0:
        return 0.0
    diffs = records[others] - records[record_i]
    dists_sq = np.sum(diffs * diffs, axis=1)
    k_eff = min(k, len(others))
    # k smallest squared distances
    nearest = np.partition(dists_sq, k_eff - 1)[:k_eff] if k_eff < len(others) else dists_sq
    return float(np.mean(nearest))


def sum_k_over_records(
    record_indices: list[int],
    candidates_per_record: dict[int, list[int]],
    records: np.ndarray,
    k: int = 8,
) -> float:
    """
    Sum K(i) over a list of record indices, where each record's K is
    computed against `candidates_per_record[i]`.

    This is the load-bearing routine: pass {i: full_dataset} for the
    baseline; pass {i: same_chart_only} for without-halo;
    pass {i: same_chart + halo} for with-halo.
    """
    return sum(
        k_at_record(i, candidates_per_record[i], records, k=k)
        for i in record_indices
    )


# ============================================================================
# Partition + halo construction
# ============================================================================


def hash_partition(record_indices: list[int], n_charts: int) -> dict[int, list[int]]:
    """
    Deterministic hash partition: record `i` lands in chart `i % n_charts`.
    Returns: chart_id -> list of record indices.
    """
    out: dict[int, list[int]] = {c: [] for c in range(n_charts)}
    for i in record_indices:
        out[i % n_charts].append(i)
    return out


def compute_halo_for_chart(
    chart_records: list[int],
    other_records: list[int],
    records: np.ndarray,
    k: int = 8,
) -> list[int]:
    """
    For chart with `chart_records`, compute its halo:
    the records in `other_records` that would be in some
    chart_record's k-NN if we computed globally over `chart_records`
    ∪ `other_records`.

    This is the `imagine_halo` primitive in its simplest form.
    Intra-bundle hash-sharded case -- the imagined records are real
    records from other charts; the IMAGINE projection is the
    identity transition. For cross-atlas the projection would be
    non-trivial (bridge translator).

    Circular-logic guard: this function takes only (chart_records,
    other_records, records, k). It does NOT consult the baseline K
    or the partition shape outside the inputs.
    """
    halo_set: set[int] = set()
    full_candidate_set = chart_records + other_records
    for ci in chart_records:
        # Find which records from `other_records` are in ci's k-NN.
        others_excluding_self = [j for j in full_candidate_set if j != ci]
        diffs = records[others_excluding_self] - records[ci]
        dists_sq = np.sum(diffs * diffs, axis=1)
        # k smallest indices
        k_eff = min(k, len(others_excluding_self))
        if k_eff == 0:
            continue
        nearest_pos = np.argpartition(dists_sq, k_eff - 1)[:k_eff]
        for pos in nearest_pos:
            j = others_excluding_self[pos]
            if j in other_records:
                halo_set.add(j)
    return sorted(halo_set)


# ============================================================================
# The three computational paths
# ============================================================================


def k_sum_baseline(records: np.ndarray, k: int = 8) -> float:
    """Direct K computation on the full dataset -- ground truth."""
    n = records.shape[0]
    all_indices = list(range(n))
    candidates = {i: all_indices for i in all_indices}
    return sum_k_over_records(all_indices, candidates, records, k=k)


def k_sum_sharded_no_halo(records: np.ndarray, n_charts: int, k: int = 8) -> float:
    """
    Phase D current behavior: each chart's K uses only same-chart
    records as k-NN candidates. Documents the fragmentation.
    """
    n = records.shape[0]
    all_indices = list(range(n))
    partition = hash_partition(all_indices, n_charts)
    total = 0.0
    for chart_id, chart_records in partition.items():
        candidates = {i: chart_records for i in chart_records}
        total += sum_k_over_records(chart_records, candidates, records, k=k)
    return total


def k_sum_sharded_with_halo(records: np.ndarray, n_charts: int, k: int = 8) -> float:
    """
    IMAGINE/halo path: each chart's records get k-NN candidates =
    chart records UNION halo records (the imagine_halo result).

    Expected: equals k_sum_baseline exactly (gauge-equivariance).
    """
    n = records.shape[0]
    all_indices = list(range(n))
    partition = hash_partition(all_indices, n_charts)
    total = 0.0
    for chart_id, chart_records in partition.items():
        other_records = [i for i in all_indices if i not in chart_records]
        halo = compute_halo_for_chart(chart_records, other_records, records, k=k)
        # Each chart record's k-NN candidates = chart + halo
        candidates_set = chart_records + halo
        candidates = {i: candidates_set for i in chart_records}
        total += sum_k_over_records(chart_records, candidates, records, k=k)
    return total


# ============================================================================
# Test runner
# ============================================================================


def main():
    print("=" * 72)
    print("T12: Halo-as-IMAGINE makes sharded CURVATURE partition-invariant")
    print("=" * 72)
    print("  60 synthetic records on a noisy S^2. Hash-partition into")
    print("  n_charts in {2, 4, 8}. Compare three paths:")
    print("    (1) baseline -- direct K on full dataset")
    print("    (2) sharded WITHOUT halo -- Phase D current behavior")
    print("    (3) sharded WITH halo -- IMAGINE pivot")
    print()

    records = generate_records(60, seed=42)
    k = 8
    print(f"  records: 60 3D points; k-NN with k = {k}")

    # Part (0): baseline
    baseline = k_sum_baseline(records, k=k)
    print(f"\n  baseline k_sum (full dataset, no partition) : {baseline:.6f}")

    # Part (A): without halo -- documenting fragmentation
    print("\n-- PART A: WITHOUT halo (Phase D fragmentation) " + "-" * 22)
    no_halo_sums = {}
    for n_charts in [2, 4, 8]:
        s = k_sum_sharded_no_halo(records, n_charts=n_charts, k=k)
        no_halo_sums[n_charts] = s
        rel_diff = abs(s - baseline) / abs(baseline)
        print(f"  n_charts={n_charts}: k_sum = {s:.6f}  rel diff from baseline = {rel_diff:.3%}")

    no_halo_max_diff = max(
        abs(no_halo_sums[n] - baseline) for n in [2, 4, 8]
    )
    no_halo_max_rel_diff = no_halo_max_diff / abs(baseline)
    fragmentation_evident = no_halo_max_rel_diff > 0.05
    print(f"  max relative diff (no-halo vs baseline): {no_halo_max_rel_diff:.3%}")
    print(f"  Phase D fragmentation reproduced (>5%): {fragmentation_evident}")

    # Part (B): with halo -- partition-invariance
    print("\n-- PART B: WITH halo (IMAGINE pivot, partition-invariant) " + "-" * 12)
    with_halo_sums = {}
    for n_charts in [2, 4, 8]:
        s = k_sum_sharded_with_halo(records, n_charts=n_charts, k=k)
        with_halo_sums[n_charts] = s
        rel_diff = abs(s - baseline) / abs(baseline)
        match = abs(s - baseline) < 1e-10
        print(f"  n_charts={n_charts}: k_sum = {s:.6f}  |k_sum - baseline| = {abs(s - baseline):.3e}  match = {match}")

    with_halo_all_match_baseline = all(
        abs(with_halo_sums[n] - baseline) < 1e-10 for n in [2, 4, 8]
    )
    print(f"  all match baseline (< 1e-10): {with_halo_all_match_baseline}")

    # Part (C): cross-partition agreement with halos
    print("\n-- PART C: WITH halo, cross-partition agreement " + "-" * 21)
    vals = [with_halo_sums[n] for n in [2, 4, 8]]
    spread = max(vals) - min(vals)
    print(f"  k_sum_with_halo (n=2): {vals[0]:.6f}")
    print(f"  k_sum_with_halo (n=4): {vals[1]:.6f}")
    print(f"  k_sum_with_halo (n=8): {vals[2]:.6f}")
    print(f"  spread (max - min): {spread:.3e}")
    cross_partition_invariant = spread < 1e-10
    print(f"  cross-partition invariant (< 1e-10): {cross_partition_invariant}")

    # Summary
    print("\n" + "=" * 72)
    print("SUMMARY")
    print("=" * 72)
    flag_a = "PASS" if fragmentation_evident else "FAIL"
    flag_b = "PASS" if with_halo_all_match_baseline else "FAIL"
    flag_c = "PASS" if cross_partition_invariant else "FAIL"
    print(f"  [{flag_a}] Part A: Phase D fragmentation reproduced (test exercises right failure mode)")
    print(f"  [{flag_b}] Part B: WITH halo matches baseline exactly across all partitions")
    print(f"  [{flag_c}] Part C: WITH halo cross-partition agreement (all three n_charts same)")

    all_ok = fragmentation_evident and with_halo_all_match_baseline and cross_partition_invariant
    if all_ok:
        print("\n  T12 GREEN -- halo-as-IMAGINE solves the Phase D fragmentation.")
        print("    Without halo: per-chart k_sum varies with partition count.")
        print("    With halo: per-chart k_sum matches the direct single-shard")
        print("      k_sum EXACTLY, regardless of partition count.")
        print("  This is the gauge-equivariance proof for sharded CURVATURE.")
        print("  The Phase D RED disclosure can be removed once Rust ships the halo path.")
        return 0
    else:
        print("\n  T12 RED.")
        return 1


if __name__ == "__main__":
    sys.exit(main())
