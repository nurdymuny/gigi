"""Figures for the edge paper "By Enough, But Not Too Much" (22 November 2026).

Every number drawn comes from a receipt or from a computation in this file (double precision, COMPUTED); nothing is
typed in by hand except the closed forms quoted from the paper.

  fig_helike   her shape |f|^2 = cos^(2j)(eta/2) by sector, and her lift term 2xj/sqrt t (closed forms, as in
               scripts/ricci_su2_twist_family_figures.py).
  fig_region   the region of the sectors j <= 12 in (x, y) at t = 2, 3, 4, 6, 8, between x_lo and x*_12, coloured by
               the block that sets the upper edge (computed here with Blocks.phi_A of scripts/ricci_su2_exact_edge.py;
               cache research/edge-paper-figures-20261122/region_j12.json), the proved bound r_lo, and the half-plane
               x > 0 where Theorem xpos proves she loses (H + GG^*).
  fig_berger   along the Berger path, t in [2.55, 2.85]: her eigenvalue lambda_t(4) against the lowest competitor of
               sector 4, for H + GG^* and for the gauge slice, and the margins over j <= 12 (computed here; cache
               berger_path.json); t_A and t* from research/su2-floor-family-20261119/exact_edge.json.
  fig_prereg   Test A (wp4_heldout.json) and Test B (wp4_general_fields.json).
  fig_su3      Test C post-hoc margins (wp4_su3_diagnostic_posthoc.json) next to the SU(2) analogue at t = 4 (computed
               here; cache su2_margins_t4.json).

usage: OMP_NUM_THREADS=1 python scripts/edge_paper_figures.py [--recompute]
"""
import json
import os
import sys
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
OUT = ROOT / "docs/papers/by-enough/figures"
CACHE = ROOT / "research/edge-paper-figures-20261122"
R1119 = ROOT / "research/su2-floor-family-20261119"
R1121 = ROOT / "research/edge-theorem-20261121"

T_REGION = (2.0, 3.0, 4.0, 6.0, 8.0)
Y_REGION = tuple(float(v) for v in np.round(np.arange(0.05, 3.0001, 0.05), 4))
X_SCAN = np.round(np.arange(-14.0, 1.5001, 0.05), 6)

_BL = None


def _blocks():
    global _BL
    if _BL is None:
        import ricci_su2_exact_edge as E
        _BL = (E, E.Blocks())
    return _BL


# ----------------------------------------------------------------------------------------------- computations
def _region_point(ty):
    """Sign changes of phi_A (j <= 12) on X_SCAN at (t, y), refined by brentq; blocks either side of each change."""
    from scipy.optimize import brentq
    tv, yv = ty
    E, bl = _blocks()
    f = lambda xv: bl.phi_A(xv, yv, tv)                        # noqa: E731
    vals = np.array([f(xv) for xv in X_SCAN])
    changes = []
    for i in range(len(X_SCAN) - 1):
        a, b = vals[i], vals[i + 1]
        if (a < 0) != (b < 0):
            r = brentq(f, X_SCAN[i], X_SCAN[i + 1], xtol=1e-12)
            changes.append({"x": r, "dir": "up" if b >= 0 else "down",
                            "block_left": list(bl.phi_A(r - 1e-4, yv, tv, True)[1]),
                            "block_right": list(bl.phi_A(r + 1e-4, yv, tv, True)[1])})
    return {"t": tv, "y": yv, "changes": changes, "phi_at_scan_ends": [float(vals[0]), float(vals[-1])]}


def compute_region():
    pts = [(tv, yv) for tv in T_REGION for yv in Y_REGION]
    with ProcessPoolExecutor(max_workers=14) as ex:
        rows = list(ex.map(_region_point, pts, chunksize=2))
    out = {"status": "COMPUTED (double precision); operator H + GG^*; sectors j <= 12; phi_A of "
                     "scripts/ricci_su2_exact_edge.py; sign changes on a 0.05 grid of x in [-14, 1.5], brentq to 1e-12",
           "t": list(T_REGION), "y": list(Y_REGION), "rows": rows}
    (CACHE / "region_j12.json").write_text(json.dumps(out, indent=1))
    return out


