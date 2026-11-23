"""S2b: generalized spectrum of the linearised SU(2) force against the native
tangent mass. Spec: docs/RICCI_SU2_PG_STABILITY_SPEC.md. Research script.

usage: python scripts/ricci_su2_pg_stability.py LEVEL TRIAL [RECEIPT.json]
  TRIAL  native | pg | pg_onering
"""
import json
import os
import sys
import time
from collections import deque
from pathlib import Path

import numpy as np
from scipy.linalg import eig

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import ricci_su2_pg_force as F    # noqa: E402  (sets pg.BASE from GIGI_RICCI_DATA_ROOT)
from ricci_su2_pg_force import pg, tw, su, compose, inverse, rotations, defects, defect_derivatives  # noqa: E402
from test_ricci_operator_native import sparse_matrix  # noqa: E402

ORDER = 6
EPS_COL = 1e-6


def exp_tau(t, k):
    v = np.zeros(4)
    v[0] = np.cos(t)
    v[1 + k] = np.sin(t)
    return v


class Setup:
    def __init__(self, level, trial):
        self.payload = json.loads((F.BERGER / f"level{level}_berger_q{ORDER}.input.json").read_text())
        self.raw = json.loads((F.BERGER / f"level{level}_berger_q{ORDER}.output.json").read_text())
        self.links = np.asarray(self.raw["links"], float)
        self.cells = self.payload["cells"]
        self.trial = trial
        self.n = len(self.links)
        cells = self.cells
        self.quad, self.M = [], []
        for c in cells:
            q = F.quadrature(c["vertices"], (3.0, 1.0, 1.0), ORDER)
            self.quad.append(q)
            self.M.append(q["m2"])
        self.reads = [set(c["edges"]) for c in cells]
        if trial != "native":
            rings = 1 if trial == "pg_onering" else 2
            faces = pg.face_index(cells)
            vid = su.vertex_ids(cells)
            zeros = np.zeros(self.n)
            self.plan = []
            for ci in range(len(cells)):
                fit = tw.fit_cell_rings(cells, zeros, faces, ci, rings=rings)
                nb = tw.neighbourhood_rings(cells, faces, ci, rings)
                # faces in the fit map's order
                seen, flist = set(), []
                for nj in [ci] + [k for k in nb if k != ci]:
                    for f in pg.FACES:
                        key = pg.face_key(cells[nj], f)
                        if key not in seen:
                            seen.add(key)
                            flist.append((nj, f))
                # BFS tree over the neighbourhood's vertex graph (same rule as su.recover_cell)
                adj = {}
                for nj in nb:
                    for (i, j) in pg.EDGES:
                        a, b = vid[(nj, i)], vid[(nj, j)]
                        adj.setdefault(a, {})[b] = (nj, i, j)
                        adj.setdefault(b, {})[a] = (nj, j, i)
                root = vid[(ci, 0)]
                order, parent, done = [], {}, {root}
                queue = deque([root])
                while queue:
                    u = queue.popleft()
                    for v in sorted(adj[u]):
                        if v not in done:
                            done.add(v)
                            parent[v] = (u,) + adj[u][v]
                            order.append(v)
                            queue.append(v)
                # pairing matrix: hvec[i, A] = sum_f T[i, f] Y[f, A]
                quad = self.quad[ci]
                PT = np.stack([pg.proxy_ref(dict(fit, x=np.eye(11)[m]), quad["local"]) for m in range(11)], axis=-1)
                T = np.einsum('qab,qib,qam,q,mf->if', quad["g"], quad["two"], PT, quad["w"], fit["map"])
                read = set(cells[ci]["edges"])
                for nj, f in flist:
                    for (i, j) in [(f[0], f[1]), (f[1], f[2]), (f[0], f[2])]:
                        read.add(cells[nj]["edges"][pg.EDGES.index((i, j))])
                for v in order:
                    u, nj, i, j = parent[v]
                    read.add(cells[nj]["edges"][pg.EDGES.index((min(i, j), max(i, j)))])
                self.reads[ci] = read
                self.plan.append(dict(flist=flist, order=order, parent=parent, root=root, T=T, vid=vid))
        self.affected = {}
        for ci, r in enumerate(self.reads):
            for g in r:
                self.affected.setdefault(g, []).append(ci)

    def cell_contrib(self, ci, links):
        """{global link: 3-vector} contribution of cell ci to the gradient."""
        cell = self.cells[ci]
        u = F.local_links(cell, links)
        D = defects(u)
        h = self.M[ci] @ D
        if self.trial != "native":
            pl = self.plan[ci]
            W = {pl["root"]: F.ONE.copy()}
            for v in pl["order"]:
                p, nj, i, j = pl["parent"][v]
                W[v] = compose(W[p], su.local_link(self.cells[nj], links, i, j))
            Y = []
            for nj, (i, j, k) in pl["flist"]:
                c = self.cells[nj]
                hol = compose(compose(su.local_link(c, links, i, j), su.local_link(c, links, j, k)),
                              su.local_link(c, links, k, i))
                Y.append(su.ad(W[pl["vid"][(nj, i)]], hol[1:]))
            hvec = pl["T"] @ np.array(Y)
            h = np.column_stack([h[:, 0], hvec])
        dD = defect_derivatives(u)
        a_loc = 4 * np.einsum('ekic,ic->ek', dD, h)
        out = {}
        for e in range(6):
            g = cell["edges"][e]
            lift = -rotations(links[g]) if cell["reverse"][e] else np.eye(3)
            out[g] = out.get(g, 0) + cell["weight"] * lift.T @ a_loc[e]
        return out

    def gradient(self, links):
        grad = np.zeros((self.n, 3))
        for ci in range(len(self.cells)):
            for g, v in self.cell_contrib(ci, links).items():
                grad[g] += v
        return grad

    def column(self, g, k):
        col = np.zeros((self.n, 3))
        for sign in (1, -1):
            pert = self.links.copy()
            pert[g] = compose(self.links[g], exp_tau(sign * EPS_COL, k))
            for ci in self.affected.get(g, []):
                for gg, v in self.cell_contrib(ci, pert).items():
                    col[gg] += sign * v
        return (col / (2 * EPS_COL)).ravel()


