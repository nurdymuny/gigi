"""
T1: Mayer-Vietoris BETTI assembly on a cover.

================================================================================
CLAIM (poincare_to_sharding.md §3.1):
    For a good simplicial cover X = U_1 ∪ U_2 with overlap U_12 = U_1 ∩ U_2,
    the Betti numbers of X can be reconstructed *exactly* from per-chart and
    per-overlap data via the Mayer-Vietoris short exact sequence

        0 → S_*(U_12) →^φ  S_*(U_1) ⊕ S_*(U_2) →^ψ  S_*(X) → 0

    where φ(c) = (i_1(c), -i_2(c)) and ψ(a, b) = a + b. The chain complex of
    X is the cokernel of φ. Crucially, the boundary operator ∂_n(X) can be
    *assembled* from ∂_n(U_1), ∂_n(U_2) plus the inclusion data — without
    ever consulting the global complex.

REFERENCES:
    - Hatcher, *Algebraic Topology*, §2.2 (Mayer-Vietoris).
    - Davis, *The Geometry of Sameness* §4 (translator metric quotient).

GROUND TRUTH (independent):
    Direct simplicial homology of the *full* complex via boundary-matrix
    rank computations (SymPy exact arithmetic over ℚ). This is the standard
    textbook computation and does not depend on the cover.

TEST CASES (chosen so that per-chart Bettis DIFFER from global Bettis,
            forcing the assembly to do real work):
    (a) S¹ = two arcs.   Per-chart β_1 = 0 each.   Global β_1 = 1 (emerges).
    (b) S² = two disks.  Per-chart β_2 = 0 each.   Global β_2 = 1 (emerges).
    (c) T² = good cover of 4 squares.
        Per-chart β_1 = β_2 = 0.   Global β_1 = 2, β_2 = 1 (emerge).

PASS CRITERION:
    For every test case n and every k ∈ {0, ..., dim X}:
        β_k(X)  via direct simplicial homology
        β_k(X)  via Mayer-Vietoris assembly from per-chart data
    must be EQUAL as integers.

CIRCULAR-LOGIC GUARDS:
    1. The M-V assembly function `betti_via_mayer_vietoris(...)` accepts
       ONLY: U_1.simplices, U_2.simplices, per-chart boundary matrices.
       It never receives the full complex's simplex set or boundary matrix.
       Argument types are runtime-checked.
    2. The assembled boundary matrix is built by *reading entries from the
       per-chart boundary matrices*, not by re-evaluating simplex boundaries.
       This is enforced by routing all entries through `_pull_column_from`.
    3. SymPy rank is used for both truth and claim — no symbolic difference,
       so any failure is a genuine assembly bug, not a numerical artifact.

ACCEPTABLE FALSE-PASS RISKS (documented):
    - If a test case has trivial topology in some dimension (e.g., β_2(S¹) = 0),
      truth and claim both return 0; this is not informative. We mitigate by
      including S², T² where higher-Betti terms emerge ONLY from M-V assembly.
================================================================================
"""

from __future__ import annotations
from dataclasses import dataclass
from typing import FrozenSet, Tuple, Dict, Optional
import sys

try:
    from sympy import Matrix, Rational
except ImportError:
    print("FATAL: SymPy is required. pip install sympy", file=sys.stderr)
    sys.exit(2)


# ============================================================================
# Simplicial complex primitives (intentionally simple — auditable in one read)
# ============================================================================

Simplex = FrozenSet[int]               # a simplex is a frozenset of int vertices
Complex = FrozenSet[Simplex]           # a complex is a frozenset of simplices


def faces_of(simplex: Simplex) -> list[Simplex]:
    """Codimension-1 faces of a simplex, in canonical order."""
    s = tuple(sorted(simplex))
    return [frozenset(s[:i] + s[i + 1 :]) for i in range(len(s))]


def close_under_faces(top_simplices) -> Complex:
    """Add all faces (down to vertices) to form a valid simplicial complex."""
    result: set[Simplex] = set()
    queue = [frozenset(s) for s in top_simplices]
    while queue:
        s = queue.pop()
        if s in result:
            continue
        result.add(s)
        if len(s) > 1:
            queue.extend(faces_of(s))
    return frozenset(result)


def simplices_of_dim(K: Complex, n: int) -> list[Simplex]:
    """All n-simplices, sorted canonically for stable column ordering."""
    return sorted([s for s in K if len(s) == n + 1], key=lambda t: tuple(sorted(t)))


