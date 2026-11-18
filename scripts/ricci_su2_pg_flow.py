"""S3: the SU(2) force in time against the Ricci-flowed Levi-Civita links (BW07 replica).
Spec: docs/RICCI_SU2_PG_FLOW_SPEC.md. Research script, not an engine verb.

usage:
  python scripts/ricci_su2_pg_flow.py controls LEVEL
  python scripts/ricci_su2_pg_flow.py run LEVEL TRIAL N [ORDER] [RECEIPT.json]
    TRIAL  pg | native | pg_fixed | pg_onering        N  steps over [0, 0.02]
"""
import json
import sys
import time
from collections import deque
from pathlib import Path

import numpy as np
from scipy.integrate import solve_ivp
from scipy.sparse import coo_matrix
from scipy.sparse.linalg import cg, LinearOperator

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import ricci_su2_pg_force as F                     # noqa: E402  (sets pg.BASE from GIGI_RICCI_DATA_ROOT)
from ricci_su2_pg_force import pg, tw, su, compose, inverse, rotations, defects, defect_derivatives  # noqa: E402
sys.path.insert(0, str(HERE.parent / "e2e/probes"))
from ricci_lattice_demo import matrix_link, SIGMA  # noqa: E402
import ricci_oracle as RO                          # noqa: E402
from test_ricci_operator_native import sparse_matrix  # noqa: E402

T_END = 0.02
G0 = np.array([3.0, 1.0, 1.0])
ONE = np.array([1.0, 0, 0, 0])


# ---------------------------------------------------------------- group helpers
def qexp(v):
    v = np.asarray(v, float)
    n = np.linalg.norm(v, axis=-1, keepdims=True)
    s = np.where(n > 1e-300, np.sin(n) / np.where(n > 1e-300, n, 1), 1.0)
    return np.concatenate([np.cos(n), s * v], axis=-1)


def qlog(q):
    q = np.asarray(q, float)
    q = q * np.where(q[..., :1] < 0, -1.0, 1.0)          # shortest representative (U and -U are distinct in SU(2), but
    v = q[..., 1:]                                        # every link here stays near its exact value)
    n = np.linalg.norm(v, axis=-1, keepdims=True)
    ang = np.arctan2(n, q[..., :1])
    return np.where(n > 1e-300, ang / np.where(n > 1e-300, n, 1), 1.0) * v


def bracket(a, b):
    return -2.0 * np.cross(a, b)          # [ (0,a), (0,b) ] under compose; checked by C-bracket


def dexpinv_right(omega, v):
    """Omega' for Y = Y0 exp(Omega), Y' = Y v:  v + 1/2 [Omega, v] + 1/12 [Omega, [Omega, v]]."""
    c1 = bracket(omega, v)
    return v + 0.5 * c1 + bracket(omega, c1) / 12.0


def to_quat(M):
    q0 = np.trace(M).real / 2
    qv = [(np.trace(s @ M) / 2j).real for s in SIGMA]
    return np.r_[q0, qv]


# ---------------------------------------------------------------- metric along the Ricci flow
class Metric:
    def __init__(self, fixed=False):
        self.fixed = fixed
        sol = solve_ivp(lambda t, g: RO.velocity(g, False), (0, T_END), G0, method="DOP853",
                        rtol=2e-12, atol=2e-14, dense_output=True)
        self.sol = sol

    def __call__(self, t):
        return G0.copy() if self.fixed else self.sol.sol(t)


