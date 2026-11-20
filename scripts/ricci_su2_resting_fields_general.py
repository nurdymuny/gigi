"""Resting fields on unevenly stretched spheres g = (t1, t2, 1): left-invariant Yang-Mills critical points and their
Morse index by spin sector. Exploration (double precision; multistart is evidence, not a completeness proof).

E(m) = 1/2 sum_{i<j} |m_i x m_j - sum_k c_ij^k m_k|^2 on the 9-dimensional space of left-invariant SU(2) fields
(symmetric criticality: critical points are Yang-Mills fields). Multistart Newton at each grid point; classification
up to constant gauge rotation by E and the singular values of the 3 x 3 matrix with rows m_i. For each non-flat
critical point: negative-eigenvalue counts of H + G G^* in sectors j = 0..JMAX (the Morse index by sector).

usage: python scripts/ricci_su2_resting_fields_general.py OUT.json [JMAX]
"""
import json
import multiprocessing as mp
import sys
from pathlib import Path

import numpy as np
import sympy as sp
from scipy.optimize import root

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_spin_sector as S        # noqa: E402
import ricci_su2_soft_mode_flow as F     # noqa: E402

G1, G2 = sp.symbols("g1 g2", positive=True)
X = sp.symbols("x0:9")
_F = {}


def build():
    s = [sp.sqrt(G1), sp.sqrt(G2), sp.Integer(1)]
    c = {}
    for (i, j, k), v in S.BR.items():
        c[(i, j, k)] = v * s[k] / (s[i] * s[j])
    m = [sp.Matrix(X[3 * i:3 * i + 3]) for i in range(3)]
    E = 0
    for (i, j) in S.PAIRS:
        f = m[i].cross(m[j]) - sum((c.get((i, j, k), 0) * m[k] for k in range(3)), sp.zeros(3, 1))
        E += (f.T * f)[0] / 2
    E = sp.expand(E)
    g = [sp.diff(E, x) for x in X]
    H = [[sp.diff(gi, x) for x in X] for gi in g]
    return (sp.lambdify((G1, G2, X), E, "numpy"), sp.lambdify((G1, G2, X), g, "numpy"),
            sp.lambdify((G1, G2, X), H, "numpy"))


def structure(g):
    s = np.sqrt(np.asarray(g, float))
    c = np.zeros((3, 3, 3))
    for (i, j, k), v in S.BR.items():
        c[i, j, k] = v * s[k] / (s[i] * s[j])
    return s, c


def _init():
    _F["E"], _F["G"], _F["H"] = build()


def critical_points(t1, t2, starts=300, seed=0):
    fE, fG, fH = _F["E"], _F["G"], _F["H"]
    rng = np.random.default_rng(seed); sols = []
    for _ in range(starts):
        x0 = rng.normal(scale=rng.choice([0.5, 1.0, 2.0, 3.0]), size=9)
        r = root(lambda x: np.array(fG(t1, t2, x), float), x0, jac=lambda x: np.array(fH(t1, t2, x), float),
                 method="hybr", tol=1e-13)
        x = r.x
        if np.linalg.norm(fG(t1, t2, x)) > 1e-9:
            continue
        E = float(fE(t1, t2, x)); sv = np.linalg.svd(x.reshape(3, 3), compute_uv=False)
        if all(abs(E - q["E"]) > 1e-6 or np.max(np.abs(sv - q["sv"])) > 1e-5 for q in sols):
            sols.append({"E": E, "sv": sv, "x": x})
    return sorted(sols, key=lambda q: q["E"])


def sector_index(x, g, jmax):
    s, c = structure(g); m = x.reshape(3, 3)
    out = []
    for j in range(jmax + 1):
        w = np.linalg.eigvalsh(F.operator(m, s, c, j))
        out.append((int((w < -1e-9).sum()), float(w[0])))
    return out


def task(arg):
    t1, t2, jmax = arg
    sols = critical_points(t1, t2)
    res = []
    for q in sols:
        row = {"E": q["E"], "singular_values": q["sv"].tolist(), "m": q["x"].tolist()}
        if q["E"] > 1e-8:
            si = sector_index(q["x"], [t1, t2, 1.0], jmax)
            row["negative_by_sector"] = [a for a, _ in si]; row["lowest_by_sector"] = [b for _, b in si]
        res.append(row)
    return {"t1": t1, "t2": t2, "critical_points": res}


def main():
    out = Path(sys.argv[1]); jmax = int(sys.argv[2]) if len(sys.argv) > 2 else 8
    vals = [0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 5.0]
    grid = [(a, b, jmax) for a in vals for b in vals if a <= b]            # t1 <= t2 (the swap is a symmetry)
    with mp.get_context("spawn").Pool(8, initializer=_init) as pool:
        rows = []
        for r in pool.imap_unordered(task, grid):
            rows.append(r)
            nf = [q for q in r["critical_points"] if q["E"] > 1e-8]
            print(f"t=({r['t1']}, {r['t2']}, 1): {len(r['critical_points'])} critical points, non-flat: " +
                  "; ".join(f"E={q['E']:.4f} sv={np.round(q['singular_values'], 3).tolist()} neg={q['negative_by_sector']}"
                            for q in nf), flush=True)
    out.write_text(json.dumps(sorted(rows, key=lambda r: (r["t1"], r["t2"])), indent=1) + "\n")


if __name__ == "__main__":
    main()
