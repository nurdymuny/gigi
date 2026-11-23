"""Targeted mechanism removals; no flow solves or project-wide tests."""
import io
import json
from pathlib import Path
import subprocess
import sys
import unittest
from unittest.mock import patch
import numpy as np

import test_ricci_shared_face_math as unit
import test_ricci_shared_face_patch as contract
import ricci_shared_face_math as math
from ricci_shared_face_patch import OUT, sha


def execute(case):
    stream = io.StringIO()
    result = unittest.TextTestRunner(stream=stream, verbosity=2).run(case)
    return {"passed": result.wasSuccessful(), "tests": result.testsRun,
            "failures": len(result.failures), "errors": len(result.errors), "output": stream.getvalue()}


def one(cls, name):
    return unittest.TestSuite([cls(name)])


def worker():
    destination = OUT/"mechanism-removals.json"
    if destination.exists():
        raise ValueError("preserve prior mechanism-removal receipt")
    loader = unittest.defaultTestLoader
    baseline = execute(unittest.TestSuite([loader.loadTestsFromModule(unit), loader.loadTestsFromModule(contract)]))
    if not baseline["passed"] or baseline["tests"] != 20:
        raise ValueError("focused baseline failed")
    mutants = {}
    original = math.trace_report
    with patch.object(unit, "pullback", lambda h, j: math.pullback(h, j)+h[:, 2, None]):
        mutants["normal_component_leaks_into_trace"] = execute(one(unit.SharedFaceTests,
            "test_normal_difference_is_not_tangential_mismatch"))
    with patch.object(unit, "trace_report", lambda left, right, *rest: original(abs(left), abs(right), *rest)):
        mutants["signs_folded"] = execute(one(unit.SharedFaceTests, "test_equal_magnitudes_opposite_signs_fail"))
    def euclidean(left, right, metric, weights, basis, orientation):
        return original(left, right, np.broadcast_to(np.eye(2), metric.shape), weights, basis, orientation)
    with patch.object(unit, "trace_report", euclidean):
        mutants["physical_metric_removed"] = execute(one(unit.SharedFaceTests, "test_metric_norm_not_coordinate_norm"))
    def weak_promoted(*args):
        report = original(*args)
        report["sampled_trace_match"] = report["whitney_moments_match"]
        return report
    with patch.object(unit, "trace_report", weak_promoted):
        mutants["weak_match_promoted_to_strong"] = execute(one(unit.SharedFaceTests, "test_weak_only_match_is_not_promoted"))
    with patch.object(unit, "trace_report", lambda *args: original(*args[:-1], 1)):
        mutants["orientation_ignored"] = execute(one(unit.SharedFaceTests,
            "test_orientation_reversing_chart_preserves_signed_moments"))
    def third_removed(*args):
        report = original(*args); report["moments"][2] = 0.
        return report
    with patch.object(unit, "trace_report", third_removed):
        mutants["third_face_moment_removed"] = execute(one(unit.SharedFaceTests,
            "test_all_three_whitney_moments_match_analytic_integrals"))
    with patch.object(contract, "verify", lambda directory: {"constraint_checker_verified": True}):
        mutants["independent_replay_removed"] = execute(one(contract.PatchConstraintTests,
            "test_forged_averaged_trace_refused_without_stale_receipt"))
    unlink = Path.unlink
    def keep_stale(self, *args, **kwargs):
        if self.name != "VERIFIED.json":
            return unlink(self, *args, **kwargs)
    with patch.object(Path, "unlink", keep_stale):
        mutants["stale_marker_removal_disabled"] = execute(one(contract.PatchConstraintTests,
            "test_missing_face_refused_without_stale_receipt"))
    if any(result["passed"] or result["errors"] or result["failures"] != 1 for result in mutants.values()):
        raise ValueError("a mechanism removal did not trigger its expected assertion failure")
    source_paths = [Path(__file__), Path(unit.__file__), Path(contract.__file__), Path(math.__file__)]
    receipt = {"scope": "constraint checker only, not operator repair", "baseline": baseline,
               "mutants": mutants, "sources": {str(p): sha(p) for p in source_paths}}
    destination.write_text(json.dumps(receipt, indent=2)+"\n", encoding="utf-8")
    print("20 focused tests passed. All 8 mechanism removals caused assertion failures, not setup errors.")


if __name__ == "__main__":
    if "--worker" in sys.argv:
        worker()
    else:
        subprocess.run([sys.executable, __file__, "--worker"], check=True, timeout=90)
