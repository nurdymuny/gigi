"""Mechanism-removal gates: separate source copy AND separate Cargo cache."""
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
    target = target / "ricci-wilson-mutants"
    output.mkdir(parents=True, exist_ok=True)
    receipt = output / "mutations.PASS.json"
    receipt.unlink(missing_ok=True)
    cases = [
        ("frame_rescaling_removed", "- 0.5*x[i]*(v[j]/g[j]+v[k]/g[k])", "- 0.*x[i]*(v[j]/g[j]+v[k]/g[k])",
         ["--test", "ricci_wilson", "cw01_round_and_noncritical_berger"]),
        ("structure_bracket_removed", "- 2.*x[i]", "- 0.*x[i]",
         ["--lib", "ricci_wilson::tests::cw02_arbitrary_connection_gradient"]),
        ("mass_weight_removed", "-g[i] * gradient", "-gradient",
         ["--lib", "ricci_wilson::tests::cw02_arbitrary_connection_gradient"]),
        ("descent_sign_reversed", "-g[i] * gradient", "g[i] * gradient",
         ["--lib", "ricci_wilson::tests::cw02_arbitrary_connection_gradient"]),
        ("residual_invented_zero", "let residual: Triple = std::array::from_fn(|i| p[i]-w[i]);",
         "let residual: Triple = [0.;3];",
         ["--lib", "ricci_wilson::tests::cw07_residual_cannot_be_invented_zero"]),
    ]
    results = []
    with tempfile.TemporaryDirectory(prefix="gigi-ricci-wilson-mutants-") as temp:
        work = Path(temp)
        for folder in ["src", "examples", "benches", "dashboard"]:
            shutil.copytree(ROOT/folder, work/folder)
        (work/"tests").mkdir()
        shutil.copy2(ROOT/"tests/ricci_wilson.rs", work/"tests/ricci_wilson.rs")
        for file in ["Cargo.toml", "Cargo.lock", "openapi.json", "build.rs"]:
            if (ROOT/file).exists():
                shutil.copy2(ROOT/file, work/file)
        source = work/"src/ricci_wilson.rs"
        original = source.read_text(encoding="utf-8")

        def execute(name, args, negative):
            cmd = ["cargo", "test", "--features", "halcyon", "--target-dir", str(target), *args]
            log = output/(name + ".log")
            with log.open("w", encoding="utf-8") as f:
                done = subprocess.run(cmd, cwd=work, stdout=f, stderr=subprocess.STDOUT, timeout=1800,
                                      env={**os.environ, "CARGO_BUILD_JOBS": "4"})
            text = log.read_text(encoding="utf-8", errors="replace")
            valid = (done.returncode != 0 and "test result: FAILED" in text
                     and args[-1]+" ... FAILED" in text and "could not compile" not in text) if negative else (
                     done.returncode == 0 and "test result: ok" in text and "running 0 tests" not in text)
            results.append({"name": name, "command": cmd, "exit_code": done.returncode, "valid": valid,
                "source_sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
                "log_sha256": hashlib.sha256(log.read_bytes()).hexdigest()})
            (output/"mutations.json").write_text(json.dumps(results, indent=2), encoding="utf-8")
            if not valid:
                raise AssertionError(f"{name}: not valid behavioral evidence; {log}")
            print(name + (": behavioral failure detected" if negative else ": baseline passed"), flush=True)

        execute("baseline_integration", ["--test", "ricci_wilson"], False)
        execute("baseline_internal", ["--lib", "ricci_wilson::tests"], False)
        for name, old, new, args in cases:
            if original.count(old) != 1:
                raise AssertionError(f"anchor drift {name}")
            source.write_text(original.replace(old, new), encoding="utf-8")
            execute(name, args, True)
            source.write_text(original, encoding="utf-8")
    receipt.write_text(json.dumps({"status": "PASS", "mutants_detected": len(cases),
        "receipt_sha256": hashlib.sha256((output/"mutations.json").read_bytes()).hexdigest()}, indent=2), encoding="utf-8")


if __name__ == "__main__":
    p = argparse.ArgumentParser()
    p.add_argument("--output", type=Path, required=True)
    p.add_argument("--target-dir", type=Path, required=True)
    a = p.parse_args()
    run(a.output.resolve(), a.target_dir.resolve())
