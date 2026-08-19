"""TDD-IDX math validation — closed forms for the index-set spectral battery.

Companion to theory/gigi/TDD-IDX_index_set_durability.md section 6. Run before
implementing against that battery: it checks the CLOSED FORMS themselves, not
the engine. If a number here disagrees with the spec, the spec is wrong.

It has now earned its keep twice.

  v1: caught the spec asserting lambda1 = 0 for disjoint cliques, when the
      operator's smallest non-zero eigenvalue there is g/(g-1) and the 0.0 the
      engine returns is a guard firing.

  v2: caught the correction ITSELF being wrong about the engine. The spec then
      claimed that deleting the guard would yield g/(g-1). It does not - that is
      the operator's eigenvalue, computed here in numpy, not what the engine's
      single-vector deflation returns. simulate_engine_solver() below models
      what spectral.rs actually does and gets 0.0 / 1.615 / 0.0 instead. The
      lesson both times: a number computed about an operator is not a number
      measured from an implementation, and saying "verified numerically" does
      not distinguish them.

    python theory/gigi/TDD-IDX_math_validation.py

Verified 2026-08-15 against numpy on main @ dd92ef8. No engine involved.
"""
import numpy as np

np.random.seed(0)
FAILURES = []


def check(label, ok, detail=""):
    if not ok:
        FAILURES.append(label)
    print(f"   {'OK  ' if ok else 'FAIL'}  {label}{('  ' + detail) if detail else ''}")


# ── graph construction, exactly as the engine would: union of cliques ────────

def graph_from_buckets(n, fields):
    """fields: list of field-partitions; each is a list of buckets of record ids.

    Mirrors field_index_graph (spectral.rs:280) — boolean adjacency, coincident
    edges collapse, because the engine accumulates into a HashSet.
    """
    A = np.zeros((n, n))
    for buckets in fields:
        for b in buckets:
            for p in b:
                for q in b:
                    if p != q:
                        A[p, q] = 1.0
    return A


def norm_lap(A):
    """Normalized Laplacian with the ZERO-ROW convention for isolated vertices
    (Def 3.10 + spectral.rs:2234). Returns (eigenvalues, kernel_dimension)."""
    d = A.sum(axis=1)
    n = A.shape[0]
    L = np.zeros((n, n))
    for u in range(n):
        if d[u] > 0:
            L[u, u] = 1.0
    for u in range(n):
        for v in range(n):
            if A[u, v] and u != v:
                L[u, v] = -A[u, v] / np.sqrt(d[u] * d[v])
    ev = np.sort(np.linalg.eigvalsh(L))
    return ev, int((np.abs(ev) < 1e-10).sum())


def smallest_nonzero(ev):
    nz = ev[ev > 1e-10]
    return float(nz[0]) if nz.size else None


def gap_to_next_distinct(ev):
    nz = ev[ev > 1e-10]
    if nz.size == 0:
        return None
    lam1 = nz[0]
    rest = nz[np.abs(nz - lam1) > 1e-9]
    return float(rest[0] - lam1) if rest.size else None


def simulate_engine_solver(A, iters=20000):
    """Model spectral.rs:717-721 — power iteration on M = D^-1/2 W D^-1/2,
    deflating the SINGLE vector u = D^1/2 * 1, then lambda1 = 1 - mu2.

    This is what the engine returns with the component guard removed. It is NOT
    the operator's smallest non-zero eigenvalue when the kernel has dimension
    greater than one.
    """
    d = A.sum(axis=1)
    Dm = np.diag(1.0 / np.sqrt(np.where(d > 0, d, 1.0)))
    M = Dm @ A @ Dm
    n = A.shape[0]
    u = np.sqrt(d)
    nu = np.linalg.norm(u)
    if nu > 0:
        u = u / nu
    v = np.random.randn(n)
    for _ in range(iters):
        v = v - (v @ u) * u
        w = M @ v
        nw = np.linalg.norm(w)
        if nw < 1e-300:
            return 0.0
        v = w / nw
    v = v - (v @ u) * u
    v = v / np.linalg.norm(v)
    return max(1.0 - v @ (M @ v), 0.0)


# ── fixtures ─────────────────────────────────────────────────────────────────

