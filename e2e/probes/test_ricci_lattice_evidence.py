import copy
import json
from pathlib import Path
import tempfile
import unittest
import os
from unittest.mock import patch
from ricci_lattice_evidence import verify


class RefusalTests(unittest.TestCase):
    def test_missing_evidence_cannot_leave_stale_pass(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);(root/'gate.PASS.json').write_text('{"status":"PASS"}')
            with self.assertRaises((ValueError,FileNotFoundError)):verify(root)
            self.assertFalse((root/'gate.PASS.json').exists())

    def test_invented_proof_refused(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);(root/'diagnostic.json').write_text(json.dumps({
                'schema':'rf2b_operator_diagnostic_v1','status':'DIAGNOSTIC_COMPLETE_FULL_GATE_OPEN',
                'gate':'PASS','continuum_convergence_proven':True,'time_scale':1,'fitted_constants':[],'cases':{}}))
            with self.assertRaises(ValueError):verify(root)

    def test_malformed_evidence_removes_both_receipts(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);(root/'diagnostic.json').write_text('{')
            for name in ['gate.PASS.json','VERIFIED_DIAGNOSTIC.json']:(root/name).write_text('{}')
            with self.assertRaises(ValueError):verify(root)
            self.assertFalse((root/'gate.PASS.json').exists())
            self.assertFalse((root/'VERIFIED_DIAGNOSTIC.json').exists())


@unittest.skipUnless(os.environ.get('GIGI_RF2B_OPERATOR_BINARY'),'requires explicitly pinned native probe')
class ReplayAttackTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        from ricci_lattice_operator import orbit_fixture
        from ricci_lattice_convergence import measures,ROOT
        from ricci_lattice_mesh import mesh,validate
        from test_ricci_operator_native import invoke
        from monodromy_evidence import sha
        cls.fixture=orbit_fixture(0);cls.raw=invoke(cls.fixture)
        binary=Path(os.environ['GIGI_RF2B_OPERATOR_BINARY']).resolve()
        cls.payload=dict(metric=[3.,1.,1.],order=4,edge_points=cls.fixture['edge_points'],cells=cls.fixture['cells'])
        cls.receipt={'schema':'rf2b_operator_diagnostic_v1','status':'DIAGNOSTIC_COMPLETE_FULL_GATE_OPEN','gate':'OPEN',
            'continuum_convergence_proven':False,'time_scale':1,'fitted_constants':[],
            'binary':{'path':str(binary),'sha256':sha(binary)},
            'sources':{str(ROOT/'src/gauge/ricci_simplicial.rs'):sha(ROOT/'src/gauge/ricci_simplicial.rs')},
            'cases':{'level0_berger_q4':{'metric':[3.,1.,1.],'level':0,'quadrature_order':4,
                'mesh':validate(mesh(0)),'measures':measures(cls.fixture,[3.,1.,1.],cls.raw)}}}

    def evidence(self,root):
        from monodromy_evidence import sha
        receipt=copy.deepcopy(self.receipt)
        a=root/'level0_berger_q4.input.json';b=root/'level0_berger_q4.output.json'
        a.write_text(json.dumps(self.payload));b.write_text(json.dumps(self.raw,separators=(',',':')))
        # Native serde output uses its own number spelling and ordering; retain
        # exact executable bytes for the baseline rather than rewriting JSON.
        import subprocess
        subprocess.run([receipt['binary']['path'],str(a),str(b)],check=True,capture_output=True)
        case=receipt['cases']['level0_berger_q4'];case['input_sha256']=sha(a);case['output_sha256']=sha(b)
        return receipt

    def test_honest_partial_data_remains_open(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);r=self.evidence(root);(root/'diagnostic.json').write_text(json.dumps(r))
            result=verify(root)
            self.assertTrue(result['evidence_valid']);self.assertEqual(result['operator_numeric_gate'],'OPEN')
            self.assertEqual(result['correspondence'],'NOT_ESTABLISHED')
            self.assertFalse((root/'gate.PASS.json').exists())

    def test_forgery_matrix_and_no_stale_success(self):
        from monodromy_evidence import sha
        for attack in ['invent_zero','deleted_cases','fitted_clock','stale_source','stale_binary','wrong_metric','wrong_mesh','raw_force_changed','missing_raw','running_not_finished']:
            with self.subTest(attack=attack),tempfile.TemporaryDirectory() as folder:
                root=Path(folder);r=self.evidence(root);case=r['cases']['level0_berger_q4']
                if attack=='invent_zero':case['measures']['full_mass_error']=0.
                elif attack=='deleted_cases':r['cases']={}
                elif attack=='fitted_clock':r['time_scale']=1.1
                elif attack=='stale_source':r['sources'][next(iter(r['sources']))]='0'*64
                elif attack=='stale_binary':r['binary']['sha256']='0'*64
                elif attack=='wrong_metric':case['metric']=[1.,1.,1.]
                elif attack=='wrong_mesh':case['mesh']['tetrahedra']=599
                elif attack=='raw_force_changed':
                    p=root/'level0_berger_q4.output.json';out=json.loads(p.read_text());out['gradient'][0][0]=0.
                    p.write_text(json.dumps(out));case['output_sha256']=sha(p)
                elif attack=='missing_raw':(root/'level0_berger_q4.output.json').unlink()
                elif attack=='running_not_finished':r['status']='IN_PROGRESS'
                (root/'diagnostic.json').write_text(json.dumps(r))
                for name in ['gate.PASS.json','VERIFIED_DIAGNOSTIC.json']:(root/name).write_text('{}')
                with self.assertRaises((ValueError,FileNotFoundError)):verify(root)
                self.assertFalse((root/'gate.PASS.json').exists());self.assertFalse((root/'VERIFIED_DIAGNOSTIC.json').exists())

    def test_solver_failure_cannot_be_reported_as_a_number(self):
        from ricci_lattice_convergence import solve
        import numpy as np
        with patch('ricci_lattice_convergence.cg',return_value=(np.zeros(18),4)):
            with self.assertRaises(ValueError):solve(self.raw)


if __name__=='__main__':unittest.main()
