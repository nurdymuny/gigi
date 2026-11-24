"""Frozen positive-cone controls, Mq 6 -> 8. Run only after review on the large machine.

python scripts/run_helike_cone_convergence.py OUT_DIR [--verify]
Verification checks receipt consistency, not matrices, authenticity or physical certification.
The earlier Mq 4 -> 6 failure remains on record. No threshold or model changes here.
"""
import argparse
import json
import math
import platform
import shutil
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
import helike_positive_cone as pc  # noqa: E402
from run_helike_flux_background import HASH_POLICY, REPLAY_SCOPE, save_receipt, sha, write  # noqa: E402

S, T, G2S, LADDER = 1.15, 3.0, (0.004, 0.002), (6, 8)
GAP_TOL, WEIGHT_MIN = 0.003, 0.95
SCHEMA = "cone-convergence-v1"
CHANNELS = ("helike", "competitor")
ROW_SCOPE = "positive-cone interior trial values; no boundary or sector convergence certificate"
SOURCES = tuple("scripts/" + s for s in (
    "helike_positive_cone.py", "helike_condensate_fock.py", "helike_condensate_pt.py",
    "helike_berger_model.py", "ym_two_grain_molien.py", "run_helike_cone_convergence.py",
    "run_helike_flux_background.py")) + (
    "docs/research/helike-quasimode-certification-spec-2026-11-24.md",
    "docs/research/helike-quasimode-evidence-amendment-2026-11-24.md")


def number(value):
    if type(value) not in (int, float) or not math.isfinite(value):
        raise ValueError("measurement must be a finite number, not bool")
    return value


def finite_tree(value):
    if isinstance(value, float) and not math.isfinite(value):
        raise ValueError("nonfinite evidence")
    if isinstance(value, dict):
        for child in value.values():
            finite_tree(child)
    elif isinstance(value, list):
        for child in value:
            finite_tree(child)


def strict_json(path):
    def pairs(items):
        result = {}
        for key, value in items:
            if key in result:
                raise ValueError(f"duplicate JSON key: {key}")
            result[key] = value
        return result
    def constant(value):
        raise ValueError(f"nonfinite JSON constant: {value}")
    result = json.loads(Path(path).read_text(encoding="utf-8"), object_pairs_hook=pairs,
                        parse_constant=constant)
    finite_tree(result)
    if not isinstance(result, dict):
        raise ValueError("evidence must be a JSON object")
    return result


def check_rung(r, m):
    if not isinstance(r, dict) or "error" in r:
        raise ValueError("failed or malformed rung")
    finite_tree(r)
    if type(r["dimension"]) is not int or r["dimension"] != math.comb(m+6, 6):
        raise ValueError("matrix dimension differs from the registered six-mode basis")
    for key in ("E0", "energy_shift", "seconds", "asymmetry", "reference_weight_error"):
        number(r[key])
    if min(r["seconds"], r["asymmetry"], r["reference_weight_error"]) < 0:
        raise ValueError("negative diagnostic")
    if r["reference_weight_error"] > 1e-6 or r["overlap_orders"] != [10, 12]:
        raise ValueError("reference weights unresolved at the frozen overlap rule")
    if r["boundary_status"] != "UNRESOLVED_PHYSICAL_REALIZATION" or r["scope"] != ROW_SCOPE:
        raise ValueError("interior trial evidence cannot claim physical certification")
    for ch in CHANNELS:
        entry = r[ch]
        if number(entry["gap"]) < 0:
            raise ValueError("negative excitation gap")
        for key in ("weight", "log_basis_weight"):
            if not 0 <= number(entry[key]) <= 1 + 1e-6:
                raise ValueError("invalid reference weight")
        if type(entry["level_index"]) is not int or not 1 <= entry["level_index"] < 12:
            raise ValueError("invalid selected level index")


def check_rows(rows):
    if not isinstance(rows, dict) or set(rows) != set(G2S):
        raise ValueError("exactly both registered couplings are required")
    for g2 in G2S:
        by = rows[g2]
        if not isinstance(by, dict) or set(by) != set(LADDER):
            raise ValueError("exactly both registered refinement rungs are required")
        for m in LADDER:
            check_rung(by[m], m)


def decide(rows):
    """Malformed/incomplete measurements cannot converge; valid negative results stay distinct."""
    try:
        check_rows(rows)
    except (ValueError, TypeError, KeyError, OverflowError):
        return "UNRESOLVED", {}
    changes = {g: {ch: abs(rows[g][8][ch]["gap"]-rows[g][6][ch]["gap"])
                   for ch in CHANNELS} for g in G2S}
    if any(rows[g][m][ch]["weight"] < WEIGHT_MIN for g in G2S for m in LADDER for ch in CHANNELS):
        return "UNRESOLVED", changes
    if any(d > GAP_TOL for by in changes.values() for d in by.values()):
        return "NOT_CONVERGED", changes
    return "CONVERGED", changes


def provenance():
    return dict(revision=subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
                source_dirty=bool(subprocess.check_output(
                    ["git", "status", "--porcelain", "--", *SOURCES], cwd=ROOT, text=True).strip()))


