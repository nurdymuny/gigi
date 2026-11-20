"""M1b: the moment correction applied only to what the reading cannot reproduce (MAGNET_M1_SPEC.md, Amendment 1).
Research script, not an engine run. Abelian control, q6, M0's reading (k = 1 MLS, Wendland, kappa = 1, rho = h_max).

  R~1 a = R1 a - Z (a - G a),   Z v = sum_e v_e sum_a c_e^a phi(|X - m_e| / rho) theta^a,  c = z / N,
  G = I1 R1 (line integrals of M0's reading along every edge; each edge averaged over the cells that hold it).
Data the reading reproduces (G a = a) is left untouched, so linear fields stay exact; the moment left over per edge
is (G^T z)_e instead of 0. Arms and gates as in M1, with M1-moment replaced by M1b-moment (Amendment 1).

usage: python scripts/ricci_magnet_m1b.py LEVEL OUTDIR [--no-eig]
"""
import json
import multiprocessing as mp
import os
import sys
import time
from pathlib import Path

import numpy as np
from scipy.linalg import eigh

HERE = Path(__file__).resolve().parent
sys.path[:0] = [str(HERE), str(HERE.parent / "e2e/probes")]
import ricci_magnet_m0 as M                      # noqa: E402
import ricci_magnet_m1 as M1                     # noqa: E402
from ricci_consistency_math import solve_rhs     # noqa: E402

pg, ein, ORDER, SIGMA = M.pg, M.ein, M.ORDER, M.SIGMA
ARMS = ("unseated", "seated")
M0_RECEIPTS = M1.M0_RECEIPTS
_W = {}


def _init(level, rho, coef, Q):
    os.environ.setdefault("OMP_NUM_THREADS", "1")
    L = pg.load_level(level, ORDER)
    _W.update(cells=L["cells"], a=np.asarray(L["a"]), faces=pg.face_index(L["cells"]), rho=rho, coef=coef, Q=Q)


def _w_pass1(ci):
    """N_e contributions and the cell's own-edge line integrals of M0's reading (rows of G)."""
    cells, faces, rho = _W["cells"], _W["faces"], _W["rho"]
    c = M.Cell(cells, faces, ci, rho)
    q = pg.quadrature(c.verts[None], ORDER)
    X, _, _, _ = M1.ref_geometry(c.verts, q["xref"])
    phi, _ = M.wendland(np.linalg.norm(X[:, None, :] - c.mid[None], axis=2) / rho)
    return (M1._row_edges(c), c.weight * ein("pr,p->r", phi, q["measure"][0]),
            list(cells[ci]["edges"]), c.edges, c.own_line_map(rho))


