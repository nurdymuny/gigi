"""Reproducible figures and typeset tables from the verified Helike survey."""

import argparse
import csv
import hashlib
import json
import math
from pathlib import Path
import subprocess

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import matplotlib.patches
import matplotlib.transforms
from matplotlib import font_manager
from matplotlib.colors import LinearSegmentedColormap, Normalize
import numpy as np

ROOT = Path(__file__).resolve().parents[1]
RUN_ID = "20261124T021903Z-c326a3c5"
RUN = ROOT / "research/helike-quantum-probe-20261123" / ("survey-" + RUN_ID)
OUT = ROOT / "docs/papers/helike-quantum-probe"
TEAL, GOLD, RED, INK, GRAY = "#176F78", "#A16623", "#AC4756", "#25343D", "#75848A"
SECTOR_RUN = ROOT / "research/sector-review-20261124/controls-abd81f97"


def make_sector_payload(receipt, verification):
    """Validated publication view of the registered two-sector controls."""
    if (receipt.get("status") != "COMPLETE" or receipt.get("mode") != "J0_J2" or
        receipt.get("scientific_status") != "UNRESOLVED" or
        verification.get("status") != "PASS" or verification.get("cases") != 16 or
        verification.get("certified") is not False or
        verification.get("separation_verified") is not False or
        verification.get("scientific_status") != "UNRESOLVED"):
        raise ValueError("sector completion or scientific scope differs")
    expected = [(g, a, n) for g, a in ((1., 1.7), (2., 2.3)) for n in range(10, 25, 2)]
    if [(c["g2"], c["alpha"], c["cutoff"]) for c in receipt["cases"]] != expected:
        raise ValueError("registered sector cases differ")
    rows = []
    for case in receipt["cases"]:
        m = case["measurements"]
        s, t = m["scalar_sector"], m["tensor_sector"]
        es, et, rs, rt = (np.array(x) for x in (s["energies"], t["energies"], s["residuals"], t["residuals"]))
        if any(x.shape != (6,) or not np.isfinite(x).all() for x in (es, et, rs, rt)) or min(*rs, *rt) < 0:
            raise ValueError("sector energies and residuals must be finite")
        gs, gt = es[1]-es[0], et[0]-es[0]
        allowance = .01 * max(1., min(gs, gt))
        for key, value in (("E0", es[0]), ("eps0", rs[0]), ("gap_scalar", gs),
                           ("gap_tensor", gt), ("D", et[0]-es[1]), ("allowance", allowance)):
            if not math.isclose(m[key], value, rel_tol=0, abs_tol=1e-13):
                raise ValueError("sector arithmetic differs: " + key)
        residuals = dict(vacuum=float(rs[0]), scalar=float(rs[1]), tensor=float(rt[0]))
        gates = {k: v <= allowance for k, v in residuals.items()}
        strict = {k: v <= .01 for k, v in residuals.items()}
        if m["residual_gate"] != gates or m["conservative_residual_gate"] != strict:
            raise ValueError("sector residual gate differs")
        if (m["allowance_scope"] != "J0_J2_ONLY" or m["global_gap_verified"] is not False or
            m["ordering"]["unconditional_order"] != "ORDER_UNRESOLVED" or
            m["ordering"]["separation_verified"] is not False or m["ordering"]["certified"] is not False):
            raise ValueError("unverified sector conclusion promoted")
        n = case["cutoff"]
        count = lambda k: sum(1 for a in range(k//2+1) for b in range(k//4+1)
                             for c in range(k//3+1) if 2*a+4*b+3*c <= k)
        ns, nt = count(n), count(n-2)+count(n-4)
        if (s["dimension"], t["dimension"]) != (ns, nt):
            raise ValueError("sector dimension differs from the invariant construction")
        if n == 24 and (set(case["precision"]) != {"0", "2"} or
            any(p["passed"] is not True or p["dps"] != 100 or
                not np.isfinite([p["max_abs_dE"], p["max_rel_dR"]]).all() or
                p["max_abs_dE"] > 1e-11 or p["max_rel_dR"] > 1e-8
                for p in case["precision"].values())):
            raise ValueError("top-cutoff precision recheck missing or failed")
        rows.append(dict(g2=case["g2"], alpha=case["alpha"], N=n, E0=float(es[0]),
            scalar_energy=float(es[1]), tensor_energy=float(et[0]), scalar_gap=float(gs), tensor_gap=float(gt),
            vacuum_residual=float(rs[0]), scalar_residual=float(rs[1]), tensor_residual=float(rt[0]),
            next_scalar_residual=float(rs[2]), next_tensor_residual=float(rt[1]), allowance=float(allowance),
            max_ratio=max(residuals.values())/allowance, joint_pass=all(gates.values()), strict_pass=all(strict.values()),
            D=float(et[0]-es[1]), scalar_dimension=ns, tensor_multiplets=nt,
            all_spin_dimension=math.comb(n//2+6, 6)+math.comb((n-3)//2+6, 6)))
    return dict(source_revision=receipt["source_revision"], scientific_status="UNRESOLVED", scope="J0_J2_ONLY",
        cases=rows, final=[r for r in rows if r["N"] == 24],
        first_joint_pass={g: next(r["N"] for r in rows if r["g2"] == g and r["joint_pass"]) for g in (1., 2.)},
        first_strict_pass={g: next(r["N"] for r in rows if r["g2"] == g and r["strict_pass"]) for g in (1., 2.)})


def load_sector_evidence():
    receipt = json.loads((SECTOR_RUN / "receipt.json").read_text())
    verification = json.loads((SECTOR_RUN / "verification.json").read_text())
    payload = make_sector_payload(receipt, verification)
    for name, expected in receipt["source_hashes"].items():
        if digest(SECTOR_RUN / "source" / name) != expected:
            raise ValueError("sector source snapshot changed: " + name)
    for case in receipt["cases"]:
        for group in ("arrays", "high_arrays"):
            for spin, entry in case[group].items():
                path = SECTOR_RUN / entry["file"]
                if digest(path) != entry["sha256"]:
                    raise ValueError("sector matrix hash changed")
                with np.load(path, allow_pickle=False) as z:
                    expected = case["measurements"]["scalar_sector" if spin == "0" else "tensor_sector"]
                    if group == "arrays" and (not np.array_equal(z["E"], expected["energies"]) or
                                               not np.array_equal(z["R"], expected["residuals"])):
                        raise ValueError("sector receipt differs from saved array")
    return receipt, payload


def sector_figures(payload):
    fig, axes = plt.subplots(1, 2, figsize=(7.1, 3.0), layout="constrained", sharey=True)
    for ax, g in zip(axes, (1., 2.)):
        rows = [r for r in payload["cases"] if r["g2"] == g]
        for key, color, marker, label in (("vacuum_residual", GRAY, ".", "Vacuum"),
                ("scalar_residual", GOLD, "s", "Scalar"), ("tensor_residual", TEAL, "o", "Tensor")):
            ax.semilogy([r["N"] for r in rows], [r[key] for r in rows], color=color, marker=marker,
                        markersize=4, label=label)
        ax.plot([r["N"] for r in rows], [r["allowance"] for r in rows], "--", color=INK, lw=1, label="Shared allowance")
        ax.axhline(.01, color=GRAY, lw=1, linestyle=":", label="Stricter 0.01 check")
        ax.set_title(rf"$g^2={g:g}$"); ax.set_xlabel("Polynomial cutoff N")
        ax.set_xticks(range(10, 25, 2)); ax.grid(axis="y", alpha=.18)
    axes[0].set_ylabel(r"Full-operator residual $\|(H-E)\psi\|$")
    handles, labels = axes[0].get_legend_handles_labels()
    fig.legend(handles, labels, loc="outside upper center", ncol=3, frameon=False, fontsize=9)
    save(fig, "06-sector-residuals")

    rows = [r for r in payload["cases"] if r["g2"] == 1.]
    fig, ax = plt.subplots(figsize=(7.1, 2.5), layout="constrained")
    for key, color, marker, label in (("all_spin_dimension", GRAY, "^", "All spatial spins (count only)"),
            ("scalar_dimension", GOLD, "s", "Scalar basis functions"),
            ("tensor_multiplets", TEAL, "o", "Tensor multiplets (5 states each)")):
        values = [r[key] for r in rows]
        ax.semilogy([r["N"] for r in rows], values, color=color, marker=marker, markersize=4, label=label)
        ax.annotate(f"{values[-1]:,}", (24, values[-1]), xytext=(6, 0), textcoords="offset points", color=color, va="center")
    ax.set_xlim(9.6, 26); ax.set_xticks(range(10, 25, 2)); ax.set_xlabel("Polynomial cutoff N")
    ax.set_ylabel("Basis size / multiplicities (log scale)"); ax.legend(frameon=False, fontsize=9, loc="upper left")
    ax.grid(axis="y", alpha=.18); save(fig, "07-sector-dimensions")

    fig, axes = plt.subplots(1, 2, figsize=(7.1, 2.7), layout="constrained")
    for ax, g in zip(axes, (1., 2.)):
        rows = [r for r in payload["cases"] if r["g2"] == g]
        for key, color, marker, label in (("scalar_gap", GOLD, "s", "Scalar"), ("tensor_gap", TEAL, "o", "Tensor")):
            ax.plot([r["N"] for r in rows], [r[key] for r in rows], color=color, marker=marker, markersize=4, label=label)
        ax.set_title(rf"$g^2={g:g}$"); ax.set_xlabel("Polynomial cutoff N")
        ax.set_xticks(range(10, 25, 2)); ax.ticklabel_format(axis="y", useOffset=False)
        ax.grid(axis="y", alpha=.18); ax.legend(frameon=False, fontsize=9)
    axes[0].set_ylabel(r"Sector excitation estimate $(E_J-E_0)R$")
    save(fig, "08-sector-energies")


def write_sector_tables(payload):
    generated = OUT / "generated"
    header = r"\begin{tabular}{@{}rrrrr@{}}\toprule" + "\n"
    header += r"$N$ & $\epsilon_s$, $g^2=1$ & $\epsilon_t$, $g^2=1$ & $\epsilon_s$, $g^2=2$ & $\epsilon_t$, $g^2=2$\\\midrule" + "\n"
    lines = []
    for n in range(10, 25, 2):
        rows = [r for r in payload["cases"] if r["N"] == n]
        values = [r[k] for r in rows for k in ("scalar_residual", "tensor_residual")]
        lines.append(str(n) + " & " + " & ".join(f"{v:.6f}" for v in values) + r"\\")
    (generated/"sector-residuals.tex").write_text(header+"\n".join(lines)+"\n"+r"\bottomrule\end{tabular}"+"\n", encoding="ascii")
    header = r"\begin{tabular}{@{}rrrrrr@{}}\toprule" + "\n"
    header += r"$g^2$ & $E_0$ & $E_s$ & $E_t$ & $E_s-E_0$ & $E_t-E_0$\\\midrule" + "\n"
    lines = [f"{r['g2']:g} & " + " & ".join(f"{r[k]:.9f}" for k in
             ("E0", "scalar_energy", "tensor_energy", "scalar_gap", "tensor_gap")) + r"\\" for r in payload["final"]]
    (generated/"sector-levels.tex").write_text(header+"\n".join(lines)+"\n"+r"\bottomrule\end{tabular}"+"\n", encoding="ascii")
    with (OUT/"sector-summary.csv").open("w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=list(payload["cases"][0])); writer.writeheader(); writer.writerows(payload["cases"])
    (OUT/"sector-plot-data.json").write_text(json.dumps(payload, indent=2, allow_nan=False)+"\n", encoding="utf-8")


def quadratures():
    bc = np.diag([0., 1., -1.])/np.sqrt(2)
    bs = np.array([[0., 0., 0.], [0., 0., 1.], [0., 1., 0.]])/np.sqrt(2)
    return bc, bs


def make_payload(receipt, verification):
    if receipt["status"] != "UNRESOLVED":
        raise ValueError("this report describes the registered UNRESOLVED survey only")
    if (verification.get("integrity") != "VERIFIED" or
        verification.get("run_id") != receipt["run_id"] or
        verification.get("survey_status") != receipt["status"] or
        verification.get("scientific_acceptance") is not False or
        verification.get("replayed_cases") != 15):
        raise ValueError("verification does not identify this unresolved 15-case survey")
    if [c["g2"] for c in receipt["couplings"]] != [1., .5, 2.]:
        raise ValueError("registered coupling set differs")
    rows, final, levels = [], [], []
    for coupling in receipt["couplings"]:
        if [c["cutoff"] for c in coupling["cases"]] != [4, 6, 8, 10, 12]:
            raise ValueError("cutoff history is incomplete")
        for case in coupling["cases"]:
            numeric = [case[k] for k in ("E0", "E1", "absolute_gap")]
            numeric += [case[ch][k] for ch in ("H2", "H0") for k in ("lowest_weight", "accessible_gap")]
            if not np.isfinite(numeric).all():
                raise ValueError("energy and weight claims must be finite")
            if abs(case["E1"] - case["E0"] - case["absolute_gap"]) > 1e-10:
                raise ValueError("declared gap is not the energy difference")
            for channel in ("H2", "H0"):
                weights = [l["weight"] for l in case[channel]["levels"]]
                if (not np.isfinite(weights).all() or min(weights) < 0 or
                    abs(sum(weights)-1) > 1e-10 or
                    abs(weights[0]-case[channel]["lowest_weight"]) > 1e-12):
                    raise ValueError("level weights do not preserve the normalized response")
                for index, level in enumerate(case[channel]["levels"]):
                    levels.append(dict(g2=coupling["g2"], N=case["cutoff"], channel=channel,
                                       level=index+1, gap=level["gap"], weight=level["weight"],
                                       multiplicity=level["multiplicity"]))
            casimirs = np.array(case["low_level_spin_casimirs"][1])
            spin = int(round((np.sqrt(1+4*float(casimirs.mean()))-1)/2))
            if not np.allclose(casimirs, spin*(spin+1), atol=1e-8, rtol=0):
                raise ValueError("lowest level has unresolved spin content")
            ext = case["exterior"]
            row = dict(g2=coupling["g2"], N=case["cutoff"], dimension=case["dimension"],
                       E0=case["E0"], E1=case["E1"], gap=case["absolute_gap"], lowest_spin=spin,
                       residual=max(ext["level_worst_residuals"]), allowed_residual=ext["allowed_residual"],
                       residual_ratio=max(ext["level_worst_residuals"])/ext["allowed_residual"])
            for ch in ("H2", "H0"):
                row[ch+"_gap"] = case[ch]["accessible_gap"]
                row[ch+"_weight"] = case[ch]["lowest_weight"]
                row[ch+"_tail"] = ext["probe_discarded_fractions"][ch]
                row[ch+"_tail_ratio"] = row[ch+"_tail"]/.001
            rows.append(row)
        final.append(rows[-1])
    return {"run_id": receipt["run_id"], "status": receipt["status"],
            "units": "energies in 1/R; R=1 and hbar=1; weights dimensionless",
            "cases": rows, "final": final, "levels": levels}


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load_evidence():
    r = json.loads((RUN / "receipt.json").read_text())
    v = json.loads((RUN / "verification.json").read_text())
    if r["run_id"] != RUN_ID:
        raise ValueError("wrong run")
    for name, expected in r["source_hashes"].items():
        if digest(RUN / "source" / name) != expected:
            raise ValueError("source snapshot changed")
    for coupling in r["couplings"]:
        for case in coupling["cases"]:
            if digest(RUN / case["arrays"]) != case["arrays_sha256"]:
                raise ValueError("stored matrix hash changed")
    return r, make_payload(r, v)


def style():
    for face in ("regular", "italic", "bold", "bolditalic"):
        result = subprocess.run(["kpsewhich", f"texgyrepagella-{face}.otf"], capture_output=True, text=True)
        path = Path(result.stdout.strip())
        if not path.is_file():
            raise RuntimeError("TeX Gyre Pagella fonts are required to match the paper typography")
        font_manager.fontManager.addfont(str(path))
    plt.rcParams.update({"font.family": "TeX Gyre Pagella", "font.size": 11,
        "mathtext.fontset": "custom", "mathtext.rm": "TeX Gyre Pagella",
        "mathtext.it": "TeX Gyre Pagella:italic", "mathtext.bf": "TeX Gyre Pagella:bold",
        "mathtext.cal": "TeX Gyre Pagella:italic", "mathtext.fallback": "stix",
        "axes.labelcolor": INK, "text.color": INK,
        "xtick.color": INK, "ytick.color": INK, "axes.edgecolor": "#BAC5C8",
        "axes.spines.top": False, "axes.spines.right": False,
        "axes.titlelocation": "left", "axes.titlesize": 13, "axes.titlepad": 12,
        "figure.facecolor": "white", "axes.facecolor": "white",
        "savefig.facecolor": "white", "pdf.fonttype": 3, "ps.fonttype": 3,
        "svg.fonttype": "none", "svg.hashsalt": "helike-quantum-report-v1"})


def save(fig, name):
    folder = OUT / "figures"
    folder.mkdir(parents=True, exist_ok=True)
    fig.savefig(folder/(name+".pdf"), bbox_inches="tight", dpi=300, metadata={"Author": "", "Creator": "", "CreationDate": None})
    fig.savefig(folder/(name+".svg"), bbox_inches="tight", dpi=300, metadata={"Creator": "", "Date": None})
    fig.savefig(folder/(name+".png"), bbox_inches="tight", dpi=240)
    plt.close(fig)


def response_figure(payload):
    fig, ax = plt.subplots(figsize=(7.1, 3.0), layout="constrained")
    x = np.arange(3)
    for ch, color, offset, label in (("H2", TEAL, -.19, "H2: directional probe"),
                                     ("H0", GOLD, .19, "H0: scalar composite")):
        values = [100*r[ch+"_weight"] for r in payload["final"]]
        ax.bar(x+offset, values, .32, color=color, label=label, zorder=3)
        for location, value in zip(x+offset, values):
            text = f"{value:.2f}%" if value > .001 else r"$<10^{-23}\%$"
            ax.text(location, max(2, value+2), text, ha="center", va="bottom", fontsize=11, color=color)
    ax.set_xticks(x, [r"$g^2=1$"+"\nPrimary", r"$g^2=0.5$", r"$g^2=2$"])
    ax.set_ylim(0, 116)
    ax.set_yticks([0, 25, 50, 75, 100])
    ax.set_ylabel("Response in the lowest finite-basis level (%)")
    ax.grid(axis="y", color="#E7ECEE", zorder=0)
    ax.legend(loc="upper left", bbox_to_anchor=(0, 1.22), ncol=2, frameon=False)
    save(fig, "01-lowest-level-response")


def geometry_figure():
    cmap = LinearSegmentedColormap.from_list("signed", [GOLD, "#F3F5F5", TEAL])
    phi, theta = np.meshgrid(np.linspace(0, 2*np.pi, 121), np.linspace(0, np.pi, 81))
    xyz = np.array([np.sin(theta)*np.cos(phi), np.sin(theta)*np.sin(phi), np.cos(theta)])
    norm = Normalize(-1/np.sqrt(2), 1/np.sqrt(2))
    fig = plt.figure(figsize=(7.1, 2.65), layout="constrained")
    axes = []
    for index, (b, title) in enumerate(zip(quadratures(),
            [r"A  $B_c$: $(n_1^2-n_2^2)/\sqrt{2}$", r"B  $B_s$: $\sqrt{2}n_1 n_2$"])):
        ax = fig.add_subplot(1, 2, index+1, projection="3d")
        values = np.einsum("iab,ij,jab->ab", xyz, b, xyz)
        ax.plot_surface(*xyz, facecolors=cmap(norm(values)), rcount=81, ccount=121,
                        linewidth=0, antialiased=False, shade=False, rasterized=True)
        for k in range(3):
            line = np.zeros((3, 2)); line[k, 1] = 1.35
            ax.plot(*line, color=GRAY, linewidth=.8)
            label = np.zeros(3); label[k] = 1.5
            ax.text(*label, f"$n_{k}$", fontsize=11)
        ax.view_init(elev=23, azim=32)
        ax.set_box_aspect((1, 1, 1))
        ax.set_xlim(-1.2, 1.4); ax.set_ylim(-1.2, 1.4); ax.set_zlim(-1.2, 1.4)
        ax.set_axis_off()
        ax.set_title(title, pad=0, fontsize=12)
        axes.append(ax)
    colorbar = fig.colorbar(matplotlib.cm.ScalarMappable(norm=norm, cmap=cmap), ax=axes,
                            location="bottom", shrink=.65, aspect=40, pad=.01)
    colorbar.set_ticks([-1/np.sqrt(2), 0, 1/np.sqrt(2)], labels=[r"$-1/\sqrt{2}$", "0", r"$+1/\sqrt{2}$"])
    colorbar.set_label(r"Signed quadratic form $n^T B n$ on the unit sphere", labelpad=3)
    save(fig, "02-probe-geometry")


def spectrum_figure(payload):
    fig, axes = plt.subplots(2, 3, figsize=(7.1, 4.4), sharex=True, sharey=True, layout="constrained")
    for col, row in enumerate(payload["final"]):
        for j, (ch, color) in enumerate((("H2", TEAL), ("H0", GOLD))):
            ax = axes[j, col]
            levels = [l for l in payload["levels"] if l["N"] == 12 and l["g2"] == row["g2"] and l["channel"] == ch and l["gap"] <= 10]
            gaps = [l["gap"] for l in levels]; weights = [l["weight"] for l in levels]
            ax.axvline(row["gap"], color=GRAY, linestyle="--", linewidth=1)
            ax.vlines(gaps, 0, weights, color=color, linewidth=1.4)
            ax.scatter(gaps, weights, color=color, s=14, zorder=3)
            if j == 0:
                ax.set_title(rf"$g^2={row['g2']:g}$" + ("  (primary)" if col == 0 else ""))
            if col == 0:
                ax.set_ylabel(f"{ch} response weight")
            if j == 1:
                ax.set_xlabel(r"Excitation energy $(E-E_0)R$")
            ax.set_xlim(0, 10); ax.set_ylim(-.025, 1.10)
            ax.set_yticks([0, .5, 1]); ax.set_xticks([0, 2, 4, 6, 8, 10])
            ax.grid(axis="y", color="#E7ECEE")
    fig.suptitle("Complete-level weights, N = 12  |  dashed line: absolute finite-basis gap", fontsize=12)
    save(fig, "03-spectral-response")


def cutoff_figure(payload):
    fig, axes = plt.subplots(1, 2, figsize=(7.1, 3.25), layout="constrained")
    for g2, color, marker in ((1., INK, "o"), (.5, GOLD, "s"), (2., TEAL, "D")):
        rows = [r for r in payload["cases"] if r["g2"] == g2]
        axes[0].plot([r["N"] for r in rows], [r["gap"] for r in rows], color=color,
                     marker=marker, markersize=4, label=rf"$g^2={g2:g}$")
    axes[0].set_title("A  Absolute gap at every cutoff")
    axes[0].set_ylabel(r"$(E_1-E_0)R$"); axes[0].legend(frameon=False)
    rows = [r for r in payload["cases"] if r["g2"] == 2]
    for ch, color, marker in (("H2", TEAL, "D"), ("H0", GOLD, "s")):
        axes[1].plot([r["N"] for r in rows], [r[ch+"_gap"] for r in rows], color=color,
                     marker=marker, markersize=4, label=ch+" accessible level")
    axes[1].set_title(r"B  Competing channels at $g^2=2$")
    axes[1].set_ylabel(r"Accessible excitation energy $\Delta R$")
    axes[1].legend(frameon=False, fontsize=10)
    axes[1].annotate("Scalar lower at N = 8", xy=(8, rows[2]["H0_gap"]), xytext=(5.4, 4.7),
                     arrowprops=dict(arrowstyle="->", color=GOLD, lw=.9), fontsize=10, color=GOLD)
    for ax in axes:
        ax.set_xlabel("Polynomial cutoff N"); ax.set_xticks([4, 6, 8, 10, 12])
        ax.grid(axis="y", color="#E7ECEE")
    save(fig, "04-cutoff-history")


def gate_figure(payload):
    fig, axes = plt.subplots(1, 2, figsize=(7.1, 3.15), layout="constrained")
    x = np.arange(3)
    values = [r["residual_ratio"] for r in payload["final"]]
    axes[0].bar(x, values, .5, color=RED, zorder=3)
    for i, value in enumerate(values):
        axes[0].text(i, value*1.15, f"{value:.1f}", ha="center", fontsize=11, color=RED)
    axes[0].set_title("A  Full residual / permitted residual")
    for ch, color, offset in (("H2", TEAL, -.18), ("H0", GOLD, .18)):
        values = [r[ch+"_tail_ratio"] for r in payload["final"]]
        axes[1].bar(x+offset, values, .30, color=color, label=ch, zorder=3)
        for i, value in enumerate(values):
            axes[1].text(i+offset, value*1.12, f"{value:.2f}", ha="center", fontsize=10, color=color,
                         bbox=dict(facecolor="white", edgecolor="none", pad=.6))
    axes[1].set_title("B  Discarded response / permitted tail")
    axes[1].legend(frameon=False, loc="upper left", ncol=2)
    for ax in axes:
        ax.axhspan(.1, 1, color=TEAL, alpha=.06)
        ax.axhline(1, color=INK, linewidth=1, linestyle="--")
        ax.set_yscale("log"); ax.set_ylim(.1, 120)
        ax.set_xticks(x, [r"$g^2=1$", r"$g^2=0.5$", r"$g^2=2$"])
        ax.set_ylabel("Ratio to the fixed acceptance limit")
        ax.grid(axis="y", color="#E7ECEE", which="major", zorder=0)
    save(fig, "05-convergence-gates")


def write_tables(payload):
    generated = OUT / "generated"
    generated.mkdir(exist_ok=True)
    lines = []
    for row in payload["final"]:
        lines.append(f"{row['g2']:g} & {row['E0']:.9f} & {row['gap']:.9f} & {row['lowest_spin']} & {row['H2_gap']:.9f} & {row['H0_gap']:.9f} " + r"\\")
    header = (r"\begin{tabular}{@{}rrrrrr@{}}\toprule" "\n"
              r"$g^2$ & $E_0R$ & $\Delta_{\rm abs}R$ & $J_{\rm lowest}$ & $\Delta_{\Htwo}R$ & $\Delta_{\Hzero}R$\\\midrule" "\n")
    (generated/"final-levels.tex").write_text(header+"\n".join(lines)+"\n"+r"\bottomrule\end{tabular}"+"\n", encoding="ascii")
    lines = []
    for n in (4, 6, 8, 10, 12):
        rows = [r for r in payload["cases"] if r["N"] == n]
        lines.append(f"{n} & {rows[0]['dimension']:,} & " + " & ".join(f"{r['gap']:.9f}" for r in rows) + f" & {rows[-1]['lowest_spin']} " + r"\\")
    header = (r"\begin{tabular}{@{}rrrrrr@{}}\toprule" "\n"
              r"$N$ & Dimension & $\Delta R$, $g^2=1$ & $\Delta R$, $g^2=0.5$ & $\Delta R$, $g^2=2$ & $J$, $g^2=2$\\\midrule" "\n")
    (generated/"cutoffs.tex").write_text(header+"\n".join(lines)+"\n"+r"\bottomrule\end{tabular}"+"\n", encoding="ascii")
    for name, rows in (("case-summary", payload["cases"]), ("level-weights", payload["levels"])):
        with (OUT/(name+".csv")).open("w", newline="", encoding="utf-8") as f:
            writer = csv.DictWriter(f, fieldnames=list(rows[0])); writer.writeheader(); writer.writerows(rows)
    (OUT/"plot-data.json").write_text(json.dumps(payload, indent=2, allow_nan=False)+"\n", encoding="utf-8")


# ---------------------------------------------------------------------------------------------------------------
# Flux-background section (25 November 2026): four hash-pinned receipts, refused on any promoted or forged claim.

FLUX_RECEIPTS = {
    "convergence": ("research/cone-convergence-20261124/controls-59534df9/receipt.json",
                    "f1eec3b8d49a9a93c95ca0cb98bbe3a344ae51e561b5d47543c438bc6abeeb74"),
    "sectors": ("research/leakage-gate-20261125/inputs/cone_sector_bounds_c31aab8e.json",
                "4ead4c2dcd51e851b8654bfed9018061cfcfbdb257edfa9ca2a611a291e557a7"),
    "leakage": ("research/leakage-gate-20261125/controls-fbfa958/receipt.json",
                "2203e230a7cda2629b996c9c21b06d984c43ee874778544896e336702f60c82f"),
    "temple": ("research/temple-20261125/cone_temple_c31aab8e.json",
               "938e9b3f9027070aa112c55968f50ac4165779aaa400bdd906c845fe90569484"),
}
FLUX_CONTROLS = [(0.004, 6), (0.004, 8), (0.002, 6), (0.002, 8)]


SC_RECEIPT = ("research/sc-status-20261126/sc_status.json",
              "a8ebb0800da0fa4b9cf78e7d9545746c7d5ba28649c28be0ce39c543662248e5")


def make_flux_payload(conv, sectors, leak, temple):
    """Rows for the flux-background pages. Refuses any receipt that promotes, forges or reverses a claim."""
    if conv.get("decision") != "CONVERGED" or conv.get("status") != "COMPLETE":
        raise ValueError("flux convergence receipt is not a completed CONVERGED run")
    if (leak.get("relative_leakage_gate") != "PASS" or leak.get("certified") is not False or
            leak.get("full_grid_released") is not False or leak.get("historical_cutoff_gate_passed_unchanged") is not False):
        raise ValueError("leakage receipt is not the registered PASS with the grid held and history unchanged")
    if sectors.get("status") != "INTERVAL_ENCLOSURE" or temple.get("status") != "INTERVAL_ENCLOSURE":
        raise ValueError("bounds must be interval enclosures")
    if temple["params"].get("assume") != "S_C":
        raise ValueError("ordering receipt must state its separation hypothesis")
    sec = {(c["g2"], c["Mq"]): {s["name"]: s for s in c["states"]} for c in sectors["results"]["controls"]}
    tem = {(c["g2"], c["Mq"]): c for c in temple["results"]["controls"]}
    lk = {(s["g2"], s["Mq"], s["state"]): s for s in leak["states"]}
    rows = []
    for key in FLUX_CONTROLS:
        g2, mq = key
        d = tem[key]["decision"]
        verdict, margin = d["verdict"], d["margin_lower"]
        if verdict not in ("ORDERED", "UNRESOLVED"):
            raise ValueError("an ordering verdict other than ORDERED or UNRESOLVED is never reported")
        if verdict == "ORDERED" and not (d["lemma3"] and margin > 0):
            raise ValueError("ORDERED requires Lemma 3 and a positive margin")
        if verdict == "UNRESOLVED" and d["lemma3"] and margin > 0:
            raise ValueError("a positive margin with Lemma 3 must be reported as ORDERED")
        gaps = conv["rows"][str(g2)][str(mq)]
        e0 = tem[key]["states"][0]["center"]
        states = []
        for name in ("ground", "helike_plus", "competitor"):
            s = sec[key][name]
            sb = s["sector"]["Ok"]
            ratio = lk[(g2, mq, name)]["leakage_over_r_lower_upper"]
            if not (s["r_lower"] <= s["r_upper"] and ratio <= 1e-3):
                raise ValueError("residual enclosure or leakage ratio inconsistent")
            states.append({"state": name, "r_lower": s["r_lower"], "r_upper": s["r_upper"], "charge_upper": s["charge_upper"],
                           "leakage_upper": s["leakage_norm_upper"], "leakage_over_r": ratio,
                           "q_lower": sb["q_lower"], "residual_bound": sb["residual_upper"],
                           "window": [s["center"] - e0 - sb["residual_upper"], s["center"] - e0 + sb["residual_upper"]]})
        rows.append({"g2": g2, "Mq": mq, "ritz_gap_helike": gaps["helike"]["gap"], "ritz_gap_competitor": gaps["competitor"]["gap"],
                     "helike_upper": max(d["helike_upper"]) - e0, "competitor_lower": d["competitor_lower"] - e0,
                     "margin": margin, "verdict": verdict, "states": states,
                     "helike_gap": d["gap_report"]["helike_gap"], "competitor_gap_lower": d["gap_report"]["competitor_gap_lower"]})
    return {"controls": rows, "hypothesis": "S_C", "theorem_A": "proved (helike-discrete-spectrum-lemma-2026-11-25)",
            "ordered": sum(r["verdict"] == "ORDERED" for r in rows)}


def load_flux_evidence():
    data = {}
    for name, (path, expected) in FLUX_RECEIPTS.items():
        if digest(ROOT/path) != expected:
            raise ValueError("flux receipt changed: " + name)
        data[name] = json.loads((ROOT/path).read_text())
    return make_flux_payload(data["convergence"], data["sectors"], data["leakage"], data["temple"])


def flux_figure(payload):
    fig, ax = plt.subplots(figsize=(7.2, 3.6))
    for i, r in enumerate(payload["controls"]):
        y = len(payload["controls"]) - 1 - i
        h, c = r["states"][1], r["states"][2]
        ax.plot(h["window"], [y + 0.12] * 2, color=TEAL, lw=5, alpha=0.25, solid_capstyle="butt")
        ax.plot(c["window"], [y - 0.12] * 2, color=GOLD, lw=5, alpha=0.25, solid_capstyle="butt")
        ax.plot([r["helike_upper"]], [y + 0.12], marker="|", ms=16, mew=2.2, color=TEAL)
        ax.plot([r["competitor_lower"]], [y - 0.12], marker="|", ms=16, mew=2.2, color=GOLD)
        label = f"{r['verdict']}\nmargin {r['margin']:+.4f}"
        ax.text(1.02, y, label, va="center", fontsize=8.5, color=INK if r["verdict"] == "ORDERED" else RED,
                transform=matplotlib.transforms.blended_transform_factory(ax.transAxes, ax.transData))
    ax.set_yticks(range(len(payload["controls"])))
    ax.set_yticklabels([f"g$^2$ = {r['g2']}, Mq = {r['Mq']}" for r in reversed(payload["controls"])])
    ax.set_xlim(1.0, 1.21)
    ax.set_xlabel("energy above the saved ground centre")
    ax.set_title("Helike's level (teal) against the competitor's (gold)")
    ax.plot([], [], color=TEAL, lw=5, alpha=0.25, label="Weinstein window")
    ax.plot([], [], marker="|", ms=12, mew=2, color=TEAL, ls="none", label="Helike upper bound")
    ax.plot([], [], marker="|", ms=12, mew=2, color=GOLD, ls="none", label="competitor lower bound (S$_C$)")
    ax.legend(loc="upper center", bbox_to_anchor=(0.5, -0.2), ncol=3, fontsize=8, frameon=False)
    save(fig, "09-flux-ordering")


def flux_bounds_figure(payload):
    fig, ax = plt.subplots(figsize=(7.2, 4.4))
    colors = {"ground": GRAY, "helike_plus": TEAL, "competitor": GOLD}
    labels, y = [], 0
    for r in payload["controls"]:
        for st in r["states"]:
            ax.plot([st["leakage_upper"], st["r_lower"]], [y, y], color=colors[st["state"]], lw=1, alpha=0.5)
            ax.plot(st["r_lower"], y, "o", ms=6, color=colors[st["state"]])
            ax.plot(st["leakage_upper"], y, "o", ms=6, mfc="white", mec=colors[st["state"]], mew=1.4)
            name = {"ground": "ground", "helike_plus": "Helike", "competitor": "competitor"}[st["state"]]
            labels.append(f"{name}  (g$^2$={r['g2']}, Mq={r['Mq']})")
            y += 1
    ax.set_xscale("log")
    ax.set_yticks(range(y)); ax.set_yticklabels(labels, fontsize=8); ax.invert_yaxis()
    ax.set_xlabel("norm (relative to the trial state)")
    ax.set_title("Residual r (filled) against cutoff leakage \u2113 (open)")
    ax.text(1.0, -0.16, "worst \u2113 / r = {:.1e}; registered limit 1e-3".format(max(s["leakage_over_r"] for r in payload["controls"] for s in r["states"])),
            transform=ax.transAxes, ha="right", fontsize=9, color=INK)
    save(fig, "10-flux-bounds")


def flux_convergence_figure(payload):
    fig, axes = plt.subplots(1, 2, figsize=(7.2, 3.4), sharey=False)
    for ax, g2 in zip(axes, (0.004, 0.002)):
        rows = [r for r in payload["controls"] if r["g2"] == g2]
        mqs = [r["Mq"] for r in rows]
        ax.plot(mqs, [r["ritz_gap_helike"] for r in rows], "-o", color=TEAL, label="Helike (Ritz)")
        ax.plot(mqs, [r["ritz_gap_competitor"] for r in rows], "-o", color=GOLD, label="competitor (Ritz)")
        for r in rows:
            if r["helike_gap"]:
                lo, hi = r["helike_gap"]
                ax.plot([r["Mq"] + 0.12] * 2, [lo, hi], color=TEAL, lw=3, alpha=0.5)
            ax.annotate("", xy=(r["Mq"] + 0.24, r["competitor_gap_lower"]), xytext=(r["Mq"] + 0.24, r["competitor_gap_lower"] - 0.003),
                        arrowprops=dict(arrowstyle="-|>", color=GOLD, lw=1.2))
        ax.set_xticks([6, 8]); ax.set_xlim(5.5, 8.7)
        ax.set_title(f"g$^2$ = {g2}"); ax.set_xlabel("basis cutoff Mq")
    axes[0].set_ylabel("excitation gap")
    axes[1].legend(*axes[0].get_legend_handles_labels(), fontsize=8, frameon=False, loc="center right")
    fig.text(0.5, -0.06, "Bars: rigorous enclosure of Helike's gap; arrows: rigorous lower bound on the competitor's (both under S$_0$, S$_H$, S$_C$).",
             ha="center", fontsize=8, color=INK)
    save(fig, "11-flux-convergence")


def flux_window_figure():
    fig, axes = plt.subplots(1, 2, figsize=(7.2, 3.2), sharey=True)
    grids = {3: [1.12, 1.15, 1.165, 1.18, 1.195, 1.22, 1.24], 4: [1.09, 1.12, 1.14, 1.16, 1.18]}
    for ax, t in zip(axes, (3.0, 4.0)):
        smax = (t + 2) / (t + 1)
        s = np.linspace(1.0, smax + 0.06, 400)
        k1 = s * ((2 + t) - s * (1 + t)) / t
        k0 = s * (2 + t - 2 * s)
        ax.axvspan(1.0, smax, color=TEAL, alpha=0.08, lw=0)
        ax.axhline(0, color=GRAY, lw=0.8)
        ax.axvline(smax, color=GRAY, lw=0.8, ls="--")
        ax.plot(s, k1, color=TEAL, lw=2, label="$\\omega_1^2-b$")
        ax.text(1.005, 0.46, f"$\\omega_0^2-a\\geq{k0[s <= smax].min():.2f}$ (off scale)", fontsize=8, color=GOLD)
        pts = np.array(grids[int(t)])
        ax.plot(pts, pts * ((2 + t) - pts * (1 + t)) / t, "o", ms=4, color=INK, label="registered points")
        if t == 3:
            ax.plot([1.15], [1.15 * ((2 + t) - 1.15 * (1 + t)) / t], "s", ms=6, mfc="white", mec=RED, mew=1.4, label="controls")
        ax.set_ylim(-0.08, 0.5); ax.set_title(f"t = {t:g}: window ends at s = {smax:.3f}")
        ax.set_xlabel("s")
    axes[0].set_ylabel("quadratic coefficient")
    axes[1].legend(*axes[0].get_legend_handles_labels(), fontsize=7.5, frameon=False, loc="lower left")
    save(fig, "12-flux-window")


def flux_chain_figure():
    fig, ax = plt.subplots(figsize=(7.2, 2.3)); ax.axis("off")
    steps = [("trial state\n$\\psi$ (Ritz)", "two solvers\nagree to 1e-13"),
             ("cutoff $\\eta$", "Arb: tail $t$,\nleakage $\\ell$"),
             ("charge\nprojection $P_m$", "interval: $c$,\n$q$"),
             ("residual\n$R=(r+\\ell)/q$", "interval: $r$,\nlinear $s$"),
             ("Lemmas 1\u20133", "Rayleigh quotient,\nwindows"),
             ("Kato\u2013Temple", "under S$_C$\n(A) proved")]
    n = len(steps)
    for i, (top, bottom) in enumerate(steps):
        x = i / n + 0.01
        ax.add_patch(matplotlib.patches.FancyBboxPatch((x, 0.42), 1 / n - 0.03, 0.46, boxstyle="round,pad=0.01",
                     fc="white", ec=TEAL if i < n - 1 else GOLD, lw=1.4, transform=ax.transAxes))
        ax.text(x + (1 / n - 0.03) / 2, 0.65, top, ha="center", va="center", fontsize=8.5, transform=ax.transAxes)
        ax.text(x + (1 / n - 0.03) / 2, 0.2, bottom, ha="center", va="center", fontsize=7.5, color=GRAY, transform=ax.transAxes)
        if i < n - 1:
            ax.annotate("", xy=(x + 1 / n - 0.005, 0.65), xytext=(x + 1 / n - 0.03, 0.65), xycoords="axes fraction",
                        arrowprops=dict(arrowstyle="-|>", color=INK, lw=1))
    save(fig, "13-flux-chain")


def write_flux_tables(payload):
    generated = OUT/"generated"
    generated.mkdir(parents=True, exist_ok=True)
    header = r"\begin{tabular}{@{}rrrrrrl@{}}\toprule $g^2$&Mq&Ritz gaps H / C&$U_H-E_0$&$L_C-E_0$&margin&verdict\\\midrule" + "\n"
    lines = [f"{r['g2']}&{r['Mq']}&{r['ritz_gap_helike']:.6f} / {r['ritz_gap_competitor']:.6f}&{r['helike_upper']:.6f}&"
             f"{r['competitor_lower']:.6f}&{r['margin']:+.6f}&{r['verdict']}\\\\" for r in payload["controls"]]
    (generated/"flux-ordering.tex").write_text(header+"\n".join(lines)+"\n"+r"\bottomrule\end{tabular}"+"\n", encoding="ascii")
    header = r"\begin{tabular}{@{}rrlrrrrr@{}}\toprule $g^2$&Mq&state&$r$ (lower)&$c\leq$&$\ell\leq$&$\ell/r\leq$&$R\leq$\\\midrule" + "\n"
    lines = []
    for r in payload["controls"]:
        for s in r["states"]:
            name = {"ground": "ground", "helike_plus": "Helike", "competitor": "competitor"}[s["state"]]
            lines.append(f"{r['g2']}&{r['Mq']}&{name}&{s['r_lower']:.6f}&{s['charge_upper']:.2e}&{s['leakage_upper']:.1e}&"
                         f"{s['leakage_over_r']:.1e}&{s['residual_bound']:.6f}\\\\")
    (generated/"flux-bounds.tex").write_text(header+"\n".join(lines)+"\n"+r"\bottomrule\end{tabular}"+"\n", encoding="ascii")


def make_sc_payload(status):
    """Rows for the S_C status page. S_C stays a stated hypothesis until the interval Lehmann stage (R2) closes;
    any receipt that promotes it, or marks a floating homotopy as rigorous, is refused."""
    if status.get("schema") != "helike/sc-status/v1":
        raise ValueError("not an S_C status receipt")
    if status.get("S_C") != "STATED_HYPOTHESIS":
        raise ValueError("S_C is reported only as a stated hypothesis until R2 closes")
    if any("homotopy" in item for item in status.get("proved_rigorous", [])):
        raise ValueError("the Lehmann homotopy is floating point; it is never listed as rigorous")
    if not status.get("tests_passed") or status.get("mutants") != "5/5":
        raise ValueError("R1.8 tests and mutants must pass")
    rows = []
    for c in status["controls"]:
        if c["fold"]["verdict"] != "PROVED" or c["outside"]["o1"] != "PROVED" or c["outside"]["o2"] != "PROVED":
            raise ValueError("fold and outside bound must be proved to be reported")
        if not (c["c3_sampled"] <= c["c3_symmetric"] <= c["c3_old"]):
            raise ValueError("C3 bounds out of order")
        last = [r["trail"][-1] for r in c["homotopy_floating"] if r["verdict"] == "PASS"]
        if not last:
            raise ValueError("no completed floating homotopy")
        rows.append(dict(g2=c["g2"], c3_old=c["c3_old"], c3=c["c3_symmetric"], c3_sampled=c["c3_sampled"],
                         fold=(c["fold"]["a"], c["fold"]["b"]), base_uniform=c["base_uniform"], base_folded=c["base_folded"],
                         o2_cells=c["outside"]["o2_cells"], rho_b=c["rho_b"], rho_C=c["rho_C_recomputed"],
                         runs=[r for r in c["homotopy_floating"] if r["verdict"] == "PASS"],
                         l5_margin=max(t["l5"] for t in last) - c["rho_C_recomputed"],
                         l11_margin=max(t["l11"] for t in last) - c["rho_b"]))
    return {"controls": rows, "S_C": "STATED_HYPOTHESIS", "remaining": status["remaining"]}


def load_sc_evidence():
    path, expected = SC_RECEIPT
    if digest(ROOT/path) != expected:
        raise ValueError("S_C status receipt changed")
    return make_sc_payload(json.loads((ROOT/path).read_text()))


def sc_fold_figure(payload):
    fig, axes = plt.subplots(1, 2, figsize=(7.2, 3.3))
    rows = payload["controls"]
    x = np.arange(len(rows)); w = 0.36
    ax = axes[0]
    ax.bar(x - w / 2, [r["base_uniform"]["levels_below_rho_b"] for r in rows], w, color=GRAY, label="uniform fold")
    ax.bar(x + w / 2, [r["base_folded"]["levels_below_rho_b"] for r in rows], w, color=TEAL, label="ray-aware fold")
    for i, r in enumerate(rows):
        ax.text(i - w / 2, r["base_uniform"]["levels_below_rho_b"] + 2, str(r["base_uniform"]["levels_below_rho_b"]), ha="center", fontsize=8)
        ax.text(i + w / 2, r["base_folded"]["levels_below_rho_b"] + 2, str(r["base_folded"]["levels_below_rho_b"]), ha="center", fontsize=8)
    ax.set_xticks(x); ax.set_xticklabels([f"g$^2$ = {r['g2']}" for r in rows])
    ax.set_ylabel("base levels below $\\rho_b$ (enclosed)"); ax.set_title("Levels the homotopy must open")
    ax.legend(fontsize=8, frameon=False)
    ax = axes[1]
    ax.bar(x - w / 2, [r["base_uniform"]["lambda_11"] for r in rows], w, color=GRAY)
    ax.bar(x + w / 2, [r["base_folded"]["lambda_11"] for r in rows], w, color=TEAL)
    for i, r in enumerate(rows):
        ax.plot([i - 0.45, i + 0.45], [r["rho_b"]] * 2, color=GOLD, lw=1.6)
    ax.plot([], [], color=GOLD, lw=1.6, label="$\\rho_b$")
    ax.set_ylim(5.5, 8.0); ax.set_xticks(x); ax.set_xticklabels([f"g$^2$ = {r['g2']}" for r in rows])
    ax.set_ylabel("base $\\lambda_{11}$ (above $V_{bg}$)"); ax.set_title("How far below the target the base starts")
    ax.legend(fontsize=8, frameon=False, loc="upper left")
    save(fig, "14-sc-fold")


def sc_homotopy_figure(payload):
    fig, axes = plt.subplots(1, 2, figsize=(7.2, 3.4), sharey=True)
    for ax, r in zip(axes, payload["controls"]):
        run = max(r["runs"], key=lambda q: q["loss"])
        ax.plot([t["s"] for t in run["trail"]], [t["l5"] for t in run["trail"]], "-o", color=TEAL, label="$\\ell_5$ (Lehmann)")
        ax.plot([t["s"] for t in run["trail"]], [t["l11"] for t in run["trail"]], "-s", color=GRAY, ms=4, label="$\\ell_{11}$")
        ax.axhline(r["rho_C"], color=GOLD, lw=1.6, label="$\\rho_C$")
        ax.axhline(r["rho_b"], color=GOLD, lw=1, ls="--", label="$\\rho_b$")
        last = run["trail"][-1]
        xm = last["s"] + 0.03
        ax.annotate("", xy=(xm, last["l5"]), xytext=(xm, r["rho_C"]), arrowprops=dict(arrowstyle="<->", color=TEAL, lw=1.1))
        ax.text(xm + 0.02, (last["l5"] + r["rho_C"]) / 2, f"+{last['l5'] - r['rho_C']:.3f}", ha="left", va="center",
                fontsize=8.5, color=TEAL)
        ax.set_title(f"g$^2$ = {r['g2']}: {len(run['trail'])} real steps"); ax.set_xlabel("homotopy parameter s")
        ax.set_xlim(0, 1.25); ax.set_xticks([0, 0.25, 0.5, 0.75, 1.0])
    axes[0].set_ylabel("lower bound (above $V_{bg}$), floating point")
    axes[1].legend(fontsize=7.5, frameon=False, loc="lower right")
    save(fig, "15-sc-homotopy")


def write_sc_table(payload):
    generated = OUT/"generated"
    generated.mkdir(parents=True, exist_ok=True)
    header = (r"\begin{tabular}{@{}rrrrrrr@{}}\toprule $g^2$&$C_3\leq$ (old / new)&fold $(a,b)$&levels (uniform / fold)"
              r"&O2 cells&$\ell_5-\rho_C$&$\ell_{11}-\rho_b$\\\midrule" + "\n")
    lines = [f"{r['g2']}&{r['c3_old']:.4f} / {r['c3']:.4f}&({r['fold'][0]}, {r['fold'][1]})&"
             f"{r['base_uniform']['levels_below_rho_b']} / {r['base_folded']['levels_below_rho_b']}&{r['o2_cells']:,}&"
             f"{r['l5_margin']:+.3f}&{r['l11_margin']:+.4f}\\\\" for r in payload["controls"]]
    (generated/"sc-status.tex").write_text(header+"\n".join(lines)+"\n"+r"\bottomrule\end{tabular}"+"\n", encoding="ascii")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pdf", action="store_true", help="also compile the paper-ready LaTeX report")
    args = parser.parse_args()
    receipt, payload = load_evidence()
    sector_receipt, sectors = load_sector_evidence()
    flux = load_flux_evidence()
    OUT.mkdir(parents=True, exist_ok=True)
    style()
    response_figure(payload); geometry_figure(); spectrum_figure(payload)
    cutoff_figure(payload); gate_figure(payload); write_tables(payload)
    sector_figures(sectors); write_sector_tables(sectors)
    flux_figure(flux); flux_bounds_figure(flux); flux_convergence_figure(flux)
    flux_window_figure(); flux_chain_figure(); write_flux_tables(flux)
    sc = load_sc_evidence()
    sc_fold_figure(sc); sc_homotopy_figure(sc); write_sc_table(sc)
    (OUT/"sc-plot-data.json").write_text(json.dumps(sc, indent=2)+"\n", encoding="utf-8")
    (OUT/"flux-plot-data.json").write_text(json.dumps(flux, indent=2)+"\n", encoding="utf-8")
    if args.pdf:
        logs = ROOT/"tmp/pdfs/helike-report"
        logs.mkdir(parents=True, exist_ok=True)
        for pass_number in (1, 2):
            run = subprocess.run(["pdflatex", "-interaction=nonstopmode", "-halt-on-error", "report.tex"],
                                 cwd=OUT, capture_output=True, text=True)
            (logs/f"latex-pass-{pass_number}.txt").write_text(run.stdout+run.stderr, encoding="utf-8")
            if run.returncode:
                raise RuntimeError(f"LaTeX pass {pass_number} failed; see {logs}")
    paths = list((OUT/"figures").glob("*")) + list((OUT/"generated").glob("*.tex"))
    paths += [OUT/"plot-data.json", OUT/"case-summary.csv", OUT/"level-weights.csv",
              OUT/"sector-plot-data.json", OUT/"sector-summary.csv", OUT/"flux-plot-data.json", OUT/"sc-plot-data.json"]
    if args.pdf:
        paths += [OUT/"report.pdf"]
    manifest = {"source_run": RUN_ID, "source_status": receipt["status"],
                "source_receipt_sha256": digest(RUN/"receipt.json"),
                "source_verification_sha256": digest(RUN/"verification.json"),
                "builder_sha256": digest(Path(__file__)), "latex_sha256": digest(OUT/"report.tex"),
                "sector_source_revision": sector_receipt["source_revision"],
                "sector_receipt_sha256": digest(SECTOR_RUN/"receipt.json"),
                "sector_verification_sha256": digest(SECTOR_RUN/"verification.json"),
                "flux_receipts_sha256": {k: v[1] for k, v in FLUX_RECEIPTS.items()},
                "sc_receipt_sha256": SC_RECEIPT[1],
                "outputs": {str(p.relative_to(OUT)).replace("\\", "/"): digest(p) for p in paths}}
    (OUT/"artifact-manifest.json").write_text(json.dumps(manifest, indent=2)+"\n", encoding="utf-8")
    print(json.dumps({"status": "BUILT", "source_status": receipt["status"], "figures": 15,
                      "cases": len(payload["cases"]), "sector_cases": len(sectors["cases"]), "output": str(OUT)}, indent=2))


if __name__ == "__main__":
    main()