def main():
    level, trial = int(sys.argv[1]), sys.argv[2]
    receipt = Path(sys.argv[3]) if len(sys.argv) > 3 else None
    if receipt and receipt.exists():
        raise SystemExit(f"refusing to overwrite {receipt}")
    t0 = time.time()
    S = Setup(level, trial)
    # C-grad
    grad = S.gradient(S.links)
    if trial == "native":
        ref = np.asarray(S.raw["gradient"], float)
    else:
        ref = F.run(level, ORDER, trial)["_grad"]
    c_grad = float(np.linalg.norm(grad - ref) / np.linalg.norm(ref))
    print("setup", round(time.time() - t0, 1), "s; C-grad", c_grad, flush=True)
    # C-coord: energy derivative equals the native gradient (native only)
    # C-col: one column, full gradient vs local
    g0, k0 = int(np.argmax(np.linalg.norm(grad, axis=1))), 0
    full = np.zeros_like(grad)
    for sign in (1, -1):
        pert = S.links.copy()
        pert[g0] = compose(S.links[g0], exp_tau(sign * EPS_COL, k0))
        full += sign * S.gradient(pert)
    full = (full / (2 * EPS_COL)).ravel()
    local = S.column(g0, k0)
    c_col = float(np.linalg.norm(full - local) / max(np.linalg.norm(full), 1e-300))
    print("C-col", c_col, flush=True)
    N = 3 * S.n
    K = np.zeros((N, N))
    t1 = time.time()
    for g in range(S.n):
        for k in range(3):
            K[:, 3 * g + k] = S.column(g, k)
        if g % 200 == 0:
            print("columns", g, "/", S.n, round(time.time() - t1, 1), "s", flush=True)
    B = sparse_matrix(S.raw).toarray()
    asym = float(np.max(np.abs(B - B.T)))
    lam = eig(K, B, right=False)
    small = int(np.sum(np.abs(lam) < 1))
    rough = int(np.sum(lam.real < -10))
    phys = lam[np.abs(lam) >= 1]
    out = {"level": level, "trial": trial, "dof": N, "C_col": c_col, "C_grad": c_grad,
           "B_asymmetry": asym, "eigen_count": int(len(lam)),
           "small_modulus_below_1": small, "rough_real_below_minus10": rough,
           "negative_real_physical": int(np.sum(phys.real < 0)),
           "min_real": float(lam.real.min()), "min_real_physical": float(phys.real.min()),
           "max_abs_imag": float(np.abs(lam.imag).max()),
           "seconds": round(time.time() - t0, 1)}
    print(json.dumps(out), flush=True)
    if receipt:
        np.save(receipt.with_suffix(".eigs.npy"), lam)
        receipt.write_text(json.dumps(out, indent=2) + "\n")


if __name__ == "__main__":
    main()