def k_n(n):
    return graph_from_buckets(n, [[list(range(n))]])


def disjoint_cliques(v, g):
    return graph_from_buckets(v * g, [[list(range(i * g, (i + 1) * g)) for i in range(v)]])


def cycle_via_two_fields(n):
    """V-3: two edge-disjoint perfect matchings whose union is C_n. n even, >= 4."""
    fa = [[i, i + 1] for i in range(0, n, 2)]
    fb = [[i, (i + 1) % n] for i in range(1, n, 2)]
    return graph_from_buckets(n, [fa, fb])


def hypercube(d):
    n = 2 ** d
    A = np.zeros((n, n))
    for i in range(n):
        for b in range(d):
            A[i, i ^ (1 << b)] = 1.0
    return A


def prism():
    """Triangular prism K3 box K2 — 3-regular, vertex-transitive, lambda1 SIMPLE.

    Hallie's counterexample to v2's claim that vertex-transitivity forces
    multiplicity. Decomposes into 3 edge-disjoint perfect matchings, so it is
    Lemma-constructible: a0a1a2 = 0,1,2 triangle; b0b1b2 = 3,4,5 triangle;
    rungs i--i+3.
    """
    M1 = [[0, 1], [3, 4], [2, 5]]
    M2 = [[1, 2], [4, 5], [0, 3]]
    M3 = [[2, 0], [5, 3], [1, 4]]
    return graph_from_buckets(6, [M1, M2, M3])


def k_mm(m):
    n = 2 * m
    A = np.zeros((n, n))
    for i in range(m):
        for j in range(m, n):
            A[i, j] = A[j, i] = 1.0
    return A


# ── V-1 ──────────────────────────────────────────────────────────────────────
print("V-1  complete graph K_n  (1 field, 1 value)          true lambda1 = n/(n-1)")
for n in (3, 4, 5, 10):
    ev, ker = norm_lap(k_n(n))
    check(f"n={n:<3} lambda1={smallest_nonzero(ev):.12f}",
          abs(smallest_nonzero(ev) - n / (n - 1)) < 1e-10 and ker == 1,
          f"closed={n / (n - 1):.12f} ker={ker}")

# ── V-2 ──────────────────────────────────────────────────────────────────────
print("\nV-2  v disjoint cliques  (1 field, v values)")
print("     engine returns 0.0 by GUARD; the operator's smallest non-zero is g/(g-1);")
print("     with the guard DELETED the engine returns neither - it is unstable")
for v, g in ((2, 3), (5, 2), (3, 4)):
    A = disjoint_cliques(v, g)
    ev, ker = norm_lap(A)
    op = smallest_nonzero(ev)
    eng = simulate_engine_solver(A)
    check(f"v={v} g={g}  ker={ker} (want {v})",
          ker == v and abs(op - g / (g - 1)) < 1e-10,
          f"operator={op:.9f} (=g/(g-1)={g / (g - 1):.9f})  guard-deleted engine={eng:.9f}")
print("     ^ the guard-deleted column is why V-2 asserts on components, not on a value")

# ── V-3 ──────────────────────────────────────────────────────────────────────
print("\nV-3  cycle C_n  (2 edge-disjoint matchings)          true lambda1 = 1-cos(2pi/n)")
for n in (4, 6, 8, 12):
    A = cycle_via_two_fields(n)
    ev, ker = norm_lap(A)
    deg = A.sum(axis=1)
    lam1 = smallest_nonzero(ev)
    exp = 1 - np.cos(2 * np.pi / n)
    mult = int((np.abs(ev - lam1) < 1e-9).sum())
    check(f"n={n:<3} lambda1={lam1:.12f}",
          abs(lam1 - exp) < 1e-10 and (deg == 2).all() and ker == 1,
          f"closed={exp:.12f} 2-regular={bool((deg == 2).all())} mult={mult} "
          f"gap={gap_to_next_distinct(ev):.12f}")

# ── V-3 hypothesis (iii): non-disjoint matchings break the Lemma ─────────────
print("\nV-3' Lemma hypothesis (iii): two fields pairing the SAME records")
n = 8
same = [[i, i + 1] for i in range(0, n, 2)]
A = graph_from_buckets(n, [same, same])
deg = A.sum(axis=1)
check("k=2 fields, coincident matchings -> 1-regular, not 2-regular",
      (deg == 1).all(),
      f"degrees={sorted(set(deg.astype(int)))} — D != kI, so L_norm = L_comb/k fails")

