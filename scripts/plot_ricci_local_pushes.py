"""Figures from retained attribution receipts; no numerical solves."""
import json
from pathlib import Path
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.patches import FancyArrowPatch
from mpl_toolkits.mplot3d.art3d import Poly3DCollection
import numpy as np

from ricci_local_push_audit import OUT, ROOT, NATIVE, read, sha
from ricci_variational_math import FACES
from ricci_variational_mesh import topology


def main():
    receipt_file = OUT / "plot-receipt-v3.json"
    if receipt_file.exists():
        raise ValueError("preserve previous plot receipt")
    receipt = read(OUT / "receipt.json")
    interface = read(OUT / "interface-receipt.json")
    if sha(OUT / "receipt.json") != interface["prior_receipt_sha256"]:
        raise ValueError("attribution receipt drift")
    for data, name in ((receipt, "attributions.npz"), (interface, "interfaces.npz")):
        if sha(OUT / name) != data["arrays_sha256"]:
            raise ValueError("array drift")
        for filename, digest in data["sources"].items():
            if sha(Path(filename)) != digest:
                raise ValueError("source drift")
    with np.load(OUT / "attributions.npz", allow_pickle=False) as data:
        points = data["patch_points"].copy(); chart = data["chart"].copy()
        point = data["point"].copy(); ids = data["patch_ids"].tolist()
    inv = np.linalg.inv(chart.T)
    homogeneous = points @ inv.T
    xyz = homogeneous[:, :, 1:] / homogeneous.sum(axis=2)[:, :, None]
    homogeneous_point = inv @ point
    dot = homogeneous_point[1:] / homogeneous_point.sum()
    pair = interface["patch_internal_faces"][0]
    assert pair["face"] == 0
    labels = {ci: f"C{i+1}" for i, ci in enumerate(ids)}
    plt.rcParams.update({"font.family": "DejaVu Sans", "font.size": 12})
    fig = plt.figure(figsize=(12, 8), layout="constrained")
    grid = fig.add_gridspec(2, 2, width_ratios=[1, 1.35])
    ax = fig.add_subplot(grid[:, 0], projection="3d")
    for i, ci in enumerate(ids):
        color = "#3782ad" if ci == pair["cells"][0] else ("#cf872d" if ci == pair["cells"][1] else "#b5bec1")
        triangles = [xyz[i, list(face)] for face in FACES]
        ax.add_collection3d(Poly3DCollection(triangles, facecolors=color, alpha=.13,
                                            edgecolors=color, linewidths=.8))
        center = xyz[i].mean(axis=0)
        offsets = {3: [0., 0., .055], 456: [0., 0., -.045], 459: [-.03, 0., -.02]}
        ax.text(*(center + np.array(offsets.get(ci, [0., 0., 0.]))), labels[ci], fontsize=12, color="#222222")
    topo = topology(read(NATIVE / "level1_berger_q6.input.json"))
    ci, local, _ = topo["owners"][pair["face"]][0]
    triangle = xyz[ids.index(ci), list(FACES[local])]
    ax.add_collection3d(Poly3DCollection([triangle], facecolors="#cc5262", alpha=.30,
                                        edgecolors="#ac3346", linewidths=1.6))
    ax.scatter(*dot, color="#151515", s=42, depthshade=False)
    ax.text2D(.06, .18, "Black dot: fixed readout", transform=ax.transAxes, fontsize=11)
    low = xyz.reshape(-1, 3).min(axis=0); high = xyz.reshape(-1, 3).max(axis=0)
    middle = (low+high)/2; half = max(high-low)*.55
    ax.set_xlim(middle[0]-half, middle[0]+half)
    ax.set_ylim(middle[1]-half, middle[1]+half)
    ax.set_zlim(middle[2]-half, middle[2]+half)
    ax.set_box_aspect([1, 1, 1]); ax.view_init(elev=23, azim=-55); ax.set_axis_off()
    ax.set_title("One fixed six-cell patch\nHighlighted face: C1 / C4", fontsize=15)

    arrows = fig.add_subplot(grid[0, 1])
    for row, (name, parts) in enumerate((("Smooth reference: balances", pair["smooth_sides"]),
                                       ("Discrete curvature: leftover", pair["sides"]))):
        y = 1.15 - row
        first, second = np.array(parts)[:, 0]
        end = first+second
        arrows.text(-1.53, y+.27, name, fontsize=13)
        arrows.add_patch(FancyArrowPatch((0, y+.08), (first, y+.08), arrowstyle="-|>",
                                         mutation_scale=14, color="#3782ad", linewidth=2))
        arrows.add_patch(FancyArrowPatch((first, y-.08), (end, y-.08), arrowstyle="-|>",
                                         mutation_scale=14, color="#cf872d", linewidth=2))
        arrows.plot([first, first], [y-.08, y+.08], color="#777777", linewidth=.8)
        arrows.text(first/2, y+.13, f"C1: {first:+.6f}", ha="center", fontsize=11)
        arrows.text(first/2, y-.27, f"C4: {second:+.6f}", ha="center", fontsize=11)
        arrows.plot(end, y-.08, "o", color="#ac3346", markersize=4)
        if row == 1:
            arrows.annotate(f"Net +{end:.6f}", xy=(end, y-.08), xytext=(-.26, y-.48),
                            arrowprops={"arrowstyle": "->", "color": "#ac3346"}, fontsize=11)
    arrows.axvline(0, color="#777777", linestyle="--", linewidth=.8)
    arrows.set_xlim(-1.57, .25); arrows.set_ylim(-.53, 1.73)
    arrows.set_yticks([]); arrows.set_xticks([-1.4, -.7, 0])
    arrows.spines[["left", "top", "right"]].set_visible(False)
    arrows.set_xlabel("Signed contribution to first sideways readout", fontsize=11)
    arrows.set_title("Two sides of the same shared face", fontsize=15)

    bars = fig.add_subplot(grid[1, 1])
    pairs = interface["patch_internal_faces"]
    values = np.array([p["net"] for p in pairs])
    ys = np.arange(len(pairs))
    bars.barh(ys-.15, values[:, 0], height=.27, color="#246d97", label="Sideways direction 1")
    bars.barh(ys+.15, values[:, 1], height=.27, color="#a94d78", label="Sideways direction 2")
    bars.set_yticks(ys, [" / ".join(labels[ci] for ci in p["cells"]) for p in pairs])
    bars.invert_yaxis(); bars.axvline(0, color="#777777", linewidth=.8)
    bars.spines[["top", "right", "left"]].set_visible(False)
    bars.set_xlim(-.052, .082)
    bars.set_xticks([-.04, 0, .04, .08], ["-4%", "0", "+4%", "+8%"])
    bars.legend(loc="lower right", frameon=False, fontsize=10)
    bars.set_title("Leftovers at all six internal shared faces", fontsize=14)
    bars.set_xlabel("Contribution relative to intended speed; enlarged scale", fontsize=11)
    fig.suptitle("Shared-face contributions do not quite cancel", fontsize=19)
    fig.supxlabel("Arrows are signed contributions to the fixed readout, not cell trajectories.\n"
                  "The full accounting includes exterior faces and the rest of the mesh. Original Abelian control only.",
                  fontsize=11)
    image = OUT / "shared-face-pushes-v3.png"
    fig.savefig(image, dpi=170)
    plt.close(fig)
    output = {"scope": "visualization only", "source_receipt_sha256": sha(OUT/"receipt.json"),
        "interface_receipt_sha256": sha(OUT/"interface-receipt.json"), "script_sha256": sha(Path(__file__)),
        "image_sha256": sha(image), "illustrated_face": "first face in fixed patch, not largest error"}
    receipt_file.write_text(json.dumps(output, indent=2)+"\n", encoding="utf-8")
    print(str(image))


if __name__ == "__main__":
    main()
