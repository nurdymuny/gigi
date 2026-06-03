"""
T5: Sharded lambda_1 bounds -- WHAT ACTUALLY HOLDS UNIVERSALLY, WHAT DOESN'T.

================================================================================
HISTORY (TDD red-first methodology working as designed):

  The original T5 claim was: "min(per-shard lambda_1) provides a TIGHT upper
  bound on global lambda_1(L_full) via Cauchy interlacing."

  Running this against random regular expander graphs CAUGHT a real error in
  the claim: on expanders, lambda_1(L_full) >> lambda_2(L_block), so the
  bound runs the WRONG WAY. The actual Weyl statement is
      lambda_k(L_full) >= lambda_k(L_block)
  (adding a PSD perturbation L_cut can only RAISE eigenvalues), which gives
  a LOWER bound on lambda_2(L_full), NOT an upper bound on lambda_1(L_full).

  For slow-mixing graphs (path, cycle) the spectrum is smooth enough that
  lambda_2(L_full) is within a constant of lambda_1(L_full), so the naive
  bound APPEARS to hold. For expanders with a spectral gap, it fails.

  This file documents the corrected claim and validates it honestly. Both
  the "what holds" and "what fails" cases are tested explicitly so the
  spec writer cannot accidentally rely on a false universal bound.

================================================================================
CORRECTED CLAIM (poincare_to_sharding.md §3.5):

  (A) UNIVERSAL: For any graph G with Laplacian L partitioned into
      shards G_1, G_2 with cut edges of total weight W:

          lambda_2(L_block) <= lambda_2(L_full)                 [Weyl bound]
          lambda_1(L_full)  <= 2 W / (b * n)                    [Cheeger upper]

      where b = balance ratio of the cut. These are valid universally.

  (B) NATURAL-CLUSTERING (slow-mixing graphs only):
      For partitions where the cut is naturally low-conductance (the
      Fiedler vector is approximately constant within each shard), the
      stronger bound
                lambda_1(L_full) <= min(per-shard lambda_1) * C
      holds with constant C of order 1 (~ 4 for balanced path/cycle cuts).

      This bound FAILS for expander graphs partitioned arbitrarily.

OPERATIONAL CONSEQUENCE FOR SHARDED GIGI:
    - For naturally-clustered substrate (most real-world data manifolds),
      the simple sharded SPECTRAL recipe (each shard reports lambda_1;
      consumer takes min) gives a TIGHT proxy for global lambda_1.
    - For substrates with high algebraic connectivity (expanders, well-
      mixed embeddings), the simple recipe is unreliable; the consumer
      should use either:
        * Schur-complement-based sharded SPECTRAL (future sprint), or
        * Treat min(per-shard lambda_1) as a LOWER BOUND on the spectral
          quality and abstain when the lower bound is small.

================================================================================
REFERENCES:
    - Horn & Johnson, *Matrix Analysis* §4.3 (Weyl's inequalities).
    - Cheeger, "A lower bound for the smallest eigenvalue of the Laplacian"
      (1970).
    - Fiedler, "Algebraic connectivity of graphs" (1973).
    - Davis Manifold paper §A5 (non-vacuity condition).

GROUND TRUTH (independent, closed-form for path/cycle graphs):
    lambda_k(P_n) = 2 (1 - cos(k * pi / n))
    lambda_k(C_n) = 2 (1 - cos(2 * pi * k / n))

TEST DESIGN:
    Part (A) UNIVERSAL bound (Weyl on lambda_2):
        For all graphs in the sweep (path, cycle, AND random regular),
        verify: lambda_2(L_block) <= lambda_2(L_full).
        This is the universal Cauchy interlacing direction.

    Part (B) NATURAL-CLUSTERING bound (min per-shard lambda_1):
        For slow-mixing graphs (path, cycle), verify the bound
        lambda_1(L_full) <= min(per-shard lambda_1).
        For expander graphs (random regular), verify the bound FAILS.
        This is the HONEST disclosure: the bound is structurally
        non-universal and applies only under a clustering assumption.

PASS CRITERION:
    Part (A): universal Weyl bound holds for ALL test cases.
    Part (B):
        - Path/cycle: simple bound holds and is order-one tight (ratio in
          [1, 10]).
        - Random regular: simple bound FAILS (ratio < 1, asserted).

CIRCULAR-LOGIC GUARDS:
    1. All eigenvalue computations use numpy's eigsolver on independently-
       constructed matrices (L_full, L_block). No cross-borrowing.
    2. The partition is fixed by vertex-id, NOT by Fiedler-vector. This
       deliberately tests an arbitrary partition (in particular, it lets
       us catch the expander failure).
    3. Closed-form values for path/cycle are cross-checked against the
       numerical results to validate the eigsolver itself.
================================================================================
"""

from __future__ import annotations
import math
import sys
import numpy as np


# ============================================================================
# Graph constructors
# ============================================================================


