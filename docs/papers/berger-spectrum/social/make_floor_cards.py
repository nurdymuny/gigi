"""LinkedIn cards for the Floor paper, in the plate style of Reading the Curve's cards (the plate helpers, reused).

Every number is either an exact formula from the published paper (doi:10.5281/zenodo.24412603, v2) or labelled as a
v3-draft result. See README.md beside this file for the source of each number.
usage: python docs/papers/berger-spectrum/social/make_floor_cards.py
"""
import sys
from pathlib import Path
import numpy as np
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent.parent / "two-ring-pg"))
import make_polished_plates as M          # noqa: E402  (the plate helpers and palette)

OUT = HERE / "cards"
INK, MUTED, BLUE, COPPER, TEAL, GREY = M.INK, M.MUTED, M.BLUE, M.COPPER, M.TEAL, M.GREY
RULE = M.RULE
SERIF = "Palatino Linotype"
DPI = 200
DOI = "doi:10.5281/zenodo.24412603"


def floor_law(j, t):
    return (j * (j + 2) - 2 * (t - 3) * j - 2 * (t - 2) * (t + 4)) / t


def mast(fig, right):
    M.text(fig, .055, .955, "DAVIS GEOMETRIC  /  THE FLOOR", 7.5, MUTED)
    M.text(fig, .945, .955, right, 7.5, MUTED, ha="right")


def rule(fig, y):
    fig.add_artist(plt.Line2D([.055, .945], [y, y], color=RULE, lw=.8, transform=fig.transFigure))


def save(fig, name):
    OUT.mkdir(exist_ok=True)
    fig.savefig(OUT / name, dpi=DPI, facecolor="white")
    plt.close(fig)
    print("wrote", OUT / name)


def law_axes(ax, small=False):
    t = np.linspace(1.0, 10.0, 400)
    ax.axvspan(2.75, 10.0, color=TEAL, alpha=.07, lw=0)
    for j in range(0, 9):
        ax.plot(t, floor_law(j, t), color=BLUE if j else COPPER, lw=1.6 if j == 0 else 1.0,
                alpha=1.0 if j == 0 else .55)
    ax.axhline(0, color=GREY, lw=.6)
    ax.plot([3], [-14 / 3], "o", color=COPPER, ms=5)
    if not small:
        ax.annotate("t = 3, j = 0:  −14/3", (3, -14 / 3), (1.15, -11.0), fontsize=8, color=INK,
                    arrowprops=dict(arrowstyle="-", color=MUTED, lw=.6))
        ax.text(5.9, 10.5, "each curve is the floor of its\nown sector j, proved for every\nstretch t in [2.75, 10]", fontsize=8, color=TEAL)
        ax.text(1.35, 6.0, "j = 0", fontsize=8, color=COPPER)
        ax.text(2.0, 13.8, "j = 1 … 8", fontsize=8, color=BLUE)
    for s in ("top", "right"):
        ax.spines[s].set_visible(False)
    for s in ("left", "bottom"):
        ax.spines[s].set_color(MUTED)
    ax.tick_params(colors=MUTED, labelsize=7)
    ax.set_xlabel("stretch  t", fontsize=8, color=MUTED)
    ax.set_ylabel("eigenvalue", fontsize=8, color=MUTED)
    ax.set_xlim(1, 10); ax.set_ylim(-16, 16)


def card_cover():
    fig = plt.figure(figsize=(6, 6))
    mast(fig, "THE FLOOR THEOREM")
    M.text(fig, .055, .86, "Where a squashed sphere's field", 20, fontfamily=SERIF)
    M.text(fig, .055, .81, "gives way first, exactly.", 20, fontfamily=SERIF)
    M.text(fig, .055, .745, r"$\lambda_j(t) = [\,j(j+2) - 2(t-3)j - 2(t-2)(t+4)\,]\,/\,t$", 12, color=BLUE)
    ax = fig.add_axes([.12, .14, .80, .55])
    law_axes(ax)
    M.text(fig, .055, .05, "One exact eigenvalue in every sector j, for every stretch t.  " + DOI, 7.5, MUTED)
    save(fig, "01_cover_floor_law.png")


