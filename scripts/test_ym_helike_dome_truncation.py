"""Gate for the dome truncation diagnostic."""
import unittest

import ym_helike_dome_truncation as dm


class TruncationDiagnostic(unittest.TestCase):
    def test_outside_ratio_exceeds_total_ratio_and_shrinks_with_K(self):
        """The tail is high degree, so it lies mostly outside the origin basis: outside >> total."""
        a = dm.truncation_diagnostic(6, 1., 1.7, 2.0, 12, 24, dps=60)
        b = dm.truncation_diagnostic(6, 1., 1.7, 2.0, 18, 24, dps=60)
        self.assertGreater(a["outside"], 10 * a["total"])
        self.assertLess(b["outside"], a["outside"])
        self.assertTrue(0 < a["outside_fraction_of_dome"] < 1)


if __name__ == "__main__":
    unittest.main()
