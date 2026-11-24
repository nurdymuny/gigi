"""Gate for the dome truncation diagnostic."""
import unittest
from unittest.mock import patch

import mpmath as mp

import ym_helike_dome_truncation as dm


class TruncationDiagnostic(unittest.TestCase):
    def test_reversed_orders_refuse(self):
        with self.assertRaisesRegex(ValueError, 'K'):
            dm.truncation_diagnostic(0, 1., 1.7, 2., 6, 4, dps=60)

    def test_unresolved_reference_refuses(self):
        with self.assertRaisesRegex(ArithmeticError, 'outside|exterior'):
            dm.truncation_diagnostic(0, 1., 1., 1., 0, 0, dps=60)

    def test_negative_projected_tail_refuses(self):
        original = dm.Moments.inner
        def corrupt(self, f, a, h, b):
            if f == {dm.ONE: mp.mpf(1)} and len(h) == 1 and (1, 0, 0) in h:
                return mp.mpf(1000)
            return original(self, f, a, h, b)
        with patch.object(dm.Moments, 'inner', corrupt):
            with self.assertRaisesRegex(ArithmeticError, 'negative'):
                dm.truncation_diagnostic(0, 1., 2., 2., 0, 2, dps=60)

    def test_gaussian_moment_oracle(self):
        # F_2(2q)=1+(2/3)T; under exp(-2T), E[T]=9/4, Var[T]=9/8.
        a = dm.truncation_diagnostic(0, 1., 2., 2., 0, 2, dps=60)
        self.assertAlmostEqual(a['outside_fraction_of_dome'], 2/27, places=13)
        self.assertAlmostEqual(a['total'], (11/27)**.5, places=13)
        self.assertAlmostEqual(a['outside'], 1., places=13)

    def test_equal_orders_are_exactly_zero(self):
        a = dm.truncation_diagnostic(0, 1., 2., 2., 2, 2, dps=60)
        self.assertEqual(a['total'], 0.)
        self.assertEqual(a['outside'], 0.)

    def test_provenance_is_complete(self):
        a = dm.truncation_diagnostic(0, 1., 2., 2., 0, 2, dps=60)
        for key in ('alpha', 'm', 'dps', 'reference_is_finite', 'projection_diagnostics'):
            self.assertIn(key, a)

    def test_outside_ratio_exceeds_total_ratio_and_shrinks_with_K(self):
        """The tail is high degree, so it lies mostly outside the origin basis: outside >> total."""
        a = dm.truncation_diagnostic(6, 1., 1.7, 2.0, 12, 24, dps=60)
        b = dm.truncation_diagnostic(6, 1., 1.7, 2.0, 18, 24, dps=60)
        self.assertGreater(a["outside"], 10 * a["total"])
        self.assertLess(b["outside"], a["outside"])
        self.assertTrue(0 < a["outside_fraction_of_dome"] < 1)


if __name__ == "__main__":
    unittest.main()
