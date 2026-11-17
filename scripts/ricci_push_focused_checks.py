"""Eight diagnostic tests and four actual mechanism-removal checks only."""
import io
import json
from pathlib import Path
import unittest
from unittest.mock import patch
import numpy as np

import test_ricci_local_push_math as accounting
import test_ricci_push_interfaces as interfaces
import ricci_local_push_math as math
import ricci_push_interfaces as boundary
from ricci_local_push_audit import OUT, sha


def run(suite):
    stream = io.StringIO()
    result = unittest.TextTestRunner(stream=stream, verbosity=2).run(suite)
    return {"passed": result.wasSuccessful(), "tests": result.testsRun,
            "failures": len(result.failures), "errors": len(result.errors), "output": stream.getvalue()}


def main():
    destination = OUT / "focused-checks.json"
    if destination.exists():
        raise ValueError("preserve prior focused checks")
    loader = unittest.defaultTestLoader
    baseline = run(unittest.TestSuite([loader.loadTestsFromModule(accounting), loader.loadTestsFromModule(interfaces)]))
    if not baseline["passed"] or baseline["tests"] != 8:
        raise ValueError("focused baseline failed")
    mutants = {}
    original = math.contributions
    with patch.object(accounting, "contributions", lambda ids, signs, forces, adjoints:
                      original(ids, np.ones_like(signs), forces, adjoints)):
        mutants["orientation_removed"] = run(unittest.TestSuite([
            accounting.PushAccountingTests("test_signed_parts_match_independent_global_assembly")]))
    with patch.object(accounting, "check_closure", lambda *args, **kwargs: 0.):
        mutants["closure_check_removed"] = run(unittest.TestSuite([
            accounting.PushAccountingTests("test_missing_cell_is_rejected")]))
    original_parts = boundary.cell_parts
    def without_boundary(*args, **kwargs):
        result = original_parts(*args, **kwargs)
        result["faces"] = np.zeros_like(result["faces"])
        return result
    with patch.object(interfaces, "cell_parts", without_boundary):
        mutants["boundary_removed"] = run(unittest.TestSuite([
            interfaces.InterfaceAccountingTests("test_integration_by_parts_matches_direct_force")]))
    with patch.object(interfaces, "cell_parts", lambda *args, **kwargs:
                      original_parts(*args, **(kwargs | {"flip_normals": True}))):
        mutants["outward_normal_flipped"] = run(unittest.TestSuite([
            interfaces.InterfaceAccountingTests("test_integration_by_parts_matches_direct_force")]))
    if any(r["passed"] or r["errors"] or r["failures"] != 1 for r in mutants.values()):
        raise ValueError("mechanism removal did not produce the required assertion failure")
    sources = [Path(__file__), Path(accounting.__file__), Path(interfaces.__file__),
               Path(math.__file__), Path(boundary.__file__)]
    receipt = {"scope": "diagnostic accounting, not a repaired operator or release gate",
               "baseline": baseline, "mutants": mutants,
               "sources": {str(p): sha(p) for p in sources}}
    destination.write_text(json.dumps(receipt, indent=2)+"\n", encoding="utf-8")
    print("8 focused tests passed; 4 mechanism-removal mutants failed by assertion as required.")


if __name__ == "__main__":
    main()
