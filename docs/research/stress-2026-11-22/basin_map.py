"""Figure: the two basins of her family at t = 3 (from stress_receipt.json, S5), with the squashed meron on the
boundary, the two flat fields, the Berger field's fall, the cut (y -> 0) and one flip (x -> -x).
usage (repository root): python docs/research/stress-2026-11-22/basin_map.py
"""
import json
import sys
from pathlib import Path

import numpy as np
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.colors import ListedColormap

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import stress_tests as S  # noqa: E402

PAPER, INK, GRID = "#f6f1e4", "#1d1b18", "#c9bfa6"
B0, BT = "#dfe6e2", "#efe0c6"      # trivial basin (cool), theta basin (warm)
R = json.loads((HERE / "stress_receipt.json").read_text())
t = 3.0
lab = np.array(R["S5"]["grid"])
xs = np.linspace(-3, 3, 41); ys = np.linspace(0, 3, 41)
Z = (lab == "A=theta").astype(float)

plt.rcParams.update({"font.family": "serif", "font.size": 9.5})
fig, ax = plt.subplots(figsize=(6.2, 3.6), facecolor=PAPER)
ax.set_facecolor(PAPER)
dx = xs[1] - xs[0]; dy = ys[1] - ys[0]
ax.imshow(Z, origin="lower", extent=[xs[0] - dx / 2, xs[-1] + dx / 2, ys[0] - dy / 2, ys[-1] + dy / 2],
          cmap=ListedColormap([B0, BT]), aspect="auto", interpolation="nearest")
ax.contour(xs, ys, Z, levels=[0.5], colors=INK, linewidths=0.9, linestyles="--")


def path(x0, y0, T=3.0):
    sol = S.flow(x0, y0, t, T=T, dense=True)
    ss = np.linspace(0, T, 600)
    return np.array([sol.sol(s) for s in ss])


xb, yb = S.berger(t)
p = path(xb, yb); ax.plot(p[:, 0], p[:, 1], color=INK, lw=1.3)
ax.annotate("", xy=p[40], xytext=p[10], arrowprops=dict(arrowstyle="-|>", color=INK, lw=1.1))
p = path(xb, 0.0); ax.plot(p[:, 0], p[:, 1] + 0.012, color=INK, lw=1.3, ls=(0, (3, 2)))
p = path(2.0, 1.75); ax.plot(p[:, 0], p[:, 1], color=INK, lw=1.0, ls=(0, (1, 1.5)))
ax.plot([xb, xb], [yb, 0.02], color=INK, lw=0.6, ls=":")

m = R["S5"]["meron"][0]
pts = [((0, 0), "A = 0 (trivial basin)", (0.12, 0.06), "o"), ((2 / np.sqrt(t), 2), "A = θ", (0.12, 0.12), "o"),
       ((m["x"], m["y"]), "squashed meron\n(the pass)", (0.18, -0.62), "D"), ((xb, yb), "Berger field", (-1.15, 0.12), "s"),
       ((2.0, 1.75), "a flipped field", (0.1, 0.12), "^")]
for (px, py), txt, (ox, oy), mk in pts:
    ax.plot(px, py, mk, ms=6.5, color=INK, mfc=INK if mk in "oD" else PAPER, mew=1.1, zorder=5)
    ax.text(px + ox, py + oy, txt, fontsize=8.5, color=INK, va="bottom")
ax.text(-2.85, 2.75, "falls to A = θ", fontsize=9, style="italic", color=INK)
ax.text(-2.85, 0.25, "falls to A = 0", fontsize=9, style="italic", color=INK)
ax.text(-1.7, 0.03, "cut: y → 0", fontsize=8, color=INK)
ax.set_xlim(-3.05, 3.05); ax.set_ylim(-0.04, 3.05)
for s_ in ax.spines.values():
    s_.set_color(INK); s_.set_linewidth(0.8)
ax.tick_params(colors=INK, direction="in", top=True, right=True)
ax.set_xlabel("x, the field along her circle", color=INK)
ax.set_ylabel("y, the field across it", color=INK)
fig.tight_layout()
for ext in ("png", "pdf"):
    fig.savefig(HERE / f"basin_map_t3.{ext}", facecolor=PAPER, dpi=220)
print("wrote", HERE / "basin_map_t3.png")