def read_valid_receipt(path):
    """Read and recheck, never trust a previous PASS. Replay requires the pinned source checkout."""
    path = Path(path)
    if path.name != "receipt.json":
        raise ValueError("expected the runner's receipt.json and sibling evidence")
    integrity = strict_json(path.with_name("receipt-integrity.json"))
    if integrity != {"sha256": sha(path)}:
        raise ValueError("receipt integrity mismatch")
    rec = strict_json(path)
    contract = dict(schema=SCHEMA, status="COMPLETE", scope=REPLAY_SCOPE, hash_policy=HASH_POLICY,
                    s=S, t=T, g2=list(G2S), ladder=list(LADDER), gap_tol=GAP_TOL, weight_min=WEIGHT_MIN)
    for key, expected in contract.items():
        if type(rec[key]) is not type(expected) or rec[key] != expected:
            raise ValueError(f"changed contract: {key}")
    if type(rec["source_dirty"]) is not bool or any(rec[k] != v for k, v in provenance().items()):
        raise ValueError("revision or dirty-source provenance differs from this pinned checkout")
    for key in ("python", "platform", "started_utc", "finished_utc"):
        if not isinstance(rec[key], str) or not rec[key]:
            raise ValueError(f"missing run metadata: {key}")
    if set(rec["sources_sha256"]) != set(SOURCES):
        raise ValueError("incomplete source manifest")
    for source in SOURCES:
        digest = rec["sources_sha256"][source]
        if sha(path.parent/"sources"/source) != digest or sha(ROOT/source) != digest:
            raise ValueError(f"changed source or snapshot: {source}")
    raw = rec["rows"]
    if not isinstance(raw, dict) or set(raw) != {str(g) for g in G2S}:
        raise ValueError("missing or extra control")
    rows = {}
    for g in G2S:
        by = raw[str(g)]
        if not isinstance(by, dict) or set(by) != {str(m) for m in LADDER}:
            raise ValueError("missing or extra rung")
        rows[g] = {m: by[str(m)] for m in LADDER}
    check_rows(rows)
    decision, changes = decide(rows)
    expected_changes = {str(k): v for k, v in changes.items()}
    # Compare JSON types too: False must not masquerade as a measured zero change.
    if rec["decision"] != decision or json.dumps(rec["changes"], sort_keys=True) != json.dumps(expected_changes, sort_keys=True):
        raise ValueError("forged decision or refinement changes")
    return rec


def verify_receipt(out):
    report_path = out/"verification.json"
    report = dict(status="RUNNING", scope=REPLAY_SCOPE)
    write(report_path, report)
    try:
        rec = read_valid_receipt(out/"receipt.json")
        report.update(status="PASS", decision=rec["decision"], receipt_sha256=sha(out/"receipt.json"))
    except Exception as exc:
        report.update(status="FAIL", error=f"{type(exc).__name__}: {exc}")
    write(report_path, report)
    print(report["status"], REPLAY_SCOPE, flush=True)
    return 0 if report["status"] == "PASS" else 1


def utc():
    return time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out_dir")
    ap.add_argument("--verify", action="store_true")
    a = ap.parse_args()
    out = Path(a.out_dir)
    if a.verify:
        return verify_receipt(out)
    if out.exists() and any(out.iterdir()):
        sys.exit(f"refusing to write into non-empty {out}")
    out.mkdir(parents=True, exist_ok=True)
    path = out/"receipt.json"
    rec = dict(schema=SCHEMA, status="RUNNING", decision="UNRESOLVED", scope=REPLAY_SCOPE,
               hash_policy=HASH_POLICY, s=S, t=T, g2=list(G2S), ladder=list(LADDER), gap_tol=GAP_TOL,
               weight_min=WEIGHT_MIN, python=sys.version, platform=platform.platform(),
               started_utc=utc(), rows={}, changes={})
    save_receipt(path, rec)
    rows = {}
    try:
        rec.update(provenance())
        rec["sources_sha256"] = {s: sha(ROOT/s) for s in SOURCES}
        for source in SOURCES:
            dest = out/"sources"/source
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT/source, dest)
            if sha(dest) != rec["sources_sha256"][source]:
                raise ValueError("source changed during snapshot")
        save_receipt(path, rec)
        for g2 in G2S:
            for m in LADDER:
                rec["active_case"] = dict(g2=g2, Mq=m)
                save_receipt(path, rec)
                print(f"START g2={g2} Mq={m}", flush=True)
                t0 = time.monotonic()
                solver = pc.ConeRitz(S, T, math.sqrt(g2), Mq=m, max_Mq=max(LADDER))
                r = solver.helike_and_competitor()
                r.update(dimension=len(solver.basis), asymmetry=solver.asym, seconds=time.monotonic()-t0)
                check_rung(r, m)
                rows.setdefault(g2, {})[m] = r
                rec["rows"] = {str(g): {str(n): v for n, v in by.items()} for g, by in rows.items()}
                save_receipt(path, rec)
                print(f"DONE g2={g2} Mq={m}: helike {r['helike']['gap']:.10f} "
                      f"competitor {r['competitor']['gap']:.10f} ({r['seconds']:.1f}s)", flush=True)
                del solver
        rec["decision"], changes = decide(rows)
        rec.update(status="COMPLETE", changes={str(g): v for g, v in changes.items()}, finished_utc=utc())
        rec.pop("active_case", None)
        save_receipt(path, rec)
    except (Exception, KeyboardInterrupt) as exc:
        rec.update(status="FAILED", decision="UNRESOLVED", error=f"{type(exc).__name__}: {exc}", finished_utc=utc())
        save_receipt(path, rec)
        print(rec["error"], flush=True)
        return 2
    print(rec["decision"], json.dumps(rec["changes"]), flush=True)
    return {"CONVERGED": 0, "NOT_CONVERGED": 1, "UNRESOLVED": 2}[rec["decision"]]


if __name__ == "__main__":
    sys.exit(main())
