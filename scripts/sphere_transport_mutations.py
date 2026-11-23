"""Compile isolated copies of the checked adapter; never modify the worktree.

Reuse the actual built GIGI library for the native kernel, types, and Engine.
The same integration tests plus private postcondition tests run on each copy.
Compilation failure is NOT a killed mutation. No benchmarks or server launched.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "src/dials/sphere_transport.rs"
TESTS = ROOT / "tests/sphere_transport.rs"


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--rlib", type=Path, required=True)
    parser.add_argument("--output", type=Path, default=ROOT / "docs/research/evidence/sphere-transport-20261113")
    args = parser.parse_args()
    rlib = args.rlib.resolve()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=True)
    original = SOURCE.read_text(encoding="utf-8")
    tests = TESTS.read_text(encoding="utf-8").replace(
        "use gigi::dials::{sphere_parallel_transport as transport, sphere_parallel_transport_records};",
        "use crate::adapter::checked::{sphere_parallel_transport as transport, sphere_parallel_transport_records};",
    )
    changes = {
        "identity_instead_of_rotation": (
            "super::transport_rotation_matrix(&p, &q, angle)",
            "vec![1.,0.,0.,0.,1.,0.,0.,0.,1.]"),
        "reverse_rotation": ("super::transport_rotation_matrix(&p, &q, angle)",
                             "super::transport_rotation_matrix(&p, &q, -angle)"),
        "no_unit_guard": ("!n.is_finite() || (n - 1.).abs() > UNIT_TOL", "false"),
        "no_tangency_guard": ("dot(p, direction).abs() > UNIT_TOL", "false"),
        "no_antipodal_guard": ("1. + cosine <= ANTIPODAL_GAP", "false"),
        "no_magnitude_guard": ("vn > 1e100", "vn > f64::MAX"),
        "no_roundoff_normalization": ("Ok((v.map(|x| x / n), n))", "Ok((v, n))"),
        "missing_measurements_become_zero": (
            "_ => return Err(format!(\"{label} field '{field}' must contain a numeric measurement; missing, null and nonnumeric values cannot be replaced by zero\")),",
            "_ => 0.,"),
        "no_postcondition_check": (
            "transported.iter().any(|x| !x.is_finite())\n        || diagnostics.iter().any(|x| !x.is_finite() || *x > RESIDUAL_TOL)",
            "false"),
    }
    pins = {str(p.relative_to(ROOT)): sha(p) for p in [SOURCE, TESTS, rlib]}
    rows = []
    with tempfile.TemporaryDirectory(prefix="gigi-sphere-mutations-") as tmp:
        work = Path(tmp)
        for name, change in [("baseline", None), *changes.items()]:
            text = original
            if change:
                old, new = change
                assert text.count(old) == 1, (name, text.count(old))
                text = text.replace(old, new)
            implementation = work / "checked.rs"
            implementation.write_text(text, encoding="utf-8")
            harness = work / "audit.rs"
            harness.write_text(
                'extern crate gigi;\nmod types { pub use gigi::types::{Record,Value}; }\n'
                'mod adapter { pub use gigi::dials::transport_rotation_matrix;\n'
                '#[path="' + implementation.as_posix() + '"] pub mod checked; }\n'
                'mod contract {\n' + tests + '\n}\n', encoding="utf-8")
            binary = work / ("audit.exe" if os.name == "nt" else "audit")
            command = ["rustc", "--edition=2021", "--test", str(harness), "--extern", f"gigi={rlib}",
                       "-L", f"dependency={rlib.parent}", "--extern", "tempfile=" + str(max(rlib.parent.glob("libtempfile-*.rlib"), key=lambda p: p.stat().st_mtime)), "-o", str(binary)]
            build = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
            (out / f"{name}-build.log").write_text(build.stdout+build.stderr, encoding="utf-8")
            assert build.returncode == 0, f"Compile failed for {name}; this does not count as a caught mutation"
            run = subprocess.run([str(binary), "--nocapture"], cwd=ROOT, capture_output=True, text=True)
            log = run.stdout+run.stderr
            (out / f"{name}-test.log").write_text(log, encoding="utf-8")
            if change:
                assert run.returncode != 0 and "test result: FAILED" in log, f"Surviving mutation: {name}"
            else:
                assert run.returncode == 0 and "test result: ok" in log, log
            rows.append({"name": name, "compiled": True, "test_exit": run.returncode,
                         "assertion_failure": "panicked at" in log,
                         "source_sha256": hashlib.sha256(text.encode()).hexdigest()})
            print(json.dumps(rows[-1]), flush=True)
    for path, digest in pins.items():
        assert sha(ROOT/path) == digest, f"Pinned file changed during audit: {path}"
    receipt = {"passed": True, "scope": "Isolated source mutations against actual GIGI library, no worktree edits",
               "pinned_files": pins, "cases": rows}
    (out/"mutations.json").write_text(json.dumps(receipt, indent=2)+"\n", encoding="utf-8")


if __name__ == "__main__":
    main()
