"""Figures for the Helike Ladder Theorem (By Enough v2, Section 4), drawn from closed forms and light sector solves.

usage: python scripts/helike_ladder_figures.py OUT_DIR

fig_ladder      the ladder: rungs by helicity, sideways leak = gauge part, top rung (Helike) leak-free
fig_prices      the two prices on the Berger path: helicity cost per sector against the flux discount B
fig_fluxfan     the nine flux eigenvalues on the Berger path; -B peels off below at t = 2; the flux gap 2t - 4
fig_coverage    where each proof reaches over (t, j): ladder floor theorem, Floor certificates, computed only
fig_xband       flux boundary x_flux against the published edges x*_A, x*_S (By Enough Table 1)
fig_spin        schematic: total spin on the round sphere, and the one component that survives stretching
Light: closed forms everywhere except fig_coverage, which solves 9(j+1)-dimensional sectors (j <= 24).
"""
import json
import math
import sys
from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402
import numpy as np  # noqa: E402
from matplotlib.patches import FancyArrowPatch, Rectangle  # noqa: E402

sys.path.insert(0, str(Path(__file__).resolve().parent))
import helike_curl_spectrum as hc  # noqa: E402

BLUE, ORANGE, AQUA = "#2a78d6", "#eb6834", "#1baf7a"
INK, INK2, MUTED, GRID, SURF = "#0b0b0b", "#52514e", "#8a8984", "#e4e3df", "#fcfcfb"
plt.rcParams.update({
    "font.family": "serif", "font.size": 9.5, "axes.edgecolor": MUTED, "axes.labelcolor": INK2,
    "xtick.color": INK2, "ytick.color": INK2, "axes.spines.top": False, "axes.spines.right": False,
    "axes.grid": True, "grid.color": GRID, "grid.linewidth": 0.6, "figure.facecolor": SURF,
    "axes.facecolor": SURF, "savefig.facecolor": SURF, "legend.frameon": False,
})


def save(fig, out, name):
    for ext in ("pdf", "png"):
        fig.savefig(out / f"{name}.{ext}", dpi=200, bbox_inches="tight")
    plt.close(fig)


def berger_h(t, j, m=None):
    """Helicity on the Berger path: h_m = (t - 4 - 2m)/sqrt t, top weight m = j/2 gives (t - 4 - j)/sqrt t."""
    m = j / 2 if m is None else m
    return (t - 4 - 2 * m) / math.sqrt(t)


def fig_ladder(out):
    fig, axes = plt.subplots(1, 2, figsize=(7.6, 3.6), sharex=True)
    for ax, t in zip(axes, (3.0, 8.0)):
        j = 6
        l = j / 2
        ms = np.arange(-l, l + 1)
        for m in ms:
            h = berger_h(t, j, m)
            leak = 2 * (l * (l + 1) - m * (m + 1))
            top = m == l
            ax.plot([-2.2, 0.0], [h, h], color=BLUE if top else INK2, lw=3.2 if top else 1.8, solid_capstyle="round")
            if leak > 0:
                ax.add_patch(FancyArrowPatch((0.15, h), (0.15 + leak, h), arrowstyle="-|>", mutation_scale=9,
                                             color=MUTED, lw=1.2))
                ax.text(0.15 + leak + 0.4, h, f"{leak:g}", va="center", fontsize=8, color=INK2)
            else:
                ax.text(0.3, h, "Helike: leaks nothing", va="center", fontsize=9, color=BLUE)
            ax.text(36, h, f"m = {m:+g}", va="center", ha="right", fontsize=8, color=BLUE if top else MUTED)
        ax.set_title(f"t = {t:g}, sector j = {j}", fontsize=10, color=INK)
        ax.set_xlim(-3.5, 37)
        ax.set_xticks([0, 10, 20, 30])
        ax.grid(axis="y", visible=False)
    axes[0].set_ylabel("helicity h_m (curl eigenvalue on the rung)")
    fig.supxlabel("sideways leak = gauge part, 2[l(l+1) − m(m+1)]", fontsize=9.5, color=INK2, y=-0.04)
    fig.suptitle("The Helike ladder: every rung but the top leaks, and the leak is its gauge part",
                 fontsize=10.5, color=INK, y=1.02)
    save(fig, out, "fig_ladder")


