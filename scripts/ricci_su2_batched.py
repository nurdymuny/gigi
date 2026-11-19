"""The SU(2) force of S3/S3b, batched across cells. Spec: docs/RICCI_SU2_BATCHED_FORCE_SPEC.md.
Research script, not an engine verb. The per-cell loop in ricci_su2_pg_flow.Lattice is the reference.

usage:
  python scripts/ricci_su2_batched.py parity LEVEL TRIAL [RECEIPT.json]
  python scripts/ricci_su2_batched.py time LEVEL TRIAL [LOOP_REPEATS]
  python scripts/ricci_su2_batched.py run LEVEL TRIAL N [RECEIPT.json]   (S3b Cayley step, batched force)
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
            mp = loop.metric_pieces(g)
            Bl, Bb = loop.mass(U, mp), bat.mass(U, mp)
            row = {"state": sname, "metric": tname,
                   "P_grad": rel(bat.gradient(U, mp), loop.gradient(U, mp)),
                   "P_mass": float(np.sqrt(abs((Bb - Bl).multiply(Bb - Bl).sum())) / np.sqrt(Bl.multiply(Bl).sum())),
                   "P_vel": rel(bat.velocity(U, g), loop.velocity(U, g)),
                   "P_angles": rel(bat.face_angles(U), loop.face_angles(U))}
            print(json.dumps(row), flush=True)
            rows.append(row)
    worst = {k: max(r[k] for r in rows) for k in ("P_grad", "P_mass", "P_vel", "P_angles")}
    thr = {"P_grad": 1e-12, "P_mass": 1e-13, "P_vel": 1e-10, "P_angles": 1e-13}
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


def main():
    mode = sys.argv[1]
    level, trial = int(sys.argv[2]), sys.argv[3]
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
