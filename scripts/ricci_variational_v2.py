"""V2: variational force from a constrained three-ring recovery, change kept off closed linear and
quadratic fields. Spec: docs/VARIATIONAL_RECOVERY_V2_SPEC.md (with Amendments 1-3). Research
script, not an engine verb. Abelian control of *Reading the Curve*, q6.

Arms (all at three rings unless --rings is given):
  base   linear | quad | hybrid      (Amendment 1: quad = V2q; Amendment 3: hybrid = V2h)
  excl   N2 (left null of [X_lin | X_quad], the V2 construction) | N1 (V1's, the removal arm)
One process computes every requested arm, sharing the geometry and the per-edge pairings.

usage: python scripts/ricci_variational_v2.py LEVEL OUTDIR [--rings R] [--arms linear:N2,quad:N2,...]
       [--c0only]   (C = 0 only: C-repro)
"""
import json
import sys
import time
from pathlib import Path

import numpy as np
from scipy.linalg import eigh
from scipy.sparse import coo_matrix
from scipy.sparse.linalg import lsqr

HERE = Path(__file__).resolve().parent
sys.path[:0] = [str(HERE), str(HERE.parent / "e2e/probes")]
import ricci_variational_v1 as V                 # noqa: E402  (sets pg.BASE, chart helpers)
import ricci_variational_baseprobe as BP         # noqa: E402  (fit_maps: linear, quad, hybrid)
from ricci_consistency_math import solve_rhs     # noqa: E402

pg, tw = V.pg, V.tw
ORDER = V.ORDER
ALL_ARMS = [("linear", "N2"), ("quad", "N2"), ("hybrid", "N2"),
            ("linear", "N1"), ("quad", "N1"), ("hybrid", "N1")]


def null_left(X):
    """Orthonormal basis of the left null space of X (nf x p): nf x (nf - rank)."""
    u, s, vt = np.linalg.svd(X.T)
    r = int((s > pg.RCOND * s[0]).sum())
    return vt[r:].T


class Cell:
    def __init__(self, cells, faces, ci, rings):
        c = cells[ci]
        self.ci, self.weight = ci, float(c["weight"])
        self.verts = np.array(c["vertices"], float)
        self.nb = tw.neighbourhood_rings(cells, faces, ci, rings)
        self.cells_ref = cells
        fm = BP.fit_maps(cells, faces, ci, rings)
        self.maps = {k: fm[k] for k in ("linear", "quad", "hybrid")}
        self.Xl, self.Xq, n_own = fm["Xl"], fm["Xq"], fm["n_own"]
        self.n_own = n_own
        terms = fm["terms"]
        self.tcols = sorted({e for t in terms for e, _ in t})
        self.col = {e: p for p, e in enumerate(self.tcols)}
        self.D = np.zeros((len(terms), len(self.tcols)))
        for fi, t in enumerate(terms):
            for e, cf in t:
                self.D[fi, self.col[e]] += cf
        A = self.Xl[:n_own]
        _, sa, vta = np.linalg.svd(A)
        ra = int((sa > pg.RCOND * sa[0]).sum())
        self.Z = vta[ra:].T                                             # 11 x (11 - ra)
        self.Ns = {"N1": null_left(self.Xl), "N2": null_left(np.hstack([self.Xl, self.Xq]))}
        self.NDs = {k: N.T @ self.D for k, N in self.Ns.items()}
        J, t0 = fm["J"], fm["t0"]
        q = pg.quadrature(self.verts[None], ORDER)
        self.lam, self.g, self.w = q["lam"], q["g"][0], q["measure"][0] / q["det"][0]
        xo = q["xref"] @ J.T + t0
        Porth = pg.BC[None] + np.einsum("kij,qj->qki", pg.BB, xo)
        self.trial = np.linalg.det(J) * np.einsum("ij,qkj->qki", np.linalg.inv(J), Porth)
        self.two = q["two"]
        self.G = np.einsum("qka,qab,qlb,q->kl", self.trial, self.g, self.trial, self.w)
        s = np.where(c["reverse"], -1., 1.)
        self.TW = pg.INCIDENCE * s[None, :]
        self.wedges = list(c["edges"])
        self.Pex = pg.exact_proxy_from_maurer(q["maurer"])[0]
        self.Wp = np.einsum("qfa,qab,qkb,q->fk", self.two, self.g, self.trial, self.w)

    def pair(self, P):
        v = np.einsum("qka,qab,qmb,q->km", self.trial, self.g, P, self.w)
        u = np.einsum("qfa,qab,qmb,q->fm", self.two, self.g, P, self.w)
        return v, u