# ---------------------------------------------------------------- the lattice
class Lattice:
    def __init__(self, level, trial, order=6):
        self.level, self.trial, self.order = level, trial, order
        self.payload = json.loads((F.BERGER / f"level{level}_berger_q{order}.input.json").read_text())
        self.raw = json.loads((F.BERGER / f"level{level}_berger_q{order}.output.json").read_text())
        self.cells = self.payload["cells"]
        self.edge_points = self.payload["edge_points"]
        self.n = len(self.raw["links"])
        self._quad_cache()
        if trial in ("pg", "pg_fixed", "pg_onering"):
            self._plan(1 if trial == "pg_onering" else 2)

    # metric-independent quadrature pieces, per cell
    def _quad_cache(self):
        import itertools
        roots, weights = np.polynomial.legendre.leggauss(self.order)
        roots = (roots + 1) / 2
        weights = weights / 2
        idx = np.array(list(itertools.product(range(self.order), repeat=3)))
        uu, vv, ww = roots[idx].T
        local = np.column_stack([uu, (1 - uu) * vv, (1 - uu) * (1 - vv) * ww])
        lam = np.column_stack([1 - local.sum(axis=1), local])
        weight = np.prod(weights[idx], axis=1) * (1 - uu) ** 2 * (1 - vv)
        grads = np.vstack([-np.ones(3), np.eye(3)])
        two = np.array([2 * (lam[:, i, None] * np.cross(grads[j], grads[k])
                             - lam[:, j, None] * np.cross(grads[i], grads[k])
                             + lam[:, k, None] * np.cross(grads[i], grads[j]))
                        for i, j, k in pg.FACES]).transpose(1, 0, 2)
        one = np.array([lam[:, a, None] * grads[b] - lam[:, b, None] * grads[a]
                        for a, b in pg.EDGES]).transpose(1, 0, 2)
        self.local, self.qweight, self.two, self.one = local, weight, two, one
        maur = []
        for c in self.cells:
            points = np.asarray(c["vertices"], float)
            b = (points[1:] - points[0]).T
            y = lam @ points
            norm = np.linalg.norm(y, axis=1)
            x = y / norm[:, None]
            dx = (b[None] - x[:, :, None] * (x @ b)[:, None, :]) / norm[:, None, None]
            frame = np.empty((len(x), 4, 3))
            frame[:, 0, :] = -x[:, 1:]
            sk = np.zeros((len(x), 3, 3))
            a_, bb, cc = x[:, 1], x[:, 2], x[:, 3]
            sk[:, 0, 1] = -cc; sk[:, 0, 2] = bb; sk[:, 1, 0] = cc
            sk[:, 1, 2] = -a_; sk[:, 2, 0] = -bb; sk[:, 2, 1] = a_
            frame[:, 1:, :] = x[:, 0, None, None] * np.eye(3) + sk
            maur.append(np.einsum("qia,qib->qab", frame, dx))
        self.maurer = np.array(maur)                         # cells x q x 3 x 3

    def metric_pieces(self, g):
        G = np.einsum("cqia,i,cqib->cqab", self.maurer, np.asarray(g, float), self.maurer)
        det = np.linalg.det(G)
        meas = self.qweight[None] * np.sqrt(det)
        w = meas / det
        m2 = np.einsum("qea,cqab,qfb,cq->cef", self.two, G, self.two, w)
        m1 = np.einsum("qea,cqab,qfb,cq->cef", self.one, np.linalg.inv(G), self.one, meas)
        vol = float(np.sum(meas.sum(axis=1) * np.array([c["weight"] for c in self.cells])))
        return {"G": G, "w": w, "m1": m1, "m2": m2, "vol": vol}

    def _plan(self, rings):
        cells = self.cells
        faces = pg.face_index(cells)
        vid = su.vertex_ids(cells)
        zeros = np.zeros(self.n)
        self.plan = []
        for ci in range(len(cells)):
            fit = tw.fit_cell_rings(cells, zeros, faces, ci, rings=rings)
            nb = tw.neighbourhood_rings(cells, faces, ci, rings)
            seen, flist = set(), []
            for nj in [ci] + [k for k in nb if k != ci]:
                for f in pg.FACES:
                    key = pg.face_key(cells[nj], f)
                    if key not in seen:
                        seen.add(key)
                        flist.append((nj, f))
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
            PT = np.stack([pg.proxy_ref(dict(fit, x=np.eye(11)[m]), self.local) for m in range(11)], axis=-1)
            self.plan.append(dict(flist=flist, order=order, parent=parent, root=root, vid=vid,
                                  PT=PT, map=fit["map"]))

    # ------------------------------------------------------------ force and mass
    def gradient(self, links, mp):
        grad = np.zeros((self.n, 3))
        for ci, cell in enumerate(self.cells):
            u = F.local_links(cell, links)
            D = defects(u)
            h = mp["m2"][ci] @ D
            if self.trial != "native":
                pl = self.plan[ci]
                T = np.einsum("qab,qib,qam,q,mf->if", mp["G"][ci], self.two, pl["PT"], mp["w"][ci], pl["map"])
                W = {pl["root"]: ONE.copy()}
                for v in pl["order"]:
                    p, nj, i, j = pl["parent"][v]
                    W[v] = compose(W[p], su.local_link(self.cells[nj], links, i, j))
                Y = []
                for nj, (i, j, k) in pl["flist"]:
                    c = self.cells[nj]
                    hol = compose(compose(su.local_link(c, links, i, j), su.local_link(c, links, j, k)),
                                  su.local_link(c, links, k, i))
                    Y.append(su.ad(W[pl["vid"][(nj, i)]], hol[1:]))
                h = np.column_stack([h[:, 0], T @ np.array(Y)])
            dD = defect_derivatives(u)
            a_loc = 4 * np.einsum("ekic,ic->ek", dD, h)
            for e in range(6):
                gid = cell["edges"][e]
                lift = -rotations(links[gid]) if cell["reverse"][e] else np.eye(3)
                grad[gid] += cell["weight"] * lift.T @ a_loc[e]
        return grad

    def mass(self, links, mp):
        rows, cols, vals = [], [], []
        for ci, cell in enumerate(self.cells):
            u = F.local_links(cell, links)
            R = rotations(np.array([u[0], u[1], u[2], u[1], u[2], u[2]]))
            lift = [(-rotations(links[cell["edges"][e]]) if cell["reverse"][e] else np.eye(3)) for e in range(6)]
            m1 = mp["m1"][ci]
            for e in range(6):
                for f in range(6):
                    blk = cell["weight"] * lift[e].T @ (4 * m1[e, f] * R[e].T @ R[f]) @ lift[f]
                    ge, gf = cell["edges"][e], cell["edges"][f]
                    r, c = np.meshgrid(np.arange(3) + 3 * ge, np.arange(3) + 3 * gf, indexing="ij")
                    rows.append(r.ravel()); cols.append(c.ravel()); vals.append(blk.ravel())
        N = 3 * self.n
        return coo_matrix((np.concatenate(vals), (np.concatenate(rows), np.concatenate(cols))), shape=(N, N)).tocsr()

    def velocity(self, links, g):
        mp = self.metric_pieces(g)
        B = self.mass(links, mp)
        rhs = -self.gradient(links, mp).ravel()
        d = B.diagonal()
        x, status = cg(B, rhs, rtol=1e-12, atol=1e-14, maxiter=5000, M=LinearOperator(B.shape, matvec=lambda z: z / d))
        res = np.linalg.norm(B @ x - rhs) / max(np.linalg.norm(rhs), 1e-300)
        if status != 0 or res > 1e-10:
            raise RuntimeError(f"mass solve failed: status {status}, residual {res:.2e}")
        return x.reshape(self.n, 3)

    # ------------------------------------------------------------ exact links
    def exact_links(self, g):
        out = []
        for (p, q), stored in zip(self.edge_points, self.raw["links"]):
            out.append(to_quat(matrix_link(np.asarray(g, float), p, q)))
        return np.array(out)

    def error(self, U, L, g):
        mp = self.metric_pieces(g)
        B = self.mass(L, mp)
        d = qlog(compose(inverse(L), U)).ravel()
        return float(np.sqrt(max(d @ (B @ d), 0.0) / mp["vol"]))

    def face_angles(self, links):
        a = []
        for c in self.cells:
            Dq = defects(F.local_links(c, links)) + ONE
            a.append(np.arctan2(np.linalg.norm(Dq[:, 1:], axis=1), Dq[:, 0]))
        return np.concatenate(a)


