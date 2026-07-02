#!/usr/bin/env python3
"""Class-descent experiment for obtuseness-based tetrahedron classes.

Implements the classification from Castillo et al.: a tetrahedron is
Class n if it has exactly n obtuse dihedral angles (> pi/2); a subclass
is the S4-orbit of the obtuse-edge configuration (for n=2: the two
obtuse edges are 'adjacent' if they share a vertex, 'opposite' if
disjoint; for n=3: 'star' = all at one vertex, 'path' = a 3-edge chain,
'triangle' = a face cycle).

Questions instrumented (the thesis questions, as telemetry):
  Q1: starting from a Class-n tetrahedron, does recursive longest-edge
      bisection produce terminal complexes of strictly lower class?
  Q2: does it reach Class 0 (all non-obtuse) in finitely many levels?

This is evidence, not proof: per-level class distributions with exact
dihedral arithmetic (float64), exemplar tets found by randomized search
with a fixed seed, fully reproducible.

Usage: python3 examples/tetmesh_class_descent.py
"""

import itertools
import math
import random

PAIRS = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)]
# dihedral along edge (a,b) is between the two faces NOT opposite to a or b;
# face i is the face opposite vertex i.
EDGE_FACES = {e: tuple(i for i in range(4) if i not in e) for e in PAIRS}


def sub(a, b):
    return (a[0] - b[0], a[1] - b[1], a[2] - b[2])


def dot(a, b):
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def cross(a, b):
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0])


def norm(a):
    return math.sqrt(dot(a, a))


def dihedrals(v):
    """dict edge -> internal dihedral angle, via outward face normals."""
    normals = []
    for i in range(4):
        a, b, c = (v[j] for j in range(4) if j != i)
        n = cross(sub(b, a), sub(c, a))
        ln = norm(n)
        if ln < 1e-14:
            return None  # degenerate
        n = tuple(x / ln for x in n)
        if dot(n, sub(v[i], a)) > 0:
            n = tuple(-x for x in n)
        normals.append(n)
    out = {}
    for e, (i, j) in EDGE_FACES.items():
        c = max(-1.0, min(1.0, -dot(normals[i], normals[j])))
        out[e] = math.acos(c)
    return out


def classify(v):
    """(class_n, subclass_label, obtuse_edges) per Castillo et al."""
    ds = dihedrals(v)
    if ds is None:
        return None
    obtuse = [e for e, a in ds.items() if a > math.pi / 2 + 1e-12]
    n = len(obtuse)
    if n == 0:
        return 0, "0", obtuse
    if n == 1:
        return 1, "1", obtuse
    if n == 2:
        shared = set(obtuse[0]) & set(obtuse[1])
        return 2, "2-adjacent" if shared else "2-opposite", obtuse
    if n == 3:
        counts = {}
        for e in obtuse:
            for vtx in e:
                counts[vtx] = counts.get(vtx, 0) + 1
        mx = max(counts.values())
        if mx == 3:
            return 3, "3-star", obtuse
        if len(counts) == 3:
            return 3, "3-triangle", obtuse
        return 3, "3-path", obtuse
    return n, f"{n}", obtuse  # n >= 4, if it exists


def bisect_longest(v):
    lens = {(a, b): norm(sub(v[b], v[a])) for a, b in PAIRS}
    a, b = max(lens, key=lens.get)
    m = tuple((v[a][k] + v[b][k]) / 2 for k in range(3))
    c1 = list(v); c1[a] = m
    c2 = list(v); c2[b] = m
    return tuple(c1), tuple(c2)


def descent(v0, levels):
    """Per-level class distribution under uniform longest-edge bisection."""
    current = [tuple(v0)]
    table = []
    for lvl in range(levels + 1):
        dist = {}
        worst = 0
        for t in current:
            c = classify(t)
            if c is None:
                dist["degenerate"] = dist.get("degenerate", 0) + 1
                continue
            dist[c[1]] = dist.get(c[1], 0) + 1
            worst = max(worst, c[0])
        table.append((lvl, len(current), dict(sorted(dist.items())), worst))
        if lvl < levels:
            current = [c for t in current for c in bisect_longest(t)]
    return table


def find_exemplars(seed=7, tries=2_000_000):
    """Randomized search (fixed seed) for one exemplar per subclass."""
    rng = random.Random(seed)
    want = {"1", "2-adjacent", "2-opposite", "3-star", "3-path", "3-triangle"}
    found, class_census = {}, {}
    for _ in range(tries):
        v = tuple(tuple(rng.uniform(0, 1) for _ in range(3)) for _ in range(4))
        c = classify(v)
        if c is None:
            continue
        class_census[c[0]] = class_census.get(c[0], 0) + 1
        if c[1] in want and c[1] not in found:
            found[c[1]] = v
        if want <= set(found):
            break
    return found, class_census


def main():
    print("=== census: random unit-cube tetrahedra by class (fixed seed) ===")
    found, census = find_exemplars()
    total = sum(census.values())
    for n in sorted(census):
        print(f"  Class {n}: {census[n]:8d}  ({100 * census[n] / total:.2f}%)")
    print(f"  (max observed class: {max(census)}; sampled {total:,} tets)")
    missing = {"1", "2-adjacent", "2-opposite", "3-star", "3-path",
               "3-triangle"} - set(found)
    if missing:
        print(f"  subclasses NOT found by random search: {sorted(missing)}")
        print("  (non-emptiness of these is exactly Connor's CAD question)")

    print("\n=== Q1/Q2: class descent under uniform longest-edge bisection ===")
    for label in ["1", "2-adjacent", "2-opposite", "3-star", "3-path",
                  "3-triangle"]:
        if label not in found:
            continue
        v = found[label]
        print(f"\n-- start: Class {classify(v)[0]}, subclass {label}")
        for lvl, count, dist, worst in descent(v, 6):
            short = ", ".join(f"{k}:{c}" for k, c in dist.items())
            print(f"  L{lvl}: {count:3d} tets  worst=Class {worst}  [{short}]")


if __name__ == "__main__":
    main()
