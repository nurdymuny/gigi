"""The amended flux-edge grid runner (docs/research/helike-flux-method-amendment-2026-11-25.md, section 3.5).
The solver is mocked: these tests fix the decision logic, not physics. No grid point is computed here."""
import copy
import json
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

    def test_overall_rule(self):
        o = grid.overall
        self.assertEqual(o(["SUPPORTED"] * 3), "SUPPORTED")
        self.assertEqual(o(["SUPPORTED", "UNRESOLVED", "SUPPORTED"]), "UNRESOLVED")
        self.assertEqual(o(["NOT_SUPPORTED"] * 3), "NOT_SUPPORTED")
        self.assertEqual(o(["SUPPORTED", "NOT_SUPPORTED", "SUPPORTED"]), "MIXED")
        with self.assertRaises(ValueError):
            o(["SUPPORTED"] * 2)


class Release(unittest.TestCase):
    def test_domain_gate_is_open_until_implemented(self):
        self.assertEqual(grid.domain_gate({"Mq": 10}), "OPEN")

    def test_release_conditions_read_the_pinned_control_receipts(self):
        unmet = grid.unmet_release_conditions()
        self.assertNotIn("convergence", unmet)
        self.assertNotIn("certificate", unmet)
        self.assertNotIn("method_amendment", unmet)
        self.assertIn("domain_gate_stage", unmet)      # the per-point domain gate is not implemented yet

    def test_a_changed_certificate_receipt_is_refused(self):
        pins = copy.deepcopy(grid.PINNED)
        pins["temple"] = (pins["temple"][0], "0" * 64)
        self.assertIn("certificate", grid.unmet_release_conditions(pins))


if __name__ == "__main__":
    unittest.main()
