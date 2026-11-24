"""Focused gates for the registered dome runner, with no new physical measurements."""
import unittest
import json
import tempfile
from pathlib import Path

import run_ym_helike_dome as run


class ExecutionTests(unittest.TestCase):
    def setUp(self):
        self.gates = {f"D{i:02}": True for i in range(6)}
        self.primary = [dict(H1a=True, H1b=True) for _ in range(8)]

    def test_truncation_has_its_registered_verdict(self):
        self.gates["D04"] = False
        self.assertEqual(run.decide(self.primary, self.gates, [], 8), "UNRESOLVED_TRUNCATION")

    def test_incomplete_primary_cannot_be_supported(self):
        self.assertEqual(run.decide(self.primary[:-1], self.gates, [], 8), "INVALID")

    def test_all_top_cutoff_domes_are_repeated(self):
        self.assertEqual(run.repeated_dome_sizes(run.DESIGN), [0, 3, 4])

    def test_each_registered_decision_has_its_own_fixture(self):
        self.assertEqual(run.decide(self.primary, self.gates, [], 8), "SUPPORTED")
        negative = [dict(H1a=False, H1b=True) for _ in range(8)]
        self.assertEqual(run.decide(negative, self.gates, [], 8), "NOT_SUPPORTED")
        mixed = self.primary[:-1] + [dict(H1a=True, H1b=False)]
        self.assertEqual(run.decide(mixed, self.gates, [], 8), "MIXED")

    def test_numerical_failure_and_width_refusal_are_not_scientific_answers(self):
        self.gates["D03"] = False
        self.assertEqual(run.decide(self.primary, self.gates, [], 8), "INVALID")
        self.assertEqual(run.decide([], self.gates, [dict(refused=True)], 8), "REFUSED_BETA_EDGE")

    def test_empty_complete_receipt_rejected_and_stale_pass_cleared(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "run"
            rec = run.initialize(out,run.DESIGN,False)
            rec.update(status="COMPLETE",decision="SUPPORTED",gates=self.gates)
            run.write(out / "receipt.json",rec)
            run.write(out / "verification.json",dict(status="PASS"))
            with self.assertRaises(ValueError):
                run.verify(out)
            self.assertEqual(json.loads((out / "verification.json").read_text())["status"],"FAIL")

    def test_refuses_reusing_output(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "run"
            run.initialize(out,run.DESIGN,False)
            with self.assertRaises(ValueError):
                run.initialize(out,run.DESIGN,False)


if __name__ == "__main__":
    unittest.main(verbosity=2)
