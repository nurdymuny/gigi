import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import ricci_shared_curvature_patch as subject


class EvidenceTests(unittest.TestCase):
    def test_bad_evidence_removes_stale_verified_marker(self):
        expected = {"schema": "test", "polynomial": [1.], "report": {"edge_error": 0.},
                    "fluxes": [1., 2.], "local_gate_passed": True}
        attacks = [
            {**expected, "polynomial": [0.]},
            {**expected, "fluxes": [1.]},
            {**expected, "report": {"edge_error": 1e-3}},
            {**expected, "extra_acceptance": True},
        ]
        for bad in attacks:
            with self.subTest(bad=bad), tempfile.TemporaryDirectory() as folder:
                directory = Path(folder)
                (directory/"diagnostic.json").write_text(json.dumps(bad), encoding="utf-8")
                (directory/"VERIFIED.json").write_text("stale PASS", encoding="utf-8")
                with patch.object(subject, "build_receipt", return_value=expected, create=True):
                    with self.assertRaisesRegex(ValueError, "replay"):
                        subject.verify(directory)
                self.assertFalse((directory/"VERIFIED.json").exists())

    def test_invalid_json_removes_stale_verified_marker(self):
        for raw in ('{"v": NaN}', '{"v": 1,"v": 2}'):
            with self.subTest(raw=raw), tempfile.TemporaryDirectory() as folder:
                directory = Path(folder)
                (directory/"diagnostic.json").write_text(raw, encoding="utf-8")
                (directory/"VERIFIED.json").write_text("stale PASS", encoding="utf-8")
                with self.assertRaises(ValueError):
                    subject.verify(directory)
                self.assertFalse((directory/"VERIFIED.json").exists())


if __name__ == "__main__":
    unittest.main(verbosity=2)
