"""M1: the moment-corrected magnet (docs/MAGNET_M1_SPEC.md, Part 2). Research script, not an engine run.
Abelian control, q6, the M0 meshes and data, M0's reading (k = 1 MLS, Wendland, kappa = 1, rho = h_max).

The reading, with every bump corrected to its Whitney tent's zeroth moment:
  R~1 a = R1 a - sum_e a_e sum_a c_e^a phi(|X - m_e| / rho) theta^a,   c_e^a = z_e^a / N_e,
  N_e = int phi(|X - m_e| / rho) dvol (same quadrature as the moments), z from the Part 1 audit
  (research/magnet-m1-20261119/moments_L{n}.npz). theta^a is the left-invariant coframe of
  pg.frame_quantities; d theta^a = -2 theta^(a+1) x theta^(a+2) (cyclic), checked against finite differences.
Arms
  seated    E~ = 1/2 sum_K w_K int_K |curl R~1 a|^2 + 1/2 sigma sum_K w_K r~_K^T G^W_K r~_K   (gated)
  unseated  E~ without the spring                                                        (reported)
Removal arm: M0 itself (research/magnet-m0-20261119/spawn/).

usage: python scripts/ricci_magnet_m1.py LEVEL OUTDIR [--no-eig]
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
from ricci_consistency_math import solve_rhs     # noqa: E402

pg, ein, ORDER, EPS, SIGMA = M.pg, M.ein, M.ORDER, M.EPS, M.SIGMA
GL6_T, GL6_W = np.polynomial.legendre.leggauss(6)
GL6_T, GL6_W = (GL6_T + 1) / 2, GL6_W / 2
ARMS = ("unseated", "seated")
M0_RECEIPTS = {1: (0.5115045112111449, 0.07654215366023452), 2: (0.5105870214614711, 0.0646164529473736)}
_W = {}


def frame4(X):
    """(P,4,3): the left-invariant frame used by pg.frame_quantities; theta^a(t) = frame[:, :, a] . t."""
    P = X.shape[0]
    fr = np.empty((P, 4, 3))
    fr[:, 0, :] = -X[:, 1:]
    a_, b_, c_ = X[:, 1], X[:, 2], X[:, 3]
    sk = np.zeros((P, 3, 3))
    sk[:, 0, 1] = -c_; sk[:, 0, 2] = b_; sk[:, 1, 0] = c_
    sk[:, 1, 2] = -a_; sk[:, 2, 0] = -b_; sk[:, 2, 1] = a_
    fr[:, 1:, :] = X[:, 0, None, None] * np.eye(3)[None] + sk
    return fr


def ref_geometry(verts, xi):
    """At reference points xi (P,3) of the cell with R^4 vertices verts: X (P,4), dX/dxi (P,4,3),
    theta^a in reference coordinates (P,3a,3b), its exterior derivative as a reference 2-form proxy (P,3a,3)."""
    lam = pg.barycentric(xi)
    y = lam @ verts
    ny = np.linalg.norm(y, axis=1)
    X = y / ny[:, None]
    b = (verts[1:] - verts[0]).T                                          # (4,3)
    dX = ein("pij,jb->pib", (np.eye(4)[None] - X[:, :, None] * X[:, None, :]) / ny[:, None, None], b)
    th = ein("pia,pib->pab", frame4(X), dX)
    dth = np.stack([-2 * np.cross(th[:, (a + 1) % 3], th[:, (a + 2) % 3]) for a in range(3)], axis=1)
    return X, dX, th, dth


def bumps(X, dX, mids, rho):
    """phi (P,R) and its reference gradient (P,R,3) for Wendland bumps centred at mids (R,4)."""
    diff = X[:, None, :] - mids[None]
    d = np.linalg.norm(diff, axis=2)
    phi, dphi = M.wendland(d / rho)
    gd = ein("pra,pab->prb", diff / np.maximum(d, 1e-300)[:, :, None], dX)
    return phi, (dphi / rho)[:, :, None] * gd


def _init(level, rho, coef):
    os.environ.setdefault("OMP_NUM_THREADS", "1")
    L = pg.load_level(level, ORDER)
    _W.update(cells=L["cells"], a=np.asarray(L["a"]), faces=pg.face_index(L["cells"]), rho=rho, coef=coef)


def _row_edges(c):
    return np.array([c.edges[i] for i in np.argmax(c.Psel, axis=1)])


def _w_norm(ci):
    """Pass 1: this cell's contribution to N_e = int phi_e dvol, per global edge."""
    cells, faces, rho = _W["cells"], _W["faces"], _W["rho"]
    c = M.Cell(cells, faces, ci, rho)
    q = pg.quadrature(c.verts[None], ORDER)
    X, _, _, _ = ref_geometry(c.verts, q["xref"])
    diff = X[:, None, :] - c.mid[None]
    phi, _ = M.wendland(np.linalg.norm(diff, axis=2) / rho)
    return _row_edges(c), c.weight * ein("pr,p->r", phi, q["measure"][0])


