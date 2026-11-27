"""Planted linear-algebra evidence, not a synthetic physics success."""
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import numpy as np
import run_helike_coupled_enrichment as check
import check_helike_enrichment_evidence as audit


def fixture():
    return dict(positions=[0,2], columns=np.array([[0.,1.,0.,0.,0.]]),
                vectors=np.eye(3), residuals=np.array([[0.,0.,0.,4.,0.],[0.,0.,0.,0.,3.]]),
                inside=[0.,0.], outside=[4.,3.])


class Evidence(unittest.TestCase):
    def test_failed_replay_replaces_old_pass(self):
        with tempfile.TemporaryDirectory() as root:
            root=Path(root); run=root/'run'; run.mkdir(); out=root/'audit.json'
            (run/'receipt.json').write_text(json.dumps({'status':'RUNNING'}))
            (run/'native-receipt.json').write_text('{}')
            out.write_text(json.dumps({'baseline_replay':'PASS'}))
            with patch('sys.argv',['audit',str(run),str(out)]):
                try: audit.main()
                except ValueError: pass
            result=json.loads(out.read_text())
            self.assertEqual(result.get('status'),'STOPPED')
            self.assertNotEqual(result.get('baseline_replay'),'PASS')

    def test_changed_contract_refuses_before_reading_results(self):
        good={'schema':'helike-lab/cone-enrichment/v1','s':1.18,'t':3.,'g2':.03,'base_mq':12,'probe_mq':14,
              'base_dimension':18564,'dimension':18568,'ambient_dimension':38760,'domain_gate':'NOT_RUN',
              'identification_residual_space':'W','certified':False,'full_grid_released':False,'solver':check.SETTINGS}
        for key,value in [('dimension',16),('certified',True),('full_grid_released',True),('domain_gate','PASS'),
                          ('solver',dict(check.SETTINGS,tol=1e-4)),('identification_residual_space','V14')]:
            with self.subTest(key=key),self.assertRaises(ValueError): check.inspect(dict(good,**{key:value}),{})

    def test_changed_parent_stops_and_existing_output_refuses(self):
        with tempfile.TemporaryDirectory() as root:
            out=Path(root)/'result'
            with patch('sys.argv',['runner',str(out)]),patch.object(check.grid,'sha',return_value='changed'):
                self.assertEqual(check.main(),2)
                r=json.loads((out/'receipt.json').read_text())
                self.assertEqual(r['status'],'STOPPED')
                self.assertEqual(r['overall'],'UNRESOLVED')
                self.assertIs(r['full_grid_released'],False)
                with self.assertRaises(FileExistsError): check.main()

    def test_valid_projection(self):
        r=check.geometry(**fixture())
        np.testing.assert_allclose(r['outside'],[4.,3.])

    def test_bad_evidence_refused(self):
        variants=[]
        x=fixture(); x['outside'][0]=0.; variants.append(x)
        x=fixture(); x['columns'][0,0]=1.; variants.append(x)
        x=fixture(); x['columns']*=2.; variants.append(x)
        x=fixture(); x['positions']=[0,0]; variants.append(x)
        x=fixture(); x['vectors'][0,0]=2.; variants.append(x)
        x=fixture(); x['residuals'][0,0]=1.; variants.append(x)
        x=fixture(); x['residuals'][0,0]=np.nan; variants.append(x)
        for x in variants:
            with self.subTest(x=x),self.assertRaises(ValueError): check.geometry(**x)

    def test_small_step_cannot_release_grid_or_replace_weight(self):
        old={'helike':{'gap':1.05},'competitor':{'gap':1.09}}
        new={'helike':{'gap':1.049,'dressed_weight':.96},'competitor':{'gap':1.089,'dressed_weight':.91,'dressed_overlap':.99}}
        x=check.assessment(old,new)
        self.assertEqual(x['competitor']['step_gate'],'PASS')
        self.assertEqual(x['competitor']['export_weight_necessary_gate'],'FAIL')
        self.assertEqual(x['overall'],'UNRESOLVED')
        self.assertIs(x['full_grid_released'],False)
        self.assertEqual(x['domain_gate'],'NOT_RUN')
        new['competitor']['gap']=float('nan')
        with self.assertRaises(ValueError): check.assessment(old,new)


if __name__=='__main__': unittest.main()