def fig_prices(out):
    fig, axes = plt.subplots(1, 3, figsize=(9.2, 3.2), sharey=False)
    for ax, t in zip(axes, (4.0, 6.0, 10.0)):
        js = np.arange(0, 17)
        cost = np.array([berger_h(t, j) ** 2 for j in js])
        B = 3 * t - 4
        x, y = hc.berger_xy(t)
        proved = [hc.floor_sufficient(x, y, t, int(j)) for j in js]
        ax.bar(js, cost, width=0.72, color=[BLUE if p else "#b9c9e0" for p in proved], zorder=2)
        ax.axhline(B, color=ORANGE, lw=1.6, zorder=3)
        ax.text(-0.6, B * 1.04, f"flux discount B = {B:g}", color=INK2, fontsize=8, ha="left", va="bottom",
                bbox=dict(facecolor=SURF, edgecolor="none", pad=1))
        best = int(js[np.argmin(cost)])
        ax.annotate(f"cheapest: j = {best}", (best, cost[best]), xytext=(best + 1.5, B * 0.55),
                    fontsize=8, color=INK2, arrowprops=dict(arrowstyle="-", color=MUTED, lw=0.8))
        ax.set_title(f"t = {t:g}", fontsize=10, color=INK)
        ax.set_xlabel("sector j")
        ax.set_ylim(0, max(B * 1.6, cost.max() * 0.0 + B * 1.6))
        ax.grid(axis="x", visible=False)
    axes[0].set_ylabel("helicity cost h_l² = (t − 4 − j)²/t")
    fig.suptitle("Two prices on the Berger path: λ_j = helicity cost − B. "
                 "Dark bars: sectors proved by the ladder floor theorem", fontsize=10, color=INK, y=1.03)
    save(fig, out, "fig_prices")


def fig_fluxfan(out):
    t = np.linspace(0.25, 10, 600)
    B, Th = 3 * t - 4, -t
    r = np.sqrt(B * B + 8 * Th * Th)
    curves = [("+B", B), ("+Θ  (×2)", Th), ("−Θ  (×2)", -Th), ("(B + √(B²+8Θ²))/2", (B + r) / 2),
              ("(B − √(B²+8Θ²))/2", (B - r) / 2)]
    fig, ax = plt.subplots(figsize=(6.4, 4.0))
    others = np.min(np.vstack([c for _, c in curves]), axis=0)
    gap = t >= 2
    ax.fill_between(t[gap], -B[gap], others[gap], color=BLUE, alpha=0.10, lw=0, zorder=1)
    for name, c in curves:
        ax.plot(t, c, color=MUTED, lw=1.2, zorder=2)
        ax.text(10.15, c[-1], name, color=INK2, fontsize=8, va="center")
    ax.plot(t, -B, color=BLUE, lw=2.4, zorder=3)
    ax.text(10.15, -B[-1], "−B  (×2): Helike's flux", color=BLUE, fontsize=8.5, va="center")
    ax.axvline(2, color=INK2, lw=0.8, ls=(0, (3, 3)))
    ax.text(2.1, 22, "t = 2: she becomes\nthe flux minimiser", fontsize=8, color=INK2, va="top")
    ax.text(6.3, -12.5, "flux gap 2t − 4", fontsize=8.5, color=BLUE)
    ax.set_xlim(0.25, 10)
    ax.set_xlabel("stretch t (Berger path)")
    ax.set_ylabel("eigenvalue of the flux form φ")
    ax.set_title("The flux fan: nine exact eigenvalues, the same in every sector", fontsize=10.5, color=INK)
    save(fig, out, "fig_fluxfan")


