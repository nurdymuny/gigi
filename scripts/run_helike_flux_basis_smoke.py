"""One registered diagnostic, not a replacement or release of the flux-grid experiment."""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import subprocess
import time

import run_helike_flux_grid as grid

ROOT = Path(__file__).resolve().parents[1]
POINT = {"s": 1.18, "t": 3.0, "g2": 0.03}
SOLVER = {"nev": 12, "block": 4, "tol": 1e-10, "max_dim": 1600}
STEP_TOL, WEIGHT_MIN, EXPORT_MIN = 0.003, 0.9, 0.95
BASELINE = ROOT / "research/flux-grid-20261126-gate57/receipt.json"
BASELINE_SHA = "f1c880bb1bb7f025dc8c29a60fbc346746cdbe279ba655c5b2d2a9e96df9081d"


def finite(value):
    if isinstance(value, bool) or not isinstance(value, (float, int)) or not math.isfinite(value):
        raise ValueError("nonfinite or nonnumeric measurement")
    return value


def validate(raw, mq):
    expected = {**POINT, "Mq": mq, "extra": 4, "solver": SOLVER}
    if raw["status"] != "COMPUTED" or raw["params"] != expected:
        raise ValueError("point, cutoff or numerical settings do not match registration")
    r = raw["results"]
    if any(r[k] != v for k, v in {**POINT, "Mq": mq, "dimension": math.comb(mq + 6, 6),
                                  "moments": "exact"}.items()):
        raise ValueError("result point, cutoff, dimension or operator mismatch")
    finite(r["quadrature_cross_check"])
    q = r["result"]
    levels, residuals = q["levels"], q["in_space_lanczos_residuals"]
    if len(levels) != SOLVER["nev"] or len(residuals) != len(levels):
        raise ValueError("incomplete Ritz levels or residuals")
    for value, residual in zip(levels, residuals):
        if not 0 <= finite(residual) <= SOLVER["tol"] * max(1.0, abs(finite(value))):
            raise ValueError("finite-space Lanczos residual exceeds frozen solver tolerance")
    if abs(finite(q["E0"]) - levels[0]) > 1e-10:
        raise ValueError("ground energy inconsistent with levels")
    z = finite(q["vacuum_weight"])
    if not 0 < z <= 1 + 1e-6 or not 0 <= finite(q["reference_weight_error"]) <= 1e-6:
        raise ValueError("reference weights unresolved")
    for name in ("helike", "competitor"):
        c = q[name]
        gap, weight, dressed = map(finite, (c["gap"], c["weight"], c["dressed_weight"]))
        i = c["level_index"]
        if type(i) is not int or not 0 < i < len(levels) or gap < 0:
            raise ValueError("invalid excitation index or gap")
        if abs(levels[i] - levels[0] - gap) > 1e-10:
            raise ValueError("gap inconsistent with selected level")
        if not 0 <= weight <= 1 + 1e-6 or dressed < 0 or abs(dressed * z - weight) > 1e-9:
            raise ValueError("inconsistent reference weight / vacuum weight")
    return q


def reproduce(raw, old):
    q = validate(raw, 10)
    if old["Mq"] != 10:
        raise ValueError("baseline cutoff is not 10")
    for name in ("helike", "competitor"):
        for key, tol in (("gap", 1e-8), ("weight", 1e-6), ("dressed_weight", 1e-6)):
            if abs(q[name][key] - finite(old[name][key])) > tol:
                raise ValueError(f"Mq 10 does not reproduce {name}.{key}")
    return q


def assess(low, high, old):
    a, b = reproduce(low, old), validate(high, 12)
    steps = {n: abs(b[n]["gap"] - a[n]["gap"]) for n in ("helike", "competitor")}
    return {"steps": steps, "basis_step_pass": max(steps.values()) <= STEP_TOL,
            "grid_identification_pass": min(b[n]["dressed_weight"] for n in steps) >= WEIGHT_MIN,
            "competitor_blocks_export": b["competitor"]["dressed_weight"] < EXPORT_MIN,
            "domain_gate": "NOT_RUN", "overall": "UNRESOLVED", "certified": False,
            "full_grid_released": False}


