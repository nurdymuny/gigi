"""Figures for the map of the circle-symmetric family (exploratory; 21 November 2026).

fig_twist_map.png: for t = 1, 2, 3, 4, 6, the number of sectors j <= 8 in which the twist is the floor, over (x, y),
  with the region where she is the floor of all nine outlined, x = 0 dashed, and the Berger field (star), the
  squashed meron (diamond) and the line A = s theta (dashed) marked and labelled.
fig_twist_shape.png: her amplitude on the base sphere, |f|^2 = cos^(2j)(eta/2) (eta the angle from the pole of the
  long circle), for j = 0, 2, 4, 8, 16: she gathers into a cap about the pole as the sector rises; and the lift
  2xj/sqrt(t) against x, the discount that exists only for x < 0.
usage: python scripts/ricci_su2_twist_family_figures.py MAP.json OUTDIR
"""
import json
import sys
from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt                     # noqa: E402
from matplotlib.colors import BoundaryNorm, ListedColormap   # noqa: E402
import numpy as np                                  # noqa: E402

SURF, INK, INK2, GRID = "#fcfcfb", "#0b0b0b", "#52514e", "#d9d8d4"
SEQ = ["#f0efec", "#cde2fb", "#b7d3f6", "#9ec5f4", "#86b6ef", "#6da7ec", "#5598e7", "#3987e5", "#256abf", "#104281"]
ORANGE, AQUA, MAGENTA = "#eb6834", "#1baf7a", "#e87ba4"
plt.rcParams.update({"font.size": 9, "axes.edgecolor": INK2, "axes.labelcolor": INK, "xtick.color": INK2,
                     "ytick.color": INK2, "figure.facecolor": SURF, "axes.facecolor": SURF, "savefig.facecolor": SURF})


def fig_map(data, out):
    cells = data["cells"]; marks = data["marks"]
    ts = sorted({c["t"] for c in cells})
    xs = sorted({c["x"] for c in cells}); ys = sorted({c["y"] for c in cells})
    fig, axes = plt.subplots(1, len(ts), figsize=(2.6 * len(ts), 3.3), sharey=True, constrained_layout=True)
    cmap = ListedColormap(SEQ); norm = BoundaryNorm(np.arange(-0.5, 10.5, 1), cmap.N)
    for ax, t in zip(axes, ts):
        Z = np.zeros((len(ys), len(xs))); Fl = np.zeros_like(Z)
        for c in cells:
            if c["t"] == t:
                Z[ys.index(c["y"]), xs.index(c["x"])] = c["sectors_won"]
                Fl[ys.index(c["y"]), xs.index(c["x"])] = 1.0 if c["floor_all"] else 0.0
        im = ax.pcolormesh(xs, ys, Z, cmap=cmap, norm=norm, shading="nearest", rasterized=True)
        if Fl.any():
            ax.contour(xs, ys, Fl, levels=[0.5], colors=INK, linewidths=1.6)
        ax.axvline(0, color=INK2, lw=1, ls=(0, (3, 3)))
        yy = np.linspace(0, 3, 50); xx = yy / np.sqrt(t)
        ax.plot(xx, yy, color=MAGENTA, lw=2, ls=(0, (6, 3)))
        bx, by = marks[str(t)]["berger"]; mx, my = marks[str(t)]["meron"]
        ax.plot([mx], [my], marker="D", ms=7, color=AQUA, mec=SURF, mew=1.2, ls="none")
        ax.plot([bx], [by], marker="*", ms=13, color=ORANGE, mec=SURF, mew=1.2, ls="none")
        if abs(bx - mx) + abs(by - my) < 1e-6:
            ax.annotate("Berger = meron\n(round sphere)", (bx, by), xytext=(-8, 8), textcoords="offset points", color=INK,
                            fontsize=8, ha="right")
        else:
            ax.annotate("Berger", (bx, by), xytext=(4, 6), textcoords="offset points", color=INK, fontsize=8)
            ax.annotate("meron", (mx, my), xytext=(5, -10), textcoords="offset points", color=INK, fontsize=8)
        ax.set_title(f"stretch t = {t:g}", color=INK, fontsize=10)
        ax.set_xlabel("x  (field along the long circle)")
        ax.set_xlim(-3, 3); ax.set_ylim(0, 3)
        for s in ("top", "right"):
            ax.spines[s].set_visible(False)
    axes[0].set_ylabel("y  (field across it)")
    axes[-1].annotate("A = sθ", (3 / np.sqrt(ts[-1]) * 0.8, 2.4), color=INK, fontsize=8)
    cb = fig.colorbar(im, ax=axes, ticks=range(10), fraction=0.025, pad=0.01)
    cb.set_label("sectors j ≤ 8 where she is the floor", color=INK)
    fig.suptitle("Where the twist is the floor: every circle-symmetric field  (black outline: all nine sectors; dashed: x = 0)",
                 color=INK, fontsize=10)
    fig.savefig(out, dpi=180)


