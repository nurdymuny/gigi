"""Option 1 as a DIAGNOSTIC, not a gated feature: Whitney plus a zero-mean correction.

Prediction stated before running (docs/RICCI_ZERO_MEAN_RESULTS.md): curvature
order about 1, because a zero-mean correction keeps Whitney's cell means, which
are first-order accurate (measured 0.93, 0.98); variational force bounded but
flooring, because zero mean alone leaves an order-one transpose term.

Construction on cell K:  R_K a = W_K a + (P_K a - mean_K(P_K a)),
W_K the Whitney (lowest-order) curvature, P_K the closed linear star fit from
ricci_pg_curvature, mean_K the projection onto constants in the face_mass
inner product. So R keeps Whitney's mean exactly and takes only the fit's
variation. Variational force: gradient of one half sum over K of the integral
of R_K-transpose g R_K over root det g. Same mass, data and BW05 measurement.

Controls run first, refuse otherwise: the correction has zero weighted mean on
every cell; R maps pure gauge to zero is implied by W and P both doing so;
Whitney-only (correction dropped) must reproduce native stiffness @ a.
"""
import json
import sys
import time
from pathlib import Path

import numpy as np
from scipy.sparse import coo_matrix

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_pg_curvature as pg            # noqa: E402
from ricci_consistency_math import INCIDENCE, solve_rhs   # noqa: E402


def run(level, with_correction=True):
    L = pg.load_level(level, 6)
    cells, a, mass, vol = L["cells"], L["a"], L["mass"], L["volume"]
    faces = pg.face_index(cells)
    rows, cols, vals = [], [], []
    num = den = 0.
    worst_mean = 0.
    for start in range(0, len(cells), 400):
        idx = range(start, min(start + 400, len(cells)))
        q = pg.quadrature(np.array([cells[i]["vertices"] for i in idx], float), 6)
        wq = q["measure"] / q["det"]
        Pex = pg.exact_proxy_from_maurer(q["maurer"])
        for li, ci in enumerate(idx):
            cell = cells[ci]
            s = np.where(cell["reverse"], -1., 1.)
            g, w = q["g"][li], wq[li]
            Gm = np.einsum("qab,q->ab", g, w)
            fit = pg.fit_cell(cells, a, faces, ci)
            J, t0 = fit["J"], fit["t0"]
            xo = q["xref"] @ J.T + t0
            Porth = pg.BC[None] + np.einsum("kij,qj->qki", pg.BB, xo)
            fb = np.linalg.det(J) * np.einsum("ij,qkj->qki", np.linalg.inv(J), Porth)   # (Q,11,3)
            means = np.linalg.solve(Gm, np.einsum("qab,qkb,q->ak", g, fb, w)).T        # (11,3)
            zb = fb - means[None]                                                      # zero-mean basis
            check = np.linalg.solve(Gm, np.einsum("qab,qkb,q->ak", g, zb, w))
            worst_mean = max(worst_mean, float(np.max(abs(check))))
            tcols = sorted({e for t in fit["terms"] for e, _ in t} | set(cell["edges"]))
            col = {e: p for p, e in enumerate(tcols)}
            D = np.zeros((len(fit["terms"]), len(tcols)))
            for fi, t in enumerate(fit["terms"]):
                for e, cf in t:
                    D[fi, col[e]] += cf
            Wmap = np.zeros((4, len(tcols)))
            for k, e in enumerate(cell["edges"]):
                Wmap[:, col[e]] += INCIDENCE[:, k] * s[k]
            if with_correction:
                basis = np.concatenate([q["two"], zb], axis=1)                  # (Q,15,3)
                tmap = np.vstack([Wmap, fit["map"] @ D])                         # 15 x ncols
            else:
                basis, tmap = q["two"], Wmap
            G = np.einsum("qka,qab,qlb,q->kl", basis, g, basis, w)
            emap = float(cell["weight"]) * (tmap.T @ G @ tmap)
            rows.append(np.repeat(tcols, len(tcols)))
            cols.append(np.tile(tcols, len(tcols)))
            vals.append(emap.ravel())
            P = np.einsum("qkb,k->qb", basis, tmap @ a[tcols])
            d = P - Pex[li]
            num += float(np.einsum("qa,qab,qb,q->", d, g, d, w))
            den += float(np.einsum("qa,qab,qb,q->", Pex[li], g, Pex[li], w))
    if worst_mean > 1e-10:
        raise SystemExit(f"correction is not zero-mean: {worst_mean:.3e}")
    op = coo_matrix((np.concatenate(vals), (np.concatenate(rows), np.concatenate(cols))),
                    shape=(len(a), len(a))).tocsr()
    force = op @ a
    if not with_correction:
        ref = L["stiffness"] @ a
        if np.max(abs(force - ref)) / np.max(abs(ref)) > 1e-12:
            raise SystemExit("Whitney-only control does not reproduce native stiffness")
    v = solve_rhs(mass, -force)
    norm = lambda x: float(np.sqrt(max(0., x @ (mass @ x)) / vol))   # noqa: E731
    return {"level": level, "with_correction": with_correction,
            "relative_error": norm(v + 4 * a) / norm(4 * a),
            "curvature_relative_error": float(np.sqrt(num / den)),
            "worst_correction_mean": worst_mean}


def main():
    out_path = pg.OUT / "zero_mean_diagnostic.json"
    if out_path.exists():
        raise SystemExit(f"refusing to overwrite {out_path}")
    t = time.time()
    rows = [run(0, with_correction=False)]
    print("control (Whitney only, level 0):", f"{rows[0]['relative_error']:.6%}")
    for level in (0, 1, 2):
        r = run(level)
        rows.append(r)
        print(f"level {level}: force {r['relative_error']:.4%}  curvature {r['curvature_relative_error']:.4%}"
              f"  worst correction mean {r['worst_correction_mean']:.1e}")
    out_path.write_text(json.dumps({
        "kind": "diagnostic of option 1; not a feature gate",
        "rows": rows, "seconds": round(time.time() - t, 1),
        "source_sha256": {"scripts/ricci_zero_mean_diagnostic.py": pg.sha(__file__),
                          "scripts/ricci_pg_curvature.py": pg.sha(pg.__file__)}}, indent=2) + "\n")
    print("wrote", out_path.relative_to(pg.ROOT))


if __name__ == "__main__":
    main()