def _w_cell(ci):
    cells, a, faces, rho, coef, Q = _W["cells"], _W["a"], _W["faces"], _W["rho"], _W["coef"], _W["Q"]
    c = M.Cell(cells, faces, ci, rho)
    wt = c.weight
    re = M1._row_edges(c)
    Qr = Q[re]                                                            # (R, ne): rows of (I - G) for the bumps here
    ext = np.unique(np.concatenate([np.array(c.edges), np.nonzero(np.abs(Qr).sum(0))[0]]))
    pos = {g: i for i, g in enumerate(ext)}
    Qx = Qr[:, ext]                                                       # (R, X)
    lift = np.zeros((len(c.edges), len(ext))); lift[np.arange(len(c.edges)), [pos[g] for g in c.edges]] = 1.0
    q = pg.quadrature(c.verts[None], ORDER)
    mu = q["measure"][0]
    Xq, dX, th, dth = M1.ref_geometry(c.verts, q["xref"])
    phi, gphi = M1.bumps(Xq, dX, c.mid, rho)
    cr = coef[re]
    corr = ein("pr,ra,pab,rx->pbx", phi, cr, th, Qx)                      # (P,3,X)
    ccurl = np.cross(gphi[:, :, None, :], th[:, None, :, :]) + phi[:, :, None, None] * dth[:, None]
    ccurl = ein("prak,ra,rx->pkx", ccurl, cr, Qx)
    Pr0 = ein("ab,pbe->pae", c.piola, c.curl_map(c.xq, rho, False))
    Pr = ein("pae,ex->pax", Pr0, lift) - ccurl
    Einv = np.linalg.inv(th)
    mom = wt * ein("pbx,pba,p->xa", corr, Einv, mu)
    out = {"ci": ci, "ext": ext, "own": list(cells[ci]["edges"]), "arms": {}, "mom": mom}
    uF = ein("qfa,qab,qb,q->f", c.two, c.g, c.Pex, c.w)
    P = ein("pax,x->pa", Pr, a[ext]); diff = P - c.Pex
    base = {"K": wt * ein("pax,pab,pby,p->xy", Pr, c.g, Pr, c.w),
            "rec": wt * ein("pax,pab,pb,p->x", Pr, c.g, diff, c.w),
            "tv": wt * ein("pax,pab,pb,p->x", Pr, c.g, c.Pex, c.w),
            "tvown": wt * (c.TW.T @ uF),
            "cn": wt * float(ein("pa,pab,pb,p->", diff, c.g, diff, c.w)),
            "cd": wt * float(ein("pa,pab,pb,p->", c.Pex, c.g, c.Pex, c.w))}
    out["arms"]["unseated"] = base
    Lm = c.own_line_map(rho) @ lift                                        # (6, X)
    Jinv = np.linalg.inv(c.ch.J)
    Lc = np.zeros_like(Lm)
    for k in range(6):
        dxi = Jinv @ (c.ownQ[k] - c.ownP[k])
        for t, w in zip(M1.GL6_T, M1.GL6_W):
            xi = (Jinv @ (c.ownP[k] + t * (c.ownQ[k] - c.ownP[k]) - c.ch.t0))[None]
            Xp, dXp, thp, _ = M1.ref_geometry(c.verts, xi)
            ph, _ = M1.bumps(Xp, dXp, c.mid, rho)
            Lc[k] += w * ein("r,ra,ab,b,rx->x", ph[0], cr, thp[0], dxi, Qx)
    Lm = Lm - Lc
    Pown = np.zeros_like(Lm); Pown[np.arange(6), [pos[g] for g in out["own"]]] = 1.0
    Rr = c.TW @ (Lm - Pown)
    out["arms"]["seated"] = dict(base, Ks=SIGMA * wt * Rr.T @ c.GW @ Rr)
    qa = Qx @ a[ext]                                                      # (R,): (a - G a) at the bumps' edges
    seam = []
    for f in pg.FACES:
        for key, Xw, tg in M._wall_points(c, cells, ci, f):
            x = c.ch.to(Xw); v = c.ch.push(Xw, tg)
            val = float(c.reading_map(x, rho)[0] @ a[c.edges] @ v)
            ph, _ = M.wendland(np.linalg.norm(Xw[None] - c.mid, axis=1) / rho)
            thv = M1.frame4(Xw[None])[0].T @ tg
            val -= float(ph @ ((cr @ thv) * qa))
            seam.append((key, val))
    out["seam"] = seam
    return out


