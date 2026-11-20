"""M0: a seamless moving-least-squares (MLS) 1-form reading, k = 1, and its energy's force.
Spec: docs/MAGNET_M0_SPEC.md (with Amendment 1). Research script, not an engine run. Abelian control, q6.

Arms
  unseated  E = 1/2 sum_K w_K int_K |curl R1 a|^2            (curl with the full MLS derivative)
  seated    E + 1/2 sigma sum_K w_K r_K^T G^W_K r_K           (M5: own-wall flux misfit, Whitney-stiff, sigma = 1)
  frozen    MLS fit frozen at each cell's centroid (seamed removal arm), unseated energy

Reading. At chart point x of cell K (K's gnomonic chart, the published one), fit the 12-coefficient linear 1-form
p_c minimising sum_e w_e(x) (int_e p_c - a_e)^2 over the edges near x. Weights are chart-free (Amendment 1):
w_e(x) = phi(|X(x) - m_e| / rho), X(x) the point of S^3 under x, m_e the normalised R^4 midpoint of e on K's sheet,
phi the Wendland C2 function, rho = kappa * h_max. kappa is the smallest of {1, 1.25, 1.5, 2, 2.5, 3} giving at least
24 weighted edges and a weighted-Vandermonde condition number <= 1e8 at every quadrature point.

usage: python scripts/ricci_magnet_m0.py LEVEL OUTDIR [--kappa K] [--no-eig]
"""
import json
import sys
import time
from pathlib import Path

import numpy as np
from scipy.linalg import eigh
from functools import partial

ein = partial(np.einsum, optimize=True)

HERE = Path(__file__).resolve().parent
sys.path[:0] = [str(HERE), str(HERE.parent / "e2e/probes")]
import ricci_variational_v1 as V                 # noqa: E402  (sets pg.BASE from GIGI_RICCI_DATA_ROOT)
from ricci_consistency_math import solve_rhs     # noqa: E402

pg, tw = V.pg, V.tw
ORDER = V.ORDER
NCOEF = 12
KAPPAS = (1.0, 1.25, 1.5, 2.0, 2.5, 3.0)
VOTE = 2 * NCOEF
COND_MAX = 1e8
SIGMA = 1.0
EPS = np.zeros((3, 3, 3))
for _i, _j, _k in ((0, 1, 2), (1, 2, 0), (2, 0, 1)):
    EPS[_i, _j, _k], EPS[_j, _i, _k] = 1., -1.          # EPS[i, j, k]: (curl w)_k = EPS[i,j,k] d_i w_j
GL_T, GL_W = (np.array([0.5 - np.sqrt(3) / 6, 0.5 + np.sqrt(3) / 6]), np.array([0.5, 0.5]))   # exact for deg <= 3


def wendland(r):
    r = np.minimum(r, 1.0)
    return (1 - r) ** 4 * (4 * r + 1), -20 * r * (1 - r) ** 3


# ---- the chart (same construction as pg.chart, with its inverse) ----------------------------------
class Chart:
    def __init__(self, K):
        p0, b = K[0], (K[1:] - K[0]).T
        n = np.linalg.svd(b.T)[2][-1]
        c = float(n @ p0)
        if c < 0:
            n, c = -n, -c
        Q, _ = np.linalg.qr(b)
        cen = K.mean(axis=0); cen = cen * (c / (cen @ n))
        diam = max(np.linalg.norm(K[i] - K[j]) for i in range(4) for j in range(i + 1, 4))
        self.n, self.c, self.Q, self.cen, self.diam = n, c, Q, cen, diam
        self.J = (Q.T @ b) / diam
        self.t0 = ((p0 - cen) @ Q) / diam

    def to(self, X):                              # R^4 point(s) -> chart
        X = np.atleast_2d(X)
        return ((X * (self.c / (X @ self.n))[:, None] - self.cen) @ self.Q) / self.diam

    def push(self, X, t):                         # tangent t at X -> chart vector (gnomonic derivative)
        den = X @ self.n
        dy = (self.c / den) * t - (self.c * (self.n @ t) / den ** 2) * X
        return (dy @ self.Q) / self.diam

    def lift(self, x):                            # chart point(s) -> (X on S^3, dX/dx) ; x (P,3)
        y = self.cen[None] + self.diam * x @ self.Q.T
        ny = np.linalg.norm(y, axis=1)
        X = y / ny[:, None]
        dXdx = ein("pab,bk->pak", (np.eye(4)[None] - X[:, :, None] * X[:, None, :]) / ny[:, None, None],
                         self.diam * self.Q)
        return X, dXdx


