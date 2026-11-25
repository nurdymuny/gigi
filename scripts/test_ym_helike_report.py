"""Report gates: evidence identity, faithful numbers, geometry and no false PASS."""

import copy
import json
from pathlib import Path
import unittest

import numpy as np
import build_ym_helike_report as report

ROOT = Path(__file__).resolve().parents[1]
RUN = ROOT / "research/helike-quantum-probe-20261123/survey-20261124T021903Z-c326a3c5"


class ReportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.receipt = json.loads((RUN / "receipt.json").read_text())
        cls.verification = json.loads((RUN / "verification.json").read_text())

    def test_plot_rows_are_the_recorded_results(self):
        payload = report.make_payload(self.receipt, self.verification)
        self.assertEqual(payload["status"], "UNRESOLVED")
        self.assertEqual(len(payload["cases"]), 15)
        strongest = payload["final"][2]
        self.assertAlmostEqual(strongest["gap"], 3.094157392269535, places=12)
        self.assertAlmostEqual(strongest["H2_weight"], 0.991539953302153, places=14)
        self.assertEqual([r["lowest_spin"] for r in payload["cases"] if r["g2"] == 2], [2, 2, 0, 2, 2])

    def test_stale_verification_and_forged_pass_are_refused(self):
        bad = dict(self.verification, run_id="old")
        with self.assertRaisesRegex(ValueError, "verification"):
            report.make_payload(self.receipt, bad)
        bad = dict(self.receipt, status="PASS")
        with self.assertRaisesRegex(ValueError, "UNRESOLVED"):
            report.make_payload(bad, self.verification)

    def test_altered_gap_and_nan_weight_are_refused(self):
        bad = copy.deepcopy(self.receipt)
        bad["couplings"][0]["cases"][-1]["absolute_gap"] += 1
        with self.assertRaisesRegex(ValueError, "gap"):
            report.make_payload(bad, self.verification)
        bad = copy.deepcopy(self.receipt)
        bad["couplings"][0]["cases"][-1]["H2"]["lowest_weight"] = float("nan")
        with self.assertRaisesRegex(ValueError, "finite"):
            report.make_payload(bad, self.verification)

    def test_missing_case_and_changed_level_weight_are_refused(self):
        bad = copy.deepcopy(self.receipt)
        bad["couplings"][0]["cases"].pop(0)
        with self.assertRaisesRegex(ValueError, "cutoff"):
            report.make_payload(bad, self.verification)
        bad = copy.deepcopy(self.receipt)
        bad["couplings"][0]["cases"][-1]["H2"]["levels"][1]["weight"] = .1
        with self.assertRaisesRegex(ValueError, "weight"):
            report.make_payload(bad, self.verification)

    def test_gate_ratios_use_registered_thresholds(self):
        row = report.make_payload(self.receipt, self.verification)["final"][0]
        self.assertAlmostEqual(row["residual_ratio"], .8994883348247144/.022898146240874235)
        self.assertAlmostEqual(row["H0_tail_ratio"], .005604987952301565/.001)
        self.assertGreater(row["residual_ratio"], 1)

    def test_geometry_is_the_declared_tensor_not_artwork(self):
        bc, bs = report.quadratures()
        self.assertAlmostEqual(np.trace(bc), 0)
        self.assertAlmostEqual(np.trace(bs), 0)
        self.assertAlmostEqual(np.sum(bc*bc), 1)
        self.assertAlmostEqual(np.sum(bs*bs), 1)
        self.assertAlmostEqual(np.sum(bc*bs), 0)
        e1 = np.array([0., 1., 0.])
        diagonal = np.array([0., 1., 1.])/np.sqrt(2)
        self.assertAlmostEqual(e1@bc@e1, 1/np.sqrt(2))
        self.assertAlmostEqual(e1@bs@e1, 0)
        self.assertAlmostEqual(diagonal@bs@diagonal, 1/np.sqrt(2))


class SectorReportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.receipt = json.loads((report.SECTOR_RUN / "receipt.json").read_text())
        cls.verification = json.loads((report.SECTOR_RUN / "verification.json").read_text())

    def test_sector_results_are_measured_not_promoted(self):
        p = report.make_sector_payload(self.receipt, self.verification)
        self.assertEqual(len(p["cases"]), 16)
        self.assertEqual(p["first_joint_pass"], {1.0: 20, 2.0: 20})
        self.assertEqual(p["first_strict_pass"], {1.0: 22, 2.0: 22})
        self.assertEqual(p["scientific_status"], "UNRESOLVED")
        for row, case in zip(p["cases"], self.receipt["cases"]):
            self.assertEqual(row["scalar_residual"], case["measurements"]["scalar_sector"]["residuals"][1])
        self.assertAlmostEqual(p["final"][1]["tensor_residual"], .004551948676215114, places=15)

    def test_rejects_missing_case_and_forged_completion(self):
        bad = copy.deepcopy(self.receipt)
        bad["cases"].pop()
        with self.assertRaises(ValueError):
            report.make_sector_payload(bad, self.verification)
        with self.assertRaises(ValueError):
            report.make_sector_payload(dict(self.receipt, status="RUNNING"), self.verification)

    def test_rejects_loose_allowance_and_forged_residual_pass(self):
        for key, value in (("allowance", 1.0), ("residual_gate", dict(vacuum=True, scalar=True, tensor=True))):
            bad = copy.deepcopy(self.receipt)
            bad["cases"][0]["measurements"][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                report.make_sector_payload(bad, self.verification)

    def test_rejects_nonfinite_residual_and_changed_gap(self):
        bad = copy.deepcopy(self.receipt)
        bad["cases"][0]["measurements"]["scalar_sector"]["residuals"][1] = float("nan")
        with self.assertRaises(ValueError):
            report.make_sector_payload(bad, self.verification)
        bad = copy.deepcopy(self.receipt)
        bad["cases"][0]["measurements"]["gap_scalar"] += 1
        with self.assertRaises(ValueError):
            report.make_sector_payload(bad, self.verification)

    def test_rejects_false_certification_and_failed_precision(self):
        with self.assertRaises(ValueError):
            report.make_sector_payload(self.receipt, dict(self.verification, certified=True))
        bad = copy.deepcopy(self.receipt)
        bad["cases"][-1]["precision"]["2"]["passed"] = False
        with self.assertRaises(ValueError):
            report.make_sector_payload(bad, self.verification)

    def test_dimension_claims_preserve_sector_scope(self):
        p = report.make_sector_payload(self.receipt, self.verification)
        self.assertEqual(len(p["final"]), 2)
        for row in p["final"]:
            self.assertEqual((row["scalar_dimension"], row["tensor_multiplets"], row["all_spin_dimension"]),
                             (169, 244, 26572))
        self.assertEqual(p["scope"], "J0_J2_ONLY")

    def test_full_residual_identity_includes_both_orthogonal_parts(self):
        h = np.array([[1., .3, .7], [.3, 2., .2], [.7, .2, 5.]])
        u = np.eye(3)[:, :2]
        ht = u.T @ h @ u
        exterior = (h @ u).T @ (h @ u) - ht @ ht
        v = np.array([.6, .8]); energy = 1.9
        direct = np.linalg.norm(h @ u @ v - energy * u @ v)**2
        projected = np.linalg.norm(ht @ v - energy * v)**2
        escaped = v @ exterior @ v
        self.assertAlmostEqual(direct, projected + escaped, places=14)
        self.assertGreater(projected, .01)
        self.assertGreater(escaped, .01)

    def test_manuscript_cometric_is_the_cartesian_pullback(self):
        import sympy as sp
        coords = sp.symbols("x0:9")
        x = sp.Matrix(3, 3, coords)
        g = x * x.T
        t, p, d = sp.trace(g), sp.trace(g*g), x.det()
        grad = sp.Matrix([t, p, d]).jacobian(coords)
        expected = sp.Matrix([[4*t, 8*p, 6*d],
            [8*p, -8*t**3+24*t*p+48*d**2, 4*t*d],
            [6*d, 4*t*d, (t**2-p)/2]])
        self.assertEqual((grad*grad.T-expected).applyfunc(sp.expand), sp.zeros(3))
        self.assertEqual([sp.expand(sum(sp.diff(f, z, 2) for z in coords)) for f in (t,p,d)],
                         [18, 28*t, 0])
        a1, a2 = g-t*sp.eye(3)/3, g*g-p*sp.eye(3)/3
        self.assertEqual(sp.expand(sp.trace(a1*a2)-(-t**3/2+7*t*p/6+3*d**2)), 0)

    def test_manuscript_superpotential_and_path_are_exact(self):
        import sympy as sp
        coords = sp.symbols("q0:9")
        q = sp.Matrix(3, 3, coords)
        g, a = sp.symbols("g a", positive=True)
        t, p, d = sp.trace(q*q.T), sp.trace((q*q.T)**2), q.det()
        w = t/2-g*d
        grad = sp.Matrix([sp.diff(w,z) for z in coords])
        potential = t/2-3*g*d+g*g*(t*t-p)/4
        self.assertEqual(sp.expand((grad.T*grad)[0]/2-potential), 0)
        self.assertEqual(sp.expand(sum(sp.diff(w,z,2) for z in coords)), 9)
        path = a*sp.eye(3)/g
        self.assertEqual((path-g*path.cofactor_matrix()-a*(1-a)*sp.eye(3)/g).applyfunc(sp.simplify), sp.zeros(3))
        action = sp.integrate(3*a*(1-a)/g**2, (a,0,1))
        self.assertEqual(action, 1/(2*g**2))


if __name__ == "__main__":
    unittest.main(verbosity=2)


class FluxReportTests(unittest.TestCase):
    """Pages 11-14: the four flux receipts are read faithfully and no promoted or reversed claim is accepted."""

    @classmethod
    def setUpClass(cls):
        cls.data = {k: json.loads((ROOT / path).read_text()) for k, (path, _) in report.FLUX_RECEIPTS.items()}

    def payload(self, **override):
        d = copy.deepcopy(self.data)
        d.update(override)
        return report.make_flux_payload(d["convergence"], d["sectors"], d["leakage"], d["temple"])

    def test_flux_rows_are_the_recorded_results(self):
        p = self.payload()
        self.assertEqual(p["ordered"], 3)
        self.assertEqual([r["verdict"] for r in p["controls"]], ["UNRESOLVED", "ORDERED", "ORDERED", "ORDERED"])
        self.assertAlmostEqual(p["controls"][1]["margin"], 0.007179373217930162, places=15)
        self.assertLessEqual(max(s["leakage_over_r"] for r in p["controls"] for s in r["states"]), 1e-3)

    def test_rejects_reversed_or_promoted_claims(self):
        t = copy.deepcopy(self.data["temple"])
        t["results"]["controls"][1]["decision"]["verdict"] = "COMPETITOR_BELOW"
        with self.assertRaisesRegex(ValueError, "never reported"):
            self.payload(temple=t)
        t = copy.deepcopy(self.data["temple"])
        t["results"]["controls"][0]["decision"]["verdict"] = "ORDERED"
        with self.assertRaisesRegex(ValueError, "positive margin"):
            self.payload(temple=t)

    def test_rejects_released_grid_and_missing_hypothesis(self):
        leak = dict(self.data["leakage"], full_grid_released=True)
        with self.assertRaisesRegex(ValueError, "grid held"):
            self.payload(leakage=leak)
        t = copy.deepcopy(self.data["temple"])
        t["params"]["assume"] = None
        with self.assertRaisesRegex(ValueError, "separation hypothesis"):
            self.payload(temple=t)
