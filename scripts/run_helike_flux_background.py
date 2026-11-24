"""Registered run: docs/research/helike-flux-background-registration-2026-11-24.md (the sourced condensate).

usage: python scripts/run_helike_flux_background.py OUT_DIR [--smoke]

Fail-closed: OUT_DIR must not exist or be empty. The receipt OUT_DIR/receipt.json is RUNNING while it works and ends
COMPLETE (with a decision) or FAILED; the exit code is nonzero unless COMPLETE. The design below is frozen; only
--smoke (a tiny unregistered configuration, for software checks) changes it.
"""
import argparse
import hashlib
import json
import os
import platform
import sys
import time
import traceback
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import helike_berger_model as bm  # noqa: E402
import helike_condensate_fock as lf  # noqa: E402

DESIGN = {
    "primary": {"t": 3.0, "g2": [0.03, 0.05, 0.07], "s": [1.12, 1.15, 1.165, 1.18, 1.195, 1.22, 1.24]},
    "secondary": {"t": 4.0, "g2": [0.03, 0.05], "s": [1.09, 1.12, 1.14, 1.16, 1.18]},
    "ladder": [[6, 6], [6, 8], [8, 8]],                  # (Taylor order P, total quanta Mq)
    "convergence_tol": 0.003,                            # last-step change of each gap
    "weight_min": 0.9,                                   # single-quantum dominance of the identified levels
    "s_tolerance": 0.015,
    "predicted_s_star": {"3.0": {"0.03": 1.1674, "0.05": 1.1809, "0.07": 1.1931},
                         "4.0": {"0.03": 1.1260, "0.05": 1.1413}},
}
SMOKE = {"primary": {"t": 3.0, "g2": [0.05], "s": [1.15, 1.2]}, "secondary": None, "ladder": [[4, 4]],
         "convergence_tol": 1.0, "weight_min": 0.0, "s_tolerance": 1.0, "predicted_s_star": {"3.0": {"0.05": 1.1809}}}
SOURCES = ("helike_condensate_fock.py", "helike_condensate_pt.py", "helike_berger_model.py",
           "run_helike_flux_background.py", "test_helike_condensate_fock.py")


def sha(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()


def write(path, obj):
    tmp = path.with_suffix(".tmp")
    tmp.write_text(json.dumps(obj, indent=1, allow_nan=False))
    os.replace(tmp, path)


def switch_point(svals, diffs):
    """Linear interpolation of the first sign change of (gap_competitor - gap_helike) from - to +; None if absent."""
    for i in range(len(svals) - 1):
        if diffs[i] < 0 <= diffs[i + 1]:
            return svals[i] + (svals[i + 1] - svals[i]) * (-diffs[i]) / (diffs[i + 1] - diffs[i])
    return None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out_dir")
    ap.add_argument("--smoke", action="store_true")
    args = ap.parse_args()
    D = SMOKE if args.smoke else DESIGN
    out = Path(args.out_dir)
    if out.exists() and any(out.iterdir()):
        sys.exit(f"refusing to write into non-empty {out}")
    out.mkdir(parents=True, exist_ok=True)
    path = out / "receipt.json"
    rec = {"status": "RUNNING", "smoke": args.smoke, "design": D,
           "registration": "docs/research/helike-flux-background-registration-2026-11-24.md",
           "sources_sha256": {s: sha(HERE / s) for s in SOURCES}, "python": sys.version,
           "platform": platform.platform(), "started_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
           "certified": False, "cases": []}
    write(path, rec)
    try:
        for label in ("primary", "secondary"):
            block = D.get(label)
            if not block:
                continue
            t = block["t"]
            for g2 in block["g2"]:
                case = {"block": label, "t": t, "g2": g2, "points": []}
                rec["cases"].append(case)
                for s in block["s"]:
                    if s >= bm.mc_s_max(t):
                        raise ValueError(f"s = {s} outside the bounded window at t = {t}")
                    point = {"s": s, "depth_c_over_g2": -bm.mc_depth(s, t, np.sqrt(g2)),
                             "omega_helike": float(np.sqrt(bm.mc_helike_freq2(s, t))), "ladder": []}
                    for P, Mq in D["ladder"]:
                        t0 = time.time()
                        L = lf.LocalFock(s, t, np.sqrt(g2), P=P, Mq=Mq)
                        r = L.helike_and_competitor()
                        r.update({"P": P, "Mq": Mq, "dimension": len(L.basis), "asymmetry": L.asym,
                                  "seconds": round(time.time() - t0, 1)})
                        point["ladder"].append(r)
                        write(path, rec)
                        print(f"t={t} g2={g2} s={s} P={P} Mq={Mq}: helike {r['helike']['gap']:.5f} "
                              f"competitor {r['competitor']['gap']:.5f} ({r['seconds']}s)", flush=True)
                    last = point["ladder"][-1]
                    prev = point["ladder"][-2] if len(point["ladder"]) > 1 else None
                    point["converged"] = bool(prev is None or max(
                        abs(last["helike"]["gap"] - prev["helike"]["gap"]),
                        abs(last["competitor"]["gap"] - prev["competitor"]["gap"])) <= D["convergence_tol"])
                    point["identified"] = bool(min(last["helike"]["weight"], last["competitor"]["weight"]) >= D["weight_min"])
                    point["diff_competitor_minus_helike"] = last["competitor"]["gap"] - last["helike"]["gap"]
                    case["points"].append(point)
                    write(path, rec)
                sv = [p["s"] for p in case["points"]]
                dv = [p["diff_competitor_minus_helike"] for p in case["points"]]
                sq = switch_point(sv, dv)
                pred = D["predicted_s_star"][str(t)][str(g2)]
                ok_conv = all(p["converged"] and p["identified"] for p in case["points"])
                case["measured_switch"] = sq
                case["predicted_switch"] = pred
                if not ok_conv:
                    case["status"] = "UNRESOLVED"
                elif sq is None:
                    case["status"] = "NOT_SUPPORTED"
                else:
                    case["status"] = "SUPPORTED" if abs(sq - pred) <= D["s_tolerance"] else "NOT_SUPPORTED"
                write(path, rec)
        prim = [c for c in rec["cases"] if c["block"] == "primary"]
        sts = [c["status"] for c in prim]
        rec["decision"] = ("SUPPORTED" if all(s == "SUPPORTED" for s in sts) else
                           "UNRESOLVED" if any(s == "UNRESOLVED" for s in sts) else
                           "NOT_SUPPORTED" if all(s == "NOT_SUPPORTED" for s in sts) else "MIXED")
        rec["status"] = "COMPLETE"
    except BaseException as exc:
        rec["status"] = "FAILED"
        rec["error"] = "".join(traceback.format_exception(type(exc), exc, exc.__traceback__))
        write(path, rec)
        raise
    rec["finished_utc"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    write(path, rec)
    print(rec["status"], rec["decision"])
    sys.exit(0 if rec["status"] == "COMPLETE" else 1)


if __name__ == "__main__":
    main()
