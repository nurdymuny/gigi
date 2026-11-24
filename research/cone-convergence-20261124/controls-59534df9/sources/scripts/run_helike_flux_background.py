"""Registered run: docs/research/helike-flux-background-registration-2026-11-24.md (the sourced condensate).

usage: python scripts/run_helike_flux_background.py OUT_DIR [--smoke]

Fail-closed: OUT_DIR must not exist or be empty. The receipt OUT_DIR/receipt.json is RUNNING while it works and ends
COMPLETE (diagnostic smoke only), HOLD_DOMAIN (registered run not evaluated), or FAILED.
The design below is frozen. See the review amendment for the domain hold and separate refinement checks.
--verify replays receipt consistency, not the underlying matrix calculation.
"""
import argparse
import copy
import hashlib
import json
import os
import platform
import shutil
import subprocess
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
ROOT = HERE.parent
REGISTRATION = "docs/research/helike-flux-background-registration-2026-11-24.md"
AMENDMENT = "docs/research/helike-flux-review-amendment-2026-11-24.md"
SOURCES = tuple("scripts/" + s for s in (
    "helike_condensate_fock.py", "helike_condensate_pt.py", "helike_berger_model.py", "ym_two_grain_molien.py",
    "run_helike_flux_background.py", "test_helike_condensate_fock.py", "test_flux_review.py")) + (REGISTRATION, AMENDMENT)
REPLAY_SCOPE = "receipt consistency only, not matrix replay or physical certification"
HASH_POLICY = "sha256-universal-newline-v1"


def sha(p):
    """Text evidence digest independent of Git's LF/CRLF checkout convention."""
    data = Path(p).read_bytes().replace(b"\r\n", b"\n").replace(b"\r", b"\n")
    return hashlib.sha256(data).hexdigest()


def write(path, obj):
    tmp = path.with_suffix(".tmp")
    with tmp.open("w", encoding="utf-8") as f:
        f.write(json.dumps(obj, indent=1, allow_nan=False))
        f.flush()
        os.fsync(f.fileno())
    os.replace(tmp, path)


def save_receipt(path, rec):
    write(path, rec)
    # Integrity receipt, not a signature. A crash between writes fails closed on replay.
    write(path.with_name("receipt-integrity.json"), {"sha256": sha(path)})


def switch_point(svals, diffs):
    """Unique bracketed -/+ crossing; reject ambiguous, unordered or incomplete evidence."""
    s, d = np.asarray(svals, dtype=float), np.asarray(diffs, dtype=float)
    if s.ndim != 1 or d.shape != s.shape or len(s) < 2 or not np.isfinite(s).all() or not np.isfinite(d).all():
        raise ValueError("crossing requires matching finite coordinate and difference arrays of length >= 2")
    if not np.all(np.diff(s) > 0):
        raise ValueError("crossing coordinates must be strictly increasing")
    nz = np.flatnonzero(d)
    if len(nz) != len(d):
        zeros = np.flatnonzero(d == 0)
        if len(zeros) != 1 or zeros[0] in (0, len(d)-1) or d[zeros[0]-1]*d[zeros[0]+1] >= 0:
            raise ValueError("zero differences need a unique interior sign-change bracket")
    changes = [(a, b) for a, b in zip(nz[:-1], nz[1:]) if np.sign(d[a]) != np.sign(d[b])]
    if len(changes) > 1:
        raise ValueError("multiple crossings: one interpolated switch would be ambiguous")
    if changes:
        a, b = changes[0]
        if d[a] < 0:
            if b > a + 1:
                return float(s[a+1])
            return float(s[a] + (s[b]-s[a]) * (-d[a]) / (d[b]-d[a]))
    return None


def check_rung(r):
    values = [r["E0"], r["asymmetry"], r["seconds"]]
    for name in ("helike", "competitor"):
        gap, weight = r[name]["gap"], r[name]["weight"]
        values.extend((gap, weight))
        if gap < 0 or not 0 <= weight <= 1 + 1e-10:
            raise ValueError("invalid excitation gap or single-quantum weight")
    if not np.isfinite(values).all() or r["asymmetry"] < 0 or r["seconds"] < 0:
        raise ValueError("nonfinite or invalid rung diagnostics")
    if not isinstance(r["dimension"], int) or r["dimension"] < 1:
        raise ValueError("invalid matrix dimension")


