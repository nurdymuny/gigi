"""Independent numerical cross-check of the free Wick results (docs/research/h6-wick-triangle-proof-2026-11-27.md).

Free Maxwell field in Euclidean R^4, Feynman gauge: <A_m(x) A_n(y)> = delta_mn D(x-y), D(z) = 1/(4 pi^2 z^2), so
<F_mn(x) F_rs(y)> is built from d_a d_b D(z) = (8 z_a z_b - 2 delta_ab z^2) / (4 pi^2 z^6). For O = sum_mn :F_mn F_mn:
(one colour):
  - the connected three-point function is proportional to tr(G(x-y) G(y-z) G(z-x)), which must vanish at separated points;
  - the connected four-point function at 0, e0, 2e0, 3e0 is 16 times the sum of the three distinct loop orderings of
    tr(G G G G), and must equal 676/(27 pi^8).
The method (direct propagator contraction at random points) is independent of the proof note's Hodge-exchange argument.
usage: python scripts/check_h6_wick_numeric.py OUT.json"""
import json
import math
import sys

import numpy as np

PAIRS = [(m, n) for m in range(4) for n in range(4)]


def ddD(z):
    z2 = z @ z
    return (8 * np.outer(z, z) - 2 * np.eye(4) * z2) / (4 * math.pi ** 2 * z2 ** 3)


def G(z):
    H = ddD(z)
    M = np.zeros((16, 16))
    for i, (m, n) in enumerate(PAIRS):
        for j, (r, s) in enumerate(PAIRS):
            M[i, j] = -(H[m, r] * (n == s) - H[m, s] * (n == r) - H[n, r] * (m == s) + H[n, s] * (m == r))
    return M


def main(out):
    rng = np.random.default_rng(20261127)
    tri = []
    for _ in range(20):
        x, y, z = rng.normal(size=(3, 4))
        t3 = np.trace(G(x - y) @ G(y - z) @ G(z - x))
        scale = abs(np.trace(G(x - y) @ G(y - x))) ** 1.5
        tri.append(abs(t3) / scale)
    P = [np.array([float(k), 0, 0, 0]) for k in range(4)]
    loop = lambda a, b, c, d: np.trace(G(P[a] - P[b]) @ G(P[b] - P[c]) @ G(P[c] - P[d]) @ G(P[d] - P[a]))
    four = 16 * (loop(0, 1, 2, 3) + loop(0, 1, 3, 2) + loop(0, 2, 1, 3))
    exact = 676 / (27 * math.pi ** 8)
    rec = {"max_relative_triangle": max(tri), "four_point": four, "four_point_exact": exact,
           "four_point_relative_error": abs(four - exact) / exact}
    rec = {k: float(v) for k, v in rec.items()}
    rec["pass"] = bool(rec["max_relative_triangle"] < 1e-12 and rec["four_point_relative_error"] < 1e-12)
    assert rec["pass"], rec
    open(out, "w").write(json.dumps(rec, indent=1) + "\n")
    print(json.dumps(rec, indent=1))


if __name__ == "__main__":
    main(sys.argv[1])
