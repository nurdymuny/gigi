"""Attack real RF2A evidence, including mutually consistent forged responses."""
import copy
import json
import os
from pathlib import Path
import tempfile
import unittest
from ricci_wilson_evidence import verify


class EvidenceGate(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.original = json.loads(Path(os.environ["RICCI_WILSON_EVIDENCE"]).read_text(encoding="utf-8"))

    def test_valid_live_evidence(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d)/"live.json"
            p.write_text(json.dumps(self.original), encoding="utf-8")
            self.assertEqual(verify(p)["status"], "PASS")
            self.assertTrue(p.with_suffix(".PASS.json").exists())

    def test_coherent_tampering_cannot_leave_stale_pass(self):
        def row(d):
            return next(i["response"]["rows"][0] for i in d["requests"] if i["label"] == "sphere")
        def mapped(d): row(d)["mapped_ricci_velocity"][0] += .1
        def gradient(d): row(d)["yang_mills_velocity"][0] += .1
        def circular(d):
            r = row(d)
            r["mapped_ricci_velocity"] = r["yang_mills_velocity"] = [0.,0.,0.]
            for k in ["absolute_residual", "relative_residual", "mapped_velocity_norm", "yang_mills_velocity_norm"]:
                r[k] = 0.
            r["residual"] = [0.,0.,0.]
        def invented_residual(d):
            r = next(i["response"]["rows"][0] for i in d["requests"]
                     if i["status"] == 200 and "rows" in i["response"]
                     and any(i["response"]["rows"][0].get("residual", [])))
            r["residual"] = [0.,0.,0.]
            r["absolute_residual"] = r["relative_residual"] = 0.
        def clock(d): row(d)["time_scale"] = 2.
        def boolean_clock(d): row(d)["time_scale"] = True
        def fitted(d): row(d)["time_scale_fitted"] = True
        def lattice(d): row(d)["lattice_correspondence_tested"] = True
        def normalization(d): row(d)["normalized"] = True
        def boolean(d): row(d)["connection"][0] = True
        def missing(d): d["requests"].pop()
        def duplicate(d): d["requests"].append(d["requests"][0])
        def binary(d): d["binary"]["sha256"] = "0"*64
        def source(d): d["sources"].pop(next(iter(d["sources"])))
        def revision(d): d["head"] = "0"*40
        def reference(d): d["reference_experiments"]["berger_0"]["fixed_background"][-1][0] += .1
        def symbolic(d): d["symbolic"]["h1_residuals"][0] = "1"
        def quotient(d): d["quotient_checks"][0]["quotient_mass"][0] *= 120
        for damage in [mapped,gradient,circular,invented_residual,clock,boolean_clock,fitted,lattice,normalization,
                       boolean,missing,duplicate,binary,source,revision,reference,symbolic,quotient]:
            with self.subTest(damage=damage.__name__), tempfile.TemporaryDirectory() as folder:
                d = copy.deepcopy(self.original)
                damage(d)
                for item in d["requests"]:
                    item["raw_response"] = json.dumps(item["response"])
                p = Path(folder)/"live.json"
                p.write_text(json.dumps(d), encoding="utf-8")
                receipt = p.with_suffix(".PASS.json")
                receipt.write_text('{"status":"PASS"}', encoding="utf-8")
                with self.assertRaises((ValueError, AssertionError)):
                    verify(p)
                self.assertFalse(receipt.exists())

    def test_missing_and_malformed_invalidate_before_read(self):
        for raw in [None,"not json",'{"schema":NaN}','{"schema":1,"schema":2}']:
            with self.subTest(raw=raw), tempfile.TemporaryDirectory() as d:
                p = Path(d)/"live.json"
                if raw is not None:
                    p.write_text(raw, encoding="utf-8")
                receipt = p.with_suffix(".PASS.json")
                receipt.write_text('{"status":"PASS"}', encoding="utf-8")
                with self.assertRaises((ValueError, OSError)):
                    verify(p)
                self.assertFalse(receipt.exists())


if __name__ == "__main__":
    unittest.main()