def basis(x):
    """B (P,3,12) and dB (P,3(i),3(j),12): 1-form basis e_j * {1, x, y, z}; dB[p,i,j,m] = d_i B[p,j,m]."""
    P = x.shape[0]
    mono = np.hstack([np.ones((P, 1)), x])                  # (P,4)
    B = np.zeros((P, 3, NCOEF)); dB = np.zeros((P, 3, 3, NCOEF))
    for j in range(3):
        for t in range(4):
            m = 4 * j + t
            B[:, j, m] = mono[:, t]
            if t > 0:
                dB[:, t - 1, j, m] = 1.0
    return B, dB


def line_rows(Pc, Qc):
    """Exact line integrals of the 12 basis 1-forms along chart segments Pc->Qc: (E,12)."""
    out = np.zeros((len(Pc), NCOEF)); d = Qc - Pc
    for t, w in zip(GL_T, GL_W):
        B, _ = basis(Pc + t * d)
        out += w * ein("ejm,ej->em", B, d)
    return out


def rings_iter(cells, faces, ci):
    """Face-ring neighbourhoods of cell ci, one ring at a time; same transport and coincidence check as
    scripts/ricci_pg_tworing.neighbourhood_rings (first sheet per cell wins)."""
    out = {ci: np.array(cells[ci]["vertices"], float)}
    frontier = [ci]
    while frontier:
        fresh = []
        for cj in frontier:
            Cj = out[cj]
            for face in pg.FACES:
                for nj, fn in faces[pg.face_key(cells[cj], face)]:
                    if nj == cj or nj in out:
                        continue
                    ck, cn = pg.corners(cells[cj], face), pg.corners(cells[nj], fn)
                    Vn = np.array(cells[nj]["vertices"], float)
                    k0 = next(iter(ck))
                    h = pg.qmul(Cj[ck[k0]], pg.qconj(Vn[cn[k0]]))
                    Vn = np.array([pg.qmul(h, v) for v in Vn])
                    gap = max(float(np.max(abs(Cj[ck[k]] - Vn[cn[k]]))) for k in ck)
                    if gap > pg.COINCIDE:
                        raise ValueError(f"cell {nj} does not coincide with {cj} after transport ({gap:.3e})")
                    out[nj] = Vn
                    fresh.append(nj)
        frontier = fresh
        yield out


