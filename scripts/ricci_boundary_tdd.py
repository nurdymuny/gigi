import argparse
import io
import json
from pathlib import Path
import subprocess
import sys
import unittest
import test_ricci_boundary_extension as tests
import test_ricci_boundary_evidence as evidence
import ricci_boundary_extension as subject
from ricci_local_push_audit import sha


def main():
    parser = argparse.ArgumentParser(); parser.add_argument("phase", choices=["red", "green"])
    parser.add_argument("--worker", action="store_true")
    parser.add_argument("--evidence", action="store_true"); args = parser.parse_args()
    if not args.worker:
        subprocess.run([sys.executable, __file__, args.phase, "--worker"]+(["--evidence"] if args.evidence else []), check=True, timeout=180)
        return
    subject.OUT.mkdir(parents=True, exist_ok=True)
    output = subject.OUT/(("evidence-" if args.evidence else "")+args.phase+".json")
    if output.exists():
        raise ValueError("preserve TDD receipt")
    stream = io.StringIO()
    module = evidence if args.evidence else tests
    result = unittest.TextTestRunner(stream=stream, verbosity=2).run(unittest.defaultTestLoader.loadTestsFromModule(module))
    files = [Path(__file__), Path(subject.__file__), Path(module.__file__), subject.ROOT/"docs/RICCI_BOUNDARY_EXTENSION_SPEC.md"]
    receipt = {"phase": args.phase, "passed": result.wasSuccessful(), "tests": result.testsRun,
        "failures": len(result.failures), "errors": len(result.errors), "output": stream.getvalue(),
        "sources": {str(p): sha(p) for p in files}}
    output.write_text(json.dumps(receipt, indent=2)+"\n", encoding="utf-8")
    print(stream.getvalue())
    if result.wasSuccessful() != (args.phase == "green"):
        raise ValueError("unexpected TDD phase result")


if __name__ == "__main__":
    main()
