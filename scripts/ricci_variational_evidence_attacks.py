"""Attack real completed evidence; verify refusal removes every old PASS."""
import argparse
import copy
import json
import os
from pathlib import Path
import shutil
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'e2e/probes'))
from ricci_variational_evidence import verify
from ricci_lattice_evidence import read
from ricci_shape_experiment import write
from monodromy_evidence import sha


def run(source):
    source = Path(source); original = read(source/'diagnostic.json')
    if original['status'] != 'COMPLETE_UNVERIFIED': raise ValueError('completed real evidence required')
    def wrong_number(r): r['cases']['level0_q4']['controls']['0']['full_mass_error'] *= .1
    def missing(r): del r['cases']['level3_q6']
    def fake_pass(r): r['numerical_gate']['pass'] = True
    def fake_berger(r): r['berger'] = 'PASS'
    def fake_array(r): r['cases']['level0_q4']['artifacts']['level0_q4.arrays.npz'] = 'f'*64
    attacks = {'invented_error':wrong_number,'missing_level':missing,'forged_numeric_pass':fake_pass,
               'forged_berger':fake_berger,'changed_array_digest':fake_array,
               'nonfinite_number':lambda r:r['cases']['level0_q4']['controls']['0'].update(relative_error=float('nan'))}
    results = {}
    with tempfile.TemporaryDirectory(prefix='rf2f-evidence-attacks-') as temp:
        directory = Path(temp)
        shutil.copyfile(source/'manifest.json',directory/'manifest.json')
        for path in source.glob('*.npz'):
            # Read-only hardlinks avoid copying large arrays. No attack writes these files.
            os.link(path,directory/path.name)
        for name, attack in attacks.items():
            receipt = copy.deepcopy(original); attack(receipt)
            (directory/'diagnostic.json').write_text(json.dumps(receipt),encoding='utf-8')
            for stale in ['gate.PASS.json','VERIFIED_DIAGNOSTIC.json']:
                write(directory/stale,{'pass':True})
            try:
                verify(directory)
            except (ValueError, AssertionError) as error:
                if any((directory/p).exists() for p in ['gate.PASS.json','VERIFIED_DIAGNOSTIC.json']):
                    raise ValueError('rejected evidence left a stale PASS')
                results[name] = {'rejected':True,'reason':str(error),'stale_pass_removed':True}
            else:
                raise ValueError('bad evidence accepted: '+name)
    report = {'attacks':results,'diagnostic_sha256':sha(source/'diagnostic.json'),
              'script_sha256':sha(Path(__file__)),
              'verifier_sha256':sha(ROOT/'e2e/probes/ricci_variational_evidence.py')}
    if (source/'EVIDENCE_ATTACKS.json').exists(): raise ValueError('preserve prior attack receipt')
    write(source/'EVIDENCE_ATTACKS.json',report)
    print(json.dumps(report,indent=2))


if __name__ == '__main__':
    parser=argparse.ArgumentParser();parser.add_argument('source',type=Path);run(parser.parse_args().source)
