"""Exploratory probe (not gated): the base fit V2 would freeze.
For rings 2 and 3, and two bases, with no change (C = 0):
  linear  the published closed-linear fit (equality-constrained least squares, own faces exact)
  quad    fit closed linear + closed quadratic (26 coefficients) the same way; keep the linear part
          at the chart origin (the cell's projected centroid), where homogeneous quadratics vanish
reports the variational force's Proposition-1 terms, its BW05 error, the Petrov-Galerkin force's
BW05 error, the curvature error and the amplification. Check: linear base at rings 2 reproduces
the paper (PG 6.22%/3.83%, variational 45.4%/276%).
usage: python scripts/ricci_variational_baseprobe.py LEVEL [RECEIPT.json]"""
import json, sys, time
from pathlib import Path
import numpy as np
sys.path[:0] = [str(Path(__file__).resolve().parent), str(Path(__file__).resolve().parent.parent / "e2e/probes")]
import ricci_variational_v1 as V
import ricci_variational_quadcount_lib as QL
from ricci_consistency_math import solve_rhs
pg, tw = V.pg, V.tw


def fit_maps(cells, faces, ci, rings):
    nb = tw.neighbourhood_rings(cells, faces, ci, rings)
    to_orth, J, t0 = pg.chart(nb[ci])
    seen, own, rest = set(), [], []
    for nj in [ci] + [k for k in nb if k != ci]:
        O = to_orth(nb[nj])
        for f in pg.FACES:
            key = pg.face_key(cells[nj], f)
            if key in seen:
                continue
            seen.add(key); tri = O[list(f)]
            row = (pg.flux_row(tri), QL.quad_flux(tri), pg.face_edge_terms(cells[nj], f))
            (own if nj == ci else rest).append(row)
    rows = own + rest
    Xl = np.array([r[0] for r in rows]); Xq = np.array([r[1] for r in rows])
    terms = [r[2] for r in rows]; n_own = len(own)

    def eclsq(X):
        A, R = X[:n_own], X[n_own:]
        Ap = np.linalg.pinv(A, rcond=pg.RCOND)
        _, sa, vt = np.linalg.svd(A)
        ra = int((sa > pg.RCOND * sa[0]).sum())
        N = vt[ra:].T
        RNp = np.linalg.pinv(R @ N, rcond=pg.RCOND)
        return np.hstack([Ap - N @ RNp @ R @ Ap, N @ RNp])
    M_lin = eclsq(Xl)
    M_quad = eclsq(np.hstack([Xl, Xq]))[:11]
    return {"J": J, "t0": t0, "terms": terms, "Xl": Xl, "Xq": Xq, "linear": M_lin, "quad": M_quad}


def main():
    level = int(sys.argv[1]); receipt = Path(sys.argv[2]) if len(sys.argv) > 2 else None
    t00 = time.time()
    L = pg.load_level(level, V.ORDER)
    cells, a, mass, vol = L["cells"], np.asarray(L["a"]), L["mass"], L["volume"]
    faces = pg.face_index(cells); ne = len(a)
    norm = lambda v: float(np.sqrt(max(0., v @ (mass @ v)) / vol))     # noqa: E731
    r4 = norm(-4 * a)
    rel = lambda K: norm(solve_rhs(mass, -(K @ a)) + 4 * a) / r4         # noqa: E731
    out = {"level": level, "kind": "exploratory probe, not gated"}
    for rings in (2, 3):
        data = []
        for ci, c in enumerate(cells):
            fm = fit_maps(cells, faces, ci, rings)
            tcols = sorted({e for t in fm["terms"] for e, _ in t}); col = {e: p for p, e in enumerate(tcols)}
            D = np.zeros((len(fm["terms"]), len(tcols)))
            for fi, t in enumerate(fm["terms"]):
                for e, cf in t:
                    D[fi, col[e]] += cf
            q = pg.quadrature(np.array(c["vertices"], float)[None], V.ORDER)
            g, w = q["g"][0], q["measure"][0] / q["det"][0]
            xo = q["xref"] @ fm["J"].T + fm["t0"]
            Porth = pg.BC[None] + np.einsum("kij,qj->qki", pg.BB, xo)
            trial = np.linalg.det(fm["J"]) * np.einsum("ij,qkj->qki", np.linalg.inv(fm["J"]), Porth)
            G = np.einsum("qka,qab,qlb,q->kl", trial, g, trial, w)
            Wp = np.einsum("qfa,qab,qkb,q->fk", q["two"], g, trial, w)
            s = np.where(c["reverse"], -1., 1.)
            TW = pg.INCIDENCE * s[None, :]
            Pex = pg.exact_proxy_from_maurer(q["maurer"])[0]
            vF = np.einsum("qka,qab,qb,q->k", trial, g, Pex, w)
            uF = np.einsum("qfa,qab,qb,q->f", q["two"], g, Pex, w)
            data.append(dict(c=c, tcols=tcols, D=D, G=G, Wp=Wp, TW=TW, trial=trial, g=g, w=w, Pex=Pex,
                             vF=vF, uF=uF, maps={k: fm[k] for k in ("linear", "quad")},
                             biasq={k: float(np.linalg.norm(fm[k] @ fm["Xq"]) / max(np.linalg.norm(fm["Xq"]), 1e-300))
                                    for k in ("linear", "quad")}))
        for base in ("linear", "quad"):
            Kv = np.zeros((ne, ne)); Kp = np.zeros((ne, ne)); rec = np.zeros(ne); tv = np.zeros(ne)
            cnum = cden = 0.0; amp = []
            for d in data:
                c, M = d["c"], d["maps"][base]
                T = M @ d["D"]; wt = float(c["weight"]); tc = d["tcols"]
                Kv[np.ix_(tc, tc)] += wt * T.T @ d["G"] @ T
                Kp[np.ix_(list(c["edges"]), tc)] += wt * d["TW"].T @ d["Wp"] @ T
                coeff = T @ a[tc]
                P = np.einsum("qka,k->qa", d["trial"], coeff); diff = P - d["Pex"]
                rec[tc] += wt * (T.T @ np.einsum("qka,qab,qb,q->k", d["trial"], d["g"], diff, d["w"]))
                tv[tc] += wt * (T.T @ d["vF"]); tv[list(c["edges"])] -= wt * (d["TW"].T @ d["uF"])
                cnum += wt * float(np.einsum("qa,qab,qb,q->", diff, d["g"], diff, d["w"]))
                cden += wt * float(np.einsum("qa,qab,qb,q->", d["Pex"], d["g"], d["Pex"], d["w"]))
                amp.append(float(np.linalg.norm(M, 2)))
            key = f"rings{rings}_{base}"
            out[key] = {"pg_error": rel(Kp), "variational_error": rel(Kv),
                        "reconstruction_term": norm(solve_rhs(mass, rec)) / r4,
                        "test_variation_term": norm(solve_rhs(mass, tv)) / r4,
                        "curvature_relative_error": float(np.sqrt(cnum / cden)),
                        "amplification_median_max": [float(np.median(amp)), float(np.max(amp))],
                        "quadratic_bias_median": float(np.median([d["biasq"][base] for d in data]))}
            print(key, json.dumps(out[key]), flush=True)
    out["seconds"] = round(time.time() - t00, 1)
    if receipt:
        receipt.parent.mkdir(parents=True, exist_ok=True); receipt.write_text(json.dumps(out, indent=1) + "\n")


if __name__ == "__main__":
    main()
