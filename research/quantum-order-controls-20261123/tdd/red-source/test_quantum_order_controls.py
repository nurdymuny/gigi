"""Receipt and supervisor gates; never start a numerical survey."""
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import Mock, patch

import psutil
import run_quantum_order_controls as r


class EvidenceTests(unittest.TestCase):
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
            process.poll.return_value=None
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


if __name__=='__main__':
    unittest.main(verbosity=2)
