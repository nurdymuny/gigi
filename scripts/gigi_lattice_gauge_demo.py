"""A lattice gauge field stored in GIGI as a fibre bundle, read by holonomy (docs/research/helike-bundle-framing-math-2026-11-27.md).

Base: the oriented edges of an L x L lattice (open boundary). Fibre: SU(2), stored as a unit quaternion (a0, a1, a2, a3),
U = a0 + i a.sigma. A configuration is a section; several configurations are stored side by side (field cfg).

Checks (each against a planted, exact answer):
  1. Area law: <W(R x T)> = (I2(beta) / I1(beta))^{R T} exactly, for 2-D SU(2) with the Wilson action
     S = beta sum_p (1 - tr U_p / 2) (the theorem in the note). Wilson loops are computed ONLY from GIGI point queries
     (SECTION AT) along each loop, multiplied in order.
  2. Gauge invariance: every stored section is rewritten by a random gauge transformation U_xy -> g_x U_xy g_y^-1
     (SECTIONS ... UPSERT). Every Wilson loop must be unchanged to rounding; a single link (a "point query" of the
     connection) must change.
Configurations are sampled exactly: maximal-tree gauge (vertical links and the bottom row set to 1), independent
plaquette variables with density exp(beta a0) on SU(2) Haar measure, then scrambled by a random gauge transformation.
usage: python scripts/gigi_lattice_gauge_demo.py [--port 3141] [--L 10] [--beta 4.0] [--configs 24] OUT.json"""
import argparse
import json
import math
import time
import urllib.error
import urllib.request

import numpy as np
from scipy.special import iv

BUNDLE = "lattice_su2"


def qmul(p, q):
    a0, a = p[0], p[1:]
    b0, b = q[0], q[1:]
    return np.concatenate(([a0 * b0 - a @ b], a0 * b + b0 * a - np.cross(a, b)))


def qinv(p):
    return np.concatenate(([p[0]], -p[1:]))


def sample_plaquette(beta, rng):
    """a0 with density exp(beta a0) sqrt(1 - a0^2) on [-1, 1] (Creutz), then a uniform direction."""
    while True:
        a0 = 1 + math.log(rng.uniform(math.exp(-2 * beta), 1.0)) / beta       # exp(beta a0) on [-1, 1]
        if rng.uniform() < math.sqrt(max(0.0, 1 - a0 * a0)):
            break
    v = rng.normal(size=3); v /= np.linalg.norm(v)
    return np.concatenate(([a0], math.sqrt(max(0.0, 1 - a0 * a0)) * v))


def random_su2(rng):
    q = rng.normal(size=4)
    return q / np.linalg.norm(q)


def sample_config(L, beta, rng):
    """links[(x, y, mu)] for mu = 0 (to x+1) and mu = 1 (to y+1), exact 2-D distribution, then gauge-scrambled."""
    one = np.array([1.0, 0, 0, 0])
    U = {}
    for x in range(L):
        for y in range(L - 1):
            U[(x, y, 1)] = one
    for x in range(L - 1):
        U[(x, 0, 0)] = one
    for y in range(L - 1):
        for x in range(L - 1):
            # plaquette at (x, y): U_p = U(x,y,0) U(x+1,y,1) U(x,y+1,0)^-1 U(x,y,1)^-1 = U(x,y,0) U(x,y+1,0)^-1
            Up = sample_plaquette(beta, rng)
            U[(x, y + 1, 0)] = qmul(qinv(Up), U[(x, y, 0)])
    return gauge(U, L, rng)


def gauge(U, L, rng):
    g = {(x, y): random_su2(rng) for x in range(L) for y in range(L)}
    out = {}
    for (x, y, mu), u in U.items():
        x2, y2 = (x + 1, y) if mu == 0 else (x, y + 1)
        out[(x, y, mu)] = qmul(qmul(g[(x, y)], u), qinv(g[(x2, y2)]))
    return out


def key(cfg, x, y, mu, L):
    return ((cfg * L + x) * L + y) * 2 + mu


class Gigi:
    """GQL for schema and point queries (SECTION ... AT), REST for batch writes (insert / upsert)."""
    def __init__(self, port):
        self.base = f"http://127.0.0.1:{port}/v1"
        self.queries = 0

    def post(self, path, body):
        req = urllib.request.Request(self.base + path, data=json.dumps(body).encode(), headers={"Content-Type": "application/json"})
        try:
            with urllib.request.urlopen(req) as r:
                return json.loads(r.read())
        except urllib.error.HTTPError as e:
            return {"error": e.read().decode()}

    def gql(self, q):
        return self.post("/gql", {"query": q})

    def link(self, cfg, x, y, mu, L):
        """A point query on the bundle: the fibre over one base point (one edge)."""
        self.queries += 1
        rec = self.gql(f"SECTION {BUNDLE} AT id={key(cfg, x, y, mu, L)};")["rows"][0]
        return np.array([rec["a0"], rec["a1"], rec["a2"], rec["a3"]], dtype=float)