def new_run(path):
    path.mkdir(parents=True, exist_ok=False)


def execute(binary, out, mq):
    receipt = out / f"mq{mq}.json"
    command = [str(binary), "--out", str(receipt), "cone", "--s", str(POINT["s"]),
               "--t", str(POINT["t"]), "--g2", str(POINT["g2"]), "--mq", str(mq)]
    event = {"Mq": mq, "command": command, "started_unix": time.time(), "status": "RUNNING"}
    grid.save(out / f"mq{mq}-execution.json", event)
    print(f"Mq {mq} started: " + " ".join(command), flush=True)
    try:
        with (out / f"mq{mq}.stdout.log").open("w") as stdout, (out / f"mq{mq}.stderr.log").open("w") as stderr:
            p = subprocess.run(command, cwd=ROOT, env={**os.environ, "RAYON_NUM_THREADS": "8"},
                               stdout=stdout, stderr=stderr, timeout=900, check=False)
        event["returncode"] = p.returncode
        if p.returncode != 0:
            raise RuntimeError(f"solver exited {p.returncode}; see mq{mq}.stderr.log")
        raw = json.loads(receipt.read_text())
        validate(raw, mq)
        event["status"] = "COMPLETED"
        return raw
    except BaseException as exc:
        event.update(status="STOPPED", error=f"{type(exc).__name__}: {exc}")
        raise
    finally:
        event["elapsed_seconds"] = time.time() - event["started_unix"]
        grid.save(out / f"mq{mq}-execution.json", event)
        print(json.dumps(event), flush=True)


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("out_dir", type=Path)
    args = ap.parse_args()
    out = args.out_dir.resolve()
    new_run(out)
    binary = grid.HELIKE_BIN
    rec = {"status": "RUNNING", "point": POINT, "ladder": [10, 12], "certified": False,
           "full_grid_released": False, "overall": "UNRESOLVED", "domain_gate": "NOT_RUN", "rungs": []}
    grid.save(out / "receipt.json", rec)
    try:
        if grid.sha(BASELINE) != BASELINE_SHA:
            raise ValueError("historical baseline digest mismatch")
        history = json.loads(BASELINE.read_text())
        case = next(c for c in history["cases"] if c["t"] == POINT["t"] and c["g2"] == POINT["g2"])
        point = next(p for p in case["points"] if p["s"] == POINT["s"])
        old = point["ladder"][-1]
        revision = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
        rec.update(source_revision=revision, binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
                   baseline_sha256_lf=BASELINE_SHA,
                   source_status=subprocess.check_output(["git", "status", "--short"], cwd=ROOT, text=True),
                   harness_sha256_lf=grid.sha(Path(__file__)))
        grid.save(out / "receipt.json", rec)
        rows = []
        for mq in (10, 12):
            raw = execute(binary, out, mq)
            if mq == 10:
                reproduce(raw, old)
            rows.append(raw)
            rec["rungs"].append({"Mq": mq, "receipt": f"mq{mq}.json", "result": raw["results"]})
            grid.save(out / "receipt.json", rec)
            q = raw["results"]["result"]
            print(json.dumps({"Mq": mq, "helike": q["helike"], "competitor": q["competitor"]}), flush=True)
        rec["assessment"] = assess(*rows, old)
        previous_step = abs(old["competitor"]["gap"] - point["ladder"][-2]["competitor"]["gap"])
        rec["assessment"]["competitor_step_ratio"] = rec["assessment"]["steps"]["competitor"] / previous_step
        rec["status"] = "COMPLETE"
    except BaseException as exc:
        rec.update(status="STOPPED", error=f"{type(exc).__name__}: {exc}")
        print(rec["error"], flush=True)
    finally:
        grid.save(out / "receipt.json", rec)
    print(json.dumps({k: rec[k] for k in ("status", "overall", "full_grid_released")}), flush=True)
    return 0 if rec["status"] == "COMPLETE" else 2


if __name__ == "__main__":
    raise SystemExit(main())
