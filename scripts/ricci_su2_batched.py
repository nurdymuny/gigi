"""The SU(2) force of S3/S3b, batched across cells. Spec: docs/RICCI_SU2_BATCHED_FORCE_SPEC.md.
Research script, not an engine verb. The per-cell loop in ricci_su2_pg_flow.Lattice is the reference.

usage:
  python scripts/ricci_su2_batched.py parity LEVEL TRIAL [RECEIPT.json]
  python scripts/ricci_su2_batched.py time LEVEL TRIAL [LOOP_REPEATS]
  python scripts/ricci_su2_batched.py run LEVEL TRIAL N [RECEIPT.json]   (S3b Cayley step, batched force)
  python scripts/ricci_su2_batched.py kparity LEVEL TRIAL [RECEIPT.json]  (B5 gates vs the loop)
  python scripts/ricci_su2_batched.py kbuild LEVEL TRIAL [RECEIPT.json]   (B5: K to outputs/su2-batched-K/)
    TRIAL  native | pg
"""
import json
import sys
import time
from pathlib import Path

import numpy as np
from scipy.sparse import coo_matrix

HERE = Path(__file__).resolve().parent
REPO = HERE.parent
sys.path.insert(0, str(HERE))
import ricci_su2_pg_flow as FL                     # noqa: E402
import ricci_su2_pg_force as F                     # noqa: E402
from ricci_su2_pg_force import pg, compose, inverse, rotations, FACE_EDGES, DEFECT_FACTORS, TAU  # noqa: E402

ONE = np.array([1.0, 0, 0, 0])
CONJ = np.array([1.0, -1, -1, -1])


def b_local_links(links, E, rev):
    u = links[E].copy()                                   # C x 6 x 4
    u[rev, 1:] *= -1
    return u


def b_defects(u):
    D = []
    for f, (a, b, c) in enumerate(FACE_EDGES):
        h = compose(compose(u[:, a], u[:, b]), inverse(u[:, c]))
        if f == 3:
            h = compose(compose(u[:, 0], h), inverse(u[:, 0]))
        D.append(h - ONE)
    return np.stack(D, axis=1)                            # C x 4 faces x 4


def b_defect_derivatives(u):
    C = u.shape[0]
    out = np.zeros((C, 6, 3, 4, 4))
    one = np.broadcast_to(ONE, (C, 4))
    for f, factors in enumerate(DEFECT_FACTORS):
        plain = [inverse(u[:, e]) if inv else u[:, e] for e, inv in factors]
        for pos, (e, inv) in enumerate(factors):
            left = one
            for q in plain[:pos]:
                left = compose(left, q)
            right = one
            for q in plain[pos + 1:]:
                right = compose(right, q)
            for k in range(3):
                d = compose(u[:, e], TAU[k])
                if inv:
                    d = d * CONJ
                out[:, e, k, f] += compose(compose(left, d), right)
    return out


def b_ad(g, x):
    return compose(compose(g, np.concatenate([np.zeros(x.shape[:-1] + (1,)), x], axis=-1)), inverse(g))[..., 1:]


def link_of(cell, i, j):
    """(global id, inverted?) for local edge i -> j, as su.local_link."""
    e = pg.EDGES.index((min(i, j), max(i, j)))
    return cell["edges"][e], bool(cell["reverse"][e]) != (i > j)


def take(links, gid, inv):
    q = links[gid]
    return np.where(inv[..., None], q * CONJ, q)


