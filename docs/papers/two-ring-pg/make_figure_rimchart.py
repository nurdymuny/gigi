"""The archaeologist's rim chart (Section 1): a drawn illustration, not a photograph.

(a) A rim chart: concentric arcs of known diameter; a rim sherd laid edge-down sits flush on one.
(b) Why the amount of edge matters, computed: a true rim of radius R, one point chipped inwards by d.
    Short edge: three points, the circle through them (exactly determined). Long edge: twelve points
    over a wider arc, least-squares circle (overdetermined). The fitted radii are printed from the fit.

Run: python docs/papers/two-ring-pg/make_figure_rimchart.py   -> figures/fig_rimchart.pdf
"""
from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.patches import Polygon
import numpy as np

HERE = Path(__file__).resolve().parent
INK, MUTED, RULE = "#20272d", "#616c74", "#d8dfe3"
BLUE, COPPER, TEAL = "#2f6690", "#b5651d", "#2a9d8f"
plt.rcParams.update({"font.family": "serif", "font.serif": ["Palatino Linotype", "DejaVu Serif"],
                     "font.size": 9, "text.color": INK, "mathtext.fontset": "stix",
                     "pdf.fonttype": 42, "axes.edgecolor": INK})


def circle_through(p):
    """Circle through three points (exact)."""
    (x1, y1), (x2, y2), (x3, y3) = p
    a = np.array([[x2 - x1, y2 - y1], [x3 - x1, y3 - y1]]) * 2
    b = np.array([x2**2 - x1**2 + y2**2 - y1**2, x3**2 - x1**2 + y3**2 - y1**2])
    c = np.linalg.solve(a, b)
    return c, float(np.hypot(*(p[0] - c)))


def circle_lsq(p):
    """Algebraic least-squares circle (Kasa)."""
    A = np.c_[2 * p, np.ones(len(p))]
    b = (p**2).sum(1)
    sol = np.linalg.lstsq(A, b, rcond=None)[0]
    c = sol[:2]
    return c, float(np.sqrt(sol[2] + c @ c))


def sherd_polygon(R, t, a0, a1, rng):
    """An annular sector (rim edge at radius R, body outward) with broken, jagged ends."""
    th = np.linspace(a0, a1, 80)
    inner = np.c_[R * np.cos(th), R * np.sin(th)]
    outer = np.c_[(R + t) * np.cos(th[::-1]), (R + t) * np.sin(th[::-1])]
    def jag(r0, r1, a):
        rs = np.linspace(r0, r1, 6)
        da = rng.normal(0, 0.012, 6)
        da[0] = da[-1] = 0
        return np.c_[rs * np.cos(a + da), rs * np.sin(a + da)]
    return np.vstack([inner, jag(R, R + t, a1), outer, jag(R + t, R, a0)])


