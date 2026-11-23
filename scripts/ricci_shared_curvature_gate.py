"""Focused final gate and real-receipt attacks; bounded to this patch."""
import argparse
import copy
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
import ricci_shared_curvature_patch as subject
import test_ricci_shared_curvature_math as math_tests
import test_ricci_shared_curvature_patch as patch_tests
import test_ricci_shared_curvature_evidence as evidence_tests
from ricci_local_push_audit import sha


def main():
    parser = argparse.ArgumentParser(); parser.add_argument("--worker", action="store_true")
    args = parser.parse_args()
    if not args.worker:
        subprocess.run([sys.executable, __file__, "--worker"], check=True, timeout=180)
        return
    destination = subject.OUT/"final-gate.json"
    if destination.exists():
        raise ValueError("preserve final evidence")
    stream = io.StringIO(); suite = unittest.TestSuite()
    for module in (math_tests, patch_tests, evidence_tests):
        suite.addTests(unittest.defaultTestLoader.loadTestsFromModule(module))
    result = unittest.TextTestRunner(stream=stream, verbosity=2).run(suite)
    print(stream.getvalue(), flush=True)
    test_receipt = {"tests": result.testsRun, "failures": len(result.failures), "errors": len(result.errors),
                    "passed": result.wasSuccessful(), "output": stream.getvalue()}
    if not result.wasSuccessful():
        destination.write_text(json.dumps({"status": "FAILED", "tests": test_receipt}, indent=2)+"\n", encoding="utf-8")
        raise ValueError("focused final tests failed")
    for phase in ("run", "verify"):
        subprocess.run([sys.executable, str(Path(subject.__file__)), phase], check=True, timeout=180)
    diagnostic = json.loads((subject.OUT/"diagnostic.json").read_text(encoding="utf-8"))
    attacks = []
    for name in ("changed polynomial", "missing flux constraint", "forged face agreement", "changed source hash",
                 "nonfinite evidence", "duplicate evidence key"):
        changed = copy.deepcopy(diagnostic)
        if name == "changed polynomial":
            changed["polynomial"][0] += .01
        elif name == "missing flux constraint":
            changed["fluxes"].pop()
        elif name == "forged face agreement":
            changed["report"]["internal_faces"][0]["original_whitney"]["sampled_trace_match"] = True
        elif name == "changed source hash":
            changed["sources"][next(iter(changed["sources"]))] = "0"*64
        elif name == "nonfinite evidence":
            changed["polynomial"][0] = float("nan")
        raw = json.dumps(changed)
        if name == "duplicate evidence key":
            raw = '{"schema":"forged",'+raw[1:]
        with tempfile.TemporaryDirectory(prefix="gigi-patch-evidence-") as folder:
            directory = Path(folder)
            (directory/"diagnostic.json").write_text(raw, encoding="utf-8")
            (directory/"VERIFIED.json").write_text("stale PASS", encoding="utf-8")
            try:
                subject.verify(directory)
            except ValueError as error:
                attacks.append({"attack": name, "refused": True, "reason": str(error),
                                "stale_pass_removed": not (directory/"VERIFIED.json").exists()})
            else:
                attacks.append({"attack": name, "refused": False, "stale_pass_removed": False})
        print(name, attacks[-1], flush=True)
    mutations = json.loads((subject.OUT/"mechanism-removals.json").read_text(encoding="utf-8"))
    for filename, expected in mutations["sources"].items():
        if sha(Path(filename)) != expected:
            raise ValueError("mechanism-removal source drift")
    accepted = all(a["refused"] and a["stale_pass_removed"] for a in attacks)
    accepted &= all(m["caught"] and m["errors"] == 0 for m in mutations["mechanisms"])
    files = [Path(__file__), Path(subject.__file__), Path(math_tests.__file__),
             Path(patch_tests.__file__), Path(evidence_tests.__file__),
             subject.ROOT/"scripts/ricci_shared_curvature_math.py",
             subject.ROOT/"docs/RICCI_SHARED_CURVATURE_PATCH_SPEC.md"]
    artifacts = [subject.OUT/name for name in ("diagnostic.json", "VERIFIED.json", "mechanism-removals.json",
        "kernel-red.json", "kernel-green.json", "patch-red.json", "patch-green.json", "evidence-red.json", "evidence-green.json")]
    receipt = {"status": "LOCAL_CONSTRUCTION_VERIFIED" if accepted else "FAILED",
        "full_suite_run": False, "native_or_global_flow_change": False, "global_gate": "UNCHANGED_OPEN",
        "tests": test_receipt, "attacks": attacks, "mechanisms_caught": len(mutations["mechanisms"]),
        "sources": {str(f): sha(f) for f in files}, "artifacts": {str(f): sha(f) for f in artifacts}}
    destination.write_text(json.dumps(receipt, indent=2, allow_nan=False)+"\n", encoding="utf-8")
    if not accepted:
        (subject.OUT/"VERIFIED.json").unlink(missing_ok=True)
        raise ValueError("evidence/mutation gate failed")


if __name__ == "__main__":
    main()
