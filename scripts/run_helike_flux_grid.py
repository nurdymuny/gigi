"""The registered flux-edge grid under the method amendment (docs/research/helike-flux-method-amendment-2026-11-25.md).

usage:
  python scripts/run_helike_flux_grid.py --check-release          # report unmet release conditions, compute nothing
  python scripts/run_helike_flux_grid.py OUT_DIR                  # the registered grid (refuses until released)

- Solver: the positive-cone Ritz solver with exact Kronecker moments (helike-lab `helike cone`).
- Ladder Mq = 6, 8, 10; a point is converged if both identified gaps change by <= 0.003 on the last step AND the
  per-point domain-validity gate (relative leakage gate at the final rung) passes.
- Identification, switch definition, tolerance and decision rule: verbatim from the registration
  (docs/research/helike-flux-background-registration-2026-11-24.md); switch_point is reused from the original runner.
- The domain gate (helike_domain_gate): Rust cone-export of the final-rung states, Arb tail/leakage bounds, Rust
  interval residual/charge bounds, then the registered relative leakage gate on Helike and the competitor.
- The registered grid only starts with an explicit --registered-grid, and only if every release condition holds.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import subprocess
import sys
import tempfile
import time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
sys.path.insert(0, str(HERE))
import run_helike_flux_background as registered  # noqa: E402

DESIGN = {
    "primary": registered.DESIGN["primary"],
    "secondary": registered.DESIGN["secondary"],
    "ladder": [6, 8, 10],                    # amended: total quanta Mq; exact moments, no Taylor order
    "convergence_tol": registered.DESIGN["convergence_tol"],
    "weight_min": registered.DESIGN["weight_min"],
    "s_tolerance": registered.DESIGN["s_tolerance"],
    "predicted_s_star": registered.DESIGN["predicted_s_star"],
}
AMENDMENT = "docs/research/helike-flux-method-amendment-2026-11-25.md"
HELIKE_BIN = ROOT / ("helike-lab/target/release/helike" + (".exe" if sys.platform == "win32" else ""))

# Release evidence, pinned (same receipts the paper builder reads).
PINNED = {
    "convergence": ("research/cone-convergence-20261124/controls-59534df9/receipt.json",
                    "f1eec3b8d49a9a93c95ca0cb98bbe3a344ae51e561b5d47543c438bc6abeeb74"),
    "leakage": ("research/leakage-gate-20261125/controls-fbfa958/receipt.json",
                "2203e230a7cda2629b996c9c21b06d984c43ee874778544896e336702f60c82f"),
    "temple": ("research/temple-20261125/cone_temple_c31aab8e.json",
               "938e9b3f9027070aa112c55968f50ac4165779aaa400bdd906c845fe90569484"),
}


def sha(p):
    """LF-normalised digest (the original runner's sha256-universal-newline-v1): the pins are digests of the committed
    text, so a CRLF checkout must not read as a changed receipt."""
    return registered.sha(p)


def raw_sha(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()


SOURCES = tuple("scripts/" + f for f in (
    "run_helike_flux_grid.py", "run_helike_flux_background.py", "helike_domain_gate.py", "helike_cutoff_bounds.py",
    "helike_leakage_gate.py", "helike_positive_cone.py")) + (AMENDMENT, registered.REGISTRATION)


def provenance():
    """What produced the receipt: commit, dirty flag, binary digest, source digests, Arb library version."""
    def git(*a):
        try:
            return subprocess.run(["git", *a], cwd=ROOT, capture_output=True, text=True, check=True).stdout.strip()
        except (OSError, subprocess.CalledProcessError):
            return None
    try:
        import flint
        flint_version = flint.__version__
    except ImportError:
        flint_version = None
    return {"git_commit": git("rev-parse", "HEAD"), "git_dirty": bool(git("status", "--porcelain", "--", "scripts", "helike-lab/crates")),
            "helike_bin_sha256": raw_sha(HELIKE_BIN) if HELIKE_BIN.exists() else None,
            "sources_sha256": {f: sha(ROOT / f) for f in SOURCES}, "hash_policy": registered.HASH_POLICY,
            "python_flint": flint_version, "python": sys.version.split()[0]}


def finite_json(x):
    """Strict JSON for the receipt: a non-finite float is written as its string ("nan", "inf"), never dropped."""
    if isinstance(x, float) and not math.isfinite(x):
        return str(x)
    if isinstance(x, dict):
        return {k: finite_json(v) for k, v in x.items()}
    if isinstance(x, (list, tuple)):
        return [finite_json(v) for v in x]
    return x


def save(path, rec):
    registered.save_receipt(path, finite_json(rec))   # atomic write + LF-digest integrity file


def domain_stage_ready():
    """The domain stage needs the Rust binary and python-flint (Arb)."""
    try:
        import helike_domain_gate  # noqa: F401  (imports helike_cutoff_bounds, hence flint)
    except ImportError:
        return False
    return HELIKE_BIN.exists()


def domain_gate(s, t, g2, mq):
    """Per-point domain validity (amendment 3.3, gate re-registered in §5.7: e_cut = l + 2tr <= 3e-4) on the final
    rung's Helike and competitor states. Returns (verdict, details). Any refusal inside the stage is OPEN, never PASS."""
    import helike_domain_gate as dg
    try:
        r = dg.domain_gate_point(s, t, g2, mq)
    except subprocess.CalledProcessError as exc:
        return "OPEN", {"error": f"{type(exc).__name__}: {exc}", "command": exc.cmd,
                        "returncode": exc.returncode, "stdout": exc.stdout, "stderr": exc.stderr}
    except Exception as exc:  # noqa: BLE001 -- any refusal or failure inside the stage is OPEN, never PASS
        return "OPEN", {"error": f"{type(exc).__name__}: {exc}"}
    return r["verdict"], r


def assess_point(point, D):
    rows = point["ladder"]
    if [r["Mq"] for r in rows] != D["ladder"]:
        raise ValueError("incomplete or reordered refinement ladder")
    for r in rows:
        for n in ("helike", "competitor"):
            gap, w, wd = r[n]["gap"], r[n]["weight"], r[n]["dressed_weight"]
            if not (math.isfinite(wd) and wd >= 0 and abs(wd * r["vacuum_weight"] - w) <= 1e-9):
                raise ValueError("dressed weight is not weight / vacuum weight")
            if not (math.isfinite(gap) and gap >= 0 and 0 <= w <= 1 + 1e-10):
                raise ValueError("invalid excitation gap or weight")
    last_step = max(abs(rows[-1][n]["gap"] - rows[-2][n]["gap"]) for n in ("helike", "competitor"))
    last = rows[-1]
    return {
        "last_step_change": last_step,
        "converged": bool(last_step <= D["convergence_tol"] and point.get("domain_gate") == "PASS"),
        # amendment 5.2: the registered 0.9 applies to the dressed weight w / Z (Z = ground-state vacuum weight)
        "identified": bool(min(last[n]["dressed_weight"] for n in ("helike", "competitor")) >= D["weight_min"]),
        "diff_competitor_minus_helike": last["competitor"]["gap"] - last["helike"]["gap"],
    }


def assess_case(case, D):
    if any(p.get("refused") for p in case["points"]):
        return {"status": "UNRESOLVED", "measured_switch": None}
    pts = [assess_point(p, D) for p in case["points"]]
    if not all(a["converged"] and a["identified"] for a in pts):
        return {"status": "UNRESOLVED", "measured_switch": None}
    predicted = D["predicted_s_star"][str(case["t"])][str(case["g2"])]
    try:
        measured = registered.switch_point([p["s"] for p in case["points"]], [a["diff_competitor_minus_helike"] for a in pts])
    except ValueError as exc:
        # multiple crossings, or a zero difference without a unique interior bracket: no single switch can be read
        # off. The registered rule has no outcome for this; it is reported as UNRESOLVED with the reason (disclosed
        # amendment, fail-closed: neither SUPPORTED nor NOT_SUPPORTED is claimed).
        return {"status": "UNRESOLVED", "measured_switch": None, "predicted_switch": predicted,
                "reason": f"ambiguous crossing: {exc}"}
    if measured is None or abs(measured - predicted) > D["s_tolerance"]:
        status = "NOT_SUPPORTED"
    else:
        status = "SUPPORTED"
    return {"status": status, "measured_switch": measured, "predicted_switch": predicted}


def overall(primary_statuses):
    """The registered overall rule over the three primary couplings."""
    if len(primary_statuses) != 3:
        raise ValueError("the overall verdict needs the three primary couplings")
    if all(s == "SUPPORTED" for s in primary_statuses):
        return "SUPPORTED"
    if any(s == "UNRESOLVED" for s in primary_statuses):
        return "UNRESOLVED"
    if all(s == "NOT_SUPPORTED" for s in primary_statuses):
        return "NOT_SUPPORTED"
    return "MIXED"


def unmet_release_conditions(pins=None):
    pins = pins or PINNED
    unmet = []
    try:
        conv = json.loads((ROOT / pins["convergence"][0]).read_text())
        ok = sha(ROOT / pins["convergence"][0]) == pins["convergence"][1] and conv["decision"] == "CONVERGED"
    except (OSError, ValueError, KeyError):
        ok = False
    if not ok:
        unmet.append("convergence")
    try:
        leak = json.loads((ROOT / pins["leakage"][0]).read_text())
        tem = json.loads((ROOT / pins["temple"][0]).read_text())
        ok = (sha(ROOT / pins["leakage"][0]) == pins["leakage"][1] and sha(ROOT / pins["temple"][0]) == pins["temple"][1]
              and leak["relative_leakage_gate"] == "PASS" and leak["full_grid_released"] is False
              and tem["status"] == "INTERVAL_ENCLOSURE" and tem["params"]["assume"] == "S_C")
    except (OSError, ValueError, KeyError):
        ok = False
    if not ok:
        unmet.append("certificate")
    if not (ROOT / AMENDMENT).exists():
        unmet.append("method_amendment")
    if not domain_stage_ready():
        unmet.append("domain_gate_stage")
    return unmet


def solve(s, t, g2, mq):
    """One rung through the Rust exact-moment solver; returns the registered rung fields."""
    with tempfile.TemporaryDirectory() as d:
        out = Path(d) / "r.json"
        subprocess.run([str(HELIKE_BIN), "--out", str(out), "cone", "--s", str(s), "--t", str(t), "--g2", str(g2),
                        "--mq", str(mq)], check=True, capture_output=True, text=True)
        r = json.loads(out.read_text())["results"]
    res = r["result"]
    return {"Mq": mq, "E0": res["E0"], "dimension": r["dimension"], "seconds": r["seconds"], "moments": r["moments"],
            "vacuum_weight": res["vacuum_weight"],
            "helike": {k: res["helike"][k] for k in ("gap", "weight", "dressed_weight")},
            "competitor": {k: res["competitor"][k] for k in ("gap", "weight", "dressed_weight")},
            "overlap_orders": res["overlap_orders"], "reference_weight_error": res["reference_weight_error"],
            # amendment 5.3: report-only cross-check, never read by assess_point or the domain gate
            "cross_check": {"dressed_overlap": {n: res[n].get("dressed_overlap") for n in ("helike", "competitor")},
                            "dressed_overlap_error": res.get("dressed_overlap_error"),
                            "dressed_overlap_orthonormality": res.get("dressed_overlap_orthonormality")}}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out_dir", nargs="?")
    ap.add_argument("--check-release", action="store_true")
    ap.add_argument("--registered-grid", action="store_true", help="explicitly start the registered grid")
    a = ap.parse_args()
    unmet = unmet_release_conditions()
    if a.check_release or not a.out_dir:
        print(json.dumps({"unmet_release_conditions": unmet}))
        return 0
    if unmet:
        print("HOLD: unmet release conditions " + ", ".join(unmet))
        return 2
    if not a.registered_grid:
        print("refusing: the registered grid starts only with --registered-grid (run it on the big machine)")
        return 2
    out = Path(a.out_dir)
    if out.exists() and any(out.iterdir()):
        print("refusing to write into a non-empty directory")
        return 2
    out.mkdir(parents=True, exist_ok=True)
    rec = {"status": "RUNNING", "design": DESIGN, "amendment": AMENDMENT, "started_unix": time.time(),
           "provenance": provenance(), "certified": False, "cases": []}
    receipt = out / "receipt.json"
    for label in ("primary", "secondary"):
        block = DESIGN[label]
        for g2 in block["g2"]:
            case = {"block": label, "t": block["t"], "g2": g2, "points": []}
            rec["cases"].append(case)
            for s in block["s"]:
                try:
                    p = {"s": s, "ladder": [solve(s, block["t"], g2, mq) for mq in DESIGN["ladder"]]}
                except (subprocess.CalledProcessError, ValueError, KeyError) as exc:
                    # a solver refusal is a failed point (UNRESOLVED under the registered rule), never a crash
                    detail = (getattr(exc, "stderr", None) or str(exc)).strip()[-500:]   # keep the solver's reason
                    p = {"s": s, "ladder": [], "refused": f"{type(exc).__name__}: {detail}", "converged": False,
                         "identified": False}
                    case["points"].append(p)
                    save(receipt, rec)
                    continue
                p["domain_gate"], p["domain"] = domain_gate(s, block["t"], g2, DESIGN["ladder"][-1])
                try:
                    p.update(assess_point(p, DESIGN))
                except ValueError as exc:
                    p.update(refused=f"ValueError: {exc}", converged=False, identified=False)
                case["points"].append(p)
                save(receipt, rec)
            case.update(assess_case(case, DESIGN))
    statuses = [c["status"] for c in rec["cases"] if c["block"] == "primary"]
    rec.update(status="COMPLETE", overall=overall(statuses), certified=False, finished_unix=time.time())
    save(receipt, rec)
    print(rec["overall"])
    return 0


if __name__ == "__main__":
    sys.exit(main())
