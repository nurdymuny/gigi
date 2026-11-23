"""External review R1: how far the SU(2) two-ring recovery is from holding a cell's own face readings exactly.

A closed linear 2-form has four own-face fluxes summing to zero (rank-3 flux map); registered non-Abelian face
readings do not (the Bianchi identity is multiplicative). The recovery applies the Abelian equality-constrained fit
componentwise, so on SU(2) data the own faces are matched only up to that incompatibility. For every cell, with the
reading the force uses (flux='defect', the vector part of the face holonomy) and the logarithm (flux='log'):
  rel  = ||A C - Y_own||_F / ||Y_own||_F      (A: own-face flux rows; C: recovered coefficients; Y_own: own readings)
  floor = ||u u^T Y_own||_F / ||Y_own||_F, u the left null vector of A: the part of the readings no closed linear
          field can match (the non-Abelian Bianchi residual); rel >= floor for any closed fit
usage: python scripts/ricci_su2_ownface_residual.py OUT.json [LEVELS]   (LEVELS default 0,1,2 = n 1,2,3)
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import ricci_pg_curvature as pg            # noqa: E402
import ricci_pg_tworing as tw             # noqa: E402
import ricci_su2_recovery as su           # noqa: E402
import ricci_su2_pg_force as F            # noqa: E402


def own_rows(cells, faces, ci, rings=2):
    nb = tw.neighbourhood_rings(cells, faces, ci, rings)
    to_orth, _, _ = pg.chart(nb[ci])
    O = to_orth(nb[ci])
    return np.array([pg.flux_row(O[list(f)]) for f in pg.FACES])


def main():
    outp = Path(sys.argv[1]); levels = [int(x) for x in (sys.argv[2] if len(sys.argv) > 2 else "0,1,2").split(",")]
    res = {}
    for level in levels:
        t0 = time.time()
        payload = json.loads((F.BERGER / f"level{level}_berger_q6.input.json").read_text())
        raw = json.loads((F.BERGER / f"level{level}_berger_q6.output.json").read_text())
        links = np.asarray(raw["links"], float); cells = payload["cells"]
        faces = pg.face_index(cells); vid = su.vertex_ids(cells); zeros = np.zeros(len(links))
        out = {}
        for flux in ("defect", "log"):
            rel, sm = [], []
            for ci in range(len(cells)):
                fit = tw.fit_cell_rings(cells, zeros, faces, ci, rings=2)
                rec = su.recover_cell(cells, faces, vid, links, ci, rings=2, fit=fit, flux=flux)
                C = rec["coefficients"]                       # 3 x 11
                A = own_rows(cells, faces, ci)                # 4 x 11, same face order as the fit's own faces
                Y = own_readings(cells, faces, vid, links, ci, flux)
                Yo = Y[:4]                                    # 4 x 3
                rel.append(float(np.linalg.norm(A @ C.T - Yo) / np.linalg.norm(Yo)))
                u = np.linalg.svd(A)[0][:, -1]                # left null vector of the own-face flux map
                sm.append(float(np.linalg.norm(np.outer(u, u @ Yo)) / np.linalg.norm(Yo)))   # least possible rel
            rel, sm = np.array(rel), np.array(sm)
            out[flux] = {"cells": len(cells), "rel_median": float(np.median(rel)), "rel_p95": float(np.percentile(rel, 95)),
                         "rel_max": float(rel.max()), "floor_median": float(np.median(sm)), "floor_max": float(sm.max())}
        out["seconds"] = round(time.time() - t0, 1)
        res[f"n={level + 1}"] = out
        print(f"n={level + 1}", json.dumps(out), flush=True)
        outp.write_text(json.dumps(res, indent=1) + "\n")


def own_readings(cells, faces, vid, links, ci, flux):
    """The registered own-face readings exactly as recover_cell builds them (first four rows of its Y)."""
    nb = tw.neighbourhood_rings(cells, faces, ci, 2)
    # transport tree, as in recover_cell
    from collections import deque
    adj = {}
    for nj in nb:
        for (i, j) in pg.EDGES:
            a, b = vid[(nj, i)], vid[(nj, j)]
            u_ab = su.local_link(cells[nj], links, i, j)
            adj.setdefault(a, {})[b] = u_ab
            adj.setdefault(b, {})[a] = su.inverse(u_ab)
    root = vid[(ci, 0)]; W = {root: su.ONE.copy()}; q = deque([root])
    while q:
        u = q.popleft()
        for v in sorted(adj[u]):
            if v not in W:
                W[v] = su.compose(W[u], adj[u][v]); q.append(v)
    c = cells[ci]; Y = []
    for (i, j, k) in pg.FACES:
        uij, ujk, uki = (su.local_link(c, links, i, j), su.local_link(c, links, j, k), su.local_link(c, links, k, i))
        hol = su.compose(su.compose(uij, ujk), uki)
        x = hol[1:] if flux == "defect" else su.qlog(hol)
        Y.append(su.ad(W[vid[(ci, i)]], x))
    return np.array(Y)


if __name__ == "__main__":
    main()
