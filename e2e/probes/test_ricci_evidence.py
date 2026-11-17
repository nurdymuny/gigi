"""Adversarial evidence gates; require an actual captured live run."""
import copy
import json
import os
from pathlib import Path
import tempfile
import unittest
from ricci_evidence import verify


class EvidenceGate(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.original = json.loads(Path(os.environ["RICCI_EVIDENCE"]).read_text(encoding="utf-8"))

    def test_valid_live_evidence(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d)/"live.json"
            p.write_text(json.dumps(self.original),encoding="utf-8")
            self.assertEqual(verify(p)["status"],"PASS")
            self.assertTrue(p.with_suffix(".PASS.json").exists())

    def test_coherent_tampering_cannot_leave_stale_pass(self):
        def time(d): d["requests"][0]["response"]["rows"][1]["t"] += .01
        def metric(d): d["requests"][0]["response"]["rows"][1]["a"] += .1
        def curvature(d): d["requests"][0]["response"]["rows"][0]["scalar_curvature"] = 0
        def partial(d): d["requests"][0]["response"]["rows"].pop()
        def proof(d): d["requests"][0]["response"]["rows"][0]["topology_inferred"] = True
        def boolean_number(d): d["requests"][0]["response"]["rows"][0]["a"] = True
        def missing(d): d["requests"].pop()
        def binary(d): d["binary"]["sha256"] = "0"*64
        def source(d): d["sources"].pop(next(iter(d["sources"])))
        for damage in [time,metric,curvature,partial,proof,boolean_number,missing,binary,source]:
            with self.subTest(damage=damage.__name__), tempfile.TemporaryDirectory() as folder:
                d = copy.deepcopy(self.original)
                damage(d)
                for item in d["requests"]:
                    item["raw_response"] = json.dumps(item["response"])
                p = Path(folder)/"live.json"
                p.write_text(json.dumps(d),encoding="utf-8")
                receipt = p.with_suffix(".PASS.json")
                receipt.write_text('{"status":"PASS"}',encoding="utf-8")
                with self.assertRaises((ValueError,AssertionError)): verify(p)
                self.assertFalse(receipt.exists())

    def test_missing_and_malformed_invalidate_before_read(self):
        for raw in [None,"not json",'{"schema":NaN}','{"schema":1,"schema":2}']:
            with self.subTest(raw=raw), tempfile.TemporaryDirectory() as d:
                p = Path(d)/"live.json"
                if raw is not None: p.write_text(raw,encoding="utf-8")
                receipt = p.with_suffix(".PASS.json")
                receipt.write_text('{"status":"PASS"}',encoding="utf-8")
                with self.assertRaises((ValueError,OSError)): verify(p)
                self.assertFalse(receipt.exists())


if __name__ == "__main__": unittest.main()