def main():
    args = sys.argv[1:]
    rings = 3
    arms = ALL_ARMS
    c0only = "--c0only" in args
    if c0only:
        args.remove("--c0only")
    if "--rings" in args:
        i = args.index("--rings"); rings = int(args[i + 1]); del args[i:i + 2]
    if "--arms" in args:
        i = args.index("--arms")
        arms = [tuple(x.split(":")) for x in args[i + 1].split(",")]; del args[i:i + 2]
    level, outdir = int(args[0]), Path(args[1])
    outdir.mkdir(parents=True, exist_ok=True)
    if not c0only:
        for base, excl in arms:                      # review (b): refuse before computing, not after
            if (outdir / f"L{level}_{base}_{excl}.json").exists():
                raise SystemExit(f"refusing to overwrite {outdir / f'L{level}_{base}_{excl}.json'}")
    t0 = time.time()
    L = pg.load_level(level, ORDER)
    cells, a, mass, vol = L["cells"], np.asarray(L["a"]), L["mass"], L["volume"]
    faces = pg.face_index(cells)
    C = [Cell(cells, faces, ci, rings) for ci in range(len(cells))]
    t_cells = time.time() - t0
    print("cells ready", round(t_cells, 1), "s", flush=True)
    ne = len(a)
    norm = lambda v: float(np.sqrt(max(0., v @ (mass @ v)) / vol))     # noqa: E731
    r4 = norm(-4 * a)
    rel = lambda K: norm(solve_rhs(mass, -(K @ a)) + 4 * a) / r4         # noqa: E731

    def assemble(Ts):
        Kv = np.zeros((ne, ne)); Kp = np.zeros((ne, ne))
        for c, T in zip(C, Ts):
            Kv[np.ix_(c.tcols, c.tcols)] += c.weight * T.T @ c.G @ T
            Kp[np.ix_(c.wedges, c.tcols)] += c.weight * c.TW.T @ c.Wp @ T
        return Kv, Kp

    def terms(Ts):
        rec = np.zeros(ne); tv = np.zeros(ne); cnum = cden = 0.0
        for c, T in zip(C, Ts):
            coeff = T @ a[c.tcols]
            P = np.einsum("qka,k->qa", c.trial, coeff); diff = P - c.Pex
            rec[c.tcols] += c.weight * (T.T @ np.einsum("qka,qab,qb,q->k", c.trial, c.g, diff, c.w))
            vF = np.einsum("qka,qab,qb,q->k", c.trial, c.g, c.Pex, c.w)
            uF = np.einsum("qfa,qab,qb,q->f", c.two, c.g, c.Pex, c.w)
            tv[c.tcols] += c.weight * (T.T @ vF); tv[c.wedges] -= c.weight * (c.TW.T @ uF)
            cnum += c.weight * float(np.einsum("qa,qab,qb,q->", diff, c.g, diff, c.w))
            cden += c.weight * float(np.einsum("qa,qab,qb,q->", c.Pex, c.g, c.Pex, c.w))
        return {"reconstruction_term": norm(solve_rhs(mass, rec)) / r4,
                "test_variation_term": norm(solve_rhs(mass, tv)) / r4,
                "curvature_relative_error": float(np.sqrt(cnum / cden))}

    common = {"level": level, "rings": rings, "cells": len(C), "edges": ne,
              "kind": "V2 (research script, not an engine run)"}
    # C = 0 for every base (the "before"; C-repro at rings = 2, linear)
    before = {}
    for base in sorted({b for b, _ in arms}):
        Ts = [c.maps[base] @ c.D for c in C]
        Kv, Kp = assemble(Ts)
        before[base] = {"variational_error": rel(Kv), "pg_error": rel(Kp), **terms(Ts),
                        "amplification_median_max": [float(np.median([np.linalg.norm(c.maps[base], 2) for c in C])),
                                                     float(np.max([np.linalg.norm(c.maps[base], 2) for c in C]))]}
        print("C=0", base, json.dumps(before[base]), flush=True)
    if c0only:
        out = {**common, "before": before, "seconds": round(time.time() - t0, 1)}
        (outdir / f"L{level}_rings{rings}_c0.json").write_text(json.dumps(out, indent=1) + "\n")
        return

    # C-quad / C-count / C-copies (geometry, base-independent)
    cquad = {"linear": max(float(np.linalg.norm(c.Ns["N2"].T @ c.Xl) / np.linalg.norm(c.Xl)) for c in C),
             "quadratic": max(float(np.linalg.norm(c.Ns["N2"].T @ c.Xq) / np.linalg.norm(c.Xq)) for c in C)}
    live = {k: [int(np.linalg.matrix_rank(c.NDs[k], tol=1e-9 * max(np.abs(c.NDs[k]).max(), 1e-300)))
                if c.NDs[k].size else 0 for c in C] for k in ("N1", "N2")}

    # per-edge geometry: reference basis Qe, pairings of every user cell, Whitney rhs (shared)
    left_is = "minus"           # as detected by every V1/V1b receipt (C-span); not re-detected here
    plus, minus, _ = V.generators_on(C[0], C[0].verts)

    def enforced(c, verts):
        p_, m_, lin = V.generators_on(c, verts)
        return np.concatenate([m_ if left_is == "plus" else p_, lin], axis=1)
    owner = {}
    for ci, c in enumerate(cells):
        for e in c["edges"]:
            owner.setdefault(e, ci)
    users = {}
    for k, c in enumerate(C):
        for e in c.tcols:
            users.setdefault(e, []).append(k)
    edge_geo, multi = [], 0
    t1 = time.time()
    for e in range(ne):
        r = owner[e]
        i0, j0 = pg.EDGES[cells[r]["edges"].index(e)]
        Pv, Qv = C[r].verts[i0], C[r].verts[j0]
        gref = enforced(C[r], C[r].verts)
        Sg = np.einsum("qma,qab,qnb,q->mn", gref, C[r].g, gref, C[r].w)
        ev, evec = np.linalg.eigh(Sg)
        keep = ev > 1e-10 * ev.max()
        Qe = evec[:, keep] / np.sqrt(ev[keep])
        vQs = []
        for k in users.get(e, []):
            h, ncopies = V.lift_to(C[k], e, Pv, Qv)
            multi += ncopies > 1
            Vk = np.array([pg.qmul(h, v) for v in C[k].verts])
            v, _ = C[k].pair(enforced(C[k], Vk))
            vQs.append((k, v @ Qe))
        rhs_w = np.zeros(Qe.shape[1])
        for k, c in enumerate(cells):
            if e in c["edges"]:
                jj = c["edges"].index(e)
                a0, b0 = pg.EDGES[jj]
                p, q = C[k].verts[a0], C[k].verts[b0]
                hk = None
                for A_, B_ in ((Pv, Qv), (Qv, Pv)):
                    h = pg.qmul(A_, pg.qconj(p))
                    if np.abs(pg.qmul(h, q) - B_).max() < 1e-9:
                        hk = h; break
                if hk is None:
                    raise RuntimeError(f"cell {k} does not carry edge {e} onto the reference")
                Vk = np.array([pg.qmul(hk, v) for v in C[k].verts])
                _, u = C[k].pair(enforced(C[k], Vk))
                rhs_w += C[k].weight * (C[k].TW[:, jj] @ (u @ Qe))
        edge_geo.append((vQs, rhs_w, Qe.shape[1]))
    t_geo = time.time() - t1
    print("edge geometry", round(t_geo, 1), "s; cells seeing two copies:", multi, flush=True)
    common.update({"C_copies_cells_seeing_two_copies_of_an_edge": int(multi),
                   "C_quad_max": cquad,
                   "C_count_live_median_N1_N2": [float(np.median(live["N1"])), float(np.median(live["N2"]))],
                   "C_count_live_min_N1_N2": [int(min(live["N1"])), int(min(live["N2"]))],
                   "seconds_cells": round(t_cells, 1), "seconds_edge_geometry": round(t_geo, 1)})

    for base, excl in arms:
        ta = time.time()
        res = {**common, "base": base, "exclusion": excl, "before": before[base]}
        # constraint matrix A (rows x sum of C unknowns) and rhs b
        offs, tot = {}, 0
        for k, c in enumerate(C):
            offs[k] = tot; tot += c.Z.shape[1] * c.Ns[excl].shape[1]
        ri, ci_, vv, b, r0 = [], [], [], [], 0
        for e, (vQs, rhs_w, m) in enumerate(edge_geo):
            rhs_e = rhs_w.copy()
            for k, vQ in vQs:
                c = C[k]; ck = c.col[e]
                T = c.maps[base] @ c.D
                rhs_e -= c.weight * (T[:, ck] @ vQ)
                co = c.weight * np.einsum("pm,q->mpq", c.Z.T @ vQ, c.NDs[excl][:, ck]).reshape(m, -1)
                rr, cc = np.nonzero(co)
                ri.append(r0 + rr); ci_.append(offs[k] + cc); vv.append(co[rr, cc])
            b.append(rhs_e); r0 += m
        b = np.concatenate(b)
        A = coo_matrix((np.concatenate(vv), (np.concatenate(ri), np.concatenate(ci_))), shape=(r0, tot)).tocsr()
        res.update({"constraint_rows": int(r0), "C_unknowns": int(tot), "A_nnz": int(A.nnz)})
        # primary: lsqr from x0 = 0 (minimum-norm least squares); stops when consistent or LS-optimal
        t2 = time.time()
        sol = lsqr(A, b, atol=1e-12, btol=1e-9, iter_lim=20_000)      # Amendment 4
        x1 = sol[0]
        r1 = float(np.linalg.norm(A @ x1 - b) / np.linalg.norm(b))
        res.update({"lsqr_residual": r1, "lsqr_iterations": int(sol[2]), "lsqr_istop": int(sol[1]),
                    "lsqr_seconds": round(time.time() - t2, 1)})
        print(base, excl, "lsqr", r1, sol[2], flush=True)
        # cross-check: dense eigh of A A^T, cut 1e-12 (V1's solve)
        t3 = time.time()
        AAt = (A @ A.T).toarray()
        ev, evec = eigh(AAt)
        keep = ev > 1e-12 * ev.max()
        y = evec[:, keep] @ ((evec[:, keep].T @ b) / ev[keep])
        x2 = A.T @ y
        r2 = float(np.linalg.norm(A @ x2 - b) / np.linalg.norm(b))
        del AAt, evec
        res.update({"eigh_residual": r2, "eigh_rank": int(keep.sum()), "eigh_seconds": round(time.time() - t3, 1),
                    "solutions_rel_diff": float(np.linalg.norm(x1 - x2) / max(np.linalg.norm(x1), 1e-300))})
        x = x1 if r1 <= r2 else x2
        res["solve_used"] = "lsqr" if r1 <= r2 else "eigh"
        res["C_solve_relative_residual"] = min(r1, r2)
        res["C_solve_pass"] = bool(min(r1, r2) <= 1e-8)
        # new maps and controls
        Ts, dM, amp0, amp1 = [], [], [], []
        ex_own = ex_lin = ex_quad = 0.0
        for k, c in enumerate(C):
            nz, nq = c.Z.shape[1], c.Ns[excl].shape[1]
            Ck = x[offs[k]:offs[k] + nz * nq].reshape(nz, nq)
            M0 = c.maps[base]
            dMk = c.Z @ Ck @ c.Ns[excl].T
            Mn = M0 + dMk
            Ts.append(Mn @ c.D)
            dM.append(float(np.linalg.norm(Ck)))
            amp0.append(float(np.linalg.norm(M0, 2))); amp1.append(float(np.linalg.norm(Mn, 2)))
            ex_own = max(ex_own, float(np.abs(c.Xl[:c.n_own] @ dMk).max()))            # change leaves own fluxes
            ex_lin = max(ex_lin, float(np.abs(dMk @ c.Xl).max()))                       # linear fields untouched
            ex_quad = max(ex_quad, float(np.abs(dMk @ c.Xq).max()))                     # quadratic response frozen
        res.update({"C_exact_change_own_max": ex_own, "C_exact_change_linear_max": ex_lin,
                    "C_exact_change_quad_max": ex_quad,
                    "change_norm_median_max": [float(np.median(dM)), float(np.max(dM))],
                    "amplification_median_before_after": [float(np.median(amp0)), float(np.median(amp1))],
                    "amplification_max_before_after": [float(np.max(amp0)), float(np.max(amp1))]})
        Kv, Kp = assemble(Ts)
        res["variational_error"] = rel(Kv)
        res["pg_error"] = rel(Kp)                    # review (f): PG force with T' = M'D
        res["pg_error_base_C0"] = before[base]["pg_error"]
        res["terms_after"] = terms(Ts)
        res["asymmetry"] = float(np.abs(Kv - Kv.T).max() / np.abs(Kv).max())
        Mden = mass.toarray() if hasattr(mass, "toarray") else np.asarray(mass)
        lam = eigh(0.5 * (Kv + Kv.T), Mden, eigvals_only=True)
        phys = lam[np.abs(lam) >= 1]
        res.update({"S3_small_count": int((np.abs(lam) < 1).sum()), "S3_negative_below_minus1": int((lam < -1).sum()),
                    "S3_min_physical": float(phys.min()) if len(phys) else None})
        whitney = {0: 0.298, 1: 0.260}.get(level)
        gauge = {0: 21, 1: 163}.get(level)
        res["gates"] = {"P_recon_le_10pct": bool(res["terms_after"]["reconstruction_term"] <= 0.10),
                        "V2_acc_below_whitney": None if whitney is None else bool(res["variational_error"] < whitney),
                        "V2_stable": None if gauge is None else bool(res["S3_negative_below_minus1"] == 0 and
                                                                     res["S3_small_count"] == gauge)}
        res["seconds_arm"] = round(time.time() - ta, 1)
        print(base, excl, json.dumps({k: res[k] for k in ("variational_error", "terms_after", "gates",
                                                         "change_norm_median_max", "amplification_median_before_after",
                                                         "C_solve_relative_residual")}), flush=True)
        path = outdir / f"L{level}_{base}_{excl}.json"
        np.save(path.with_suffix(".C.npy"), x)       # review (c): keep the solution
        path.write_text(json.dumps(res, indent=1) + "\n")


if __name__ == "__main__":
    main()
