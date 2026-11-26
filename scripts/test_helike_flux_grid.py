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


def rung(mq, h, c, wh=0.97, wc=0.96, z=1.0):
    """wh, wc are dressed weights; the bare weights are wh*z, wc*z (amendment 5.2)."""
    return {"Mq": mq, "E0": 1.0, "dimension": 100, "seconds": 0.1, "moments": "exact", "vacuum_weight": z,
            "helike": {"gap": h, "weight": wh * z, "dressed_weight": wh},
            "competitor": {"gap": c, "weight": wc * z, "dressed_weight": wc}}


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


class Robustness(unittest.TestCase):
    """Fixes of 26 November (disclosed in the amendment's runner note): no single point or crossing can crash the grid."""

    def test_multiple_crossings_are_unresolved_not_a_crash(self):
        r = grid.assess_case(case(S, [-0.02, 0.01, -0.01, 0.01, 0.02, 0.03, 0.04]), grid.DESIGN)
        self.assertEqual(r["status"], "UNRESOLVED")
        self.assertIn("ambiguous crossing", r["reason"])

    def test_zero_difference_at_an_endpoint_is_unresolved_not_a_crash(self):
        r = grid.assess_case(case(S, [0.0, 0.01, 0.02, 0.03, 0.04, 0.05, 0.06]), grid.DESIGN)
        self.assertEqual(r["status"], "UNRESOLVED")

    def test_receipt_digest_ignores_line_endings(self):
        with tempfile.TemporaryDirectory() as d:
            a, b = Path(d) / "a.json", Path(d) / "b.json"
            a.write_bytes(b'{"x": 1}\n'); b.write_bytes(b'{"x": 1}\r\n')
            self.assertEqual(grid.sha(a), grid.sha(b))

    def test_nonfinite_values_are_written_as_strings(self):
        self.assertEqual(grid.finite_json({"a": [float("nan"), 1.0], "b": float("inf")}), {"a": ["nan", 1.0], "b": "inf"})

    def test_an_unexpected_domain_stage_error_is_open(self):
        with patch("helike_domain_gate.domain_gate_point", side_effect=OSError("disk")):
            v, d = grid.domain_gate(1.18, 3.0, 0.05, 10)
        self.assertEqual(v, "OPEN")


    def test_cutoff_degree_cap_is_ten(self):
        import numpy as np
        import helike_cutoff_bounds as cb
        from flint import ctx
        ctx.prec = cb.PRECISION
        cb.polynomial_bounds([(10, 0, 0, 0, 0, 0)], np.array([1.0 + 0j]), 1)
        with self.assertRaises(ValueError):
            cb.polynomial_bounds([(11, 0, 0, 0, 0, 0)], np.array([1.0 + 0j]), 1)


class Dressed(unittest.TestCase):
    """Amendment 5.2: identification by w / Z."""

    def test_dressed_weight_identifies_when_the_bare_weight_is_below_threshold(self):
        p = {"s": 1.18, "domain_gate": "PASS",
             "ladder": [rung(m, 1.08, 1.082, wh=0.95, wc=0.93, z=0.88) for m in (6, 8, 10)]}
        self.assertTrue(grid.assess_point(p, grid.DESIGN)["identified"])     # bare 0.836 / 0.818, dressed 0.95 / 0.93

    def test_genuine_mixing_still_fails_after_dressing(self):
        p = {"s": 1.18, "domain_gate": "PASS",
             "ladder": [rung(m, 1.08, 1.082, wh=0.95, wc=0.85, z=0.88) for m in (6, 8, 10)]}
        self.assertFalse(grid.assess_point(p, grid.DESIGN)["identified"])

    def test_inconsistent_dressed_weight_refuses(self):
        p = {"s": 1.18, "domain_gate": "PASS", "ladder": [rung(m, 1.08, 1.082, z=0.88) for m in (6, 8, 10)]}
        p["ladder"][-1]["helike"]["dressed_weight"] = 0.99
        with self.assertRaises(ValueError):
            grid.assess_point(p, grid.DESIGN)


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


class CrossCheck(unittest.TestCase):
    """Amendment 5.3: the dressed-overlap cross-check is reported, never decided on."""

    def test_cross_check_does_not_enter_the_decision(self):
        a = grid.assess_point(point(1.18, 0.002), grid.DESIGN)
        p = point(1.18, 0.002)
        for r in p["ladder"]:
            r["cross_check"] = {"dressed_overlap": {"helike": 0.0, "competitor": 0.0}, "dressed_overlap_error": 1.0,
                                "dressed_overlap_orthonormality": 1.0}
        self.assertEqual(grid.assess_point(p, grid.DESIGN), a)

    def test_solver_output_carries_the_cross_check(self):
        import inspect
        src = inspect.getsource(grid.solve)
        self.assertIn('"cross_check"', src)
        self.assertIn("dressed_overlap", src)