def _w_cell(ci):
    cells, a, faces, rho, coef = _W["cells"], _W["a"], _W["faces"], _W["rho"], _W["coef"]
    c = M.Cell(cells, faces, ci, rho)
    wt = c.weight
    q = pg.quadrature(c.verts[None], ORDER)
    mu = q["measure"][0]
    X, dX, th, dth = ref_geometry(c.verts, q["xref"])
    phi, gphi = bumps(X, dX, c.mid, rho)
    re = _row_edges(c)
    cr = coef[re]                                                         # (R,3): c_e^a per row
    colsel = c.Psel                                                       # (R,E)
    # correction field and its curl, reference coordinates, per edge column
    corr = ein("pr,ra,pab,re->pbe", phi, cr, th, colsel)                  # (P,3,E)
    ccurl = (np.cross(gphi[:, :, None, :], th[:, None, :, :]) + phi[:, :, None, None] * dth[:, None]) # (P,R,3a,3)
    ccurl = ein("prak,ra,re->pke", ccurl, cr, colsel)                     # (P,3,E)
    # M0's seamless curl, in reference coordinates (Piola), then corrected
    Pr0 = ein("ab,pbe->pae", c.piola, c.curl_map(c.xq, rho, False))
    Pr = Pr0 - ccurl
    # moments of the correction (M1-moment): sum_a c_e^a int phi theta^a(e_b) dvol, which should be z_e^b
    Einv = np.linalg.inv(th)
    mom = wt * ein("pbe,pba,p->ea", corr, Einv, mu)
    out = {"ci": ci, "edges": c.edges, "own": list(cells[ci]["edges"]), "arms": {}, "mom": mom}
    uF = ein("qfa,qab,qb,q->f", c.two, c.g, c.Pex, c.w)
    P = ein("pae,e->pa", Pr, a[c.edges]); diff = P - c.Pex
    base = {"K": wt * ein("pae,pab,pbf,p->ef", Pr, c.g, Pr, c.w),
            "rec": wt * ein("pae,pab,pb,p->e", Pr, c.g, diff, c.w),
            "tv": wt * ein("pae,pab,pb,p->e", Pr, c.g, c.Pex, c.w),
            "tvown": wt * (c.TW.T @ uF),
            "cn": wt * float(ein("pa,pab,pb,p->", diff, c.g, diff, c.w)),
            "cd": wt * float(ein("pa,pab,pb,p->", c.Pex, c.g, c.Pex, c.w))}
    out["arms"]["unseated"] = base
    # seat spring on the corrected reading's own-wall flux
    Lm = c.own_line_map(rho)                                              # M0 reading, chart line integrals (6,E)
    Jinv = np.linalg.inv(c.ch.J)
    Lc = np.zeros_like(Lm)
    for k in range(6):
        dxi = Jinv @ (c.ownQ[k] - c.ownP[k])
        for t, w in zip(GL6_T, GL6_W):
            xi = (Jinv @ (c.ownP[k] + t * (c.ownQ[k] - c.ownP[k]) - c.ch.t0))[None]
            Xp, dXp, thp, _ = ref_geometry(c.verts, xi)
            ph, _ = bumps(Xp, dXp, c.mid, rho)
            Lc[k] += w * ein("r,ra,ab,b,re->e", ph[0], cr, thp[0], dxi, colsel)
    Lm = Lm - Lc
    Pown = np.zeros_like(Lm); Pown[np.arange(6), c.own] = 1.0
    Rr = c.TW @ (Lm - Pown)
    out["arms"]["seated"] = dict(base, Ks=SIGMA * wt * Rr.T @ c.GW @ Rr)
    # seam: corrected reading on each wall's two edge vectors
    seam = []
    for f in pg.FACES:
        for key, Xw, tg in M._wall_points(c, cells, ci, f):
            x = c.ch.to(Xw); v = c.ch.push(Xw, tg)
            val = float(c.reading_map(x, rho)[0] @ a[c.edges] @ v)
            ph, _ = M.wendland(np.linalg.norm(Xw[None] - c.mid, axis=1) / rho)
            thv = frame4(Xw[None])[0].T @ tg                               # theta^a(tg), (3,)
            val -= float(ph @ (cr @ thv * (colsel @ a[c.edges])))
            seam.append((key, val))
    out["seam"] = seam
    return out


