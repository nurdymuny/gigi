"""Two-ring least-squares variant of the PG branch's closed linear star fit.

Same chart, basis, transport rule and coincidence check as scripts/ricci_pg_curvature.py
(imported read-only). Differences:
  * neighbourhood = cells within `rings` face-steps of K, each carried onto K's sheet by the
    left quaternion multiplication fixed at the face it was reached through;
  * fit = K's own four face fluxes as exact equality constraints, all other distinct
    neighbourhood faces in unweighted least squares (the PG spec's original step 5).
With rings=1 this reproduces the original exactly-determined interpolation (checked).
The fit stays exact for closed linear data; on rough data it no longer has to interpolate.
"""
import numpy as np

import ricci_pg_curvature as pg


def neighbourhood_rings(cells, faces, ci, rings=2):
    K = np.array(cells[ci]["vertices"], float)
    out = {ci: K}
    frontier = [ci]
    for _ in range(rings):
        fresh = []
        for cj in frontier:
            C = out[cj]
            for face in pg.FACES:
                for nj, fn in faces[pg.face_key(cells[cj], face)]:
                    if nj == cj:
                        continue
                    ck, cn = pg.corners(cells[cj], face), pg.corners(cells[nj], fn)
                    V = np.array(cells[nj]["vertices"], float)
                    k0 = next(iter(ck))
                    h = pg.qmul(C[ck[k0]], pg.qconj(V[cn[k0]]))
                    V = np.array([pg.qmul(h, v) for v in V])
                    gap = max(float(np.max(abs(C[ck[k]] - V[cn[k]]))) for k in ck)
                    if gap > pg.COINCIDE:
                        raise ValueError(f"cell {nj} does not coincide with {cj} after transport ({gap:.3e})")
                    if nj in out:
                        continue          # first sheet wins; a second sheet means the ball wrapped
                    out[nj] = V
                    fresh.append(nj)
        frontier = fresh
    return out


def fit_cell_rings(cells, a, faces, ci, transport=True, include_neighbours=True, rings=2):
    nb = neighbourhood_rings(cells, faces, ci, rings) if include_neighbours else {ci: np.array(cells[ci]["vertices"], float)}
    to_orth, J, t0 = pg.chart(nb[ci])
    seen, own, rest = set(), [], []
    for nj in [ci] + [k for k in nb if k != ci]:
        O = to_orth(nb[nj])
        for f in pg.FACES:
            key = pg.face_key(cells[nj], f)
            if key in seen:
                continue
            seen.add(key)
            tri = O[list(f)]
            d = a(tri) if callable(a) else pg.data_flux(cells[nj], a, f)
            (own if nj == ci else rest).append((pg.flux_row(tri), d, pg.face_edge_terms(cells[nj], f)))
    A = np.array([r for r, _, _ in own]); R = np.array([r for r, _, _ in rest])
    rows = np.vstack([A, R])
    s = np.linalg.svd(rows, compute_uv=False)
    rank = int((s > pg.RCOND*s[0]).sum())
    if rank < 11:
        raise ValueError(f"closed linear flux map has rank {rank} < 11 for cell {ci}")
    cond = float(s[0]/s[-1])
    if cond > pg.COND_CAP:
        raise ValueError(f"condition {cond:.3e} exceeds cap for cell {ci}")
    # equality-constrained least squares, null-space method, as a linear map of all fluxes
    Ap = np.linalg.pinv(A, rcond=pg.RCOND)                       # 11 x 4
    _, sa, vt = np.linalg.svd(A)
    ra = int((sa > pg.RCOND*sa[0]).sum())
    N = vt[ra:].T                                                 # 11 x (11-ra)
    RN = R@N
    RNp = np.linalg.pinv(RN, rcond=pg.RCOND)
    fmap = np.hstack([Ap - N@RNp@R@Ap, N@RNp])                    # 11 x nfaces
    d = np.array([dd for _, dd, _ in own+rest])
    x = fmap@d
    own_res = float(np.max(abs(A@x - d[:len(own)])))
    if own_res > 1e-12*max(1., float(np.max(abs(d)))):
        raise ValueError(f"own face fluxes of cell {ci} not reproduced ({own_res:.3e})")
    return {"x": x, "c": x@pg.BC, "B": pg._B(x), "J": J, "t0": t0, "map": fmap,
            "terms": [t for _, _, t in own+rest], "rank": rank, "cond": cond,
            "residual": float(np.max(abs(rows@x-d))), "faces": len(own)+len(rest), "cells": len(nb)}


def use(rings):
    """Route the PG branch's build() through the ring-r fit."""
    pg.fit_cell = lambda cells, a, faces, ci, transport=True, include_neighbours=True: \
        fit_cell_rings(cells, a, faces, ci, transport, include_neighbours, rings)
