"""
WISH on Sudoku -- does the BVP verb pick the right value in cells that
basic constraint propagation cannot resolve?

================================================================================
EXPERIMENT (Bee + Fable, 2026-06-10):
    Take a 9x9 Sudoku with a unique solution. Run basic constraint
    propagation (naked singles + hidden singles only -- no backtracking,
    no naked pairs, no advanced techniques). The cells CP can't determine
    are the "bottleneck cells" -- exactly the ones that "shouldn't be
    solvable by other means" in the basic-CP sense.

    For each bottleneck cell c with candidate set V_c, and each candidate
    v in V_c:
      - Set up a WISH problem on the JOINT probability-simplex manifold
        over all bottleneck cells.
      - Seed: current state (uniform over each cell's candidate set).
      - Target: same as seed, except cell c's logits are forced to one-hot
        at value v.
      - Metric: conformally flat with conformal factor
            exp(2*phi(v)) = 1 + lambda * sum over constrained pairs of
                            sum over common candidate values of
                            p_i[value] * p_j[value]
        i.e., the metric is "blown up" where two constrained cells
        have high probability mass on the same value -- so geodesics
        avoid those regions.

    For each cell, pick the candidate with shortest WISH arc length.
    Compare to truth.

FALSIFICATION:
    Random pick gets 1/|V_c| per cell. WISH beats that iff the constraint
    coupling pulls geodesics toward the unique consistent completion.

REUSE FROM W-MATH (wish_validation.py):
    Same GL-2 quadrature, same L-BFGS relaxation, same chord init -- but
    generalized to arbitrary dim (joint state instead of 2D toy).

NOTE ON SCOPE:
    This is NOT a W-math gate. W1-W5 prove the solver is mathematically
    correct on closed-form toy manifolds. This experiment proves (or
    refutes) the verb is OPERATIONALLY useful on a real combinatorial
    problem where the "right answer" exists but isn't deterministically
    reachable by basic CP. Different kind of validation.
================================================================================
"""

from __future__ import annotations

import math
import sys
import time
from dataclasses import dataclass
from typing import Callable

import numpy as np
from scipy.optimize import minimize


# ============================================================================
# Sudoku puzzle + solution.
#
# Picked a moderate "hard" puzzle that basic CP (naked + hidden singles only)
# does NOT fully resolve. If basic CP solves this entirely, switch to a
# harder puzzle below.
# ============================================================================


# A valid (less-structured) Sudoku solution -- the canonical Wikipedia
# Sudoku solution. Used as a stable reference so we can generate the
# puzzle by hiding cells until basic CP leaves a small bottleneck. The
# cyclic-shift Latin square earlier had columns too uniformly structured;
# CP always pinned any single missing cell.
SOLUTION_9x9 = """
5 3 4 6 7 8 9 1 2
6 7 2 1 9 5 3 4 8
1 9 8 3 4 2 5 6 7
8 5 9 7 6 1 4 2 3
4 2 6 8 5 3 7 9 1
7 1 3 9 2 4 8 5 6
9 6 1 5 3 7 2 8 4
2 8 7 4 1 9 6 3 5
3 4 5 2 8 6 1 7 9
"""


def hardcoded_unique_9x9() -> np.ndarray:
    """
    A unique-solution 9x9 puzzle (verified by full backtracking) that
    basic CP -- correctly implemented as one-placement-per-pass -- stalls
    on with 17 bottleneck cells. Generated from the Wikipedia Sudoku
    solution by hiding cells via seed=469 in the random-order generator,
    cached here so re-runs don't pay the 100+ second search.
    """
    return parse_grid("""
. 3 4 . 7 8 9 . 2
. . . 1 9 . 3 4 8
1 9 8 3 4 2 5 6 7
8 5 9 7 6 1 4 2 3
4 2 6 8 5 3 7 9 1
. . . . 2 4 8 5 6
9 . . . 3 7 2 8 4
. 8 . 4 1 9 6 3 5
. 4 . 2 8 . 1 7 9
""")