def control_curl(level, rho, coef, n=6):
    """C-curl: the analytic curl of the correction against central differences of the correction field."""
    L = pg.load_level(level, ORDER)
    cells, faces = L["cells"], pg.face_index(L["cells"])
    rng = np.random.default_rng(1); worst = 0.0
    for ci in rng.choice(len(cells), size=n, replace=False):
        c = M.Cell(cells, faces, int(ci), rho)
        cr = coef[_row_edges(c)]
        xi0 = rng.dirichlet(np.ones(4))[1:][None]

        def field(xi):
            X, dX, th, _ = ref_geometry(c.verts, xi)
            ph, _ = bumps(X, dX, c.mid, rho)
            return ein("pr,ra,pab->pb", ph, cr, th)[0]                  # summed over rows: one test field
        X, dX, th, dth = ref_geometry(c.verts, xi0)
        ph, gph = bumps(X, dX, c.mid, rho)
        an = ein("prak,ra->k", np.cross(gph[:, :, None, :], th[:, None]) + ph[:, :, None, None] * dth[:, None], cr)
        h = 1e-6; Jm = np.zeros((3, 3))
        for i in range(3):
            e_ = np.zeros(3); e_[i] = h
            Jm[i] = (field(xi0 + e_) - field(xi0 - e_)) / (2 * h)
        fd = ein("ijk,ij->k", EPS, Jm)
        worst = max(worst, float(np.linalg.norm(an - fd) / max(np.linalg.norm(an), 1e-300)))
    return worst


def main():
    args = sys.argv[1:]
    do_eig = "--no-eig" not in args
    args = [x for x in args if x != "--no-eig"]
    level, outdir = int(args[0]), Path(args[1])
    receipt = outdir / f"M1_L{level}.json"
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
        raise SystemExit("audit reach differs from M1's")
    mo = np.load(outdir / f"moments_L{level}.npz")
    z, l1 = mo["z"], mo["l1"]
    nproc = int(os.environ.get("M0_PROCS", "8"))
    ctx = mp.get_context("spawn")
    out = {"level": level, "edges": ne, "cells": len(cells), "rho": rho, "k": 1,
           "kind": "M1 (research script, not an engine run)"}
    # pass 1: N_e
    N = np.zeros(ne)
    with ctx.Pool(nproc, initializer=_init, initargs=(level, rho, np.zeros((ne, 3)))) as pool:
        for re, nv in pool.imap_unordered(_w_norm, range(len(cells)), chunksize=4):
            np.add.at(N, re, nv)
    if (N <= 0).any():
        raise SystemExit("an edge's bump has no support")
    coef = z / N[:, None]
    out["seconds_norm"] = round(time.time() - t00, 1)
    out["C_curl_max_rel"] = control_curl(level, rho, coef)
    print("pass 1 done; C-curl", out["C_curl_max_rel"], flush=True)
    # pass 2: arms
    Kv = {arm: np.zeros((ne, ne)) for arm in ARMS}; Ks = np.zeros((ne, ne))
    rec = {arm: np.zeros(ne) for arm in ARMS}; tv = {arm: np.zeros(ne) for arm in ARMS}
    cnum = {arm: 0.0 for arm in ARMS}; cden = {arm: 0.0 for arm in ARMS}
    mom = np.zeros((ne, 3)); seam = {}
    with ctx.Pool(nproc, initializer=_init, initargs=(level, rho, coef)) as pool:
        for k, r in enumerate(pool.imap_unordered(_w_cell, range(len(cells)), chunksize=2)):
            e = r["edges"]; ix = np.ix_(e, e)
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
    out["seconds_cells"] = round(time.time() - t00, 1)
    mres = np.linalg.norm(z - mom, axis=1) / l1
    out["M1_moment_max_rel"] = float(mres.max())
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
    out["amplification_max"] = None   # reported in the results note from the bump norms if needed
    out["arms"] = {arm: res for arm, (res, _) in arms.items()}
    # gates at this level (the cross-level clauses are judged in the results note)
    s = out["arms"]["seated"]
    m0 = M0_RECEIPTS.get(level)
    out["gates_this_level"] = {
        "M1-moment (<= 1e-8)": out["M1_moment_max_rel"] <= 1e-8,
        "M1-variational (asym <= 1e-12)": s["asymmetry"] <= 1e-12,
        "M1-wobble (seated: none below 1, none negative)": (s.get("wobble_count_below_1") == 0
                                                           and s.get("negative_physical") == 0) if do_eig else None,
        "M1-seam (<= 1%)": out["seam_rms_jump_relative"] <= 0.01,
        "M1-poke (n = 2: <= M0/10)": (s["poke_term"] <= m0[0] / 10) if level == 1 else None,
        "M1-recon (<= 2 x M0)": (s["reconstruction_term"] <= 2 * m0[1]) if m0 else None}
    out["seconds"] = round(time.time() - t00, 1)
    receipt.write_text(json.dumps(out, indent=1) + "\n")
    print(json.dumps({k: v for k, v in out.items() if k != "arms"}, indent=1), flush=True)


if __name__ == "__main__":
    main()