def main():
    rng = np.random.default_rng(7)
    fig = plt.figure(figsize=(7.2, 3.3))

    # (a) the chart
    ax = fig.add_axes([0.02, 0.06, 0.46, 0.86])
    radii = np.arange(2, 21, 2)                      # radius in cm: diameters 4..40
    matched = 12
    for r in radii:
        th = np.linspace(0, np.pi, 200)
        col, lw = (BLUE, 1.6) if r == matched else (MUTED, 0.6)
        ax.plot(r * np.cos(th), r * np.sin(th), color=col, lw=lw)
        ax.text(r, -0.9, f"{2 * r}", ha="center", va="top", fontsize=6.5,
                color=BLUE if r == matched else MUTED)
    for k in range(1, 10):                           # radial lines: tenths of the rim
        a = k * np.pi / 10
        ax.plot([0, 21 * np.cos(a)], [0, 21 * np.sin(a)], color=RULE, lw=0.5, ls=(0, (2, 2)), zorder=0)
    ax.plot([-21, 21], [0, 0], color=INK, lw=0.8)
    poly = sherd_polygon(matched, 2.2, np.radians(58), np.radians(96), rng)
    ax.add_patch(Polygon(poly, closed=True, facecolor=COPPER, edgecolor=INK, lw=0.8, alpha=0.9, zorder=3))
    ax.annotate("rim sherd, edge down", xy=(matched * np.cos(np.radians(70)) + 1.0, matched * np.sin(np.radians(70)) + 2.4),
                xytext=(4.0, 20.3), fontsize=8, color=INK,
                arrowprops=dict(arrowstyle="-", color=MUTED, lw=0.6))
    ax.text(0, -2.6, "diameter (cm)", ha="center", va="top", fontsize=7, color=MUTED)
    ax.text(-20.5, 20.5, "(a)", fontsize=10, weight="bold", va="top")
    ax.text(-20.5, 18.6, "flush on the 24 cm arc", fontsize=8, color=BLUE, va="top")
    ax.set_xlim(-21.5, 21.5); ax.set_ylim(-3.6, 21.5); ax.set_aspect("equal"); ax.axis("off")

    # (b) short edge vs long edge, one chipped point, computed fits
    ax = fig.add_axes([0.53, 0.06, 0.45, 0.86])
    R, d = 12.0, 0.25
    def pts(span_deg, n):
        th = np.radians(90 + np.linspace(-span_deg / 2, span_deg / 2, n))
        p = np.c_[R * np.cos(th), R * np.sin(th)]
        return th, p
    th_s, ps = pts(24, 3)
    ps[1] *= (R - d) / R                             # the chip: middle point pulled inwards
    th_l, pl = pts(84, 12)
    k = 5
    pl[k] *= (R - d) / R                             # the same chip on the long edge
    cs, rs = circle_through(ps)
    cl, rl = circle_lsq(pl)
    th = np.linspace(np.radians(35), np.radians(145), 300)
    ax.plot(R * np.cos(th), R * np.sin(th) - 10, color=INK, lw=0.9, ls=(0, (3, 2)), zorder=5)
    tt = np.linspace(0, 2 * np.pi, 600)
    def draw(c, r, col):
        x, y = c[0] + r * np.cos(tt), c[1] + r * np.sin(tt) - 10
        m = (x > -11.5) & (x < 11.5) & (y > -6) & (y < 4.2)
        ax.plot(np.where(m, x, np.nan), np.where(m, y, np.nan), color=col, lw=1.3)
    draw(cs, rs, COPPER)
    draw(cl, rl, TEAL)
    ax.scatter(ps[:, 0], ps[:, 1] - 10, s=22, color=COPPER, zorder=4, edgecolor=INK, lw=0.5)
    ax.scatter(pl[:, 0], pl[:, 1] - 10, s=9, color=TEAL, zorder=3, edgecolor="none")
    ax.text(-11.2, 9.0, "(b)", fontsize=10, weight="bold", va="top")
    ax.text(-11.2, 7.9, f"true rim: radius {R:.0f} cm (dashed); one point chipped by {d * 10:.1f} mm",
            fontsize=7.5, color=MUTED, va="top")
    ax.text(-11.2, 7.0, f"short edge, 3 points, exact fit: radius {rs:.1f} cm", fontsize=8, color=COPPER, va="top")
    ax.text(-11.2, 6.1, f"long edge, 12 points, least squares: radius {rl:.1f} cm", fontsize=8, color=TEAL, va="top")
    ax.set_xlim(-11.5, 11.5); ax.set_ylim(-5.5, 9.2); ax.set_aspect("equal"); ax.axis("off")

    out = HERE / "figures" / "fig_rimchart.pdf"
    fig.savefig(out, bbox_inches="tight", pad_inches=0.02)
    fig.savefig(out.with_suffix(".png"), dpi=170, bbox_inches="tight", pad_inches=0.02)
    print(f"short-edge radius {rs:.3f} cm, long-edge radius {rl:.3f} cm (true {R})")


if __name__ == "__main__":
    main()