def fig_shape(out):
    fig, (a1, a2) = plt.subplots(1, 2, figsize=(9.5, 3.4), constrained_layout=True)
    eta = np.linspace(0, np.pi, 400)
    js = [0, 2, 4, 8, 16]; shades = ["#86b6ef", "#5598e7", "#2a78d6", "#1c5cab", "#0d366b"]
    for jv, col in zip(js, shades):
        a1.plot(np.degrees(eta), np.cos(eta / 2) ** (2 * jv), color=col, lw=2)
        level = {0: 1.0, 2: 0.55, 4: 0.42, 8: 0.29, 16: 0.16}[jv]
        k = int(np.argmin(np.abs(np.cos(eta / 2) ** (2 * jv) - level))) if jv else 300
        a1.annotate(f"j = {jv}", (np.degrees(eta[k]), np.cos(eta[k] / 2) ** (2 * jv)), xytext=(5, 3),
                    textcoords="offset points", color=INK, fontsize=8)
    a1.set_xlabel("angle from the pole of the long circle on the base sphere (degrees)")
    a1.set_ylabel("her amplitude |f|²")
    a1.set_title("Her shape: a cap that tightens as the sector rises", color=INK, fontsize=10)
    a1.set_xlim(0, 180); a1.set_ylim(0, 1.05); a1.grid(color=GRID, lw=0.6)
    xv = np.linspace(-3, 3, 200)
    for tv, col in ((2.0, "#86b6ef"), (3.0, "#3987e5"), (6.0, "#104281")):
        a2.plot(xv, 2 * xv * 4 / np.sqrt(tv), color=col, lw=2)
        xl = 1.2
        a2.annotate(f"t = {tv:g}", (xl, 2 * xl * 4 / np.sqrt(tv)), xytext=(6, -4), textcoords="offset points",
                    color=INK, fontsize=8)
    a2.axhline(0, color=INK2, lw=1); a2.axvline(0, color=INK2, lw=1, ls=(0, (3, 3)))
    a2.fill_between(xv, 0, -12, where=xv < 0, color="#cde2fb", alpha=0.6, lw=0)
    a2.annotate("discount: the field along the long\ncircle turns against her (x < 0)", (-2.9, -9.5), color=INK, fontsize=8)
    a2.set_xlabel("x  (field along the long circle)")
    a2.set_ylabel("lift term 2xj/√t  (here j = 4)")
    a2.set_title("What lifts her cheaply: the sign of x", color=INK, fontsize=10)
    a2.set_ylim(-12, 12); a2.grid(color=GRID, lw=0.6)
    for ax in (a1, a2):
        for s in ("top", "right"):
            ax.spines[s].set_visible(False)
    fig.savefig(out, dpi=180)


def main():
    data = json.loads(Path(sys.argv[1]).read_text())
    od = Path(sys.argv[2]); od.mkdir(parents=True, exist_ok=True)
    fig_map(data, od / "fig_twist_map.png")
    if len(sys.argv) > 3:
        fig_map(data, Path(sys.argv[3]))                     # e.g. the paper's figures/fig_twist_map.pdf
    fig_shape(od / "fig_twist_shape.png")
    print("wrote", od / "fig_twist_map.png", od / "fig_twist_shape.png")


if __name__ == "__main__":
    main()
