"""LinkedIn cards in the paper's plate style (make_polished_plates.py, reused, not copied).

Every number is from a committed receipt; see social/README.md for the source of each.
usage: python docs/papers/two-ring-pg/social/make_linkedin_cards.py
"""
import json
import sys
from pathlib import Path

import numpy as np
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
import make_polished_plates as M          # noqa: E402  (the plate helpers, palette and renderer)

OUT = HERE / "cards"
GEO = json.loads((HERE.parent / "data" / "rings_3d.json").read_text())
C0 = GEO["cells"][0]
INK, MUTED, BLUE, COPPER, TEAL, GREY = M.INK, M.MUTED, M.BLUE, M.COPPER, M.TEAL, M.GREY
SERIF = "Palatino Linotype"
DPI = 200


def mast(fig, right):
    M.text(fig, .055, .955, "DAVIS GEOMETRIC  /  CURVATURE RECOVERY", 7.5, MUTED)
    M.text(fig, .945, .955, right, 7.5, MUTED, ha="right")


def headline(fig, y, s, size=22):
    M.text(fig, .055, y, s, size, fontfamily=SERIF)


def big(fig, x, y, num, cap, color=BLUE, size=30):
    M.text(fig, x, y, num, size, color=color, fontfamily=SERIF)
    M.text(fig, x, y - .072, cap, 9, MUTED)


def scene(fig, rect, mode, cell=C0, label=False):
    ax = M.axes(fig, rect)
    cam = M.technical_camera(M.centre_tet(cell))
    keep = [t for t in cell["tets"] if mode != "one" or t["ring"] in ("K", "ring1")]
    pts = np.vstack([M.project(np.array(t["v"]), cam) for t in keep])
    M.frame(ax, pts, pad=1.06)
    M.assembly(ax, cell, cam, mode=mode, centre_label=label)
    return ax


def save(fig, name):
    OUT.mkdir(exist_ok=True)
    fig.savefig(OUT / name, dpi=DPI, facecolor="white")
    plt.close(fig)
    print("wrote", OUT / name)


def card_cover():
    fig = plt.figure(figsize=(6, 6))
    mast(fig, "READING THE CURVE")
    headline(fig, .905, "One ring believes the noise.\nTwo rings outvote it.")
    scene(fig, [.03, .30, .46, .42], "one")
    scene(fig, [.51, .30, .46, .42], "two")
    big(fig, .09, .30, "253.5", "One ring: 5 cells.\nNoise amplification.", COPPER)
    big(fig, .57, .30, "23.1", "Two rings: 15 cells.\nNoise amplification.")
    M.rule(fig, .135, .055, .945)
    M.text(fig, .055, .115, "Force error on the Poincaré homology sphere: the standard method stalls at 26%;\n"
           "the two-ring recovery reaches 1.45%. Open source in GIGI.", 8.2, INK)
    save(fig, "01_cover_one_vs_two_rings.png")


def card_fold():
    fig = plt.figure(figsize=(6, 6))
    mast(fig, "THE FOLD")
    headline(fig, .905, "Keep the cells that change\nthe answer.")
    scene(fig, [.03, .30, .46, .42], "fold")
    ax = M.axes(fig, [.51, .30, .46, .42])
    cam = M.technical_camera(M.centre_tet(C0))
    M.frame(ax, M.exploded_points(C0, cam, 1.15), pad=1.04)
    M.exploded(ax, C0, cam, 1.15)
    big(fig, .09, .30, "2–4", "of about 10 second-ring cells,\nchosen well: stable.")
    big(fig, .57, .30, "48", "runaway modes when the same\nnumber is picked at random.", COPPER)
    M.legend(fig, .135, folded=True)
    M.text(fig, .055, .085, "Accuracy kept: 1.57% error on the finest mesh against 1.45% for all ten.", 8.2, INK)
    save(fig, "02_fold_keep_what_matters.png")


