"""Receipt and supervisor gates; never start a numerical survey."""
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import Mock, patch

import psutil
import numpy as np
import run_quantum_order_controls as r


class EvidenceTests(unittest.TestCase):
    def fixture(self, p):
        receipt=dict(status='COMPLETE',mode='EXISTING_CONTROLS',scientific_status='UNRESOLVED',
                     source_hashes={'fixture.txt':'pinned'},gates={n:True for n in r.GATES},
                     cases=[dict(cutoff=n,g2=g,alpha=a,arrays=f'N{n}-g2-{g:g}-alpha-{a:g}.npz',
                                 arrays_sha256='pinned') for n in r.CUTOFFS for g,a in r.CONTROLS])
        (p/'receipt.json').write_text(json.dumps(receipt))
        (p/'verification.json').write_text(json.dumps({'status':'PASS'}))
        return receipt

    def test_source_and_snapshot_corruption_clear_pass(self):
        for mode in ('current','snapshot'):
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as tmp:
                p=Path(tmp); self.fixture(p)
                current={'fixture.txt':'wrong' if mode=='current' else 'pinned'}
                with patch.object(r,'hashes',return_value=current), patch.object(r.base,'digest',return_value='wrong' if mode=='snapshot' else 'pinned'):
                    with self.assertRaisesRegex(ValueError,'source'):
                        r.verify_run(p,replay=False)
                self.assertEqual(json.loads((p/'verification.json').read_text())['status'],'FAIL')

    def test_array_corruption_clears_pass(self):
        with tempfile.TemporaryDirectory() as tmp:
            p=Path(tmp); receipt=self.fixture(p)
            def digest(path):
                return 'pinned' if path.name=='fixture.txt' else 'bad'
            with patch.object(r,'hashes',return_value=receipt['source_hashes']), patch.object(r.base,'digest',side_effect=digest):
                with self.assertRaisesRegex(ValueError,'integrity'):
                    r.verify_run(p,replay=False)
            self.assertEqual(json.loads((p/'verification.json').read_text())['status'],'FAIL')

    def test_wrong_array_identity_is_refused_before_read(self):
        with tempfile.TemporaryDirectory() as tmp:
            p=Path(tmp); receipt=self.fixture(p)
            receipt['cases'][0]['arrays']='../unrelated.npz'
            (p/'receipt.json').write_text(json.dumps(receipt))
            with patch.object(r,'hashes',return_value=receipt['source_hashes']), patch.object(r.base,'digest',return_value='pinned'):
                with self.assertRaisesRegex(ValueError,'identity'):
                    r.verify_run(p,replay=False)
            self.assertEqual(json.loads((p/'verification.json').read_text())['status'],'FAIL')

    def test_incomplete_replay_invalidates_stale_pass(self):
        with tempfile.TemporaryDirectory() as tmp:
            p=Path(tmp)
            (p/'receipt.json').write_text(json.dumps({'status':'RUNNING'}))
            (p/'verification.json').write_text(json.dumps({'status':'PASS'}))
            with self.assertRaisesRegex(ValueError,'incomplete'):
                r.verify_run(p)
            self.assertEqual(json.loads((p/'verification.json').read_text())['status'],'FAIL')

    def test_malformed_receipt_invalidates_stale_pass(self):
        with tempfile.TemporaryDirectory() as tmp:
            p=Path(tmp)
            (p/'receipt.json').write_text('{bad')
            (p/'verification.json').write_text(json.dumps({'status':'PASS'}))
            with self.assertRaises(ValueError):
                r.verify_run(p)
            self.assertEqual(json.loads((p/'verification.json').read_text())['status'],'FAIL')

    def test_exact_six_control_identities(self):
        cases=[{'cutoff':n,'g2':g,'alpha':a} for n in (10,12,14) for g,a in ((1.,1.7),(2.,2.3))]
        self.assertTrue(r.validate_cases(cases))
        for broken in (cases[:-1],cases+[cases[-1]],list(reversed(cases))):
            with self.subTest(broken=broken), self.assertRaisesRegex(ValueError,'control'):
                r.validate_cases(broken)

    def test_held_out_coupling_is_refused(self):
        cases=[{'cutoff':n,'g2':g,'alpha':a} for n in (10,12,14) for g,a in ((1.,1.7),(2.,2.3))]
        cases[0]=dict(cases[0],g2=1.60)
        with self.assertRaisesRegex(ValueError,'control'):
            r.validate_cases(cases)


class SupervisorTests(unittest.TestCase):
    def test_memory_stop_is_not_complete(self):
        with tempfile.TemporaryDirectory() as tmp:
            p=Path(tmp)
            (p/'receipt.json').write_text(json.dumps({'status':'RUNNING','cases':[]}))
            process=Mock(pid=123)
            process.poll.side_effect=[None,1]
            process.wait.return_value=-9
            monitored=Mock()
            monitored.memory_info.return_value=Mock(rss=9*1024**3)
            monitored.cpu_times.return_value=Mock(user=0.,system=0.)
            with patch.object(psutil,'Process',return_value=monitored):
                code=r.supervise(process,p)
            self.assertEqual(code,2)
            self.assertEqual(json.loads((p/'receipt.json').read_text())['status'],'RESOURCE_STOP')
            process.kill.assert_called_once()
            self.assertTrue((p/'resources.csv').exists())

    def test_worker_reason_preserved(self):
        with tempfile.TemporaryDirectory() as tmp:
            p=Path(tmp)
            (p/'receipt.json').write_text(json.dumps({'status':'FAILED','error':'planted exact error'}))
            process=Mock(pid=123)
            process.poll.return_value=1
            process.wait.return_value=1
            code=r.supervise(process,p)
            self.assertNotEqual(code,0)
            self.assertEqual(json.loads((p/'receipt.json').read_text())['error'],'planted exact error')


class ReplayTests(unittest.TestCase):
    def test_saved_states_and_report_are_recomputed(self):
        with tempfile.TemporaryDirectory() as tmp, patch.object(r,'event'):
            p=Path(tmp); b=r.q.build_basis(4); ops=r.q.build_operators(b)
            codes=r.q.oscillator_codes(8)
            c=r.measure(b,ops,1.,1.7,'cpu',p,codes)
            r.replay_case(p,c,b,ops,codes)
            c['levels']['tensor_next']['residual']+=.25
            c['order']=r.o.ordering(*r.branch_inputs(c['levels']))
            with self.assertRaises((ValueError,AssertionError)):
                r.replay_case(p,c,b,ops,codes)
            c['levels']['tensor_next']['residual']-=.25
            c['order']=r.o.ordering(*r.branch_inputs(c['levels']))
            path=p/c['arrays']
            with np.load(path) as saved:
                arrays={key:saved[key] for key in saved.files}
            arrays['R_tensor_next']=arrays['R_tensor_next']+.25*np.eye(5)
            np.savez_compressed(path,**arrays)
            c['arrays_sha256']=r.base.digest(path)
            with self.assertRaises(AssertionError):
                r.replay_case(p,c,b,ops,codes)


if __name__=='__main__':
    unittest.main(verbosity=2)