def record(c, x, y, mu, u, L):
    return {"id": key(c, x, y, mu, L), "cfg": c, "x": x, "y": y, "mu": mu,
            "a0": float(u[0]), "a1": float(u[1]), "a2": float(u[2]), "a3": float(u[3])}


def store(db, cfgs, L, upsert=False):
    recs = [record(c, x, y, mu, u, L) for c, U in enumerate(cfgs) for (x, y, mu), u in U.items()]
    if not upsert:
        for i in range(0, len(recs), 1000):
            r = db.post(f"/bundles/{BUNDLE}/insert", {"records": recs[i:i + 1000]})
            if "error" in r:
                raise RuntimeError(r)
    else:
        for rec in recs:                       # rewrite every stored section in place
            r = db.post(f"/bundles/{BUNDLE}/upsert", {"record": rec})
            if "error" in r:
                raise RuntimeError(r)


def wilson(db, cfg, x0, y0, R, T, L):
    """tr(Hol)/2 around the R x T rectangle with corner (x0, y0), counter-clockwise, from GIGI point queries."""
    h = np.array([1.0, 0, 0, 0])
    for i in range(R):
        h = qmul(h, db.link(cfg, x0 + i, y0, 0, L))
    for j in range(T):
        h = qmul(h, db.link(cfg, x0 + R, y0 + j, 1, L))
    for i in reversed(range(R)):
        h = qmul(h, qinv(db.link(cfg, x0 + i, y0 + T, 0, L)))
    for j in reversed(range(T)):
        h = qmul(h, qinv(db.link(cfg, x0, y0 + j, 1, L)))
    return h[0]                                       # tr(U)/2 = a0


def loops(db, n_cfg, L, sizes):
    out = {}
    for R, T in sizes:
        vals = [wilson(db, c, x, y, R, T, L) for c in range(n_cfg)
                for x in range(0, L - R, R) for y in range(0, L - T, T)]   # disjoint tiles: independent samples
        out[f"{R}x{T}"] = vals
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out")
    ap.add_argument("--port", type=int, default=3147)
    ap.add_argument("--L", type=int, default=10)
    ap.add_argument("--beta", type=float, default=4.0)
    ap.add_argument("--configs", type=int, default=24)
    a = ap.parse_args()
    rng = np.random.default_rng(20261127)
    db = Gigi(a.port)
    db.gql(f"COLLAPSE {BUNDLE};")
    r = db.gql(f"BUNDLE {BUNDLE} BASE (id NUMERIC) FIBER (cfg NUMERIC, x NUMERIC, y NUMERIC, mu NUMERIC, "
               f"a0 NUMERIC, a1 NUMERIC, a2 NUMERIC, a3 NUMERIC);")
    cfgs = [sample_config(a.L, a.beta, rng) for _ in range(a.configs)]
    t0 = time.time()
    store(db, cfgs, a.L)
    sizes = [(1, 1), (1, 2), (2, 2), (2, 3), (3, 3)]
    before = loops(db, a.configs, a.L, sizes)
    ratio = iv(2, a.beta) / iv(1, a.beta)
    area_law = {}
    for (R, T) in sizes:
        v = np.array(before[f"{R}x{T}"])
        area_law[f"{R}x{T}"] = {"measured": float(v.mean()), "stderr": float(v.std(ddof=1) / math.sqrt(len(v))),
                                "exact": ratio ** (R * T), "n": len(v),
                                "z": float((v.mean() - ratio ** (R * T)) / (v.std(ddof=1) / math.sqrt(len(v))))}
    # gauge invariance: rewrite every stored section, re-read every loop
    probe_link_before = db.link(0, 3, 3, 0, a.L)
    store(db, [gauge(U, a.L, rng) for U in cfgs], a.L, upsert=True)
    after = loops(db, a.configs, a.L, sizes)
    probe_link_after = db.link(0, 3, 3, 0, a.L)
    inv = max(abs(p - q) for k in before for p, q in zip(before[k], after[k]))
    rep = {"L": a.L, "beta": a.beta, "configs": a.configs, "exact_ratio_I2_over_I1": ratio,
           "string_tension_lattice": -math.log(ratio), "area_law": area_law,
           "gauge_invariance_max_loop_change": inv,
           "single_link_change": float(np.linalg.norm(probe_link_after - probe_link_before)),
           "gigi_point_queries": db.queries, "seconds": time.time() - t0}
    open(a.out, "w").write(json.dumps(rep, indent=1) + "\n")
    print(json.dumps({k: v for k, v in rep.items()}, indent=1))


if __name__ == "__main__":
    main()
