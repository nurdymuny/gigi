import copy
import json
from pathlib import Path
import tempfile
import unittest
from ricci_shared_face_patch import OUT, build_receipt, verify


class PatchConstraintTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        OUT.mkdir(parents=True, exist_ok=True)
        cls.truth = build_receipt()

    def test_smooth_reference_matches_all_six_faces(self):
        self.assertEqual([f["face"] for f in self.truth["faces"]], [0, 3, 8, 9, 1009, 1012])
        for face in self.truth["faces"]:
            self.assertTrue(face["oracle_control"]["sampled_trace_match"])
            self.assertTrue(face["oracle_control"]["whitney_moments_match"])

    def test_original_field_rejects_known_face_mismatches(self):
        for face in self.truth["faces"]:
            self.assertFalse(face["frozen_whitney"]["sampled_trace_match"])
            self.assertFalse(face["frozen_whitney"]["whitney_moments_match"])
            self.assertLessEqual(face["saved_readout_replay_error"], 1e-9)

    def test_clean_evidence_is_verified_without_repair_claim(self):
        with tempfile.TemporaryDirectory(dir=OUT) as tmp:
            path = Path(tmp)
            (path/"diagnostic.json").write_text(json.dumps(self.truth), encoding="utf-8")
            result = verify(path)
            self.assertTrue(result["constraint_checker_verified"])
            self.assertFalse(result["original_patch_compatible"])
            self.assertEqual(result["release_gate"], "UNCHANGED_OPEN")

    def refused(self, modify):
        damaged = copy.deepcopy(self.truth)
        modify(damaged)
        with tempfile.TemporaryDirectory(dir=OUT) as tmp:
            path = Path(tmp)
            (path/"diagnostic.json").write_text(json.dumps(damaged), encoding="utf-8")
            marker = path/"VERIFIED.json"
            marker.write_text('{"stale":true}', encoding="utf-8")
            with self.assertRaises(ValueError):
                verify(path)
            self.assertFalse(marker.exists(), "bad evidence left a stale verified receipt")

    def test_missing_face_refused_without_stale_receipt(self):
        self.refused(lambda r: r["faces"].pop())

    def test_forged_averaged_trace_refused_without_stale_receipt(self):
        def forge(receipt):
            for face in receipt["faces"]:
                report = face["frozen_whitney"]
                report.update(sampled_trace_match=True, whitney_moments_match=True,
                              physical_jump_max=0., physical_jump_l2=0., moments=[0., 0., 0.])
        self.refused(forge)

    def test_modified_moment_refused_without_stale_receipt(self):
        self.refused(lambda r: r["faces"][0]["frozen_whitney"]["moments"].__setitem__(0, .123))

    def test_nonfinite_refused_without_stale_receipt(self):
        self.refused(lambda r: r["faces"][0]["frozen_whitney"].__setitem__("physical_jump_max", float("nan")))

    def test_source_hash_drift_refused_without_stale_receipt(self):
        self.refused(lambda r: r["sources"].__setitem__(next(iter(r["sources"])), "0"*64))

    def test_orientation_forgery_refused_without_stale_receipt(self):
        self.refused(lambda r: r["faces"][0].__setitem__("orientation", -r["faces"][0]["orientation"]))


if __name__ == "__main__":
    unittest.main(verbosity=2)