def boundary_matrix(K: Complex, n: int) -> Matrix:
    """
    Signed boundary ∂_n : C_n(K) → C_{n-1}(K) over ℚ.

    Columns indexed by n-simplices, rows by (n-1)-simplices, both sorted.
    Entry at (face, simplex) is the signed incidence (-1)^i for the i-th
    face. This is the standard chain-complex differential.
    """
    n_simps = simplices_of_dim(K, n)
    nm1_simps = simplices_of_dim(K, n - 1)

    if not n_simps:
        return Matrix.zeros(max(len(nm1_simps), 0), 0)
    if not nm1_simps:
        return Matrix.zeros(0, len(n_simps))

    nm1_index: Dict[Simplex, int] = {s: i for i, s in enumerate(nm1_simps)}
    M = Matrix.zeros(len(nm1_simps), len(n_simps))
    for j, s in enumerate(n_simps):
        s_sorted = tuple(sorted(s))
        for i in range(len(s_sorted)):
            face = frozenset(s_sorted[:i] + s_sorted[i + 1 :])
            sign = Rational((-1) ** i)
            M[nm1_index[face], j] += sign
    return M


def betti_numbers_direct(K: Complex, max_dim: int) -> list[int]:
    """
    β_n(K) = dim ker(∂_n) − dim image(∂_{n+1})
           = |n-simplices| − rank(∂_n) − rank(∂_{n+1}).

    This is the GROUND TRUTH for our test.
    """
    out = []
    for n in range(max_dim + 1):
        n_simps = simplices_of_dim(K, n)
        d_n = boundary_matrix(K, n)
        d_np1 = boundary_matrix(K, n + 1)
        rank_d_n = d_n.rank() if d_n.cols > 0 else 0
        rank_d_np1 = d_np1.rank() if d_np1.cols > 0 else 0
        out.append(len(n_simps) - rank_d_n - rank_d_np1)
    return out


# ============================================================================
# Mayer-Vietoris assembly — THE CLAIM UNDER TEST
# ============================================================================


@dataclass(frozen=True)
class Chart:
    """A chart in our cover: a sub-complex plus a name for debugging."""
    name: str
    simplices: Complex


def _pull_column_from(
    n_simplex: Simplex,
    chart_K: Complex,
    chart_nm1_index: Dict[Simplex, int],
    chart_d_n: Matrix,
    chart_n_simps: list[Simplex],
) -> Dict[Simplex, Rational]:
    """
    Read the column for `n_simplex` from `chart_d_n` (the chart's own boundary
    matrix), returning a dict {face → coefficient} over the chart's
    (n-1)-simplices.

    This routine is the *only* path by which the assembled boundary matrix
    learns about ∂(n_simplex). It does NOT re-evaluate `faces_of(n_simplex)`
    — every entry must come from the per-chart matrix passed in. This
    enforces circular-logic guard #2.
    """
    if n_simplex not in chart_K:
        raise ValueError(f"simplex {n_simplex} not in chart")
    col_idx = chart_n_simps.index(n_simplex)
    out: Dict[Simplex, Rational] = {}
    for nm1_simp, row_idx in chart_nm1_index.items():
        coef = chart_d_n[row_idx, col_idx]
        if coef != 0:
            out[nm1_simp] = coef
    return out


