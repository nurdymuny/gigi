"""M8 runner (docs/research/helike-M8-weighted-geometry-registration-2026-11-26.md).
usage: python scripts/run_helike_m8.py --controls OUT_DIR     (s = 1.15, t = 3; g2 = 0.002, 0.004; Mq = 8, 10)
Grid points run only after the registered flux grid completes (a separate entry point, not yet enabled)."""
import argparse
import json
import subprocess
import sys
import time
from pathlib import Path

import numpy as np

import helike_m8_geometry as gm

ROOT = Path(__file__).resolve().parents[1]
BIN = ROOT / ("helike-lab/target/release/helike" + (".exe" if sys.platform == "win32" else ""))
Q0A_TOL = 1e-4          # relative
Q0B_TOL = 1e-3          # shares, order 10 vs 12
Q2_TOL = 1e-6           # alignment vs the solver's §5.3 dressed overlap
RESIDUAL_FLAG = 1e-2
POSITIVITY_FLOOR = 1e-12
ORDERS = (10, 12)


def export(s, t, g2, mq, out):
    path = out / f"levels_g{g2}_Mq{mq}.json"
    if not path.exists():
        subprocess.run([str(BIN), "--out", str(out / f"levels_g{g2}_Mq{mq}.receipt.json"), "cone-levels", "--s", str(s),
                        "--t", str(t), "--g2", str(g2), "--mq", str(mq), "--levels-out", str(path)],
                       check=True, capture_output=True, text=True)
    return gm.load_levels(path)


def carriers(X):
    br = gm.branches(X)
    hel = [lev for lev, q in br if q == 2]
    ident = X["raw"]["identification"]
    comp = ident["competitor"]["level_index"]
    if not hel or ident["helike"]["level_index"] not in hel[0]:
        raise ValueError("the lowest charge-2 branch is not the identified Helike level")
    if not any(lev == (comp,) and q == 0 for lev, q in br):
        raise ValueError("the identified competitor is not a single charge-0 level")
    return {"helike": (hel[0], "H"), "competitor": ((comp,), "C")}, br


def label(own, shares, block):
    top = max(shares, key=shares.get)
    if top != block:
        return "TURNED"
    return "ALIGNED" if own >= 0.5 else "DRESSED"


def point(s, t, g2, mq, out):
    X = export(s, t, g2, mq, out)
    car, br = carriers(X)
    res = {o: gm.analyse_point(X, o, car) for o in ORDERS}
    fine, coarse = res[12], res[10]
    rep = {"g2": g2, "Mq": mq, "branches": [[list(l), q] for l, q in br], "orders": {str(o): res[o] for o in ORDERS}}
    open_reasons = []
    for name, (lev, block) in car.items():
        f, c = fine[name], coarse[name]
        solver = X["raw"]["dressed_overlap_helike" if block == "H" else "dressed_overlap_competitor"]
        q2 = max(abs(a - solver[l]) for a, l in zip(f["alignment_per_level"], lev)) if all(solver[l] is not None for l in lev) else None
        seam = max(abs(f["shares"][b] - c["shares"][b]) for b in gm.BLOCKS)
        gates = {"Q0a_identity": f["identity_rel_error"] <= Q0A_TOL, "Q0b_quadrature": seam <= Q0B_TOL,
                 "Q2_cross_check": q2 is not None and q2 <= Q2_TOL,
                 "positivity": f["nonpositive_weight"] == 0.0}
        for k, v in gates.items():
            if not v:
                open_reasons.append(f"{name}: {k}")
        rep[name] = dict(levels=list(lev), block=block, own_share=f["own_share"], shares=f["shares"],
                         label=label(f["own_share"], f["shares"], block), gates=gates,
                         identity_rel_error=f["identity_rel_error"], share_seam=seam, q2_difference=q2,
                         residual_share=f["residual_share"], ground_state_limited=f["residual_share"] > RESIDUAL_FLAG,
                         gap=f["ritz_excitation"] / len(lev), alignment=f["alignment"])
    rep["shape_price"] = {"H": fine["shape_price_H"], "C": fine["shape_price_C"]}
    rep["dressing_discount"] = {"H": fine["shape_price_H"] - rep["helike"]["gap"], "C": fine["shape_price_C"] - rep["competitor"]["gap"]}
    rep["ordering"] = {"shapes": int(np.sign(fine["shape_price_C"] - fine["shape_price_H"])),
                       "levels": int(np.sign(rep["competitor"]["gap"] - rep["helike"]["gap"]))}
    rep["status"] = "OPEN" if open_reasons else "RESOLVED"
    rep["open_reasons"] = open_reasons
    rep["Q4"] = "not computed (registered as diagnostic; not implemented)"
    return rep


def controls(out_dir):
    out = Path(out_dir); out.mkdir(parents=True, exist_ok=True)
    rep = {"stage": "M8 controls", "points": []}
    for g2 in (0.002, 0.004):
        for mq in (8, 10):
            t0 = time.time()
            r = point(1.15, 3.0, g2, mq, out)
            r["seconds"] = time.time() - t0
            rep["points"].append(r)
            print(json.dumps({k: r[k] for k in ("g2", "Mq", "status", "open_reasons")} |
                             {"helike": {k: r["helike"][k] for k in ("own_share", "label", "identity_rel_error", "share_seam", "q2_difference", "residual_share")},
                              "competitor": {k: r["competitor"][k] for k in ("own_share", "label", "identity_rel_error", "share_seam", "q2_difference", "residual_share")},
                              "ordering": r["ordering"], "discount": r["dressing_discount"]}, default=float), flush=True)
            (out / "report.json").write_text(json.dumps(rep, indent=1, default=float) + "\n")
    print("m8_controls_done", flush=True)
    return rep


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("--controls", metavar="OUT_DIR")
    a = ap.parse_args()
    if a.controls:
        controls(a.controls)