# ---------------------------------------------------------------- RKMK4
def rkmk4(U0, field, t0, h):
    """One RKMK4 step for U' = U * field(U, t) (right-trivialised)."""
    def F_(omega, t):
        return dexpinv_right(omega, field(compose(U0, qexp(omega)), t))
    k1 = h * field(U0, t0)
    k2 = h * F_(0.5 * k1, t0 + 0.5 * h)
    k3 = h * F_(0.5 * k2, t0 + 0.5 * h)
    k4 = h * F_(k3, t0 + h)
    return compose(U0, qexp((k1 + 2 * k2 + 2 * k3 + k4) / 6))


# ---------------------------------------------------------------- controls
def controls(level):
    out = {}
    lat = Lattice(level, "pg")
    g0 = G0
    # C-link
    L0 = lat.exact_links(g0)
    native = np.asarray(lat.raw["links"], float)
    sgn = np.sign(np.sum(L0 * native, axis=1))[:, None]
    out["C_link"] = float(np.abs(L0 * sgn - native).max())
    out["C_link_sign_flips"] = int(np.sum(sgn < 0))
    # C-mass
    mp = lat.metric_pieces(g0)
    B = lat.mass(native, mp).toarray()
    Bn = sparse_matrix(lat.raw).toarray()
    out["C_mass"] = float(np.abs(B - Bn).max() / np.abs(Bn).max())
    out["C_volume"] = float(abs(mp["vol"] - lat.raw["volume"]) / lat.raw["volume"])
    # C-force
    grad = lat.gradient(native, mp)
    ref = F.run(level, lat.order, "pg")["_grad"]
    out["C_force"] = float(np.linalg.norm(grad - ref) / np.linalg.norm(ref))
    # C-bracket
    rng = np.random.default_rng(0)
    a, b = rng.normal(size=3), rng.normal(size=3)
    s = 1e-4
    comm = compose(compose(compose(qexp(s * a), qexp(s * b)), qexp(-s * a)), qexp(-s * b))
    out["C_bracket"] = float(np.linalg.norm(qlog(comm) / s**2 - bracket(a, b)) / np.linalg.norm(bracket(a, b)))
    # C-exact: step the exact links' own velocity
    met = Metric()
    eps = 1e-6

    def exact_field(U, t):
        Lm, Lp = lat.exact_links(met(t - eps)), lat.exact_links(met(t + eps))
        return (qlog(compose(inverse(lat.exact_links(met(t))), Lp)) - qlog(compose(inverse(lat.exact_links(met(t))), Lm))) / (2 * eps)
    Lend = lat.exact_links(met(T_END))
    errs = {}
    for N in (4, 8, 16):
        U = L0.copy()
        h = T_END / N
        for k in range(N):
            U = rkmk4(U, exact_field, k * h, h)
        errs[N] = float(np.abs(qlog(compose(inverse(Lend), U))).max())
    out["C_exact_errors"] = errs
    out["C_exact_orders"] = [float(np.log2(errs[4] / errs[8])), float(np.log2(errs[8] / errs[16]))]
    return out


