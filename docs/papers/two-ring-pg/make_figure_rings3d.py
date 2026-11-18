"""Figure: the cell and its two rings in 3-D, with the fold's choices (stills of the 3-D viewer).

  data/rings_3d.json        the geometry: three n = 2 cells, each with its one- and two-ring
                            neighbourhood in its own chart (chordal units), the ring of every
                            cell, the fold's chosen second-ring cells and the amplifications
  figures/fig_rings_3d.pdf  top row: cell 0 built up (one ring, two rings, folded stencil);
                            bottom row: pulled apart, for cell 0, a cell the fold serves with 2
                            second-ring cells and one that needs 7

Fold rule as docs/RING_FOLD_SPEC.md (greedy, stop within 1.25x of the two-ring amplification).
usage: GIGI_RICCI_DATA_ROOT=<gigi checkout with output/> python docs/papers/two-ring-pg/make_figure_rings3d.py
"""
import gzip
import json
import sys
from pathlib import Path

import numpy as np
import matplotlib
matplotlib.use("pdf")
import matplotlib.pyplot as plt
from mpl_toolkits.mplot3d.art3d import Poly3DCollection, Line3DCollection

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
sys.path[:0] = [str(REPO / "scripts"), str(REPO / "e2e/probes")]
import ring_fold as rf                       # noqa: E402  (applies GIGI_RICCI_DATA_ROOT to the PG module)

pg, tw = rf.pg, rf.tw
FIG, DATA = HERE / "figures", HERE / "data"
INK, INK2 = "#0b0b0b", "#52514e"
BLUE, ORANGE, AQUA, GREY = "#2a78d6", "#eb6834", "#1baf7a", "#9a9a96"
plt.rcParams.update({"font.family": "serif", "font.size": 8.5, "axes.titlesize": 8.6,
                     "savefig.bbox": "tight", "pdf.fonttype": 42})
W2 = 6.9
LEVEL = 1                                    # n = 2
FACES4 = [(0, 1, 2), (0, 1, 3), (0, 2, 3), (1, 2, 3)]
STYLE = {"K": (BLUE, 0.92, 1.0), "ring1": (ORANGE, 0.26, 0.75), "chosen": (AQUA, 0.24, 0.75), "ring2": (GREY, 0.07, 0.35)}
ORDER = ["ring2", "chosen", "ring1", "K"]          # drawn back to front: K always on top


def geometry():
    cells = pg.load_cells(LEVEL)["cells"]
    faces = pg.face_index(cells)
    fold = json.loads(gzip.open(REPO / "research/ring-fold-20261118/fold.json.gz", "rt").read())
    rows = {r["cell"]: r for r in fold["levels"][LEVEL]["cells"]}
    k2 = next(c for c, r in rows.items() if r["k"] == 2 and c != 0)
    k7 = next(c for c, r in rows.items() if r["k"] == 7)
    out = {"level": LEVEL, "n": 2, "units": "chordal length on the unit three-sphere", "cells": []}
    for ci in (0, k2, k7):
        nb1 = tw.neighbourhood_rings(cells, faces, ci, 1)
        nb2 = tw.neighbourhood_rings(cells, faces, ci, 2)
        set1 = set(nb1)
        a1, _ = rf.amp(cells, faces, ci, nb2, set1)
        a2, _ = rf.amp(cells, faces, ci, nb2, set(nb2))
        cand = [k for k in nb2 if k not in set1]
        chosen, cur, path = set(set1), a1, []
        while cur > rf.TARGET * a2:
            cur, best = min(((rf.amp(cells, faces, ci, nb2, chosen | {c})[0], c) for c in cand if c not in chosen))
            chosen.add(best)
            path.append([int(best), round(cur, 2)])
        if len(chosen) - len(set1) != rows[ci]["k"]:
            raise SystemExit(f"cell {ci}: greedy k differs from the fold receipt")
        to_orth, _, _ = pg.chart(nb2[ci])
        V4 = np.asarray(nb2[ci], float)
        K = to_orth(nb2[ci])
        scale = np.linalg.norm(V4[0] - V4[1]) / np.linalg.norm(K[0] - K[1])
        centre = (K * scale).mean(axis=0)
        tets = [{"id": int(k), "ring": "K" if k == ci else ("ring1" if k in set1 else ("chosen" if k in chosen else "ring2")),
                 "v": np.round(to_orth(V) * scale - centre, 6).tolist()} for k, V in nb2.items()]
        out["cells"].append({"cell": int(ci), "k": len(chosen) - len(set1), "candidates": len(cand),
                             "amp_one": round(a1, 1), "amp_two": round(a2, 1), "amp_fold": round(cur, 1),
                             "greedy_path": path, "tets": tets})
    (DATA / "rings_3d.json").write_text(json.dumps(out, indent=1))
    return out


