"""Covariant two-ring curvature recovery for SU(2) links: research prototype of
S1 in docs/RECOVER_CURVATURE_SU2_SPEC.md. Python, not an engine verb.

The fit map is the paper's (scripts/ricci_pg_tworing.py, unchanged). What is
new here is what it is fed:

  face flux    X_f = log(U_ij . U_jk . U_ki)          (holonomy at the face's base vertex i)
  register     Y_f = W_i X_f W_i^-1                    (W_i: transport from i to the cell's vertex 0)
  fit          C (3 x 11) = Y (3 x faces) . R_K^T      (one closed linear field per su(2) component)

Group product and inverse are the repository's own (ricci_consistency_math.compose
and inverse: the stored-link, minus-cross convention). Gate G1 checks by
measurement that this is consistent. A stored link for stored edge tail->head maps
the fibre at head to the fibre at tail; under a gauge change g it becomes
g_tail U g_head^-1.

Mechanism switches (for removals): register=False uses each flux in its own base
frame; abelianised=True replaces log(holonomy) by the sum of the edge logs.
"""
from collections import deque
from pathlib import Path
import sys

import numpy as np

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT / "e2e/probes"))
import ricci_pg_curvature as pg                       # noqa: E402
import ricci_pg_tworing as tw                         # noqa: E402
from ricci_consistency_math import compose, inverse  # noqa: E402

ONE = np.array([1.0, 0.0, 0.0, 0.0])


def qexp(x):
    """exp of an imaginary quaternion (3-vector) -> unit quaternion."""
    x = np.asarray(x, float)
    t = np.linalg.norm(x)
    if t < 1e-300:
        return ONE.copy()
    return np.r_[np.cos(t), np.sin(t) * x / t]


def qlog(q):
    """log of a unit quaternion -> imaginary part (3-vector). Refuses near -1."""
    q = np.asarray(q, float)
    v = q[1:]
    s = np.linalg.norm(v)
    if q[0] < -1 + 1e-6 and s < 1e-3:
        raise ValueError("face holonomy near -1: logarithm ill-defined")
    if s < 1e-300:
        return np.zeros(3)
    return v * (np.arctan2(s, q[0]) / s)


def ad(g, x):
    """Ad_g x = g x g^-1 for an imaginary quaternion x (3-vector)."""
    return compose(compose(g, np.r_[0.0, x]), inverse(g))[1:]


def vertex_ids(cells):
    """Global vertex ids on the quotient, by union-find over shared stored edges:
    the tail (head) of a stored edge is the same vertex in every cell using it."""
    parent = {}

    def find(x):
        parent.setdefault(x, x)
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    def union(a, b):
        ra, rb = find(a), find(b)
        if ra != rb:
            parent[max(ra, rb)] = min(ra, rb)

    ends = {}
    for ci, c in enumerate(cells):
        for e, (i, j) in enumerate(pg.EDGES):
            g = c["edges"][e]
            tail, head = ((ci, i), (ci, j)) if not c["reverse"][e] else ((ci, j), (ci, i))
            if g in ends:
                union(ends[g][0], tail)
                union(ends[g][1], head)
            else:
                ends[g] = (tail, head)
    roots = {}
    vid = {}
    for ci in range(len(cells)):
        for v in range(4):
            r = find((ci, v))
            vid[(ci, v)] = roots.setdefault(r, len(roots))
    return vid


def local_link(cell, links, i, j):
    """Link for local edge i -> j of a cell: maps the fibre at j to the fibre at i."""
    e = pg.EDGES.index((min(i, j), max(i, j)))
    u = np.asarray(links[cell["edges"][e]], float)
    if cell["reverse"][e]:          # stored edge runs j -> i: u maps i to j
        u = inverse(u)
    return u if i < j else inverse(u)


def recover_cell(cells, faces, vid, links, ci, rings=2, register=True, abelianised=False, fit=None,
                 flux="log"):
    """flux='log': X = log(holonomy) (S1). flux='defect': X = vector part of the
    holonomy, the native action's face defect (S2a)."""
    nb = tw.neighbourhood_rings(cells, faces, ci, rings)
    if fit is None:
        fit = tw.fit_cell_rings(cells, np.zeros(1 + max(max(c["edges"]) for c in cells)), faces, ci, rings=rings)
    fmap = fit["map"]

    # transport W[v]: fibre at v -> fibre at the cell's vertex 0, along a BFS tree
    adj = {}
    for nj in nb:
        for (i, j) in pg.EDGES:
            a, b = vid[(nj, i)], vid[(nj, j)]
            u_ab = local_link(cells[nj], links, i, j)
            adj.setdefault(a, {})[b] = u_ab
            adj.setdefault(b, {})[a] = inverse(u_ab)
    root = vid[(ci, 0)]
    W = {root: ONE.copy()}
    queue = deque([root])
    while queue:
        u = queue.popleft()
        for v in sorted(adj[u]):
            if v not in W:
                W[v] = compose(W[u], adj[u][v])
                queue.append(v)

    seen, Y = set(), []
    for nj in [ci] + [k for k in nb if k != ci]:
        c = cells[nj]
        for (i, j, k) in pg.FACES:
            key = pg.face_key(c, (i, j, k))
            if key in seen:
                continue
            seen.add(key)
            uij, ujk, uki = (local_link(c, links, i, j), local_link(c, links, j, k),
                             local_link(c, links, k, i))
            if abelianised:
                x = qlog(uij) + qlog(ujk) + qlog(uki)
            elif flux == "defect":
                x = compose(compose(uij, ujk), uki)[1:]
            else:
                x = qlog(compose(compose(uij, ujk), uki))
            Y.append(ad(W[vid[(nj, i)]], x) if register else x)
    Y = np.array(Y)
    if Y.shape[0] != fmap.shape[1]:
        raise RuntimeError("face order drifted from the fit map")
    return {"coefficients": Y.T @ fmap.T, "fit": fit, "frame_vertex": root}


def abelian_links(a, axis=2):
    """The Abelian cochain embedded in SU(2) along one axis: U_e = exp(a_e e_axis)."""
    e = np.zeros(3)
    e[axis] = 1.0
    return np.array([qexp(ae * e) for ae in np.asarray(a, float)])


def gauge_transform(cells, vid, links, gauge):
    """U' = g_tail U g_head^-1 for every stored edge (tail -> head as stored)."""
    out = np.array(links, float).copy()
    done = set()
    for ci, c in enumerate(cells):
        for e, (i, j) in enumerate(pg.EDGES):
            g = c["edges"][e]
            if g in done:
                continue
            done.add(g)
            tail, head = (i, j) if not c["reverse"][e] else (j, i)
            out[g] = compose(compose(gauge[vid[(ci, tail)]], links[g]), inverse(gauge[vid[(ci, head)]]))
    return out
