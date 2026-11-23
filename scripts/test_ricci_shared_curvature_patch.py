import unittest
from unittest.mock import patch as mock_patch
import numpy as np
from ricci_shared_curvature_patch import geometry, rank_audit, construct, diagnostics


class SharedPatchTests(unittest.TestCase):
    def test_geometry_and_boundary_of_boundary(self):
        p = geometry()
        self.assertEqual(p["cell_ids"], [0, 2, 3, 456, 458, 459])
        self.assertEqual((len(p["vertices"]), len(p["edge_ids"]), len(p["faces"])), (8, 19, 18))
        np.testing.assert_array_equal(p["D"]@p["G"], 0.)
        np.testing.assert_array_equal(p["B"]@p["D"], 0.)
        self.assertEqual(np.linalg.matrix_rank(p["G"]), 7)
        self.assertEqual(np.linalg.matrix_rank(p["D"]), 12)
        self.assertEqual(np.linalg.matrix_rank(p["B"]), 6)

    def test_geometry_only_minimal_degree(self):
        p = geometry(); audit = rank_audit(p)
        self.assertEqual([r["closed_dimensions"] for r in audit["degrees"]], [3, 11, 26])
        # This is the planted acceptance requirement, not a claimed measured rank.
        self.assertEqual(audit["chosen_degree"], 2)
        self.assertLess(audit["degrees"][1]["rank"], 12)
        self.assertEqual(audit["degrees"][2]["rank"], 12)
        self.assertLessEqual(audit["degrees"][2]["closed_space_residual"], 1e-9)

    def test_preserves_real_edges_curvature_and_all_shared_traces(self):
        p = geometry(); c = construct(p, 2, p["data"])
        report = diagnostics(p, c, p["data"])
        for name in ("edge_error", "flux_error", "closure_error", "stokes_error", "curl_error",
                     "gauge_curl_error", "constitutive_error", "derivative_error", "symmetry_error"):
            self.assertLessEqual(report[name], 1e-9, name)
        self.assertEqual(len(report["internal_faces"]), 6)
        for face in report["internal_faces"]:
            self.assertTrue(face["quadrature"]["sampled_trace_match"])
            self.assertTrue(face["quadrature"]["whitney_moments_match"])
            self.assertLessEqual(face["off_quadrature_max"], 1e-9)
        self.assertGreater(report["exterior_trace_change_max"], 1e-9)

    def test_general_inputs_and_pure_gauge(self):
        p = geometry(); rng = np.random.default_rng(17062026)
        for a in [rng.normal(size=19) for _ in range(3)]:
            c = construct(p, 2, a)
            np.testing.assert_allclose(c["edge_integrals"], a, atol=1e-9, rtol=0)
            np.testing.assert_allclose(c["fluxes"], p["D"]@a, atol=1e-9, rtol=0)
        for a in (np.zeros(19), p["G"]@rng.normal(size=8)):
            c = construct(p, 2, a)
            np.testing.assert_allclose(c["polynomial"], 0., atol=1e-9, rtol=0)
            np.testing.assert_allclose(c["edge_integrals"], a, atol=1e-9, rtol=0)


if __name__ == "__main__":
    unittest.main(verbosity=2)