def card_results():
    fig = plt.figure(figsize=(6, 6))
    mast(fig, "RESULTS")
    M.text(fig, .055, .87, "Exact, then proved the floor.", 22, fontfamily=SERIF)
    M.text(fig, .055, .805, "Published results (v2). Every value is a closed form or a certificate.", 9, MUTED)
    rows = [
        ("every t", "the twist is an exact eigenvector, symbolically in\nboth the sector j and the stretch t.", BLUE),
        ("[2.75, 10]", "it is the floor of every sector: 48,762 interval\nblock-slices with no failure, then Gershgorin.", BLUE),
        ("≤ 9", "states per block, whatever the sector's size:\na conserved charge splits the problem.", TEAL),
        ("−14/3", "the field's growth mode at t = 3 (twice); the\nleft-invariant sector has exactly three.", COPPER),
        ("168/3", "the floor gap on the Poincaré sphere: one third of\n168, its first scalar eigenvalue.", INK),
    ]
    y = .74
    rule(fig, y + .035)
    for num, cap, col in rows:
        M.text(fig, .055, y + .005, num, 24, color=col, fontfamily=SERIF)
        M.text(fig, .50, y - .005, cap, 8.5, INK)
        y -= .125
        rule(fig, y + .035)
    M.text(fig, .055, .07, "The Floor Theorem for the Yang–Mills Hessian at the Berger Field (" + DOI + ").", 7.5, MUTED)
    M.text(fig, .055, .045, "Bee Rosa Davis, Davis Geometric.", 7.5, MUTED)
    save(fig, "02_results.png")


def card_anatomy():
    fig = plt.figure(figsize=(6, 6))
    mast(fig, "ANATOMY")
    M.text(fig, .055, .87, "Three pieces, read off exactly.", 22, fontfamily=SERIF)
    M.text(fig, .055, .805, "Across a family of fields (x, y) on the sphere stretched by t, the twist's price splits as", 9, MUTED)
    M.text(fig, .055, .745, r"$\lambda \;=\; \kappa^2 \;+\; (2j + t + y^2) \;+\; \left((4 - 2t) - 2B\right)$", 15, color=BLUE)
    M.text(fig, .055, .69, r"$\kappa = (t-2)/\sqrt{t} - x - j/\sqrt{t}$,   $B = y^2 - 2\sqrt{t}\,x$", 9, MUTED)
    parts = [
        ("a square", "of its net turning, kappa: the sphere's turning\nminus the field's push and its own.", BLUE),
        ("a crossing", "the cheapest cost of crossing the circle,\n2j + t + y²: a lowest Landau level.", TEAL),
        ("a curvature", "term: the sphere's own (4 − 2t) against\ntwice the field's curvature B.", COPPER),
    ]
    y = .60
    rule(fig, y + .035)
    for num, cap, col in parts:
        M.text(fig, .055, y + .005, num, 20, color=col, fontfamily=SERIF)
        M.text(fig, .43, y - .005, cap, 8.5, INK)
        y -= .14
        rule(fig, y + .035)
    M.text(fig, .055, .13, "The same square of a net turning appears in the plasma ballooning operator,", 8.5, INK)
    M.text(fig, .055, .105, r"$\gamma/2 + \frac{3}{4}(s/\kappa - \alpha)^2$ (machine-checked).  Shared shape, not shared physics.", 8.5, INK)
    M.text(fig, .055, .05, "Exact identity, checked in Lean (Helike.price_anatomy, standard axioms). From the v3 draft.", 7.5, MUTED)
    save(fig, "03_anatomy.png")


def card_not():
    fig = plt.figure(figsize=(6, 6))
    mast(fig, "WHAT IT IS NOT")
    M.text(fig, .055, .87, "Said plainly.", 22, fontfamily=SERIF)
    items = [
        "Not a proof of the Yang–Mills mass gap. It is the exact floor of one\noperator, the Hessian, at one family of fields, on compact spheres.",
        "Not a statement about every field. Off the Berger field the twist is the\nfloor only up to an edge: the root of an explicit polynomial.",
        "Computer-assisted where it says so: interval certificates and\nGershgorin bounds, each with its receipt in the repository.",
        "Three simpler readings were written down before their data, and each\nfailed. The paper prints them, beside what held.",
    ]
    y = .78
    rule(fig, y + .03)
    for s in items:
        M.text(fig, .055, y - .015, s, 9, INK)
        y -= .15
        rule(fig, y + .03)
    M.text(fig, .055, .05, "Items 2 and 4 are from the v3 draft.  Receipts: github.com/nurdymuny/gigi.  " + DOI, 7.5, MUTED)
    save(fig, "04_what_it_is_not.png")


def card_preview():
    fig = plt.figure(figsize=(6, 3.135))
    M.text(fig, .05, .86, "DAVIS GEOMETRIC  /  THE FLOOR", 7.5, MUTED)
    M.text(fig, .05, .68, "The exact floor of", 18, fontfamily=SERIF)
    M.text(fig, .05, .56, "the Yang–Mills Hessian", 18, fontfamily=SERIF)
    M.text(fig, .05, .44, "on stretched spheres.", 18, fontfamily=SERIF)
    M.text(fig, .05, .12, DOI, 8, MUTED)
    ax = fig.add_axes([.64, .16, .32, .70])
    law_axes(ax, small=True)
    ax.set_ylabel("")
    save(fig, "05_link_preview_1200x627.png")


if __name__ == "__main__":
    card_cover(); card_results(); card_anatomy(); card_not(); card_preview()
