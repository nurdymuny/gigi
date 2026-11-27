import copy
import struct
import unittest

import run_helike_flux_support as probe


def bits(xs):
    return [struct.unpack('<Q', struct.pack('<d', x))[0] for x in xs]


class IndependentStatistics(unittest.TestCase):
    def test_statistics_recover_signed_binary64_coefficients(self):
        basis = [[0]*6, [1,0,0,0,0,0], [0,0,2,0,0,0]]
        v = probe.decode(bits([0.0, -0.6, 0.8]))
        r = probe.measure(basis, v*v, [0,1,2,3,4,5])
        self.assertAlmostEqual(r['mass'], 1)
        self.assertAlmostEqual(r['shells'][2], .64)
        self.assertAlmostEqual(r['mean_group_occupation'][1], 1.28)
        self.assertAlmostEqual(sum(r['occupation_fraction']), 1)

    def test_zero_is_absent_not_a_normalized_zero_distribution(self):
        r = probe.measure([[0]*6], [0.0], [0,1,2,3,4,5])
        self.assertIsNone(r['occupation_fraction'])
        self.assertIsNone(r['mean_group_occupation'])

    def test_a_corrupt_native_summary_refuses(self):
        r = probe.measure([[0]*6, [1,0,0,0,0,0]], [.36,.64], [0,1,2,3,4,5])
        probe.agree(r, r)
        bad = copy.deepcopy(r)
        bad['shells'][1] = .6
        with self.assertRaises(ValueError):
            probe.agree(r, bad)

    def test_bad_dimensions_nonfinite_weights_and_roles_refuse(self):
        for weights, roles in [([-1], list(range(6))), ([float('nan')], list(range(6))),
                                ([], list(range(6))), ([1], [0]*6)]:
            with self.assertRaises(ValueError):
                probe.measure([[0]*6], weights, roles)


if __name__ == '__main__':
    unittest.main()