class CutGate(unittest.TestCase):
    """§5.7: the domain gate is the cut's energy shift e_cut = l + 2 t r <= 3e-4; the ratio l / r is reported only."""
    H = "0" * 64

    def state(self, l, t, r_up, r_lo, tail_sq=None, nsq=(1, 1)):
        import helike_leakage_gate as lg
        from fractions import Fraction as F
        ru = None if r_up is None else lg.RigorousUpper(r_up, "test", self.H, "r_upper", "INTERVAL_ENCLOSURE")
        rl = lg.RigorousLower(r_lo, "test", self.H, "r_lower", "INTERVAL_ENCLOSURE")
        return lg.classify_state_cut(tail_sq_upper=F(t) ** 2 if tail_sq is None else tail_sq, norm_sq_lower=F(nsq[0]),
                                     norm_sq_upper=F(nsq[1]), leakage_upper=F(l), tail_upper=F(t), r_upper=ru, r_lower=rl)

    def test_arithmetic_and_boundary(self):
        import helike_leakage_gate as lg
        from fractions import Fraction as F
        self.assertEqual(lg.CUT_ENERGY_BUDGET, F(3, 10**4))
        self.assertEqual(lg.cut_energy(F(1, 10**4), F(1, 10**7), lg.RigorousUpper(F(1, 2), "t", self.H, "r", "INTERVAL_ENCLOSURE")),
                         F(1, 10**4) + F(1, 10**7))
        # exactly on the line passes; one ulp-scale step above fails
        self.assertEqual(self.state(F(3, 10**4) - F(2, 10**7), F(1, 10**7), 1, 1)["cut_gate"], "PASS")
        self.assertEqual(self.state(F(3, 10**4) - F(2, 10**7) + F(1, 10**30), F(1, 10**7), 1, 1)["cut_gate"], "FAIL")

    def test_tail_term_counts(self):
        from fractions import Fraction as F
        # leakage alone is inside the budget; 2 t r pushes it over
        s = self.state(F(2, 10**4), F(1, 10**6), 60, 1)
        self.assertEqual(s["cut_gate"], "FAIL")
        self.assertFalse(s["state_passed"])

    def test_threshold_is_a_tenth_of_the_gap_tolerance(self):
        from fractions import Fraction as F
        # e_cut = 1e-3: inside the 0.003 gap tolerance but not inside one tenth of it
        self.assertEqual(self.state(F(1, 10**3), 0, 1, 1)["cut_gate"], "FAIL")

    def test_relative_ratio_never_decides(self):
        from fractions import Fraction as F
        # §5.6's situation: l / r ~ 1.4e-3 > 1e-3, yet e_cut ~ 5.6e-6 -> PASS
        s = self.state(F(558, 10**8), F(1, 10**8), F(403, 10**5), F(403, 10**5))
        self.assertGreater(s["relative_ratio_reported"], 1e-3)
        self.assertEqual(s["cut_gate"], "PASS"); self.assertTrue(s["state_passed"])
        # the converse: a tiny ratio cannot rescue a large e_cut
        s = self.state(F(5, 10**4), 0, 10**6, 10**6)
        self.assertLess(s["relative_ratio_reported"], 1e-3)
        self.assertEqual(s["cut_gate"], "FAIL")
        self.assertNotIn("relative_gate", s)

    def test_unresolved_enclosure_is_open_never_pass(self):
        import helike_leakage_gate as lg
        from fractions import Fraction as F
        s = self.state(0, 0, None, 1)
        self.assertEqual(s["cut_gate"], "OPEN"); self.assertFalse(s["state_passed"])
        self.assertEqual(lg.overall_cut([s, s, s]), "OPEN")
        with self.assertRaises(ValueError):
            lg.RigorousUpper(1.0, "t", self.H, "r", "FLOATING")

    def test_budgets_still_gate(self):
        from fractions import Fraction as F
        self.assertFalse(self.state(0, 0, 1, 1, tail_sq=F(2, 10**12))["state_passed"])
        self.assertFalse(self.state(0, 0, 1, 1, nsq=(1, 1 + F(2, 10**10)))["state_passed"])

    def test_assess_point_converges_only_through_the_new_gate(self):
        import helike_domain_gate as dg
        good = {"state_passed": True, "cut_gate": "PASS", "relative_gate": "FAIL"}   # old rule would refuse
        rows = {n: dict(good) for n in dg.NAMES}
        with patch.object(dg, "domain_gate_point", lambda *a: {"verdict": dg.verdict(rows)}):
            v, _ = grid.domain_gate(1.15, 3.0, 0.004, 10)
        self.assertEqual(v, "PASS")
        p = {"s": 1.15, "domain_gate": v, "ladder": [rung(m, 1.08, 1.082) for m in (6, 8, 10)]}
        self.assertTrue(grid.assess_point(p, grid.DESIGN)["converged"])


class DomainStage(unittest.TestCase):
    """The domain gate's aggregation, and an end-to-end parity run at a control (Mq 6) against the archived
    Python-construction sector bounds."""

    def test_verdict_aggregation(self):
        import helike_domain_gate as dg
        ok = {"state_passed": True, "cut_gate": "PASS"}
        rows = {n: dict(ok) for n in dg.NAMES}
        self.assertEqual(dg.verdict(rows), "PASS")
        rows["ground"] = {"state_passed": False, "cut_gate": "FAIL"}      # the ground state is not gated
        self.assertEqual(dg.verdict(rows), "PASS")
        rows["competitor"] = {"state_passed": False, "cut_gate": "FAIL"}
        self.assertEqual(dg.verdict(rows), "FAIL")
        rows["competitor"] = {"state_passed": False, "cut_gate": "PASS"}   # a budget failed with the enclosure resolved
        self.assertEqual(dg.verdict(rows), "FAIL")
        rows["competitor"] = {"state_passed": False, "cut_gate": "OPEN"}
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
