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
    out = {"level": level, "edges": ne, "cells": len(cells), "h_max": hmax, "k": 1,
           "kind": "M0 (research script, not an engine run)"}

    # ---- reach: smallest kappa meeting the vote and the conditioning at every quadrature point ----
    for kappa in ([kappa_fixed] if kappa_fixed else KAPPAS):
        rho = kappa * hmax
        C = [Cell(cells, faces, ci, rho) for ci in range(len(cells))]
        cnt, cond = [], []
        for c in C:
            _, _, (n_, k_) = c.mls(c.xq, rho, deriv=False)
            cnt.append(n_.min()); cond.append(k_.max())
        ok = min(cnt) >= VOTE and max(cond) <= COND_MAX
        print(f"kappa {kappa}: min weighted edges {min(cnt)}, max cond {max(cond):.2e}, ok {ok}", flush=True)
        if ok or kappa_fixed:
            break
    out.update({"kappa": kappa, "rho": rho, "vote_min_weighted_edges": int(min(cnt)), "vote_required": VOTE,
                "cond_max": float(max(cond)), "reach_ok": bool(ok),
                "rings_median_max": [float(np.median([c.rings for c in C])), int(max(c.rings for c in C))],
                "candidate_edges_median": float(np.median([len(c.edges) for c in C])),
                "C_copies_cells": int(sum(c.copies > 0 for c in C)),
                "seconds_cells": round(time.time() - t00, 1)})

    # ---- controls ----
    cp = 0.0; co = 0.0
    for c in C[::max(1, len(C) // 25)]:
        S, dS, _ = c.mls(c.xq, rho, raw=True)                            # per-copy rows: polynomial data
        cp = max(cp, float(np.abs(ein("pme,en->pmn", S, c.Vrow) - np.eye(NCOEF)[None]).max()),
                 float(np.abs(ein("pime,en->pimn", dS, c.Vrow)).max()))
        for fi, f in enumerate(pg.FACES):
            ref = pg.data_flux(cells[c.ci], a, f)
            co = max(co, abs(float(c.TW[fi] @ a[[c.edges[i] for i in c.own]]) - ref))
    rng = np.random.default_rng(0); cd = 0.0
    for c in rng.choice(C, size=min(20, len(C)), replace=False):
        x = c.xq[rng.integers(len(c.xq))][None]; h = 1e-5 * 1.0
        an = c.curl_map(x, rho)[0] @ a[c.edges]
        Jm = np.zeros((3, 3))
        for i in range(3):
            e_ = np.zeros(3); e_[i] = h
            Jm[i] = ((c.reading_map(x + e_, rho)[0] - c.reading_map(x - e_, rho)[0]) @ a[c.edges]) / (2 * h)
        fd = ein("ijk,ij->k", EPS, Jm)
        cd = max(cd, float(np.linalg.norm(an - fd) / max(np.linalg.norm(an), 1e-300)))
    out.update({"C_poly_max": cp, "C_orient_max": co, "C_deriv_max_rel": cd})
    print("controls", out["C_poly_max"], out["C_orient_max"], out["C_deriv_max_rel"], flush=True)

    norm = lambda v: float(np.sqrt(max(0., v @ (mass @ v)) / vol))     # noqa: E731
    r4 = norm(-4 * a)
    rel = lambda K: norm(solve_rhs(mass, -(K @ a)) + 4 * a) / r4         # noqa: E731

    # ---- assemble the three arms ----
    arms = {}
    for arm in ("unseated", "frozen", "seated"):
        t1 = time.time()
        frozen = arm == "frozen"
        Kv = np.zeros((ne, ne)); Ks = np.zeros((ne, ne))
        rec = np.zeros(ne); tv = np.zeros(ne); cnum = cden = 0.0
        for c in C:
            Cm = c.curl_map(c.xq, rho, frozen)                            # (P,3,E)
            Pr = ein("ab,pbe->pae", c.piola, Cm)                    # reference proxy map
            ix = np.ix_(c.edges, c.edges); wt = c.weight
            Kv[ix] += wt * ein("pae,pab,pbf,p->ef", Pr, c.g, Pr, c.w)
            P = ein("pae,e->pa", Pr, a[c.edges]); diff = P - c.Pex
            rec[c.edges] += wt * ein("pae,pab,pb,p->e", Pr, c.g, diff, c.w)
            tv[c.edges] += wt * ein("pae,pab,pb,p->e", Pr, c.g, c.Pex, c.w)
            uF = ein("qfa,qab,qb,q->f", c.two, c.g, c.Pex, c.w)
            tv[[c.edges[i] for i in c.own]] -= wt * (c.TW.T @ uF)
            cnum += wt * float(ein("pa,pab,pb,p->", diff, c.g, diff, c.w))
            cden += wt * float(ein("pa,pab,pb,p->", c.Pex, c.g, c.Pex, c.w))
            if arm == "seated":
                Lm = c.own_line_map(rho)
                Pown = np.zeros_like(Lm); Pown[np.arange(6), c.own] = 1.0
                Rr = c.TW @ (Lm - Pown)                                   # (4,E): own-wall flux misfit
                Ks[ix] += SIGMA * wt * Rr.T @ c.GW @ Rr
        K = Kv + Ks
        res = {"bw05_relative_error": rel(K),
               "reconstruction_term": norm(solve_rhs(mass, rec)) / r4,
               "poke_term": norm(solve_rhs(mass, tv)) / r4,
               "curvature_relative_error": float(np.sqrt(cnum / cden)),
               "asymmetry": float(np.abs(K - K.T).max() / np.abs(K).max())}
        if arm == "seated":
            res["spring_force_share"] = norm(solve_rhs(mass, Ks @ a)) / r4
        res["seconds_assembly"] = round(time.time() - t1, 1)
        arms[arm] = (res, K)
        print(arm, json.dumps(res), flush=True)

    # ---- wobble: (K, M) on the M-orthogonal complement of the gauge directions ----
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
            print(arm, "eig", res["wobble_count_below_1"], res["min_physical_eig"], res["gauge_rayleigh_max"],
                  flush=True)
        out["seconds_eig"] = round(time.time() - t2, 1)

    # ---- seam: jump of the reading across walls, on exact tangents (unseated reading) ----
    t3 = time.time()
    jumps, vals = [], []
    bary = np.array([[2 / 3, 1 / 6, 1 / 6], [1 / 6, 2 / 3, 1 / 6], [1 / 6, 1 / 6, 2 / 3]])
    seen = set()
    for key, users in faces.items():
        if len(users) != 2 or key in seen:
            continue
        seen.add(key)
        (k1, f1), (k2, f2) = users
        c1, c2 = C[k1], C[k2]
        cr1, cr2 = pg.corners(cells[k1], f1), pg.corners(cells[k2], f2)
        common = sorted(set(cr1) & set(cr2))
        if len(common) != 3:
            continue
        V1v = np.array([c1.verts[cr1[g]] for g in common]); V2v = np.array([c2.verts[cr2[g]] for g in common])
        for lam in bary:
            X1 = lam @ V1v; X1 /= np.linalg.norm(X1)
            X2 = lam @ V2v; X2 /= np.linalg.norm(X2)
            for (i, j) in ((0, 1), (0, 2)):
                t1 = V1v[j] - V1v[i]; t1 -= (t1 @ X1) * X1
                t2 = V2v[j] - V2v[i]; t2 -= (t2 @ X2) * X2
                x1 = c1.ch.to(X1); x2 = c2.ch.to(X2)
                v1 = c1.ch.push(X1, t1); v2 = c2.ch.push(X2, t2)
                r1 = float(c1.reading_map(x1, rho)[0] @ a[c1.edges] @ v1)
                r2 = float(c2.reading_map(x2, rho)[0] @ a[c2.edges] @ v2)
                jumps.append(r1 - r2); vals.extend([r1, r2])
    out["seam_rms_jump_relative"] = float(np.sqrt(np.mean(np.square(jumps))) / np.sqrt(np.mean(np.square(vals))))
    out["seam_points"] = len(jumps)
    out["seconds_seam"] = round(time.time() - t3, 1)
    print("seam", out["seam_rms_jump_relative"], len(jumps), flush=True)

    out["arms"] = {arm: res for arm, (res, _) in arms.items()}
    out["seconds"] = round(time.time() - t00, 1)
    receipt.write_text(json.dumps(out, indent=1) + "\n")
    print("wrote", receipt, flush=True)


if __name__ == "__main__":
    main()