# ---- per cell -------------------------------------------------------------------------------------
class Cell:
    def __init__(self, cells, faces, ci, rho):
        c = cells[ci]
        self.ci, self.weight = ci, float(c["weight"])
        self.verts = np.array(c["vertices"], float)
        self.ch = Chart(self.verts)
        cen4 = self.verts.mean(0); cen4 /= np.linalg.norm(cen4)
        radius = rho + self.ch.diam
        # grow face-rings one at a time (first sheet per cell, BFS order = nearest), keep every distinct copy
        # of every edge, stop when the set of copies within `radius` of K's centroid stops changing
        prev, rings = None, 0
        for rings, nb in enumerate(rings_iter(cells, faces, ci), start=1):
            rows = []
            for nj, Vn in nb.items():
                sg = np.where(cells[nj]["reverse"], -1., 1.)
                for k, (p, q) in enumerate(pg.EDGES):
                    st, en = (Vn[p], Vn[q]) if sg[k] > 0 else (Vn[q], Vn[p])
                    m = st + en
                    if np.linalg.norm(m / np.linalg.norm(m) - cen4) <= radius:
                        rows.append((cells[nj]["edges"][k], st, en, nj == ci))
            uniq = {}
            for g, st, en, isown in rows:            # dedupe copies by position
                key = (g, tuple(np.round(st, 9)), tuple(np.round(en, 9)))
                if key not in uniq or isown:
                    uniq[key] = (g, st, en, isown or uniq.get(key, (0, 0, 0, False))[3])
            near = set(uniq)
            if (prev is not None and near == prev) or rings >= 12:
                break
            prev = near
        rows = sorted(uniq.values(), key=lambda r: (r[0], tuple(r[1])))
        self.rings = rings
        gs = [r[0] for r in rows]
        self.copies = int(len(gs) - len(set(gs)))                         # edges seen at two positions
        self.edges = sorted(set(gs))
        self.col = {g: i for i, g in enumerate(self.edges)}
        self.Psel = np.zeros((len(rows), len(self.edges)))
        self.Psel[np.arange(len(rows)), [self.col[g] for g in gs]] = 1.0
        st = np.array([r[1] for r in rows]); en = np.array([r[2] for r in rows])
        mid = st + en; self.mid = mid / np.linalg.norm(mid, axis=1)[:, None]
        self.Vrow = line_rows(self.ch.to(st), self.ch.to(en))            # (R,12) exact chart line integrals
        self.own = [self.col[g] for g in c["edges"]]                       # own edges, in K's edge order
        s_own = np.where(c["reverse"], -1., 1.)
        Pk = [(self.verts[p], self.verts[q]) if s_own[k] > 0 else (self.verts[q], self.verts[p])
              for k, (p, q) in enumerate(pg.EDGES)]
        self.ownP = self.ch.to(np.array([u for u, _ in Pk])); self.ownQ = self.ch.to(np.array([v for _, v in Pk]))
        q = pg.quadrature(self.verts[None], ORDER)
        self.g, self.w = q["g"][0], q["measure"][0] / q["det"][0]
        self.xq = q["xref"] @ self.ch.J.T + self.ch.t0
        self.two = q["two"]
        self.Pex = pg.exact_proxy_from_maurer(q["maurer"])[0]
        s = np.where(c["reverse"], -1., 1.)
        self.TW = pg.INCIDENCE * s[None, :]
        self.GW = ein("qfa,qab,qgb,q->fg", self.two, self.g, self.two, self.w)
        self.piola = np.linalg.det(self.ch.J) * np.linalg.inv(self.ch.J)
        self.xc = (np.full(3, 0.25) @ self.ch.J.T + self.ch.t0)[None]

    # MLS at chart points x (P,3): returns S (P,12,E) and dS (P,3,12,E), plus counts / conditions
    def mls(self, x, rho, deriv=True, raw=False):
        X, dXdx = self.ch.lift(x)
        diff = X[:, None, :] - self.mid[None]                             # (P,E,4)
        d = np.linalg.norm(diff, axis=2)
        phi, dphi = wendland(d / rho)
        Vr = self.Vrow
        G = ein("em,pe,en->pmn", Vr, phi, Vr)
        Gi = np.linalg.inv(G)
        A = Gi @ Vr.T[None]                                               # (P,12,E) = G^-1 V^T
        S = A * phi[:, None, :]                                           # (P,12,E) = G^-1 V^T W
        info = ((phi > 0).sum(1), np.linalg.cond(G))
        if not deriv:
            return (S if raw else S @ self.Psel), None, info
        dd = ein("pea,pak->pek", diff, dXdx) / np.maximum(d, 1e-300)[:, :, None]
        dw = (dphi / rho)[:, :, None] * dd                                # (P,E,3)  d w_e / d x_i
        Aw = A[:, None, :, :] * dw.transpose(0, 2, 1)[:, :, None, :]     # (P,3,12,E) = G^-1 V^T W_i
        dS = Aw - (Aw @ Vr[None, None]) @ S[:, None]                      # G^-1 V^T W_i (I - V S)
        if raw:
            return S, dS, info
        return S @ self.Psel, dS @ self.Psel, info

    def curl_map(self, x, rho, frozen=False):
        """(P,3,E): chart curl of the reading as a linear map of the cell's edge data."""
        B, dB = basis(x)
        if frozen:
            S0, _, _ = self.mls(self.xc, rho, deriv=False)
            return ein("ijk,pijm,me->pke", EPS, dB, S0[0])
        S, dS, _ = self.mls(x, rho)
        return ein("ijk,pijm,pme->pke", EPS, dB, S) + ein("ijk,pjm,pime->pke", EPS, B, dS)

    def reading_map(self, x, rho, frozen=False):
        B, _ = basis(x)
        if frozen:
            S0, _, _ = self.mls(self.xc, rho, deriv=False)
            return ein("pjm,me->pje", B, S0[0])
        S, _, _ = self.mls(x, rho, deriv=False)
        return ein("pjm,pme->pje", B, S)

    def own_line_map(self, rho, frozen=False):
        """(6,E): line integrals of the reading along the cell's own edges (global orientation)."""
        out = np.zeros((6, len(self.edges)))
        d = self.ownQ - self.ownP
        for t, w in zip(GL_T, GL_W):
            Rm = self.reading_map(self.ownP + t * d, rho, frozen)        # (6,3,E)
            out += w * ein("kje,kj->ke", Rm, d)
        return out