def main():
    args = sys.argv[1:]
    do_eig = "--no-eig" not in args
    args = [x for x in args if x != "--no-eig"]
    K = 0
    if "--neumann" in args:
        i = args.index("--neumann"); K = int(args[i + 1]); del args[i:i + 2]
    level, outdir = int(args[0]), Path(args[1])
    receipt = outdir / (f"M1c_L{level}.json" if K else f"M1b_L{level}.json")
    if receipt.exists():
        raise SystemExit(f"refusing to overwrite {receipt}")
    outdir.mkdir(parents=True, exist_ok=True)
    t00 = time.time()
    L = pg.load_level(level, ORDER)
    cells, a, mass, vol = L["cells"], np.asarray(L["a"]), L["mass"], L["volume"]
    ne = len(a)
    hmax = json.loads((M.V._DATA / "output/ricci-variational-20261117/abelian/diagnostic.json").read_text()
                      )["cases"][f"level{level}_q6"]["h_max"]
    rho = 1.0 * hmax
    audit = json.loads((outdir / f"audit_L{level}.json").read_text())
    if abs(audit["rho"] - rho) > 1e-12:
        raise SystemExit("audit reach differs")
    mo = np.load(outdir / f"moments_L{level}.npz")
    z, l1 = mo["z"], mo["l1"]
    nproc = int(os.environ.get("M0_PROCS", "8"))
    ctx = mp.get_context("spawn")
    out = {"level": level, "edges": ne, "cells": len(cells), "rho": rho, "k": 1,
           "kind": "M1b (research script, not an engine run)" if not K else "M1c = M1b with K Neumann rounds (Amendment 2)",
           "neumann_K": K}
    N = np.zeros(ne); G = np.zeros((ne, ne)); cnt = np.zeros(ne)
    with ctx.Pool(nproc, initializer=_init, initargs=(level, rho, None, None)) as pool:
        for re, nv, own, edges, Lm in pool.imap_unordered(_w_pass1, range(len(cells)), chunksize=2):
            np.add.at(N, re, nv)
            for k, g in enumerate(own):
                G[g, edges] += Lm[k]; cnt[g] += 1
    G /= cnt[:, None]
    Q = np.eye(ne) - G
    zk, acc = z.copy(), z.copy()
    for _ in range(K):                                   # Amendment 2 (M1c): z' = sum_{k<=K} (G^T)^k z
        zk = G.T @ zk; acc = acc + zk
    coef = acc / N[:, None]
    pred_left = G.T @ zk                                 # (G^T)^(K+1) z
    left = np.linalg.norm(pred_left, axis=1) / l1
    out.update({"seconds_pass1": round(time.time() - t00, 1),
                "G_reproduction_rel_on_data": float(np.linalg.norm(G @ a - a) / np.linalg.norm(a)),
                "predicted_leftover_moment_median": float(np.median(left)),
                "M0_moment_defect_median": float(np.median(np.linalg.norm(z, axis=1) / l1))})
    print("pass 1", json.dumps(out), flush=True)
    Kv = {arm: np.zeros((ne, ne)) for arm in ARMS}; Ks = np.zeros((ne, ne))
    rec = {arm: np.zeros(ne) for arm in ARMS}; tv = {arm: np.zeros(ne) for arm in ARMS}
    cnum = {arm: 0.0 for arm in ARMS}; cden = {arm: 0.0 for arm in ARMS}
    mom = np.zeros((ne, 3)); seam = {}
    with ctx.Pool(nproc, initializer=_init, initargs=(level, rho, coef, Q)) as pool:
        for k, r in enumerate(pool.imap_unordered(_w_cell, range(len(cells)), chunksize=2)):
            e = r["ext"]; ix = np.ix_(e, e)
            mom[e] += r["mom"]
            for arm in ARMS:
                p = r["arms"][arm]
                Kv[arm][ix] += p["K"]
                if "Ks" in p:
                    Ks[ix] += p["Ks"]
                rec[arm][e] += p["rec"]; tv[arm][e] += p["tv"]; tv[arm][r["own"]] -= p["tvown"]
                cnum[arm] += p["cn"]; cden[arm] += p["cd"]
            for key, val in r["seam"]:
                seam.setdefault(key, []).append(val)
            if (k + 1) % 200 == 0:
                print(f"  {k + 1}/{len(cells)} cells, {time.time() - t00:.0f} s", flush=True)
    zt = z - mom                                         # mom = moments of the correction = (I - G^T) z'

    zr = np.linalg.norm(zt, axis=1) / l1
    out.update({"seconds_cells": round(time.time() - t00, 1),
                "leftover_moment_median": float(np.median(zr)), "leftover_moment_max": float(zr.max()),
                "leftover_vs_prediction_rel": float(np.linalg.norm(zt - pred_left) / max(np.linalg.norm(pred_left), 1e-300))})
    norm = lambda v: float(np.sqrt(max(0., v @ (mass @ v)) / vol))     # noqa: E731
    r4 = norm(-4 * a)
    rel = lambda K: norm(solve_rhs(mass, -(K @ a)) + 4 * a) / r4         # noqa: E731
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
    if do_eig:
        Mm = mass.toarray() if hasattr(mass, "toarray") else np.asarray(mass)
        Kw = L["stiffness"].toarray() if hasattr(L["stiffness"], "toarray") else np.asarray(L["stiffness"])
        lw, Vw = eigh(Kw, Mm)
        gmask = np.abs(lw) < 1
        Pp = Vw[:, ~gmask]
        out["gauge_dimension"] = int(gmask.sum())
        for arm, (res, K) in arms.items():
            lam = np.linalg.eigvalsh(Pp.T @ (0.5 * (K + K.T)) @ Pp)
            res.update({"wobble_count_below_1": int((lam < 1).sum()), "min_physical_eig": float(lam.min()),
                        "negative_physical": int((lam < -1e-8 * abs(lam).max()).sum())})
            print(arm, "eig", res["wobble_count_below_1"], res["min_physical_eig"], flush=True)
    jumps, vals = [], []
    for key, vs in seam.items():
        if len(vs) == 2:
            jumps.append(vs[0] - vs[1]); vals.extend(vs)
    out["seam_rms_jump_relative"] = float(np.sqrt(np.mean(np.square(jumps))) / np.sqrt(np.mean(np.square(vals))))
    out["seam_points"] = len(jumps)
    out["arms"] = {arm: res for arm, (res, _) in arms.items()}
    out["seconds"] = round(time.time() - t00, 1)
    receipt.write_text(json.dumps(out, indent=1) + "\n")
    print(json.dumps({k: v for k, v in out.items() if k != "arms"}, indent=1), flush=True)


if __name__ == "__main__":
    main()
