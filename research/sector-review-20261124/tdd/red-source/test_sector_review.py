"""Adversarial review gates. No high-cutoff survey is started by these tests."""
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import mpmath as mp
import numpy as np

import ym_helike_j0 as j
import run_ym_helike_sectors as r


class ResidualTests(unittest.TestCase):
    def test_projected_eigensolver_error_is_part_of_full_residual(self):
        with mp.workdps(60):
            S=mp.eye(2); H=mp.diag([5,7]); K=H*H
            angle=.01
            V=np.array([[np.cos(angle),-np.sin(angle)],[np.sin(angle),np.cos(angle)]])
            with patch.object(np.linalg,'eigh',return_value=(np.array([5.,7.]),V)):
                got=j.finish(S,H,K,60)
            np.testing.assert_allclose(got['residuals'],[2*np.sin(angle)]*2,rtol=1e-12)

    def test_negative_exterior_cannot_turn_into_zero(self):
        with mp.workdps(60), self.assertRaisesRegex(ArithmeticError,'negative|exterior'):
            j.finish(mp.eye(2),mp.diag([5,7]),mp.diag([24,49]),60)

    def test_small_residual_is_not_lost_in_float_cancellation(self):
        with mp.workdps(60):
            H=mp.matrix([[5]]); K=mp.matrix([[25+mp.mpf('1e-36')]])
            got=j.finish(mp.eye(1),H,K,60)
            self.assertAlmostEqual(got['residuals'][0]/1e-18,1.,places=10)

    def test_bad_physical_inputs_refused(self):
        for g,a in ((-1,1),(float('nan'),1),(1,0),(1,float('inf'))):
            with self.subTest(g=g,a=a):
                try:
                    j.solve(4,g,a)
                except Exception as exc:
                    self.assertIsInstance(exc,ValueError)
                else:
                    self.fail('invalid physical inputs accepted')


class RunnerTests(unittest.TestCase):
    def test_finite_json_refuses_nan(self):
        with tempfile.TemporaryDirectory() as tmp, self.assertRaises(ValueError):
            r.write(Path(tmp)/'receipt.json',{'value':float('nan')})

    def test_common_allowance_catches_false_channel_pass(self):
        s={'dimension':6,'gram_digits_lost':0.,'min_exterior_eig':0.,
           'energies':[5.,7.,9.,10.,11.,12.],'residuals':[.005,.019,.1,.1,.1,.1]}
        t=dict(s,energies=[6.,8.,10.,11.,12.,13.],residuals=[.005,.1,.1,.1,.1,.1])
        row=r.make_row(10,s,t)
        self.assertEqual(row['allowance'],.01)
        self.assertFalse(row['residual_gate']['scalar'])
        self.assertFalse(row['global_gap_verified'])
        self.assertEqual(row['scientific_status'],'UNRESOLVED')

    def test_missing_precision_values_refused(self):
        with self.assertRaisesRegex(ValueError,'six|length'):
            r.precision_check({'energies':[1.], 'residuals':[.1]}, {'energies':[], 'residuals':[]},100)

    def test_cutoffs_and_controls_are_frozen(self):
        self.assertTrue(r.validate_request(['1:1.7','2:2.3'],[10,12,14],60,100))
        for cases,cutoffs in ((['1.60:1.7'],[10,12,14]),(['1:2','2:2.3'],[10,12,14]),
                              (['1:1.7','2:2.3'],[14,10]),(['1:1.7','2:2.3'],[10,10])):
            with self.subTest(cases=cases,cutoffs=cutoffs),self.assertRaises(ValueError):
                r.validate_request(cases,cutoffs,60,100)

    def test_existing_evidence_cannot_be_overwritten(self):
        with tempfile.TemporaryDirectory() as tmp, self.assertRaises(FileExistsError):
            r.initialize(Path(tmp),['1:1.7','2:2.3'],[10,12,14],60,100)

    def test_failed_receipt_replaces_old_pass(self):
        with tempfile.TemporaryDirectory() as tmp:
            p=Path(tmp)
            (p/'receipt.json').write_text(json.dumps({'status':'FAILED'}))
            (p/'verification.json').write_text(json.dumps({'status':'PASS'}))
            with self.assertRaises(ValueError):
                r.verify_run(p)
            self.assertEqual(json.loads((p/'verification.json').read_text())['status'],'FAIL')


if __name__=='__main__':
    unittest.main(verbosity=2)
