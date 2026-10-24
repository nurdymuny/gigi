"""Deterministic decision tests; live transport/WAL are tested separately."""

from contextlib import redirect_stdout
from copy import deepcopy
import io
import os
from pathlib import Path
import subprocess
import sys
import unittest
from unittest.mock import patch

import registry_bootstrap as db
import registry_gates as gates
import registry_paths as paths
import registry_review as review

GOOD = "DG-REL-DAVISMANIFOLD-PULLBACK-001"
SID = "DG-REF-COORDINATE-SAMENESS-001"
START = "davis-manifold-metric-construction"
SIG = GOOD + ">DG-REL-PULLBACK-JACOBIAN-001"


class RegistryTests(unittest.TestCase):
    def setUp(self):
        fixtures = (db.SOURCES, db.CONCEPTS, db.RELATIONS, db.EVIDENCE,
                    db.ASSUMPTIONS, db.CLAIMS, db.CHECKS, [])
        names = ("sources", "concepts", "relations", "relation_evidence",
                 "relation_assumptions", "claims", "checks", "path_reviews")
        self.store = {"research_" + name: {r["id"]: deepcopy(r) for r in rows}
                      for name, rows in zip(names, fixtures)}
        self.transport = patch.object(db, "call", self.call)
        self.transport.start()
        self.addCleanup(self.transport.stop)
        gates.FAILS.clear()
        self.addCleanup(gates.FAILS.clear)

    def call(self, path, payload=None, method=None):
        _, _, _, name, operation = path.split("/")
        records = self.store[name]
        if operation == "query":
            rows = [deepcopy(r) for r in records.values()
                    if all(r.get(c["field"]) == c["value"]
                           for c in payload.get("conditions", []))]
            return 200, {"data": rows[:payload["limit"]]}
        if operation == "insert":
            records.update({r["id"]: deepcopy(r) for r in payload["records"]})
            return 200, {"inserted": len(payload["records"])}
        raise AssertionError(f"unexpected request: {path}")

    def ruling(self, state="incompatible", **kwargs):
        args = dict(sig=SIG, state=state, reason="planted reviewer decision",
                    conditions="full rank J", reviewer="test reviewer", reviewer_is_agent=True)
        args.update(kwargs)
        return paths.record_composition_review(**args)

    def test_promotion_changes_stored_altitude(self):
        self.assertEqual(review.relation(GOOD)["established_altitude"], "NONE")
        ok, reason = review.promote(GOOD, "RESONANCE")
        self.assertTrue(ok, reason)
        self.assertEqual(review.relation(GOOD)["established_altitude"], "RESONANCE")

    def test_gate_rejects_promotion_without_write(self):
        with patch.object(gates, "promote", review.requirement_met), redirect_stdout(io.StringIO()):
            gates.gate2()
        self.assertEqual(gates.FAILS, ["G2b promotion changes stored NONE to RESONANCE"])

    def test_missing_sources_refuse_and_flag(self):
        self.store["research_sources"].clear()
        self.assertFalse(review.promote(GOOD, "RESONANCE")[0])
        v = review.view(GOOD)
        self.assertTrue(v["needs_review"])
        self.assertEqual({s["availability"] for s in v["sources"]}, {"missing"})

    def test_blank_locators_refuse_and_flag(self):
        for e in self.store["research_relation_evidence"].values():
            e["locator"] = "   "
        for altitude in ("INCIDENCE", "RESONANCE"):
            with self.subTest(altitude=altitude):
                self.assertFalse(review.promote(GOOD, altitude)[0])
        self.assertTrue(review.view(GOOD)["needs_review"])

    def test_one_missing_side_does_not_earn_resonance(self):
        self.store["research_relation_evidence"]["EV-002"]["source_id"] = "missing"
        self.assertFalse(review.promote(GOOD, "RESONANCE")[0])
        self.assertTrue(review.promote(GOOD, "INCIDENCE")[0])

    def test_retracted_source_refuses(self):
        self.store["research_sources"][SID]["availability"] = "retracted"
        self.assertFalse(review.promote(GOOD, "RESONANCE")[0])
        self.assertTrue(review.view(GOOD)["needs_review"])

    def test_unavailable_is_not_retracted(self):
        self.store["research_sources"][SID]["availability"] = "unavailable"
        self.assertTrue(review.promote(GOOD, "RESONANCE")[0])
        v = review.view(GOOD)
        self.assertFalse(v["needs_review"])
        self.assertTrue(any("unavailable" in f for f in v["flags"]))

    def test_unknown_availability_refuses(self):
        self.store["research_sources"][SID]["availability"] = "typo"
        self.assertFalse(review.promote(GOOD, "RESONANCE")[0])
        self.assertTrue(review.view(GOOD)["needs_review"])

    def test_structural_control_is_required(self):
        check = dict(id="TEST", relation_id=GOOD, result="pass", control_ran="true",
                     control_failed_as_required="false")
        self.store["research_checks"]["TEST"] = check
        self.assertFalse(review.promote(GOOD, "STRUCTURAL")[0])
        check["control_failed_as_required"] = "true"
        self.assertTrue(review.promote(GOOD, "STRUCTURAL")[0])
        self.assertEqual(review.relation(GOOD)["established_altitude"], "STRUCTURAL")

    def reject_writes(self, path, payload=None, method=None):
        return (422, {"error": "planted rejection"}) if path.endswith("/insert") else self.call(path, payload, method)

    def ignore_writes(self, path, payload=None, method=None):
        return (200, {"status": "ok"}) if path.endswith("/insert") else self.call(path, payload, method)

    def test_failed_promotion_write_cannot_succeed(self):
        with patch.object(db, "call", self.reject_writes):
            ok, reason = review.promote(GOOD, "RESONANCE")
        self.assertFalse(ok)
        self.assertIn("422", reason)
        self.assertEqual(review.relation(GOOD)["established_altitude"], "NONE")

    def test_noop_promotion_write_cannot_succeed(self):
        with patch.object(db, "call", self.ignore_writes):
            ok, reason = review.promote(GOOD, "RESONANCE")
        self.assertFalse(ok)
        self.assertIn("readback", reason)

    def test_review_write_failure_cannot_succeed(self):
        with patch.object(db, "call", self.reject_writes), self.assertRaisesRegex(RuntimeError, "422"):
            self.ruling()

    def test_review_noop_write_cannot_succeed(self):
        with patch.object(db, "call", self.ignore_writes), self.assertRaisesRegex(RuntimeError, "readback"):
            self.ruling()

    def test_review_key_stable_across_processes(self):
        code = "import registry_paths as p; print(p.review_id(" + repr(SIG) + "))"
        results = [subprocess.check_output([sys.executable, "-c", code],
                   cwd=Path(__file__).parent, env=dict(os.environ, PYTHONHASHSEED=seed), text=True).strip()
                   for seed in ("1", "2")]
        self.assertEqual(results[0], results[1])
        self.assertEqual(results[0], paths.review_id(SIG))

    def test_replacement_ruling_wins_over_legacy(self):
        rid = self.ruling()
        legacy = deepcopy(self.store["research_path_reviews"].pop(rid))
        legacy["id"] = "PR-old-random-id"
        self.store["research_path_reviews"][legacy["id"]] = legacy
        self.ruling("compatible_under", reason="replacement decision")
        found = paths._review(SIG)
        self.assertEqual(found["composition_state"], "compatible_under")
        self.assertEqual(found["reason"], "replacement decision")
        self.assertEqual(found["id"], rid)

    def test_ambiguous_legacy_requires_explicit_replacement(self):
        rid = self.ruling()
        record = self.store["research_path_reviews"].pop(rid)
        for i in (1, 2):
            self.store["research_path_reviews"][str(i)] = dict(record, id=str(i))
        with self.assertRaisesRegex(ValueError, "ambiguous legacy"):
            paths._review(SIG)
        self.ruling("compatible_under")
        self.assertEqual(paths._review(SIG)["composition_state"], "compatible_under")

    def test_path_retraction_warning_survives_hydration(self):
        self.assertTrue(review.promote(GOOD, "RESONANCE")[0])
        self.store["research_sources"][SID]["availability"] = "retracted"
        path = next(p for p in paths.traverse(START) if p["length"] == 1)
        self.assertTrue(path["needs_review"])
        edge = path["edges"][0]
        self.assertTrue(edge["needs_review"])
        self.assertEqual(edge["flags"], review.view(GOOD)["flags"])
        self.assertEqual(edge["established_altitude"], "RESONANCE")
        self.assertIn("retracted", {s["availability"] for s in edge["evidence"]})

    def test_invalid_rulings_refused(self):
        cases = [dict(state="anything_goes"), dict(state=paths.UNCHECKED),
                 dict(reason=" "), dict(reviewer=""), dict(reviewer_is_agent="false"),
                 dict(state="compatible_under", conditions=" ")]
        for case in cases:
            with self.subTest(case=case), self.assertRaises(ValueError):
                self.ruling(**case)
        self.assertEqual(self.store["research_path_reviews"], {})

    def test_invalid_paths_refused(self):
        for sig in ("missing", "", GOOD + ">", GOOD + ">" + GOOD,
                    "DG-REL-PULLBACK-JACOBIAN-001>" + GOOD):
            with self.subTest(sig=sig), self.assertRaises(ValueError):
                self.ruling(sig=sig)
        self.assertEqual(self.store["research_path_reviews"], {})

    def test_symmetric_path_can_be_entered_from_either_end(self):
        self.store["research_relations"][GOOD]["direction"] = "symmetric"
        self.store["research_relations"][GOOD]["subject"], self.store["research_relations"][GOOD]["object"] = (
            self.store["research_relations"][GOOD]["object"], self.store["research_relations"][GOOD]["subject"])
        self.assertEqual(self.ruling(), paths.review_id(SIG))

    def set_claim_pair(self, value1, value2, kind):
        a = dict(db.CLAIMS[0], id="A", value=value1, value_kind=kind)
        b = dict(a, id="B", value=value2)
        self.store["research_claims"] = {"A": a, "B": b}

    def test_symbolic_inequality_is_not_a_contradiction(self):
        self.set_claim_pair("r", "1*r", "expression")
        self.assertEqual(paths.contradictions(), [])
        self.assertEqual(paths.possible_conflicts()[0]["claim_ids"], ["A", "B"])

    def test_equal_numeric_spellings_are_equal(self):
        self.set_claim_pair("1", "1.0", "numeric")
        self.assertEqual(paths.contradictions(), [])
        self.assertEqual(paths.possible_conflicts(), [])

    def test_unequal_exact_numbers_conflict(self):
        self.set_claim_pair("1", "2", "numeric")
        self.assertEqual(paths.contradictions()[0]["claim_ids"], ["A", "B"])
        self.assertEqual(paths.possible_conflicts(), [])

    def test_invalid_and_untyped_values_require_review(self):
        for kind, value in (("numeric", "NaN"), ("numeric", "Infinity"),
                            ("numeric", "r"), ("numeric", 1.0), ("numeric", 1),
                            ("invariance", "unknown"), ("", "1")):
            with self.subTest(kind=kind, value=value):
                self.set_claim_pair(value, "2", kind)
                self.assertEqual(paths.contradictions(), [])
                self.assertEqual(len(paths.possible_conflicts()), 1)

    def test_missing_relation_is_named(self):
        with self.assertRaisesRegex(ValueError, "no such relation missing"):
            review.view("missing")

    def test_query_does_not_accept_empty_success_or_truncation(self):
        with patch.object(db, "call", return_value=(200, {"status": "ok"})), self.assertRaises(RuntimeError):
            db.query("research_relations")
        with self.assertRaisesRegex(RuntimeError, "completeness"):
            db.query("research_relations", limit=1)

    def test_acceptance_gates_on_fresh_fixture(self):
        with redirect_stdout(io.StringIO()):
            self.assertEqual(gates.main(), 0)


if __name__ == "__main__":
    unittest.main()
