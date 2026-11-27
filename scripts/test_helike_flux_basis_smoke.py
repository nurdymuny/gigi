"""Planted acceptance fixtures, not numerical evidence for the model."""
import copy
import math
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import subprocess
import json

import run_helike_flux_basis_smoke as smoke


def fixture(mq, h=1.05, c=1.09, wc=0.96):
    z = 0.94
    levels = [5.0, 5.0 + h, 5.0 + h, 5.0 + c] + [7.0 + i for i in range(8)]
    result = {"E0": 5.0, "vacuum_weight": z, "levels": levels,
              "in_space_lanczos_residuals": [1e-12] * 12, "reference_weight_error": 1e-8}
    for name, gap, weight, index in [("helike", h, 0.97, 1), ("competitor", c, wc, 3)]:
        result[name] = {"gap": gap, "weight": weight * z, "dressed_weight": weight,
                        "dressed_overlap": 0.999, "level_index": index}
    return {"status": "COMPUTED", "params": {**smoke.POINT, "Mq": mq, "extra": 4,
                                               "solver": dict(smoke.SOLVER)},
            "results": {**smoke.POINT, "Mq": mq, "dimension": math.comb(mq + 6, 6),
                        "moments": "exact", "max_in_space_lanczos_residual": 1e-12,
                        "quadrature_cross_check": 1e-12, "result": result}}


def baseline():
    row = copy.deepcopy(fixture(10)["results"]["result"])
    row["Mq"] = 10
    return row


class Smoke(unittest.TestCase):
    def test_frozen_design(self):
        self.assertEqual(smoke.POINT, {"s": 1.18, "t": 3.0, "g2": 0.03})
        self.assertEqual(smoke.STEP_TOL, 0.003)
        self.assertEqual(smoke.WEIGHT_MIN, 0.9)
        self.assertEqual(smoke.EXPORT_MIN, 0.95)

    def test_good_smoke_never_releases_grid_or_certifies_domain(self):
        a = smoke.assess(fixture(10), fixture(12, c=1.088), baseline())
        self.assertTrue(a["basis_step_pass"])
        self.assertTrue(a["grid_identification_pass"])
        self.assertFalse(a["full_grid_released"])
        self.assertFalse(a["certified"])
        self.assertEqual(a["overall"], "UNRESOLVED")
        self.assertEqual(a["domain_gate"], "NOT_RUN")

    def test_too_large_step_is_not_a_pass(self):
        self.assertFalse(smoke.assess(fixture(10), fixture(12, c=1.086), baseline())["basis_step_pass"])

    def test_dressed_overlap_cannot_replace_reference_weight(self):
        a = smoke.assess(fixture(10), fixture(12, wc=0.89), baseline())
        self.assertFalse(a["grid_identification_pass"])
        self.assertTrue(a["competitor_blocks_export"])

    def test_export_threshold_is_separate(self):
        a = smoke.assess(fixture(10), fixture(12, wc=0.94), baseline())
        self.assertTrue(a["grid_identification_pass"])
        self.assertTrue(a["competitor_blocks_export"])

    def test_bad_reproduction_cannot_pass(self):
        with self.assertRaisesRegex(ValueError, "reproduce"):
            smoke.assess(fixture(10, c=1.091), fixture(12), baseline())

    def test_wrong_point_cutoff_settings_or_operator_refuse(self):
        for field, value in [("s", 1.19), ("t", 4), ("g2", 0.05), ("Mq", 14), ("extra", 8),
                             ("solver", {**smoke.SOLVER, "tol": 1e-5})]:
            with self.subTest(field=field):
                r = fixture(12)
                r["params"][field] = value
                with self.assertRaises(ValueError):
                    smoke.validate(r, 12)
        for field, value in [("moments", "quadrature"), ("dimension", 1), ("g2", 0.05)]:
            r = fixture(12)
            r["results"][field] = value
            with self.assertRaises(ValueError):
                smoke.validate(r, 12)

    def test_bad_residual_empty_levels_nonfinite_and_weights_refuse(self):
        for fault in ("residual", "empty", "nan", "weight", "gap", "reference_error"):
            with self.subTest(fault=fault):
                r = fixture(12)
                q = r["results"]["result"]
                if fault == "residual":
                    q["in_space_lanczos_residuals"][0] = 1e-5
                elif fault == "empty":
                    q["levels"] = []
                    q["in_space_lanczos_residuals"] = []
                elif fault == "nan":
                    q["competitor"]["gap"] = float("nan")
                elif fault == "weight":
                    q["competitor"]["dressed_weight"] = 0.99
                elif fault == "gap":
                    q["competitor"]["gap"] = 2.0
                else:
                    q["reference_weight_error"] = 2e-6
                with self.assertRaises(ValueError):
                    smoke.validate(r, 12)

    def test_existing_evidence_directory_refuses_without_overwrite(self):
        with tempfile.TemporaryDirectory() as d:
            out = Path(d) / "run"
            smoke.new_run(out)
            marker = out / "receipt.json"
            marker.write_text('{"status":"old evidence"}')
            with self.assertRaises(FileExistsError):
                smoke.new_run(out)
            self.assertEqual(marker.read_text(), '{"status":"old evidence"}')

    def test_timeout_or_nonzero_exit_cannot_be_completed(self):
        for failure in (subprocess.TimeoutExpired("helike", 900), subprocess.CompletedProcess("helike", 2)):
            with self.subTest(failure=str(failure)), tempfile.TemporaryDirectory() as d:
                out = Path(d)
                kwargs = {"side_effect": failure} if isinstance(failure, Exception) else {"return_value": failure}
                with patch.object(smoke.subprocess, "run", **kwargs), self.assertRaises(Exception):
                    smoke.execute(Path("helike.exe"), out, 12)
                event = json.loads((out / "mq12-execution.json").read_text())
                self.assertEqual(event["status"], "STOPPED")
                self.assertIn("error", event)
                self.assertFalse((out / "mq12.json").exists())


if __name__ == "__main__":
    unittest.main()
