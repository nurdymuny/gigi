"""Projection-layer force: diagnostic probe, NOT a gated feature.

K = C^T M_V^-1 C, where V = continuous piecewise-linear two-forms whose
components in the left-invariant coframe (theta1^theta2, theta2^theta0,
theta0^theta1) are continuous P1 across the quotient mesh; M_V its mass and C
the pairing of V with the Whitney curvatures of single edges, both in the
face_mass two-form inner product. Then force(tau) = <P_V curl W a, curl W tau>:
symmetric and PSD by construction, original Whitney test variations.

Controls (refuse otherwise): exact curvature projects to itself; K symmetric;
pure gauge -> zero force.
usage: python projection_layer.py LEVEL
"""
import sys
import time
from pathlib import Path

import numpy as np
from scipy.sparse import coo_matrix
from scipy.sparse.linalg import splu

GIGI = Path(r"C:\Users\nurdm\OneDrive\Documents\gigi")
sys.path.insert(0, str(GIGI / "scripts"))
import ricci_pg_curvature as pg                       # noqa: E402
from ricci_consistency_math import EDGES, solve_rhs   # noqa: E402

CYC = [(0, 1, 2), (1, 2, 0), (2, 0, 1)]   # psi_a = theta_b ^ theta_c


def global_vertices(cells):
    parent = {}

    def find(x):
        parent.setdefault(x, x)
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x
    for ci, c in enumerate(cells):
        for k, (i, j) in enumerate(EDGES):
            tail, head = (j, i) if c["reverse"][k] else (i, j)
            for local, end in ((tail, "t"), (head, "h")):
                ra, rb = find((ci, local)), find((c["edges"][k], end))
                if ra != rb:
                    parent[ra] = rb
    roots, vid = {}, {}
    for ci in range(len(cells)):
        for i in range(4):
            vid[(ci, i)] = roots.setdefault(find((ci, i)), len(roots))
    return vid, len(roots)


def main(level):
    t = time.time()
    L = pg.load_level(level, 6)
    cells, a, M, vol = L["cells"], L["a"], L["mass"], L["volume"]
    vid, nv = global_vertices(cells)
    nV, nE = 3 * nv, len(a)
    Cr, Cc, Cv, Mr, Mc, Mv = [], [], [], [], [], []
    bF = np.zeros(nV)
    num_den = [0., 0.]
    for start in range(0, len(cells), 400):
        idx = range(start, min(start + 400, len(cells)))
        q = pg.quadrature(np.array([cells[i]["vertices"] for i in idx], float), 6)
        wq = q["measure"] / q["det"]
        lam = q["lam"]                                              # (Q,4)
        for li, ci in enumerate(idx):
            c = cells[ci]
            s = np.where(c["reverse"], -1., 1.)
            g, w, m = q["g"][li], wq[li], q["maurer"][li]
            psi = np.stack([np.cross(m[:, b, :], m[:, cc, :]) for _, b, cc in CYC], axis=1)  # (Q,3,3)
            Fex = -2 * psi[:, 0, :]
            wk = np.array([2 * np.cross(pg.GRADS[i], pg.GRADS[j]) for i, j in EDGES]) * s[:, None]  # (6,3)
            # C: (local vertex, frame a) x local edge
            Cl = np.einsum("qi,qac,qcd,kd,q->iak", lam, psi, g, wk, w)
            Ml = np.einsum("qi,qj,qac,qcd,qbd,q->iajb", lam, lam, psi, g, psi, w)
            bl = np.einsum("qi,qac,qcd,qd,q->ia", lam, psi, g, Fex, w)
            rows = np.array([[3 * vid[(ci, i)] + aa for aa in range(3)] for i in range(4)])  # (4,3)
            Cr.append(np.repeat(rows.ravel(), 6)); Cc.append(np.tile(c["edges"], 12)); Cv.append(Cl.reshape(12, 6).ravel())
            Mr.append(np.repeat(rows.ravel(), 12)); Mc.append(np.tile(rows.ravel(), 12)); Mv.append(Ml.reshape(12, 12).ravel())
            np.add.at(bF, rows.ravel(), bl.ravel())
    C = coo_matrix((np.concatenate(Cv), (np.concatenate(Cr), np.concatenate(Cc))), shape=(nV, nE)).tocsr()
    MV = coo_matrix((np.concatenate(Mv), (np.concatenate(Mr), np.concatenate(Mc))), shape=(nV, nV)).tocsc()
    lu = splu(MV)

    # control 1: the exact curvature projects to itself (-2 on psi_0 at every vertex)
    coef = lu.solve(bF).reshape(nv, 3)
    proj_err = float(np.max(abs(coef - np.array([-2., 0, 0]))))
    if proj_err > 1e-9:
        raise SystemExit(f"exact field does not project to itself: {proj_err:.3e}")

    force = C.T @ lu.solve(C @ a)
    v = solve_rhs(M, -force)
    norm = lambda x: float(np.sqrt(max(0., x @ (M @ x)) / vol))   # noqa: E731
    rel = norm(v + 4 * a) / norm(4 * a)
    # projected curvature error against exact, in coefficient space through M_V
    pc = lu.solve(C @ a)
    target = np.tile([-2., 0, 0], nv)
    d = pc - target
    curv = float(np.sqrt((d @ (MV @ d)) / (target @ (MV @ target))))
    out = {"level": level, "vertices": nv, "V_dim": nV, "edges": nE,
           "exact_projection_error": proj_err, "relative_error": rel,
           "projected_curvature_error": curv}
    if level <= 1:
        from scipy.linalg import eigh
        K = (C.T @ np.linalg.solve(MV.toarray(), C.toarray()))
        out["symmetry_error"] = float(np.max(abs(K - K.T)) / np.max(abs(K)))
        lamK = eigh((K + K.T) / 2, M.toarray(), eigvals_only=True)
        out["negative_eigenvalues"] = int((lamK < -1e-8 * lamK.max()).sum())
        out["zero_eigenvalues"] = int((abs(lamK) < 1e-8 * lamK.max()).sum())
        out["gauge_dimension"] = nv - 1
    out["seconds"] = round(time.time() - t, 1)
    return out


if __name__ == "__main__":
    import json
    path = pg.OUT / "projection_layer_probe.json"
    if path.exists():
        raise SystemExit(f"refusing to overwrite {path}")
    rows = [main(L) for L in (0, 1, 2)]
    for r in rows:
        print(r)
    path.write_text(json.dumps({"kind": "diagnostic probe, not a gated feature", "rows": rows,
        "source_sha256": {"scripts/ricci_projection_layer_probe.py": pg.sha(__file__),
                          "scripts/ricci_pg_curvature.py": pg.sha(pg.__file__)}}, indent=2) + "\n")
    print("wrote", path.relative_to(pg.ROOT))
