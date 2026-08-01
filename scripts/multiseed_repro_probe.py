#!/usr/bin/env python
"""Cross-restart reproducibility probe for the gigi arms of the multiseed sweep.

The 2026-08-01 verification of scripts/sweep_results_multiseed.json found that
gigi /cluster results reproduced only WITHIN one server process: records() fed
the fixed-LCG inits a per-process-random HashMap order (src/ml/cluster.rs /
src/bundle.rs). That was fixed by making Bundle::records() yield
base-point-sorted records. This probe is the standing re-verification:

  * starts TWO fresh gigi-stream server processes in sequence (each with its
    own scratch data dir — a genuine restart, so each server's HashMaps get
    fresh random SipHash states),
  * replays the gigi arm of every /cluster cell in the artifact (iris / wine /
    digits x kmeans / gmm / spectral), BOTH arms of the embargoed eigenspace
    cell, and the embargoed diffusion cell's gigi arm,
  * verifies (1) process A == process B value-for-value (cross-restart
    determinism), and (2) both match the committed artifact
    scripts/sweep_results_multiseed.json exactly (the artifact reproduces).

Exit code 0 only if every comparison is exact (values compared at the
artifact's 6-decimal rounding).

Usage:  python scripts/multiseed_repro_probe.py [--port 3163]
        [--artifact scripts/sweep_results_multiseed.json]
"""
import argparse
import json
import os
import sys

import numpy as np
from sklearn import datasets
from sklearn.metrics import adjusted_rand_score

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(REPO, "scripts"))
from multiseed_sweep import Gigi, GigiServer  # noqa: E402  (shared harness)


def gari(resp, y):
    lab = [r["cluster"] for r in sorted(resp["results"], key=lambda x: int(x["id"][1:]))]
    return adjusted_rand_score(y, lab)


def run_one_process(port):
    """Fresh server process -> {probe_key: rounded value}."""
    iris = datasets.load_iris()
    wine = datasets.load_wine()
    dig = datasets.load_digits()
    server = GigiServer(port)
    server.start()
    out = {}
    try:
        gg = Gigi(server.base)
        for nm, ds, k in [("iris", iris, 3), ("wine", wine, 3), ("digits", dig, 10)]:
            gg.load(nm + "_cl", ds.data)
            for m in ["kmeans", "gmm", "spectral"]:
                body = {"method": m, "k": k}
                if m == "gmm":
                    body["covariance"] = "diagonal" if nm == "digits" else "full"
                if m == "spectral":
                    body.update({"neighbors": 15, "normalized": True})
                v = gari(gg.call(f"/v1/bundles/{nm}_cl/cluster", body), ds.target)
                out[f"cluster/{nm}/{m}"] = round(float(v), 6)
        # embargoed eigenspace cell — both arms, shared gmm_em, covariance=diagonal
        v = gari(gg.call("/v1/bundles/digits_cl/cluster",
                         {"method": "spectral", "k": 10, "neighbors": 15,
                          "normalized": True, "head": "gmm", "embed_dim": 20,
                          "covariance": "diagonal"}), dig.target)
        out["embargo-a/eigenspace"] = round(float(v), 6)
        v = gari(gg.call("/v1/bundles/digits_cl/cluster",
                         {"method": "gmm", "k": 10, "covariance": "diagonal"}), dig.target)
        out["embargo-a/raw"] = round(float(v), 6)
        # embargoed diffusion cell — gigi arm
        gg.load("digits_c", dig.data, "label TEXT FIBER",
                lambda i, t=dig.target: {"label": f"c{t[i]}"})
        v = gg.call("/v1/bundles/digits_c/infer",
                    {"target": "label", "method": "diffusion", "k": 7})["metric"]["accuracy"]
        out["embargo-b/diffusion"] = round(float(v), 6)
        server.verify_alive()
    finally:
        server.stop()
    return out


def artifact_expectations(path):
    """{probe_key: list of per-seed artifact values} for every probed cell."""
    art = json.load(open(path, encoding="utf-8"))
    exp = {}
    for c in art["cells"]:
        if c["task"] == "cluster" and c["dataset"] in ("iris", "wine", "digits"):
            exp[f"cluster/{c['dataset']}/{c['method']}"] = c["arms"]["gigi"]["values"]
        if c["task"] == "cluster-representation":
            exp["embargo-a/eigenspace"] = c["arms"]["gigi"]["values"]
            exp["embargo-a/raw"] = c["arms"]["ref"]["values"]
        if c["method"].startswith("diffusion"):
            exp["embargo-b/diffusion"] = c["arms"]["gigi"]["values"]
    return exp


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--port", type=int, default=3163)
    ap.add_argument("--artifact",
                    default=os.path.join(REPO, "scripts", "sweep_results_multiseed.json"))
    args = ap.parse_args()

    exp = artifact_expectations(args.artifact)
    print("[probe] pass 1: fresh server process A")
    a = run_one_process(args.port)
    print("[probe] pass 2: fresh server process B (genuine restart)")
    b = run_one_process(args.port)

    failures = []
    for key in sorted(a):
        va, vb = a[key], b[key]
        cross = "OK" if va == vb else "MISMATCH"
        if va != vb:
            failures.append(f"{key}: cross-restart drift A={va} B={vb}")
        if key in exp:
            vals = exp[key]
            if len(set(vals)) != 1:
                failures.append(f"{key}: artifact per-seed values not constant: {vals}")
            elif vals[0] != va:
                failures.append(f"{key}: artifact={vals[0]} probe={va}")
            art_s = f"artifact={vals[0]}"
        else:
            failures.append(f"{key}: cell missing from artifact {args.artifact}")
            art_s = "artifact=MISSING"
        print(f"  {key:28} A={va:<10} B={vb:<10} [{cross}] {art_s}")

    if failures:
        print("\nPROBE FAILED:")
        for f in failures:
            print("  - " + f)
        sys.exit(1)
    print(f"\nPROBE PASSED: {len(a)} gigi-arm values identical across two fresh "
          "server processes and equal to the committed artifact.")


if __name__ == "__main__":
    main()
