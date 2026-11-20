"""The physical spectrum of the linearised SU(2) operator at the Berger field, sector by sector.

Bottom of every spin-j/2 sector on the gauge-orthogonal slice: lambda(j) = (j(j+2) - 14)/3 (Proposition in the
SU(2) stability section; scripts/ricci_su2_sector_formula.py). On the Poincare sphere only the sectors with
icosahedral invariants survive (Molien series): j = 0, 12, 20, 24, 30, ... The lowest gauge-orthogonal values of
each surviving sector come from research/su2-left-invariant-sector-20261119/gauge_check.json; the measured
lattice values at j = 0 (identified by eigenvector at n <= 2) and j = 12 (by value) from the committed spectra.

Run: python docs/papers/two-ring-pg/make_figure_spectrum_ladder.py   -> figures/fig_spectrum_ladder.pdf
"""
import json
from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
INK, MUTED, RULE, SURF = "#20272d", "#616c74", "#d8dfe3", "#fcfcfb"
NATIVE, TWORING = "#2a78d6", "#1baf7a"          # the paper's method colours (validated categorical slots 1, 3)
plt.rcParams.update({"font.family": "serif", "font.serif": ["Palatino Linotype", "DejaVu Serif"],
                     "font.size": 8.5, "text.color": INK, "mathtext.fontset": "stix", "pdf.fonttype": 42,
                     "axes.edgecolor": MUTED, "axes.labelcolor": INK, "xtick.color": MUTED, "ytick.color": MUTED,
                     "axes.spines.top": False, "axes.spines.right": False, "axes.linewidth": 0.6})

ALLOWED = [0, 12, 20, 24]                     # surviving sectors up to j = 24 (next: 30)
lam = lambda j: (j * (j + 2) - 14) / 3        # noqa: E731


def measured(target, level, trial):
    src = {0: "research/su2-pg-stability-20261117/L0_{t}.eigs.npy",
           1: "research/su2-pg-stability-20261117/L1_{t}.eigs.npy",
           2: "research/su2-spectrum-n3-20261119/L2_{t}.eigs.npy"}[level]
    ev = np.load(REPO / src.format(t=trial))
    return float(ev[np.argmin(np.abs(ev - target))].real)


def main():
    gc = json.loads((REPO / "research/su2-left-invariant-sector-20261119/gauge_check.json").read_text())
    fig, ax = plt.subplots(figsize=(6.9, 3.6))
    jj = np.linspace(0, 25, 400)
    ax.plot(jj, lam(jj), color=MUTED, lw=0.9, zorder=1)
    ax.text(25.0, lam(25) - 30, r"$\lambda(j)=\frac{j(j+2)-14}{3}$", color=MUTED, fontsize=9.5, va="center")
    # forbidden sectors: hollow, on the curve
    forb = [j for j in range(0, 25) if j not in ALLOWED]
    ax.scatter(forb, [lam(j) for j in forb], s=14, facecolor=SURF, edgecolor=RULE, lw=0.9, zorder=2)
    # surviving sectors: the bottom (exact) and the next gauge-orthogonal values as ticks
    for j in ALLOWED:
        vals = gc[f"j{j}"]["slice_lowest_10"]
        lo = vals[0]
        for v in sorted(set(np.round(vals, 6))):
            if v <= 230:
                ax.plot([j - 0.45, j + 0.45], [v, v], color=INK, lw=0.8, alpha=0.35, zorder=2)
        ax.scatter([j], [lo], s=46, color=INK, zorder=4, edgecolor=SURF, lw=0.8)
    labels = {0: r"$-\frac{14}{3}$", 12: r"$\frac{154}{3}$", 20: r"$142$", 24: r"$\frac{610}{3}$"}
    for j, t in labels.items():
        ax.annotate(t, (j, lam(j)), xytext=(-10, 6) if j else (14, -13), textcoords="offset points",
                    ha="right" if j else "left", fontsize=9, color=INK)
    # the gap
    ax.annotate("", xy=(4, lam(12)), xytext=(4, lam(0)),
                arrowprops=dict(arrowstyle="<->", color=INK, lw=0.8, shrinkA=0, shrinkB=0))
    ax.plot([0, 12], [lam(0), lam(0)], color=INK, lw=0.5, ls=(0, (2, 2)))
    ax.plot([4, 12], [lam(12), lam(12)], color=INK, lw=0.5, ls=(0, (2, 2)))
    ax.text(-0.6, 96, "spin 6 begins $\\frac{168}{3}$\nabove the growth mode;\n168 is the first scalar\neigenvalue of the\nPoincaré sphere",
            fontsize=8, va="center", color=INK)
    ax.annotate("", xy=(4, 0.5 * (lam(0) + lam(12)) + 6), xytext=(2.8, 66),
                arrowprops=dict(arrowstyle="-", color=MUTED, lw=0.5))
    # measured lattice values
    for j, target in ((0, -14 / 3), (12, 154 / 3)):
        for k, (lv, tr, col, mk) in enumerate([(0, "native", NATIVE, "s"), (1, "native", NATIVE, "s"),
                                                (2, "native", NATIVE, "s"), (2, "pg", TWORING, "o")]):
            if j == 12 and lv == 0:
                continue
            ref = target if j == 0 else 51.33
            v = measured(ref, lv, tr)
            ax.scatter([j + 0.7 + 0.32 * k], [v], s=12, marker=mk, color=col, edgecolor=SURF, lw=0.4, zorder=5)
    ax.scatter([], [], s=12, marker="s", color=NATIVE, label="lattice, native ($n=1,2,3$, left to right)")
    ax.scatter([], [], s=12, marker="o", color=TWORING, label="lattice, two-ring ($n=3$)")
    ax.scatter([], [], s=46, color=INK, label="bottom of a surviving sector (exact)")
    ax.scatter([], [], s=14, facecolor=SURF, edgecolor=RULE, label="sector absent on the Poincaré sphere")
    ax.plot([], [], color=INK, alpha=0.35, lw=0.8, label="other exact values in the sector")
    ax.legend(loc="upper left", bbox_to_anchor=(0.0, 1.02), frameon=False, fontsize=7.5, handletextpad=0.4, borderaxespad=0.2)
    ax.set_xticks(ALLOWED); ax.set_xticklabels([f"{j}\n(spin {j // 2})" for j in ALLOWED])
    ax.set_xlim(-1.5, 30); ax.set_ylim(-24, 235)
    ax.set_xlabel("sector index $j$ (spin $j/2$)")
    ax.set_ylabel("eigenvalue, gauge-orthogonal")
    ax.axhline(0, color=RULE, lw=0.6, zorder=0)
    out = HERE / "figures" / "fig_spectrum_ladder.pdf"
    fig.savefig(out, bbox_inches="tight", pad_inches=0.02)
    fig.savefig(out.with_suffix(".png"), dpi=170, bbox_inches="tight", pad_inches=0.02)
    print("wrote", out)


if __name__ == "__main__":
    main()