def assess_point(point, D):
    rows = point["ladder"]
    if not rows or [[r["P"], r["Mq"]] for r in rows] != D["ladder"]:
        raise ValueError("incomplete or reordered refinement ladder")
    for r in rows:
        check_rung(r)
    deltas = {"basis": [], "taylor": []}
    for a, b in zip(rows[:-1], rows[1:]):
        if a["P"] == b["P"] and a["Mq"] < b["Mq"]:
            kind = "basis"
        elif a["Mq"] == b["Mq"] and a["P"] < b["P"]:
            kind = "taylor"
        else:
            raise ValueError("refinement must change exactly one cutoff upwards")
        deltas[kind].append(max(abs(a[n]["gap"]-b[n]["gap"]) for n in ("helike", "competitor")))
    last = rows[-1]
    return {
        "refinement_changes": deltas,
        "converged": bool(all(deltas.values()) and all(v <= D["convergence_tol"] for ds in deltas.values() for v in ds)),
        "identified": bool(min(last[n]["weight"] for n in ("helike", "competitor")) >= D["weight_min"]),
        "diff_competitor_minus_helike": last["competitor"]["gap"]-last["helike"]["gap"],
    }


def expected_cases(D):
    primary = D.get("primary")
    if not primary or not primary["g2"] or len(primary["s"]) < 2 or not D["ladder"]:
        raise ValueError("design requires nonempty primary cases, coordinates and ladder")
    return [(label, block["t"], g2, block["s"]) for label in ("primary", "secondary")
            if (block := D.get(label)) for g2 in block["g2"]]


def assess_case(case, D):
    points = case["points"]
    measured = switch_point([p["s"] for p in points], [p["diff_competitor_minus_helike"] for p in points])
    return {"measured_switch": measured, "predicted_switch": D["predicted_s_star"][str(case["t"])][str(case["g2"])],
            "status": "DIAGNOSTIC_ONLY"}


def verify_receipt(out):
    report_path = out / "verification.json"
    report = {"status": "RUNNING", "scope": REPLAY_SCOPE}
    write(report_path, report)
    try:
        path = out / "receipt.json"
        if sha(path) != json.loads((out / "receipt-integrity.json").read_text())["sha256"]:
            raise ValueError("receipt integrity mismatch")
        rec = json.loads(path.read_text())
        if rec["hash_policy"] != HASH_POLICY:
            raise ValueError("unsupported text integrity policy")
        if rec["status"] != "COMPLETE" or rec["decision"] != "SOFTWARE_SMOKE_ONLY" or not rec["smoke"] or rec["certified"]:
            raise ValueError("only complete, uncertified software smoke receipts can pass replay")
        if rec["design"] != SMOKE or rec["registration"] != REGISTRATION or rec["amendment"] != AMENDMENT:
            raise ValueError("design or contract differs from the reviewed smoke configuration")
        if set(rec["sources_sha256"]) != set(SOURCES):
            raise ValueError("incomplete source manifest")
        for s in SOURCES:
            h = rec["sources_sha256"][s]
            if sha(out / "sources" / s) != h or sha(ROOT / s) != h:
                raise ValueError(f"changed source or snapshot: {s}")
        expected = expected_cases(SMOKE)
        if len(rec["cases"]) != len(expected):
            raise ValueError("missing or extra cases")
        for case, (label, t, g2, coords) in zip(rec["cases"], expected):
            if (case["block"], case["t"], case["g2"], [p["s"] for p in case["points"]]) != (label, t, g2, coords):
                raise ValueError("case identity or coordinate coverage differs")
            for point in case["points"]:
                for key, val in assess_point(point, SMOKE).items():
                    if point[key] != val:
                        raise ValueError(f"point assessment mismatch: {key}")
                if point["depth_c_over_g2"] != -bm.mc_depth(point["s"], t, np.sqrt(g2)) or \
                        point["omega_helike"] != float(np.sqrt(bm.mc_helike_freq2(point["s"], t))):
                    raise ValueError("background diagnostic mismatch")
            for key, val in assess_case(case, SMOKE).items():
                if case[key] != val:
                    raise ValueError(f"case assessment mismatch: {key}")
        report.update(status="PASS", receipt_sha256=sha(path))
    except Exception as exc:
        report.update(status="FAIL", error=f"{type(exc).__name__}: {exc}")
    write(report_path, report)
    print(report["status"], REPLAY_SCOPE)
    return 0 if report["status"] == "PASS" else 1


