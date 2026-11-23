"""Adversarial checks on the captured RF2C run, without repeating eight solves per attack.

The main verifier performs every native replay. These tests delegate the attacked
coarse case to the real executable, and copy already-replayed unaffected outputs.
No numerical comparison, mesh reconstruction, or source guard is mocked.
"""
import copy
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch
from monodromy_evidence import sha
from ricci_lattice_evidence import read
from ricci_shape_experiment import case_keys,write
from ricci_shape_evidence import verify


@unittest.skipUnless(os.environ.get('GIGI_RF2C_EVIDENCE'),'requires replayed RF2C evidence')
class EvidenceAttacks(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.source=Path(os.environ['GIGI_RF2C_EVIDENCE']).resolve()
        verified=read(cls.source/'VERIFIED_DIAGNOSTIC.json')
        if not verified['evidence_valid'] or verified['native_replays']!=8:raise ValueError('full replay required first')
        if verified['diagnostic_sha256']!=sha(cls.source/'diagnostic.json'):raise ValueError('unverified diagnostic')
        cls.receipt=read(cls.source/'diagnostic.json');cls.manifest=read(cls.source/'manifest.json')

    def setup_attack(self, directory):
        r=copy.deepcopy(self.receipt);m=copy.deepcopy(self.manifest)
        for key in case_keys():
            for suffix in ['.input.json','.output.json']:
                os.link(self.source/(key+suffix),directory/(key+suffix))
        for name in ['gate.PASS.json','VERIFIED_DIAGNOSTIC.json']:(directory/name).write_text('{}')
        return r,m

    def test_contract_source_and_completeness_attacks(self):
        for attack in ['missing_level','altered_threshold','invented_proof','wrong_source','missing_source',
            'wrong_binary','wrong_mesh_count','zero_error','tampered_native_gradient','partial_run']:
            with self.subTest(attack=attack),tempfile.TemporaryDirectory(prefix='rf2c-attack-') as temp:
                d=Path(temp);r,m=self.setup_attack(d);key='level0_berger_q4'
                expected=''
                if attack=='missing_level':r['cases'].pop('level3_berger_q6');expected='staged contract'
                elif attack=='altered_threshold':r['contract']['maximum_final_relative_error']=.5;expected='staged contract'
                elif attack=='invented_proof':r['contract']['continuum_convergence_proven']=True;expected='staged contract'
                elif attack=='wrong_source':m['frozen_sources'][next(iter(m['frozen_sources']))]='0'*64;expected='drift'
                elif attack=='missing_source':m['frozen_sources'].pop(next(iter(m['frozen_sources'])));expected='drift'
                elif attack=='wrong_binary':m['binary']['sha256']='0'*64;expected='drift'
                elif attack=='wrong_mesh_count':r['geometry']['0']['tetrahedra']-=1;expected='mismatch'
                elif attack=='zero_error':r['cases'][key]['measures']['full_mass_error']=0.;expected='mismatch'
                elif attack=='tampered_native_gradient':
                    p=d/(key+'.output.json');raw=read(p);raw['gradient'][0][0]+=.01
                    # Break the temporary hard link before mutating any bytes.
                    p.unlink();write(p,raw);r['cases'][key]['output_sha256']=sha(p);expected='native replay differs'
                elif attack=='partial_run':r['status']='IN_PROGRESS';expected='staged contract'
                write(d/'manifest.json',m);r['manifest_sha256']=sha(d/'manifest.json');write(d/'diagnostic.json',r)
                real_run=subprocess.run
                def replay(args,**kwargs):
                    if args[0]!=m['binary']['path'] or Path(args[1]).name==key+'.input.json':
                        return real_run(args,**kwargs)
                    output_name=Path(args[1]).name.replace('.input.json','.output.json')
                    shutil.copyfile(self.source/output_name,args[2])
                    return subprocess.CompletedProcess(args,0)
                with patch('ricci_shape_evidence.subprocess.run',side_effect=replay):
                    with self.assertRaisesRegex(ValueError,expected):verify(d)
                self.assertFalse((d/'gate.PASS.json').exists())
                self.assertFalse((d/'VERIFIED_DIAGNOSTIC.json').exists())
        self.assertEqual(sha(self.source/'diagnostic.json'),read(self.source/'VERIFIED_DIAGNOSTIC.json')['diagnostic_sha256'])


if __name__=='__main__':unittest.main()