def _berger_point(tv):
    E, bl = _blocks()
    import ricci_su3_test2_family as T2
    xv, yv = (2 - tv) / np.sqrt(tv), np.sqrt(tv)
    lam4 = E.lam_tw(xv, yv, tv, 4)
    d = bl.data(xv, yv, tv); best = np.inf
    for q in np.arange(-4, 4.5, 1.0):
        if abs(abs(q) - 4) < 1e-9:
            continue
        _, B = bl.block(d, 4, q, tv)
        if B.size:
            best = min(best, float(np.linalg.eigvalsh(B)[0]))
    S, C2, eps = T2.su2_data()
    H, G = T2.hessian_and_gauge(C2, [tv, 1.0, 1.0], eps, [xv * np.eye(3)[0], yv * np.eye(3)[1], yv * np.eye(3)[2]],
                                S.spin_rep(4))
    mu = list(T2.slice_spectrum(H, G)); removed = 0
    for _ in range(2):
        k = int(np.argmin(np.abs(np.array(mu) - lam4)))
        if abs(mu[k] - lam4) < 1e-8 * max(1, abs(lam4)):
            mu.pop(k); removed += 1
    pA, aA = bl.phi_A(xv, yv, tv, True); pS, aS = E.phi_S(xv, yv, tv, True)
    return {"t": tv, "x": xv, "y": yv, "lambda4": lam4, "comp4_A": best, "comp4_S": float(min(mu)),
            "slice_copies_removed": removed, "phi_A": float(pA), "phi_A_block": list(aA), "phi_S": float(pS),
            "phi_S_sector": int(aS)}


def compute_berger():
    ts = [float(v) for v in np.round(np.linspace(2.55, 2.85, 121), 6)]
    with ProcessPoolExecutor(max_workers=14) as ex:
        rows = list(ex.map(_berger_point, ts))
    out = {"status": "COMPUTED (double precision); Berger path x=(2-t)/sqrt t, y=sqrt t; sector 4 and min over j<=12",
           "rows": rows}
    (CACHE / "berger_path.json").write_text(json.dumps(out, indent=1))
    return out


def _su2_margins(yv, tv=4.0):
    """Per-sector gauge-orthogonal margins mu_min(non-twist) - lambda_j, j <= 12, at the x maximising the worst of
    them (the same spectrum and the same rule as the Test C diagnostic, which uses the slice spectrum)."""
    from scipy.optimize import minimize_scalar
    E, bl = _blocks()
    import ricci_su3_test2_family as T2
    S, C2, eps = T2.su2_data(); rho = {j: S.spin_rep(j) for j in range(13)}

    def per_sector(xv):
        M = [xv * np.eye(3)[0], yv * np.eye(3)[1], yv * np.eye(3)[2]]; m = []
        for j in range(13):
            H, G = T2.hessian_and_gauge(C2, [tv, 1.0, 1.0], eps, M, rho[j])
            mu = list(T2.slice_spectrum(H, G)); lam = E.lam_tw(xv, yv, tv, j)
            for _ in range(2):                                  # her two copies (Prop. exact), nothing else
                k = int(np.argmin(np.abs(np.array(mu) - lam)))
                assert abs(mu[k] - lam) < 1e-8 * max(1, abs(lam)), "twist copy missing from the slice spectrum"
                mu.pop(k)
            m.append(float(min(mu) - lam))
        return m
    xs = np.arange(-8.0, 1.0001, 0.05)
    w = [min(per_sector(xv)) for xv in xs]
    k = int(np.argmax(w))
    r = minimize_scalar(lambda xv: -min(per_sector(xv)), bounds=(xs[max(k - 1, 0)], xs[min(k + 1, len(xs) - 1)]),
                        method="bounded", options={"xatol": 1e-9})
    return {"t": tv, "y": yv, "x_best": float(r.x), "margins": per_sector(float(r.x))}


def compute_su2_margins():
    with ProcessPoolExecutor(max_workers=4) as ex:
        rows = list(ex.map(_su2_margins, (0.5, 1.0, 2.0, 3.0)))
    out = {"status": "COMPUTED (double precision); SU(2) family at t=4; gauge-orthogonal (slice) spectrum, general "
                     "builder; per-sector margins mu_min - lambda_j (two twist copies removed) at the x maximising "
                     "min_j over j <= 12 (0.05 grid on [-8, 1], then bounded refinement)", "rows": rows}
    (CACHE / "su2_margins_t4.json").write_text(json.dumps(out, indent=1))
    return out


def load_or(name, fn, recompute):
    p = CACHE / name
    if p.exists() and not recompute:
        return json.loads(p.read_text())
    return fn()