def card_results():
    fig = plt.figure(figsize=(6, 6))
    mast(fig, "RESULTS")
    headline(fig, .905, "From a 26% floor to 1.45%.")
    M.text(fig, .055, .80, "Measured on the paper's meshes. Every number traces to a saved receipt.", 8.5, MUTED)
    M.rule(fig, .755, .055, .945)
    rows = [("26% to 1.45%", "Abelian control: the standard (Whitney) force stalls;\nthe two-ring force converges.", BLUE),
            ("25.5% to 2.51%", "Non-commuting SU(2) field, research code:\nthe same reading carries over.", BLUE),
            ("0.87%", "off the true path when the lattice moves while\nthe space flows (SU(2), finest mesh, research code).", BLUE),
            ("0", "runaway modes for the two-ring SU(2) force\n(two smallest meshes); one ring has 1,451.", TEAL),
            ("10⁻¹⁴", "agreement between the GIGI engine verb\nRECOVER_CURVATURE and the research code.", INK)]
    y = .715
    for num, cap, col in rows:
        M.text(fig, .055, y, num, 20, color=col, fontfamily=SERIF)
        M.text(fig, .50, y - .004, cap, 8.2, INK)
        y -= .125
        M.rule(fig, y + .03, .055, .945)
    M.text(fig, .055, .085, "Paper: Reading the Curve, Not Counting the Points (doi:10.5281/zenodo.24328085).\n"
           "Code: GIGI, open source (PolyForm).", 7.8, MUTED)
    save(fig, "03_results.png")


def card_time():
    """The SU(2) flow in time: R at n = 1, 2, 3 for two rings, native and the frozen control."""
    flow = json.loads((HERE.parent / "data" / "su2_flow.json").read_text())
    fig = plt.figure(figsize=(6, 6))
    mast(fig, "LETTING THE LATTICE MOVE")
    headline(fig, .905, "The space flows. The lattice\nfollows it.")
    M.text(fig, .055, .755, "The links move under the force while the metric underneath follows its Ricci flow;\n"
           "at the end they are compared with where the true field went.", 8.2, MUTED)
    ax = fig.add_axes([.12, .30, .50, .38])
    n = np.array([1, 2, 3])
    series = [("pg", "two rings", BLUE, "o", "-", 2.4), ("native", "standard", GREY, "s", "-", 1.4),
              ("pg_fixed", "background frozen", COPPER, "v", (0, (4, 2)), 1.6)]
    for key, lab, col, mk, ls, lw in series:
        R = np.array([100 * flow[f"{key}_n{k}"]["R"][-1] for k in n])
        ax.plot(n, R, color=col, marker=mk, ls=ls, lw=lw, ms=5, label=lab)
    ax.set_yscale("log"); ax.set_xticks(n); ax.set_xticklabels(["n = 1", "n = 2", "n = 3"])
    ax.set_yticks([1, 2, 5, 10, 20]); ax.set_yticklabels(["1%", "2%", "5%", "10%", "20%"]); ax.minorticks_off()
    ax.set_ylim(.6, 22); ax.set_xlim(.8, 3.2)
    for s_ in ("top", "right"):
        ax.spines[s_].set_visible(False)
    ax.tick_params(labelsize=7.5, colors=MUTED)
    ax.legend(frameon=False, fontsize=7.5, loc="upper right")
    big(fig, .68, .62, "0.87%", "two rings, finest mesh:\noff the true path", BLUE, size=26)
    big(fig, .68, .43, "8%", "background frozen:\nstays stuck, as it should", COPPER, size=26)
    M.rule(fig, .215, .055, .945)
    M.text(fig, .055, .19, "13.8%, 1.76%, 0.87% at n = 1, 2, 3; the standard force 7.0%, 2.21%, 1.07%. Against only\n"
           "the motion the moving space causes, the two-ring error is 10.6% at n = 3: falling, not solved.", 8, INK)
    M.text(fig, .055, .085, "Research code, one metric, one short window. Paper: Reading the Curve, Section 9.3.", 7.8, MUTED)
    save(fig, "06_letting_the_lattice_move.png")


