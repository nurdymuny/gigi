"""M1 Part 1: the moment audit of M0 (docs/MAGNET_M1_SPEC.md). Research script, not an engine run.

For every edge e, with psi_e = R1 delta_e the M0 reading's bump (seamless arm) and W_e the Whitney tent:
  z_e^a = sum_K w_K int_K (psi_e - W_e)(e_a) dvol,   a = 0, 1, 2   (left-invariant frame e_a, dual to theta^a)
and M0's poke vector, recomputed exactly as M0 assembles it:
  t_e = <F, curl psi_e> - <F, W^2 D delta_e>.
For this control A = theta^0, F = dA, delta F = 4 theta^0 (constant in the frame), so Lemma A predicts
  p_e = 4 z_e^0   exactly (up to quadrature).
Reports the poke term from t (M0 reproduction) and from p, their dual-mass-norm cosine, the relative residual,
and the distribution of |z_e| / ||W_e||_L1; saves z for M1.

usage: python scripts/ricci_magnet_m0_moment_audit.py LEVEL OUTDIR
"""
import json
import multiprocessing as mp
import os
import sys
import time
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path[:0] = [str(HERE), str(HERE.parent / "e2e/probes")]
import ricci_magnet_m0 as M                      # noqa: E402
from ricci_consistency_math import solve_rhs     # noqa: E402

pg, ein, ORDER = M.pg, M.ein, M.ORDER
_W = {}


def _init(level, rho):
    os.environ.setdefault("OMP_NUM_THREADS", "1")
    L = pg.load_level(level, ORDER)
    _W.update(cells=L["cells"], faces=pg.face_index(L["cells"]), rho=rho)


def _cell(ci):
    cells, faces, rho = _W["cells"], _W["faces"], _W["rho"]
    c = M.Cell(cells, faces, ci, rho)
    cell = cells[ci]; wt = c.weight
    q = pg.quadrature(c.verts[None], ORDER)
    maurer = q["maurer"][0]; mu = q["measure"][0]; lam = q["lam"]
    Einv = np.linalg.inv(maurer)                                  # e_a = sum_b Einv[b, a] d/dxi_b
    # the reading's bumps, as reference covectors, on the frame
    R = c.reading_map(c.xq, rho)                                  # (Q,3,E) chart covector components
    Rxi = ein("qje,jb->qbe", R, c.ch.J)                           # reference covector components
    zr = wt * ein("qbe,qba,q->ea", Rxi, Einv, mu)                 # (E,3)
    # Whitney tents of the cell's own edges (global orientation)
    s = np.where(cell["reverse"], -1., 1.)
    zw = np.zeros((6, 3)); l1 = np.zeros(6)
    for k, (p, qq) in enumerate(pg.EDGES):
        Wxi = s[k] * (lam[:, p, None] * pg.GRADS[qq] - lam[:, qq, None] * pg.GRADS[p])
        Wf = ein("qb,qba->qa", Wxi, Einv)
        zw[k] = wt * ein("qa,q->a", Wf, mu)
        l1[k] = wt * float(np.sum(np.linalg.norm(Wf, axis=1) * mu))
    # M0's poke vector, seamless arm, exactly as ricci_magnet_m0._w_cell assembles it
    Cm = c.curl_map(c.xq, rho, False)
    Pr = ein("ab,pbe->pae", c.piola, Cm)
    tv = wt * ein("pae,pab,pb,p->e", Pr, c.g, c.Pex, c.w)
    uF = ein("qfa,qab,qb,q->f", c.two, c.g, c.Pex, c.w)
    tvown = wt * (c.TW.T @ uF)
    return {"edges": c.edges, "own": list(cell["edges"]), "zr": zr, "zw": zw, "l1": l1, "tv": tv, "tvown": tvown}


def main():
    level, outdir = int(sys.argv[1]), Path(sys.argv[2])
    outdir.mkdir(parents=True, exist_ok=True)
    receipt = outdir / f"audit_L{level}.json"
    if receipt.exists():
        raise SystemExit(f"refusing to overwrite {receipt}")
    t0 = time.time()
    L = pg.load_level(level, ORDER)
    cells, a, mass, vol = L["cells"], np.asarray(L["a"]), L["mass"], L["volume"]
    ne = len(a)
    hmax = json.loads((M.V._DATA / "output/ricci-variational-20261117/abelian/diagnostic.json").read_text()
                      )["cases"][f"level{level}_q6"]["h_max"]
    rho = 1.0 * hmax                                              # M0's reach at kappa = 1
    z_read = np.zeros((ne, 3)); z_wh = np.zeros((ne, 3)); l1 = np.zeros(ne); t = np.zeros(ne)
    nproc = int(os.environ.get("M0_PROCS", "8"))
    with mp.get_context("spawn").Pool(nproc, initializer=_init, initargs=(level, rho)) as pool:
        for k, r in enumerate(pool.imap_unordered(_cell, range(len(cells)), chunksize=2)):
            e = r["edges"]
            z_read[e] += r["zr"]; t[e] += r["tv"]
            np.add.at(z_wh, r["own"], r["zw"]); np.add.at(l1, r["own"], r["l1"]); np.add.at(t, r["own"], -r["tvown"])
            if (k + 1) % 200 == 0:
                print(f"  {k + 1}/{len(cells)} cells, {time.time() - t0:.0f} s", flush=True)
    z = z_read - z_wh
    p = 4.0 * z[:, 0]
    norm = lambda v: float(np.sqrt(max(0., v @ (mass @ v)) / vol))     # noqa: E731
    r4 = norm(-4 * a)
    ut, up = solve_rhs(mass, t), solve_rhs(mass, p)
    ip = lambda x, y: float(x @ (mass @ y))                             # noqa: E731
    cos = ip(ut, up) / np.sqrt(ip(ut, ut) * ip(up, up))
    scale = ip(ut, up) / ip(up, up)
    rel = np.linalg.norm(z, axis=1) / l1
    out = {"level": level, "edges": ne, "cells": len(cells), "rho": rho, "kind": "M1 Part 1 audit (research script)",
           "poke_term_from_t": norm(ut) / r4, "poke_term_from_p": norm(up) / r4,
           "cosine_t_p_dual_mass": cos, "best_scale_t_over_p": scale,
           "residual_rel": norm(ut - up) / norm(ut),
           "residual_rel_after_best_scale": norm(ut - scale * up) / norm(ut),
           "zeroth_moment_defect_rel": {"median": float(np.median(rel)), "p95": float(np.percentile(rel, 95)),
                                        "max": float(rel.max()),
                                        "per_component_median": [float(np.median(np.abs(z[:, k]) / l1)) for k in range(3)]},
           "constants_check_sum_z": float(np.abs(z.sum(0)).max() / np.abs(z).sum()),
           "seconds": round(time.time() - t0, 1)}
    np.savez(outdir / f"moments_L{level}.npz", z=z, z_read=z_read, z_whitney=z_wh, l1=l1, t=t, p=p)
    receipt.write_text(json.dumps(out, indent=1) + "\n")
    print(json.dumps(out, indent=1), flush=True)


if __name__ == "__main__":
    main()