# ----------------------------------------------------------------------------------------------- style
import matplotlib                                      # noqa: E402
matplotlib.use("Agg")
import matplotlib.pyplot as plt                        # noqa: E402
from matplotlib.lines import Line2D                    # noqa: E402
from matplotlib.patches import Patch                   # noqa: E402

SURF, INK, INK2, MUTED, GRID = "#ffffff", "#0b0b0b", "#52514e", "#8a8984", "#e6e5e1"
HELIKE = "#2a78d6"                                     # her colour, every figure
HELIKE_RAMP = ["#86b6ef", "#5598e7", "#2a78d6", "#1c5cab", "#0d366b"]     # one hue, light -> dark
# Rivals: one colour per competitor kind, validated with the dataviz validator (adjacent order D0, (3,1/2), (4,0), D+):
#   yellow, aqua, orange, violet -> CVD dE >= 9.1, normal-vision dE >= 22.9 (light mode).
FAM_COL = {"D0": "#eda100", "B3h": "#1baf7a", "B40": "#eb6834", "D+": "#6250d6"}
FAM_NAME = {"B40": "$B_{4,0}$", "B3h": r"$B_{3,1/2}$", "D0": r"near-top $|q|=l$", "D+": r"near-top $|q|=l+1$"}
TEXTW = 6.3                                            # inches, the paper's text width (11pt, 1.1in margins)

plt.rcParams.update({
    "font.family": "sans-serif", "font.sans-serif": ["DejaVu Sans"], "mathtext.fontset": "dejavusans",
    "font.size": 7.5, "axes.titlesize": 8, "axes.labelsize": 7.5, "xtick.labelsize": 7, "ytick.labelsize": 7,
    "legend.fontsize": 7, "axes.edgecolor": INK2, "axes.labelcolor": INK, "axes.linewidth": 0.6,
    "xtick.color": INK2, "ytick.color": INK2, "xtick.major.width": 0.6, "ytick.major.width": 0.6,
    "xtick.major.size": 2.5, "ytick.major.size": 2.5, "text.color": INK, "axes.titlecolor": INK,
    "figure.facecolor": SURF, "axes.facecolor": SURF, "savefig.facecolor": SURF, "axes.spines.top": False,
    "axes.spines.right": False, "lines.linewidth": 1.3, "lines.solid_capstyle": "round",
    "lines.solid_joinstyle": "round", "pdf.fonttype": 42, "legend.frameon": False, "hatch.linewidth": 0.5,
})


def family(block):
    j, q = block[0], abs(block[1]); l = j / 2
    if (j, q) == (4, 0.0):
        return "B40"
    if (j, q) == (3, 0.5):
        return "B3h"
    if abs(q - (l + 1)) < 1e-9:
        return "D+"
    if abs(q - l) < 1e-9:
        return "D0"
    return "other"


def bname(block, signed=False):
    j, q = block[0], float(block[1]); q = q if signed else abs(q)
    qs = {0.5: r"\frac{1}{2}", 1.5: r"\frac{3}{2}", 2.5: r"\frac{5}{2}", 3.5: r"\frac{7}{2}"}
    qa = abs(q); body = qs.get(qa, f"{int(qa)}" if qa == int(qa) else f"{qa:g}")
    return f"$({j},{'-' if q < 0 else ''}{body})$"


def grid(ax, axis="both"):
    ax.grid(color=GRID, lw=0.5, axis=axis); ax.set_axisbelow(True)


def panel_tag(ax, s):
    ax.text(-0.02, 1.02, s, transform=ax.transAxes, ha="right", va="bottom", fontsize=8.5, fontweight="bold")


def save(fig, name):
    fig.savefig(OUT / f"{name}.pdf"); fig.savefig(OUT / f"{name}.png", dpi=220); plt.close(fig)
    print("wrote", OUT / f"{name}.pdf")