def draw(ax, cell, show, explode=0.0, elev=22, azim=-58):
    ax.computed_zorder = False
    pts = []
    for ring in ORDER:
        if ring not in show:
            continue
        polys, lines = [], []
        for t in cell["tets"]:
            if t["ring"] != ring:
                continue
            V = np.array(t["v"])
            V = V + explode * V.mean(axis=0)
            pts.append(V)
            polys += [V[list(f)] for f in FACES4]
            lines += [V[[i, j]] for i in range(4) for j in range(i + 1, 4)]
        col, alpha, la = STYLE[ring]
        ax.add_collection3d(Poly3DCollection(polys, facecolors=matplotlib.colors.to_rgba(col, alpha), edgecolors="none"))
        ax.add_collection3d(Line3DCollection(lines, colors=matplotlib.colors.to_rgba(INK if ring == "K" else col, la),
                                             linewidths=0.9 if ring == "K" else 0.55))
    P = np.vstack(pts)
    c = (P.max(axis=0) + P.min(axis=0)) / 2
    r = np.abs(P - c).max() * (0.92 if explode == 0 else 0.74)
    ax.set_xlim(c[0] - r, c[0] + r); ax.set_ylim(c[1] - r, c[1] + r); ax.set_zlim(c[2] - r, c[2] + r)
    ax.set_box_aspect((1, 1, 1))
    ax.view_init(elev=elev, azim=azim)
    ax.set_axis_off()


def figure(geo):
    c0, c2, c7 = geo["cells"]
    count = lambda c, ring: sum(t["ring"] == ring for t in c["tets"])   # noqa: E731
    fig = plt.figure(figsize=(W2, 5.0))
    fig.subplots_adjust(left=0, right=1, top=0.93, bottom=0.07, wspace=0.0, hspace=0.12)
    one = 1 + count(c0, "ring1")
    panels = [
        (c0, {"K", "ring1"}, 0.0, 22, -58, f"(a) one ring: {one} cells\namplification {c0['amp_one']}"),
        (c0, {"K", "ring1", "chosen", "ring2"}, 0.0, 22, -58, f"(b) two rings: {len(c0['tets'])} cells\namplification {c0['amp_two']}"),
        (c0, {"K", "ring1", "chosen"}, 0.0, 22, -58, f"(c) folded: {one + c0['k']} cells\namplification {c0['amp_fold']}"),
        (c0, {"K", "ring1", "chosen", "ring2"}, 0.9, 34, 38, f"(d) cell {c0['cell']} pulled apart\nfold keeps {c0['k']} of {c0['candidates']}"),
        (c2, {"K", "ring1", "chosen", "ring2"}, 0.9, 34, 38, f"(e) cell {c2['cell']} pulled apart\nfold keeps {c2['k']} of {c2['candidates']}"),
        (c7, {"K", "ring1", "chosen", "ring2"}, 0.9, 34, 38, f"(f) cell {c7['cell']} pulled apart\nfold keeps {c7['k']} of {c7['candidates']}"),
    ]
    for i, (cell, show, ex, el, az, title) in enumerate(panels):
        ax = fig.add_subplot(2, 3, i + 1, projection="3d")
        draw(ax, cell, show, ex, el, az)
        ax.set_title(title, color=INK, pad=-2)
    handles = [plt.Rectangle((0, 0), 1, 1, fc=matplotlib.colors.to_rgba(STYLE[r][0], max(STYLE[r][1], 0.25)), ec=STYLE[r][0], lw=0.6)
               for r in ("K", "ring1", "chosen", "ring2")]
    fig.legend(handles, ["cell K", "first ring", "second ring, kept by the fold", "second ring, left out"],
               loc="lower center", ncol=4, frameon=False, fontsize=7.6)
    FIG.mkdir(exist_ok=True)
    fig.savefig(FIG / "fig_rings_3d.pdf")
    print("wrote", FIG / "fig_rings_3d.pdf")


if __name__ == "__main__":
    figure(geometry())
