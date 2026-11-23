import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import ricci_boundary_extension as subject


class BoundaryEvidenceTests(unittest.TestCase):
    def test_tamper_refused_and_no_stale_marker(self):
        expected = {"schema": "test", "status": "REFUSED_INCOMPATIBLE_BOUNDARY",
                    "boundary_extension_passed": False, "edge_audit": {"max_gap": .1}}
        for change in ({"boundary_extension_passed": True}, {"edge_audit": {"max_gap": 0.}},
                       {"status": "ACCEPTED"}):
            with self.subTest(change=change), tempfile.TemporaryDirectory() as folder:
                directory = Path(folder)
                (directory/"diagnostic.json").write_text(json.dumps(expected|change), encoding="utf-8")
                (directory/"VERIFIED.json").write_text("stale PASS", encoding="utf-8")
                with patch.object(subject, "build_receipt", return_value=expected, create=True):
                    with self.assertRaisesRegex(ValueError, "replay"):
                        subject.verify(directory)
                self.assertFalse((directory/"VERIFIED.json").exists())

    def test_invalid_json_no_stale_marker(self):
        for raw in ('{"v": NaN}', '{"v":1,"v":2}'):
            with self.subTest(raw=raw), tempfile.TemporaryDirectory() as folder:
                directory = Path(folder)
                (directory/"diagnostic.json").write_text(raw, encoding="utf-8")
                (directory/"VERIFIED.json").write_text("stale PASS", encoding="utf-8")
                with self.assertRaises(ValueError):
                    subject.verify(directory)
                self.assertFalse((directory/"VERIFIED.json").exists())


if __name__ == "__main__":
    unittest.main(verbosity=2)