# ----------------------------------------------------------------------------------------------- figure 1
def fig_helike():
    fig, (a1, a2) = plt.subplots(1, 2, figsize=(TEXTW, 2.35), constrained_layout=True)
    eta = np.linspace(0, np.pi, 600); deg = np.degrees(eta)
    for jv, col in zip((0, 2, 4, 8, 16), HELIKE_RAMP):
        f2 = np.cos(eta / 2) ** (2 * jv)
        a1.plot(deg, f2, color=col)
        if jv == 0:
            a1.text(178, 1.0, "$j=0$", ha="right", va="bottom", fontsize=7)
        else:
            lvl = {2: 0.72, 4: 0.52, 8: 0.32, 16: 0.12}[jv]
            k = int(np.argmin(np.abs(f2 - lvl)))
            a1.annotate(f"$j={jv}$", (deg[k], f2[k]), xytext=(7, 3), textcoords="offset points", fontsize=7, bbox=dict(boxstyle="square,pad=0.1", fc=SURF, ec="none", alpha=0.9), zorder=5)
    a1.set_xlim(0, 180); a1.set_ylim(0, 1.08); a1.set_xticks(range(0, 181, 30))
    a1.set_xlabel(r"angle $\eta$ from the pole of the long circle (degrees)")
    a1.set_ylabel(r"$|f|^2=\cos^{2j}(\eta/2)$")
    a1.set_title("Her shape: a cap that tightens as the sector rises", loc="left")
    grid(a1); panel_tag(a1, "a")

    xv = np.linspace(-3, 3, 200)
    a2.axvspan(-3, 0, color=HELIKE, alpha=0.08, lw=0)
    a2.axhline(0, color=INK2, lw=0.6); a2.axvline(0, color=INK2, lw=0.6)
    for tv, col in zip((2.0, 3.0, 6.0), (HELIKE_RAMP[0], HELIKE_RAMP[2], HELIKE_RAMP[4])):
        a2.plot(xv, 2 * xv * 4 / np.sqrt(tv), color=col)
        a2.annotate(f"$t={tv:g}$", (-3, -24 / np.sqrt(tv)), xytext=(-3, 0), textcoords="offset points",
                    ha="right", va="center", fontsize=7)
    a2.text(-2.05, 15.5, "discount: $x<0$,\nthe field along\nthe long circle\nturns against her", ha="center",
            va="top", fontsize=6.5, color=INK, bbox=dict(boxstyle="square,pad=0.1", fc=SURF, ec="none", alpha=0.9), zorder=5)
    a2.set_xlim(-3.9, 3); a2.set_ylim(-18, 18)
    a2.set_xlabel(r"$x$ (field along the long circle)")
    a2.set_ylabel(r"lift term $2xj/\sqrt{t}$  ($j=4$)")
    a2.set_title("What lifts her cheaply: the sign of $x$", loc="left")
    grid(a2); panel_tag(a2, "b")
    save(fig, "fig_helike")


