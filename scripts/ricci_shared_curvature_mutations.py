"""Remove one mechanism at a time in memory; never edits production/evidence files."""
import argparse
import io
import json
from pathlib import Path
import subprocess
import sys
import unittest
from unittest.mock import patch
import numpy as np
import ricci_shared_curvature_patch as construction
import ricci_shared_curvature_math as algebra
import test_ricci_shared_curvature_math as algebra_tests
import test_ricci_shared_curvature_patch as patch_tests
import test_ricci_shared_curvature_evidence as evidence_tests
from ricci_local_push_audit import sha


def nonclosed_basis(degree):
    basis = algebra.closed_basis(degree).copy()
    if degree:
        basis[algebra.powers(degree).index((1, 0, 0)), 0] += 1.
    return basis


def discard_flux(mass, flux_map, data):
    return algebra.minimum_energy(mass, flux_map, np.zeros_like(data))


def main():
    parser = argparse.ArgumentParser(); parser.add_argument("--worker", action="store_true")
    args = parser.parse_args()
    if not args.worker:
        subprocess.run([sys.executable, __file__, "--worker"], check=True, timeout=180)
        return
    output = construction.OUT/"mechanism-removals.json"
    if output.exists():
        raise ValueError("preserve mutation evidence")
    jobs = [
        ("closed coefficient space", algebra_tests, "closed_basis", nonclosed_basis,
         algebra_tests.SharedCurvatureMathTests, "test_closed_basis_dimensions_and_coefficient_closure"),
        ("exact flux enforcement", algebra_tests, "minimum_energy", discard_flux,
         algebra_tests.SharedCurvatureMathTests, "test_minimum_energy_matches_independent_analytic_solution"),
        ("edge gauge correction", construction, "recover_gauge", lambda p, r: np.zeros(8),
         patch_tests.SharedPatchTests, "test_preserves_real_edges_curvature_and_all_shared_traces"),
        ("two-form coordinate pullback", construction, "two_form_pullback", lambda v, j: v,
         patch_tests.SharedPatchTests, "test_preserves_real_edges_curvature_and_all_shared_traces"),
        ("reconstruction derivative", construction, "energy_gradient", lambda r, m, c: np.zeros(r.shape[1]),
         patch_tests.SharedPatchTests, "test_preserves_real_edges_curvature_and_all_shared_traces"),
        ("stale PASS cleanup", Path, "unlink", lambda *a, **k: None,
         evidence_tests.EvidenceTests, "test_bad_evidence_removes_stale_verified_marker"),
    ]
    receipts = []
    for name, module, attribute, replacement, case, test in jobs:
        stream = io.StringIO()
        with patch.object(module, attribute, replacement):
            result = unittest.TextTestRunner(stream=stream, verbosity=2).run(unittest.TestSuite([case(test)]))
        receipt = {"removed": name, "caught": bool(result.failures), "failures": len(result.failures),
                   "errors": len(result.errors), "output": stream.getvalue()}
        receipts.append(receipt)
        print(name, "CAUGHT by assertion" if receipt["caught"] else "NOT CAUGHT by assertion")
    files = [Path(__file__), Path(construction.__file__), Path(algebra.__file__),
             Path(algebra_tests.__file__), Path(patch_tests.__file__), Path(evidence_tests.__file__)]
    output.write_text(json.dumps({"mechanisms": receipts, "sources": {str(f): sha(f) for f in files}}, indent=2)+"\n", encoding="utf-8")
    if not all(r["caught"] and r["errors"] == 0 for r in receipts):
        raise ValueError("a mechanism-removal gate failed to detect the broken result by assertion")


if __name__ == "__main__":
    main()
