import unittest
import numpy as np
from ricci_push_interfaces import quadrature
from ricci_shared_face_math import pullback, trace_report


class SharedFaceTests(unittest.TestCase):
    def setUp(self):
        self.xy, self.w = quadrature(6, 2)
        u, v = self.xy.T
        self.basis = np.stack([np.column_stack([1-v, u]), np.column_stack([v, 1-u]),
                               np.column_stack([-v, u])], axis=1)
        self.g = np.broadcast_to(np.eye(2), (len(u), 2, 2)).copy()
        self.zero = np.zeros_like(self.xy)

    def report(self, left, right=None, **kwargs):
        return trace_report(left, self.zero if right is None else right,
            kwargs.get("metric", self.g), kwargs.get("weights", self.w),
            kwargs.get("basis", self.basis), kwargs.get("orientation", 1))

    def test_identical_nonzero_curved_traces_match(self):
        h = self.xy**2 + .7
        result = self.report(h, h)
        self.assertTrue(result["sampled_trace_match"])
        self.assertTrue(result["whitney_moments_match"])

    def test_normal_difference_is_not_tangential_mismatch(self):
        jacobian = np.array([[1., 0.], [0., 1.], [0., 0.]])
        h1 = np.tile([1., 2., 0.], (len(self.w), 1))
        h2 = np.tile([1., 2., 99.], (len(self.w), 1))
        result = self.report(pullback(h1, jacobian), pullback(h2, jacobian))
        self.assertTrue(result["sampled_trace_match"])

    def test_all_three_whitney_moments_match_analytic_integrals(self):
        h = np.tile([1., 0.], (len(self.w), 1))
        result = self.report(h)
        np.testing.assert_allclose(result["moments"], [1/6, 1/3, 1/6], atol=1e-14, rtol=0)
        self.assertFalse(result["whitney_moments_match"])
        self.assertFalse(result["sampled_trace_match"])

    def test_equal_magnitudes_opposite_signs_fail(self):
        h = np.tile([1., 0.], (len(self.w), 1))
        result = self.report(h, -h)
        self.assertFalse(result["sampled_trace_match"])
        self.assertAlmostEqual(result["physical_jump_max"], 2.)

    def test_weak_only_match_is_not_promoted(self):
        u = self.xy[:, 0]
        h = np.column_stack([u*u-.8*u+.1, np.zeros_like(u)])
        result = self.report(h)
        self.assertTrue(result["whitney_moments_match"])
        self.assertFalse(result["sampled_trace_match"])
        self.assertGreater(result["physical_jump_max"], .1)

    def test_metric_norm_not_coordinate_norm(self):
        h = np.tile([2., 3.], (len(self.w), 1))
        g = np.broadcast_to(np.diag([4., 9.]), self.g.shape).copy()
        result = self.report(h, metric=g)
        self.assertAlmostEqual(result["physical_jump_max"], np.sqrt(2.))
        self.assertAlmostEqual(result["physical_jump_l2"], np.sqrt(6.))

    def test_reparametrization_preserves_moments_and_physical_norm(self):
        h = self.xy+.1
        original = self.report(h)
        q = np.array([[2., .3], [.1, 1.1]])
        transformed = self.report(h@q, metric=np.einsum("ia,qij,jb->qab", q, self.g, q),
            weights=self.w/abs(np.linalg.det(q)), basis=self.basis@q)
        np.testing.assert_allclose(original["moments"], transformed["moments"], atol=1e-14, rtol=0)
        self.assertAlmostEqual(original["physical_jump_max"], transformed["physical_jump_max"])
        self.assertAlmostEqual(original["physical_jump_l2"], transformed["physical_jump_l2"])

    def test_orientation_reversing_chart_preserves_signed_moments(self):
        h = np.tile([1., 0.], (len(self.w), 1))
        q = np.array([[0., 1.], [1., 0.]])
        original = self.report(h)
        transformed = self.report(h@q, basis=self.basis@q, orientation=-1)
        np.testing.assert_allclose(original["moments"], transformed["moments"], atol=1e-14, rtol=0)

    def test_nonfinite_trace_is_refused(self):
        h = self.zero.copy(); h[0, 0] = np.nan
        with self.assertRaisesRegex(ValueError, "finite"):
            self.report(h)

    def test_indefinite_metric_is_refused(self):
        g = self.g.copy(); g[:, 0, 0] = -1
        with self.assertRaisesRegex(ValueError, "positive"):
            self.report(self.zero, metric=g)

    def test_invalid_orientation_is_refused(self):
        with self.assertRaisesRegex(ValueError, "orientation"):
            self.report(self.zero, orientation=0)


if __name__ == "__main__":
    unittest.main(verbosity=2)
