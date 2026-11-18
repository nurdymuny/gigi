"""Gate G2 of docs/RECOVER_CURVATURE_SU2_SPEC.md: the covariant two-ring recovery
on an exact non-Abelian field, the Levi-Civita spin connection of a left-invariant
metric, on the paper's frozen meshes.

The exact target uses no frames or sign conventions of its own. The repository's
exact transport (ricci_consistency_math.link_quaternions) is applied around a
vanishingly small triangle inside the cell; its holonomy logarithm is carried
straight back to the cell's frame vertex with the same exact transport. The
recovered field's flux through the same small triangle (mapped into the cell's
chart) is compared with it. Both sides are per small triangle and per su(2)
component.

usage: python scripts/ricci_su2_g2_probe.py METRIC LEVELS [CELLS_PER_LEVEL] [RECEIPT.json]
  METRIC  round | berger  ([1,1,1] or [3,1,1])
  LEVELS  e.g. 0,1,2,3
Sampled cells are drawn with a fixed seed; the count is printed and recorded.
"""
import json
import os
import sys
import time
from pathlib import Path

import numpy as np

DATA = Path(os.environ.get("GIGI_RICCI_DATA_ROOT", Path(__file__).resolve().parents[1]))
sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_pg_curvature as pg     # noqa: E402
pg.BASE = DATA / "output/ricci-shape-regular-20261117/berger"
pg.DIAG = DATA / "output/ricci-consistency-20261117/diagnostic"
import ricci_pg_tworing as tw      # noqa: E402
import ricci_su2_recovery as su    # noqa: E402
from ricci_consistency_math import link_quaternions, compose  # noqa: E402

METRICS = {"round": [1.0, 1.0, 1.0], "berger": [3.0, 1.0, 1.0]}
DELTA = 2e-5          # small-triangle size in chart units (cells have chart diameter 1)
TRIANGLES = 24        # small triangles per cell


def stored_links(cells, metric):
    """One exact link per stored edge, tail -> head, from any cell containing it."""
    n = 1 + max(max(c["edges"]) for c in cells)
    out = np.zeros((n, 4))
    done = np.zeros(n, bool)
    for c in cells:
        V = np.asarray(c["vertices"], float)
        for e, (i, j) in enumerate(pg.EDGES):
            g = c["edges"][e]
            if done[g]:
                continue
            tail, head = (i, j) if not c["reverse"][e] else (j, i)
            out[g] = link_quaternions(metric, V[tail], V[head])
            done[g] = True
    return out


def chart_inverse(K):
    """Inverse of pg.chart's projection: chart point z -> unit quaternion on S^3."""
    p0, b = K[0], (K[1:] - K[0]).T
    n = np.linalg.svd(b.T)[2][-1]
    c = float(n @ p0)
    if c < 0:
        n, c = -n, -c
    Q, _ = np.linalg.qr(b)
    cen = K.mean(axis=0)
    cen = cen * (c / (cen @ n))
    diam = max(np.linalg.norm(K[i] - K[j]) for i in range(4) for j in range(i + 1, 4))
    to_orth, _, _ = pg.chart(K)

    def inv(z):
        y = cen + diam * (Q @ np.asarray(z, float))
        return y / np.linalg.norm(y)
    # self-check: inverse really inverts the chart
    z_test = to_orth(K)[2]
    if np.max(np.abs(inv(z_test) - K[2])) > 1e-10:
        raise RuntimeError("chart inverse disagrees with pg.chart")
    return inv, to_orth


def measure(metric_name, level, n_cells, seed=11, register=True, abelianised=False):
    metric = METRICS[metric_name]
    base = pg.load_cells(level)
    cells = base["cells"]
    faces = pg.face_index(cells)
    vid = su.vertex_ids(cells)
    links = stored_links(cells, metric)
    zeros = np.zeros(len(links))
    rng = np.random.default_rng(seed)
    pick = sorted(rng.choice(len(cells), size=min(n_cells, len(cells)), replace=False).tolist())
    err2 = ref2 = 0.0
    for ci in pick:
        fit = tw.fit_cell_rings(cells, zeros, faces, ci, rings=2)
        r = su.recover_cell(cells, faces, vid, links, ci, fit=fit, register=register,
                            abelianised=abelianised)
        C = r["coefficients"]                      # 3 x 11
        K = np.asarray(cells[ci]["vertices"], float)
        inv, to_orth = chart_inverse(K)
        Z = to_orth(K)
        v0 = K[0]
        for _ in range(TRIANGLES):
            lam = rng.dirichlet(np.ones(4))
            z0 = lam @ Z
            u, w = rng.normal(size=3), rng.normal(size=3)
            tri = [z0, z0 + DELTA * u, z0 + DELTA * w]
            P = [inv(z) for z in tri]
            hol = compose(compose(link_quaternions(metric, P[0], P[1]),
                                  link_quaternions(metric, P[1], P[2])),
                          link_quaternions(metric, P[2], P[0]))
            exact = su.ad(link_quaternions(metric, v0, P[0]), su.qlog(hol))
            got = C @ pg.flux_row(np.array(tri))
            err2 += float(np.sum((got - exact) ** 2))
            ref2 += float(np.sum(exact ** 2))
    return {"metric": metric_name, "level": level, "cells_sampled": len(pick), "cells_total": len(cells),
            "triangles_per_cell": TRIANGLES, "relative_error": (err2 / ref2) ** 0.5,
            "register": register, "abelianised": abelianised}


def main():
    metric_name, levels = sys.argv[1], [int(x) for x in sys.argv[2].split(",")]
    n_cells = int(sys.argv[3]) if len(sys.argv) > 3 else 60
    receipt = Path(sys.argv[4]) if len(sys.argv) > 4 else None
    if receipt and receipt.exists():
        raise SystemExit(f"refusing to overwrite {receipt}")
    h = {0: 0.388139515370189, 1: 0.20166195340219167, 2: 0.13545926022811192, 3: 0.08159478457777686}
    out = {"variants": {}}
    for label, kw in [("full", {}), ("M1_no_register", {"register": False}),
                      ("M2_abelianised", {"abelianised": True})]:
        rows = []
        for level in levels:
            t = time.time()
            m = measure(metric_name, level, n_cells, **kw)
            m["seconds"] = round(time.time() - t, 1)
            rows.append(m)
            print(label, level, f"{100 * m['relative_error']:.4f}%", f"{m['seconds']}s", flush=True)
        orders = [float(np.log(rows[i]["relative_error"] / rows[i + 1]["relative_error"])
                        / np.log(h[rows[i]["level"]] / h[rows[i + 1]["level"]])) for i in range(len(rows) - 1)]
        out["variants"][label] = {"rows": rows, "observed_orders": orders}
        print(label, "orders", [round(o, 2) for o in orders], flush=True)
    if receipt:
        out.update({"kind": "G2 probe, research script (not an engine run)", "delta": DELTA,
                    "metric": METRICS[metric_name], "cells_per_level": n_cells})
        receipt.write_text(json.dumps(out, indent=2) + "\n")
        print("wrote", receipt)


if __name__ == "__main__":
    main()
