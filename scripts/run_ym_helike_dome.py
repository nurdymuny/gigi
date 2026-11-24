"""Registered dome (second-envelope) test: docs/research/helike-dome-envelope-preregistration-2026-11-24.md.

usage: python scripts/run_ym_helike_dome.py OUT_DIR

Fail-closed: OUT_DIR must not exist or must be empty. The receipt (OUT_DIR/receipt.json) is RUNNING while it works,
then ends with status COMPLETE (all validity gates pass; the scientific status is in `decision`), INVALID (a gate
failed) or FAILED (an exception). The exit code is nonzero unless COMPLETE. Nothing in the design can be changed
from the command line except --smoke, which runs a tiny unregistered configuration for software checks only.
"""
import argparse
import hashlib
import json
import math
import os
import platform
import sys
import time
import traceback
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import ym_helike_dome as dm  # noqa: E402
import ym_helike_j0 as j0  # noqa: E402

DESIGN = {
    "cases": [[1.0, 1.7], [2.0, 2.3]],
    "cutoffs": [12, 14, 16, 18],
    "dome_sizes": [0, 3, 4],
    "primary_dome_size": 4,
    "beta_grid": [0.75, 1.0, 1.25, 1.5, 2.0, 2.5, 3.0, 4.0],
    "beta_selection_cutoff": 10,
    "K": 30, "K_sensitivity": 36, "dps": 80, "check_dps": 120,
}
SMOKE = dict(DESIGN, cases=[[1.0, 1.7]], cutoffs=[6], dome_sizes=[0, 4], beta_grid=[0.9, 1.1, 1.3], force_beta=1.1,
             beta_selection_cutoff=6, K=16, K_sensitivity=18, dps=60, check_dps=80)
LEVELS = ("vacuum", "scalar", "scalar_next")
SOURCES = ("ym_helike_dome.py", "ym_helike_j0.py", "run_ym_helike_dome.py", "test_ym_helike_dome.py")