def fig_coverage(out, cache):
    ts = np.round(np.arange(2.0, 20.01, 0.25), 2)
    js = np.arange(0, 25)
    if cache.exists():
        comp = np.array(json.loads(cache.read_text())["is_floor"])
    else:
        comp = np.zeros((len(js), len(ts)), bool)
        for b, t in enumerate(ts):
            x, y = hc.berger_xy(t)
            for a, j in enumerate(js):
                _, H, _, _, _ = hc.operators(x, y, t, int(j))
                ev, _ = hc.gauge_orthogonal(H, hc.gauge_map(x, y, t, int(j)))
                lam = hc.twist_closed_form(x, y, t, int(j))
                comp[a, b] = ev[0] >= lam - 1e-8 * max(1, abs(lam))
        cache.write_text(json.dumps({"t": ts.tolist(), "j": js.tolist(), "is_floor": comp.tolist(),
                                     "tolerance": "1e-8 relative, double precision"}))
    lad = np.array([[hc.floor_sufficient(*hc.berger_xy(t), t, int(j)) for t in ts] for j in js])
    fig, ax = plt.subplots(figsize=(7.6, 4.4))
    dt = 0.25
    for a, j in enumerate(js):
        for b, t in enumerate(ts):
            if lad[a, b]:
                c = BLUE
            elif comp[a, b]:
                c = "#d9d8d3"
            else:
                c = "#6f6e69"
            ax.add_patch(Rectangle((t - dt / 2, j - 0.5), dt, 1, facecolor=c, edgecolor=SURF, lw=0.4))
    ax.add_patch(Rectangle((2.75, -0.5), 10 - 2.75, len(js), facecolor="none", edgecolor=AQUA, lw=2.2))
    ax.text(2.85, 23.6, "Floor Theorem: every sector proved\n(interval certificates), 2.75 ≤ t ≤ 10",
            fontsize=8, color=INK, va="top", bbox=dict(facecolor=SURF, edgecolor="none", alpha=0.85, pad=1.5))
    ax.plot(ts, ts - 4, color=INK2, lw=0.8, ls=(0, (2, 2)))
    ax.annotate("j = t − 4 (her helicity is zero)", (13.0, 9.0), xytext=(14.2, 3.2), fontsize=8, color=INK2,
                arrowprops=dict(arrowstyle="-", color=INK2, lw=0.8),
                bbox=dict(facecolor=SURF, edgecolor="none", pad=1.5))
    handles = [Rectangle((0, 0), 1, 1, color=BLUE), Rectangle((0, 0), 1, 1, color="#d9d8d3"),
               Rectangle((0, 0), 1, 1, color="#6f6e69"), Rectangle((0, 0), 1, 1, fc="none", ec=AQUA, lw=2)]
    ax.legend(handles, ["proved by the ladder floor theorem", "computed: she is the floor (not proved there)",
                        "computed: she is not the floor", "proved by the Floor certificates"],
              loc="upper left", bbox_to_anchor=(1.01, 1.0), fontsize=8)
    ax.set_xlim(1.85, 20.15)
    ax.set_ylim(-0.5, len(js) - 0.5)
    ax.set_xlabel("stretch t (Berger path)")
    ax.set_ylabel("sector j")
    ax.grid(False)
    ax.set_title("Where each proof reaches (gauge-orthogonal floor, sector by sector)", fontsize=10.5, color=INK)
    save(fig, out, "fig_coverage")
    return int(lad.sum()), int(comp.sum()), int((~comp).sum()), int((lad & ~comp).sum())


def fig_xband(out):
    tab = {2: [(-0.613240, -0.605292), (-0.682793, -0.657567), (-0.677814, -0.655100)],
           3: [(-0.335488, -0.318061), (-0.371582, -0.351580), (-0.382168, -0.355187)],
           4: [(-0.202972, -0.191702), (-0.208048, -0.191652), (-0.221163, -0.189150)],
           6: [(-0.082842, -0.078032), (-0.044874, -0.033249), (-0.035386, +0.008031)],
           8: [(-0.033799, -0.031530), (+0.031482, +0.040169), (+0.082513, +0.138269)]}
    fig, axes = plt.subplots(1, 3, figsize=(9.4, 3.3), sharey=True)
    tt = np.linspace(1.6, 8.6, 300)
    for k, (ax, lab) in enumerate(zip(axes, ("y = 0.5", "y = 1.5", "y = √t"))):
        yf = (lambda t: 0.5) if k == 0 else (lambda t: 1.5) if k == 1 else (lambda t: math.sqrt(t))
        xf = np.array([hc.x_flux(yf(t), t) for t in tt])
        ts = sorted(tab)
        xs = np.array([max(tab[t][k]) for t in ts])
        ax.fill_between(ts, xs, [hc.x_flux(yf(t), t) for t in ts], color=ORANGE, alpha=0.12, lw=0)
        ax.plot(tt, xf, color=BLUE, lw=2)
        ax.plot(ts, [tab[t][k][0] for t in ts], "o", color=INK, ms=4.5, mfc=SURF, mew=1.2)
        ax.plot(ts, [tab[t][k][1] for t in ts], "s", color=INK, ms=4, mew=0)
        ax.axhline(0, color=MUTED, lw=0.8)
        ax.set_title(lab, fontsize=10, color=INK)
        ax.set_xlabel("stretch t")
    axes[0].set_ylabel("x (field along the long circle)")
    axes[0].text(2.2, 0.75, "flux boundary x_flux:\nshe still minimises the flux below it", color=BLUE, fontsize=8)
    axes[1].text(4.7, -0.55, "shaded: she owns the flux\nbut loses on helicity", color=INK2, fontsize=8)
    axes[2].plot([], [], "o", color=INK, mfc=SURF, label="edge x*_A (H + GG*)")
    axes[2].plot([], [], "s", color=INK, label="edge x*_S (gauge-orthogonal)")
    axes[2].plot([], [], color=BLUE, lw=2, label="flux boundary x_flux")
    axes[2].legend(fontsize=7.5, loc="lower right")
    for ax in axes:
        ax.set_ylim(-0.8, 1.25)
    fig.suptitle("The edge in x is a helicity contest inside the flux-favourable region (By Enough Table 1)",
                 fontsize=10.5, color=INK, y=1.03)
    save(fig, out, "fig_xband")


