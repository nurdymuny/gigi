import unittest
import numpy as np
from ricci_local_push_math import contributions, check_closure


class PushAccountingTests(unittest.TestCase):
    def setUp(self):
        self.ids = np.array([[0, 1], [1, 2]])
        self.signs = np.array([[1., -1.], [1., 1.]])
        self.forces = np.array([[2., 3.], [5., 7.]])
        self.adjoints = np.array([[1., 2.], [3., 4.], [5., 6.]])
        self.expected = np.array([43., 54.])

    def test_signed_parts_match_independent_global_assembly(self):
        parts = contributions(self.ids, self.signs, self.forces, self.adjoints)
        np.testing.assert_allclose(parts, [[-7., -8.], [50., 62.]], atol=0, rtol=0)
        check_closure(parts, self.expected)

    def test_missing_cell_is_rejected(self):
        parts = contributions(self.ids, self.signs, self.forces, self.adjoints)
        with self.assertRaisesRegex(ValueError, "closure"):
            check_closure(parts[:-1], self.expected)

    def test_assembly_orientation_removal_is_rejected(self):
        broken = contributions(self.ids, np.ones_like(self.signs), self.forces, self.adjoints)
        with self.assertRaisesRegex(ValueError, "closure"):
            check_closure(broken, self.expected)

    def test_nonfinite_evidence_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "finite"):
            check_closure(np.array([[np.nan, 0.]]), np.zeros(2))


if __name__ == "__main__":
    unittest.main(verbosity=2)