# ----------------------------------------------------------------------------------------------- figure 2
def fig_region(reg):
    ts = reg["t"]
    fig, axes = plt.subplots(2, len(ts), figsize=(TEXTW, 5.6), sharey=True, constrained_layout=True,
                             gridspec_kw={"height_ratios": [1, 1]})
    for c, tv in enumerate(ts):
        rows = sorted([r for r in reg["rows"] if r["t"] == tv], key=lambda r: r["y"])
        y = np.array([r["y"] for r in rows])
        xlo = np.array([r["changes"][0]["x"] for r in rows]); xst = np.array([r["changes"][-1]["x"] for r in rows])
        fam = [family(r["changes"][-1]["block_right"]) for r in rows]
        blk = [tuple(r["changes"][-1]["block_right"]) for r in rows]
        rlo = -(np.sqrt(tv) + 2 / np.sqrt(tv)) - np.sqrt(5 * tv + 3 * y ** 2 + 4)
        bx, by = (2 - tv) / np.sqrt(tv), np.sqrt(tv)
        for r_, ax in enumerate(axes[:, c]):
            xl, xr = ((-13.5, 1.0) if r_ == 0 else (-2.5, 0.5))
            ax.axvspan(0, xr, facecolor="none", edgecolor="#b9b8b3", hatch="//////", lw=0, zorder=0)
            # region strips, coloured by the family of the block setting the upper edge (half-step bands in y)
            for k in range(len(y)):
                y0 = y[k] - 0.025; y1 = y[k] + 0.025
                ax.fill_betweenx([y0, y1], xlo[k], xst[k], color=FAM_COL[fam[k]], alpha=0.30, lw=0, zorder=1)
            ax.plot(xst, y, color=INK, lw=0.7, zorder=3)
            ax.plot(xlo, y, color=INK, lw=0.7, zorder=3)
            # upper edge, coloured by competitor family (runs)
            k0 = 0
            for k in range(1, len(y) + 1):
                if k == len(y) or fam[k] != fam[k0]:
                    ax.plot(xst[k0:k], y[k0:k], color=FAM_COL[fam[k0]], lw=1.8, zorder=4, solid_capstyle="butt")
                    if r_ == 1:
                        mid = (k0 + k - 1) // 2
                        if abs(y[mid] - by) < 0.2 and bx > xst[mid] - 0.3:
                            mid = max(k0, mid - 5)          # keep the label off the Berger marker
                        names = []
                        for b in blk[k0:k]:
                            if bname(b) not in names:
                                names.append(bname(b))
                        ax.text(min(xst[mid] + 0.06, 0.45), y[mid], ", ".join(names), fontsize=5.6, va="center",
                                ha="left" if xst[mid] + 0.06 < 0.45 else "right", color=INK, zorder=6,
                                bbox=dict(boxstyle="square,pad=0.08", fc=SURF, ec="none", alpha=0.85))
                    k0 = k
            ax.axvline(0, color=INK2, lw=0.7, zorder=2)
            if r_ == 0:
                ax.plot(rlo, y, color=INK2, lw=0.8, ls=(0, (3, 2)), zorder=3)
            if xl <= bx <= xr:
                ax.plot([bx], [by], marker="*", ms=8.5, color=INK, mec=SURF, mew=0.9, ls="none", zorder=7)
                below = by > 2.5
                left = r_ == 1 and bx < -1.9
                ax.annotate("Berger", (bx, by), xytext=(4 if left else -4, -9 if below else 5),
                            textcoords="offset points", ha="left" if left else "right", fontsize=6.5, zorder=7,
                            bbox=dict(boxstyle="square,pad=0.08", fc=SURF, ec="none", alpha=0.85))
            ax.set_xlim(xl, xr); ax.set_ylim(0, 3.0)
            grid(ax)
            if r_ == 0:
                ax.set_title(f"$t={tv:g}$")
                ax.set_xticks([-12, -8, -4, 0])
            else:
                ax.set_xticks([-2, -1, 0])
                ax.set_xlabel("$x$")
        if c == 0:
            axes[0, 0].set_ylabel("$y$  (whole region)"); axes[1, 0].set_ylabel("$y$  (zoom on the upper edge)")
    a = axes[0, 0]
    a.text(-9.6, 2.75, r"$r_{\mathrm{lo}}$", fontsize=6.8, color=INK2, ha="left", va="center")
    a.text(-4.6, 2.75, r"$x_{\mathrm{lo}}$", fontsize=6.8, ha="right", va="center")
    # legend 1: colour = the rival that reaches her first (sets x*_12), with the blocks each colour contains
    seen = {}
    for r in reg["rows"]:
        bk = tuple(r["changes"][-1]["block_right"]); f = family(bk)
        nm = bname((bk[0], abs(bk[1])))
        nm = nm.replace("(", r"(").replace(",", r",\pm", 1) if abs(bk[1]) > 0 else nm
        seen.setdefault(f, [])
        if nm not in seen[f]:
            seen[f].append(nm)
    words = {"B40": "sector 4, charge 0 (edge = root of $P_2$)", "B3h": "sector 3, charge $\\pm\\frac{1}{2}$",
             "D0": "top orbital weight, charge $\\pm l$", "D+": "near-top, charge $\\pm(l+1)$",
             "other": "other blocks"}
    order = [f for f in ("B40", "B3h", "D0", "D+", "other") if f in seen]
    h1 = []
    for f in order:
        blks = sorted(seen[f], key=lambda z: (len(z), z))
        h1.append(Patch(facecolor=FAM_COL[f], alpha=0.55, edgecolor=FAM_COL[f], lw=0.8,
                        label=words[f] + ":  " + ", ".join(blks)))
    l1 = fig.legend(handles=h1, loc="outside lower left", ncol=1, handlelength=1.6, borderaxespad=0.3,
                    title="Colour: the rival that reaches her first\n(it sets her upper edge $x^*_{12}$; sectors $j\\leq12$)",
                    title_fontsize=7, alignment="left")
    h2 = [Line2D([], [], color=INK, lw=0.7, label="left boundary $x_{\\mathrm{lo}}$, right boundary $x^*_{12}$; fill: her region"),
          Line2D([], [], color=INK2, lw=0.8, ls=(0, (3, 2)), label=r"$r_{\mathrm{lo}}$: the proved lower bound"),
          Patch(facecolor="none", edgecolor="#b9b8b3", hatch="//////", label="$x>0$: she loses in some sector ($H+GG^*$, proved)"),
          Line2D([], [], marker="*", color=INK, mec=SURF, ls="none", ms=8, label="the Berger field $x=(2-t)/\\sqrt{t}$, $y=\\sqrt{t}$")]
    fig.legend(handles=h2, loc="outside lower right", ncol=1, handlelength=1.6, borderaxespad=0.3,
               title="Lines and marks", title_fontsize=7, alignment="left")
    save(fig, "fig_region")


