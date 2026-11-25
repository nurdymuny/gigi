"""The amended flux-edge grid runner (docs/research/helike-flux-method-amendment-2026-11-25.md, section 3.5).
The solver is mocked: these tests fix the decision logic, not physics. No grid point is computed here."""
import copy
import json
import sys
import tempfile
from unittest.mock import patch
from pathlib import Path
import unittest

import run_helike_flux_background as old
import run_helike_flux_grid as grid

ROOT = Path(__file__).resolve().parents[1]


def rung(mq, h, c, wh=0.97, wc=0.96):
    return {"Mq": mq, "E0": 1.0, "dimension": 100, "seconds": 0.1, "moments": "exact",
            "helike": {"gap": h, "weight": wh}, "competitor": {"gap": c, "weight": wc}}


def point(s, diff, drift=0.0, domain="PASS", wh=0.97):
    h = 1.08
    rows = [rung(m, h + drift * (10 - m) / 4, h + diff + drift * (10 - m) / 4, wh=wh) for m in (6, 8, 10)]
    return {"s": s, "ladder": rows, "domain_gate": domain}


def case(svals, diffs, **kw):
    return {"block": "primary", "t": 3.0, "g2": 0.05, "points": [point(s, d, **kw) for s, d in zip(svals, diffs)]}


S = [1.12, 1.15, 1.165, 1.18, 1.195, 1.22, 1.24]


class Design(unittest.TestCase):
    def test_frozen_design_is_the_registration_with_the_amended_ladder(self):
        for key in ("primary", "secondary", "convergence_tol", "weight_min", "s_tolerance", "predicted_s_star"):
            self.assertEqual(grid.DESIGN[key], old.DESIGN[key], key)
        self.assertEqual(grid.DESIGN["ladder"], [6, 8, 10])
        self.assertEqual((grid.DESIGN["convergence_tol"], grid.DESIGN["weight_min"], grid.DESIGN["s_tolerance"]),
                         (0.003, 0.9, 0.015))


class Point(unittest.TestCase):
    def test_converged_identified_point(self):
        a = grid.assess_point(point(1.18, 0.002), grid.DESIGN)
        self.assertTrue(a["converged"] and a["identified"])
        self.assertAlmostEqual(a["diff_competitor_minus_helike"], 0.002)

    def test_last_step_change_above_tolerance_is_not_converged(self):
        a = grid.assess_point(point(1.18, 0.002, drift=0.0068), grid.DESIGN)   # 8 -> 10 change 0.0034
        self.assertFalse(a["converged"])

    def test_domain_gate_must_pass(self):
        for g in ("OPEN", "FAIL", None):
            self.assertFalse(grid.assess_point(point(1.18, 0.002, domain=g), grid.DESIGN)["converged"])

    def test_identification_threshold(self):
        self.assertFalse(grid.assess_point(point(1.18, 0.002, wh=0.89), grid.DESIGN)["identified"])

    def test_incomplete_or_reordered_ladder_refuses(self):
        p = point(1.18, 0.002)
        p["ladder"] = p["ladder"][:2]
        with self.assertRaises(ValueError):
            grid.assess_point(p, grid.DESIGN)
        p = point(1.18, 0.002)
        p["ladder"] = p["ladder"][::-1]
        with self.assertRaises(ValueError):
            grid.assess_point(p, grid.DESIGN)


class Case(unittest.TestCase):
    def test_supported_when_the_switch_is_within_tolerance(self):
        # crossing between 1.18 and 1.195 near 1.1809
        diffs = [-0.02, -0.012, -0.006, -0.0005, 0.007, 0.015, 0.02]
        r = grid.assess_case(case(S, diffs), grid.DESIGN)
        self.assertEqual(r["status"], "SUPPORTED")
        self.assertLessEqual(abs(r["measured_switch"] - 1.1809), 0.015)

    def test_not_supported_outside_tolerance_or_without_switch(self):
        diffs = [-0.02, -0.015, -0.012, -0.010, -0.008, -0.002, 0.004]        # crossing near 1.23
        self.assertEqual(grid.assess_case(case(S, diffs), grid.DESIGN)["status"], "NOT_SUPPORTED")
        diffs = [-0.02] * 7
        self.assertEqual(grid.assess_case(case(S, diffs), grid.DESIGN)["status"], "NOT_SUPPORTED")

    def test_unresolved_if_any_point_fails(self):
        diffs = [-0.02, -0.012, -0.006, -0.0005, 0.007, 0.015, 0.02]
        c = case(S, diffs)
        c["points"][3]["domain_gate"] = "OPEN"
        self.assertEqual(grid.assess_case(c, grid.DESIGN)["status"], "UNRESOLVED")

    def test_a_refused_point_makes_the_case_unresolved(self):
        diffs = [-0.02, -0.012, -0.006, -0.0005, 0.007, 0.015, 0.02]
        c = case(S, diffs)
        c["points"][2] = {"s": S[2], "ladder": [], "refused": "CalledProcessError", "converged": False, "identified": False}
        self.assertEqual(grid.assess_case(c, grid.DESIGN)["status"], "UNRESOLVED")

    def test_overall_rule(self):
        o = grid.overall
        self.assertEqual(o(["SUPPORTED"] * 3), "SUPPORTED")
        self.assertEqual(o(["SUPPORTED", "UNRESOLVED", "SUPPORTED"]), "UNRESOLVED")
        self.assertEqual(o(["NOT_SUPPORTED"] * 3), "NOT_SUPPORTED")
        self.assertEqual(o(["SUPPORTED", "NOT_SUPPORTED", "SUPPORTED"]), "MIXED")
        with self.assertRaises(ValueError):
            o(["SUPPORTED"] * 2)