def fig_spin(out):
    fig, axes = plt.subplots(1, 2, figsize=(7.4, 3.6))
    th = np.linspace(0, 2 * np.pi, 300)
    for ax, (rx, ry), title in zip(axes, ((1, 1), (0.72, 1.35)),
                                   ("Round sphere: total spin V = L + T + S", "Stretched: only V along the long circle")):
        ax.plot(rx * np.cos(th), ry * np.sin(th), color=MUTED, lw=1.2)
        ax.plot(rx * np.cos(th), 0.22 * ry * np.sin(th), color=GRID, lw=1)
        ax.set_aspect("equal")
        ax.axis("off")
        ax.set_title(title, fontsize=9.5, color=INK)
        ax.set_xlim(-1.7, 1.7)
        ax.set_ylim(-2.15, 1.75)
    a0 = axes[0]
    for ang in np.linspace(0.3, 2 * np.pi, 7, endpoint=False):
        a0.add_patch(FancyArrowPatch((0, 0), (0.85 * math.cos(ang), 0.85 * math.sin(ang)), arrowstyle="-|>",
                                     mutation_scale=8, color="#c9c8c3", lw=1))
    a0.add_patch(FancyArrowPatch((0, 0), (0.62, 0.62), arrowstyle="-|>", mutation_scale=11, color=ORANGE, lw=2))
    a0.text(0.66, 0.68, "V", color=ORANGE, fontsize=11)
    a0.text(0, -1.85, "every direction is equivalent:\nlabel states by |V| (KLC 2021)", ha="center",
            fontsize=8, color=INK2)
    a1 = axes[1]
    a1.plot([0, 0], [-1.4, 1.6], color=INK2, lw=0.8, ls=(0, (3, 3)))
    a1.text(0.06, 1.5, "long circle", fontsize=8, color=INK2)
    a1.add_patch(FancyArrowPatch((0, 0), (0.5, 0.9), arrowstyle="-|>", mutation_scale=10, color="#c9c8c3", lw=1.4))
    a1.plot([0.5, 0], [0.9, 0.9], color="#c9c8c3", lw=0.9, ls=(0, (2, 2)))
    a1.add_patch(FancyArrowPatch((0, 0), (0, 0.9), arrowstyle="-|>", mutation_scale=12, color=BLUE, lw=2.4))
    a1.text(-0.12, 0.45, "q", color=BLUE, fontsize=12, ha="right")
    a1.text(0, -1.85, "q = m + c_f + c_l survives the stretch;\nHelike is its top value", ha="center",
            fontsize=8, color=INK2)
    save(fig, out, "fig_spin")


def main(out):
    out = Path(out)
    out.mkdir(parents=True, exist_ok=True)
    fig_ladder(out)
    fig_prices(out)
    fig_fluxfan(out)
    stats = fig_coverage(out, out / "coverage_cache.json")
    fig_xband(out)
    fig_spin(out)
    print("coverage: ladder-proved cells %d, computed floor %d, computed not floor %d, ladder-proved but computed "
          "not floor %d (must be 0)" % stats)


if __name__ == "__main__":
    main(sys.argv[1])