# ----------------------------------------------------------------------------------------------- figure 3
def fig_berger(bp, ee):
    rows = bp["rows"]; t = np.array([r["t"] for r in rows])
    lam = np.array([r["lambda4"] for r in rows]); cA = np.array([r["comp4_A"] for r in rows])
    cS = np.array([r["comp4_S"] for r in rows])
    pA = np.array([r["phi_A"] for r in rows]); pS = np.array([r["phi_S"] for r in rows])
    tA = ee["berger_crossing_A"]["t"]; ts = ee["berger_crossing_S"]["t"]
    fig, (a1, a2) = plt.subplots(1, 2, figsize=(TEXTW, 2.45), constrained_layout=True)
    for ax in (a1, a2):
        for tv, lab in ((ts, r"$t^*$"), (tA, r"$t_A$")):
            ax.axvline(tv, color=INK2, lw=0.6, ls=(0, (2, 2)))
        grid(ax)
    a1.plot(t, lam, color=HELIKE, label=r"Helike, $\lambda_t(4)$")
    a1.plot(t, cA, color=FAM_COL["B40"], label=r"lowest competitor, $H+GG^*$ ($B_{4,0}$)")
    a1.plot(t, cS, color=FAM_COL["B40"], ls=(0, (4, 1.6)), label="lowest competitor, gauge slice")
    a1.legend(loc="lower left", fontsize=6.3, handlelength=1.8, borderaxespad=0.3, frameon=True, framealpha=0.95,
              edgecolor="none", facecolor=SURF)
    a1.set_xlabel("stretch $t$ (Berger path)"); a1.set_ylabel("eigenvalue in sector $j=4$")
    a1.set_title("Her eigenvalue meets the lowest competitor", loc="left")
    yl = a1.get_ylim()
    a1.text(ts, yl[1], r"$t^*$ ", ha="right", va="top", fontsize=7)
    a1.text(tA, yl[1], r" $t_A$", ha="left", va="top", fontsize=7)
    panel_tag(a1, "a")
    a2.axhline(0, color=INK2, lw=0.6)
    a2.plot(t, pA, color=FAM_COL["B40"], label=r"$\phi_A$ ($H+GG^*$)")
    a2.plot(t, pS, color=FAM_COL["B40"], ls=(0, (4, 1.6)), label=r"$\phi_S$ (gauge slice)")
    a2.legend(loc="upper left", fontsize=6.5, handlelength=1.8, borderaxespad=0.3, frameon=True, framealpha=0.95,
              edgecolor="none", facecolor=SURF)
    a2.plot([ts, tA], [0, 0], ls="none", marker="o", ms=4, color=INK, mec=SURF, mew=0.8)
    a2.set_ylim(top=1.2); yl = a2.get_ylim()
    a2.text(ts, yl[1], r"$t^*$ ", ha="right", va="top", fontsize=7)
    a2.text(tA, yl[1], r" $t_A$", ha="left", va="top", fontsize=7)
    a2.set_xlabel("stretch $t$ (Berger path)"); a2.set_ylabel(r"margin over $j\leq12$: competitor $-$ Helike")
    a2.set_title("The two edges are crossed at $t^*$ and $t_A$", loc="left")
    panel_tag(a2, "b")
    for ax in (a1, a2):
        ax.set_xlim(2.55, 2.85)
    save(fig, "fig_berger")


