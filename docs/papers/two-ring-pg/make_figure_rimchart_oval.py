"""The rim chart one order up (Section on the third ring): a drawn illustration, computed.

A real mouth is never quite round. The true rim here is an oval, semi-axes 13 cm and 11 cm; the quantity to read
at the sherd's own edge (the top of the oval) is its radius of curvature there, a^2/b = 15.36 cm.
(a) A circle laid on a long sherd: two sherds share the same own edge, one long and one longer. Least-squares
    circles through their points read different, wrong radii: how wrong depends on how much far edge there is.
(b) An oval through exactly five points, one point chipped by 0.25 mm: the oval has nothing to spare.
(c) The hybrid: an oval fitted by least squares to the long edge (chip included), read at the sherd's own edge.
    Both sherds, with the same chip as (b) on the own edge, give nearly the true radius.
(d) Clues against unknowns: independent face fluxes per neighbourhood (n = 1) against the 11 unknowns of a
    closed linear field and the 26 of a field read to second order (Lemma on the count).

Run: python docs/papers/two-ring-pg/make_figure_rimchart_oval.py   -> figures/fig_rimchart_oval.pdf
"""
from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np

HERE = Path(__file__).resolve().parent
INK, MUTED, RULE = "#20272d", "#616c74", "#d8dfe3"
BLUE, COPPER, TEAL = "#2a78d6", "#b5651d", "#2a9d8f"
plt.rcParams.update({"font.family": "serif", "font.serif": ["Palatino Linotype", "DejaVu Serif"],
                     "font.size": 8.5, "text.color": INK, "mathtext.fontset": "stix", "pdf.fonttype": 42,
                     "axes.edgecolor": MUTED, "axes.labelcolor": INK, "xtick.color": MUTED, "ytick.color": MUTED})
A_, B_ = 13.0, 11.0
RHO = A_ ** 2 / B_                      # radius of curvature at the top of the oval
CHIP = 0.025                            # 0.25 mm, in cm


def rim(th):
    return np.c_[A_ * np.cos(th), B_ * np.sin(th)]


def circle_lsq(p):
    A = np.c_[2 * p, np.ones(len(p))]
    sol = np.linalg.lstsq(A, (p ** 2).sum(1), rcond=None)[0]
    c = sol[:2]
    return c, float(np.sqrt(sol[2] + c @ c))


def conic_fit(p):
    """Algebraic conic a x^2 + b xy + c y^2 + d x + e y + f = 0: exact through 5 points, least squares otherwise."""
    x, y = p[:, 0], p[:, 1]
    M = np.c_[x * x, x * y, y * y, x, y, np.ones_like(x)]
    return np.linalg.svd(M)[2][-1]


def conic_radius_at_top(q):
    """Point of the conic on x = 0 nearest y = B_, and the radius of curvature there."""
    a, b, c, d, e, f = q
    roots = np.roots([c, e, f])
    roots = roots[np.isreal(roots)].real
    if len(roots) == 0:
        return np.nan, np.nan
    y0 = roots[np.argmin(np.abs(roots - B_))]
    Fx, Fy = 2 * a * 0 + b * y0 + d, b * 0 + 2 * c * y0 + e
    Fxx, Fxy, Fyy = 2 * a, b, 2 * c
    k = abs(Fy ** 2 * Fxx - 2 * Fx * Fy * Fxy + Fx ** 2 * Fyy) / (Fx ** 2 + Fy ** 2) ** 1.5
    return y0, 1 / k


def chipped(p, i):
    q = p.copy()
    q[i] *= 1 - CHIP / np.linalg.norm(q[i])
    return q


def draw_rim(ax):
    th = np.linspace(np.radians(15), np.radians(165), 400)
    r = rim(th)
    ax.plot(r[:, 0], r[:, 1], color=INK, lw=0.9, ls=(0, (3, 2)))


def frame(ax, label, title):
    ax.set_xlim(-14, 14); ax.set_ylim(-4.6, 15.5); ax.set_aspect("equal"); ax.axis("off")
    ax.text(-13.8, 15.4, label, fontsize=10, weight="bold", va="top")
    ax.text(-11.4, 15.2, title, fontsize=8, va="top")


def osculating(ax, y0, r, col, ls="-"):
    tt = np.linspace(np.radians(55), np.radians(125), 200)
    cy = y0 - r
    ax.plot(r * np.cos(tt), cy + r * np.sin(tt), color=col, lw=1.2, ls=ls)


