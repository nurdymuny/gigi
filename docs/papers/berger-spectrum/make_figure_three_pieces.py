"""The three pieces of the floor (Proposition "The twist is a lowest Landau level", part 3).

lambda_t(j) = kappa^2 + 2(j + t) + (12 - 8t),  kappa = (2t - 4 - j)/sqrt(t):
the fibre term (momentum along the fibre), the transverse term (lowest Landau level), the curvature term.
(a) j = 0 across the stretch t: the fibre term vanishes at t = 2, where lambda_2(0) = 0.
(b) t = 6 across the sector j: the fibre term is smallest at j = 2t - 4 = 8; the transverse term moves the minimum
    of lambda to j = t - 4 = 2, value 4 - 3t = -14.
The identity with the floor law is checked symbolically before anything is drawn.
Run: python docs/papers/berger-spectrum/make_figure_three_pieces.py -> figures/fig_three_pieces.pdf
"""
from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np
import sympy as sp

HERE = Path(__file__).resolve().parent
INK, MUTED, RULE = "#20272d", "#616c74", "#d8dfe3"
BLUE = "#2a78d6"
plt.rcParams.update({"font.family": "serif", "font.serif": ["Palatino Linotype", "DejaVu Serif"], "font.size": 8.5,
                     "text.color": INK, "mathtext.fontset": "stix", "pdf.fonttype": 42, "axes.edgecolor": MUTED,
                     "axes.labelcolor": INK, "xtick.color": MUTED, "ytick.color": MUTED,
                     "axes.spines.top": False, "axes.spines.right": False, "axes.linewidth": 0.6})

j_, t_ = sp.symbols("j t", positive=True)
fibre_s = (2 * t_ - 4 - j_) ** 2 / t_
trans_s = 2 * (j_ + t_)
curv_s = 12 - 8 * t_
law_s = (j_ * (j_ + 2) - 2 * (t_ - 3) * j_ - 2 * (t_ - 2) * (t_ + 4)) / t_
assert sp.simplify(fibre_s + trans_s + curv_s - law_s) == 0
assert law_s.subs({t_: 2, j_: 0}) == 0
assert sp.solve(sp.diff(law_s, j_), j_) == [t_ - 4]
assert sp.simplify(law_s.subs(j_, t_ - 4) - (4 - 3 * t_)) == 0
fibre, trans, curv, law = (sp.lambdify((t_, j_), e, "numpy") for e in (fibre_s, trans_s, curv_s, law_s))


def style(ax):
    ax.axhline(0, color=MUTED, lw=0.6, zorder=1)


def main():
    fig, (ax, bx) = plt.subplots(1, 2, figsize=(7.2, 3.1), gridspec_kw=dict(wspace=0.3))

    t = np.linspace(1.0, 10.0, 600); z = 0 * t
    style(ax)
    ax.plot(t, fibre(t, z), color=BLUE, lw=0.9, ls=(0, (4, 2)), label="fibre  $\\kappa^2$")
    ax.plot(t, trans(t, z), color=BLUE, lw=0.9, alpha=0.6, label="transverse  $2(j+t)$")
    ax.plot(t, curv(t, z), color=MUTED, lw=0.9, ls=(0, (1, 1.5)), label="curvature  $12-8t$")
    ax.plot(t, law(t, z), color=INK, lw=1.3, label="sum  $\\lambda_t(0)$")
    ax.plot([2], [0], "o", ms=4, color=INK, zorder=5)
    ax.annotate("$t=2$: no momentum,\n$\\lambda_2(0)=0$", (2, 0), xytext=(3.3, 14), fontsize=7.6, color=INK,
                arrowprops=dict(arrowstyle="-", color=MUTED, lw=0.5))
    ax.set_xlim(1, 10); ax.set_ylim(-70, 40)
    ax.set_xlabel("stretch $t$, metric $(t,1,1)$"); ax.set_ylabel("contribution to $\\lambda_t(0)$")
    ax.set_title("(a)  sector $j=0$ across the family", loc="left", fontsize=8)
    ax.legend(frameon=False, fontsize=7.2, loc="lower left")

    T = 6.0
    jj = np.linspace(0, 14, 400); ji = np.arange(0, 15)
    style(bx)
    bx.plot(jj, fibre(T, jj), color=BLUE, lw=0.9, ls=(0, (4, 2)))
    bx.plot(jj, trans(T, jj), color=BLUE, lw=0.9, alpha=0.6)
    bx.plot(jj, curv(T, jj) + 0 * jj, color=MUTED, lw=0.9, ls=(0, (1, 1.5)))
    bx.plot(jj, law(T, jj), color=INK, lw=1.3)
    bx.plot(ji, law(T, ji), "o", ms=2.6, color=INK, mfc="white", mew=0.7, zorder=4)
    bx.plot([2 * T - 4], [0], "o", ms=3.4, color=BLUE, zorder=5)
    bx.annotate("fibre term zero\nat $j=2t-4=8$", (2 * T - 4, 0), xytext=(9.0, 12), fontsize=7.6, color=INK,
                arrowprops=dict(arrowstyle="-", color=MUTED, lw=0.5))
    bx.plot([T - 4], [law(T, T - 4)], "o", ms=4, color=INK, zorder=5)
    bx.annotate("cheapest sector\n$j=t-4=2$, $\\lambda=-14$", (T - 4, law(T, T - 4)), xytext=(4.6, -26),
                fontsize=7.6, color=INK, arrowprops=dict(arrowstyle="-", color=MUTED, lw=0.5))
    bx.set_xlim(0, 14); bx.set_ylim(-40, 40); bx.set_xticks(range(0, 15, 2))
    bx.set_xlabel("sector $j$"); bx.set_ylabel("contribution to $\\lambda_6(j)$")
    bx.set_title("(b)  stretch $t=6$ across the sectors", loc="left", fontsize=8)

    out = HERE / "figures" / "fig_three_pieces.pdf"
    fig.savefig(out, bbox_inches="tight", pad_inches=0.02)
    fig.savefig(out.with_suffix(".png"), bbox_inches="tight", pad_inches=0.02, dpi=200)
    print("wrote", out)


if __name__ == "__main__":
    main()