# ----------------------------------------------------------------------------------------------- figure 4
def fig_prereg(A, B):
    fig, (a1, a2, a3) = plt.subplots(1, 3, figsize=(TEXTW, 2.75), constrained_layout=True,
                                     gridspec_kw={"width_ratios": [1, 1.1, 1]})
    pts = [p for p in A["points"] if p["class"] != "small_y"]
    fails = {(round(f[0], 9), round(f[1], 9)) for f in A["A1_failures"]}
    ok = [p for p in pts if (round(p["t"], 9), round(p["y"], 9)) not in fails]
    bad = [p for p in pts if (round(p["t"], 9), round(p["y"], 9)) in fails]
    lim = (-0.78, 0.32)
    a1.plot(lim, lim, color=INK2, lw=0.6)
    a1.plot([p["x_pred"] for p in ok], [p["x_meas"] for p in ok], "o", ms=4, color=FAM_COL["B40"], mec=SURF,
            mew=0.6, label=f"$B_{{4,0}}$: root of $P_2$ ({len(ok)})")
    bad = sorted(bad, key=lambda p: p["x_pred"]); groups = []
    for p in bad:
        a1.plot([p["x_pred"]], [p["x_meas"]], "D", ms=4.5, color=FAM_COL[family(p["competitor"])], mec=SURF, mew=0.6)
        if groups and abs(p["x_pred"] - groups[-1][-1]["x_pred"]) < 0.05 and p["competitor"] == groups[-1][-1]["competitor"]:
            groups[-1].append(p)
        else:
            groups.append([p])
    for g in groups:
        ts_ = ", ".join(f"{p['t']:.3g}" for p in sorted(g, key=lambda p: p["t"]))
        hi = g[-1]["x_pred"] > 0
        a1.annotate(bname(g[0]["competitor"], signed=True) + f", $t={ts_}$", (g[-1]["x_pred"], g[-1]["x_meas"]),
                    xytext=(4, -7) if hi else (9, 0), textcoords="offset points", fontsize=6,
                    va="top" if hi else "center", bbox=dict(boxstyle="square,pad=0.1", fc=SURF, ec="none", alpha=0.9), zorder=5)
    a1.plot([], [], "D", ms=4.5, color=FAM_COL["D+"], label=f"another block first ({len(bad)})")
    a1.set_xlim(lim); a1.set_ylim(lim); a1.set_aspect("equal")
    a1.set_xlabel(r"predicted edge (root of $P_2$)"); a1.set_ylabel(r"measured edge $x^*_{12}$")
    a1.set_title(f"Test A: edge at {len(pts)} points", loc="left")
    a1.legend(loc="upper left", bbox_to_anchor=(-0.05, -0.3), handletextpad=0.3, borderaxespad=0.2)
    grid(a1); panel_tag(a1, "a")

    sc = [p for p in B["points"] if p.get("x_star") is not None and p.get("rho_s") is not None]
    for p in sc:
        a2.plot([p["t"], p["t"]], [p["x_lo"], p["x_star"]], color=HELIKE, lw=0.6, alpha=0.55)
    neg = [p for p in sc if p["x_star"] <= 0]; pos = [p for p in sc if p["x_star"] > 0]
    a2.plot([p["t"] for p in neg], [p["x_star"] for p in neg], "o", ms=2.6, color=HELIKE, mec="none")
    a2.plot([p["t"] for p in pos], [p["x_star"] for p in pos], "o", ms=2.6, mfc=SURF, mec=HELIKE, mew=0.6)
    a2.plot([p["t"] for p in sc], [p["x_lo"] for p in sc], "_", ms=3, color=HELIKE, mew=0.7)
    a2.axhline(0, color=INK2, lw=0.6)
    a2.set_xlabel("stretch $t$"); a2.set_ylabel("$x$")
    a2.set_title(f"Test B: $[x_{{\\mathrm{{lo}}}},x^*_{{12}}]$, {len(sc)} fields", loc="left")
    a2.text(15.3, -9.6, f"open dot: $x^*_{{12}}>0$ ({len(pos)}),\na cut-off artefact for $H+GG^*$", fontsize=6,
            ha="right", va="bottom", bbox=dict(fc=SURF, ec="none", pad=1.0))
    a2.set_ylim(-10, 0.9)
    a2.set_xlim(0.5, 15.5)
    grid(a2); panel_tag(a2, "b")

    neutral = [p["rho_s"] for p in sc if p["competitor"][1] == 0]
    charged = [p["rho_s"] for p in sc if p["competitor"][1] != 0]
    rng = np.random.default_rng(7)
    for k, (vals, col, lab) in enumerate(((charged, INK2, "charged"), (neutral, FAM_COL["B40"], "neutral"))):
        yy = k + rng.uniform(-0.22, 0.22, len(vals))
        a3.plot(vals, yy, "o", ms=2.8, color=col, mec="none", alpha=0.8)
        nb = sum(v >= 0.5 for v in vals)
        a3.text(0.02, k + 0.27, f"{lab}: {nb} of {len(vals)} at $\\geq0.5$", fontsize=6.3, ha="left", va="bottom",
                bbox=dict(fc=SURF, ec="none", pad=0.8))
    a3.axvline(0.5, color=INK, lw=0.8)
    a3.text(0.51, 1.75, "registered threshold", fontsize=6.3, ha="left", va="top")
    a3.set_yticks([0, 1]); a3.set_yticklabels(["charged\n$q\\neq0$", "neutral\n$q=0$"])
    a3.set_ylim(-0.35, 1.8); a3.set_xlim(0, 1.0)
    a3.set_xlabel(r"slope ratio $\rho_s=|\partial_x\lambda_{\mathrm{comp}}|/|\partial_x\lambda|$")
    a3.set_title(f"Test B-S: {sum(v >= 0.5 for v in neutral + charged)} of {len(sc)} fail", loc="left")
    grid(a3, axis="x"); panel_tag(a3, "c")
    save(fig, "fig_prereg")


