"""Small planted controls and refusal gates; no new physical couplings."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import mpmath as mp
import ym_helike_dome_truncation as dm
import run_ym_helike_dome_local_review as run


class AuditTests(unittest.TestCase):
    def small(self):
        return dm.truncation_sweep(0,1.,2.,2.,[0,2],4,dps=60)

    def test_overlap_evidence_replays(self):
        b=self.small()
        self.assertEqual(len(dm.replay_batch(b)),2)

    def test_changed_ratio_rejected(self):
        b=self.small(); b['rows'][0]['outside']*=2
        with self.assertRaises(ValueError): dm.replay_batch(b)

    def test_missing_row_rejected(self):
        b=self.small(); b['rows'].pop()
        with self.assertRaises(ValueError): dm.replay_batch(b)

    def test_changed_cancellation_disclosure_rejected(self):
        b=self.small()
        b['rows'][0]['projection_diagnostics']['reference']['raw']='12345'
        with self.assertRaises(ValueError):dm.replay_batch(b)

    def test_tiny_negative_is_disclosed_and_zero_not_absolute(self):
        with mp.workdps(60):
            v,d=dm.checked_remainder(mp.mpf(1),1+mp.mpf('1e-45'),60,'test')
            self.assertEqual(v,0)
            self.assertTrue(d['roundoff_clipped'])
            self.assertLess(mp.mpf(d['raw']),0)

    def test_invalid_parameters_refuse(self):
        for k,v in (('g2',0),('alpha',float('nan')),('beta',-1),('N',True),('dps',10),('m',(-1,0,0))):
            kw=dict(N=0,g2=1.,alpha=2.,beta=2.,K=0,K_ref=2,dps=60)
            kw[k]=v
            with self.subTest(k=k),self.assertRaises(ValueError): dm.truncation_diagnostic(**kw)

    def test_job_scope_is_frozen(self):
        self.assertEqual(len(run.jobs()),8)
        self.assertEqual({j['g2'] for j in run.jobs()},{1.,2.})
        self.assertEqual(sum(j['dps']==120 for j in run.jobs()),2)
        self.assertEqual(sum(j['K_ref']==54 for j in run.jobs()),2)

    def test_precision_requires_relative_and_absolute_agreement(self):
        low=self.small()['rows']; hi=copy.deepcopy(low)
        self.assertTrue(run.precision(low,hi)['passed'])
        hi[0]['outside']+=1e-5
        self.assertFalse(run.precision(low,hi)['passed'])

    def test_precision_checks_small_fractions_relatively(self):
        low=self.small()['rows']; low[0]['outside_fraction_of_dome']=1e-15
        hi=copy.deepcopy(low); hi[0]['outside_fraction_of_dome']=2e-15
        self.assertFalse(run.precision(low,hi)['passed'])

    def test_empty_complete_receipt_clears_stale_pass(self):
        with tempfile.TemporaryDirectory() as tmp:
            out=Path(tmp)/'audit'; r=run.initialize(out)
            r.update(status='COMPLETE',precision_checks=[])
            run.shared.write(out/'receipt.json',r)
            run.shared.write(out/'verification.json',dict(status='PASS'))
            with self.assertRaises(ValueError): run.verify(out)
            self.assertEqual(json.loads((out/'verification.json').read_text())['status'],'FAIL')

    def test_nonempty_output_refuses(self):
        with tempfile.TemporaryDirectory() as tmp:
            out=Path(tmp)/'audit';run.initialize(out)
            with self.assertRaises(ValueError):run.initialize(out)

    def test_small_pipeline_and_tampered_archive(self):
        small=[dict(N=0,g2=1.,alpha=2.,beta=2.,orders=[0,2],K_ref=4,dps=p) for p in (80,120)]
        with tempfile.TemporaryDirectory() as tmp,patch.object(run,'jobs',return_value=small):
            out=Path(tmp)/'audit';run.initialize(out)
            self.assertEqual(run.worker(out),0)
            self.assertEqual(run.verify(out)['replayed_batches'],2)
            (out/'batch-000.json').write_text('{}')
            with self.assertRaises(ValueError):run.verify(out)
            self.assertEqual(json.loads((out/'verification.json').read_text())['status'],'FAIL')


if __name__=='__main__':unittest.main(verbosity=2)
