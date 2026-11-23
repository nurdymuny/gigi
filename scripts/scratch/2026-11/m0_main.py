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
    common = sorted(cr)
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
    for arm in ARMS:
        Cm = c.curl_map(c.xq, rho, arm == "frozen")
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


