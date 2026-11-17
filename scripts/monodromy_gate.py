"""Sequential, fail-closed release gate. Never shares a running test executable.

All subprocesses are awaited. Only the demonstration's private server is started.
Run from any directory with --output DIR and --target-dir DIR.
"""
import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "e2e/probes"))
from monodromy_group_order_demo import source_manifest
from monodromy_evidence import sha, verify


def run(output, target):
    output.mkdir(parents=True, exist_ok=True)
    receipt = output / "gate.PASS.json"
    receipt.unlink(missing_ok=True)
    started = datetime.now(timezone.utc).isoformat()
    before = source_manifest()
    steps = []

    def execute(name, cmd, extra_env=None, timeout=3600):
        log_path = output / (name + ".log")
        print(f"Running {name}", flush=True)
        with log_path.open("w", encoding="utf-8") as log:
            result = subprocess.run(cmd, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT,
                                    env={**os.environ, "CARGO_BUILD_JOBS": "4", **(extra_env or {})}, timeout=timeout)
        text = log_path.read_text(encoding="utf-8", errors="replace")
        counts = re.findall(r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;", text)
        totals = [sum(int(c[i]) for c in counts) for i in range(3)]
        steps.append({"name": name, "command": cmd, "exit_code": result.returncode,
                      "log_sha256": sha(log_path), "rust_passed_failed_ignored": totals})
        (output / "steps.json").write_text(json.dumps(steps, indent=2), encoding="utf-8")
        if result.returncode != 0:
            raise RuntimeError(f"Gate OPEN: {name} failed; see {log_path}")
        print(f"Passed {name}; Rust counts={totals}", flush=True)
        return text

    execute("build-isolation", [sys.executable, "-m", "unittest", "discover", "-s", "e2e/probes", "-p", "test_monodromy_build_isolation.py"])
    cargo = ["cargo", "test", "--no-fail-fast", "--jobs", "4", "--target-dir", str(target)]
    default = execute("default-full", cargo)
    if "mg07_feature_off_refuses ... ok" not in default or "tests::monodromy_wire_gate ... ok" not in default:
        raise RuntimeError("default feature-off gates did not execute")
    enabled = execute("halcyon-full", cargo + ["--features", "halcyon"])
    for test in ["mg02_icosahedral_120_and_perfection", "mg05_inconsistent_buffer_metadata_refuses",
                 "mg06_table_certifier_rejects_bad_evidence", "monodromy_wire_gate"]:
        if test + " ... ok" not in enabled:
            raise RuntimeError(f"required gate did not execute: {test}")
    execute("mutations", [sys.executable, str(ROOT / "scripts/monodromy_mutations.py"),
                          "--output", str(output / "mutations"), "--target-dir", str(target)])
    mutation_receipt = json.loads((output / "mutations/mutations.PASS.json").read_text(encoding="utf-8"))
    if mutation_receipt["mutants_detected"] != 6:
        raise RuntimeError("not all mechanism removals detected")
    execute("build", ["cargo", "build", "--features", "halcyon", "--bin", "gigi-stream", "--target-dir", str(target)])
    binary = output / ("gigi-stream-verified.exe" if os.name == "nt" else "gigi-stream-verified")
    shutil.copy2(target / "debug" / ("gigi-stream.exe" if os.name == "nt" else "gigi-stream"), binary)
    execute("live", [sys.executable, str(ROOT / "e2e/probes/monodromy_group_order_demo.py"),
                     "--binary", str(binary), "--output", str(output / "live")])
    execute("evidence-adversarial", [sys.executable, "-m", "unittest", "discover", "-s", "e2e/probes", "-p", "test_monodromy_evidence.py"],
            {"MONODROMY_EVIDENCE": str(output / "live/live.json")})
    execute("diff-check", ["git", "diff", "--check"])
    if before != source_manifest():
        raise RuntimeError("Gate OPEN: source changed during validation")
    live = verify(output / "live/live.json")
    result = {"status": "PASS", "started_utc": started, "finished_utc": datetime.now(timezone.utc).isoformat(),
              "head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
              "worktree": "contains uncommitted changes; source hashes are authoritative",
              "sources": before, "binary_sha256": sha(binary), "steps": steps,
              "live_cases": live["cases_verified"], "mutants_detected": 6,
              "scope": "bounded numerical SU(2) supplied-generator closure; not manifold recognition or exact f64 finiteness"}
    with tempfile.NamedTemporaryFile("w", dir=output, delete=False, encoding="utf-8") as stream:
        tmp = Path(stream.name)
        json.dump(result, stream, indent=2)
        stream.flush()
        os.fsync(stream.fileno())
    try:
        os.replace(tmp, receipt)
    finally:
        tmp.unlink(missing_ok=True)
    print(f"ALL GATES PASS: {receipt}", flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--target-dir", type=Path, required=True)
    args = parser.parse_args()
    run(args.output.resolve(), args.target_dir.resolve())
