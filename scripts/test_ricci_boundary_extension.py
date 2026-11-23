import unittest
import numpy as np
from scipy.linalg import null_space
import ricci_boundary_extension as subject
import ricci_shared_curvature_patch as original
from ricci_shared_curvature_math import curl, divergence


class LiftTests(unittest.TestCase):
    def test_exact_weighted_minimum_and_preserved_flux(self):
        m = np.diag([2., 3., 5.]); q = np.array([[1., 0., 0.]])
        t = np.array([[0., 1., 1.]])
        fit = subject.exact_lift(m, q, t, np.array([2., 0., 0.]), np.array([1.]))
        self.assertEqual(fit["status"], "ACCEPTED")
        np.testing.assert_allclose(fit["coefficients"], [2., .625, .375], atol=1e-12, rtol=0)
        self.assertAlmostEqual(fit["correction_energy"], .9375)

    def test_incompatible_targets_refuse_no_coefficients(self):
        fit = subject.exact_lift(np.eye(3), np.array([[1., 0., 0.]]),
            np.array([[0., 1., 0.], [0., 1., 0.]]), np.zeros(3), np.array([0., 1.]))
        self.assertEqual(fit["status"], "REFUSED_INCOMPATIBLE_BOUNDARY")
        self.assertNotIn("coefficients", fit)
        self.assertAlmostEqual(fit["residual_norm"], np.sqrt(.5))
        self.assertLess(fit["witness_orthogonality_error"], 1e-12)
        self.assertLess(fit["witness_identity_error"], 1e-12)

    def test_conflicting_flux_and_trace_refuse(self):
        fit = subject.exact_lift(np.eye(2), np.array([[1., 0.]]),
            np.array([[1., 0.]]), np.array([2., 0.]), np.array([3.]))
        self.assertEqual(fit["status"], "REFUSED_INCOMPATIBLE_BOUNDARY")

    def test_zero_correction(self):
        fit = subject.exact_lift(np.eye(3), np.array([[1., 0., 0.]]), np.eye(3),
            np.array([2., 3., 4.]), np.array([2., 3., 4.]))
        self.assertEqual(fit["status"], "ACCEPTED")
        np.testing.assert_allclose(fit["delta"], 0., atol=1e-14, rtol=0)

    def test_nonfinite_refused(self):
        with self.assertRaisesRegex(ValueError, "finite"):
            subject.exact_lift(np.eye(2), np.ones((1, 2)), np.eye(2), np.zeros(2), [np.nan, 0.])

    def test_oriented_physical_component(self):
        h = np.array([[6., 8.]]); g = np.array([np.diag([4., 9.])])
        np.testing.assert_allclose(subject.unit_component(h, np.array([1., 0.]), g), [3.])
        np.testing.assert_allclose(subject.unit_component(h, np.array([-1., 0.]), g), [-3.])

    def test_complete_linear_derivative(self):
        m = np.diag([2., 3., 5.]); q = np.array([[1., 0., 0.]])
        t = np.array([[0., 1., 1.]])
        base = np.array([.4, -.3, .2]); target = np.array([.9])
        fit = subject.exact_lift(m, q, t, base, target)
        analytic = np.r_[fit["base_map"].T@m@fit["coefficients"], fit["target_map"].T@m@fit["coefficients"]]
        numerical = []
        for i in range(4):
            energies = []
            for sign in (1, -1):
                a = np.r_[base, target].copy(); a[i] += sign*1e-5
                c = subject.exact_lift(m, q, t, a[:3], a[3:])["coefficients"]
                energies.append(.5*c@m@c)
            numerical.append((energies[0]-energies[1])/2e-5)
        np.testing.assert_allclose(analytic, numerical, atol=1e-9, rtol=0)


class FixedPatchControls(unittest.TestCase):
    def test_planted_compatible_extension(self):
        p = original.geometry(); base = original.construct(p, 2, p["data"])
        uv, _ = original.quadrature(6, 2)
        system = subject.boundary_system(p, base["basis"], uv)
        z = null_space(base["flux_map"], rcond=1e-12)
        self.assertEqual(z.shape[1], 14)
        planted = base["coefficients"] + z@np.linspace(-.002, .003, 14)
        fit = subject.exact_lift(base["mass"], base["flux_map"], system["trace"],
                                 base["coefficients"], system["trace"]@planted)
        self.assertEqual(fit["status"], "ACCEPTED")
        np.testing.assert_allclose(fit["coefficients"], planted, atol=1e-9, rtol=0)
        changed = subject.finalize(p, base, fit)
        np.testing.assert_allclose(changed["edge_integrals"], p["data"], atol=1e-9, rtol=0)
        np.testing.assert_allclose(changed["fluxes"], p["D"]@p["data"], atol=1e-9, rtol=0)
        np.testing.assert_allclose(divergence(2)@changed["polynomial"], 0., atol=1e-9, rtol=0)
        np.testing.assert_allclose(curl(changed["potential"], 3), changed["polynomial"], atol=1e-9, rtol=0)
        faces, _, _ = original.face_diagnostics(p, changed)
        self.assertTrue(all(f["quadrature"]["sampled_trace_match"] for f in faces))
        self.assertTrue(all(f["quadrature"]["whitney_moments_match"] for f in faces))
        off = np.array([[.2,.2],[.6,.2],[.2,.6],[.1,.35],[.45,.45],[.7,.1],[1/3,1/3]])
        check = subject.boundary_system(p, base["basis"], off)
        np.testing.assert_allclose(check["trace"]@(fit["coefficients"]-planted), 0., atol=1e-9, rtol=0)

    def test_common_field_edges_are_compatible(self):
        p = original.geometry(); base = original.construct(p, 2, p["data"])
        report = subject.edge_audit(p, base["polynomial"])
        self.assertEqual(len(report["edges"]), 18)
        self.assertLessEqual(report["max_gap"], 1e-9)
        self.assertLessEqual(report["reversal_error"], 1e-9)


if __name__ == "__main__":
    unittest.main(verbosity=2)
