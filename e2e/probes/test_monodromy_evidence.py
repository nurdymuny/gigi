"""Adversarial evidence tests. Synthetic receipts are not live-engine evidence."""
import json
import os
from pathlib import Path
import tempfile
import unittest

from monodromy_evidence import verify


class EvidenceGate(unittest.TestCase):
    def test_malformed_input_removes_stale_pass(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            path = root / "live.json"
            path.write_text("{broken", encoding="utf-8")
            receipt = root / "live.PASS.json"
            receipt.write_text('{"status":"PASS"}', encoding="utf-8")
            with self.assertRaises((ValueError, AssertionError)):
                verify(path)
            self.assertFalse(receipt.exists())

    def test_missing_input_removes_stale_pass(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            receipt = root / "live.PASS.json"
            receipt.write_text('{"status":"PASS"}', encoding="utf-8")
            with self.assertRaises(FileNotFoundError):
                verify(root / "live.json")
            self.assertFalse(receipt.exists())

    def test_forged_acceptance_fields_are_not_evidence(self):
        with tempfile.TemporaryDirectory() as d:
            path = Path(d) / "live.json"
            path.write_text(json.dumps({"status": "PASS", "group_order": 120}), encoding="utf-8")
            with self.assertRaises((ValueError, AssertionError)):
                verify(path)
            self.assertFalse(path.with_suffix(".PASS.json").exists())

    @unittest.skipUnless(os.environ.get("MONODROMY_EVIDENCE"), "requires a completed live run")
    def test_live_receipt_and_tampering(self):
        original = json.loads(Path(os.environ["MONODROMY_EVIDENCE"]).read_text(encoding="utf-8"))
        with tempfile.TemporaryDirectory() as d:
            path = Path(d) / "live.json"
            path.write_text(json.dumps(original), encoding="utf-8")
            self.assertEqual(verify(path)["status"], "PASS")
            def wrong_answer(x):
                next(r for r in x["requests"] if r["label"] == "icosahedral")["response"]["rows"][0]["group_order"] = 119
            def missing_case(x):
                x["requests"] = [r for r in x["requests"] if r["label"] != "identity"]
            def false_certificate(x):
                next(r for r in x["requests"] if r["label"] == "identity")["response"]["rows"][0]["exact_subgroup_certified"] = True
            def wrong_binary(x):
                x["binary"]["sha256"] = "0" * 64
            def nonfinite(x):
                next(r for r in x["requests"] if r["label"] == "identity")["response"]["rows"][0]["max_closure_residual"] = float("nan")
            def wrong_histogram(x):
                next(r for r in x["requests"] if r["label"] == "icosahedral")["response"]["rows"][0]["element_order_counts"][2] = 19
            for mutate in [wrong_answer, missing_case, false_certificate, wrong_binary, nonfinite, wrong_histogram]:
                with self.subTest(mutation=mutate.__name__):
                    changed = json.loads(json.dumps(original))
                    mutate(changed)
                    # Alter both representations: rejection must reach the mathematical
                    # contract, not succeed merely because raw and parsed copies differ.
                    for record in changed["requests"]:
                        record["raw_response"] = json.dumps(record["response"])
                    path.with_suffix(".PASS.json").write_text('{"status":"PASS"}', encoding="utf-8")
                    path.write_text(json.dumps(changed), encoding="utf-8")
                    with self.assertRaises(ValueError):
                        verify(path)
                    self.assertFalse(path.with_suffix(".PASS.json").exists())


if __name__ == "__main__":
    unittest.main()
