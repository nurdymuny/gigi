"""Report gates: evidence identity, faithful numbers, geometry and no false PASS."""

import copy
import json
from pathlib import Path
import unittest

import numpy as np
import build_ym_helike_report as report

ROOT = Path(__file__).resolve().parents[1]
RUN = ROOT / "research/helike-quantum-probe-20261123/survey-20261124T021903Z-c326a3c5"


class ReportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.receipt = json.loads((RUN / "receipt.json").read_text())
        cls.verification = json.loads((RUN / "verification.json").read_text())

    def test_plot_rows_are_the_recorded_results(self):
        payload = report.make_payload(self.receipt, self.verification)
        self.assertEqual(payload["status"], "UNRESOLVED")
        self.assertEqual(len(payload["cases"]), 15)
        strongest = payload["final"][2]
        self.assertAlmostEqual(strongest["gap"], 3.094157392269535, places=12)
        self.assertAlmostEqual(strongest["H2_weight"], 0.991539953302153, places=14)
        self.assertEqual([r["lowest_spin"] for r in payload["cases"] if r["g2"] == 2], [2, 2, 0, 2, 2])

    def test_stale_verification_and_forged_pass_are_refused(self):
        bad = dict(self.verification, run_id="old")
        with self.assertRaisesRegex(ValueError, "verification"):
            report.make_payload(self.receipt, bad)
        bad = dict(self.receipt, status="PASS")
        with self.assertRaisesRegex(ValueError, "UNRESOLVED"):
            report.make_payload(bad, self.verification)

    def test_altered_gap_and_nan_weight_are_refused(self):
        bad = copy.deepcopy(self.receipt)
        bad["couplings"][0]["cases"][-1]["absolute_gap"] += 1
        with self.assertRaisesRegex(ValueError, "gap"):
            report.make_payload(bad, self.verification)
        bad = copy.deepcopy(self.receipt)
        bad["couplings"][0]["cases"][-1]["H2"]["lowest_weight"] = float("nan")
        with self.assertRaisesRegex(ValueError, "finite"):
            report.make_payload(bad, self.verification)

    def test_missing_case_and_changed_level_weight_are_refused(self):
        bad = copy.deepcopy(self.receipt)
        bad["couplings"][0]["cases"].pop(0)
        with self.assertRaisesRegex(ValueError, "cutoff"):
            report.make_payload(bad, self.verification)
        bad = copy.deepcopy(self.receipt)
        bad["couplings"][0]["cases"][-1]["H2"]["levels"][1]["weight"] = .1
        with self.assertRaisesRegex(ValueError, "weight"):
            report.make_payload(bad, self.verification)

    def test_gate_ratios_use_registered_thresholds(self):
        row = report.make_payload(self.receipt, self.verification)["final"][0]
        self.assertAlmostEqual(row["residual_ratio"], .8994883348247144/.022898146240874235)
        self.assertAlmostEqual(row["H0_tail_ratio"], .005604987952301565/.001)
        self.assertGreater(row["residual_ratio"], 1)

    def test_geometry_is_the_declared_tensor_not_artwork(self):
        bc, bs = report.quadratures()
        self.assertAlmostEqual(np.trace(bc), 0)
        self.assertAlmostEqual(np.trace(bs), 0)
        self.assertAlmostEqual(np.sum(bc*bc), 1)
        self.assertAlmostEqual(np.sum(bs*bs), 1)
        self.assertAlmostEqual(np.sum(bc*bs), 0)
        e1 = np.array([0., 1., 0.])
        diagonal = np.array([0., 1., 1.])/np.sqrt(2)
        self.assertAlmostEqual(e1@bc@e1, 1/np.sqrt(2))
        self.assertAlmostEqual(e1@bs@e1, 0)
        self.assertAlmostEqual(diagonal@bs@diagonal, 1/np.sqrt(2))


if __name__ == "__main__":
    unittest.main(verbosity=2)