class BatchedLattice(FL.Lattice):
    def __init__(self, level, trial, order=6):
        self._mp_key, self._mp = None, None
        super().__init__(level, trial, order)
        cells = self.cells
        self.E = np.array([c["edges"] for c in cells])
        self.rev = np.array([c["reverse"] for c in cells], bool)
        self.w = np.array([c["weight"] for c in cells], float)
        C = len(cells)
        r = 3 * self.E[:, :, None] + np.arange(3)                       # C x 6 x 3
        self.mrows = np.broadcast_to(r[:, :, None, :, None], (C, 6, 6, 3, 3)).ravel()
        self.mcols = np.broadcast_to(r[:, None, :, None, :], (C, 6, 6, 3, 3)).ravel()
        if trial != "native":
            self._batch_plan()

    def _batch_plan(self):
        cells, C = self.cells, len(self.cells)
        Fmax = max(len(p["flist"]) for p in self.plan)
        self.PT = np.stack([p["PT"] for p in self.plan])                  # C x q x 3 x 11
        self.map = np.zeros((C, 11, Fmax))
        layers = {}
        Smax = 0
        ycell, yface, ylinks, yslot = [], [], [], []
        for ci, p in enumerate(self.plan):
            self.map[ci, :, :p["map"].shape[1]] = p["map"]
            slot, depth = {p["root"]: 0}, {p["root"]: 0}
            for v in p["order"]:
                par, nj, i, j = p["parent"][v]
                slot[v] = len(slot)
                depth[v] = depth[par] + 1
                gid, inv = link_of(cells[nj], i, j)
                layers.setdefault(depth[v], []).append((ci, slot[v], slot[par], gid, inv))
            Smax = max(Smax, len(slot))
            for fi, (nj, (i, j, k)) in enumerate(p["flist"]):
                c = cells[nj]
                ycell.append(ci); yface.append(fi)
                ylinks.append([link_of(c, i, j), link_of(c, j, k), link_of(c, k, i)])
                yslot.append(slot[p["vid"][(nj, i)]])
        self.Smax, self.Fmax = Smax, Fmax
        self.layers = []
        for d in sorted(layers):
            a = layers[d]
            self.layers.append(tuple(np.array([x[m] for x in a]) for m in range(5)))
        self.ycell, self.yface, self.yslot = np.array(ycell), np.array(yface), np.array(yslot)
        yl = np.array([[g for g, _ in t] for t in ylinks]), np.array([[iv for _, iv in t] for t in ylinks])
        self.ygid, self.yinv = yl
        # B5: the same plan, padded per cell, for row-batched evaluation (one cell per row).
        # Tree: entries in BFS order (a parent always precedes its children); padding writes a
        # dummy slot Smax from slot 0 and is never read.
        S1 = Smax - 1
        self.t_slot = np.full((C, S1), Smax); self.t_pslot = np.zeros((C, S1), int)
        self.t_gid = np.zeros((C, S1), int); self.t_inv = np.zeros((C, S1), bool)
        for ci, p in enumerate(self.plan):
            slot = {p["root"]: 0}
            for m, v in enumerate(p["order"]):
                par, nj, i, j = p["parent"][v]
                slot[v] = m + 1
                gid, inv = link_of(cells[nj], i, j)
                self.t_slot[ci, m], self.t_pslot[ci, m] = m + 1, slot[par]
                self.t_gid[ci, m], self.t_inv[ci, m] = gid, inv
        self.f_valid = np.zeros((C, Fmax), bool)
        self.f_gid = np.zeros((C, Fmax, 3), int); self.f_inv = np.zeros((C, Fmax, 3), bool)
        self.f_slot = np.zeros((C, Fmax), int)
        self.f_valid[self.ycell, self.yface] = True
        self.f_gid[self.ycell, self.yface] = self.ygid
        self.f_inv[self.ycell, self.yface] = self.yinv
        self.f_slot[self.ycell, self.yface] = self.yslot

    # ------------------------------------------------------------ B5: rows with one link nudged
    def reads(self):
        """Global links each cell's contribution depends on."""
        out = [set(e) for e in self.E.tolist()]
        if self.trial != "native":
            for ci in range(len(self.cells)):
                out[ci].update(self.t_gid[ci][self.t_slot[ci] < self.Smax].tolist())
                out[ci].update(self.f_gid[ci][self.f_valid[ci]].ravel().tolist())
        return [sorted(s) for s in out]

    def row_contrib(self, links, mp, T, cell, g, q):
        """contrib[r] (6 x 3): cell[r]'s gradient contribution at its six edges, with link g[r]
        replaced by quaternion q[r] wherever the cell reads it (lift included)."""
        def get(gids, inv=None):
            vals = links[gids]
            hit = gids == g.reshape((-1,) + (1,) * (gids.ndim - 1))
            vals = np.where(hit[..., None], q.reshape((-1,) + (1,) * (gids.ndim - 1) + (4,)), vals)
            return vals if inv is None else np.where(inv[..., None], vals * CONJ, vals)
        E, rev = self.E[cell], self.rev[cell]
        raw = get(E)                                                      # R x 6 x 4
        u = raw.copy(); u[rev, 1:] *= -1
        D = b_defects(u)
        h = np.matmul(mp["m2"][cell], D)
        if self.trial != "native":
            R = len(cell)
            W = np.zeros((R, self.Smax + 1, 4)); W[:, 0] = ONE
            ts, tp = self.t_slot[cell], self.t_pslot[cell]
            tl = get(self.t_gid[cell], self.t_inv[cell])                  # R x S1 x 4
            rr = np.arange(R)
            for m in range(ts.shape[1]):
                W[rr, ts[:, m]] = compose(W[rr, tp[:, m]], tl[:, m])
            L = get(self.f_gid[cell], self.f_inv[cell])                   # R x F x 3 x 4
            hol = compose(compose(L[:, :, 0], L[:, :, 1]), L[:, :, 2])
            Wf = W[rr[:, None], self.f_slot[cell]]                        # R x F x 4
            Y = np.where(self.f_valid[cell][..., None], b_ad(Wf, hol[..., 1:]), 0.0)
            hvec = np.matmul(T[cell], Y)
            h = np.concatenate([h[:, :, :1], hvec], axis=2)
        dD = b_defect_derivatives(u)
        a_loc = 4 * np.einsum("nekic,nic->nek", dD, h)
        lift = np.where(rev[:, :, None, None], -rotations(raw), np.eye(3))
        return np.einsum("ceji,cej->cei", lift, a_loc) * self.w[cell][:, None, None]

    def build_K(self, eps=1e-6, chunk=4000, columns=None):
        """K at the stored links and g(0) by central differences, as Setup.column. columns: an
        optional set of global links to build (all if None)."""
        from scipy.sparse import coo_matrix
        links = np.asarray(self.raw["links"], float)
        mp = self.metric_pieces(FL.G0)
        T = self._T(mp) if self.trial != "native" else None
        pairs = [(c, g) for c, rd in enumerate(self.reads()) for g in rd
                 if columns is None or g in columns]
        cell = np.array([c for c, _ in pairs]); gl = np.array([g for _, g in pairs])
        import ricci_su2_pg_stability as S2B
        N = 3 * self.n
        K = None
        for k in range(3):                     # assembled one direction at a time (memory at n = 5)
            rows, cols, vals = [], [], []
            for sign in (1.0, -1.0):
                q_all = compose(links[gl], S2B.exp_tau(sign * eps, k))
                for a in range(0, len(cell), chunk):
                    c, g, q = cell[a:a + chunk], gl[a:a + chunk], q_all[a:a + chunk]
                    v = self.row_contrib(links, mp, T, c, g, q) * (sign / (2 * eps))
                    rows.append((3 * self.E[c][:, :, None] + np.arange(3)).ravel().astype(np.int32))
                    cols.append(np.repeat(3 * g + k, 18).astype(np.int32))
                    vals.append(v.ravel())
            Kk = coo_matrix((np.concatenate(vals), (np.concatenate(rows), np.concatenate(cols))),
                            shape=(N, N)).tocsc()
            K = Kk if K is None else K + Kk
        return K

    def metric_pieces(self, g):
        """Amendment 1: cached by g (exact), optimised contraction order (rounding only)."""
        g = np.asarray(g, float)
        if self._mp_key is not None and np.array_equal(self._mp_key, g):
            return self._mp
        G = np.einsum("cqia,i,cqib->cqab", self.maurer, g, self.maurer, optimize=True)
        det = np.linalg.det(G)
        meas = self.qweight[None] * np.sqrt(det)
        w = meas / det
        m2 = np.einsum("qea,cqab,qfb,cq->cef", self.two, G, self.two, w, optimize=True)
        m1 = np.einsum("qea,cqab,qfb,cq->cef", self.one, np.linalg.inv(G), self.one, meas, optimize=True)
        vol = float(np.sum(meas.sum(axis=1) * self.w))
        self._mp_key, self._mp = g.copy(), {"G": G, "w": w, "m1": m1, "m2": m2, "vol": vol}
        return self._mp

    def _T(self, mp):
        Gw = mp["G"] * mp["w"][:, :, None, None]
        tmp = np.einsum("qib,cqab->cqia", self.two, Gw)
        S = np.einsum("cqia,cqam->cim", tmp, self.PT)
        return S @ self.map                                               # C x 4 x Fmax

    def gradient(self, links, mp):
        links = np.asarray(links, float)
        u = b_local_links(links, self.E, self.rev)
        D = b_defects(u)
        h = np.matmul(mp["m2"], D)
        if self.trial != "native":
            C = len(self.cells)
            W = np.zeros((C, self.Smax, 4))
            W[:, 0] = ONE
            for ci, s, ps, gid, inv in self.layers:
                W[ci, s] = compose(W[ci, ps], take(links, gid, inv))
            L = take(links, self.ygid, self.yinv)                       # entries x 3 x 4
            hol = compose(compose(L[:, 0], L[:, 1]), L[:, 2])
            Y = np.zeros((C, self.Fmax, 3))
            Y[self.ycell, self.yface] = b_ad(W[self.ycell, self.yslot], hol[:, 1:])
            hvec = np.matmul(self._T(mp), Y)                              # C x 4 x 3
            h = np.concatenate([h[:, :, :1], hvec], axis=2)
        dD = b_defect_derivatives(u)
        a_loc = 4 * np.einsum("nekic,nic->nek", dD, h)
        lift = np.where(self.rev[:, :, None, None], -rotations(links[self.E]), np.eye(3))
        contrib = np.einsum("ceji,cej->cei", lift, a_loc) * self.w[:, None, None]
        grad = np.zeros((self.n, 3))
        np.add.at(grad, self.E.ravel(), contrib.reshape(-1, 3))
        return grad

    def mass(self, links, mp):
        links = np.asarray(links, float)
        u = b_local_links(links, self.E, self.rev)
        R = rotations(u[:, [0, 1, 2, 1, 2, 2]])
        lift = np.where(self.rev[:, :, None, None], -rotations(links[self.E]), np.eye(3))
        A = np.matmul(R, lift)
        blk = np.einsum("ceji,cfjk->cefik", A, A) * (4 * self.w[:, None, None] * mp["m1"])[..., None, None]
        N = 3 * self.n
        return coo_matrix((blk.ravel(), (self.mrows, self.mcols)), shape=(N, N)).tocsr()

    def face_angles(self, links):
        Dq = b_defects(b_local_links(np.asarray(links, float), self.E, self.rev)) + ONE
        return np.arctan2(np.linalg.norm(Dq[..., 1:], axis=-1), Dq[..., 0]).ravel()