def card_three_ways():
    fig = plt.figure(figsize=(6, 6))
    M.text(fig, .055, .955, "DAVIS GEOMETRIC  /  THREE WAYS ACROSS A TURN", 7.5, MUTED)
    headline(fig, .905, "Three ways across a turn.")
    ax = fig.add_axes([.06, .31, .52, .52])
    ax.set_aspect("equal"); ax.axis("off")
    t = np.radians(70)
    s = np.linspace(0, 2 * np.pi, 400)
    ax.plot(np.cos(s), np.sin(s), color=M.RULE, lw=1)
    ax.plot([0, 1], [0, 0], color=INK, lw=1); ax.plot([0, np.cos(t)], [0, np.sin(t)], color=INK, lw=1)
    a = np.linspace(0, t, 100)
    ax.plot(np.cos(a), np.sin(a), color=BLUE, lw=3)
    ax.plot([1, np.cos(t)], [0, np.sin(t)], color=COPPER, lw=3)
    q = np.array([1, np.tan(t / 2)])
    ax.plot([1, q[0], np.cos(t)], [0, q[1], np.sin(t)], color=TEAL, lw=2.4, ls=(0, (4, 2)))
    for p in ([1, 0], [np.cos(t), np.sin(t)]):
        ax.plot(*p, "o", color=INK, ms=4)
    ax.set_xlim(-1.08, 1.25); ax.set_ylim(-1.08, 1.12)
    for i, (col, name, ls) in enumerate([(COPPER, "chord: the shortcut", "-"), (BLUE, "arc: the true path", "-"),
                                         (TEAL, "outside: the detour", (0, (4, 2)))]):
        y0 = .255 - i * .03
        fig.add_artist(plt.Line2D([.09, .13], [y0, y0], transform=fig.transFigure, color=col, lw=3, ls=ls))
        M.text(fig, .14, y0 + .011, name, 8.4)
    rows = [("57.30°", "the arc fits in one ticket\n(one radian)", BLUE),
            ("60°", "the chord fits in one ticket\n(the hexagon)", COPPER),
            ("90°", "the chord costs one ticket\nof lattice energy (Pythagoras)", TEAL)]
    y = .76
    for num, cap, col in rows:
        M.text(fig, .62, y, num, 20, color=col, fontfamily=SERIF)
        M.text(fig, .62, y - .062, cap, 8, INK)
        y -= .165
    M.rule(fig, .155, .055, .945)
    M.text(fig, .055, .14, "For small turns, the shortcut falls short of the true path by half what the detour overshoots.\n"
           "A lattice measures the chord; the continuum is made of arcs; the gap between them is curvature.", 8.2, INK)
    M.text(fig, .055, .085, "Paper: Three Ways Across a Turn: Chord, Arc and Tangent (doi:10.5281/zenodo.24375338).\n"
           "Every identity checked by script.", 7.8, MUTED)
    save(fig, "04_three_ways_across_a_turn.png")


def card_landscape():
    fig = plt.figure(figsize=(6, 3.135))
    M.text(fig, .045, .93, "DAVIS GEOMETRIC  /  CURVATURE RECOVERY", 7, MUTED)
    M.text(fig, .045, .84, "Reading the curve,\nnot counting the points.", 21, fontfamily=SERIF)
    M.text(fig, .045, .47, "A cell of the mesh reads its curvature\nfrom two rings of neighbours. The force\nerror falls from a 26% floor to 1.45%.", 8.4, MUTED)
    M.text(fig, .045, .19, "1.45%", 26, color=BLUE, fontfamily=SERIF)
    M.text(fig, .25, .155, "force error,\nfinest mesh", 8, MUTED)
    scene(fig, [.57, .04, .41, .92], "two")
    save(fig, "05_link_preview_1200x627.png")


if __name__ == "__main__":
    card_cover(); card_fold(); card_results(); card_three_ways(); card_landscape(); card_time()