def main():
    sherds = {"long": np.radians(np.linspace(55, 125, 24)), "longer": np.radians(np.linspace(30, 150, 24))}
    own = np.radians(np.array([84.0, 90.0, 96.0]))
    fig, axs = plt.subplots(2, 2, figsize=(7.2, 5.9), gridspec_kw=dict(hspace=0.04, wspace=0.08))
    res = {}

    # (a) circles on long sherds
    ax = axs[0, 0]; frame(ax, "(a)", "a circle on a long sherd")
    draw_rim(ax)
    for (name, th), col in zip(sherds.items(), (COPPER, BLUE)):
        p = rim(th)
        c, r = circle_lsq(p)
        res[f"circle_{name}"] = r
        tt = np.linspace(np.radians(30), np.radians(150), 300)
        ax.plot(c[0] + r * np.cos(tt), c[1] + r * np.sin(tt), color=col, lw=1.2)
        ax.scatter(p[:, 0], p[:, 1], s=7, color=col, zorder=4)
    ax.scatter(*rim(own).T, s=24, facecolor="none", edgecolor=INK, lw=0.8, zorder=5)
    ax.text(-13.6, -0.6, f"long sherd: circle radius {res['circle_long']:.2f} cm", color=COPPER, fontsize=7.6)
    ax.text(-13.6, -2.0, f"longer sherd: circle radius {res['circle_longer']:.2f} cm", color=BLUE, fontsize=7.6)
    ax.text(-13.6, -3.4, f"true, at the sherd's own edge (circled): {RHO:.2f} cm", color=MUTED, fontsize=7.6)

    # (b) oval through exactly five points, one chipped
    ax = axs[0, 1]; frame(ax, "(b)", "an oval through exactly five points")
    draw_rim(ax)
    th5 = np.radians(np.linspace(72, 108, 5))
    p5 = chipped(rim(th5), 2)
    q = conic_fit(p5)
    y0, r5 = conic_radius_at_top(q)
    res["oval5_radius"] = r5
    xx, yy = np.meshgrid(np.linspace(-14, 14, 500), np.linspace(0.5, 15.5, 400))
    Z = q[0] * xx ** 2 + q[1] * xx * yy + q[2] * yy ** 2 + q[3] * xx + q[4] * yy + q[5]
    ax.contour(xx, yy, Z, levels=[0], colors=[COPPER], linewidths=1.2)
    ax.scatter(p5[:, 0], p5[:, 1], s=14, color=COPPER, zorder=4, edgecolor=INK, lw=0.4)
    ax.annotate("chip 0.25 mm", xy=p5[2], xytext=(4.0, 13.2), fontsize=7.6, color=INK,
                arrowprops=dict(arrowstyle="-", color=MUTED, lw=0.5))
    ax.text(-13.6, -0.6, f"radius read at the own edge: {r5:.1f} cm", color=COPPER, fontsize=7.6)
    ax.text(-13.6, -2.0, f"true: {RHO:.2f} cm; one chip and the oval collapses", color=MUTED, fontsize=7.6)

    # (c) the hybrid: least-squares oval over the long edge, read at the own edge
    ax = axs[1, 0]; frame(ax, "(c)", "oval from the long edge, read at the own edge")
    draw_rim(ax)
    for (name, th), col in zip(sherds.items(), (COPPER, BLUE)):
        p = rim(th)
        k = int(np.argmin(np.abs(th - np.radians(90))))
        p = chipped(p, k)
        q = conic_fit(p)
        y0, r = conic_radius_at_top(q)
        res[f"hybrid_{name}"] = r
        ax.scatter(p[:, 0], p[:, 1], s=7, color=col, zorder=4)
        osculating(ax, y0, r, col, ls="-" if name == "long" else (0, (4, 2)))
    ax.scatter(*rim(own).T, s=24, facecolor="none", edgecolor=INK, lw=0.8, zorder=5)
    ax.text(-13.6, -0.6, f"long sherd: radius {res['hybrid_long']:.2f} cm", color=COPPER, fontsize=7.6)
    ax.text(-13.6, -2.0, f"longer sherd: radius {res['hybrid_longer']:.2f} cm", color=BLUE, fontsize=7.6)
    ax.text(-13.6, -3.4, f"true: {RHO:.2f} cm; the chip of (b) is on the own edge", color=MUTED, fontsize=7.6)

    # (d) clues against unknowns
    ax = axs[1, 1]
    names = ["one ring", "two rings", "three rings"]
    clues = [11, 26, 50]
    ax.bar(range(3), clues, width=0.5, color=TEAL, edgecolor="white", lw=2)
    for i, c in enumerate(clues):
        ax.text(i, c - 4.2, f"{c}", ha="center", fontsize=9, color="white", weight="bold")
    ax.axhline(11, color=INK, lw=0.8, ls=(0, (3, 2)))
    ax.axhline(26, color=COPPER, lw=0.8, ls=(0, (3, 2)))
    ax.text(-0.45, 12.2, "11 unknowns: a closed linear field", fontsize=7.6, ha="left", color=INK)
    ax.text(-0.45, 27.2, "26 unknowns: read to second order", fontsize=7.6, ha="left", color=COPPER)
    ax.text(-0.45, 63, "(d)", fontsize=10, weight="bold", va="top")
    ax.text(-0.12, 62.6, "independent clues per cell ($n=1$)", fontsize=8, va="top")
    ax.set_xticks(range(3)); ax.set_xticklabels(names)
    ax.set_ylim(0, 63); ax.set_xlim(-0.5, 2.6)
    ax.spines[["top", "right"]].set_visible(False)
    ax.tick_params(axis="y", labelsize=7.5)

    out = HERE / "figures" / "fig_rimchart_oval.pdf"
    fig.savefig(out, bbox_inches="tight", pad_inches=0.02)
    fig.savefig(out.with_suffix(".png"), dpi=170, bbox_inches="tight", pad_inches=0.02)
    print({k: round(v, 3) for k, v in res.items()}, "true", round(RHO, 3))


if __name__ == "__main__":
    main()
