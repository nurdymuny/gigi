"""Focused boundary tests, mechanism removals and real evidence attacks."""
import argparse
import copy
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
import numpy as np
import ricci_boundary_extension as subject
import test_ricci_boundary_extension as tests
import test_ricci_boundary_evidence as evidence_tests
import test_ricci_shared_curvature_patch as prior_tests
from ricci_local_push_audit import sha

ORIGINAL_LIFT = subject.exact_lift
ORIGINAL_COMPONENT = subject.unit_component


def bypass_refusal(*args):
    result = ORIGINAL_LIFT(*args)
    if result["status"] != "ACCEPTED":
        result = {**result, "status": "ACCEPTED", "coefficients": np.asarray(args[3]).copy()}
    return result


def omit_derivative(*args):
    result = ORIGINAL_LIFT(*args)
    if result["status"] == "ACCEPTED":
        result["base_map"] = np.eye(len(args[3]))
        result["target_map"] = np.zeros((len(args[3]), len(args[4])))
    return result


def run_tests(suite):
    stream = io.StringIO()
    result = unittest.TextTestRunner(stream=stream, verbosity=2).run(suite)
    return {"tests": result.testsRun, "passed": result.wasSuccessful(), "failures": len(result.failures),
            "errors": len(result.errors), "output": stream.getvalue()}


def work():
    output = subject.OUT/"final-gate.json"
    if output.exists():
        raise ValueError("preserve final gate evidence")
    suites = unittest.TestSuite()
    for module in (tests, evidence_tests, prior_tests):
        suites.addTests(unittest.defaultTestLoader.loadTestsFromModule(module))
    green = run_tests(suites)
    print(green["output"], flush=True)
    if not green["passed"]:
        raise ValueError("focused tests failed")
    jobs = [
        ("flux-nullspace restriction", subject, "null_space", lambda a, rcond: np.eye(a.shape[1]),
         tests.LiftTests, "test_conflicting_flux_and_trace_refuse"),
        ("incompatible-target refusal", subject, "exact_lift", bypass_refusal,
         tests.LiftTests, "test_incompatible_targets_refuse_no_coefficients"),
        ("physical normalization", subject, "unit_component", lambda h, d, g: h@d,
         tests.LiftTests, "test_oriented_physical_component"),
        ("orientation sign", subject, "unit_component", lambda *a: abs(ORIGINAL_COMPONENT(*a)),
         tests.LiftTests, "test_oriented_physical_component"),
        ("full boundary derivative", subject, "exact_lift", omit_derivative,
         tests.LiftTests, "test_complete_linear_derivative"),
        ("stale marker cleanup", Path, "unlink", lambda *a, **k: None,
         evidence_tests.BoundaryEvidenceTests, "test_tamper_refused_and_no_stale_marker"),
    ]
    removals = []
    for name, module, attribute, replacement, case, test in jobs:
        with patch.object(module, attribute, replacement):
            result = run_tests(unittest.TestSuite([case(test)]))
        removals.append({"removed": name, **result})
        print(name, "caught" if result["failures"] and not result["errors"] else "NOT CAUGHT", flush=True)
    if not all(r["failures"] and not r["errors"] for r in removals):
        (subject.OUT/"failed-removals.json").write_text(json.dumps(removals, indent=2)+"\n", encoding="utf-8")
        raise ValueError("removal tests must fail by assertion")
    for phase in ("run", "verify"):
        subprocess.run([sys.executable, subject.__file__, phase], check=True, timeout=180)
    diagnostic = json.loads((subject.OUT/"diagnostic.json").read_text(encoding="utf-8"))
    if diagnostic["boundary_extension_passed"] or diagnostic["status"] != "REFUSED_INCOMPATIBLE_BOUNDARY":
        raise ValueError("observed incompatible real case cannot be labeled a successful extension")
    attacks = []
    for name in ("changed edge value", "omitted boundary edge", "omitted boundary face", "forged acceptance",
                 "changed source hash", "nonfinite number", "duplicate key"):
        bad = copy.deepcopy(diagnostic)
        if name == "changed edge value":
            bad["edge_audit"]["edges"][0]["sides"][0]["values"][0] += .01
        elif name == "omitted boundary edge":
            bad["edge_audit"]["edges"].pop()
        elif name == "omitted boundary face":
            bad["linear_certificate"]["boundary_faces"].pop()
        elif name == "forged acceptance":
            bad["boundary_extension_passed"] = True
        elif name == "changed source hash":
            bad["sources"][next(iter(bad["sources"]))] = "0"*64
        elif name == "nonfinite number":
            bad["edge_audit"]["max_gap"] = float("nan")
        raw = json.dumps(bad)
        if name == "duplicate key":
            raw = '{"schema":"forged",'+raw[1:]
        with tempfile.TemporaryDirectory(prefix="gigi-boundary-attack-") as folder:
            directory = Path(folder)
            (directory/"diagnostic.json").write_text(raw, encoding="utf-8")
            (directory/"VERIFIED.json").write_text("stale PASS", encoding="utf-8")
            try:
                subject.verify(directory)
            except ValueError as error:
                result = {"attack": name, "refused": True, "reason": str(error),
                          "stale_marker_removed": not (directory/"VERIFIED.json").exists()}
            else:
                result = {"attack": name, "refused": False, "stale_marker_removed": False}
        attacks.append(result); print(result, flush=True)
    files = [Path(__file__), Path(subject.__file__), Path(tests.__file__), Path(evidence_tests.__file__),
        Path(prior_tests.__file__), subject.ROOT/"docs/RICCI_BOUNDARY_EXTENSION_SPEC.md"]
    artifacts = [subject.OUT/name for name in ("red.json", "green.json", "evidence-red.json", "evidence-green.json",
                                              "diagnostic.json", "VERIFIED.json")]
    accepted = all(a["refused"] and a["stale_marker_removed"] for a in attacks)
    receipt = {"checker_verified": accepted, "boundary_extension_passed": False,
        "global_gate": "UNCHANGED_OPEN", "full_suite_run": False, "tests": green,
        "mechanism_removals": removals, "evidence_attacks": attacks,
        "sources": {str(f): sha(f) for f in files}, "artifacts": {str(f): sha(f) for f in artifacts}}
    output.write_text(json.dumps(receipt, indent=2, allow_nan=False)+"\n", encoding="utf-8")
    if not accepted:
        raise ValueError("evidence attack gate failed")


def main():
    parser = argparse.ArgumentParser(); parser.add_argument("--worker", action="store_true")
    args = parser.parse_args()
    try:
        if args.worker:
            work()
        else:
            subprocess.run([sys.executable, __file__, "--worker"], check=True, timeout=180)
    except BaseException:
        (subject.OUT/"VERIFIED.json").unlink(missing_ok=True)
        raise


if __name__ == "__main__":
    main()