def make_puzzle_with_bottleneck(solution: np.ndarray,
                                target_bot: tuple[int, int] = (3, 8),
                                n_seeds: int = 256,
                                require_unique: bool = True,
                                ) -> np.ndarray:
    """
    Hide cells from `solution` until basic CP leaves a bottleneck of
    `target_bot[0]..target_bot[1]` cells AND (per Marcella-team review)
    the puzzle has exactly one valid completion.

    The second clause is what the prior run missed: a puzzle that's hard
    for basic CP is NOT necessarily a unique-solution puzzle. The naked-
    pair pattern admits a global symmetry that row/col/box constraints
    can't break. We now verify uniqueness via full backtracking before
    accepting a puzzle.

    Returns the candidate puzzle whose bottleneck size is closest to the
    midpoint of `target_bot`, among unique-solution snapshots found.
    """
    import random
    midpoint = (target_bot[0] + target_bot[1]) / 2
    best = None
    best_score = float("inf")
    for seed in range(n_seeds):
        rng = random.Random(seed)
        coords = [(i, j) for i in range(9) for j in range(9)]
        rng.shuffle(coords)
        puzzle = solution.copy()
        for c in coords:
            puzzle[c] = 0
            final, cand = basic_cp(puzzle)
            bot = sum(1 for cc, s in cand.items() if len(s) > 1)
            if target_bot[0] <= bot <= target_bot[1]:
                # Check uniqueness before accepting. Cheap: backtracking
                # on a near-complete grid terminates fast.
                if require_unique:
                    n_sol = count_completions(puzzle, limit=2)
                    if n_sol != 1:
                        continue  # skip ambiguous puzzles
                score = abs(bot - midpoint)
                if score < best_score:
                    best_score = score
                    best = puzzle.copy()
            if bot > target_bot[1] + 4:
                break  # Move to next seed; too many bottleneck cells
        if best is not None and best_score == 0:
            return best
    if best is not None:
        return best
    raise RuntimeError(
        f"couldn't generate puzzle with bottleneck in {target_bot} "
        f"that has a unique solution across {n_seeds} seeds"
    )


def parse_grid(s: str) -> np.ndarray:
    rows = [r for r in s.strip().split("\n") if r.strip()]
    g = np.zeros((9, 9), dtype=int)
    for i, r in enumerate(rows):
        toks = [t for t in r.split() if t]
        for j, t in enumerate(toks):
            g[i, j] = 0 if t == "." else int(t)
    return g


def print_grid(g: np.ndarray) -> None:
    for i in range(9):
        if i % 3 == 0 and i > 0:
            print("  ------+-------+------")
        row = []
        for j in range(9):
            if j % 3 == 0 and j > 0:
                row.append("|")
            row.append(str(g[i, j]) if g[i, j] != 0 else ".")
        print("  " + " ".join(row))


# ============================================================================
# Basic constraint propagation:
#   - naked single: cell with exactly one candidate -> place it
#   - hidden single: within a row/col/box, a value with exactly one possible
#                    cell -> place it there
# NO naked pairs, NO backtracking, NO advanced techniques.
# Bottleneck cells = empty cells remaining after CP saturates.
# ============================================================================