# ---------------------------------------------------------------- gates
def rel(a, b):
    a, b = np.asarray(a), np.asarray(b)
    return float(np.linalg.norm(a - b) / np.linalg.norm(b))


def parity(level, trial):
    loop, bat = FL.Lattice(level, trial), BatchedLattice(level, trial)
    met = FL.Metric()
    states = {"initial": np.asarray(loop.raw["links"], float),
              "s3b_final": np.load(REPO / "research" / "su2-cayley-20261118" / f"L{level}_pg_N8.final.npy")}
    rows = []
    for sname, U in states.items():
        for tname, t in (("g(0)", 0.0), ("g(0.02)", FL.T_END)):
            g = met(t)
            mp, mb = loop.metric_pieces(g), bat.metric_pieces(g)
            Bl, Bb = loop.mass(U, mp), bat.mass(U, mb)
            row = {"state": sname, "metric": tname,
                   "P_metric": max(rel(mb[k], mp[k]) for k in ("G", "w", "m1", "m2", "vol")),
                   "P_grad": rel(bat.gradient(U, mb), loop.gradient(U, mp)),
                   "P_mass": float(np.sqrt(abs((Bb - Bl).multiply(Bb - Bl).sum())) / np.sqrt(Bl.multiply(Bl).sum())),
                   "P_vel": rel(bat.velocity(U, g), loop.velocity(U, g)),
                   "P_angles": rel(bat.face_angles(U), loop.face_angles(U))}
            print(json.dumps(row), flush=True)
            rows.append(row)
    worst = {k: max(r[k] for r in rows) for k in ("P_metric", "P_grad", "P_mass", "P_vel", "P_angles")}
    thr = {"P_metric": 1e-13, "P_grad": 1e-12, "P_mass": 1e-13, "P_vel": 1e-10, "P_angles": 1e-13}
    return {"level": level, "trial": trial, "rows": rows, "worst": worst,
            "pass": {k: worst[k] <= thr[k] for k in thr}}