# ----------------------------------------------------------------------------------------------- figure 5
SU3_ORDER = ["(0, 0)", "(1, 0)", "(1, 1)", "(2, 0)", "(2, 1)", "(3, 0)", "(2, 2)", "(3, 1)", "(4, 0)"]


def fig_su3(su3, su2):
    fig, (a1, a2) = plt.subplots(1, 2, figsize=(TEXTW, 2.55), sharey=True, constrained_layout=True,
                                 gridspec_kw={"width_ratios": [1.25, 1]})
    ramp = [HELIKE_RAMP[0], HELIKE_RAMP[1], HELIKE_RAMP[3], HELIKE_RAMP[4]]
    for ax in (a1, a2):
        ax.axhline(0, color=INK2, lw=0.6); grid(ax)
    js = np.arange(13)
    for r, col in zip(su2["rows"], ramp):
        a1.plot(js, r["margins"], color=col, marker="o", ms=2.6, mec="none", label=f"$y={r['y']:g}$")
    a1.set_xticks(range(0, 13, 2)); a1.set_xlim(-0.4, 12.4)
    a1.legend(loc="upper left", title="cross field", title_fontsize=6.5)
    a1.set_xlabel("sector $j$"); a1.set_ylabel(r"margin: lowest competitor $-$ Helike")
    a1.set_title("$SU(2)$ family, $t=4$: the margin climbs for her", loc="left")
    panel_tag(a1, "a")
    xs = np.arange(len(SU3_ORDER))
    for r, col in zip(su3, ramp):
        m = [r["best"]["margins"][k] for k in SU3_ORDER]
        a2.plot(xs, m, color=col, marker="o", ms=2.6, mec="none", label=f"$({r['u']:g},\\,{r['y']:g})$")
    a2.set_xticks(xs); a2.set_xticklabels([s.replace(" ", "") for s in SU3_ORDER], rotation=45, fontsize=6.3)
    a2.set_xlim(-0.4, len(xs) - 0.6)
    a2.legend(loc="upper right", title="shape $(u,y)$", title_fontsize=6.5)
    a2.set_xlabel("sector $(p,q)$, by Casimir")
    a2.set_title("$SU(3)$ free fibre, $t=4$: it climbs against her", loc="left")
    panel_tag(a2, "b")
    save(fig, "fig_su3")


def main():
    CACHE.mkdir(parents=True, exist_ok=True); OUT.mkdir(parents=True, exist_ok=True)
    rc = "--recompute" in sys.argv
    reg = load_or("region_j12.json", compute_region, rc)
    bp = load_or("berger_path.json", compute_berger, rc)
    su2 = load_or("su2_margins_t4.json", compute_su2_margins, rc)
    ee = json.loads((R1119 / "exact_edge.json").read_text())
    A = json.loads((R1121 / "wp4_heldout.json").read_text())
    B = json.loads((R1121 / "wp4_general_fields.json").read_text())
    su3 = json.loads((R1121 / "wp4_su3_diagnostic_posthoc.json").read_text())
    fig_helike(); fig_region(reg); fig_berger(bp, ee); fig_prereg(A, B); fig_su3(su3, su2)


if __name__ == "__main__":
    main()