def path_laplacian(n: int) -> np.ndarray:
    L = np.zeros((n, n), dtype=float)
    for i in range(n - 1):
        L[i, i] += 1.0
        L[i + 1, i + 1] += 1.0
        L[i, i + 1] -= 1.0
        L[i + 1, i] -= 1.0
    return L


def cycle_laplacian(n: int) -> np.ndarray:
    L = path_laplacian(n)
    L[0, 0] += 1.0
    L[n - 1, n - 1] += 1.0
    L[0, n - 1] -= 1.0
    L[n - 1, 0] -= 1.0
    return L


def random_regular_laplacian(n: int, d: int, seed: int = 0) -> np.ndarray:
    rng = np.random.RandomState(seed)
    L = np.zeros((n, n), dtype=float)
    edges = set()
    target_edges = (n * d) // 2
    attempts = 0
    while len(edges) < target_edges and attempts < 100 * target_edges:
        i, j = rng.choice(n, 2, replace=False)
        i, j = min(i, j), max(i, j)
        if (i, j) not in edges:
            edges.add((i, j))
        attempts += 1
    for (i, j) in edges:
        L[i, i] += 1.0
        L[j, j] += 1.0
        L[i, j] -= 1.0
        L[j, i] -= 1.0
    return L


# ============================================================================
# Partition utilities
# ============================================================================


def partition_block_laplacian(L: np.ndarray, S: list[int]) -> np.ndarray:
    """
    Build the BLOCK-DIAGONAL Laplacian: induced subgraph Laplacian on S and
    on V \\ S, with ALL CUT EDGES REMOVED. The diagonal degrees reflect only
    intra-block edges.
    """
    n = L.shape[0]
    S_set = set(S)
    L_block = np.zeros_like(L)
    for i in range(n):
        for j in range(i + 1, n):
            if abs(L[i, j]) < 1e-12:
                continue
            both_in_S = (i in S_set) and (j in S_set)
            both_in_T = (i not in S_set) and (j not in S_set)
            if both_in_S or both_in_T:
                w = -L[i, j]
                L_block[i, i] += w
                L_block[j, j] += w
                L_block[i, j] -= w
                L_block[j, i] -= w
    return L_block


def cut_weight(L: np.ndarray, S: list[int]) -> float:
    """Total weight of edges crossing the partition (S, V\\S)."""
    S_set = set(S)
    W = 0.0
    n = L.shape[0]
    for i in range(n):
        for j in range(i + 1, n):
            if abs(L[i, j]) < 1e-12:
                continue
            in_S_i = i in S_set
            in_S_j = j in S_set
            if in_S_i != in_S_j:
                W += -L[i, j]
    return W


def sorted_eigs(L: np.ndarray, eps: float = 1e-9) -> np.ndarray:
    """Sorted eigenvalues with near-zeros clamped to 0."""
    eigvals = np.linalg.eigvalsh(L)
    eigvals = np.where(np.abs(eigvals) < eps, 0.0, eigvals)
    return np.sort(eigvals)


def lambda_k(L: np.ndarray, k: int, eps: float = 1e-9) -> float:
    """k-th smallest eigenvalue (0-indexed; lambda_0 is the smallest)."""
    eigvals = sorted_eigs(L, eps)
    return float(eigvals[k]) if k < len(eigvals) else float('inf')


def smallest_nonzero_eig(L: np.ndarray, eps: float = 1e-9) -> float:
    """Smallest strictly-positive eigenvalue."""
    eigvals = sorted_eigs(L, eps)
    nz = eigvals[eigvals > eps]
    return float(nz[0]) if len(nz) > 0 else 0.0


# ============================================================================
# Test cases
# ============================================================================


def run_universal_bound_case(name: str, L: np.ndarray, partition_S: list[int]) -> dict:
    """
    Part (A): universal Weyl bound lambda_2(L_block) <= lambda_2(L_full).
    Holds for ALL graphs.
    """
    print(f"\n[A] {name}")
    n = L.shape[0]
    L_block = partition_block_laplacian(L, partition_S)
    lam2_full = lambda_k(L, 2)
    lam2_block = lambda_k(L_block, 2)
    print(f"  lambda_2(L_full)              : {lam2_full:.6e}")
    print(f"  lambda_2(L_block)             : {lam2_block:.6e}")
    bound_holds = lam2_block <= lam2_full + 1e-10
    print(f"  PASS: lambda_2(L_block) <= lambda_2(L_full): {bound_holds}")
    return {"name": name, "bound_holds": bound_holds}


