"""The squashed-meron family (the non-flat left-invariant Yang-Mills field on the Berger sphere (t, 1, 1)) and its
spin-sector spectra: do eigenvalues cross zero as t varies (a soft mode at a resting background)?

Continuation in t by Newton from the meron (t = 1); at each t, H + G G^* at the family's background in sectors
j = 0..JMAX: negative-eigenvalue counts (the Morse index by sector, gauge directions lifted by G G^*), the lowest
eigenvalue, and for the lowest eigenvector its top/bottom-weight content (|J_0| = j/2 fraction: 1 for the twist).
usage: python scripts/ricci_su2_meron_family_soft.py OUT.json [JMAX] [TMAX]
"""
import json
import sys
from pathlib import Path

import numpy as np
from scipy.optimize import root

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_resting_fields as R     # noqa: E402
import ricci_su2_soft_mode_flow as F     # noqa: E402
import ricci_su2_spin_sector as S        # noqa: E402


def x_of(p):
    x, y = p
    return np.array([x, 0, 0, 0, y, 0, 0, 0, y], float)


def follow(ts, x0=None):
    """The squashed meron in the ansatz m_0 = x e_0, m_1 = y e_1, m_2 = y e_2 (x, y > 0; the meron at t = 1 is
    x = y = 1); every point is checked critical in all nine directions."""
    xs, p = [], np.array([1.0, 1.0])
    for t in ts:
        r = root(lambda q: [R.fG(t, x_of(q))[0], R.fG(t, x_of(q))[4]], p, tol=1e-14)
        x = x_of(r.x)
        if np.linalg.norm(R.fG(t, x)) > 1e-9 or R.fE(t, x) < 1e-6:
            raise SystemExit(f"continuation lost at t = {t}")
        p = r.x; xs.append(x)
    return xs


def weight_content(v, j):
    """Fraction of |v|^2 on the top and bottom weights (|m| = j/2) of V_j; layout (frame i, Lie a, weight w)."""
    d = j + 1
    w = np.abs(v.reshape(3, 3, d)) ** 2
    return float((w[:, :, 0].sum() + w[:, :, -1].sum()) / w.sum()) if j > 0 else 1.0


def main():
    out = Path(sys.argv[1]); jmax = int(sys.argv[2]) if len(sys.argv) > 2 else 12
    tmax = float(sys.argv[3]) if len(sys.argv) > 3 else 10.0
    ts = np.round(np.arange(1.0, tmax + 1e-9, 0.05), 4)
    # meron at t = 1: the Levi-Civita field of the round sphere
    xs = follow(ts)
    rows = []
    for t, x in zip(ts, xs):
        s, c = F.structure(t); m = x.reshape(3, 3)
        row = {"t": float(t), "E": float(R.fE(t, x)), "sectors": {}}
        for j in range(jmax + 1):
            A = F.operator(m, s, c, j)
            w, V = np.linalg.eigh(A)
            row["sectors"][j] = {"neg": int((w < -1e-9).sum()), "lowest": float(w[0]),
                                 "lowest_top_bottom_weight": weight_content(V[:, 0], j)}
        rows.append(row)
    # zero crossings of the lowest eigenvalue, per sector
    cross = {}
    for j in range(jmax + 1):
        lo = [r["sectors"][j]["lowest"] for r in rows]
        cross[j] = [{"between": [float(ts[k]), float(ts[k + 1])], "from": lo[k], "to": lo[k + 1],
                     "twist_content": rows[k + 1]["sectors"][j]["lowest_top_bottom_weight"]}
                    for k in range(len(ts) - 1) if np.sign(lo[k]) != np.sign(lo[k + 1])]
        negs = [r["sectors"][j]["neg"] for r in rows]
        print(f"j={j:2d}: negative count t=1 -> {tmax}: {negs[0]} ... {negs[-1]} (changes at t = "
              f"{[float(ts[k + 1]) for k in range(len(ts) - 1) if negs[k] != negs[k + 1]]}); zero crossings of the "
              f"floor: {[(c['between'], round(c['twist_content'], 3)) for c in cross[j]]}", flush=True)
    out.write_text(json.dumps({"rows": rows, "floor_crossings": cross}, indent=1) + "\n")


if __name__ == "__main__":
    main()
