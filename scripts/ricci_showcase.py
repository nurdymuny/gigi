"""Render only verified captured GIGI rows; no simulated or invented plot data."""
import argparse
import json
from pathlib import Path
import sys
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/"e2e/probes"))
from monodromy_evidence import sha


def run(path,output):
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    receipt = json.loads(path.with_suffix(".PASS.json").read_text(encoding="utf-8"))
    if receipt["status"] != "PASS" or receipt["evidence_sha256"] != sha(path):
        raise ValueError("showcase requires matching verified live evidence")
    data = json.loads(path.read_text(encoding="utf-8"))
    cases = {r["label"]:r for r in data["requests"]}
    fig, axes = plt.subplots(1,3,figsize=(14,4),layout="constrained")
    for ax,name,title in zip(axes,["round_shrink","rounding","negative_scalar"],
                             ["Round sphere: shrink","Distorted sphere: become round","Negative scalar curvature: evolve"]):
        rows = cases[name]["response"]["rows"]
        t = [r["t"] for r in rows]
        if name == "negative_scalar":
            ax.plot(t,[r["scalar_curvature"] for r in rows],color="#b54368",label="Scalar curvature")
            ax.axhline(0,color="#999999",lw=.7)
            ax.set_ylabel("Scalar curvature")
        else:
            for key,color in [("a","#007f7b"),("b","#b54368"),("c","#d29318")]:
                ax.plot(t,[r[key] for r in rows],label=key.upper(),color=color,lw=1.8)
            ax.set_ylabel("Metric coefficients (squared length)")
        ax.set_title(title,fontsize=11)
        ax.set_xlabel("Ricci-flow time (squared length)")
        ax.legend(frameon=False,fontsize=9)
        ax.spines[["top","right"]].set_visible(False)
        ax.grid(alpha=.15)
    fig.suptitle("GIGI: native Ricci flow on homogeneous three-spheres",fontsize=15)
    fig.savefig(output,dpi=180)
    plt.close(fig)


if __name__ == "__main__":
    p=argparse.ArgumentParser()
    p.add_argument("evidence",type=Path)
    p.add_argument("output",type=Path)
    a=p.parse_args()
    run(a.evidence,a.output)
