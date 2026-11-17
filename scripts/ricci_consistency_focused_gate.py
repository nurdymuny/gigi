"""Run only the RF2D focused tests, with a receipt distinct from BW05."""
import argparse
import json
from pathlib import Path
import subprocess
import sys
import unittest

ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'e2e/probes'))
from monodromy_evidence import sha
from ricci_lattice_evidence import read,same
from ricci_consistency_diagnostic import frozen
from ricci_consistency_evidence import extract_hhe

MODULES=['test_ricci_consistency_math','test_ricci_consistency_controls',
    'test_ricci_consistency_evidence','test_ricci_consistency_spectral']


def main(directory):
    path=directory/'FOCUSED_TESTS.json';receipt={'status':'IN_PROGRESS','release_gate':'OPEN',
        'full_flow_gate':'FAIL_RETAINED_FROM_RF2C','full_regressions':'DEFERRED',
        'test_sources':{str(ROOT/'e2e/probes'/(n+'.py')):sha(ROOT/'e2e/probes'/(n+'.py')) for n in MODULES},
        'runner_sha256':sha(Path(__file__))}
    path.write_text(json.dumps(receipt,indent=2))
    try:
        if read(directory/'manifest.json')!=frozen():raise ValueError('source drift')
        verified=read(directory/'VERIFIED_DIAGNOSTIC.json')
        if verified['diagnostic_sha256']!=sha(directory/'diagnostic.json') or not verified['evidence_valid']:
            raise ValueError('verified complete diagnostic required')
        suite=unittest.defaultTestLoader.loadTestsFromNames(MODULES)
        result=unittest.TextTestRunner(verbosity=2).run(suite)
        receipt.update(python_tests=result.testsRun,failures=[(str(t),x) for t,x in result.failures],
            errors=[(str(t),x) for t,x in result.errors],skipped=[(str(t),x) for t,x in result.skipped])
        if not result.wasSuccessful() or result.skipped:raise ValueError('focused Python gate failed')
        native=read(directory/'NATIVE_RECEIPT.json')
        if sha(Path(native['binary']))!=native['binary_sha256']:raise ValueError('native test executable changed')
        output=subprocess.run([native['binary'],'--nocapture','--test-threads=1'],check=True,capture_output=True,text=True,timeout=120)
        if '6 passed; 0 failed' not in output.stdout:raise ValueError('six native tests required')
        same(extract_hhe(output.stdout),[native['hhe']])
        receipt['native_tests']=6;receipt['status']='PASS'
        if read(directory/'manifest.json')!=frozen():raise ValueError('source drift during test run')
    except BaseException as error:
        receipt['status']='FAIL';receipt['error']=str(error);raise
    finally:path.write_text(json.dumps(receipt,indent=2))


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('directory',type=Path);a=p.parse_args();main(a.directory.resolve())
