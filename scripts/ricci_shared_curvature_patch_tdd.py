"""Focused real-patch RED/GREEN evidence; 180-second worker budget."""
import argparse
import hashlib
import io
import json
from pathlib import Path
import subprocess
import sys
import unittest
import test_ricci_shared_curvature_patch as tests
import ricci_shared_curvature_patch as implementation


def main():
    parser = argparse.ArgumentParser(); parser.add_argument("phase", choices=["red", "green"])
    parser.add_argument("--worker", action="store_true"); args = parser.parse_args()
    if not args.worker:
        subprocess.run([sys.executable, __file__, args.phase, "--worker"], check=True, timeout=180)
        return
    destination = implementation.OUT/("patch-"+args.phase+".json")
    if destination.exists():
        raise ValueError("preserve TDD evidence")
    stream = io.StringIO()
    result = unittest.TextTestRunner(stream=stream, verbosity=2).run(unittest.defaultTestLoader.loadTestsFromModule(tests))
    files = [Path(__file__), Path(tests.__file__), Path(implementation.__file__),
        implementation.ROOT/"scripts/ricci_shared_curvature_math.py",
        implementation.ROOT/"docs/RICCI_SHARED_CURVATURE_PATCH_SPEC.md"]
    receipt = {"phase": args.phase, "passed": result.wasSuccessful(), "tests": result.testsRun,
        "failures": len(result.failures), "errors": len(result.errors), "output": stream.getvalue(),
        "sources": {str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in files}}
    destination.write_text(json.dumps(receipt, indent=2)+"\n", encoding="utf-8")
    print(stream.getvalue())
    if result.wasSuccessful() != (args.phase == "green"):
        raise ValueError("unexpected TDD result; keep failed evidence")


if __name__ == "__main__":
    main()
