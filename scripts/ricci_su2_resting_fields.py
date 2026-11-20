"""Resting fields on the stretched spheres: left-invariant Yang-Mills critical points on the Berger sphere (t, 1, 1).

A left-invariant SU(2) connection A = sum_i m_i theta^i (m_i in R^3 = su(2)) has energy
  E(m) = 1/2 sum_{i<j} |m_i x m_j - sum_k c_ij^k m_k|^2,
with the structure constants of the orthonormal left-invariant frame of g = (t, 1, 1). By symmetric criticality
(Palais; the left-translation group is compact), a critical point of E on this 9-dimensional space is a Yang-Mills
field. E, its gradient and Hessian are exact polynomials (sympy). Multistart Newton finds the critical points at each
t; they are classified up to constant gauge rotations (m_i -> R m_i) by E and the singular values of the 3 x 3 matrix
with rows m_i. The Berger field (Levi-Civita) is critical only at t = 1 (the meron); the trivial field m = 0 is flat.

usage: python scripts/ricci_su2_resting_fields.py OUT.json
"""
import json
import sys
from pathlib import Path

import numpy as np
import sympy as sp
from scipy.optimize import root

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_spin_sector as S       # noqa: E402

T = sp.symbols("t", positive=True)
X = sp.symbols("x0:9")


def build():
    s = [sp.sqrt(T), sp.Integer(1), sp.Integer(1)]
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
    return (sp.lambdify((T, X), E, "numpy"), sp.lambdify((T, X), g, "numpy"), sp.lambdify((T, X), H, "numpy"))


fE, fG, fH = build()


def solve_all(t, starts=400, seed=0):
    rng = np.random.default_rng(seed)
    sols = []
    for k in range(starts):
        x0 = rng.normal(scale=rng.choice([0.5, 1.0, 2.0, 3.0]), size=9)
        r = root(lambda x: np.array(fG(t, x), float), x0, jac=lambda x: np.array(fH(t, x), float), method="hybr", tol=1e-13)
        if not r.success:
            continue
        x = r.x
        if np.linalg.norm(fG(t, x)) > 1e-9:
            continue
        E = float(fE(t, x))
        sv = np.linalg.svd(x.reshape(3, 3), compute_uv=False)
        hess = np.linalg.eigvalsh(np.array(fH(t, x), float))
        key = (round(E, 7), tuple(np.round(sv, 6)))
        if all(abs(key[0] - q["E"]) > 1e-6 or np.max(np.abs(np.array(key[1]) - q["sv"])) > 1e-5 for q in sols):
            sols.append({"E": key[0], "sv": np.array(key[1]), "x": x,
                         "hess_eigs": hess, "index_left_invariant": int((hess < -1e-8).sum()),
                         "zero_modes": int((np.abs(hess) <= 1e-8).sum())})
    sols.sort(key=lambda q: q["E"])
    return sols


def main():
    out = Path(sys.argv[1]); res = {}
    for t in (1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0, 5.0):
        sols = solve_all(t)
        res[str(t)] = [{"E": q["E"], "singular_values": q["sv"].tolist(), "m": q["x"].tolist(),
                        "index_left_invariant": q["index_left_invariant"], "zero_modes": q["zero_modes"]} for q in sols]
        print(f"t={t}: {len(sols)} critical points (up to gauge)", flush=True)
        for q in sols:
            print(f"   E={q['E']:10.6f}  sv={np.round(q['sv'], 5)}  index(left-inv)={q['index_left_invariant']}  zero={q['zero_modes']}", flush=True)
    out.write_text(json.dumps(res, indent=1) + "\n")


if __name__ == "__main__":
    main()
