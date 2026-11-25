"""Relative cutoff-leakage gate (docs/research/helike-leakage-gate-amendment-2026-11-24.md), section 5 contract."""
from fractions import Fraction
import json
from pathlib import Path
import unittest

import helike_leakage_gate as lg
import run_helike_leakage_gate as run

ROOT = Path(__file__).resolve().parents[1]
HISTORICAL = ROOT / 'research/cutoff-tail-bounds-20261124/controls-c31aab8e/receipt.json'


def rigorous(x):
    return lg.RigorousLower(x, source='fixture', sha256='0' * 64, field='r_lower', status='INTERVAL_ENCLOSURE')


class Gate(unittest.TestCase):
    def test_kappa_and_boundary(self):
        self.assertEqual(lg.KAPPA, Fraction(1, 1000))
        r = 0.01
        self.assertEqual(lg.relative_gate(Fraction(1, 2) * lg.KAPPA * Fraction(r), rigorous(r)), 'PASS')
        self.assertEqual(lg.relative_gate(2 * lg.KAPPA * Fraction(r), rigorous(r)), 'FAIL')
        self.assertEqual(lg.relative_gate(lg.KAPPA * Fraction(r), rigorous(r)), 'PASS')   # inclusive bound

    def test_open_without_a_rigorous_lower_bound(self):
        self.assertEqual(lg.relative_gate(1e-12, None), 'OPEN')
        self.assertEqual(lg.relative_gate(1e-12, 0.01), 'OPEN')            # a float is not an enclosure
        self.assertEqual(lg.relative_gate(1e-12, rigorous(0.0)), 'OPEN')   # undefined ratio
        with self.assertRaises(ValueError):
            lg.RigorousLower(0.01, source='x', sha256='0' * 64, field='r_lower', status='COMPUTED')

    def test_absolute_diagnostic_is_reported_not_gating(self):
        s = lg.classify_state(tail_sq_upper=Fraction(0), norm_sq_lower=Fraction(1), norm_sq_upper=Fraction(1),
                              leakage_upper=Fraction(2.7e-7), r_lower=rigorous(0.0088))
        self.assertFalse(s['absolute_1e-10_diagnostic'])
        self.assertEqual(s['relative_gate'], 'PASS')
        self.assertTrue(s['state_passed'])
        self.assertIn('diagnostic', s['absolute_1e-10_diagnostic_label'])

    def test_tail_and_normalisation_budgets_still_gate(self):
        base = dict(norm_sq_lower=Fraction(1), norm_sq_upper=Fraction(1), leakage_upper=Fraction(0), r_lower=rigorous(0.01))
        self.assertFalse(lg.classify_state(tail_sq_upper=Fraction(2, 10**12), **base)['state_passed'])
        wide = dict(base, norm_sq_upper=Fraction(1) + Fraction(2, 10**10))
        self.assertFalse(lg.classify_state(tail_sq_upper=Fraction(0), **wide)['state_passed'])

    def test_residual_bound_and_temple_enlargement(self):
        r, l, q = Fraction(1, 100), Fraction(1, 1000), Fraction(1, 2)
        self.assertEqual(lg.residual_bound(r, l, q), (r + l) / q)
        self.assertEqual(lg.temple_enlargement(r, l, q), (2 * r * l + l * l) / (q * q))
        with self.assertRaises(ValueError):
            lg.residual_bound(r, l, Fraction(0))

    def test_overall_requires_every_state(self):
        good = dict(state_passed=True, relative_gate='PASS')
        self.assertEqual(lg.overall([good] * 16, expected=16), 'PASS')
        self.assertEqual(lg.overall([good] * 15 + [dict(state_passed=False, relative_gate='FAIL')], expected=16), 'FAIL')
        self.assertEqual(lg.overall([good] * 15 + [dict(state_passed=False, relative_gate='OPEN')], expected=16), 'OPEN')
        with self.assertRaises(ValueError):
            lg.overall([good] * 15, expected=16)

    def test_dyadic_decoding_is_exact(self):
        self.assertEqual(lg.dyadic({'mantissa': '39', 'exponent': -2}), Fraction(39, 4))
        self.assertEqual(lg.dyadic({'mantissa': '3', 'exponent': 2}), Fraction(12))


class Runner(unittest.TestCase):
    def test_r_lower_is_the_lower_endpoint(self):
        state = {'r_lower': 0.009, 'r_upper': 0.011}
        self.assertEqual(run.r_lower_value(state), 0.009)

    def test_historical_receipt_is_not_relabelled(self):
        rec = json.loads(HISTORICAL.read_text())
        self.assertIs(run.historical_gate(rec), False)
        self.assertEqual(rec['cutoff_tail_gate_passed'], False)

    def test_pairing_refuses_mismatched_leakage(self):
        arb = {'leakage_norm': {'upper': {'mantissa': '1', 'exponent': -30}}}
        rust = {'leakage_norm_upper': 2.0 ** -40}
        with self.assertRaises(ValueError):
            run.check_pairing(arb, rust)
        ok = {'leakage_norm_upper': 2.0 ** -30}
        run.check_pairing(arb, ok)


if __name__ == '__main__':
    unittest.main()
