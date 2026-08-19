"""TDD-IDX math validation - closed forms for the index-set spectral battery.

Companion to theory/gigi/TDD-IDX_index_set_durability.md section 6. Run before
implementing against that battery: it checks the CLOSED FORMS themselves, not
the engine. If a number here disagrees with the spec, the spec is wrong.

It has already earned its keep once - it caught the first draft asserting
lambda1 = 0 for disjoint cliques, when the operator's smallest non-zero
eigenvalue there is g/(g-1) and the 0.0 comes from a guard.

    python theory/gigi/TDD-IDX_math_validation.py

Verified 2026-08-15 against numpy on main @ dd92ef8. No engine involved.
"""
import numpy as np

def graph_from_buckets(n, fields):
    """fields: list of list-of-buckets; each bucket is a list of record ids."""
    A = np.zeros((n, n))
    for buckets in fields:
        for b in buckets:
            for p in b:
                for q in b:
                    if p != q:
                        A[p, q] = 1.0
    return A

def norm_lap_eigs(A):
    d = A.sum(axis=1)
    keep = d > 0
    if not keep.any():
        return np.zeros(A.shape[0]), int(A.shape[0])
    As, ds = A[np.ix_(keep, keep)], d[keep]
    Dm = np.diag(1.0 / np.sqrt(ds))
    L = np.eye(As.shape[0]) - Dm @ As @ Dm
    ev = np.sort(np.linalg.eigvalsh(L))
    zeros = int((ev < 1e-10).sum()) + int((~keep).sum())
    return ev, zeros

def lam1(ev):
    nz = ev[ev > 1e-10]
    return float(nz[0]) if nz.size else 0.0

print("V-1  complete graph K_n   (1 field, 1 value)      closed form n/(n-1)")
for n in (3, 4, 5, 10):
    A = graph_from_buckets(n, [[list(range(n))]])
    ev, z = norm_lap_eigs(A)
    exp = n / (n - 1)
    print(f"   n={n:>2}  measured={lam1(ev):.12f}  closed={exp:.12f}  "
          f"zeros={z}  {'OK' if abs(lam1(ev)-exp) < 1e-10 else 'MISMATCH'}")

print("\nV-2  v disjoint cliques  (1 field, v values)")
print("     engine returns 0.0 by GUARD (spectral.rs:358), NOT by measurement;")
print("     the operator's own smallest non-zero eigenvalue is g/(g-1)")
for v, g in ((2, 3), (5, 2), (3, 4)):
    n = v * g
    buckets = [list(range(i*g, (i+1)*g)) for i in range(v)]
    A = graph_from_buckets(n, [buckets])
    ev, z = norm_lap_eigs(A)
    unguarded = g / (g - 1)
    ok = z == v and abs(lam1(ev) - unguarded) < 1e-10
    print(f"   v={v} g={g} n={n:>2}  zeros={z} (want {v})  "
          f"guard-deleted={lam1(ev):.12f} (want {unguarded:.12f})  "
          f"{'OK' if ok else 'MISMATCH'}")

print("\nV-3  cycle C_n  (2 fields, all buckets size 2)    closed 1-cos(2pi/n)")
for n in (4, 6, 8, 12):
    fa = [[i, i+1] for i in range(0, n, 2)]
    fb = [[i, (i+1) % n] for i in range(1, n, 2)]
    A = graph_from_buckets(n, [fa, fb])
    deg = A.sum(axis=1)
    ev, z = norm_lap_eigs(A)
    exp = 1 - np.cos(2*np.pi/n)
    ok = abs(lam1(ev)-exp) < 1e-10 and (deg == 2).all() and z == 1
    print(f"   n={n:>2}  measured={lam1(ev):.12f}  closed={exp:.12f}  "
          f"2-regular={bool((deg==2).all())}  comps={z}  {'OK' if ok else 'MISMATCH'}")

print("\nV-4  empty index set                              lambda1=0, zeros=n")
for n in (5, 10):
    A = graph_from_buckets(n, [])
    ev, z = norm_lap_eigs(A)
    print(f"   n={n:>2}  lambda1={lam1(ev):.1f}  zeros={z}  "
          f"{'OK' if lam1(ev) == 0.0 and z == n else 'MISMATCH'}")

print("\nV-6  adversarial: combinatorial instead of normalized")
n = 8
fa = [[i, i+1] for i in range(0, n, 2)]
fb = [[i, (i+1) % n] for i in range(1, n, 2)]
A = graph_from_buckets(n, [fa, fb])
Lc = np.diag(A.sum(axis=1)) - A
evc = np.sort(np.linalg.eigvalsh(Lc))
print(f"   C_8 normalized lambda1 = {lam1(norm_lap_eigs(A)[0]):.9f}")
print(f"   C_8 combinatorial      = {lam1(evc):.9f}  <- V-3 catches this")
Ak = graph_from_buckets(4, [[list(range(4))]])
Lk = np.diag(Ak.sum(axis=1)) - Ak
print(f"   K_4 normalized         = {lam1(norm_lap_eigs(Ak)[0]):.9f}")
print(f"   K_4 combinatorial      = {lam1(np.sort(np.linalg.eigvalsh(Lk))):.9f}"
      f"   <- differs too, but K_4 returns a constant, so only V-3 is a live check")