class Release(unittest.TestCase):
    def test_domain_gate_passes_the_stage_verdict_through(self):
        import helike_domain_gate as dg
        for v in ("PASS", "FAIL", "OPEN"):
            with patch.object(dg, "domain_gate_point", lambda *a, v=v: {"verdict": v}):
                self.assertEqual(grid.domain_gate(1.15, 3.0, 0.05, 10)[0], v)
        def refuse(*a):
            raise ValueError("planted refusal")
        with patch.object(dg, "domain_gate_point", refuse):
            self.assertEqual(grid.domain_gate(1.15, 3.0, 0.05, 10)[0], "OPEN")

    def test_release_conditions_read_the_pinned_control_receipts(self):
        unmet = grid.unmet_release_conditions()
        self.assertNotIn("convergence", unmet)
        self.assertNotIn("certificate", unmet)
        self.assertNotIn("method_amendment", unmet)
        with patch.object(grid, "domain_stage_ready", lambda: False):
            self.assertIn("domain_gate_stage", grid.unmet_release_conditions())

    def test_the_grid_needs_an_explicit_flag(self):
        """Never reaches the real solver: solve and the domain gate are stubs that record whether the grid started."""
        started = []
        def fake_solve(s, t, g2, mq):
            started.append((s, t, g2, mq))
            return rung(mq, 1.08, 1.09)
        with tempfile.TemporaryDirectory() as d, \
                patch.object(grid, "unmet_release_conditions", lambda *a: []), \
                patch.object(grid, "solve", fake_solve), \
                patch.object(grid, "domain_gate", lambda *a: ("OPEN", {})), \
                patch.object(sys, "argv", ["runner", str(Path(d) / "out")]):
            code = grid.main()
        self.assertEqual(code, 2)
        self.assertEqual(started, [], "the grid started without --registered-grid")

    def test_a_changed_certificate_receipt_is_refused(self):
        pins = copy.deepcopy(grid.PINNED)
        pins["temple"] = (pins["temple"][0], "0" * 64)
        self.assertIn("certificate", grid.unmet_release_conditions(pins))


if __name__ == "__main__":
    unittest.main()


class DomainStage(unittest.TestCase):
    """The domain gate's aggregation, and an end-to-end parity run at a control (Mq 6) against the archived
    Python-construction sector bounds."""

    def test_verdict_aggregation(self):
        import helike_domain_gate as dg
        ok = {"state_passed": True, "relative_gate": "PASS"}
        rows = {n: dict(ok) for n in dg.NAMES}
        self.assertEqual(dg.verdict(rows), "PASS")
        rows["ground"] = {"state_passed": False, "relative_gate": "FAIL"}      # the ground state is not gated
        self.assertEqual(dg.verdict(rows), "PASS")
        rows["competitor"] = {"state_passed": False, "relative_gate": "FAIL"}
        self.assertEqual(dg.verdict(rows), "FAIL")
        rows["competitor"] = {"state_passed": False, "relative_gate": "OPEN"}
        self.assertEqual(dg.verdict(rows), "OPEN")

    def test_control_reproduces_the_archived_bounds(self):
        import helike_domain_gate as dg
        r = dg.domain_gate_point(1.15, 3.0, 0.004, 6)
        self.assertEqual(r["verdict"], "PASS")
        arch = json.loads((ROOT / "research/leakage-gate-20261125/inputs/cone_sector_bounds_c31aab8e.json").read_text())
        ref = {s["name"]: s for c in arch["results"]["controls"] if (c["g2"], c["Mq"]) == (0.004, 6) for s in c["states"]}
        for name in dg.NAMES:
            got, want = r["states"][name], ref[name]
            for key in ("r_lower", "r_upper", "charge_upper"):
                self.assertAlmostEqual(got[key], want[key], delta=1e-6 * abs(want[key]) + 1e-12, msg=(name, key))
            self.assertAlmostEqual(got["leakage_norm_upper"], want["leakage_norm_upper"], delta=1e-3 * want["leakage_norm_upper"])
        self.assertTrue(all(abs(abs(c) - 2) < 0.01 for c in r["compressed_charges"]))