# ── V-4 ──────────────────────────────────────────────────────────────────────
print("\nV-4  empty index set                                 true lambda1 UNDEFINED")
for n in (5, 10):
    ev, ker = norm_lap(graph_from_buckets(n, []))
    check(f"n={n:<3} ker={ker} (want {n})",
          ker == n and smallest_nonzero(ev) is None,
          "no non-zero eigenvalue exists — L is the zero matrix")

# ── V-6 ──────────────────────────────────────────────────────────────────────
print("\nV-6  adversarial: combinatorial Laplacian instead of normalized")
A = cycle_via_two_fields(8)
ev, _ = norm_lap(A)
Lc = np.diag(A.sum(axis=1)) - A
evc = np.sort(np.linalg.eigvalsh(Lc))
check("C_8 normalized != combinatorial (V-3 catches a normalisation error)",
      abs(smallest_nonzero(ev) - smallest_nonzero(evc)) > 1e-6,
      f"norm={smallest_nonzero(ev):.9f} comb={smallest_nonzero(evc):.9f}")
A4 = k_n(4)
ev4, _ = norm_lap(A4)
print("        K_4 returns a constant from the fast path, so V-1 would stay green")
print(f"        under the same error (norm={smallest_nonzero(ev4):.9f}) — hence V-3 exists")

# ── V-7 / V-8 ────────────────────────────────────────────────────────────────
print("\nV-7  nulls are isolated vertices (K_{n-m} + m singletons)")
for n, m in ((6, 0), (6, 1), (8, 3)):
    covered = list(range(n - m))
    A = graph_from_buckets(n, [[covered]] if covered else [])
    _, ker = norm_lap(A)
    check(f"n={n} m={m}  components={ker} (want {1 + m if m or n - m else n})",
          ker == (1 + m if n - m > 0 else n),
          "m=0 must reduce exactly to V-1" if m == 0 else "")

print("\nV-8  NaN: same component shape as V-7, plus a leaked index entry each")
print("     (component arithmetic is identical; the leak is a Rust-side assertion,")
print("      see tests/tmp_nan_value_contract.rs — expected red until Value's Eq is fixed)")

# ── V-9 ──────────────────────────────────────────────────────────────────────
print("\nV-9  additional exact fixtures (Konig: k-regular bipartite = k matchings)")
for name, A, closed in (("Q_3", hypercube(3), 2 / 3),
                        ("prism", prism(), 2 / 3),
                        ("K_{3,3}", k_mm(3), 1.0),
                        ("K_{4,4}", k_mm(4), 1.0)):
    ev, ker = norm_lap(A)
    lam1 = smallest_nonzero(ev)
    deg = A.sum(axis=1)
    mult = int((np.abs(ev - lam1) < 1e-9).sum())
    check(f"{name:<8} lambda1={lam1:.12f}",
          abs(lam1 - closed) < 1e-10 and ker == 1,
          f"closed={closed:.12f} k={int(deg[0])} mult={mult}")
print("     Q_3 is multiplicity 3, not 1: Q_d eigenvalues are 2k/d with multiplicity")
print("     C(d,k), so k=1 gives C(3,1)=3. Hallie's original reason for proposing")
print("     Q_3 ('its lambda1 is simple') was wrong.")
print("     RETRACTED from v2: 'vertex-transitivity forces multiplicity' is FALSE.")
print("     The prism is 3-regular AND vertex-transitive AND has a simple lambda1.")
print("     Q_3 and prism share lambda1 = 2/3 at multiplicity 3 vs 1, so the PAIR")
print("     separates 'reports an eigenvalue' from 'reports something about the")
print("     eigenspace'. No other pair in this battery does that.")
print("     Multiplicity does not threaten the asserted VALUE either way; only an")
print("     eigenvector assertion would care, and this battery makes none.")

print("\n" + ("ALL CHECKS PASSED" if not FAILURES
             else f"FAILURES: {len(FAILURES)} -> {FAILURES}"))
raise SystemExit(1 if FAILURES else 0)