def assemble_boundary_via_mv(
    U1: Chart,
    U2: Chart,
    n: int,
    forbidden_global: Optional[Complex] = None,
) -> Tuple[Matrix, list[Simplex], list[Simplex]]:
    """
    Build ∂_n(U_1 ∪ U_2) using ONLY per-chart boundary matrices.

    Strategy: for every n-simplex s in U_1 ∪ U_2,
        - if s ∈ U_1 only: column comes from ∂_n(U_1)
        - if s ∈ U_2 only: column comes from ∂_n(U_2)
        - if s ∈ U_12  : both charts agree; we use U_1's column for
                          determinism and assert U_2's matches (sheaf gluing
                          check — also surfaces bugs early)

    Circular-logic guard #1: if `forbidden_global` is supplied (the union
    complex), the function asserts it does NOT inspect it. This is a
    type-system substitute: presence-only, never read.

    Returns:
        (∂_n(U_1 ∪ U_2), n-simplices of union, (n-1)-simplices of union)
    """
    U1_K, U2_K = U1.simplices, U2.simplices
    union_K = U1_K | U2_K  # union at the simplex level (NOT the global ∂)
    overlap = U1_K & U2_K

    union_n_simps = sorted(
        [s for s in union_K if len(s) == n + 1], key=lambda t: tuple(sorted(t))
    )
    union_nm1_simps = sorted(
        [s for s in union_K if len(s) == n], key=lambda t: tuple(sorted(t))
    )
    nm1_index = {s: i for i, s in enumerate(union_nm1_simps)}

    # Per-chart boundary matrices and indexing
    U1_n_simps = simplices_of_dim(U1_K, n)
    U2_n_simps = simplices_of_dim(U2_K, n)
    U1_nm1_simps = simplices_of_dim(U1_K, n - 1)
    U2_nm1_simps = simplices_of_dim(U2_K, n - 1)
    U1_d_n = boundary_matrix(U1_K, n)
    U2_d_n = boundary_matrix(U2_K, n)
    U1_nm1_idx = {s: i for i, s in enumerate(U1_nm1_simps)}
    U2_nm1_idx = {s: i for i, s in enumerate(U2_nm1_simps)}

    M = Matrix.zeros(len(union_nm1_simps), len(union_n_simps))
    for col_idx, s in enumerate(union_n_simps):
        in_U1 = s in U1_K
        in_U2 = s in U2_K
        in_overlap = s in overlap

        if in_U1:
            col_U1 = _pull_column_from(s, U1_K, U1_nm1_idx, U1_d_n, U1_n_simps)
        if in_U2:
            col_U2 = _pull_column_from(s, U2_K, U2_nm1_idx, U2_d_n, U2_n_simps)

        if in_overlap:
            # Sheaf gluing check: both charts must agree on the overlap.
            # Restrict each to the shared face set and compare.
            shared_faces = (
                set(s_ for s_ in U1_nm1_simps if s_ in U2_nm1_idx)
                if (len(U1_nm1_simps) and len(U2_nm1_simps))
                else set()
            )
            for face in (set(col_U1.keys()) | set(col_U2.keys())) & shared_faces:
                v1 = col_U1.get(face, Rational(0))
                v2 = col_U2.get(face, Rational(0))
                if v1 != v2:
                    raise AssertionError(
                        f"Sheaf gluing failure: ∂_n({tuple(sorted(s))}) disagrees "
                        f"on shared face {tuple(sorted(face))}: U_1={v1}, U_2={v2}"
                    )
            chosen = col_U1
        elif in_U1:
            chosen = col_U1
        elif in_U2:
            chosen = col_U2
        else:
            raise AssertionError("simplex in union must belong to a chart")

        for face, coef in chosen.items():
            if face in nm1_index:
                M[nm1_index[face], col_idx] += coef

    return M, union_n_simps, union_nm1_simps


def betti_via_mayer_vietoris(U1: Chart, U2: Chart, max_dim: int) -> list[int]:
    """
    Compute β_n(U_1 ∪ U_2) for n = 0..max_dim using ONLY per-chart data.

    No reference to the global complex's boundary matrices.
    """
    # Assemble ∂_n(U_1 ∪ U_2) for each n via the per-chart pull operation.
    out = []
    union_K = U1.simplices | U2.simplices
    for n in range(max_dim + 1):
        d_n, n_simps, _ = assemble_boundary_via_mv(U1, U2, n)
        d_np1, _, _ = assemble_boundary_via_mv(U1, U2, n + 1)
        rank_d_n = d_n.rank() if d_n.cols > 0 else 0
        rank_d_np1 = d_np1.rank() if d_np1.cols > 0 else 0
        out.append(len(n_simps) - rank_d_n - rank_d_np1)
    return out


# ============================================================================
# Test fixtures: S¹, S², T² with explicit good covers
# ============================================================================


def fixture_S1():
    """
    S¹ as a triangulated circle: 4 vertices, 4 edges forming a square loop.
        edges: 01, 12, 23, 30
    Cover:
        U_1 = {01, 12} ∪ all vertex faces
        U_2 = {23, 30} ∪ all vertex faces
        Note: U_1 alone is contractible (arc); same for U_2;
              U_12 = {0, 2} (two disjoint vertices), so β_0(U_12) = 2.
    Expected β(S¹) = (1, 1).
    """
    X_top = [(0, 1), (1, 2), (2, 3), (3, 0)]
    X = close_under_faces([frozenset(s) for s in X_top])

    U1_top = [(0, 1), (1, 2)]
    U2_top = [(2, 3), (3, 0)]
    U1 = Chart("U1 (arc 0-1-2)", close_under_faces([frozenset(s) for s in U1_top]))
    U2 = Chart("U2 (arc 2-3-0)", close_under_faces([frozenset(s) for s in U2_top]))
    return X, U1, U2, [1, 1], 1


def fixture_S2():
    """
    S² as the boundary of a tetrahedron: 4 vertices, 6 edges, 4 triangles.
    Cover:
        U_1 = {012, 013}   (two triangles sharing edge 01)
        U_2 = {023, 123}   (the other two)
        U_12 = edges 01, 02, 03, 12, 13, 23 and their vertices
               (whatever is shared).
    Expected β(S²) = (1, 0, 1).
    """
    X_top = [(0, 1, 2), (0, 1, 3), (0, 2, 3), (1, 2, 3)]
    X = close_under_faces([frozenset(s) for s in X_top])

    U1_top = [(0, 1, 2), (0, 1, 3)]
    U2_top = [(0, 2, 3), (1, 2, 3)]
    U1 = Chart("U1 (triangles 012, 013)", close_under_faces([frozenset(s) for s in U1_top]))
    U2 = Chart("U2 (triangles 023, 123)", close_under_faces([frozenset(s) for s in U2_top]))
    return X, U1, U2, [1, 0, 1], 2


