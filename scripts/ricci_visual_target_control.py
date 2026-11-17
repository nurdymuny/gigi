"""Render the retained reference through the identical mesh; no flow solves."""
import json
from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np

from ricci_visual_review import NATIVE, OLD, RECOVERED, OUT, read, section, sha, slice_cells


def main():
    receipt_path = OUT / "target-control-receipt.json"
    if receipt_path.exists():
        raise ValueError("preserve previous visual receipt")
    previous = read(OUT / "receipt.json")
    for path, expected in previous["sources"].items():
        if sha(Path(path)) != expected:
            raise ValueError("retained source drift: " + path)
    if sha(Path(__file__).with_name("ricci_visual_review.py")) != previous["script_sha256"]:
        raise ValueError("visual sampling source drift")
    chart = np.array(read(NATIVE / "level0_berger_q6.input.json")["cells"][0]["vertices"])
    plt.rcParams.update({"font.family": "DejaVu Sans", "font.size": 12})
    fig, axes = plt.subplots(2, 2, figsize=(12, 6.8), layout="constrained")
    receipt = {
        "scope": "same-renderer reference control only; no new flow solve or gate claim",
        "script_sha256": sha(Path(__file__)),
        "previous_receipt_sha256": sha(OUT / "receipt.json"),
        "levels": {},
    }
    for row, level in enumerate((1, 3)):
        key = f"level{level}_q6"
        source = OUT / (key + ".visual.npz")
        with np.load(source, allow_pickle=False) as saved:
            line = saved["line"].copy()
            actual = saved["line_values"].copy()
            owners = saved["line_owners"].copy()
        with np.load(OLD / (key + ".arrays.npz"), allow_pickle=False) as saved:
            a = saved["abelian_edge_integrals"].copy()
            old_velocity = saved["abelian_velocity"].copy()
        with np.load(RECOVERED / (key + ".arrays.npz"), allow_pickle=False) as saved:
            np.testing.assert_allclose(a, saved["a"][:, 0], atol=1e-13, rtol=0)
        payload = read(NATIVE / f"level{level}_berger_q6.input.json")
        cells, _ = slice_cells(payload, chart)
        control, repeated_owners, _ = section(payload, cells, chart, [-4 * a, old_velocity], line)
        np.testing.assert_array_equal(owners, repeated_owners)
        np.testing.assert_allclose(actual[0], control[1], atol=1e-13, rtol=0)
        assert np.isfinite(control).all()
        values = np.concatenate([actual, control[:1]], axis=0)
        sideways = np.linalg.norm(values[:, :, 1:], axis=2)
        np.savez_compressed(OUT / (key + ".target-control.npz"), line=line, values=values, owners=owners)
        receipt["levels"][str(level)] = {
            "visual_samples_sha256": sha(source),
            "control_samples_sha256": sha(OUT / (key + ".target-control.npz")),
            "same_renderer_actual_replay": "matched within absolute 1e-13",
            "reference_forward_range": [float(values[2, :, 0].min()), float(values[2, :, 0].max())],
            "reference_sideways_max": float(sideways[2].max()),
        }
        for col, quantities in enumerate((values[:, :, 0], sideways)):
            ax = axes[row, col]
            for position in line[np.r_[False, np.diff(owners) != 0], 0]:
                ax.axvline(position, color="#aaaaaa", linewidth=0.7, alpha=0.65)
            for i, (label, color, style) in enumerate((
                    ("Original lattice", "#2166ac", "-"),
                    ("Fitted recovery", "#b33e52", "-"),
                    ("Reference through same mesh", "#222222", "--"))):
                ax.plot(line[:, 0], quantities[i], color=color, linestyle=style,
                        linewidth=1.7, label=label)
            ax.set_xlim(line[:, 0].min(), line[:, 0].max())
            ax.set_ylim((0.45, 1.55) if col == 0 else (-0.06, 1.7))
            ax.set_yticks([0.5, 1, 1.5] if col == 0 else [0, 0.5, 1, 1.5])
            ax.set_xticks([])
            ax.spines[["top", "right"]].set_visible(False)
            ax.set_ylabel("Relative to desired speed")
            ax.set_title(("Forward motion" if col == 0 else "Sideways motion") + " | "
                         + ("coarser mesh" if row == 0 else "finer mesh"), loc="left")
    handles, labels = axes[0, 0].get_legend_handles_labels()
    fig.legend(handles, labels, loc="outside upper center", ncol=3, frameon=False)
    fig.supxlabel("Same fixed crossing; gray lines mark cell boundaries.\n"
                  "Saved Abelian control fields, not a Ricci-flow trajectory or the final RF2G candidate.",
                  fontsize=11)
    image = OUT / "motion-target-control.png"
    fig.savefig(image, dpi=160)
    plt.close(fig)
    receipt["image_sha256"] = sha(image)
    receipt_path.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(receipt["levels"], indent=2))


if __name__ == "__main__":
    main()