# ---- parallel workers (spawn: fresh processes, no fork; Accelerate is not fork-safe on macOS) -------------
import multiprocessing as _mp
import os as _os
_W = {}
NPROC = int(_os.environ.get("M0_PROCS", "8"))
ARMS = ("unseated", "frozen", "seated")
BARY = np.array([[2 / 3, 1 / 6, 1 / 6], [1 / 6, 2 / 3, 1 / 6], [1 / 6, 1 / 6, 2 / 3]])


def _w_init(level, rho):
    _os.environ.setdefault("OMP_NUM_THREADS", "1")
    L = pg.load_level(level, ORDER)
    _W.update(cells=L["cells"], a=np.asarray(L["a"]), faces=pg.face_index(L["cells"]), rho=rho)


def _wall_points(c, cells, ci, f):
    """This cell's side of a wall: points and tangents in its own sheet, keyed by the wall's global vertices."""
    cr = pg.corners(cells[ci], f)
    common = sorted(cr, key=lambda g: tuple(sorted(g)) if isinstance(g, frozenset) else (g,))   # a total order
    Vv = np.array([c.verts[cr[g]] for g in common])
    for b, lam in enumerate(BARY):
        X = lam @ Vv; X /= np.linalg.norm(X)
        for t, (i, j) in enumerate(((0, 1), (0, 2))):
            tg = Vv[j] - Vv[i]; tg -= (tg @ X) * X
            yield (pg.face_key(cells[ci], f), b, t), X, tg


def _w_cell(ci):
    cells, a, faces, rho = _W["cells"], _W["a"], _W["faces"], _W["rho"]
    c = Cell(cells, faces, ci, rho)
    _, _, (n_, k_) = c.mls(c.xq, rho, deriv=False)
    out = {"ci": ci, "edges": c.edges, "own": [c.edges[i] for i in c.own], "rings": c.rings, "copies": c.copies,
           "ncand": len(c.edges), "vote": int(n_.min()), "cond": float(k_.max()), "arms": {}}
    uF = ein("qfa,qab,qb,q->f", c.two, c.g, c.Pex, c.w)
    cm_seamless = None
    for arm in ARMS:
        if arm == "frozen":
            Cm = c.curl_map(c.xq, rho, True)
        else:                                   # unseated and seated share the same seamless curl map
            if cm_seamless is None:
                cm_seamless = c.curl_map(c.xq, rho, False)
            Cm = cm_seamless
        Pr = ein("ab,pbe->pae", c.piola, Cm)
        wt = c.weight
        P = ein("pae,e->pa", Pr, a[c.edges]); diff = P - c.Pex
        r = {"K": wt * ein("pae,pab,pbf,p->ef", Pr, c.g, Pr, c.w),
             "rec": wt * ein("pae,pab,pb,p->e", Pr, c.g, diff, c.w),
             "tv": wt * ein("pae,pab,pb,p->e", Pr, c.g, c.Pex, c.w),
             "tvown": wt * (c.TW.T @ uF),
             "cn": wt * float(ein("pa,pab,pb,p->", diff, c.g, diff, c.w)),
             "cd": wt * float(ein("pa,pab,pb,p->", c.Pex, c.g, c.Pex, c.w))}
        if arm == "seated":
            Lm = c.own_line_map(rho)
            Pown = np.zeros_like(Lm); Pown[np.arange(6), c.own] = 1.0
            Rr = c.TW @ (Lm - Pown)
            r["Ks"] = SIGMA * wt * Rr.T @ c.GW @ Rr
        out["arms"][arm] = r
    seam = []
    for f in pg.FACES:
        for key, X, tg in _wall_points(c, cells, ci, f):
            x = c.ch.to(X); v = c.ch.push(X, tg)
            seam.append((key, float(c.reading_map(x, rho)[0] @ a[c.edges] @ v)))
    out["seam"] = seam
    return out


