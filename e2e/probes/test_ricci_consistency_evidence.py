import copy
import json
import os
from pathlib import Path
import tempfile
import unittest
from monodromy_evidence import sha
from ricci_lattice_evidence import read
from ricci_consistency_evidence import verify,extract_hhe
from ricci_consistency_diagnostic import ROOT

DATA=ROOT/'output/ricci-consistency-20261117/diagnostic'


class EvidenceTests(unittest.TestCase):
    def test_rust_harness_inline_receipt_is_not_lost(self):
        expected={'status':'bounded_holomorphic'}
        text='test oc08_hhe ... OC08_HHE '+json.dumps(expected)+'\nok\n'
        self.assertEqual(extract_hhe(text),[expected])

    def test_missing_and_malformed_evidence_never_leave_stale_success(self):
        for raw in [None,'{','{}']:
            with self.subTest(raw=raw),tempfile.TemporaryDirectory() as t:
                d=Path(t)
                for name in ['gate.PASS.json','VERIFIED_DIAGNOSTIC.json']:(d/name).write_text('{}')
                if raw is not None:(d/'diagnostic.json').write_text(raw)
                with self.assertRaises((ValueError,FileNotFoundError)):verify(d)
                for name in ['gate.PASS.json','VERIFIED_DIAGNOSTIC.json']:self.assertFalse((d/name).exists())

    def test_source_result_and_array_forgery_matrix(self):
        original=read(DATA/'diagnostic.json');manifest=read(DATA/'manifest.json')
        for attack in ['missing_level','changed_mapping','zero_action','erased_sign','array_forgery','invented_pass','partial']:
            with self.subTest(attack=attack),tempfile.TemporaryDirectory() as t:
                d=Path(t);r=copy.deepcopy(original);m=copy.deepcopy(manifest)
                for p in DATA.glob('*.npz'):os.link(p,d/p.name)
                for name in ['gate.PASS.json','VERIFIED_DIAGNOSTIC.json']:(d/name).write_text('{}')
                if attack=='missing_level':r['cases'].pop('level3_q6')
                elif attack=='changed_mapping':m['time_scale']=2
                elif attack=='zero_action':r['cases']['level0_q4']['berger_split']['action_error']=0.
                elif attack=='erased_sign':r['cases']['level0_q4']['double_cover']['face_occurrences']['q0_min']=1.
                elif attack=='array_forgery':
                    import numpy as np
                    p=d/'level0_q4.arrays.npz';a=dict(np.load(p));a['load']=np.zeros_like(a['load'])
                    p.unlink();np.savez_compressed(p,**a);r['cases']['level0_q4']['arrays_sha256']=sha(p)
                elif attack=='invented_pass':r['release_gate']='PASS'
                elif attack=='partial':r['status']='IN_PROGRESS'
                (d/'manifest.json').write_text(json.dumps(m));r['manifest_sha256']=sha(d/'manifest.json')
                (d/'diagnostic.json').write_text(json.dumps(r))
                with self.assertRaises((ValueError,AssertionError)):verify(d)
                for name in ['gate.PASS.json','VERIFIED_DIAGNOSTIC.json']:self.assertFalse((d/name).exists())


if __name__=='__main__':unittest.main()
