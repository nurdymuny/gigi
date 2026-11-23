"""Hybrid three-ring fit for the Petrov-Galerkin force (spec: docs/RICCI_PG_HYBRID_SPEC.md).

Same chart, basis, transport rule and coincidence check as scripts/ricci_pg_curvature.py, and the
same neighbourhood construction as scripts/ricci_pg_tworing.py (both imported read-only). The only
change is the per-cell map from neighbourhood fluxes to the 11 closed linear coefficients:

  1. fit the 11 closed linear AND 15 closed quadratic coefficients (26) by equality-constrained least
     squares: K's own four face fluxes exact, every other distinct neighbourhood face in unweighted
     least squares, exactly as fit_cell_rings does for 11;
  2. keep the linear part (the chart origin is K's projected centroid, where homogeneous quadratics
     vanish), M26[:11];
  3. add the minimum-norm closed linear field that carries the quadratic part's flux through K's OWN
     faces: M = M26[:11] + pinv(A_lin) A_quad M26[11:].

So K's own fluxes are reproduced exactly (for all data: own fluxes of edge data are closed), closed
linear data are reproduced exactly, and a quadratic field is read as K's own-face representative of
it, with no bias drawn from the outer rings. The trial space, the test functions, the force assembly,
the mass, the data and the BW05 measurement are unchanged.

Closed quadratic fields: the 15 divergence-free homogeneous quadratic vector fields in the orthonormal
chart; face fluxes by the edge-midpoint rule, which is exact for quadratics.
"""
import itertools

import numpy as np

import ricci_pg_curvature as pg
import ricci_pg_tworing as tr

# ---- closed quadratic fields (identical to scripts/ricci_variational_quadcount_lib.py) -------------
_MON = list(itertools.combinations_with_replacement(range(3), 2))       # x_i x_j, 6 monomials


def _div_row(comp, m):
    r = np.zeros(3); i, j = m
    if i == comp:
        r[j] += 1
    if j == comp:
        r[i] += 1
    return r


_DV = np.zeros((3, 18))
for _c in range(3):
    for _mi, _m in enumerate(_MON):
        _DV[:, _c * 6 + _mi] = _div_row(_c, _m)
_, _s, _vt = np.linalg.svd(_DV)
_Q = _vt[np.sum(_s > 1e-12):]                                            # 15 x 18
assert _Q.shape[0] == 15


def quad_field(x):
    vals = np.stack([x[..., i] * x[..., j] for i, j in _MON], -1)
    out = np.zeros(x.shape[:-1] + (15, 3))
    for comp in range(3):
        out[..., comp] = vals @ _Q[:, comp * 6:(comp + 1) * 6].T
    return out


def quad_flux(tri):
    z1, z2, z3 = tri
    area = np.cross(z2 - z1, z3 - z1) / 2
    mids = np.array([(z1 + z2) / 2, (z2 + z3) / 2, (z3 + z1) / 2])
    return quad_field(mids).mean(0) @ area                                 # 15


# ---- the fit ---------------------------------------------------------------------------------------
def _eclsq(X, n_own):
    """Equality-constrained least squares as a linear map: own rows exact, the rest least squares."""
    A, R = X[:n_own], X[n_own:]
    Ap = np.linalg.pinv(A, rcond=pg.RCOND)
    _, sa, vt = np.linalg.svd(A)
    ra = int((sa > pg.RCOND * sa[0]).sum())
    N = vt[ra:].T
    RNp = np.linalg.pinv(R @ N, rcond=pg.RCOND)
    return np.hstack([Ap - N @ RNp @ R @ Ap, N @ RNp])


def fit_cell_hybrid(cells, a, faces, ci, transport=True, include_neighbours=True, rings=3):
    nb = tr.neighbourhood_rings(cells, faces, ci, rings) if include_neighbours else \
        {ci: np.array(cells[ci]["vertices"], float)}
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
            (own if nj == ci else rest).append((pg.flux_row(tri), quad_flux(tri), d, pg.face_edge_terms(cells[nj], f)))
    rows = own + rest
    n_own = len(own)
    Xl = np.array([r[0] for r in rows]); Xq = np.array([r[1] for r in rows])
    X = np.hstack([Xl, Xq])
    s = np.linalg.svd(X, compute_uv=False)
    rank = int((s > pg.RCOND * s[0]).sum())
    if rank < 26:
        raise ValueError(f"closed linear+quadratic flux map has rank {rank} < 26 for cell {ci}")
    cond = float(s[0] / s[-1])
    if cond > pg.COND_CAP:
        raise ValueError(f"condition {cond:.3e} exceeds cap for cell {ci}")
    M26 = _eclsq(X, n_own)                                                 # 26 x nfaces
    Al, Aq = Xl[:n_own], Xq[:n_own]
    fmap = M26[:11] + np.linalg.pinv(Al, rcond=pg.RCOND) @ Aq @ M26[11:]  # 11 x nfaces
    d = np.array([r[2] for r in rows])
    x = fmap @ d
    own_res = float(np.max(abs(Al @ x - d[:n_own])))
    if own_res > 1e-12 * max(1., float(np.max(abs(d)))):
        raise ValueError(f"own face fluxes of cell {ci} not reproduced ({own_res:.3e})")
    return {"x": x, "c": x @ pg.BC, "B": pg._B(x), "J": J, "t0": t0, "map": fmap,
            "terms": [r[3] for r in rows], "rank": rank, "cond": cond,
            "amplification": float(np.linalg.norm(fmap, 2)),
            "own_residual": own_res, "faces": len(rows), "cells": len(nb)}


def use(rings=3):
    """Route the PG branch's build() through the hybrid fit."""
    pg.fit_cell = lambda cells, a, faces, ci, transport=True, include_neighbours=True: \
        fit_cell_hybrid(cells, a, faces, ci, transport, include_neighbours, rings)