RELEASE_CONDITIONS = (
    ("convergence", "positive-cone controls CONVERGED at Mq 6->8 under 0.003 (run_helike_cone_convergence.py receipt)"),
    ("certificate", "validated interval quasimode certificate at the controls; verifier not implemented, condition held"),
    ("method_amendment", "registered method amended from LocalFock to the certified cone method; LocalFock is "
                         "not semibounded and cannot be released for the grid under any receipt"),
)


def unmet_release_conditions(convergence=None, certificate=None):
    """Names of release conditions not met. The method amendment is never met by this runner."""
    unmet = []
    try:
        from run_helike_cone_convergence import read_valid_receipt
        rec = read_valid_receipt(Path(convergence)) if convergence else None
        ok = rec is not None and rec["decision"] == "CONVERGED" and rec["source_dirty"] is False
    except (OSError, ValueError, TypeError, KeyError, OverflowError, subprocess.SubprocessError):
        ok = False
    if not ok:
        unmet.append("convergence")
    # No implemented certificate verifier: even a plausible status string proves nothing.
    unmet.append("certificate")
    unmet.append("method_amendment")
    return unmet


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--convergence", help="receipt.json from run_helike_cone_convergence.py")
    ap.add_argument("--certificate", help="receipt.json from the quasimode certification run")
    ap.add_argument("out_dir")
    ap.add_argument("--smoke", action="store_true")
    ap.add_argument("--verify", action="store_true")
    args = ap.parse_args()
    D = copy.deepcopy(SMOKE if args.smoke else DESIGN)
    out = Path(args.out_dir)
    if args.verify:
        sys.exit(verify_receipt(out))
    if out.exists() and any(out.iterdir()):
        sys.exit(f"refusing to write into non-empty {out}")
    out.mkdir(parents=True, exist_ok=True)
    path = out / "receipt.json"
    rec = {"status": "RUNNING", "smoke": args.smoke, "design": D, "hash_policy": HASH_POLICY,
           "registration": REGISTRATION, "amendment": AMENDMENT,
           "sources_sha256": {s: sha(ROOT / s) for s in SOURCES}, "python": sys.version,
           "platform": platform.platform(), "started_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
           "certified": False, "decision": "NOT_EVALUATED", "cases": []}
    save_receipt(path, rec)
    try:
        rec["revision"] = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
        rec["source_dirty"] = bool(subprocess.check_output(["git", "status", "--porcelain", "--", *SOURCES], cwd=ROOT, text=True).strip())
        for s in SOURCES:
            dest = out / "sources" / s
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / s, dest)
            if sha(dest) != rec["sources_sha256"][s]:
                raise ValueError(f"source changed during snapshot: {s}")
        if not args.smoke:
            unmet = unmet_release_conditions(args.convergence, args.certificate)
            rec.update(status="HOLD_DOMAIN", reason="positive-cone domain not enforced; see review amendment",
                       hold_conditions={k: d for k, d in RELEASE_CONDITIONS if k in unmet})
            save_receipt(path, rec)
            print("HOLD_DOMAIN NOT_EVALUATED; unmet: " + ", ".join(unmet))
            return 2
        expected_cases(D)
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
                    case["points"].append(point)
                    save_receipt(path, rec)
                    for P, Mq in D["ladder"]:
                        t0 = time.time()
                        L = lf.LocalFock(s, t, np.sqrt(g2), P=P, Mq=Mq)
                        r = L.helike_and_competitor()
                        r.update({"P": P, "Mq": Mq, "dimension": len(L.basis), "asymmetry": L.asym,
                                  "seconds": round(time.time() - t0, 1)})
                        check_rung(r)
                        point["ladder"].append(r)
                        save_receipt(path, rec)
                        print(f"t={t} g2={g2} s={s} P={P} Mq={Mq}: helike {r['helike']['gap']:.5f} "
                              f"competitor {r['competitor']['gap']:.5f} ({r['seconds']}s)", flush=True)
                    point.update(assess_point(point, D))
                    save_receipt(path, rec)
                case.update(assess_case(case, D))
                save_receipt(path, rec)
        rec["decision"] = "SOFTWARE_SMOKE_ONLY"
        rec["status"] = "COMPLETE"
    except BaseException as exc:
        rec["status"] = "FAILED"
        rec["error"] = "".join(traceback.format_exception(type(exc), exc, exc.__traceback__))
        save_receipt(path, rec)
        raise
    rec["finished_utc"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    save_receipt(path, rec)
    print(rec["status"], rec["decision"])
    sys.exit(0 if rec["status"] == "COMPLETE" else 1)


if __name__ == "__main__":
    sys.exit(main())