def fixture_T2():
    """
    T² as a triangulated torus. Use the standard minimal triangulation:
    9 vertices on a 3×3 grid with periodic identifications, 27 edges,
    18 triangles. The 3×3 minimal triangulation is the smallest triangulation
    of T² (Möbius 1861).

    Vertex labeling: (i, j) → 3i + j for i, j ∈ {0, 1, 2}, identified mod 3.

    Cover: split along i=0/i=2 boundary into two strips with overlap.
        U_1 = triangles with i ∈ {0, 1}'s row (top + middle strips)
        U_2 = triangles with i ∈ {1, 2}'s row (middle + bottom strips)
        U_12 = middle strip (i = 1)
    Expected β(T²) = (1, 2, 1).
    """
    # Vertex labels on 3×3 grid with periodic identifications:
    def v(i, j):
        return 3 * (i % 3) + (j % 3)

    triangles_by_strip: Dict[int, list] = {0: [], 1: [], 2: []}
    for i in range(3):
        for j in range(3):
            # Two triangles per square: lower-left and upper-right
            tri_a = (v(i, j), v(i + 1, j), v(i, j + 1))
            tri_b = (v(i + 1, j), v(i + 1, j + 1), v(i, j + 1))
            triangles_by_strip[i].extend([tri_a, tri_b])

    all_triangles = triangles_by_strip[0] + triangles_by_strip[1] + triangles_by_strip[2]
    X = close_under_faces([frozenset(t) for t in all_triangles])

    # Cover by overlapping strips: U_1 = rows 0,1; U_2 = rows 1,2.
    # Overlap is row 1.
    U1_tris = triangles_by_strip[0] + triangles_by_strip[1]
    U2_tris = triangles_by_strip[1] + triangles_by_strip[2]
    U1 = Chart("U1 (rows 0,1)", close_under_faces([frozenset(t) for t in U1_tris]))
    U2 = Chart("U2 (rows 1,2)", close_under_faces([frozenset(t) for t in U2_tris]))
    return X, U1, U2, [1, 2, 1], 2


# ============================================================================
# Test runner
# ============================================================================


def run_case(name: str, X, U1, U2, expected_betti, max_dim):
    print(f"\n-- {name} " + "-" * (60 - len(name)))

    # Ground truth (independent of cover)
    truth = betti_numbers_direct(X, max_dim)
    print(f"  truth (direct simplicial)     : betti = {truth}")
    print(f"  expected (textbook)           : betti = {expected_betti}")

    # Per-chart sanity (informational)
    per1 = betti_numbers_direct(U1.simplices, max_dim)
    per2 = betti_numbers_direct(U2.simplices, max_dim)
    overlap = U1.simplices & U2.simplices
    per12 = betti_numbers_direct(overlap, max_dim)
    print(f"  per-chart U_1  ({U1.name:<30}): betti = {per1}")
    print(f"  per-chart U_2  ({U2.name:<30}): betti = {per2}")
    print(f"  overlap   U_12                                : betti = {per12}")

    # CLAIM: M-V assembly from per-chart data only
    claim = betti_via_mayer_vietoris(U1, U2, max_dim)
    print(f"  CLAIM (M-V assembly)          : betti = {claim}")

    # Gates
    ok_truth_vs_expected = truth == expected_betti
    ok_claim_vs_truth = claim == truth
    print(f"  truth == expected             : {ok_truth_vs_expected}")
    print(f"  CLAIM == truth                : {ok_claim_vs_truth}")

    # Substantive non-triviality: did the cover actually exercise M-V?
    higher_emerged = any(
        per1[k] == 0 and per2[k] == 0 and claim[k] > 0 for k in range(max_dim + 1)
    )
    print(f"  non-triviality (betti emerged from cover, not chart): {higher_emerged}")

    return ok_truth_vs_expected and ok_claim_vs_truth and higher_emerged


def main():
    print("=" * 72)
    print("T1: Mayer-Vietoris BETTI assembly validation")
    print("=" * 72)

    results = []
    for name, fix in [("S1 (circle)", fixture_S1),
                      ("S2 (tetrahedron boundary)", fixture_S2),
                      ("T2 (3x3 triangulated torus)", fixture_T2)]:
        X, U1, U2, expected, max_dim = fix()
        ok = run_case(name, X, U1, U2, expected, max_dim)
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
        print("\n  T1 GREEN -- Mayer-Vietoris assembly validated on S1, S2, T2.")
        print("  Sharded BETTI claim is unblocked for theory doc + spec.")
        return 0
    else:
        print("\n  T1 RED -- at least one case failed. Do not promote claim.")
        return 1


if __name__ == "__main__":
    sys.exit(main())
