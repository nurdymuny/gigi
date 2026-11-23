"""Focused real-patch/evidence TDD receipts, with a hard subprocess budget."""
import argparse
import hashlib
import io
import json
from pathlib import Path
import subprocess
import sys
import unittest
import test_ricci_shared_face_patch as tests
import ricci_shared_face_patch as implementation


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("phase", choices=["red", "green"])
    parser.add_argument("--worker", action="store_true")
    args = parser.parse_args()
    if not args.worker:
        subprocess.run([sys.executable, __file__, args.phase, "--worker"], check=True, timeout=90)
        return
    path = implementation.OUT / ("contract-"+args.phase+".json")
    if path.exists():
        raise ValueError("preserve prior contract receipt")
    stream = io.StringIO()
    result = unittest.TextTestRunner(stream=stream, verbosity=2).run(
        unittest.defaultTestLoader.loadTestsFromModule(tests))
    files = [Path(__file__), Path(tests.__file__), Path(implementation.__file__)]
    receipt = {"phase": args.phase, "passed": result.wasSuccessful(), "tests": result.testsRun,
        "failures": len(result.failures), "errors": len(result.errors), "output": stream.getvalue(),
        "sources": {str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in files}}
    path.write_text(json.dumps(receipt, indent=2)+"\n", encoding="utf-8")
    print(stream.getvalue())
    if result.wasSuccessful() != (args.phase == "green"):
        raise ValueError("unexpected contract TDD phase outcome")


if __name__ == "__main__":
    main()
