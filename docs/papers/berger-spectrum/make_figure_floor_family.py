"""The floor law across the Berger family g = (t, 1, 1) (Section on the family).

(a) lambda_t(j) = [j(j+2) - 2(t-3) j - 2(t-2)(t+4)] / t for j = 0..6, with the crossings t_j where it vanishes, the
    certified range [2.75, 10] shaded and the threshold t* ~ 2.698 marked.
(b) the floor gap lambda_t(12) - lambda_t(0) = (240 - 24 t)/t on the Poincare sphere.
Run: python docs/papers/berger-spectrum/make_figure_floor_family.py -> figures/fig_floor_family.pdf
"""
from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np

HERE = Path(__file__).resolve().parent
INK, MUTED, RULE, SURF = "#20272d", "#616c74", "#d8dfe3", "#fcfcfb"
BLUE = "#2a78d6"
plt.rcParams.update({"font.family": "serif", "font.serif": ["Palatino Linotype", "DejaVu Serif"], "font.size": 8.5,
                     "text.color": INK, "mathtext.fontset": "stix", "pdf.fonttype": 42, "axes.edgecolor": MUTED,
                     "axes.labelcolor": INK, "xtick.color": MUTED, "ytick.color": MUTED,
                     "axes.spines.top": False, "axes.spines.right": False, "axes.linewidth": 0.6})
lam = lambda t, j: (j * (j + 2) - 2 * (t - 3) * j - 2 * (t - 2) * (t + 4)) / t          # noqa: E731
tcross = lambda j: (np.sqrt((j + 2) ** 2 + 2 * (j + 4) ** 2) - (j + 2)) / 2              # noqa: E731
TSTAR = 2.6980269405200263


def main():
    fig, (ax, bx) = plt.subplots(1, 2, figsize=(7.2, 3.1), gridspec_kw=dict(width_ratios=[1.55, 1], wspace=0.32))
    t = np.linspace(1.0, 10.0, 600)
    ax.axvspan(2.75, 10.0, color=RULE, alpha=0.45, lw=0, zorder=0)
    ax.text(6.4, 33, "certified: the floor\nfor every $t\\in[2.75,10]$", color=MUTED, fontsize=7.6, ha="center")
    ax.axhline(0, color=MUTED, lw=0.6, zorder=1)
    ax.axvline(TSTAR, color=MUTED, lw=0.6, ls=(0, (3, 2)))
    ax.text(TSTAR - 0.1, -26.5, "$t^*\\approx2.698$", color=MUTED, fontsize=7.6, ha="right")
    for j in range(7):
        ax.plot(t, lam(t, j), color=INK if j == 0 else BLUE, lw=1.1 if j == 0 else 0.9, alpha=1 if j == 0 else 0.35 + 0.1 * j)
        tj = tcross(j)
        if tj <= 10:
            ax.plot([tj], [0], "o", ms=3.2, color=INK, zorder=4)
            ax.text(tj, 2.2 + (1.6 if j % 2 else 0), f"{j}", fontsize=7.2, ha="center", va="bottom", color=INK)
    ax.plot([3], [lam(3, 0)], "o", ms=4, color=BLUE, zorder=5)
    ax.annotate("$-14/3$ at $t=3$", (3, lam(3, 0)), xytext=(4.0, -21), fontsize=7.6, color=INK,
                arrowprops=dict(arrowstyle="-", color=MUTED, lw=0.5))
    ax.set_xlim(1, 10.2); ax.set_ylim(-30, 42)
    ax.set_xlabel("stretch $t$, metric $(t,1,1)$"); ax.set_ylabel("$\\lambda_t(j)$")
    ax.set_title("(a)  the twist's energy $\\lambda_t(j)$, $j=0,\\dots,6$", loc="left", fontsize=8)

    tt = np.linspace(2.75, 12, 400)
    bx.axhline(0, color=MUTED, lw=0.6)
    bx.plot(tt, (240 - 24 * tt) / tt, color=INK, lw=1.1)
    bx.plot([3], [56], "o", ms=4, color=BLUE)
    bx.annotate("$56=168/3$ at $t=3$", (3, 56), xytext=(5.0, 62), fontsize=7.6, color=INK,
                arrowprops=dict(arrowstyle="-", color=MUTED, lw=0.5))
    bx.plot([10], [0], "o", ms=3.2, color=INK)
    bx.text(9.6, -9, "closes at $t=10$", fontsize=7.6, color=MUTED, ha="right")
    bx.set_xlabel("stretch $t$"); bx.set_ylabel("$\\lambda_t(12)-\\lambda_t(0)$")
    bx.set_xlim(2.5, 12); bx.set_ylim(-12, 80)
    bx.set_title("(b)  floor gap on the Poincaré sphere", loc="left", fontsize=8)
    out = HERE / "figures" / "fig_floor_family.pdf"
    fig.savefig(out, bbox_inches="tight", pad_inches=0.02)
    fig.savefig(out.with_suffix(".png"), dpi=170, bbox_inches="tight", pad_inches=0.02)
    print("wrote", out)


if __name__ == "__main__":
    main()
