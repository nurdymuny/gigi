"""Inspect cached visual samples only; no assembly, solve, or gate change."""
import hashlib
import json
from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np


ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "output/ricci-visual-review-20261117"


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    receipt_path = OUT / "closeup-receipt.json"
    if receipt_path.exists():
        raise ValueError("preserve previous visual receipt")
    previous = json.loads((OUT / "receipt.json").read_text(encoding="utf-8"))
    for path, expected in previous["sources"].items():
        if digest(Path(path)) != expected:
            raise ValueError("retained source drift: " + path)
    if digest(ROOT / "scripts/ricci_visual_review.py") != previous["script_sha256"]:
        raise ValueError("visual sampling source drift")

    plt.rcParams.update({"font.family": "DejaVu Sans", "font.size": 12})
    fig, axes = plt.subplots(2, 2, figsize=(12, 6.8), layout="constrained")
    receipt = {
        "scope": "visual only, fixed local Abelian slice; no new flow solve or acceptance claim",
        "cases": "RF2D original and RF2F fitted recovery, not the final RF2G candidate",
        "previous_receipt_sha256": digest(OUT / "receipt.json"),
        "script_sha256": digest(Path(__file__)),
        "samples": {},
    }
    for row, level in enumerate((1, 3)):
        path = OUT / f"level{level}_q6.visual.npz"
        with np.load(path, allow_pickle=False) as data:
            values = data["line_values"]
            along = data["line"][:, 0]
            owners = data["line_owners"]
        assert values.shape == (2, 1200, 3)
        assert np.isfinite(values).all()
        assert np.all(np.diff(along) > 0)
        sideways = np.linalg.norm(values[:, :, 1:], axis=2)
        assert values[:, :, 0].min() >= 0.45 and values[:, :, 0].max() <= 1.55
        assert sideways.max() <= 1.7
        receipt["samples"][path.name] = {
            "sha256": digest(path), "samples": len(along),
            "original_sideways_max": float(sideways[0].max()),
            "recovered_sideways_max": float(sideways[1].max()),
        }
        for col, quantities in enumerate((values[:, :, 0], sideways)):
            ax = axes[row, col]
            for position in along[np.r_[False, np.diff(owners) != 0]]:
                ax.axvline(position, color="#aaaaaa", linewidth=0.7, alpha=0.65)
            for i, (name, color) in enumerate((("Original lattice", "#2166ac"),
                                               ("Fitted recovery", "#b33e52"))):
                ax.plot(along, quantities[i], color=color, linewidth=1.7, label=name)
            ax.axhline(1 if col == 0 else 0, color="#168265", linewidth=2,
                       linestyle="--", label="Desired motion")
            ax.set_title(("Coarser" if row == 0 else "Finer") + " mesh", loc="left")
            ax.set_xlim(along.min(), along.max())
            ax.set_ylim((0.45, 1.55) if col == 0 else (-0.06, 1.7))
            ax.set_yticks([0.5, 1, 1.5] if col == 0 else [0, 0.5, 1, 1.5])
            ax.set_xticks([])
            ax.spines[["top", "right"]].set_visible(False)
            ax.set_ylabel("Relative to desired speed")
    axes[0, 0].set_title("Forward motion | coarser mesh", loc="left")
    axes[0, 1].set_title("Unwanted sideways motion | coarser mesh", loc="left")
    axes[1, 0].set_title("Forward motion | finer mesh", loc="left")
    axes[1, 1].set_title("Unwanted sideways motion | finer mesh", loc="left")
    handles, labels = axes[0, 0].get_legend_handles_labels()
    fig.legend(handles, labels, loc="outside upper center", ncol=3, frameon=False)
    fig.supxlabel("Same fixed crossing in both meshes; gray lines mark cell boundaries.\n"
                  "Saved Abelian control fields. Local view only, not the global acceptance score.",
                  fontsize=11)
    image = OUT / "motion-closeup.png"
    fig.savefig(image, dpi=160)
    plt.close(fig)
    receipt["image_sha256"] = digest(image)
    receipt_path.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(receipt["samples"], indent=2))


if __name__ == "__main__":
    main()
