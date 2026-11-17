"""Independent exact Q(sqrt(5)) oracle; never used by GIGI's query evaluator.

Pairs (a,b) denote a+b*sqrt(5), with rational a,b. This validates the
mathematically specified generators, not exact finiteness of rounded f64 input.
"""
from collections import Counter
from fractions import Fraction as F


def r(a=0, b=0):
    return (F(a), F(b))


def add(x, y):
    return (x[0] + y[0], x[1] + y[1])


def neg(x):
    return (-x[0], -x[1])


def mul(x, y):
    return (x[0]*y[0] + 5*x[1]*y[1], x[0]*y[1] + x[1]*y[0])


ID = (r(1), r(), r(), r())
S = (r(F(1, 2)),) * 4
T = (r(F(1, 4), F(1, 4)), r(F(1, 2)), r(-F(1, 4), F(1, 4)), r())


def product(a, b):
    # Independent coefficient formula, with GIGI's documented minus-cross convention.
    out = [add(mul(a[0], b[0]), neg(sum_ring(mul(a[i], b[i]) for i in (1, 2, 3))))]
    for i, j, k in ((1, 2, 3), (2, 3, 1), (3, 1, 2)):
        out.append(sum_ring((mul(a[0], b[i]), mul(a[i], b[0]),
                             neg(mul(a[j], b[k])), mul(a[k], b[j]))))
    return tuple(out)


def sum_ring(xs):
    out = r()
    for x in xs:
        out = add(out, x)
    return out


def inverse(q):
    return (q[0],) + tuple(neg(x) for x in q[1:])


def exact_closure(generators):
    elements = [ID]
    seen = {ID}
    for a in elements:
        for b in generators:
            c = product(a, b)
            if c not in seen:
                if len(elements) >= 256:
                    raise AssertionError("oracle exceeded its fixed bound")
                seen.add(c)
                elements.append(c)
    return elements


def oracle():
    elements = exact_closure([S, T])
    assert all(sum_ring(mul(x, x) for x in q) == r(1) for q in elements)
    index = {q: i for i, q in enumerate(elements)}
    table = [[index[product(a, b)] for b in elements] for a in elements]
    n = len(elements)
    inv = [index[inverse(q)] for q in elements]
    histogram = Counter()
    for a in range(n):
        p = 0
        for order in range(1, n+1):
            p = table[p][a]
            if p == 0:
                histogram[order] += 1
                break
        else:
            raise AssertionError("oracle element lacks finite order")
    comms = {table[table[table[a][b]][inv[a]]][inv[b]]
             for a in range(n) for b in range(n)}
    derived = {0}
    frontier = [0]
    for a in frontier:
        for b in sorted(comms):
            c = table[a][b]
            if c not in derived:
                derived.add(c)
                frontier.append(c)
    assert n == 120
    assert dict(sorted(histogram.items())) == {1: 1, 2: 1, 3: 20, 4: 30, 5: 24, 6: 20, 10: 24}
    assert len(derived) == 120
    assert product(product(S, S), S) == tuple(neg(x) for x in ID)
    t5 = ID
    for _ in range(5):
        t5 = product(t5, T)
    assert t5 == tuple(neg(x) for x in ID)
    assert product(product(S, T), product(S, T)) == tuple(neg(x) for x in ID)
    return {"arithmetic": "exact rational coefficients in Q(sqrt(5))",
            "group_order": n, "element_orders": dict(sorted(histogram.items())),
            "commutator_subgroup_order": len(derived), "is_perfect": True,
            "scope": "exact algebraic generators; not an f64 finiteness certificate"}


if __name__ == "__main__":
    import json
    print(json.dumps(oracle(), indent=2))
