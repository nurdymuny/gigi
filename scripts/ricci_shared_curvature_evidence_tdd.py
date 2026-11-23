import argparse
import hashlib
import io
import json
from pathlib import Path
import unittest
import test_ricci_shared_curvature_evidence as tests
import ricci_shared_curvature_patch as implementation


def main():
    parser = argparse.ArgumentParser(); parser.add_argument("phase", choices=["red", "green"])
    args = parser.parse_args()
    destination = implementation.OUT/("evidence-"+args.phase+".json")
    if destination.exists():
        raise ValueError("preserve TDD receipt")
    stream = io.StringIO()
    result = unittest.TextTestRunner(stream=stream, verbosity=2).run(unittest.defaultTestLoader.loadTestsFromModule(tests))
    files = [Path(__file__), Path(tests.__file__), Path(implementation.__file__)]
    receipt = {"phase": args.phase, "passed": result.wasSuccessful(), "tests": result.testsRun,
        "failures": len(result.failures), "errors": len(result.errors), "output": stream.getvalue(),
        "sources": {str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in files}}
    destination.write_text(json.dumps(receipt, indent=2)+"\n", encoding="utf-8")
    print(stream.getvalue())
    if result.wasSuccessful() != (args.phase == "green"):
        raise ValueError("unexpected TDD result")


if __name__ == "__main__":
    main()