def timing(level, trial, loop_repeats=1):
    met = FL.Metric()
    g = met(0.5 * FL.T_END / 8)
    out = {"level": level, "trial": trial}
    for name, cls, reps in (("batched", BatchedLattice, 3), ("loop", FL.Lattice, loop_repeats)):
        t0 = time.time()
        lat = cls(level, trial)
        out[f"{name}_setup_s"] = round(time.time() - t0, 2)
        U = np.asarray(lat.raw["links"], float)
        ts = []
        for _ in range(reps):
            t0 = time.time()
            lat.velocity(U, g)
            ts.append(time.time() - t0)
        out[f"{name}_s_per_eval"] = round(min(ts), 3)
        print(json.dumps(out), flush=True)
    out["speedup"] = round(out["loop_s_per_eval"] / out["batched_s_per_eval"], 1)
    return out


def k_parity(level, trial):
    """B5 gates: P-K1 (n = 1, every column) or P-K2 (60 seeded columns), and P-pattern."""
    import ricci_su2_pg_stability as S2B
    t0 = time.time()
    bat = BatchedLattice(level, trial)
    N = 3 * bat.n
    if level == 0:
        idx = np.arange(N)
    else:
        idx = np.sort(np.random.default_rng(0).choice(N, size=60, replace=False))
    K = bat.build_K(columns=None if level == 0 else set((idx // 3).tolist()))
    t_b = time.time() - t0
    S = S2B.Setup(level, trial)
    rels, pattern_ok = [], True
    num = den = 0.0
    for j in idx:
        ref = S.column(int(j // 3), int(j % 3))
        b = K[:, int(j)].toarray().ravel()
        rels.append(float(np.linalg.norm(b - ref) / np.linalg.norm(ref)))
        num += float(np.sum((b - ref) ** 2)); den += float(np.sum(ref ** 2))
        thr = 1e-9 * max(np.abs(ref).max(), np.abs(b).max())   # Amendment 1
        pattern_ok &= bool(np.array_equal(np.abs(b) > thr, np.abs(ref) > thr))
    out = {"level": level, "trial": trial, "columns_checked": int(len(idx)),
           "worst_column_rel": max(rels), "median_column_rel": float(np.median(rels)),
           "matrix_rel": float(np.sqrt(num / den)), "pattern_identical": pattern_ok,
           "gate": "P-K1" if level == 0 else "P-K2",
           "pass": (max(rels) <= 1e-8) and pattern_ok,
           "batched_seconds": round(t_b, 1), "seconds_total": round(time.time() - t0, 1)}
    if level == 2:
        from scipy.sparse import load_npz
        disk = REPO / "outputs" / "su2-cayley" / f"L2_{'pg' if trial == 'pg' else trial}_K.npz"
        if disk.exists() and trial in ("native", "pg"):
            D = load_npz(disk).tocsc()[:, idx].toarray()
            out["vs_S3b_disk_K_rel"] = float(np.linalg.norm(K[:, idx].toarray() - D) / np.linalg.norm(D))
    return out


def k_build(level, trial, out_dir):
    import hashlib
    from scipy.sparse import save_npz
    t0 = time.time()
    bat = BatchedLattice(level, trial)
    t_setup = time.time() - t0
    K = bat.build_K()
    t_build = time.time() - t0 - t_setup
    out_dir.mkdir(parents=True, exist_ok=True)
    path = out_dir / f"L{level}_{trial}_K.npz"
    if path.exists():
        raise SystemExit(f"refusing to overwrite {path}")
    save_npz(path, K)
    h = hashlib.sha256(path.read_bytes()).hexdigest()
    return {"level": level, "trial": trial, "dof": 3 * bat.n, "nnz": int(K.nnz), "file": str(path.relative_to(REPO)).replace("\\", "/"),
            "sha256": h, "setup_seconds": round(t_setup, 1), "build_seconds": round(t_build, 1),
            "metric": "g(0) = (3, 1, 1)", "eps": 1e-6, "kind": "B5 batched Jacobian (research script, not an engine run)"}


def main():
    mode = sys.argv[1]
    level, trial = int(sys.argv[2]), sys.argv[3]
    if mode in ("kparity", "kbuild"):
        receipt = Path(sys.argv[4]) if len(sys.argv) > 4 else None
        if receipt and receipt.exists():
            raise SystemExit(f"refusing to overwrite {receipt}")
        out = k_parity(level, trial) if mode == "kparity" else \
            k_build(level, trial, REPO / "outputs" / "su2-batched-K")
        print(json.dumps(out), flush=True)
        if receipt:
            receipt.parent.mkdir(parents=True, exist_ok=True)
            receipt.write_text(json.dumps(out, indent=1) + "\n")
        return
    if mode == "parity":
        receipt = Path(sys.argv[4]) if len(sys.argv) > 4 else None
        if receipt and receipt.exists():
            raise SystemExit(f"refusing to overwrite {receipt}")
        out = parity(level, trial)
        print(json.dumps({k: out[k] for k in ("level", "trial", "worst", "pass")}), flush=True)
        if receipt:
            receipt.parent.mkdir(parents=True, exist_ok=True)
            receipt.write_text(json.dumps(out, indent=1) + "\n")
    elif mode == "time":
        print(json.dumps(timing(level, trial, int(sys.argv[4]) if len(sys.argv) > 4 else 1)), flush=True)
    elif mode == "run":
        import ricci_su2_pg_flow_cayley as CAY
        N = int(sys.argv[4])
        receipt = Path(sys.argv[5]) if len(sys.argv) > 5 else None
        if receipt and receipt.exists():
            raise SystemExit(f"refusing to overwrite {receipt}")
        CAY.FL.Lattice = BatchedLattice                    # the Cayley step, unchanged, on the batched force
        out = CAY.run(level, trial, N)
        final = out.pop("_final")
        out["force"] = "batched (scripts/ricci_su2_batched.py)"
        print(json.dumps({k: v for k, v in out.items() if k != "rows"}), flush=True)
        if receipt:
            receipt.parent.mkdir(parents=True, exist_ok=True)
            np.save(receipt.with_suffix(".final.npy"), final)
            receipt.write_text(json.dumps(out, indent=1) + "\n")


if __name__ == "__main__":
    main()