def main():
    args = sys.argv[1:]
    kappa_fixed = None; do_eig = "--no-eig" not in args
    if not do_eig:
        args.remove("--no-eig")
    if "--kappa" in args:
        i = args.index("--kappa"); kappa_fixed = float(args[i + 1]); del args[i:i + 2]
    level, outdir = int(args[0]), Path(args[1])
    receipt = outdir / f"L{level}.json"
    if receipt.exists():
        raise SystemExit(f"refusing to overwrite {receipt}")
    outdir.mkdir(parents=True, exist_ok=True)
    t00 = time.time()
    L = pg.load_level(level, ORDER)
    cells, a, mass, vol = L["cells"], np.asarray(L["a"]), L["mass"], L["volume"]
    faces = pg.face_index(cells); ne = len(a)
    hmax = json.loads((V._DATA / "output/ricci-variational-20261117/abelian/diagnostic.json").read_text()
                      )["cases"][f"level{level}_q6"]["h_max"]
    out = {"level": level, "edges": ne, "cells": len(cells), "h_max": hmax, "k": 1, "workers": NPROC,
           "kind": "M0 (research script, not an engine run)"}
    norm = lambda v: float(np.sqrt(max(0., v @ (mass @ v)) / vol))     # noqa: E731
    r4 = norm(-4 * a)
    rel = lambda K: norm(solve_rhs(mass, -(K @ a)) + 4 * a) / r4         # noqa: E731

    # ---- one parallel pass per reach: every cell's arms, reach statistics and wall readings ----
    for kappa in ([kappa_fixed] if kappa_fixed else KAPPAS):
        rho = kappa * hmax
        Kv = {arm: np.zeros((ne, ne)) for arm in ARMS}; Ks = np.zeros((ne, ne))
        rec = {arm: np.zeros(ne) for arm in ARMS}; tv = {arm: np.zeros(ne) for arm in ARMS}
        cnum = {arm: 0.0 for arm in ARMS}; cden = {arm: 0.0 for arm in ARMS}
        cnt, cond, rings, copies, ncand, seam = [], [], [], [], [], {}
        ctx = _mp.get_context("spawn")
        with ctx.Pool(NPROC, initializer=_w_init, initargs=(level, rho)) as pool:
            for k, r in enumerate(pool.imap_unordered(_w_cell, range(len(cells)), chunksize=2)):
                e = r["edges"]; ix = np.ix_(e, e)
                for arm in ARMS:
                    p = r["arms"][arm]
                    Kv[arm][ix] += p["K"]
                    if "Ks" in p:
                        Ks[ix] += p["Ks"]
                    rec[arm][e] += p["rec"]; tv[arm][e] += p["tv"]; tv[arm][r["own"]] -= p["tvown"]
                    cnum[arm] += p["cn"]; cden[arm] += p["cd"]
                cnt.append(r["vote"]); cond.append(r["cond"]); rings.append(r["rings"])
                copies.append(r["copies"]); ncand.append(r["ncand"])
                for key, val in r["seam"]:
                    seam.setdefault(key, []).append(val)
                if (k + 1) % 200 == 0:
                    print(f"  {k + 1}/{len(cells)} cells, {time.time() - t00:.0f} s", flush=True)
        ok = min(cnt) >= VOTE and max(cond) <= COND_MAX
        print(f"kappa {kappa}: min weighted edges {min(cnt)}, max cond {max(cond):.2e}, ok {ok}", flush=True)
        if ok or kappa_fixed:
            break
    out.update({"kappa": kappa, "rho": rho, "vote_min_weighted_edges": int(min(cnt)), "vote_required": VOTE,
                "cond_max": float(max(cond)), "reach_ok": bool(ok),
                "rings_median_max": [float(np.median(rings)), int(max(rings))],
                "candidate_edges_median": float(np.median(ncand)),
                "C_copies_cells": int(sum(x > 0 for x in copies)),
                "seconds_cells_and_arms": round(time.time() - t00, 1)})

    # ---- controls, on a sample of cells built here ----
    t1 = time.time()
    sample = list(range(0, len(cells), max(1, len(cells) // 25)))
    rng = np.random.default_rng(0)
    dsample = sorted(rng.choice(len(cells), size=min(20, len(cells)), replace=False).tolist())
    Cs = {ci: Cell(cells, faces, ci, rho) for ci in sorted(set(sample) | set(dsample))}
    cp = 0.0; co = 0.0
    for ci in sample:
        c = Cs[ci]
        S, dS, _ = c.mls(c.xq, rho, raw=True)
        cp = max(cp, float(np.abs(ein("pme,en->pmn", S, c.Vrow) - np.eye(NCOEF)[None]).max()),
                 float(np.abs(ein("pime,en->pimn", dS, c.Vrow)).max()))
        for fi, f in enumerate(pg.FACES):
            ref = pg.data_flux(cells[c.ci], a, f)
            co = max(co, abs(float(c.TW[fi] @ a[[c.edges[i] for i in c.own]]) - ref))
    cd = 0.0
    for ci in dsample:
        c = Cs[ci]
        x = c.xq[rng.integers(len(c.xq))][None]; h = 1e-5
        an = c.curl_map(x, rho)[0] @ a[c.edges]
        Jm = np.zeros((3, 3))
        for i in range(3):
            e_ = np.zeros(3); e_[i] = h
            Jm[i] = ((c.reading_map(x + e_, rho)[0] - c.reading_map(x - e_, rho)[0]) @ a[c.edges]) / (2 * h)
        fd = ein("ijk,ij->k", EPS, Jm)
        cd = max(cd, float(np.linalg.norm(an - fd) / max(np.linalg.norm(an), 1e-300)))
    out.update({"C_poly_max": cp, "C_orient_max": co, "C_deriv_max_rel": cd, "seconds_controls": round(time.time() - t1, 1)})
    print("controls", cp, co, cd, flush=True)

    # ---- the three arms ----
    arms = {}
    for arm in ARMS:
        K = Kv[arm] + (Ks if arm == "seated" else 0.0)
        res = {"bw05_relative_error": rel(K),
               "reconstruction_term": norm(solve_rhs(mass, rec[arm])) / r4,
               "poke_term": norm(solve_rhs(mass, tv[arm])) / r4,
               "curvature_relative_error": float(np.sqrt(cnum[arm] / cden[arm])),
               "asymmetry": float(np.abs(K - K.T).max() / np.abs(K).max())}
        if arm == "seated":
            res["spring_force_share"] = norm(solve_rhs(mass, Ks @ a)) / r4
        arms[arm] = (res, K)
        print(arm, json.dumps(res), flush=True)
    del Kv

    # ---- wobble ----
    if do_eig:
        t2 = time.time()
        M = mass.toarray() if hasattr(mass, "toarray") else np.asarray(mass)
        Kw = L["stiffness"].toarray() if hasattr(L["stiffness"], "toarray") else np.asarray(L["stiffness"])
        lw, Vw = eigh(Kw, M)
        gmask = np.abs(lw) < 1
        Pp, Pg = Vw[:, ~gmask], Vw[:, gmask]
        out["gauge_dimension"] = int(gmask.sum())
        for arm, (res, K) in arms.items():
            Ksym = 0.5 * (K + K.T)
            lam = np.linalg.eigvalsh(Pp.T @ Ksym @ Pp)
            lg = np.linalg.eigvalsh(Pg.T @ Ksym @ Pg)
            res.update({"wobble_count_below_1": int((lam < 1).sum()), "min_physical_eig": float(lam.min()),
                        "negative_physical": int((lam < -1e-8 * abs(lam).max()).sum()),
                        "gauge_rayleigh_max": float(lg.max()),
                        "gauge_rayleigh_over_min_physical": float(lg.max() / max(lam[lam >= 1].min(), 1e-300))
                        if (lam >= 1).any() else None})
            print(arm, "eig", res["wobble_count_below_1"], res["min_physical_eig"], res["gauge_rayleigh_max"], flush=True)
        out["seconds_eig"] = round(time.time() - t2, 1)

    # ---- seam: pair the two sides of every wall ----
    jumps, vals = [], []
    for key, vs in seam.items():
        if len(vs) == 2:
            jumps.append(vs[0] - vs[1]); vals.extend(vs)
    out["seam_rms_jump_relative"] = float(np.sqrt(np.mean(np.square(jumps))) / np.sqrt(np.mean(np.square(vals))))
    out["seam_points"] = len(jumps)
    print("seam", out["seam_rms_jump_relative"], len(jumps), flush=True)

    out["arms"] = {arm: res for arm, (res, _) in arms.items()}
    out["seconds"] = round(time.time() - t00, 1)
    receipt.write_text(json.dumps(out, indent=1) + "\n")
    print("wrote", receipt, flush=True)


if __name__ == "__main__":
    main()