def sha(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()


def write(path, obj):
    tmp = path.with_suffix(".tmp")
    tmp.write_text(json.dumps(obj, indent=1, allow_nan=False))
    os.replace(tmp, path)


def slim(sol):
    keys = ("dimension", "n_origin", "n_dome", "beta", "n_d", "K", "dps", "gram_digits_lost", "energies",
            "residuals", "outside_origin_fraction", "min_exterior_eig")
    return {k: sol[k] for k in keys if k in sol}


def gains(base, aug, nxt):
    n_dome, dn = aug["n_dome"], nxt["dimension"] - base["dimension"]
    out = {}
    for i, lev in enumerate(LEVELS):
        e0, ed, en = base["residuals"][i], aug["residuals"][i], nxt["residuals"][i]
        out[lev] = {"eps_origin": e0, "eps_dome": ed, "eps_next_cutoff": en,
                    "r_dome": math.log(e0 / ed) / n_dome, "r_orig": math.log(e0 / en) / dn,
                    "outside_origin_fraction": aug["outside_origin_fraction"][i]}
    out["H1a"] = out["scalar"]["r_dome"] > out["scalar"]["r_orig"]
    out["H1b"] = out["scalar"]["r_dome"] > out["vacuum"]["r_dome"]
    out["n_dome"], out["delta_n_origin"] = n_dome, dn
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out_dir")
    ap.add_argument("--smoke", action="store_true", help="tiny unregistered software check")
    args = ap.parse_args()
    D = SMOKE if args.smoke else DESIGN
    out = Path(args.out_dir)
    if out.exists() and any(out.iterdir()):
        sys.exit(f"refusing to write into non-empty {out}")
    out.mkdir(parents=True, exist_ok=True)
    path = out / "receipt.json"
    rec = {"status": "RUNNING", "registration": "docs/research/helike-dome-envelope-preregistration-2026-11-24.md",
           "smoke": args.smoke, "design": D, "sources_sha256": {s: sha(HERE / s) for s in SOURCES},
           "started_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), "python": sys.version,
           "platform": platform.platform(), "certified": False, "cases": []}
    write(path, rec)
    gates = {"D00": True, "D01": True, "D02": True, "D03": True, "D04": True, "D05": True}
    try:
        for g2, alpha in D["cases"]:
            case = {"g2": g2, "alpha": alpha}
            rec["cases"].append(case)
            sel = {}
            for beta in D["beta_grid"]:
                s = dm.solve(D["beta_selection_cutoff"], g2, alpha, n_d=0, beta=beta, K=D["K"], dps=D["dps"])
                sel[str(beta)] = s["energies"][1]
            best = min(D["beta_grid"], key=lambda b: (sel[str(b)], b))
            edge = best in (D["beta_grid"][0], D["beta_grid"][-1])
            if "force_beta" in D:            # smoke only: exercise the pipeline without a selection
                best, edge = D["force_beta"], False
            case["beta_selection"] = {"second_ritz_value": sel, "beta": best, "edge": edge}
            write(path, rec)
            print(f"g2={g2}: beta selected", flush=True)
            if case["beta_selection"]["edge"]:
                case["refused"] = "BETA_AT_GRID_EDGE"
                continue
            beta = best
            case["points"] = []
            for N in D["cutoffs"]:
                base = dm.solve(N, g2, alpha, dps=D["dps"])
                nxt = dm.solve(N + 2, g2, alpha, dps=D["dps"])
                ref = j0.solve(N, g2, alpha, dps=60)
                d00 = (max(abs(a - b) for a, b in zip(base["energies"], ref["energies"])) <= 1e-12
                       and max(abs(a - b) for a, b in zip(base["residuals"], ref["residuals"])) <= 1e-12)
                gates["D00"] &= d00
                point = {"cutoff": N, "origin": slim(base), "next_cutoff": slim(nxt), "D00": d00, "dome": {}}
                for n_d in D["dome_sizes"]:
                    aug = dm.solve(N, g2, alpha, n_d=n_d, beta=beta, K=D["K"], dps=D["dps"])
                    d01 = all(a <= b + 1e-12 for a, b in zip(aug["energies"][:3], base["energies"][:3]))
                    d05 = all(-1e-12 <= f <= 1 + 1e-12 for f in aug["outside_origin_fraction"])
                    gates["D01"] &= d01
                    gates["D05"] &= d05
                    point["dome"][str(n_d)] = {"solution": slim(aug), "comparison": gains(base, aug, nxt),
                                               "D01": d01, "D05": d05}
                if N == D["cutoffs"][-1]:
                    nd = D["primary_dome_size"]
                    augK = dm.solve(N, g2, alpha, n_d=nd, beta=beta, K=D["K_sensitivity"], dps=D["dps"])
                    cK = gains(base, augK, nxt)
                    c0 = point["dome"][str(nd)]["comparison"]
                    d04 = (cK["H1a"], cK["H1b"]) == (c0["H1a"], c0["H1b"])
                    gates["D04"] &= d04
                    point["K_sensitivity"] = {"solution": slim(augK), "comparison": cK, "D04": d04}
                    hi_b = dm.solve(N, g2, alpha, dps=D["check_dps"])
                    hi_a = dm.solve(N, g2, alpha, n_d=nd, beta=beta, K=D["K"], dps=D["check_dps"])
                    aug0 = point["dome"][str(nd)]["solution"]
                    dE = max(max(abs(a - b) for a, b in zip(hi_b["energies"], base["energies"])),
                             max(abs(a - b) for a, b in zip(hi_a["energies"], aug0["energies"])))
                    dR = max(max(abs(a - b) / b for a, b in zip(hi_b["residuals"], base["residuals"])),
                             max(abs(a - b) / b for a, b in zip(hi_a["residuals"], aug0["residuals"])))
                    d03 = dE <= 1e-11 and dR <= 1e-6
                    gates["D03"] &= d03
                    point["precision_check"] = {"dps": D["check_dps"], "max_abs_dE": dE, "max_rel_dR": dR,
                                                "D03": d03}
                case["points"].append(point)
                write(path, rec)
                print(f"g2={g2} N={N} done", flush=True)
        primary = [p["dome"][str(D["primary_dome_size"])]["comparison"]
                   for c in rec["cases"] for p in c.get("points", [])]
        refused = [c for c in rec["cases"] if "refused" in c]
        if refused or not primary:
            decision = "REFUSED_BETA_EDGE"
        elif not all(gates.values()):
            decision = "INVALID"
        elif not gates["D04"]:
            decision = "UNRESOLVED_TRUNCATION"
        elif all(c["H1a"] and c["H1b"] for c in primary):
            decision = "SUPPORTED"
        elif not any(c["H1a"] for c in primary):
            decision = "NOT_SUPPORTED"
        else:
            decision = "MIXED"
        rec["gates"] = gates
        rec["decision"] = decision
        rec["status"] = "COMPLETE" if all(gates.values()) and not refused else "INVALID"
    except BaseException as exc:
        rec["status"] = "FAILED"
        rec["error"] = "".join(traceback.format_exception(type(exc), exc, exc.__traceback__))
        write(path, rec)
        raise
    rec["finished_utc"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    write(path, rec)
    print(rec["status"], rec.get("decision"), rec["gates"])
    sys.exit(0 if rec["status"] == "COMPLETE" else 1)


if __name__ == "__main__":
    main()
