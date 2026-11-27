"""Exact free R4 covariances and connected quadratic Wick contractions.

No fitted data or spectral solver. Matrix entries use independent two-form
components (01, 02, 03, 12, 13, 23), and retain canonical pi factors.
"""
from functools import lru_cache
from itertools import combinations, product

import sympy as s

PAIRS = tuple(combinations(range(4), 2))


@lru_cache(maxsize=1)
def _differentiated_covariance():
    x = s.symbols('r0:4', real=True)
    G = 1/(4*s.pi**2*sum(v*v for v in x))
    # The derivative at the second insertion reverses the displacement.
    D = s.Matrix(4, 4, lambda a, b: -s.diff(G, x[a], x[b]))
    C = s.Matrix(6, 6, lambda a, b:
                 s.KroneckerDelta(PAIRS[a][1], PAIRS[b][1])*D[PAIRS[a][0], PAIRS[b][0]]
                 -s.KroneckerDelta(PAIRS[a][1], PAIRS[b][0])*D[PAIRS[a][0], PAIRS[b][1]]
                 -s.KroneckerDelta(PAIRS[a][0], PAIRS[b][1])*D[PAIRS[a][1], PAIRS[b][0]]
                 +s.KroneckerDelta(PAIRS[a][0], PAIRS[b][0])*D[PAIRS[a][1], PAIRS[b][1]])
    return x, s.ImmutableMatrix(C.applyfunc(s.cancel))


def field_covariance(displacement):
    r = tuple(s.sympify(v) for v in displacement)
    if len(r) != 4 or any(v.has(s.Float) or v.is_real is not True or v.is_finite is False for v in r):
        raise ValueError('Four exact real displacement components are required')
    if s.simplify(sum(v*v for v in r)) == 0:
        raise ValueError('Coincident points require a distributional prescription, not this evaluator')
    x, C = _differentiated_covariance()
    return s.Matrix(C.subs(dict(zip(x, r)), simultaneous=True))


def hodge_star():
    return s.Matrix(6, 6, lambda a, b: s.LeviCivita(*PAIRS[a], *PAIRS[b]))


@lru_cache(maxsize=3)
def connected_pairings(n):
    if n not in (2, 3, 4):
        raise ValueError('Only two, three or four quadratic insertions are supported')

    def matchings(slots):
        if not slots:
            yield ()
            return
        a = slots[0]
        for k in range(1, len(slots)):
            b = slots[k]
            if a//2 == b//2:
                continue
            rest = slots[1:k]+slots[k+1:]
            for tail in matchings(rest):
                yield ((a, b),)+tail

    result = []
    for pairing in matchings(tuple(range(2*n))):
        adjacency = [set() for _ in range(n)]
        for a, b in pairing:
            adjacency[a//2].add(b//2)
            adjacency[b//2].add(a//2)
        visited, pending = set(), [0]
        while pending:
            vertex = pending.pop()
            if vertex not in visited:
                visited.add(vertex)
                pending.extend(adjacency[vertex]-visited)
        if len(visited) == n:
            result.append(pairing)
    return tuple(result)


def connected_quadratic(blocks, scale=2, colours=1):
    if not blocks or not isinstance(colours, int) or isinstance(colours, bool) or colours < 1:
        raise ValueError('Supply covariance blocks and a positive integer colour count')
    if any(not isinstance(key, tuple) or len(key) != 2 or
           not all(isinstance(v, int) and v >= 0 for v in key) or key[0] >= key[1] for key in blocks):
        raise ValueError('Blocks are keyed by increasing insertion indices')
    n = max(j for _, j in blocks)+1
    pairings = connected_pairings(n)
    if set(blocks) != set(combinations(range(n), 2)):
        raise ValueError('All cross-covariance blocks are required')
    matrices = {key: s.Matrix(value) for key, value in blocks.items()}
    dimension = next(iter(matrices.values())).rows
    if dimension < 1 or any(C.shape != (dimension, dimension) for C in matrices.values()):
        raise ValueError('Covariance blocks must have the same nonempty square shape')
    scale = s.sympify(scale)
    if any(v.has(s.Float) or v.is_real is not True or v.is_finite is False
           for v in (scale, *(entry for C in matrices.values() for entry in C))):
        raise ValueError('Use exact real finite entries and observable scale')

    answer = s.Integer(0)
    for components in product(range(dimension), repeat=n):
        for pairing in pairings:
            term = s.Integer(1)
            for a, b in pairing:
                i, j = a//2, b//2
                term *= matrices[i, j][components[i], components[j]]
                if term == 0:
                    break
            answer += term
    # A connected contraction forces a single shared colour around the cycle.
    return s.Integer(0)