def initial_candidates(grid: np.ndarray) -> dict[tuple[int, int], set[int]]:
    cand = {}
    for i in range(9):
        for j in range(9):
            if grid[i, j] != 0:
                continue
            used = set()
            used.update(grid[i, :].tolist())
            used.update(grid[:, j].tolist())
            bi, bj = (i // 3) * 3, (j // 3) * 3
            used.update(grid[bi:bi + 3, bj:bj + 3].flatten().tolist())
            used.discard(0)
            cand[(i, j)] = set(range(1, 10)) - used
    return cand


def basic_cp(grid: np.ndarray) -> tuple[np.ndarray, dict[tuple[int, int], set[int]]]:
    """
    Naked singles + hidden singles, ONE PLACEMENT PER PASS so each
    placement's effect on neighbor candidates is observed before the next
    decision. The "batch place all naked singles from one snapshot" version
    is wrong: when placing cell A invalidates cell B's only candidate, B's
    stale snapshot still says "naked single," and B gets a wrong value.
    The bug showed up as contradictory grids (two of the same value in
    a row) and would have silently poisoned the uniqueness check too.
    """
    g = grid.copy()
    while True:
        progressed = False
        cand = initial_candidates(g)
        # Naked single: place ONE and restart.
        for (i, j), c in cand.items():
            if len(c) == 1:
                g[i, j] = next(iter(c))
                progressed = True
                break
        if progressed:
            continue
        # Hidden single: scan units, place ONE and restart.
        for unit in _units():
            empties = [(i, j) for (i, j) in unit if g[i, j] == 0]
            for v in range(1, 10):
                cells_for_v = [c for c in empties if v in cand.get(c, set())]
                if len(cells_for_v) == 1:
                    g[cells_for_v[0]] = v
                    progressed = True
                    break
            if progressed:
                break
        if not progressed:
            break
    return g, initial_candidates(g)


def _units() -> list[list[tuple[int, int]]]:
    units = []
    for i in range(9):
        units.append([(i, j) for j in range(9)])
    for j in range(9):
        units.append([(i, j) for i in range(9)])
    for bi in range(0, 9, 3):
        for bj in range(0, 9, 3):
            units.append([(i, j) for i in range(bi, bi + 3) for j in range(bj, bj + 3)])
    return units


def count_completions(grid: np.ndarray, limit: int = 2) -> int:
    """
    Full backtracking search. Returns the number of valid completions of
    `grid`, but stops counting once `limit` is reached -- for our use we
    only need to distinguish "unique" (1) from "ambiguous" (>= 2).

    Uses MRV (minimum remaining values) heuristic: at each branching point
    pick the empty cell with the fewest candidates. This makes the search
    fast on well-constrained puzzles.
    """
    g = grid.copy()
    count = [0]

    def recurse():
        if count[0] >= limit:
            return
        cand = initial_candidates(g)
        if not cand:
            # No empty cells left -- one valid completion found.
            count[0] += 1
            return
        # MRV: cell with fewest candidates.
        best = min(cand.keys(), key=lambda c: len(cand[c]))
        if not cand[best]:
            return  # dead end
        # Branch on each candidate value.
        for v in sorted(cand[best]):
            g[best] = v
            recurse()
            g[best] = 0
            if count[0] >= limit:
                return

    recurse()
    return count[0]


def cells_share_unit(a: tuple[int, int], b: tuple[int, int]) -> bool:
    if a == b:
        return False
    if a[0] == b[0] or a[1] == b[1]:
        return True
    return (a[0] // 3 == b[0] // 3) and (a[1] // 3 == b[1] // 3)


# ============================================================================
# General-dim relaxation solver (GL-2 quadrature, L-BFGS, chord init).
# Same algorithm as wish_validation.py but with the conformal factor and
# scalar curvature accepted as callables rather than tied to the toy
# manifolds. Solver core is ~40 lines.
# ============================================================================


GL2_S_MINUS = 0.5 - 1.0 / (2.0 * math.sqrt(3.0))
GL2_S_PLUS = 0.5 + 1.0 / (2.0 * math.sqrt(3.0))


@dataclass
class WishGen:
    """Outcome of one generalized WISH solve."""

    converged: bool
    arc_length: float          # tau = int sqrt(g(gamma_dot, gamma_dot)) dt
    integrated_K: float        # K = int |K_scalar(gamma(t))| dt  (numerical)
    capacity: float            # tau / K  (inf if K ~ 0)
    iterations: int
    path: np.ndarray | None    # (N+1, d)
    final_energy: float


def relaxation_analytic(
    seed: np.ndarray,
    target: np.ndarray,
    lam: float,
    offsets: list[int],
    dims: list[int],
    pairs: list[tuple[int, int, list[tuple[int, int]]]],
    n_nodes: int = 16,
    max_iter: int = 500,
    grad_tol: float = 1e-8,
) -> WishGen:
    """
    Relaxation solver using the analytic gradient. Per Marcella-team review
    note: the finite-difference scipy default makes the per-cell tau
    differential indistinguishable from numerical noise -- the signal at the
    4th-5th decimal of tau was below the gradient accuracy floor. The
    analytic gradient makes the signal credible (or proves it isn't there).
    """
    d_state = len(seed)
    N = n_nodes
    # Chord init.
    init = np.zeros((N - 1, d_state))
    for i in range(N - 1):
        s = (i + 1) / N
        init[i] = (1 - s) * seed + s * target
    z0 = init.flatten()

    def eg(z):
        return energy_and_grad_analytic(z, seed, target, lam,
                                        offsets, dims, pairs, N)

    res = minimize(
        eg, z0, method="L-BFGS-B", jac=True,
        options={"maxiter": max_iter, "gtol": grad_tol, "ftol": 1e-12},
    )
    converged = (res.status == 0)

    # Unpack path; compute arc length via GL-2 quadrature on sqrt(exp(2*phi)).
    path = np.empty((N + 1, d_state))
    path[0] = seed
    path[-1] = target
    path[1:-1] = res.x.reshape(N - 1, d_state)
    tau = 0.0
    for i in range(N):
        seg_d = path[i + 1] - path[i]
        D = float(seg_d @ seg_d)
        if D < 1e-30:
            continue
        seg_norm = math.sqrt(D)
        for s in (GL2_S_MINUS, GL2_S_PLUS):
            v_s = path[i] + s * seg_d
            ov, _ = overlap_value_and_grad(v_s, offsets, dims, pairs)
            tau += 0.5 * math.sqrt(1.0 + lam * ov) * seg_norm
    return WishGen(
        converged=converged,
        arc_length=tau,
        integrated_K=1.0,         # K_scalar deferred
        capacity=tau,             # placeholder
        iterations=int(res.nit),
        path=path,
        final_energy=float(res.fun),
    )


def relaxation_general(
    seed: np.ndarray,
    target: np.ndarray,
    exp2phi: Callable[[np.ndarray], float],
    K_scalar: Callable[[np.ndarray], float],
    n_nodes: int = 24,
    max_iter: int = 300,
    grad_tol: float = 1e-6,
) -> WishGen:
    d = len(seed)
    N = n_nodes
    # Chord init: interior nodes on the straight segment from seed to target.
    init = np.zeros((N - 1, d))
    for i in range(N - 1):
        s = (i + 1) / N
        init[i] = (1 - s) * seed + s * target
    z0 = init.flatten()

    def unpack(z: np.ndarray) -> np.ndarray:
        path = np.zeros((N + 1, d))
        path[0] = seed
        path[-1] = target
        path[1:-1] = z.reshape(N - 1, d)
        return path

    def energy(z: np.ndarray) -> float:
        path = unpack(z)
        e = 0.0
        for i in range(N):
            seg = path[i + 1] - path[i]
            s2 = float(np.dot(seg, seg))
            if s2 < 1e-30:
                continue
            # GL-2 quadrature on exp(2*phi) along the segment.
            for s in (GL2_S_MINUS, GL2_S_PLUS):
                p_s = path[i] + s * seg
                e += 0.5 * exp2phi(p_s) * s2
        return e

    res = minimize(
        energy, z0, method="L-BFGS-B",
        options={"maxiter": max_iter, "gtol": grad_tol, "ftol": 1e-10},
    )
    converged = (res.status == 0) or (res.status == 1 and res.nit >= max_iter // 2)
    # Use whatever path scipy ended with -- non-converged paths still
    # carry honest geometry information (we report verdict accordingly).
    path = unpack(res.x)

    # Arc length tau and integrated K along the path.
    tau = 0.0
    K_int = 0.0
    for i in range(N):
        seg = path[i + 1] - path[i]
        s2 = float(np.dot(seg, seg))
        if s2 < 1e-30:
            continue
        seg_norm = math.sqrt(s2)
        for s in (GL2_S_MINUS, GL2_S_PLUS):
            p_s = path[i] + s * seg
            tau += 0.5 * math.sqrt(exp2phi(p_s)) * seg_norm
            K_int += 0.5 * abs(K_scalar(p_s)) * seg_norm

    capacity = tau / K_int if K_int > 1e-12 else float("inf")
    return WishGen(
        converged=converged,
        arc_length=tau,
        integrated_K=K_int,
        capacity=capacity,
        iterations=int(res.nit),
        path=path,
        final_energy=float(res.fun),
    )


# ============================================================================
# Sudoku-as-manifold: probability simplices per bottleneck cell, joint
# conformal factor from constraint overlap.
# ============================================================================


def softmax(v: np.ndarray) -> np.ndarray:
    m = float(v.max())
    e = np.exp(v - m)
    return e / e.sum()


def overlap_value_and_grad(
    v: np.ndarray,
    offsets: list[int],
    dims: list[int],
    pairs: list[tuple[int, int, list[tuple[int, int]]]],
) -> tuple[float, np.ndarray]:
    """
    Joint overlap value AND its gradient w.r.t. v, both in one O(pairs) pass.

    overlap(v) = sum over constrained-pair (a, b) of
                 sum over (k_a, k_b) common-value-index pair of
                 p_a[k_a] * p_b[k_b]

    where p_a = softmax(v_a_slice).

    Gradient (slice a): grad_a = p_a * (n_a - <n_a, p_a>),
    where n_a[k_a] = sum over b paired with a, common (k_a, k_b), of p_b[k_b]
    ("constraint pressure" on cell a's k_a-th candidate from its partners).

    Why this form is right (sanity check kept in code, not just comment):
    softmax Jacobian gives d p_a[k] / d v_a[j] = p_a[k] * (delta(j=k) - p_a[j]).
    Plugging in and grouping by p_a[j] gives the closed form above. The
    finite-difference cross-check in cross_check_gradient() verifies this
    matches to ~1e-7.
    """
    K = len(dims)
    ps = [softmax(v[offsets[a]:offsets[a + 1]]) for a in range(K)]
    ns = [np.zeros(dims[a]) for a in range(K)]
    overlap_val = 0.0
    for (a, b, common) in pairs:
        for (k_a, k_b) in common:
            overlap_val += ps[a][k_a] * ps[b][k_b]
            ns[a][k_a] += ps[b][k_b]
            ns[b][k_b] += ps[a][k_a]
    grad = np.zeros(len(v))
    for a in range(K):
        pa = ps[a]
        na = ns[a]
        dot = float(np.dot(na, pa))
        grad[offsets[a]:offsets[a + 1]] = pa * (na - dot)
    return overlap_val, grad


def energy_and_grad_analytic(
    z: np.ndarray,
    seed: np.ndarray,
    target: np.ndarray,
    lam: float,
    offsets: list[int],
    dims: list[int],
    pairs: list[tuple[int, int, list[tuple[int, int]]]],
    n_nodes: int,
) -> tuple[float, np.ndarray]:
    """
    Total energy E(z) and analytic gradient dE/dz over the interior nodes z.

    Per-segment: E_seg(p, q) = 0.5 * [f(v_-) + f(v_+)] * D, where
       d   = q - p
       D   = d . d
       v_s = p + s * d (GL-2 quadrature node, s in {s_-, s_+})
       f(v) = 1 + lam * overlap(v)
    Derivatives:
       dE_seg/dq = 0.5 * D * (s_- * grad_f_- + s_+ * grad_f_+) + S * d
       dE_seg/dp = 0.5 * D * ((1-s_-)*grad_f_- + (1-s_+)*grad_f_+) - S * d
    Interior node z_k sums the q-contribution of segment (k-1, k) and the
    p-contribution of segment (k, k+1).
    """
    d_state = len(seed)
    N = n_nodes
    path = np.empty((N + 1, d_state))
    path[0] = seed
    path[-1] = target
    path[1:-1] = z.reshape(N - 1, d_state)

    grad_path = np.zeros((N + 1, d_state))
    total_E = 0.0

    for i in range(N):
        p = path[i]
        q = path[i + 1]
        seg_d = q - p
        D = float(seg_d @ seg_d)
        if D < 1e-30:
            continue
        v_minus = p + GL2_S_MINUS * seg_d
        v_plus = p + GL2_S_PLUS * seg_d
        ov_m, gov_m = overlap_value_and_grad(v_minus, offsets, dims, pairs)
        ov_p, gov_p = overlap_value_and_grad(v_plus, offsets, dims, pairs)
        f_m = 1.0 + lam * ov_m
        f_p = 1.0 + lam * ov_p
        S = f_m + f_p
        total_E += 0.5 * S * D
        gf_m = lam * gov_m
        gf_p = lam * gov_p
        dE_dq = 0.5 * D * (GL2_S_MINUS * gf_m + GL2_S_PLUS * gf_p) + S * seg_d
        dE_dp = (0.5 * D * ((1.0 - GL2_S_MINUS) * gf_m
                            + (1.0 - GL2_S_PLUS) * gf_p)
                 - S * seg_d)
        grad_path[i] += dE_dp
        grad_path[i + 1] += dE_dq

    grad_interior = grad_path[1:-1].flatten()
    return total_E, grad_interior


def cross_check_gradient(
    seed: np.ndarray,
    target: np.ndarray,
    lam: float,
    offsets: list[int],
    dims: list[int],
    pairs: list[tuple[int, int, list[tuple[int, int]]]],
    n_nodes: int,
    eps: float = 1e-6,
) -> tuple[float, float]:
    """
    Finite-difference cross-check of the analytic gradient.
    Returns (max_abs_diff, max_rel_diff). Should be ~1e-7 for the analytic
    code path to be trusted.
    """
    d_state = len(seed)
    # Random non-chord starting state so the gradient is nontrivial.
    rng = np.random.default_rng(2026)
    z = 0.5 * rng.standard_normal((n_nodes - 1) * d_state)
    E0, g_an = energy_and_grad_analytic(z, seed, target, lam,
                                        offsets, dims, pairs, n_nodes)
    g_fd = np.zeros_like(z)
    for k in range(len(z)):
        zp = z.copy(); zp[k] += eps
        zm = z.copy(); zm[k] -= eps
        E_p, _ = energy_and_grad_analytic(zp, seed, target, lam,
                                          offsets, dims, pairs, n_nodes)
        E_m, _ = energy_and_grad_analytic(zm, seed, target, lam,
                                          offsets, dims, pairs, n_nodes)
        g_fd[k] = (E_p - E_m) / (2 * eps)
    diff = g_an - g_fd
    max_abs = float(np.max(np.abs(diff)))
    denom = np.maximum(np.abs(g_an), np.abs(g_fd))
    max_rel = float(np.max(np.abs(diff) / np.maximum(denom, 1e-12)))
    return max_abs, max_rel


def make_sudoku_metric(
    bottleneck: list[tuple[int, int]],
    candidates: dict[tuple[int, int], list[int]],
    lam: float = 50.0,
) -> tuple[Callable, Callable, list[int]]:
    """
    Build the (exp2phi, K_scalar, dim_per_cell) for the joint state space.

    The joint state v has dimension sum(|candidates[c]| for c in bottleneck).
    Each cell's slice softmaxes to a distribution over its candidate values.

    exp(2*phi(v)) = 1 + lam * sum over constrained-pair (i,j) of
                        sum over common values of p_i[value] * p_j[value]

    Where two bottleneck cells share a unit (row/col/box) AND share at
    least one candidate value, that overlap probability inflates the
    metric -- so the geodesic to "both cells believing the same value"
    must cross high-K terrain.

    K_scalar: a numerical proxy -- the magnitude of the conformal factor's
    spatial variation, computed from a finite-difference estimate of the
    second derivative of phi. Used for the C = tau/K capacity.
    """
    K = len(bottleneck)
    dims = [len(candidates[c]) for c in bottleneck]
    offsets = [0]
    for k in dims:
        offsets.append(offsets[-1] + k)

    # Precompute constraint pairs with common-value index lists.
    pairs = []  # (a, b, [(k_a, k_b), ...])
    for a in range(K):
        for b in range(a + 1, K):
            if not cells_share_unit(bottleneck[a], bottleneck[b]):
                continue
            cand_a = candidates[bottleneck[a]]
            cand_b = candidates[bottleneck[b]]
            common_pairs = []
            for k_a, va in enumerate(cand_a):
                for k_b, vb in enumerate(cand_b):
                    if va == vb:
                        common_pairs.append((k_a, k_b))
            if common_pairs:
                pairs.append((a, b, common_pairs))

    def overlap_sum(v: np.ndarray) -> float:
        ps = []
        for a in range(K):
            ps.append(softmax(v[offsets[a]:offsets[a + 1]]))
        total = 0.0
        for (a, b, common) in pairs:
            for (k_a, k_b) in common:
                total += ps[a][k_a] * ps[b][k_b]
        return total

    def exp2phi(v: np.ndarray) -> float:
        return 1.0 + lam * overlap_sum(v)

    # K_scalar disabled for v0.1 experiment: the finite-difference Laplacian
    # is O(d) per call and dominates wall-clock at d ~ 20-30. For the
    # arc-length-based picker (the falsification test), K is not needed;
    # only tau matters. C = tau/K is a follow-up refinement.
    def K_scalar(v: np.ndarray) -> float:
        return 1.0

    return exp2phi, K_scalar, dims


# ============================================================================
# Experiment.
# ============================================================================


def run_experiment_analytic(puzzle: np.ndarray, truth: np.ndarray,
                            lams: list[float] = (10.0, 50.0, 100.0),
                            n_nodes: int = 16,
                            label: str = "9x9") -> dict:
    """
    Analytic-gradient version, with a lambda sweep so the coupling-strength
    hyperparameter has empirical support (per Marcella-team review concern #3).
    Returns a dict of {lam: (clean_correct, ties, misses)} so the caller can
    report the breakdown honestly.

    Tie-counting rule (per Marcella-team review concern #2): if the two
    candidate tau values agree to 1e-6 (~ practical solver precision), the
    pick is a tie and counted separately from "clean correct" -- the
    candidate that happens to match truth gets credit, but only as a
    tie-break, not as a signal-driven pick.
    """
    print("=" * 72)
    print(f"WISH-on-Sudoku, analytic gradient ({label})")
    print("=" * 72)
    print_grid(puzzle)
    final, cand = basic_cp(puzzle)
    bottleneck = sorted([c for c, s in cand.items() if len(s) > 1])
    print(f"\nBottleneck cells: {len(bottleneck)}")
    for c in bottleneck:
        print(f"  {c}: candidates={sorted(cand[c])}, truth={int(truth[c])}")

    candidates: dict[tuple[int, int], list[int]] = {c: sorted(cand[c]) for c in bottleneck}
    K = len(bottleneck)
    dims = [len(candidates[c]) for c in bottleneck]
    offsets = [0]
    for k in dims:
        offsets.append(offsets[-1] + k)
    d = offsets[-1]

    # Build constraint pairs once -- independent of lambda.
    pairs = []
    for a in range(K):
        for b in range(a + 1, K):
            if not cells_share_unit(bottleneck[a], bottleneck[b]):
                continue
            cand_a, cand_b = candidates[bottleneck[a]], candidates[bottleneck[b]]
            common = [(ka, kb)
                      for ka, va in enumerate(cand_a)
                      for kb, vb in enumerate(cand_b) if va == vb]
            if common:
                pairs.append((a, b, common))
    print(f"\nJoint state dim: {d}; constraint pairs: {len(pairs)}")

    # Gradient cross-check on a representative target (cell 0, candidate 0).
    seed = np.zeros(d)
    target0 = seed.copy()
    K_a = dims[0]
    for k in range(K_a):
        target0[offsets[0] + k] = 8.0 if k == 0 else -8.0
    max_abs, max_rel = cross_check_gradient(
        seed, target0, lam=50.0, offsets=offsets, dims=dims, pairs=pairs,
        n_nodes=n_nodes,
    )
    print(f"\nGradient cross-check (finite-diff vs analytic, eps=1e-6):")
    print(f"  max_abs_diff = {max_abs:.2e}, max_rel_diff = {max_rel:.2e}")
    if max_rel > 1e-4:
        raise RuntimeError(
            f"Analytic gradient disagrees with finite-difference at rel={max_rel:.2e}. "
            f"Fix the derivative before drawing conclusions."
        )
    print("  -> analytic gradient verified")

    tie_eps = 1e-6
    summary = {}
    for lam in lams:
        print(f"\n--- lambda = {lam} ---")
        clean_correct = 0
        ties_on_truth = 0
        ties_against_truth = 0
        clean_wrong = 0
        t0 = time.time()
        for a, cell in enumerate(bottleneck):
            cand_list = candidates[cell]
            K_a = dims[a]
            results = {}
            for k_idx, val in enumerate(cand_list):
                tgt = seed.copy()
                for k in range(K_a):
                    tgt[offsets[a] + k] = 8.0 if k == k_idx else -8.0
                out = relaxation_analytic(
                    seed, tgt, lam=lam,
                    offsets=offsets, dims=dims, pairs=pairs,
                    n_nodes=n_nodes,
                )
                results[val] = out
            true_val = int(truth[cell])
            # Detect tie: all pairwise tau differences below tie_eps
            taus = {v: r.arc_length for v, r in results.items()}
            tau_min = min(taus.values())
            tied = [v for v in taus if abs(taus[v] - tau_min) < tie_eps]
            if len(tied) > 1:
                pick = tied[0]  # iterator order, just to record
                if true_val in tied:
                    ties_on_truth += 1
                    tag = "TIE_OK"
                else:
                    ties_against_truth += 1
                    tag = "TIE_BAD"
            else:
                pick = min(taus, key=lambda v: taus[v])
                if pick == true_val:
                    clean_correct += 1
                    tag = "CLEAN_OK"
                else:
                    clean_wrong += 1
                    tag = "CLEAN_WRONG"
            tau_str = ", ".join(f"{v}={taus[v]:.10f}" for v in cand_list)
            diff = max(taus.values()) - min(taus.values())
            print(f"  {cell}: cands={cand_list}, taus={{{tau_str}}}, "
                  f"diff={diff:.2e}, pick={pick}, truth={true_val}: {tag}")
        elapsed = time.time() - t0
        total = len(bottleneck)
        baseline = sum(1.0 / len(candidates[c]) for c in bottleneck) / total
        clean_rate = clean_correct / total
        print(f"  [{elapsed:.2f}s] clean_correct={clean_correct}/{total}, "
              f"ties_on_truth={ties_on_truth}, ties_against_truth={ties_against_truth}, "
              f"clean_wrong={clean_wrong}")
        print(f"  clean signal rate = {clean_rate:.3f}, random baseline = {baseline:.3f}")
        summary[lam] = dict(
            clean_correct=clean_correct,
            ties_on_truth=ties_on_truth,
            ties_against_truth=ties_against_truth,
            clean_wrong=clean_wrong,
            elapsed=elapsed,
        )
    return summary


def run_experiment(puzzle: np.ndarray, truth: np.ndarray, lam: float = 50.0,
                   n_nodes: int = 16, label: str = "9x9") -> bool:
    print("=" * 72)
    print(f"WISH-on-Sudoku experiment ({label})")
    print("=" * 72)
    grid = puzzle
    print("\nPuzzle:")
    print_grid(grid)

    final, cand = basic_cp(grid)
    print("\nAfter basic CP (naked + hidden singles):")
    print_grid(final)

    bottleneck = sorted([c for c, s in cand.items() if len(s) > 1])
    print(f"\nBottleneck cells ({len(bottleneck)}): cells basic CP cannot resolve")
    for c in bottleneck:
        cs = sorted(cand[c])
        print(f"  {c}: candidates={cs}, truth={truth[c]}")
    if not bottleneck:
        print("\nNo bottleneck cells -- basic CP solved the puzzle. Pick a harder one.")
        return False

    # Convert candidate sets to ordered lists (canonical index for softmax).
    candidates: dict[tuple[int, int], list[int]] = {c: sorted(cand[c]) for c in bottleneck}

    exp2phi, K_scalar, dims = make_sudoku_metric(bottleneck, candidates, lam=lam)
    d = sum(dims)
    offsets = [0]
    for k in dims:
        offsets.append(offsets[-1] + k)

    # Seed: uniform over each cell's candidate set (zeros in logit space).
    seed = np.zeros(d)

    print(f"\nJoint state dim: {d}; constraint pairs among bottleneck: "
          f"{sum(1 for c in bottleneck for d2 in bottleneck if cells_share_unit(c, d2)) // 2}; "
          f"coupling lambda = {lam}")
    print(f"Solving {sum(dims)} per-(cell,candidate) WISH problems "
          f"at n_nodes={n_nodes}...\n")

    # Per cell, per candidate, run WISH and compare arc lengths.
    correct = 0
    total = 0
    random_baseline = 0.0
    t0 = time.time()
    for a, cell in enumerate(bottleneck):
        cand_list = candidates[cell]
        K_a = dims[a]
        results = {}
        for k_idx, val in enumerate(cand_list):
            target = seed.copy()
            # Force cell a's logits to one-hot at value v (index k_idx).
            for k in range(K_a):
                target[offsets[a] + k] = 8.0 if k == k_idx else -8.0
            out = relaxation_general(seed, target, exp2phi, K_scalar,
                                     n_nodes=n_nodes)
            results[val] = out
        # Pick by SHORTEST arc length -- the path with least metric distance,
        # i.e., least constraint violation traversed.
        pick = min(cand_list, key=lambda v: results[v].arc_length)
        true_val = int(truth[cell])
        ok = (pick == true_val)
        correct += int(ok)
        total += 1
        random_baseline += 1.0 / len(cand_list)
        tau_str = ", ".join(f"{v}: tau={results[v].arc_length:.4f} "
                            f"K={results[v].integrated_K:.2e} "
                            f"iter={results[v].iterations}"
                            for v in cand_list)
        print(f"  {cell}: cands={cand_list}, {tau_str}")
        print(f"      pick={pick}, truth={true_val}: {'OK' if ok else 'WRONG'}")

    elapsed = time.time() - t0
    print(f"\nResults ({elapsed:.2f}s):")
    print(f"  WISH correct:    {correct}/{total} = {correct/total:.3f}")
    print(f"  Random baseline: ~{random_baseline:.2f}/{total} = "
          f"{random_baseline/total:.3f}")
    print(f"  Lift: {(correct/total - random_baseline/total)*100:+.1f}pp")
    return correct > random_baseline


if __name__ == "__main__":
    truth = parse_grid(SOLUTION_9x9)
    print("Using hardcoded unique-solution 9x9 (basic CP stalls at 17 cells, "
          "verified one and only one completion exists by backtracking).")
    puzzle = hardcoded_unique_9x9()
    # Sanity: verify the puzzle is actually unique-solution and consistent
    # with truth.
    n_sol = count_completions(puzzle, limit=2)
    if n_sol != 1:
        raise RuntimeError(f"hardcoded puzzle is not unique-solution: count={n_sol}")
    print(f"Uniqueness re-verified: count_completions={n_sol}")
    summary = run_experiment_analytic(puzzle, truth, lams=[10.0, 50.0, 100.0],
                                      n_nodes=16, label="9x9 unique")
    print("\n" + "=" * 72)
    print("Summary across lambda sweep")
    print("=" * 72)
    print(f"  {'lambda':>8} {'clean_ok':>10} {'tie_ok':>8} {'tie_bad':>8} "
          f"{'clean_wrong':>12} {'elapsed':>8}")
    for lam, s in summary.items():
        print(f"  {lam:>8.1f} {s['clean_correct']:>10} {s['ties_on_truth']:>8} "
              f"{s['ties_against_truth']:>8} {s['clean_wrong']:>12} "
              f"{s['elapsed']:>7.2f}s")
    sys.exit(0)
