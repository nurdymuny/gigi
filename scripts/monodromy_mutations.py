"""Mechanism-removal checks in an isolated copy; never edit the worktree.

Compilation errors, timeouts, and missing tests do NOT count as a killed mutant.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def run(output, target):
    # Cargo can consider a copied package fresh by timestamps. A mutant must
    # never populate the cache used to build the real server.
    target = target / "monodromy-mutants"
    output.mkdir(parents=True, exist_ok=True)
    receipt = output / "mutations.PASS.json"
    receipt.unlink(missing_ok=True)
    cases = [
        ("closure_disabled", "while head < self.elements.len() {", "while false && head < self.elements.len() {",
         ["--test", "monodromy_group_order", "mg02_icosahedral"]),
        ("antipodes_collapsed", "fn distance(a: GroupElement, b: GroupElement) -> f64 {",
         "fn distance(a: GroupElement, b: GroupElement) -> f64 {\n    let x = coordinates(a); let y = coordinates(b);\n    let plus = x.iter().zip(y).map(|(a,b)| (a+b).powi(2)).sum::<f64>().sqrt();\n    distance_unquotiented(a,b).min(plus)\n}\nfn distance_unquotiented(a: GroupElement, b: GroupElement) -> f64 {",
         ["--test", "monodromy_group_order", "mg01_identity_central"]),
        ("cap_ignored", "if self.elements.len() >= cap {", "if self.elements.len() >= 256 {",
         ["--test", "monodromy_group_order", "mg04_cap"]),
        ("associativity_disabled", "if table[table[a][b]][c] != table[a][table[b][c]] {",
         "if false && table[table[a][b]][c] != table[a][table[b][c]] {",
         ["--lib", "monodromy::tests::mg06_table_certifier"]),
        ("product_table_corrupted", "table[a][b] = i;",
         "table[a][b] = if a == 1 && b == 1 { n } else { i };",
         ["--test", "monodromy_group_order", "mg02_icosahedral"]),
        ("commutators_suppressed", "let c = table[table[table[a][b]][inverses[a]]][inverses[b]];",
         "let c = 0;", ["--test", "monodromy_group_order", "mg01_quaternion"]),
    ]
    receipts = []
    with tempfile.TemporaryDirectory(prefix="gigi-monodromy-mutations-") as scratch:
        work = Path(scratch)
        for folder in ["src", "examples", "benches", "dashboard"]:
            shutil.copytree(ROOT / folder, work / folder)
        (work / "tests").mkdir()
        shutil.copy2(ROOT / "tests/monodromy_group_order.rs", work / "tests/monodromy_group_order.rs")
        for name in ["Cargo.toml", "Cargo.lock", "openapi.json", "build.rs"]:
            if (ROOT / name).exists():
                shutil.copy2(ROOT / name, work / name)
        source = work / "src/monodromy.rs"
        original = source.read_text(encoding="utf-8")

        def execute(name, args, expect_failure):
            cmd = ["cargo", "test", "--features", "halcyon", "--target-dir", str(target), *args]
            log_path = output / (name + ".log")
            with log_path.open("w", encoding="utf-8") as log:
                completed = subprocess.run(cmd, cwd=work, stdout=log, stderr=subprocess.STDOUT,
                                           timeout=1800, env={**os.environ, "CARGO_BUILD_JOBS": "4"})
            text = log_path.read_text(encoding="utf-8", errors="replace")
            if expect_failure:
                valid = completed.returncode != 0 and "test result: FAILED" in text and "... FAILED" in text and "could not compile" not in text
            else:
                valid = completed.returncode == 0 and "test result: ok" in text
            receipts.append({"name": name, "command": cmd, "exit_code": completed.returncode,
                             "expected_behavior_observed": valid,
                             "source_sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
                             "log_sha256": hashlib.sha256(log_path.read_bytes()).hexdigest()})
            (output / "mutations.json").write_text(json.dumps(receipts, indent=2), encoding="utf-8")
            if not valid:
                raise AssertionError(f"{name}: invalid mutation evidence; inspect {log_path}")
            print(f"{name}: {'detected behavioral failure' if expect_failure else 'baseline passed'}", flush=True)

        execute("baseline_integration", ["--test", "monodromy_group_order"], False)
        execute("baseline_internal", ["--lib", "monodromy::tests"], False)
        for name, old, new, args in cases:
            if original.count(old) != 1:
                raise AssertionError(f"mutation anchor drift: {name}")
            source.write_text(original.replace(old, new), encoding="utf-8")
            execute(name, args, True)
            source.write_text(original, encoding="utf-8")
    receipt.write_text(json.dumps({"status": "PASS", "mutants_detected": len(cases),
        "receipt_sha256": hashlib.sha256((output / "mutations.json").read_bytes()).hexdigest()}, indent=2), encoding="utf-8")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--target-dir", type=Path, required=True)
    args = parser.parse_args()
    run(args.output.resolve(), args.target_dir.resolve())