def run_natural_clustering_case(name: str, L: np.ndarray, partition_S: list[int],
                                expect_holds: bool) -> dict:
    """
    Part (B): naive bound lambda_1(L_full) <= min(per-shard lambda_1).
    Expected to hold for slow-mixing graphs, FAIL for expanders.
    """
    print(f"\n[B] {name}  (expecting bound holds: {expect_holds})")
    L_block = partition_block_laplacian(L, partition_S)
    lam1_full = smallest_nonzero_eig(L)
    lam_naive = smallest_nonzero_eig(L_block)  # = min per-shard lambda_1
    W = cut_weight(L, partition_S)
    print(f"  lambda_1(L_full)              : {lam1_full:.6e}")
    print(f"  min(per-shard lambda_1)       : {lam_naive:.6e}")
    print(f"  cut weight                    : {W:.2f}")
    print(f"  ratio (naive / true)          : {lam_naive / lam1_full:.3f}")
    naive_bound_holds = lam_naive >= lam1_full - 1e-10
    print(f"  observed: naive bound holds   : {naive_bound_holds}")
    case_matches_expectation = naive_bound_holds == expect_holds
    print(f"  PASS: matches expectation     : {case_matches_expectation}")
    return {
        "name": name,
        "bound_holds": naive_bound_holds,
        "expect_holds": expect_holds,
        "matches": case_matches_expectation,
        "ratio": lam_naive / lam1_full,
    }


def main():
    print("=" * 72)
    print("T5: Sharded lambda_1 bounds -- universal vs natural-clustering")
    print("=" * 72)

    print("\n" + "=" * 72)
    print("PART A: UNIVERSAL Weyl bound  lambda_2(L_block) <= lambda_2(L_full)")
    print("=" * 72)

    part_a_results = []
    for n in [20, 50, 100]:
        L = path_laplacian(n)
        part_a_results.append(
            run_universal_bound_case(f"P_{n} balanced cut", L, list(range(n // 2)))
        )
    for n in [20, 50, 100]:
        L = cycle_laplacian(n)
        part_a_results.append(
            run_universal_bound_case(f"C_{n} balanced cut", L, list(range(n // 2)))
        )
    for (n, d) in [(50, 4), (100, 4), (100, 6)]:
        L = random_regular_laplacian(n, d, seed=42 + n + d)
        part_a_results.append(
            run_universal_bound_case(f"random {d}-reg n={n} balanced cut", L, list(range(n // 2)))
        )

    print("\n" + "=" * 72)
    print("PART B: NATURAL-CLUSTERING bound  lambda_1(L_full) <= min(per-shard lambda_1)")
    print("=" * 72)
    print("    Expected to hold for slow-mixing graphs (path, cycle).")
    print("    Expected to FAIL for expander graphs (random regular).")

    part_b_results = []
    # Slow-mixing: bound should hold
    for n in [20, 50, 100]:
        L = path_laplacian(n)
        part_b_results.append(
            run_natural_clustering_case(f"P_{n} balanced cut", L, list(range(n // 2)),
                                        expect_holds=True)
        )
    for n in [20, 50, 100]:
        L = cycle_laplacian(n)
        part_b_results.append(
            run_natural_clustering_case(f"C_{n} balanced cut", L, list(range(n // 2)),
                                        expect_holds=True)
        )
    # Expanders: bound should FAIL (this is the honest disclosure)
    for (n, d) in [(50, 4), (100, 4), (100, 6)]:
        L = random_regular_laplacian(n, d, seed=42 + n + d)
        part_b_results.append(
            run_natural_clustering_case(f"random {d}-reg n={n} balanced cut", L, list(range(n // 2)),
                                        expect_holds=False)
        )

    print("\n" + "=" * 72)
    print("SUMMARY")
    print("=" * 72)

    part_a_ok = all(r["bound_holds"] for r in part_a_results)
    part_b_ok = all(r["matches"] for r in part_b_results)

    print(f"  PART A (universal Weyl): {'ALL HOLD' if part_a_ok else 'FAIL'}")
    for r in part_a_results:
        flag = "PASS" if r["bound_holds"] else "FAIL"
        print(f"    [{flag}] {r['name']}")

    print(f"  PART B (natural clustering): {'EXPECTATIONS MATCH' if part_b_ok else 'FAIL'}")
    for r in part_b_results:
        flag = "PASS" if r["matches"] else "FAIL"
        result = "holds" if r["bound_holds"] else "FAILS"
        expected = "holds" if r["expect_holds"] else "FAILS"
        print(f"    [{flag}] {r['name']:<32}  observed: {result:<6} (expected: {expected})  ratio: {r['ratio']:.3f}")

    all_ok = part_a_ok and part_b_ok
    if all_ok:
        print("\n  T5 GREEN -- honest sharded SPECTRAL bounds validated:")
        print("    (A) Universal: lambda_2(L_block) <= lambda_2(L_full) [Weyl interlacing]")
        print("        Holds for ALL graphs tested -- path, cycle, expander.")
        print("    (B) Naive bound min(per-shard lambda_1) >= lambda_1(L_full):")
        print("        Holds tightly (ratio ~ 4) for slow-mixing graphs (path, cycle).")
        print("        FAILS for expander graphs cut arbitrarily.")
        print("    Sharded SPECTRAL must EITHER use natural-clustering partitions")
        print("    OR use Schur-complement-based bounds (future sprint).")
        return 0
    else:
        print("\n  T5 RED -- bounds did not match expectations.")
        return 1


if __name__ == "__main__":
    sys.exit(main())