# ---------------------------------------------------------------- the run
def run(level, trial, N, order=6):
    t0 = time.time()
    lat = Lattice(level, trial, order)
    met = Metric(fixed=(trial == "pg_fixed"))
    true_met = Metric()
    h = T_END / N
    U = np.asarray(lat.raw["links"], float).copy()
    L0 = U.copy()
    samples = sorted(set([N // 4, N // 2, 3 * N // 4, N]))
    rows = []
    status = "ok"
    for k in range(N):
        try:
            U = rkmk4(U, lambda Y, t: lat.velocity(Y, met(t)), k * h, h)
        except Exception as exc:                       # noqa: BLE001  (reported, not hidden)
            status = f"failed at step {k + 1}: {exc}"
            break
        if not np.all(np.isfinite(U)):
            status = f"nonfinite at step {k + 1}"
            break
        U /= np.linalg.norm(U, axis=1, keepdims=True)
        if (k + 1) in samples:
            t = (k + 1) * h
            g = true_met(t)
            L = lat.exact_links(g)
            E, D = lat.error(U, L, g), lat.error(L0, L, g)
            fa_U, fa_L, fa_0 = lat.face_angles(U), lat.face_angles(L), lat.face_angles(L0)
            rows.append({"t": t, "E": E, "D": D, "R": E / D,
                         "gauge_invariant_R": float(np.linalg.norm(fa_U - fa_L) / np.linalg.norm(fa_0 - fa_L)),
                         "seconds": round(time.time() - t0, 1)})
            print(json.dumps(rows[-1]), flush=True)
    return {"level": level, "trial": trial, "N": N, "h": h, "order": order, "status": status,
            "rows": rows, "R_final": rows[-1]["R"] if rows and rows[-1]["t"] == T_END else None,
            "seconds": round(time.time() - t0, 1), "_final": U}


def main():
    mode = sys.argv[1]
    if mode == "controls":
        print(json.dumps(controls(int(sys.argv[2])), indent=1))
        return
    level, trial, N = int(sys.argv[2]), sys.argv[3], int(sys.argv[4])
    order = int(sys.argv[5]) if len(sys.argv) > 5 else 6
    receipt = Path(sys.argv[6]) if len(sys.argv) > 6 else None
    if receipt and receipt.exists():
        raise SystemExit(f"refusing to overwrite {receipt}")
    out = run(level, trial, N, order)
    final = out.pop("_final")
    print(json.dumps({k: v for k, v in out.items() if k != "rows"}), flush=True)
    if receipt:
        receipt.parent.mkdir(parents=True, exist_ok=True)
        np.save(receipt.with_suffix(".final.npy"), final)   # Amendment 1: final link states, for the gap
        receipt.write_text(json.dumps(out, indent=1) + "\n")


if __name__ == "__main__":
    main()
