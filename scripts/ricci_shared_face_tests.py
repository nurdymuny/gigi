"""Persist focused RED/GREEN receipts without invoking the project suite."""
import argparse
import hashlib
import io
import json
from pathlib import Path
import unittest
import test_ricci_shared_face_math as tests
import ricci_shared_face_math as implementation

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "output/ricci-shared-face-20261117"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("phase", choices=["red", "green"])
    args = parser.parse_args()
    OUT.mkdir(parents=True, exist_ok=True)
    path = OUT / (args.phase + ".json")
    if path.exists():
        raise ValueError("preserve prior TDD receipt")
    stream = io.StringIO()
    result = unittest.TextTestRunner(stream=stream, verbosity=2).run(
        unittest.defaultTestLoader.loadTestsFromModule(tests))
    sources = [Path(__file__), Path(tests.__file__), Path(implementation.__file__),
               ROOT/"docs/RICCI_SHARED_FACE_CONSTRAINT_SPEC.md"]
    receipt = {"phase": args.phase, "passed": result.wasSuccessful(), "tests": result.testsRun,
        "errors": len(result.errors), "failures": len(result.failures), "output": stream.getvalue(),
        "sources": {str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in sources}}
    path.write_text(json.dumps(receipt, indent=2)+"\n", encoding="utf-8")
    print(stream.getvalue())
    if result.wasSuccessful() != (args.phase == "green"):
        raise ValueError("unexpected TDD phase outcome")


if __name__ == "__main__":
    main()
