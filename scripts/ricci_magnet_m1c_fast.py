"""M1c, assembled fast (MAGNET_M1_SPEC.md, Amendment 2). Same mathematics as
`ricci_magnet_m1b.py LEVEL OUTDIR --neumann K`; must reproduce its receipts to round-off.

The reading R~1 a = R1 a - Z'(Q a), Q = I - G, has per-cell curl Pr = Pr0 a_loc - C (Q a)[bumps], with Pr0 M0's curl
on the cell's own reach and C the correction curl per bump row (coefficients c = z'/N, before Q). Instead of widening
every cell to the columns of Q, everything is accumulated in the small spaces and Q is applied once:
  K  = K0 - Q^T X - X^T Q + Q^T B Q,   K0 = sum Pr0^T g Pr0,  X = sum C^T g Pr0,  B = sum C^T g C
and the same pattern for the seat spring and the force vectors. The seat-spring weight is SIGMA, from the
environment M1_SIGMA (default 1, as M0), multiplied by (h / h_ref)^2 if M1_SIGMA_H2 is set (Amendment 3).

usage: python scripts/ricci_magnet_m1c_fast.py LEVEL OUTDIR [--neumann K] [--tag NAME] [--no-eig]
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
import ricci_magnet_m1b as B1                    # noqa: E402
from ricci_consistency_math import solve_rhs     # noqa: E402

pg, ein, ORDER = M.pg, M.ein, M.ORDER
ARMS = ("unseated", "seated")
_W = {}


def _init(level, rho, coef, qa, sigma):
    os.environ.setdefault("OMP_NUM_THREADS", "1")
    L = pg.load_level(level, ORDER)
    _W.update(cells=L["cells"], a=np.asarray(L["a"]), faces=pg.face_index(L["cells"]), rho=rho, coef=coef, qa=qa,
              sigma=sigma)


def _w_cell(ci):
    cells, a, faces, rho, coef, qa, sigma = (_W[k] for k in ("cells", "a", "faces", "rho", "coef", "qa", "sigma"))
    c = M.Cell(cells, faces, ci, rho)
    wt = c.weight
    re = M1._row_edges(c)
    cr = coef[re]
    q = pg.quadrature(c.verts[None], ORDER)
    mu = q["measure"][0]
    Xq, dX, th, dth = M1.ref_geometry(c.verts, q["xref"])
    phi, gphi = M1.bumps(Xq, dX, c.mid, rho)
    C = ein("prak,ra->pkr", np.cross(gphi[:, :, None, :], th[:, None, :, :]) + phi[:, :, None, None] * dth[:, None], cr)
    Pr0 = ein("ab,pbe->pae", c.piola, c.curl_map(c.xq, rho, False))
    P = ein("pae,e->pa", Pr0, a[c.edges]) - ein("pkr,r->pk", C, qa[re]); diff = P - c.Pex
    gw = lambda U, V: wt * ein("pax,pab,pby,p->xy", U, c.g, V, c.w)    # noqa: E731
    gv = lambda U, v: wt * ein("pax,pab,pb,p->x", U, c.g, v, c.w)      # noqa: E731
    uF = ein("qfa,qab,qb,q->f", c.two, c.g, c.Pex, c.w)
    out = {"edges": c.edges, "re": re, "own": list(cells[ci]["edges"]),
           "K0": gw(Pr0, Pr0), "X": gw(C, Pr0), "B": gw(C, C),
           "rec0": gv(Pr0, diff), "yrec": gv(C, diff), "tv0": gv(Pr0, c.Pex), "ytv": gv(C, c.Pex),
           "tvown": wt * (c.TW.T @ uF),
           "cn": wt * float(ein("pa,pab,pb,p->", diff, c.g, diff, c.w)),
           "cd": wt * float(ein("pa,pab,pb,p->", c.Pex, c.g, c.Pex, c.w)),
           "mrow": wt * cr * ein("pr,p->r", phi, mu)[:, None]}               # moment of each bump's correction
    # seat spring: own-wall flux misfit of the corrected reading
    Lm = c.own_line_map(rho)                                              # (6,E)
    Jinv = np.linalg.inv(c.ch.J)
    Lcr = np.zeros((6, len(re)))
    for k in range(6):
        dxi = Jinv @ (c.ownQ[k] - c.ownP[k])
        for t, w in zip(M1.GL6_T, M1.GL6_W):
            xi = (Jinv @ (c.ownP[k] + t * (c.ownQ[k] - c.ownP[k]) - c.ch.t0))[None]
            Xp, dXp, thp, _ = M1.ref_geometry(c.verts, xi)
            ph, _ = M1.bumps(Xp, dXp, c.mid, rho)
            Lcr[k] += w * ein("r,ra,ab,b->r", ph[0], cr, thp[0], dxi)
    Pown = np.zeros_like(Lm); Pown[np.arange(6), c.own] = 1.0
    V0, V1 = c.TW @ (Lm - Pown), c.TW @ Lcr
    s = sigma * wt
    out.update(Ks0=s * V0.T @ c.GW @ V0, Xs=s * V1.T @ c.GW @ V0, Bs=s * V1.T @ c.GW @ V1)
    seam = []
    for f in pg.FACES:
        for key, Xw, tg in M._wall_points(c, cells, ci, f):
            x = c.ch.to(Xw); v = c.ch.push(Xw, tg)
            val = float(c.reading_map(x, rho)[0] @ a[c.edges] @ v)
            ph, _ = M.wendland(np.linalg.norm(Xw[None] - c.mid, axis=1) / rho)
            thv = M1.frame4(Xw[None])[0].T @ tg
            val -= float(ph @ ((cr @ thv) * qa[re]))
            seam.append((key, val))
    out["seam"] = seam
    return out


def main():
    args = sys.argv[1:]
    do_eig = "--no-eig" not in args
    args = [x for x in args if x != "--no-eig"]
    K = 8; tag = "M1c"
    if "--neumann" in args:
        i = args.index("--neumann"); K = int(args[i + 1]); del args[i:i + 2]
    if "--tag" in args:
        i = args.index("--tag"); tag = args[i + 1]; del args[i:i + 2]
    level, outdir = int(args[0]), Path(args[1])
    receipt = outdir / f"{tag}_L{level}.json"
    if receipt.exists():
        raise SystemExit(f"refusing to overwrite {receipt}")
    t00 = time.time()
    L = pg.load_level(level, ORDER)
    cells, a, mass, vol = L["cells"], np.asarray(L["a"]), L["mass"], L["volume"]
    ne = len(a)
    diag = json.loads((M.V._DATA / "output/ricci-variational-20261117/abelian/diagnostic.json").read_text())["cases"]
    hmax = diag[f"level{level}_q6"]["h_max"]
    rho = 1.0 * hmax
    sigma = float(os.environ.get("M1_SIGMA", "1.0"))
    if os.environ.get("M1_SIGMA_H2"):
        sigma *= (hmax / diag["level0_q6"]["h_max"]) ** 2
    mo = np.load(outdir / f"moments_L{level}.npz")
    z, l1 = mo["z"], mo["l1"]
    nproc = int(os.environ.get("M0_PROCS", "8"))
    ctx = mp.get_context("spawn")
    out = {"level": level, "edges": ne, "cells": len(cells), "rho": rho, "k": 1, "neumann_K": K, "sigma": sigma,
           "kind": f"{tag} (fast assembly of M1c; research script, not an engine run)"}
    N = np.zeros(ne); G = np.zeros((ne, ne)); cnt = np.zeros(ne)
    with ctx.Pool(nproc, initializer=B1._init, initargs=(level, rho, None, None)) as pool:
        for re, nv, own, edges, Lm in pool.imap_unordered(B1._w_pass1, range(len(cells)), chunksize=2):
            np.add.at(N, re, nv)
            for k, g in enumerate(own):
                G[g, edges] += Lm[k]; cnt[g] += 1
    G /= cnt[:, None]
    Q = np.eye(ne) - G
    zk, acc = z.copy(), z.copy()
    for _ in range(K):
        zk = G.T @ zk; acc = acc + zk
    coef = acc / N[:, None]
    pred_left = G.T @ zk
    qa = Q @ a
    out.update({"seconds_pass1": round(time.time() - t00, 1),
                "predicted_leftover_moment_median": float(np.median(np.linalg.norm(pred_left, axis=1) / l1))})
    print("pass 1", json.dumps(out), flush=True)
    K0 = np.zeros((ne, ne)); Xg = np.zeros((ne, ne)); Bg = np.zeros((ne, ne))
    Ks0 = np.zeros((ne, ne)); Xs = np.zeros((ne, ne)); Bs = np.zeros((ne, ne))
    rec0 = np.zeros(ne); yrec = np.zeros(ne); tv0 = np.zeros(ne); ytv = np.zeros(ne)
    mrow = np.zeros((ne, 3)); cn = cd = 0.0; seam = {}
    with ctx.Pool(nproc, initializer=_init, initargs=(level, rho, coef, qa, sigma)) as pool:
        for k, r in enumerate(pool.imap_unordered(_w_cell, range(len(cells)), chunksize=2)):
            e, re, own = r["edges"], r["re"], r["own"]
            # bump rows can repeat an edge (two sheet positions): np.add.at accumulates repeats, += would drop them
            e = np.asarray(e); re = np.asarray(re); own = np.asarray(own)
            K0[np.ix_(e, e)] += r["K0"]; Ks0[np.ix_(e, e)] += r["Ks0"]
            np.add.at(Xg, (re[:, None], e[None, :]), r["X"]); np.add.at(Bg, (re[:, None], re[None, :]), r["B"])
            np.add.at(Xs, (re[:, None], e[None, :]), r["Xs"]); np.add.at(Bs, (re[:, None], re[None, :]), r["Bs"])
            rec0[e] += r["rec0"]; tv0[e] += r["tv0"]
            np.add.at(yrec, re, r["yrec"]); np.add.at(ytv, re, r["ytv"]); np.add.at(mrow, re, r["mrow"])
            np.add.at(tv0, own, -r["tvown"]); cn += r["cn"]; cd += r["cd"]
            for key, val in r["seam"]:
                seam.setdefault(key, []).append(val)
            if (k + 1) % 200 == 0:
                print(f"  {k + 1}/{len(cells)} cells, {time.time() - t00:.0f} s", flush=True)
    Kv = K0 - Q.T @ Xg - Xg.T @ Q + Q.T @ Bg @ Q
    Ks = Ks0 - Q.T @ Xs - Xs.T @ Q + Q.T @ Bs @ Q
    rec = rec0 - Q.T @ yrec; tv = tv0 - Q.T @ ytv
    mom = Q.T @ mrow
    zt = z - mom; zr = np.linalg.norm(zt, axis=1) / l1
    out.update({"seconds_cells": round(time.time() - t00, 1),
                "leftover_moment_median": float(np.median(zr)), "leftover_moment_max": float(zr.max()),
                "leftover_vs_prediction_rel": float(np.linalg.norm(zt - pred_left) / max(np.linalg.norm(pred_left), 1e-300))})
    norm = lambda v: float(np.sqrt(max(0., v @ (mass @ v)) / vol))     # noqa: E731
    r4 = norm(-4 * a)
    rel = lambda KK: norm(solve_rhs(mass, -(KK @ a)) + 4 * a) / r4       # noqa: E731
    arms = {}
    for arm in ARMS:
        KK = Kv + (Ks if arm == "seated" else 0.0)
        res = {"bw05_relative_error": rel(KK), "reconstruction_term": norm(solve_rhs(mass, rec)) / r4,
               "poke_term": norm(solve_rhs(mass, tv)) / r4, "curvature_relative_error": float(np.sqrt(cn / cd)),
               "asymmetry": float(np.abs(KK - KK.T).max() / np.abs(KK).max())}
        if arm == "seated":
            res["spring_force_share"] = norm(solve_rhs(mass, Ks @ a)) / r4
        arms[arm] = (res, KK)
        print(arm, json.dumps(res), flush=True)
    if do_eig:
        Mm = mass.toarray() if hasattr(mass, "toarray") else np.asarray(mass)
        Kw = L["stiffness"].toarray() if hasattr(L["stiffness"], "toarray") else np.asarray(L["stiffness"])
        lw, Vw = eigh(Kw, Mm)
        Pp = Vw[:, ~(np.abs(lw) < 1)]
        out["gauge_dimension"] = int((np.abs(lw) < 1).sum())
        for arm, (res, KK) in arms.items():
            lam = np.linalg.eigvalsh(Pp.T @ (0.5 * (KK + KK.T)) @ Pp)
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
