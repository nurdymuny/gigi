"""The exact left-invariant sector of the linearised SU(2) Yang-Mills operator at the Berger field,
and its comparison with the measured lattice spectra (S2b n = 1, 2; S2d n = 3). Research script.

The Berger field is the Levi-Civita (spin) connection of the left-invariant metric g = (3, 1, 1) on S^3,
brackets [e_i, e_j] = 2 e_k (as e2e/probes/ricci_oracle.py). Everything is left-invariant, so the Hessian of
E = 1/2 |F|^2 preserves the 9-dimensional space of left-invariant su(2)-valued 1-forms. Its eigenvalues against
the L2 mass are computed exactly (sympy). --abelian drops the commutator (geometry only).

usage: python scripts/ricci_su2_left_invariant_sector.py [OUT.json]
"""
import json
import math
import sys
from pathlib import Path

import numpy as np
import sympy as sp

REPO = Path(__file__).resolve().parents[1]
BR = {}
for i, j, k in [(0, 1, 2), (1, 2, 0), (2, 0, 1)]:
    BR[(i, j, k)] = 2
    BR[(j, i, k)] = -2


def so3(v):
    a, b, c = v
    return sp.Matrix([[0, -c, b], [c, 0, -a], [-b, a, 0]])


def ip(X, Y):
    return -sp.Rational(1, 2) * (X * Y).trace()


def sector(g, abelian=False):
    g = [sp.nsimplify(x) for x in g]
    s = [sp.sqrt(x) for x in g]
    c = lambda i, j, k: BR.get((i, j, k), 0) * s[k] / (s[i] * s[j])          # noqa: E731  orthonormal frame
    Gam = lambda i, j, k: sp.Rational(1, 2) * (c(i, j, k) - c(j, k, i) + c(k, i, j))   # noqa: E731
    M = [sp.Matrix(3, 3, lambda k, j: Gam(i, j, k)) for i in range(3)]
    b = sp.symbols("b0:9")
    B = [so3(b[3 * i:3 * i + 3]) for i in range(3)]
    A = [M[i] + B[i] for i in range(3)]
    E = 0
    for i, j in [(0, 1), (1, 2), (0, 2)]:
        F = (sp.zeros(3) if abelian else A[i] * A[j] - A[j] * A[i]) - sum((c(i, j, k) * A[k] for k in range(3)), sp.zeros(3))
        E += ip(F, F) / 2
    H = sp.hessian(E, b).subs({x: 0 for x in b})
    mass = sp.hessian(sum(ip(Bi, Bi) for Bi in B) / 2, b)
    ev = (mass.inv() * H).eigenvals()
    grad = [float(sp.diff(E, x).subs({y: 0 for y in b})) for x in b]
    return sorted([(str(sp.nsimplify(sp.simplify(k))), float(k), int(v)) for k, v in ev.items()], key=lambda r: r[1]), grad


def measured():
    out = {}
    src = {1: REPO / "research/su2-pg-stability-20261117/L0_{t}.eigs.npy",
           2: REPO / "research/su2-pg-stability-20261117/L1_{t}.eigs.npy",
           3: REPO / "research/su2-spectrum-n3-20261119/L2_{t}.eigs.npy"}
    for n, p in src.items():
        for t in ("native", "pg"):
            lam = np.load(str(p).format(t=t))
            out[f"n{n}_{t}"] = lam
    return out


def main():
    full, grad = sector([3, 1, 1])
    abel, _ = sector([3, 1, 1], abelian=True)
    rnd, grad_r = sector([1, 1, 1])
    lam = measured()
    rows = []
    for expr, val, mult in full:
        row = {"exact": expr, "value": val, "multiplicity": mult}
        for key, l in lam.items():
            near = l[np.argsort(np.abs(l - val))][:mult]
            row[key] = [[float(z.real), float(z.imag)] for z in near]
        rows.append(row)
    res = {"berger_full": full, "berger_abelianised": abel, "round_full": rnd,
           "berger_gradient_at_background": grad, "round_gradient_at_background": grad_r,
           "comparison": rows,
           "kind": "exact sector (sympy) + comparison with committed receipts (research script, not an engine run)"}
    for r in rows:
        print(f"{r['exact']:>22s} = {r['value']:9.4f} x{r['multiplicity']}  " +
              "  ".join(f"{k}:{r[k][0][0]:.4f}" for k in sorted(k for k in r if k.startswith('n'))))
    if len(sys.argv) > 1:
        Path(sys.argv[1]).write_text(json.dumps(res, indent=1) + "\n")


if __name__ == "__main__":
    main()
